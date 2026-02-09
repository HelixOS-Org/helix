//! Test suite definitions.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::case::TestCase;
use super::result::{TestExecution, TestResult};
use crate::core::NexusTimestamp;

/// A collection of tests
pub struct TestSuite {
    /// Suite name
    pub name: String,
    /// Tests
    pub(crate) tests: Vec<TestCase>,
    /// Setup function
    setup: Option<Box<dyn Fn() + Send + Sync>>,
    /// Teardown function
    teardown: Option<Box<dyn Fn() + Send + Sync>>,
}

impl TestSuite {
    /// Create a new test suite
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tests: Vec::new(),
            setup: None,
            teardown: None,
        }
    }

    /// Add a test
    #[inline(always)]
    pub fn add_test(&mut self, test: TestCase) {
        self.tests.push(test);
    }

    /// Add a simple test
    #[inline(always)]
    pub fn test(
        &mut self,
        name: impl Into<String>,
        func: impl Fn() -> Result<(), String> + Send + Sync + 'static,
    ) {
        self.add_test(TestCase::new(name, func));
    }

    /// Set setup function
    #[inline(always)]
    pub fn with_setup(mut self, setup: impl Fn() + Send + Sync + 'static) -> Self {
        self.setup = Some(Box::new(setup));
        self
    }

    /// Set teardown function
    #[inline(always)]
    pub fn with_teardown(mut self, teardown: impl Fn() + Send + Sync + 'static) -> Self {
        self.teardown = Some(Box::new(teardown));
        self
    }

    /// Run all tests
    pub fn run(&self) -> SuiteExecution {
        let start = NexusTimestamp::now();
        let mut executions = Vec::new();

        // Setup
        if let Some(ref setup) = self.setup {
            setup();
        }

        // Run tests
        for test in &self.tests {
            executions.push(test.run());
        }

        // Teardown
        if let Some(ref teardown) = self.teardown {
            teardown();
        }

        let end = NexusTimestamp::now();

        SuiteExecution {
            suite_name: self.name.clone(),
            tests: executions,
            total_duration: end.duration_since(start),
        }
    }

    /// Run tests matching a tag
    pub fn run_tagged(&self, tag: &str) -> SuiteExecution {
        let start = NexusTimestamp::now();
        let mut executions = Vec::new();

        if let Some(ref setup) = self.setup {
            setup();
        }

        for test in &self.tests {
            if test.tags.iter().any(|t| t == tag) {
                executions.push(test.run());
            }
        }

        if let Some(ref teardown) = self.teardown {
            teardown();
        }

        let end = NexusTimestamp::now();

        SuiteExecution {
            suite_name: self.name.clone(),
            tests: executions,
            total_duration: end.duration_since(start),
        }
    }

    /// Get test count
    #[inline(always)]
    pub fn test_count(&self) -> usize {
        self.tests.len()
    }
}

/// Result of running a test suite
#[derive(Debug, Clone)]
pub struct SuiteExecution {
    /// Suite name
    pub suite_name: String,
    /// Individual test results
    pub tests: Vec<TestExecution>,
    /// Total duration
    pub total_duration: u64,
}

impl SuiteExecution {
    /// Count passed tests
    #[inline]
    pub fn passed(&self) -> usize {
        self.tests
            .iter()
            .filter(|t| t.result == TestResult::Passed)
            .count()
    }

    /// Count failed tests
    #[inline(always)]
    pub fn failed(&self) -> usize {
        self.tests.iter().filter(|t| t.result.is_failure()).count()
    }

    /// Count skipped tests
    #[inline]
    pub fn skipped(&self) -> usize {
        self.tests
            .iter()
            .filter(|t| t.result == TestResult::Skipped)
            .count()
    }

    /// All tests passed?
    #[inline]
    pub fn all_passed(&self) -> bool {
        self.tests
            .iter()
            .all(|t| t.result.is_success() || t.result == TestResult::Skipped)
    }

    /// Get summary
    #[inline]
    pub fn summary(&self) -> String {
        format!(
            "{}: {} passed, {} failed, {} skipped ({} cycles)",
            self.suite_name,
            self.passed(),
            self.failed(),
            self.skipped(),
            self.total_duration
        )
    }
}
