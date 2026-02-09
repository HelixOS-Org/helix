//! Resource Limits
//!
//! CPU, memory, I/O, and PIDs limits for cgroups.

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;

/// CPU limits
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuLimits {
    /// CPU shares (relative weight)
    pub shares: u64,
    /// CPU quota (microseconds per period)
    pub quota_us: i64,
    /// CPU period (microseconds)
    pub period_us: u64,
    /// CPU burst (microseconds)
    pub burst_us: u64,
    /// Maximum CPU usage (percentage * 100)
    pub max_percent: u32,
    /// Weight (cgroup v2)
    pub weight: u32,
}

impl CpuLimits {
    /// Default CPU limits
    #[inline]
    pub fn default_limits() -> Self {
        Self {
            shares: 1024,
            quota_us: -1,
            period_us: 100_000,
            burst_us: 0,
            max_percent: 10000,
            weight: 100,
        }
    }

    /// Check if quota is limited
    #[inline(always)]
    pub fn is_throttled(&self) -> bool {
        self.quota_us > 0
    }

    /// Calculate effective CPU fraction
    #[inline]
    pub fn effective_fraction(&self) -> f32 {
        if self.quota_us <= 0 || self.period_us == 0 {
            return 1.0;
        }
        (self.quota_us as f32) / (self.period_us as f32)
    }
}

/// Memory limits
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryLimits {
    /// Memory limit (bytes)
    pub limit: u64,
    /// Soft limit (bytes)
    pub soft_limit: u64,
    /// Swap limit (bytes)
    pub swap_limit: u64,
    /// Memory + swap limit (bytes)
    pub memsw_limit: u64,
    /// Kernel memory limit (bytes)
    pub kmem_limit: u64,
    /// Low memory threshold
    pub low: u64,
    /// High memory threshold
    pub high: u64,
    /// Maximum memory
    pub max: u64,
    /// OOM killer enabled
    pub oom_kill_enabled: bool,
}

impl MemoryLimits {
    /// Default memory limits (unlimited)
    pub fn default_limits() -> Self {
        Self {
            limit: u64::MAX,
            soft_limit: u64::MAX,
            swap_limit: u64::MAX,
            memsw_limit: u64::MAX,
            kmem_limit: u64::MAX,
            low: 0,
            high: u64::MAX,
            max: u64::MAX,
            oom_kill_enabled: true,
        }
    }

    /// Check if memory is limited
    #[inline(always)]
    pub fn is_limited(&self) -> bool {
        self.limit != u64::MAX || self.max != u64::MAX
    }

    /// Get effective limit
    #[inline(always)]
    pub fn effective_limit(&self) -> u64 {
        self.limit.min(self.max)
    }
}

/// I/O limits
#[derive(Debug, Clone, Default)]
pub struct IoLimits {
    /// Read BPS limit per device
    pub read_bps: LinearMap<u64, 64>,
    /// Write BPS limit per device
    pub write_bps: LinearMap<u64, 64>,
    /// Read IOPS limit per device
    pub read_iops: LinearMap<u64, 64>,
    /// Write IOPS limit per device
    pub write_iops: LinearMap<u64, 64>,
    /// Weight (1-10000)
    pub weight: u16,
    /// Latency target (microseconds)
    pub latency_target: u64,
}

impl IoLimits {
    /// Default I/O limits
    #[inline]
    pub fn default_limits() -> Self {
        Self {
            read_bps: LinearMap::new(),
            write_bps: LinearMap::new(),
            read_iops: LinearMap::new(),
            write_iops: LinearMap::new(),
            weight: 100,
            latency_target: 0,
        }
    }

    /// Check if I/O is limited
    #[inline]
    pub fn is_limited(&self) -> bool {
        !self.read_bps.is_empty()
            || !self.write_bps.is_empty()
            || !self.read_iops.is_empty()
            || !self.write_iops.is_empty()
    }
}

/// PIDs limits
#[derive(Debug, Clone, Copy, Default)]
pub struct PidsLimits {
    /// Maximum number of processes
    pub max: u64,
    /// Current number of processes
    pub current: u64,
}

impl PidsLimits {
    /// Default PIDs limits (unlimited)
    #[inline]
    pub fn default_limits() -> Self {
        Self {
            max: u64::MAX,
            current: 0,
        }
    }

    /// Check if at limit
    #[inline(always)]
    pub fn is_at_limit(&self) -> bool {
        self.max != u64::MAX && self.current >= self.max
    }

    /// Get utilization
    #[inline]
    pub fn utilization(&self) -> f32 {
        if self.max == 0 || self.max == u64::MAX {
            return 0.0;
        }
        self.current as f32 / self.max as f32
    }
}
