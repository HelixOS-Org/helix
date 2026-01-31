//! Tracing configuration.

/// Configuration for tracing
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Enable tracing
    pub enabled: bool,

    /// Ring buffer size for events
    pub buffer_size: usize,

    /// Enable causal graph construction
    pub enable_causal_graph: bool,

    /// Enable deterministic replay
    pub enable_replay: bool,

    /// Maximum trace depth
    pub max_trace_depth: usize,

    /// Sample rate (1.0 = all events, 0.1 = 10% of events)
    pub sample_rate: f32,

    /// Enable timestamp synchronization
    pub sync_timestamps: bool,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 1_000_000, // 1M events
            enable_causal_graph: true,
            enable_replay: false,
            max_trace_depth: 100,
            sample_rate: 1.0,
            sync_timestamps: true,
        }
    }
}

impl TracingConfig {
    /// Minimal tracing configuration
    pub fn minimal() -> Self {
        Self {
            enabled: true,
            buffer_size: 10_000,
            enable_causal_graph: false,
            enable_replay: false,
            max_trace_depth: 20,
            sample_rate: 0.1,
            sync_timestamps: false,
        }
    }

    /// Full tracing configuration
    pub fn full() -> Self {
        Self {
            enabled: true,
            buffer_size: 10_000_000,
            enable_causal_graph: true,
            enable_replay: true,
            max_trace_depth: 500,
            sample_rate: 1.0,
            sync_timestamps: true,
        }
    }
}
