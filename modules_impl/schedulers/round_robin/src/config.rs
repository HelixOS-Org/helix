//! # Round-Robin Scheduler Configuration

/// Configuration for the round-robin scheduler
#[derive(Debug, Clone)]
pub struct RoundRobinConfig {
    /// Default time slice for normal threads (in nanoseconds)
    pub default_time_slice_ns: u64,
    /// Minimum time slice
    pub min_time_slice_ns: u64,
    /// Maximum time slice
    pub max_time_slice_ns: u64,
    /// Time slice calculation based on priority
    pub priority_time_scaling: bool,
    /// Number of priority levels
    pub priority_levels: usize,
    /// Enable load balancing for SMP
    pub load_balancing: bool,
    /// Load balance interval (in ticks)
    pub load_balance_interval: u64,
    /// Idle thread priority boost
    pub interactive_boost: bool,
}

impl RoundRobinConfig {
    /// Default time slice: 10ms
    pub const DEFAULT_TIME_SLICE_NS: u64 = 10_000_000;
    /// Minimum time slice: 1ms
    pub const MIN_TIME_SLICE_NS: u64 = 1_000_000;
    /// Maximum time slice: 100ms
    pub const MAX_TIME_SLICE_NS: u64 = 100_000_000;

    /// Create default configuration
    pub fn new() -> Self {
        Self {
            default_time_slice_ns: Self::DEFAULT_TIME_SLICE_NS,
            min_time_slice_ns: Self::MIN_TIME_SLICE_NS,
            max_time_slice_ns: Self::MAX_TIME_SLICE_NS,
            priority_time_scaling: true,
            priority_levels: 140,
            load_balancing: true,
            load_balance_interval: 100,
            interactive_boost: true,
        }
    }

    /// Create a simple configuration (no priority scaling)
    pub fn simple() -> Self {
        Self {
            priority_time_scaling: false,
            load_balancing: false,
            interactive_boost: false,
            ..Self::new()
        }
    }

    /// Create a real-time configuration
    pub fn realtime() -> Self {
        Self {
            default_time_slice_ns: 1_000_000, // 1ms
            min_time_slice_ns: 100_000,       // 100µs
            max_time_slice_ns: 10_000_000,    // 10ms
            priority_time_scaling: true,
            priority_levels: 140,
            load_balancing: true,
            load_balance_interval: 10,
            interactive_boost: false,
        }
    }

    /// Get time slice for a priority level
    pub fn time_slice_for_priority(&self, priority: u8) -> u64 {
        if !self.priority_time_scaling {
            return self.default_time_slice_ns;
        }

        // Higher priority (lower number) = longer time slice
        let scale = if priority < 100 {
            // Real-time: longer slices
            2.0 - (priority as f64 / 100.0)
        } else {
            // Normal: shorter slices for lower priority
            1.0 - ((priority - 100) as f64 / 80.0)
        };

        let slice = (self.default_time_slice_ns as f64 * scale) as u64;
        slice.clamp(self.min_time_slice_ns, self.max_time_slice_ns)
    }
}

impl Default for RoundRobinConfig {
    fn default() -> Self {
        Self::new()
    }
}
