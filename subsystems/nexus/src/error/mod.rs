//! # NEXUS Error Types
//!
//! Comprehensive error handling for the NEXUS system.

extern crate alloc;

use alloc::string::String;
use core::fmt;

use crate::core::ComponentId;

// ============================================================================
// SUBMODULES
// ============================================================================

mod chain;
mod healing;
mod prediction;
mod tracing;
mod types;

pub use chain::ErrorChain;
pub use healing::HealingError;
pub use prediction::PredictionError;
pub use tracing::TracingError;
pub use types::{ComponentErrorKind, ConfigErrorKind, ResourceKind};

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
        let chain = ErrorChain::new(NexusError::not_initialized())
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
