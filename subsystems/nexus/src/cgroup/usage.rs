//! Resource Usage
//!
//! CPU, memory, and I/O usage statistics.

/// CPU usage statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuUsage {
    /// Total CPU time used (nanoseconds)
    pub usage_ns: u64,
    /// User CPU time (nanoseconds)
    pub user_ns: u64,
    /// System CPU time (nanoseconds)
    pub system_ns: u64,
    /// Number of throttle periods
    pub nr_throttled: u64,
    /// Throttled time (nanoseconds)
    pub throttled_ns: u64,
    /// Number of periods
    pub nr_periods: u64,
    /// Number of bursts
    pub nr_bursts: u64,
    /// Burst time (nanoseconds)
    pub burst_ns: u64,
}

impl CpuUsage {
    /// Calculate throttle percentage
    pub fn throttle_percent(&self) -> f32 {
        if self.nr_periods == 0 {
            return 0.0;
        }
        (self.nr_throttled as f32 / self.nr_periods as f32) * 100.0
    }
}

/// Memory pressure level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MemoryPressure {
    /// No pressure
    #[default]
    None,
    /// Low pressure
    Low,
    /// Medium pressure
    Medium,
    /// Critical pressure
    Critical,
}

/// Memory usage statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryUsage {
    /// Current memory usage (bytes)
    pub usage: u64,
    /// Maximum memory usage (bytes)
    pub max_usage: u64,
    /// Swap usage (bytes)
    pub swap_usage: u64,
    /// Kernel memory usage (bytes)
    pub kmem_usage: u64,
    /// Cache (bytes)
    pub cache: u64,
    /// RSS (bytes)
    pub rss: u64,
    /// Number of OOM events
    pub oom_events: u64,
    /// Number of OOM kills
    pub oom_kills: u64,
    /// Memory pressure level
    pub pressure_level: MemoryPressure,
}

impl MemoryUsage {
    /// Calculate usage percentage
    pub fn usage_percent(&self, limit: u64) -> f32 {
        if limit == 0 || limit == u64::MAX {
            return 0.0;
        }
        (self.usage as f32 / limit as f32) * 100.0
    }
}

/// I/O usage statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct IoUsage {
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Read operations
    pub read_ops: u64,
    /// Write operations
    pub write_ops: u64,
    /// Discard bytes
    pub bytes_discarded: u64,
    /// Discard operations
    pub discard_ops: u64,
}

impl IoUsage {
    /// Total bytes
    pub fn total_bytes(&self) -> u64 {
        self.bytes_read + self.bytes_written
    }

    /// Total operations
    pub fn total_ops(&self) -> u64 {
        self.read_ops + self.write_ops
    }
}
