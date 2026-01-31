//! Configuration errors.

/// Configuration errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    /// Memory budget is too low
    MemoryBudgetTooLow,
    /// CPU budget is too high
    CpuBudgetTooHigh,
    /// Decision time is too short
    DecisionTimeTooShort,
    /// Event queue is too small
    EventQueueTooSmall,
    /// Invalid configuration
    Invalid,
}
