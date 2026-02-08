// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic cgroup I/O Controller — BFQ-style proportional I/O bandwidth
//!
//! Manages per-cgroup I/O weight, bandwidth limits, IOPS throttling, and
//! latency targets for block devices.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// I/O scheduling policy for a cgroup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupIoPolicy {
    None,
    Weight,
    BandwidthMax,
    BandwidthMin,
    IopsMax,
    IopsMin,
    LatencyTarget,
    Proportional,
    RoundRobin,
}

/// I/O direction tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoDirection {
    Read,
    Write,
    Discard,
    Flush,
}

/// Device major:minor identification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CgroupIoDeviceId {
    pub major: u32,
    pub minor: u32,
}

impl CgroupIoDeviceId {
    pub fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub fn dev_number(&self) -> u64 {
        ((self.major as u64) << 20) | (self.minor as u64)
    }
}

/// Per-device I/O limits for a cgroup.
#[derive(Debug, Clone)]
pub struct CgroupIoDeviceLimit {
    pub device: CgroupIoDeviceId,
    pub read_bps_max: u64,
    pub write_bps_max: u64,
    pub read_iops_max: u64,
    pub write_iops_max: u64,
    pub latency_target_us: u64,
    pub weight: u32,
}

impl CgroupIoDeviceLimit {
    pub fn new(device: CgroupIoDeviceId) -> Self {
        Self {
            device,
            read_bps_max: u64::MAX,
            write_bps_max: u64::MAX,
            read_iops_max: u64::MAX,
            write_iops_max: u64::MAX,
            latency_target_us: 0,
            weight: 100,
        }
    }

    pub fn is_limited(&self) -> bool {
        self.read_bps_max != u64::MAX
            || self.write_bps_max != u64::MAX
            || self.read_iops_max != u64::MAX
            || self.write_iops_max != u64::MAX
    }
}

/// I/O accounting for a cgroup.
#[derive(Debug, Clone)]
pub struct CgroupIoAccounting {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub bytes_discarded: u64,
    pub ios_read: u64,
    pub ios_written: u64,
    pub ios_discarded: u64,
    pub throttle_count: u64,
    pub throttle_time_us: u64,
    pub avg_latency_us: u64,
    pub max_latency_us: u64,
}

impl CgroupIoAccounting {
    pub fn new() -> Self {
        Self {
            bytes_read: 0,
            bytes_written: 0,
            bytes_discarded: 0,
            ios_read: 0,
            ios_written: 0,
            ios_discarded: 0,
            throttle_count: 0,
            throttle_time_us: 0,
            avg_latency_us: 0,
            max_latency_us: 0,
        }
    }

    pub fn record_io(&mut self, direction: IoDirection, bytes: u64, latency_us: u64) {
        match direction {
            IoDirection::Read => {
                self.bytes_read += bytes;
                self.ios_read += 1;
            }
            IoDirection::Write => {
                self.bytes_written += bytes;
                self.ios_written += 1;
            }
            IoDirection::Discard => {
                self.bytes_discarded += bytes;
                self.ios_discarded += 1;
            }
            IoDirection::Flush => {}
        }
        if latency_us > self.max_latency_us {
            self.max_latency_us = latency_us;
        }
        let total_ios = self.ios_read + self.ios_written + self.ios_discarded;
        if total_ios > 0 {
            self.avg_latency_us = ((self.avg_latency_us * (total_ios - 1)) + latency_us) / total_ios;
        }
    }

    pub fn total_bytes(&self) -> u64 {
        self.bytes_read + self.bytes_written + self.bytes_discarded
    }

    pub fn total_iops(&self) -> u64 {
        self.ios_read + self.ios_written + self.ios_discarded
    }
}

/// A cgroup I/O instance.
#[derive(Debug, Clone)]
pub struct CgroupIoInstance {
    pub cgroup_id: u64,
    pub name: String,
    pub policy: CgroupIoPolicy,
    pub default_weight: u32,
    pub device_limits: BTreeMap<u64, CgroupIoDeviceLimit>,
    pub accounting: CgroupIoAccounting,
    pub children: Vec<u64>,
    pub parent_id: Option<u64>,
    pub is_active: bool,
}

impl CgroupIoInstance {
    pub fn new(cgroup_id: u64, name: String) -> Self {
        Self {
            cgroup_id,
            name,
            policy: CgroupIoPolicy::None,
            default_weight: 100,
            device_limits: BTreeMap::new(),
            accounting: CgroupIoAccounting::new(),
            children: Vec::new(),
            parent_id: None,
            is_active: true,
        }
    }

    pub fn set_device_limit(&mut self, limit: CgroupIoDeviceLimit) {
        let dev_num = limit.device.dev_number();
        self.device_limits.insert(dev_num, limit);
    }

    pub fn check_throttle(&self, device: CgroupIoDeviceId, direction: IoDirection, bps: u64) -> bool {
        let dev_num = device.dev_number();
        if let Some(limit) = self.device_limits.get(&dev_num) {
            match direction {
                IoDirection::Read => bps > limit.read_bps_max,
                IoDirection::Write => bps > limit.write_bps_max,
                _ => false,
            }
        } else {
            false
        }
    }
}

/// Statistics for the holistic cgroup I/O controller.
#[derive(Debug, Clone)]
pub struct CgroupIoStats {
    pub total_cgroups: u64,
    pub total_device_limits: u64,
    pub total_bytes_tracked: u64,
    pub total_iops_tracked: u64,
    pub throttle_events: u64,
    pub latency_violations: u64,
    pub policy_changes: u64,
}

/// Main holistic cgroup I/O controller.
pub struct HolisticCgroupIo {
    pub cgroups: BTreeMap<u64, CgroupIoInstance>,
    pub next_id: u64,
    pub stats: CgroupIoStats,
}

impl HolisticCgroupIo {
    pub fn new() -> Self {
        Self {
            cgroups: BTreeMap::new(),
            next_id: 1,
            stats: CgroupIoStats {
                total_cgroups: 0,
                total_device_limits: 0,
                total_bytes_tracked: 0,
                total_iops_tracked: 0,
                throttle_events: 0,
                latency_violations: 0,
                policy_changes: 0,
            },
        }
    }

    pub fn create_cgroup(&mut self, name: String, parent: Option<u64>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut inst = CgroupIoInstance::new(id, name);
        inst.parent_id = parent;
        if let Some(pid) = parent {
            if let Some(p) = self.cgroups.get_mut(&pid) {
                p.children.push(id);
            }
        }
        self.cgroups.insert(id, inst);
        self.stats.total_cgroups += 1;
        id
    }

    pub fn set_policy(&mut self, cgroup_id: u64, policy: CgroupIoPolicy) -> bool {
        if let Some(cg) = self.cgroups.get_mut(&cgroup_id) {
            cg.policy = policy;
            self.stats.policy_changes += 1;
            true
        } else {
            false
        }
    }

    pub fn record_io(
        &mut self,
        cgroup_id: u64,
        direction: IoDirection,
        bytes: u64,
        latency_us: u64,
    ) -> bool {
        if let Some(cg) = self.cgroups.get_mut(&cgroup_id) {
            cg.accounting.record_io(direction, bytes, latency_us);
            self.stats.total_bytes_tracked += bytes;
            self.stats.total_iops_tracked += 1;
            true
        } else {
            false
        }
    }

    pub fn cgroup_count(&self) -> usize {
        self.cgroups.len()
    }
}
