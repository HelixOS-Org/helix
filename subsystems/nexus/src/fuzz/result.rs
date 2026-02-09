//! Fuzz result types

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;

// ============================================================================
// FUZZ RESULT
// ============================================================================

/// Result of a fuzz execution
#[derive(Debug, Clone)]
pub enum FuzzResult {
    /// No issues found
    Ok,
    /// Crash detected
    Crash { message: String },
    /// Timeout
    Timeout,
    /// Hang detected
    Hang,
    /// New coverage found
    NewCoverage { coverage_hash: u64 },
}

impl FuzzResult {
    /// Is this interesting?
    #[inline(always)]
    pub fn is_interesting(&self) -> bool {
        matches!(self, Self::Crash { .. } | Self::NewCoverage { .. })
    }

    /// Is this a crash?
    #[inline(always)]
    pub fn is_crash(&self) -> bool {
        matches!(self, Self::Crash { .. })
    }
}
