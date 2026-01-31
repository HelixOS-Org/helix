//! Resource Accountant
//!
//! Resource usage tracking and rate calculation.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::CgroupId;

/// Resource sample
#[derive(Debug, Clone, Copy)]
pub struct ResourceSample {
    /// Timestamp
    pub timestamp: u64,
    /// CPU usage (nanoseconds)
    pub cpu_ns: u64,
    /// Memory usage (bytes)
    pub memory: u64,
    /// I/O bytes
    pub io_bytes: u64,
}

/// Resource accountant
pub struct ResourceAccountant {
    /// Samples per cgroup
    samples: BTreeMap<CgroupId, Vec<ResourceSample>>,
    /// Maximum samples per cgroup
    max_samples: usize,
    /// Accounting interval (nanoseconds)
    interval_ns: u64,
    /// Last accounting timestamp
    last_accounting: u64,
}

impl ResourceAccountant {
    /// Create new resource accountant
    pub fn new() -> Self {
        Self {
            samples: BTreeMap::new(),
            max_samples: 100,
            interval_ns: 1_000_000_000,
            last_accounting: 0,
        }
    }

    /// Record sample
    pub fn record_sample(&mut self, cgroup: CgroupId, sample: ResourceSample) {
        let samples = self.samples.entry(cgroup).or_default();
        if samples.len() >= self.max_samples {
            samples.remove(0);
        }
        samples.push(sample);
    }

    /// Get samples for cgroup
    pub fn get_samples(&self, cgroup: CgroupId) -> Option<&[ResourceSample]> {
        self.samples.get(&cgroup).map(|v| v.as_slice())
    }

    /// Calculate CPU rate (ns/s)
    pub fn cpu_rate(&self, cgroup: CgroupId) -> Option<u64> {
        let samples = self.samples.get(&cgroup)?;
        if samples.len() < 2 {
            return None;
        }

        let latest = samples.last()?;
        let oldest = samples.first()?;

        let time_delta = latest.timestamp.saturating_sub(oldest.timestamp);
        if time_delta == 0 {
            return None;
        }

        let cpu_delta = latest.cpu_ns.saturating_sub(oldest.cpu_ns);
        Some(cpu_delta * 1_000_000_000 / time_delta)
    }

    /// Calculate memory trend (bytes/s)
    pub fn memory_trend(&self, cgroup: CgroupId) -> Option<i64> {
        let samples = self.samples.get(&cgroup)?;
        if samples.len() < 2 {
            return None;
        }

        let latest = samples.last()?;
        let oldest = samples.first()?;

        let time_delta = latest.timestamp.saturating_sub(oldest.timestamp);
        if time_delta == 0 {
            return None;
        }

        let mem_delta = latest.memory as i64 - oldest.memory as i64;
        Some(mem_delta * 1_000_000_000 / time_delta as i64)
    }

    /// Calculate I/O rate (bytes/s)
    pub fn io_rate(&self, cgroup: CgroupId) -> Option<u64> {
        let samples = self.samples.get(&cgroup)?;
        if samples.len() < 2 {
            return None;
        }

        let latest = samples.last()?;
        let oldest = samples.first()?;

        let time_delta = latest.timestamp.saturating_sub(oldest.timestamp);
        if time_delta == 0 {
            return None;
        }

        let io_delta = latest.io_bytes.saturating_sub(oldest.io_bytes);
        Some(io_delta * 1_000_000_000 / time_delta)
    }

    /// Clear samples for cgroup
    pub fn clear_samples(&mut self, cgroup: CgroupId) {
        self.samples.remove(&cgroup);
    }

    /// Set accounting interval
    pub fn set_interval(&mut self, interval_ns: u64) {
        self.interval_ns = interval_ns;
    }

    /// Get accounting interval
    pub fn interval(&self) -> u64 {
        self.interval_ns
    }

    /// Set max samples
    pub fn set_max_samples(&mut self, max: usize) {
        self.max_samples = max;
    }
}

impl Default for ResourceAccountant {
    fn default() -> Self {
        Self::new()
    }
}
