//! Chaos testing configuration.

/// Configuration for chaos testing
#[derive(Debug, Clone)]
pub struct ChaosConfig {
    /// Enable chaos testing
    pub enabled: bool,

    /// Probability of injecting a fault (0.0 - 1.0)
    pub fault_probability: f32,

    /// Enable memory fault injection
    pub inject_memory_faults: bool,

    /// Enable CPU fault injection
    pub inject_cpu_faults: bool,

    /// Enable I/O fault injection
    pub inject_io_faults: bool,

    /// Enable latency injection
    pub inject_latency: bool,

    /// Maximum injected latency in microseconds
    pub max_latency_us: u64,

    /// Fault injection seed (for reproducibility)
    pub seed: u64,
}

impl Default for ChaosConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            fault_probability: 0.001, // 0.1%
            inject_memory_faults: true,
            inject_cpu_faults: false,
            inject_io_faults: true,
            inject_latency: true,
            max_latency_us: 1000,
            seed: 0,
        }
    }
}

impl ChaosConfig {
    /// Disable chaos testing
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Aggressive chaos testing
    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            fault_probability: 0.01, // 1%
            inject_memory_faults: true,
            inject_cpu_faults: true,
            inject_io_faults: true,
            inject_latency: true,
            max_latency_us: 10_000,
            seed: 42,
        }
    }
}
