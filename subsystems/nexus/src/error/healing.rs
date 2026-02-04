//! Healing-specific error types

use alloc::string::String;
use core::fmt;

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
