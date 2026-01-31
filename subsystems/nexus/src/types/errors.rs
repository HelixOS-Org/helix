//! NEXUS Error Types
//!
//! Error types for the cognitive system.

#![allow(dead_code)]

use alloc::string::String;
use super::identifiers::DomainId;
use super::temporal::Timestamp;

// ============================================================================
// NEXUS ERROR
// ============================================================================

/// NEXUS-specific error
#[derive(Debug, Clone)]
pub struct NexusError {
    /// Error code
    pub code: ErrorCode,
    /// Error message
    pub message: String,
    /// Source domain
    pub domain: Option<DomainId>,
    /// Timestamp
    pub timestamp: Timestamp,
}

impl NexusError {
    /// Create new error
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            domain: None,
            timestamp: Timestamp::now(),
        }
    }

    /// With domain
    pub fn with_domain(mut self, domain: DomainId) -> Self {
        self.domain = Some(domain);
        self
    }

    /// Is recoverable?
    pub fn is_recoverable(&self) -> bool {
        self.code.is_recoverable()
    }

    /// Is critical?
    pub fn is_critical(&self) -> bool {
        self.code.is_critical()
    }
}

impl core::fmt::Display for NexusError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

// ============================================================================
// ERROR CODE
// ============================================================================

/// Error codes organized by domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // ============== General ==============
    /// Unknown error
    Unknown,
    /// Not initialized
    NotInitialized,
    /// Already initialized
    AlreadyInitialized,
    /// Invalid state
    InvalidState,
    /// Invalid argument
    InvalidArgument,
    /// Not found
    NotFound,
    /// Already exists
    AlreadyExists,
    /// Timeout
    Timeout,
    /// Cancelled
    Cancelled,
    /// Permission denied
    PermissionDenied,
    /// Not supported
    NotSupported,
    /// Internal error
    Internal,

    // ============== Perception (SENSE) ==============
    /// Probe failure
    ProbeFailure,
    /// Sensor unavailable
    SensorUnavailable,
    /// Data corrupted
    DataCorrupted,
    /// Sampling error
    SamplingError,
    /// Stream overflow
    StreamOverflow,

    // ============== Comprehension (UNDERSTAND) ==============
    /// Parse error
    ParseError,
    /// Pattern not recognized
    PatternNotRecognized,
    /// Model failure
    ModelFailure,
    /// Feature extraction failed
    FeatureExtractionFailed,
    /// Classification failed
    ClassificationFailed,

    // ============== Reasoning (REASON) ==============
    /// Inference failure
    InferenceFailure,
    /// Contradiction detected
    ContradictionDetected,
    /// Simulation failed
    SimulationFailed,
    /// Causal loop detected
    CausalLoopDetected,
    /// Insufficient evidence
    InsufficientEvidence,

    // ============== Decision (DECIDE) ==============
    /// Policy violation
    PolicyViolation,
    /// Conflict unresolved
    ConflictUnresolved,
    /// No valid option
    NoValidOption,
    /// Approval required
    ApprovalRequired,
    /// Deadline missed
    DeadlineMissed,

    // ============== Execution (ACT) ==============
    /// Action failed
    ActionFailed,
    /// Validation failed
    ValidationFailed,
    /// Transaction aborted
    TransactionAborted,
    /// Rollback failed
    RollbackFailed,
    /// Rate limited
    RateLimited,
    /// Effector unavailable
    EffectorUnavailable,

    // ============== Memory ==============
    /// Memory full
    MemoryFull,
    /// Consolidation failed
    ConsolidationFailed,
    /// Persistence failed
    PersistenceFailed,
    /// Recall failed
    RecallFailed,
    /// Memory corrupted
    MemoryCorrupted,

    // ============== Reflection (REFLECT) ==============
    /// Calibration failed
    CalibrationFailed,
    /// Diagnosis failed
    DiagnosisFailed,
    /// Introspection failed
    IntrospectionFailed,
    /// Evolution failed
    EvolutionFailed,
}

impl ErrorCode {
    /// Is this error recoverable?
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Timeout
                | Self::RateLimited
                | Self::ConflictUnresolved
                | Self::ApprovalRequired
                | Self::DeadlineMissed
        )
    }

    /// Is this error critical?
    pub const fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::DataCorrupted
                | Self::MemoryCorrupted
                | Self::RollbackFailed
                | Self::TransactionAborted
                | Self::ContradictionDetected
        )
    }

    /// Get error category
    pub const fn category(&self) -> ErrorCategory {
        match self {
            Self::Unknown
            | Self::NotInitialized
            | Self::AlreadyInitialized
            | Self::InvalidState
            | Self::InvalidArgument
            | Self::NotFound
            | Self::AlreadyExists
            | Self::Timeout
            | Self::Cancelled
            | Self::PermissionDenied
            | Self::NotSupported
            | Self::Internal => ErrorCategory::General,

            Self::ProbeFailure
            | Self::SensorUnavailable
            | Self::DataCorrupted
            | Self::SamplingError
            | Self::StreamOverflow => ErrorCategory::Perception,

            Self::ParseError
            | Self::PatternNotRecognized
            | Self::ModelFailure
            | Self::FeatureExtractionFailed
            | Self::ClassificationFailed => ErrorCategory::Comprehension,

            Self::InferenceFailure
            | Self::ContradictionDetected
            | Self::SimulationFailed
            | Self::CausalLoopDetected
            | Self::InsufficientEvidence => ErrorCategory::Reasoning,

            Self::PolicyViolation
            | Self::ConflictUnresolved
            | Self::NoValidOption
            | Self::ApprovalRequired
            | Self::DeadlineMissed => ErrorCategory::Decision,

            Self::ActionFailed
            | Self::ValidationFailed
            | Self::TransactionAborted
            | Self::RollbackFailed
            | Self::RateLimited
            | Self::EffectorUnavailable => ErrorCategory::Execution,

            Self::MemoryFull
            | Self::ConsolidationFailed
            | Self::PersistenceFailed
            | Self::RecallFailed
            | Self::MemoryCorrupted => ErrorCategory::Memory,

            Self::CalibrationFailed
            | Self::DiagnosisFailed
            | Self::IntrospectionFailed
            | Self::EvolutionFailed => ErrorCategory::Reflection,
        }
    }
}

/// Error category (by domain)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    General,
    Perception,
    Comprehension,
    Reasoning,
    Decision,
    Execution,
    Memory,
    Reflection,
}

impl ErrorCategory {
    /// Get category name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Perception => "perception",
            Self::Comprehension => "comprehension",
            Self::Reasoning => "reasoning",
            Self::Decision => "decision",
            Self::Execution => "execution",
            Self::Memory => "memory",
            Self::Reflection => "reflection",
        }
    }
}

// ============================================================================
// RESULT TYPE
// ============================================================================

/// NEXUS Result type
pub type NexusResult<T> = Result<T, NexusError>;

// ============================================================================
// HELPER MACROS
// ============================================================================

/// Create a NexusError quickly
#[macro_export]
macro_rules! nexus_error {
    ($code:expr, $msg:expr) => {
        $crate::types::errors::NexusError::new($code, $msg)
    };
    ($code:expr, $fmt:expr, $($arg:tt)*) => {
        $crate::types::errors::NexusError::new($code, alloc::format!($fmt, $($arg)*))
    };
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = NexusError::new(ErrorCode::Timeout, "Operation timed out");
        assert_eq!(err.code, ErrorCode::Timeout);
        assert!(err.is_recoverable());
    }

    #[test]
    fn test_error_category() {
        assert_eq!(ErrorCode::ProbeFailure.category(), ErrorCategory::Perception);
        assert_eq!(ErrorCode::PolicyViolation.category(), ErrorCategory::Decision);
    }

    #[test]
    fn test_critical_errors() {
        assert!(ErrorCode::DataCorrupted.is_critical());
        assert!(!ErrorCode::Timeout.is_critical());
    }
}
