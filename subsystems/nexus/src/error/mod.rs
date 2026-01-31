//! # NEXUS Error Types
//!
//! Comprehensive error handling for the NEXUS system.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use crate::core::ComponentId;

// ============================================================================
// RESULT TYPE
// ============================================================================

/// Result type for NEXUS operations
pub type NexusResult<T> = Result<T, NexusError>;

// ============================================================================
// MAIN ERROR ENUM
// ============================================================================

/// Main error type for NEXUS
#[derive(Debug, Clone)]
pub enum NexusError {
    /// NEXUS is already initialized
    AlreadyInitialized,

    /// NEXUS is not initialized
    NotInitialized,

    /// NEXUS is not in the correct state
    InvalidState {
        expected: &'static str,
        actual: &'static str,
    },

    /// Resource exhausted
    ResourceExhausted(ResourceKind),

    /// Operation timed out
    Timeout {
        operation: &'static str,
        timeout_ms: u64,
    },

    /// Component error
    Component {
        component: ComponentId,
        kind: ComponentErrorKind,
        message: String,
    },

    /// Prediction error
    Prediction(PredictionError),

    /// Healing error
    Healing(HealingError),

    /// Tracing error
    Tracing(TracingError),

    /// Configuration error
    Config(ConfigErrorKind),

    /// Internal error
    Internal(String),

    /// Feature not enabled
    FeatureNotEnabled(&'static str),

    /// Operation not supported
    NotSupported(&'static str),

    /// Validation failed
    ValidationFailed { field: &'static str, reason: String },
}

impl fmt::Display for NexusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyInitialized => write!(f, "NEXUS is already initialized"),
            Self::NotInitialized => write!(f, "NEXUS is not initialized"),
            Self::InvalidState { expected, actual } => {
                write!(f, "Invalid state: expected {}, got {}", expected, actual)
            },
            Self::ResourceExhausted(kind) => write!(f, "Resource exhausted: {:?}", kind),
            Self::Timeout {
                operation,
                timeout_ms,
            } => {
                write!(
                    f,
                    "Operation '{}' timed out after {}ms",
                    operation, timeout_ms
                )
            },
            Self::Component {
                component,
                kind,
                message,
            } => {
                write!(
                    f,
                    "Component {:?} error ({:?}): {}",
                    component, kind, message
                )
            },
            Self::Prediction(e) => write!(f, "Prediction error: {}", e),
            Self::Healing(e) => write!(f, "Healing error: {}", e),
            Self::Tracing(e) => write!(f, "Tracing error: {}", e),
            Self::Config(kind) => write!(f, "Configuration error: {:?}", kind),
            Self::Internal(msg) => write!(f, "Internal error: {}", msg),
            Self::FeatureNotEnabled(feature) => write!(f, "Feature not enabled: {}", feature),
            Self::NotSupported(op) => write!(f, "Operation not supported: {}", op),
            Self::ValidationFailed { field, reason } => {
                write!(f, "Validation failed for '{}': {}", field, reason)
            },
        }
    }
}

// ============================================================================
// RESOURCE KIND
// ============================================================================

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

// ============================================================================
// COMPONENT ERROR KIND
// ============================================================================

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

// ============================================================================
// PREDICTION ERROR
// ============================================================================

/// Errors specific to prediction
#[derive(Debug, Clone)]
pub enum PredictionError {
    /// Insufficient data for prediction
    InsufficientData { required: usize, available: usize },
    /// Model not trained
    ModelNotTrained,
    /// Feature extraction failed
    FeatureExtractionFailed(String),
    /// Prediction confidence too low
    LowConfidence { confidence: f32, minimum: f32 },
    /// Invalid feature
    InvalidFeature { name: String, reason: String },
}

impl fmt::Display for PredictionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InsufficientData {
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient data: need {}, have {}",
                    required, available
                )
            },
            Self::ModelNotTrained => write!(f, "Model not trained"),
            Self::FeatureExtractionFailed(msg) => {
                write!(f, "Feature extraction failed: {}", msg)
            },
            Self::LowConfidence {
                confidence,
                minimum,
            } => {
                write!(f, "Confidence {} below minimum {}", confidence, minimum)
            },
            Self::InvalidFeature { name, reason } => {
                write!(f, "Invalid feature '{}': {}", name, reason)
            },
        }
    }
}

// ============================================================================
// HEALING ERROR
// ============================================================================

/// Errors specific to healing
#[derive(Debug, Clone)]
pub enum HealingError {
    /// No checkpoint available
    NoCheckpoint,
    /// Checkpoint corrupted
    CheckpointCorrupted,
    /// Rollback failed
    RollbackFailed(String),
    /// State reconstruction failed
    ReconstructionFailed(String),
    /// Substitution failed
    SubstitutionFailed { component: String, reason: String },
    /// Maximum healing attempts exceeded
    MaxAttemptsExceeded { attempts: u32, max: u32 },
    /// Healing timed out
    Timeout { elapsed_ms: u64, max_ms: u64 },
    /// Component cannot be healed
    Unhealable(String),
    /// Healing would cause cascade failure
    CascadeRisk(String),
}

impl fmt::Display for HealingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoCheckpoint => write!(f, "No checkpoint available"),
            Self::CheckpointCorrupted => write!(f, "Checkpoint corrupted"),
            Self::RollbackFailed(msg) => write!(f, "Rollback failed: {}", msg),
            Self::ReconstructionFailed(msg) => write!(f, "State reconstruction failed: {}", msg),
            Self::SubstitutionFailed { component, reason } => {
                write!(f, "Substitution of '{}' failed: {}", component, reason)
            },
            Self::MaxAttemptsExceeded { attempts, max } => {
                write!(f, "Max healing attempts exceeded: {} > {}", attempts, max)
            },
            Self::Timeout { elapsed_ms, max_ms } => {
                write!(f, "Healing timed out: {}ms > {}ms", elapsed_ms, max_ms)
            },
            Self::Unhealable(msg) => write!(f, "Component cannot be healed: {}", msg),
            Self::CascadeRisk(msg) => write!(f, "Healing would cause cascade: {}", msg),
        }
    }
}

// ============================================================================
// TRACING ERROR
// ============================================================================

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
    ReplayFailed(String),
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

// ============================================================================
// CONFIG ERROR KIND
// ============================================================================

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

// ============================================================================
// ERROR CHAIN
// ============================================================================

/// Chain of errors for context
#[derive(Debug, Clone)]
pub struct ErrorChain {
    /// Errors in the chain (most recent first)
    pub errors: Vec<NexusError>,
    /// Context messages
    pub context: Vec<String>,
}

impl ErrorChain {
    /// Create a new error chain
    pub fn new(error: NexusError) -> Self {
        Self {
            errors: alloc::vec![error],
            context: Vec::new(),
        }
    }

    /// Add context to the chain
    pub fn context(mut self, msg: impl Into<String>) -> Self {
        self.context.push(msg.into());
        self
    }

    /// Chain another error
    pub fn chain(mut self, error: NexusError) -> Self {
        self.errors.push(error);
        self
    }

    /// Get the root cause
    pub fn root_cause(&self) -> Option<&NexusError> {
        self.errors.last()
    }

    /// Get the most recent error
    pub fn current(&self) -> Option<&NexusError> {
        self.errors.first()
    }
}

impl fmt::Display for ErrorChain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(current) = self.current() {
            write!(f, "{}", current)?;
        }
        for ctx in &self.context {
            write!(f, "\n  Context: {}", ctx)?;
        }
        if self.errors.len() > 1 {
            write!(f, "\n  Caused by:")?;
            for (i, err) in self.errors.iter().skip(1).enumerate() {
                write!(f, "\n    {}: {}", i + 1, err)?;
            }
        }
        Ok(())
    }
}

// ============================================================================
// CONVERSION TRAITS
// ============================================================================

impl From<PredictionError> for NexusError {
    fn from(e: PredictionError) -> Self {
        Self::Prediction(e)
    }
}

impl From<HealingError> for NexusError {
    fn from(e: HealingError) -> Self {
        Self::Healing(e)
    }
}

impl From<TracingError> for NexusError {
    fn from(e: TracingError) -> Self {
        Self::Tracing(e)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = NexusError::InvalidState {
            expected: "Running",
            actual: "Stopped",
        };
        let display = alloc::format!("{}", error);
        assert!(display.contains("Running"));
        assert!(display.contains("Stopped"));
    }

    #[test]
    fn test_error_chain() {
        let chain = ErrorChain::new(NexusError::NotInitialized)
            .context("While processing event")
            .chain(NexusError::Internal("Root cause".into()));

        assert_eq!(chain.errors.len(), 2);
        assert_eq!(chain.context.len(), 1);
    }

    #[test]
    fn test_prediction_error_conversion() {
        let pred_err = PredictionError::ModelNotTrained;
        let nexus_err: NexusError = pred_err.into();
        assert!(matches!(nexus_err, NexusError::Prediction(_)));
    }
}
