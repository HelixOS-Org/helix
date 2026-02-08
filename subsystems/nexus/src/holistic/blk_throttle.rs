// SPDX-License-Identifier: GPL-2.0
//! Holistic blk_throttle — block I/O throttling and bandwidth control.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// I/O direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoDirection {
    Read,
    Write,
}

/// Throttle policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottlePolicy {
    /// Hard bandwidth limit
    Limit,
    /// Proportional weight
    Weight,
    /// Latency target
    LatencyTarget,
    /// Idle-time based
    Idle,
    /// No throttling
    None,
}

/// Bandwidth limit
#[derive(Debug, Clone)]
pub struct BwLimit {
    pub bps_read: u64,
    pub bps_write: u64,
    pub iops_read: u64,
    pub iops_write: u64,
}

impl BwLimit {
    pub fn unlimited() -> Self {
        Self { bps_read: u64::MAX, bps_write: u64::MAX, iops_read: u64::MAX, iops_write: u64::MAX }
    }

    pub fn is_unlimited(&self) -> bool {
        self.bps_read == u64::MAX && self.bps_write == u64::MAX
            && self.iops_read == u64::MAX && self.iops_write == u64::MAX
    }

    pub fn check_bps(&self, dir: IoDirection, bytes: u64) -> bool {
        match dir {
            IoDirection::Read => bytes <= self.bps_read,
            IoDirection::Write => bytes <= self.bps_write,
        }
    }

    pub fn check_iops(&self, dir: IoDirection, ios: u64) -> bool {
        match dir {
            IoDirection::Read => ios <= self.iops_read,
            IoDirection::Write => ios <= self.iops_write,
        }
    }
}

/// Latency target config
#[derive(Debug, Clone)]
pub struct LatencyTarget {
    pub target_us: u64,
    pub window_us: u64,
    pub percentile: f64,
    pub enabled: bool,
}

/// Per-cgroup I/O stats
#[derive(Debug)]
pub struct CgroupIoStat {
    pub cgroup_id: u64,
    pub device_id: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub ios_read: u64,
    pub ios_written: u64,
    pub throttled_bytes: u64,
    pub throttled_ios: u64,
    pub throttle_time_us: u64,
    pub avg_latency_us: u64,
    pub weight: u16,
}

impl CgroupIoStat {
    pub fn new(cgroup_id: u64, device_id: u64) -> Self {
        Self {
            cgroup_id, device_id,
            bytes_read: 0, bytes_written: 0,
            ios_read: 0, ios_written: 0,
            throttled_bytes: 0, throttled_ios: 0,
            throttle_time_us: 0, avg_latency_us: 0,
            weight: 100,
        }
    }

    pub fn total_bytes(&self) -> u64 {
        self.bytes_read + self.bytes_written
    }

    pub fn total_ios(&self) -> u64 {
        self.ios_read + self.ios_written
    }

    pub fn throttle_ratio(&self) -> f64 {
        let total = self.total_bytes();
        if total == 0 { return 0.0; }
        self.throttled_bytes as f64 / total as f64
    }

    pub fn avg_io_size(&self) -> u64 {
        let ios = self.total_ios();
        if ios == 0 { return 0; }
        self.total_bytes() / ios
    }
}

/// Block device for throttling
#[derive(Debug)]
pub struct ThrottleDevice {
    pub device_id: u64,
    pub device_name: String,
    pub policy: ThrottlePolicy,
    pub limit: BwLimit,
    pub latency: LatencyTarget,
    pub queue_depth: u32,
    pub max_queue_depth: u32,
    pub dispatched_bytes: u64,
    pub dispatched_ios: u64,
    pub pending_ios: u32,
}

impl ThrottleDevice {
    pub fn new(device_id: u64, name: String) -> Self {
        Self {
            device_id, device_name: name,
            policy: ThrottlePolicy::None,
            limit: BwLimit::unlimited(),
            latency: LatencyTarget { target_us: 5000, window_us: 100_000, percentile: 0.95, enabled: false },
            queue_depth: 0, max_queue_depth: 128,
            dispatched_bytes: 0, dispatched_ios: 0,
            pending_ios: 0,
        }
    }

    pub fn queue_utilization(&self) -> f64 {
        if self.max_queue_depth == 0 { return 0.0; }
        self.pending_ios as f64 / self.max_queue_depth as f64
    }

    pub fn is_congested(&self) -> bool {
        self.queue_utilization() > 0.8
    }

    pub fn avg_io_size(&self) -> u64 {
        if self.dispatched_ios == 0 { return 0; }
        self.dispatched_bytes / self.dispatched_ios
    }
}

/// Throttle event
#[derive(Debug, Clone)]
pub struct ThrottleEvent {
    pub cgroup_id: u64,
    pub device_id: u64,
    pub direction: IoDirection,
    pub bytes: u64,
    pub delay_us: u64,
    pub reason: ThrottleReason,
    pub timestamp: u64,
}

/// Throttle reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrottleReason {
    BpsLimit,
    IopsLimit,
    LatencyExceeded,
    WeightFairness,
    QueueFull,
}

/// Throttle stats
#[derive(Debug, Clone)]
pub struct BlkThrottleStats {
    pub total_throttle_events: u64,
    pub total_bytes_throttled: u64,
    pub total_delay_us: u64,
    pub device_count: u32,
    pub cgroup_count: u32,
    pub avg_throttle_delay_us: u64,
}

/// Main block throttle manager
pub struct HolisticBlkThrottle {
    devices: BTreeMap<u64, ThrottleDevice>,
    cgroup_io: BTreeMap<(u64, u64), CgroupIoStat>,
    cgroup_limits: BTreeMap<(u64, u64), BwLimit>,
    events: Vec<ThrottleEvent>,
    max_events: usize,
    stats: BlkThrottleStats,
}

impl HolisticBlkThrottle {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            cgroup_io: BTreeMap::new(),
            cgroup_limits: BTreeMap::new(),
            events: Vec::new(),
            max_events: 4096,
            stats: BlkThrottleStats {
                total_throttle_events: 0, total_bytes_throttled: 0,
                total_delay_us: 0, device_count: 0, cgroup_count: 0,
                avg_throttle_delay_us: 0,
            },
        }
    }

    pub fn add_device(&mut self, dev: ThrottleDevice) {
        self.stats.device_count += 1;
        self.devices.insert(dev.device_id, dev);
    }

    pub fn set_limit(&mut self, cgroup_id: u64, device_id: u64, limit: BwLimit) {
        self.cgroup_limits.insert((cgroup_id, device_id), limit);
    }

    pub fn record_io(&mut self, cgroup_id: u64, device_id: u64, dir: IoDirection, bytes: u64) {
        let key = (cgroup_id, device_id);
        let stat = self.cgroup_io.entry(key)
            .or_insert_with(|| CgroupIoStat::new(cgroup_id, device_id));
        match dir {
            IoDirection::Read => { stat.bytes_read += bytes; stat.ios_read += 1; }
            IoDirection::Write => { stat.bytes_written += bytes; stat.ios_written += 1; }
        }
        if let Some(dev) = self.devices.get_mut(&device_id) {
            dev.dispatched_bytes += bytes;
            dev.dispatched_ios += 1;
        }
    }

    pub fn record_throttle(&mut self, event: ThrottleEvent) {
        self.stats.total_throttle_events += 1;
        self.stats.total_bytes_throttled += event.bytes;
        self.stats.total_delay_us += event.delay_us;
        let n = self.stats.total_throttle_events;
        self.stats.avg_throttle_delay_us =
            ((self.stats.avg_throttle_delay_us * (n - 1)) + event.delay_us) / n;

        let key = (event.cgroup_id, event.device_id);
        if let Some(stat) = self.cgroup_io.get_mut(&key) {
            stat.throttled_bytes += event.bytes;
            stat.throttled_ios += 1;
            stat.throttle_time_us += event.delay_us;
        }

        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    pub fn should_throttle(&self, cgroup_id: u64, device_id: u64, dir: IoDirection, bytes: u64) -> bool {
        if let Some(limit) = self.cgroup_limits.get(&(cgroup_id, device_id)) {
            if limit.is_unlimited() { return false; }
            return !limit.check_bps(dir, bytes);
        }
        false
    }

    pub fn congested_devices(&self) -> Vec<u64> {
        self.devices.iter()
            .filter(|(_, d)| d.is_congested())
            .map(|(&id, _)| id)
            .collect()
    }

    pub fn most_throttled_cgroups(&self, n: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<_> = self.cgroup_io.values()
            .map(|s| (s.cgroup_id, s.throttle_ratio()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.dedup_by_key(|e| e.0);
        v.truncate(n);
        v
    }

    pub fn stats(&self) -> &BlkThrottleStats {
        &self.stats
    }
}
