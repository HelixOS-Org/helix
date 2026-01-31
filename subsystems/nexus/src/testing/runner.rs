//! Test runner.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::suite::{SuiteExecution, TestSuite};

/// Test runner
pub struct TestRunner {
    /// Suites to run
    suites: Vec<TestSuite>,
    /// Stop on first failure
    fail_fast: bool,
    /// Filter by tag
    tag_filter: Option<String>,
    /// Total tests run
    total_run: AtomicU64,
    /// Total passed
    total_passed: AtomicU64,
    /// Total failed
    total_failed: AtomicU64,
}

impl TestRunner {
    /// Create a new test runner
    pub fn new() -> Self {
        Self {
            suites: Vec::new(),
            fail_fast: false,
            tag_filter: None,
            total_run: AtomicU64::new(0),
            total_passed: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
        }
    }

    /// Add a test suite
    pub fn add_suite(&mut self, suite: TestSuite) {
        self.suites.push(suite);
    }

    /// Enable fail-fast mode
    pub fn fail_fast(mut self) -> Self {
        self.fail_fast = true;
        self
    }

    /// Filter by tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag_filter = Some(tag.into());
        self
    }

    /// Run all tests
    pub fn run(&self) -> Vec<SuiteExecution> {
        let mut results = Vec::new();

        for suite in &self.suites {
            let execution = if let Some(ref tag) = self.tag_filter {
                suite.run_tagged(tag)
            } else {
                suite.run()
            };

            // Update counters
            self.total_run
                .fetch_add(execution.tests.len() as u64, Ordering::Relaxed);
            self.total_passed
                .fetch_add(execution.passed() as u64, Ordering::Relaxed);
            self.total_failed
                .fetch_add(execution.failed() as u64, Ordering::Relaxed);

            let had_failures = execution.failed() > 0;
            results.push(execution);

            if self.fail_fast && had_failures {
                break;
            }
        }

        results
    }

    /// Get total tests run
    pub fn total_run(&self) -> u64 {
        self.total_run.load(Ordering::Relaxed)
    }

    /// Get total passed
    pub fn total_passed(&self) -> u64 {
        self.total_passed.load(Ordering::Relaxed)
    }

    /// Get total failed
    pub fn total_failed(&self) -> u64 {
        self.total_failed.load(Ordering::Relaxed)
    }
}

impl Default for TestRunner {
    fn default() -> Self {
        Self::new()
    }
}
