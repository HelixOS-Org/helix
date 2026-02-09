//! Test case definitions.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::result::{TestExecution, TestResult};
use crate::core::NexusTimestamp;

/// A test case
pub struct TestCase {
    /// Test name
    pub name: String,
    /// Test function
    pub func: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
    /// Timeout (cycles)
    pub timeout: Option<u64>,
    /// Should ignore failures
    pub ignore: bool,
    /// Tags
    pub tags: Vec<String>,
}

impl TestCase {
    /// Create a new test case
    pub fn new(
        name: impl Into<String>,
        func: impl Fn() -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            func: Box::new(func),
            timeout: None,
            ignore: false,
            tags: Vec::new(),
        }
    }

    /// Set timeout
    #[inline(always)]
    pub fn with_timeout(mut self, cycles: u64) -> Self {
        self.timeout = Some(cycles);
        self
    }

    /// Mark as ignored
    #[inline(always)]
    pub fn ignore(mut self) -> Self {
        self.ignore = true;
        self
    }

    /// Add a tag
    #[inline(always)]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Run the test
    pub fn run(&self) -> TestExecution {
        let start = NexusTimestamp::now();

        if self.ignore {
            return TestExecution {
                name: self.name.clone(),
                result: TestResult::Skipped,
                duration: 0,
                error: None,
            };
        }

        // Run the test
        let result = (self.func)();

        let end = NexusTimestamp::now();
        let duration = end.duration_since(start);

        // Check timeout
        if let Some(timeout) = self.timeout {
            if duration > timeout {
                return TestExecution {
                    name: self.name.clone(),
                    result: TestResult::Timeout,
                    duration,
                    error: Some(format!("Test exceeded timeout of {} cycles", timeout)),
                };
            }
        }

        match result {
            Ok(()) => TestExecution {
                name: self.name.clone(),
                result: TestResult::Passed,
                duration,
                error: None,
            },
            Err(e) => TestExecution {
                name: self.name.clone(),
                result: TestResult::Failed,
                duration,
                error: Some(e),
            },
        }
    }
}
