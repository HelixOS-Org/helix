//! Basic error types and enums for NEXUS

/// Kind of resource that was exhausted
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    /// Memory exhausted
    Memory,
    /// CPU budget exhausted
    Cpu,
    /// Event queue full
    EventQueue,
    /// Decision history full
    DecisionHistory,
    /// Trace buffer full
    TraceBuffer,
    /// Checkpoint storage full
    CheckpointStorage,
    /// Handler slots exhausted
    HandlerSlots,
}

/// Kind of component error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentErrorKind {
    /// Component not found
    NotFound,
    /// Component not initialized
    NotInitialized,
    /// Component is unhealthy
    Unhealthy,
    /// Component is quarantined
    Quarantined,
    /// Component failed
    Failed,
    /// Component timed out
    Timeout,
    /// Component returned invalid data
    InvalidData,
}

/// Kind of configuration error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigErrorKind {
    /// Value out of range
    OutOfRange,
    /// Invalid value
    InvalidValue,
    /// Conflicting options
    Conflict,
    /// Missing required field
    MissingField,
    /// Feature dependency not met
    FeatureDependency,
}
