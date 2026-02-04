//! Tracing-specific error types

use core::fmt;

/// Errors specific to tracing
#[derive(Debug, Clone)]
pub enum TracingError {
    /// Buffer overflow
    BufferOverflow,
    /// Invalid span
    InvalidSpan { span_id: u64 },
    /// Span not found
    SpanNotFound { span_id: u64 },
    /// Causal graph cycle detected
    CausalCycle,
    /// Replay failed
    ReplayFailed(alloc::string::String),
    /// Timestamp synchronization failed
    TimeSyncFailed,
}

impl fmt::Display for TracingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferOverflow => write!(f, "Trace buffer overflow"),
            Self::InvalidSpan { span_id } => write!(f, "Invalid span: {}", span_id),
            Self::SpanNotFound { span_id } => write!(f, "Span not found: {}", span_id),
            Self::CausalCycle => write!(f, "Causal graph cycle detected"),
            Self::ReplayFailed(msg) => write!(f, "Replay failed: {}", msg),
            Self::TimeSyncFailed => write!(f, "Timestamp synchronization failed"),
        }
    }
}
