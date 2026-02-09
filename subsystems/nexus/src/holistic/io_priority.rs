//! # Holistic IO Prioritization Engine
//!
//! System-wide IO priority management with holistic context:
//! - Proportional-share IO scheduling (BFQ-inspired)
//! - Latency-targeted IO classes
//! - IO bandwidth partitioning across cgroups
//! - Read-ahead tuning based on access patterns
//! - IO merge and coalescing decisions
//! - Device saturation awareness

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IO priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IoPriorityClass {
    Idle,
    BestEffort,
    Realtime,
    System,
}

/// IO operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOpType {
    Read,
    Write,
    Sync,
    Discard,
    Flush,
}

/// IO request descriptor
#[derive(Debug, Clone)]
pub struct IoRequest {
    pub request_id: u64,
    pub process_id: u64,
    pub device_id: u32,
    pub op: IoOpType,
    pub offset: u64,
    pub size: u32,
    pub priority: IoPriorityClass,
    pub submitted_ns: u64,
    pub deadline_ns: Option<u64>,
}

/// Per-process IO weight and accounting
#[derive(Debug, Clone)]
pub struct ProcessIoWeight {
    pub process_id: u64,
    pub weight: u32, // 100-1000
    pub priority: IoPriorityClass,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub io_ops: u64,
    pub total_latency_ns: u64,
    pub deadline_misses: u64,
    pub budget_bytes: u64,
    pub budget_remaining: u64,
}

impl ProcessIoWeight {
    pub fn new(process_id: u64, weight: u32, priority: IoPriorityClass) -> Self {
        Self {
            process_id,
            weight: weight.max(100).min(1000),
            priority,
            bytes_read: 0,
            bytes_written: 0,
            io_ops: 0,
            total_latency_ns: 0,
            deadline_misses: 0,
            budget_bytes: 0,
            budget_remaining: 0,
        }
    }

    #[inline(always)]
    pub fn avg_latency_ns(&self) -> u64 {
        if self.io_ops == 0 { return 0; }
        self.total_latency_ns / self.io_ops
    }

    #[inline(always)]
    pub fn total_bytes(&self) -> u64 {
        self.bytes_read + self.bytes_written
    }

    #[inline]
    pub fn read_write_ratio(&self) -> f64 {
        let total = self.total_bytes();
        if total == 0 { return 0.5; }
        self.bytes_read as f64 / total as f64
    }

    #[inline(always)]
    pub fn budget_consumed_ratio(&self) -> f64 {
        if self.budget_bytes == 0 { return 0.0; }
        1.0 - (self.budget_remaining as f64 / self.budget_bytes as f64)
    }
}

/// Device saturation tracking
#[derive(Debug, Clone)]
pub struct DeviceSaturation {
    pub device_id: u32,
    pub queue_depth: u32,
    pub max_queue_depth: u32,
    pub iops_capacity: u64,
    pub current_iops: u64,
    pub bandwidth_bps: u64,
    pub current_bandwidth_bps: u64,
    pub avg_latency_ns: u64,
    pub p99_latency_ns: u64,
}

impl DeviceSaturation {
    pub fn new(device_id: u32, max_qd: u32, iops_cap: u64, bw_bps: u64) -> Self {
        Self {
            device_id,
            queue_depth: 0,
            max_queue_depth: max_qd,
            iops_capacity: iops_cap,
            current_iops: 0,
            bandwidth_bps: bw_bps,
            current_bandwidth_bps: 0,
            avg_latency_ns: 0,
            p99_latency_ns: 0,
        }
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.iops_capacity == 0 { return 0.0; }
        self.current_iops as f64 / self.iops_capacity as f64
    }

    #[inline(always)]
    pub fn bandwidth_utilization(&self) -> f64 {
        if self.bandwidth_bps == 0 { return 0.0; }
        self.current_bandwidth_bps as f64 / self.bandwidth_bps as f64
    }

    #[inline(always)]
    pub fn is_saturated(&self) -> bool {
        self.utilization() > 0.9 || self.queue_depth >= self.max_queue_depth
    }

    #[inline(always)]
    pub fn queue_pressure(&self) -> f64 {
        if self.max_queue_depth == 0 { return 0.0; }
        self.queue_depth as f64 / self.max_queue_depth as f64
    }
}

/// Readahead tuning state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ReadaheadState {
    pub device_id: u32,
    pub process_id: u64,
    pub current_ra_pages: u32,
    pub max_ra_pages: u32,
    pub sequential_hits: u64,
    pub random_hits: u64,
    pub wasted_ra_pages: u64,
}

impl ReadaheadState {
    pub fn new(device_id: u32, pid: u64, max_pages: u32) -> Self {
        Self {
            device_id,
            process_id: pid,
            current_ra_pages: max_pages / 4,
            max_ra_pages: max_pages,
            sequential_hits: 0,
            random_hits: 0,
            wasted_ra_pages: 0,
        }
    }

    #[inline]
    pub fn sequential_ratio(&self) -> f64 {
        let total = self.sequential_hits + self.random_hits;
        if total == 0 { return 0.0; }
        self.sequential_hits as f64 / total as f64
    }

    /// Adapt readahead based on pattern
    pub fn adapt(&mut self) {
        let ratio = self.sequential_ratio();
        if ratio > 0.8 {
            // Highly sequential — increase readahead
            self.current_ra_pages = (self.current_ra_pages * 2).min(self.max_ra_pages);
        } else if ratio < 0.3 {
            // Mostly random — minimize readahead
            self.current_ra_pages = (self.current_ra_pages / 2).max(4);
        }

        // Reduce if too many wasted pages
        let waste_ratio = if self.sequential_hits > 0 {
            self.wasted_ra_pages as f64 / (self.sequential_hits * self.current_ra_pages as u64) as f64
        } else { 0.0 };
        if waste_ratio > 0.5 {
            self.current_ra_pages = (self.current_ra_pages * 3 / 4).max(4);
        }
    }
}

/// Holistic IO Priority stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticIoPriorityStats {
    pub total_processes: usize,
    pub total_devices: usize,
    pub saturated_devices: usize,
    pub total_io_ops: u64,
    pub total_deadline_misses: u64,
    pub avg_global_latency_ns: u64,
}

/// Holistic IO Prioritization Engine
pub struct HolisticIoPriority {
    processes: BTreeMap<u64, ProcessIoWeight>,
    devices: BTreeMap<u32, DeviceSaturation>,
    readahead: BTreeMap<u64, ReadaheadState>, // key: fnv(device_id, pid)
    stats: HolisticIoPriorityStats,
}

impl HolisticIoPriority {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            devices: BTreeMap::new(),
            readahead: BTreeMap::new(),
            stats: HolisticIoPriorityStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pw: ProcessIoWeight) {
        self.processes.insert(pw.process_id, pw);
    }

    #[inline(always)]
    pub fn register_device(&mut self, ds: DeviceSaturation) {
        self.devices.insert(ds.device_id, ds);
    }

    /// Submit IO and decide priority
    pub fn submit_io(&mut self, req: &IoRequest, now_ns: u64) -> IoPriorityClass {
        let base_prio = if let Some(proc) = self.processes.get(&req.process_id) {
            proc.priority
        } else { IoPriorityClass::BestEffort };

        // Check device saturation
        let saturated = self.devices.get(&req.device_id)
            .map(|d| d.is_saturated())
            .unwrap_or(false);

        if saturated && base_prio == IoPriorityClass::BestEffort {
            // Downgrade to idle if device is saturated and not high-priority
            return IoPriorityClass::Idle;
        }

        // Check deadline urgency
        if let Some(deadline) = req.deadline_ns {
            let remaining = deadline.saturating_sub(now_ns);
            if remaining < 1_000_000 {
                return IoPriorityClass::Realtime;
            }
        }

        base_prio
    }

    /// Account for completed IO
    pub fn complete_io(&mut self, req: &IoRequest, latency_ns: u64) {
        if let Some(proc) = self.processes.get_mut(&req.process_id) {
            proc.io_ops += 1;
            proc.total_latency_ns += latency_ns;
            match req.op {
                IoOpType::Read => proc.bytes_read += req.size as u64,
                IoOpType::Write => proc.bytes_written += req.size as u64,
                _ => {}
            }
            if let Some(deadline) = req.deadline_ns {
                let completion = req.submitted_ns + latency_ns;
                if completion > deadline {
                    proc.deadline_misses += 1;
                }
            }
            proc.budget_remaining = proc.budget_remaining.saturating_sub(req.size as u64);
        }
        self.recompute();
    }

    fn recompute(&mut self) {
        self.stats.total_processes = self.processes.len();
        self.stats.total_devices = self.devices.len();
        self.stats.saturated_devices = self.devices.values().filter(|d| d.is_saturated()).count();
        self.stats.total_io_ops = self.processes.values().map(|p| p.io_ops).sum();
        self.stats.total_deadline_misses = self.processes.values().map(|p| p.deadline_misses).sum();
        let total_lat: u64 = self.processes.values().map(|p| p.total_latency_ns).sum();
        let total_ops: u64 = self.processes.values().map(|p| p.io_ops).sum();
        self.stats.avg_global_latency_ns = if total_ops > 0 { total_lat / total_ops } else { 0 };
    }

    #[inline(always)]
    pub fn process(&self, id: u64) -> Option<&ProcessIoWeight> { self.processes.get(&id) }
    #[inline(always)]
    pub fn device(&self, id: u32) -> Option<&DeviceSaturation> { self.devices.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &HolisticIoPriorityStats { &self.stats }
}
