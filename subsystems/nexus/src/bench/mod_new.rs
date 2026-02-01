//! # Benchmarking Framework
//!
//! Micro and macro benchmarking for kernel performance measurement.
//!
//! ## Key Features
//!
//! - **Micro-benchmarks**: Nanosecond precision timing
//! - **Macro-benchmarks**: End-to-end performance tests
//! - **Statistical Analysis**: Mean, median, std dev, percentiles
//! - **Regression Detection**: Track performance over time
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `result`: Benchmark result and statistics
//! - `benchmark`: Benchmark definition and execution
//! - `suite`: Benchmark suite
//! - `regression`: Regression tracking and quick utilities

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

// Submodules
pub mod benchmark;
pub mod regression;
pub mod result;
pub mod suite;

// Re-export result
pub use result::BenchmarkResult;

// Re-export benchmark
pub use benchmark::Benchmark;

// Re-export suite
pub use suite::{BenchmarkSuite, SuiteResult};

// Re-export regression
pub use regression::{measure, quick_bench, RegressionReport, RegressionTracker};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result() {
        let samples = vec![100, 110, 105, 95, 120, 90, 115, 100, 108, 102];
        let result = BenchmarkResult::from_samples("test", &samples);

        assert_eq!(result.iterations, 10);
        assert!(result.mean > 0.0);
        assert!(result.median > 0.0);
        assert!(result.min == 90);
        assert!(result.max == 120);
    }

    #[test]
    fn test_benchmark() {
        let bench = Benchmark::new("simple", || {
            let mut x = 0u64;
            for i in 0..100 {
                x += i;
            }
            core::hint::black_box(x);
        })
        .with_min_iterations(100);

        let result = bench.run();
        assert!(result.iterations >= 100);
        assert!(result.mean > 0.0);
    }

    #[test]
    fn test_suite() {
        let mut suite = BenchmarkSuite::new("test_suite");

        suite.bench("add", || {
            core::hint::black_box(1 + 1);
        });

        suite.bench("mul", || {
            core::hint::black_box(2 * 2);
        });

        let result = suite.run();
        assert_eq!(result.results.len(), 2);
    }

    #[test]
    fn test_regression_tracker() {
        let mut tracker = RegressionTracker::new(0.1); // 10% threshold

        // Set baseline
        let baseline = BenchmarkResult {
            name: "test".into(),
            iterations: 100,
            total_time: 10000,
            mean: 100.0,
            median: 100.0,
            std_dev: 5.0,
            min: 90,
            max: 110,
            p95: 108.0,
            p99: 109.0,
            throughput: 10_000_000.0,
        };
        tracker.set_baseline(baseline);

        // Test no regression
        let current = BenchmarkResult {
            name: "test".into(),
            mean: 105.0,
            ..Default::default()
        };
        assert!(tracker.check(&current).is_none());

        // Test regression
        let current = BenchmarkResult {
            name: "test".into(),
            mean: 120.0, // 20% slower
            ..Default::default()
        };
        assert!(tracker.check(&current).is_some());
    }

    #[test]
    fn test_quick_bench() {
        let result = quick_bench("quick", 50, || {
            core::hint::black_box(42);
        });

        assert_eq!(result.iterations, 50);
    }
}
