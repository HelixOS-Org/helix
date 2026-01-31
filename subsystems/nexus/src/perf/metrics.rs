//! Performance Metrics
//!
//! IPC, cache rates, and derived performance metrics.

// ============================================================================
// DERIVED METRICS
// ============================================================================

/// IPC (Instructions Per Cycle)
#[derive(Debug, Clone, Copy)]
pub struct Ipc(pub f64);

impl Ipc {
    /// Calculate from instructions and cycles
    pub fn calculate(instructions: u64, cycles: u64) -> Self {
        if cycles == 0 {
            return Self(0.0);
        }
        Self(instructions as f64 / cycles as f64)
    }

    /// Is good (>= 1.0 typically good for most workloads)
    pub fn is_good(&self) -> bool {
        self.0 >= 1.0
    }

    /// Rating
    pub fn rating(&self) -> &'static str {
        if self.0 >= 2.0 {
            "Excellent"
        } else if self.0 >= 1.0 {
            "Good"
        } else if self.0 >= 0.5 {
            "Fair"
        } else {
            "Poor"
        }
    }
}

/// Cache miss rate
#[derive(Debug, Clone, Copy)]
pub struct CacheMissRate(pub f64);

impl CacheMissRate {
    /// Calculate from references and misses
    pub fn calculate(references: u64, misses: u64) -> Self {
        if references == 0 {
            return Self(0.0);
        }
        Self(misses as f64 / references as f64 * 100.0)
    }

    /// Is good (<= 5% typically good)
    pub fn is_good(&self) -> bool {
        self.0 <= 5.0
    }
}

/// Branch misprediction rate
#[derive(Debug, Clone, Copy)]
pub struct BranchMissRate(pub f64);

impl BranchMissRate {
    /// Calculate
    pub fn calculate(branches: u64, misses: u64) -> Self {
        if branches == 0 {
            return Self(0.0);
        }
        Self(misses as f64 / branches as f64 * 100.0)
    }

    /// Is good (<= 2% typically good)
    pub fn is_good(&self) -> bool {
        self.0 <= 2.0
    }
}

// ============================================================================
// PERF METRICS
// ============================================================================

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct PerfMetrics {
    /// CPU cycles
    pub cycles: u64,
    /// Instructions
    pub instructions: u64,
    /// IPC
    pub ipc: Option<f64>,
    /// Cache references
    pub cache_refs: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Cache miss rate
    pub cache_miss_rate: Option<f64>,
    /// Branch instructions
    pub branches: u64,
    /// Branch misses
    pub branch_misses: u64,
    /// Branch miss rate
    pub branch_miss_rate: Option<f64>,
    /// Context switches
    pub context_switches: u64,
    /// Page faults
    pub page_faults: u64,
    /// Duration (ns)
    pub duration_ns: u64,
}

impl PerfMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate derived metrics
    pub fn calculate_derived(&mut self) {
        if self.cycles > 0 {
            self.ipc = Some(self.instructions as f64 / self.cycles as f64);
        }
        if self.cache_refs > 0 {
            self.cache_miss_rate = Some(self.cache_misses as f64 / self.cache_refs as f64 * 100.0);
        }
        if self.branches > 0 {
            self.branch_miss_rate = Some(self.branch_misses as f64 / self.branches as f64 * 100.0);
        }
    }

    /// Instructions per second
    pub fn instructions_per_second(&self) -> f64 {
        if self.duration_ns == 0 {
            return 0.0;
        }
        self.instructions as f64 / (self.duration_ns as f64 / 1_000_000_000.0)
    }
}
