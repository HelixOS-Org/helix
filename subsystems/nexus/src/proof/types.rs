//! Core verification types
//!
//! This module defines fundamental types for formal verification including
//! property types and verification outcomes.

#![allow(dead_code)]

/// Type of property
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyType {
    /// Safety property - bad things don't happen
    Safety,
    /// Liveness property - good things eventually happen
    Liveness,
    /// Invariant - always true
    Invariant,
    /// Progress - system makes progress
    Progress,
    /// Fairness - resources are fairly distributed
    Fairness,
    /// Memory safety
    MemorySafety,
    /// Concurrency property
    Concurrency,
}

/// Verification outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationOutcome {
    /// Property verified
    Verified,
    /// Property falsified
    Falsified,
    /// Unknown (timeout or resource limit)
    Unknown,
    /// Property is vacuously true
    Vacuous,
    /// Error during verification
    Error,
}

impl VerificationOutcome {
    /// Is this a successful verification?
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Verified | Self::Vacuous)
    }

    /// Is this a failure?
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Falsified)
    }

    /// Is this inconclusive?
    pub fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown | Self::Error)
    }
}
