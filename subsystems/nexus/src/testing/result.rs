//! Test result types.

use alloc::string::String;

/// Result of a test
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestResult {
    /// Test passed
    Passed,
    /// Test failed
    Failed,
    /// Test was skipped
    Skipped,
    /// Test timed out
    Timeout,
    /// Test panicked
    Panicked,
}

impl TestResult {
    /// Is this a success?
    #[inline(always)]
    pub fn is_success(&self) -> bool {
        *self == Self::Passed
    }

    /// Is this a failure?
    #[inline(always)]
    pub fn is_failure(&self) -> bool {
        matches!(self, Self::Failed | Self::Timeout | Self::Panicked)
    }

    /// Get display name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Passed => "PASSED",
            Self::Failed => "FAILED",
            Self::Skipped => "SKIPPED",
            Self::Timeout => "TIMEOUT",
            Self::Panicked => "PANICKED",
        }
    }
}

/// Result of running a test
#[derive(Debug, Clone)]
pub struct TestExecution {
    /// Test name
    pub name: String,
    /// Result
    pub result: TestResult,
    /// Duration (cycles)
    pub duration: u64,
    /// Error message (if failed)
    pub error: Option<String>,
}
