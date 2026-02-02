//! # Testing Framework
//!
//! Comprehensive testing framework for kernel validation.
//!
//! ## Key Features
//!
//! - **Unit Tests**: Low-level component tests
//! - **Integration Tests**: Multi-component tests
//! - **Property Tests**: Randomized property-based testing
//! - **Stress Tests**: Load and endurance testing

#![allow(dead_code)]

extern crate alloc;

mod assertions;
mod case;
mod result;
mod runner;
mod suite;

// Re-export result types
// Re-export assertions
pub use assertions::{
    assert_eq, assert_err, assert_false, assert_in_range, assert_ne, assert_ok, assert_true,
};
// Re-export case
pub use case::TestCase;
pub use result::{TestExecution, TestResult};
// Re-export runner
pub use runner::TestRunner;
// Re-export suite
pub use suite::{SuiteExecution, TestSuite};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_case() {
        let test = TestCase::new("simple_test", || Ok(()));
        let result = test.run();
        assert_eq!(result.result, TestResult::Passed);
    }

    #[test]
    fn test_failing_test() {
        let test = TestCase::new("failing_test", || Err("Test failed".into()));
        let result = test.run();
        assert_eq!(result.result, TestResult::Failed);
    }

    #[test]
    fn test_skipped_test() {
        let test = TestCase::new("skipped_test", || Ok(())).ignore();
        let result = test.run();
        assert_eq!(result.result, TestResult::Skipped);
    }

    #[test]
    fn test_suite() {
        let mut suite = TestSuite::new("test_suite");
        suite.test("test1", || Ok(()));
        suite.test("test2", || Ok(()));
        suite.test("test3", || Err("Fail".into()));

        let result = suite.run();
        assert_eq!(result.passed(), 2);
        assert_eq!(result.failed(), 1);
    }

    #[test]
    fn test_assertions() {
        assert!(assertions::assert_eq(1, 1).is_ok());
        assert!(assertions::assert_eq(1, 2).is_err());
        assert!(assertions::assert_ne(1, 2).is_ok());
        assert!(assertions::assert_true(true, "").is_ok());
        assert!(assertions::assert_false(false, "").is_ok());
        assert!(assertions::assert_in_range(5, 1, 10).is_ok());
        assert!(assertions::assert_in_range(15, 1, 10).is_err());
    }
}
