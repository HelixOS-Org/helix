//! Regression tracking and quick benchmarking utilities

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

use super::result::BenchmarkResult;
use super::suite::SuiteResult;

// ============================================================================
// REGRESSION TRACKER
// ============================================================================

/// Track benchmark results over time for regression detection
pub struct RegressionTracker {
    /// Baseline results by name
    baselines: BTreeMap<String, BenchmarkResult>,
    /// Regression threshold (e.g., 0.05 = 5%)
    threshold: f64,
}

impl RegressionTracker {
    /// Create a new tracker
    pub fn new(threshold: f64) -> Self {
        Self {
            baselines: BTreeMap::new(),
            threshold,
        }
    }

    /// Set baseline
    pub fn set_baseline(&mut self, result: BenchmarkResult) {
        self.baselines.insert(result.name.clone(), result);
    }

    /// Set baselines from suite result
    pub fn set_baselines(&mut self, suite_result: &SuiteResult) {
        for result in &suite_result.results {
            self.set_baseline(result.clone());
        }
    }

    /// Check for regression
    pub fn check(&self, result: &BenchmarkResult) -> Option<RegressionReport> {
        let baseline = self.baselines.get(&result.name)?;

        if result.is_regression(baseline, self.threshold) {
            Some(RegressionReport {
                name: result.name.clone(),
                baseline_mean: baseline.mean,
                current_mean: result.mean,
                slowdown: result.mean / baseline.mean,
                threshold: self.threshold,
            })
        } else {
            None
        }
    }

    /// Check all results from a suite
    pub fn check_suite(&self, suite_result: &SuiteResult) -> Vec<RegressionReport> {
        let mut regressions = Vec::new();

        for result in &suite_result.results {
            if let Some(report) = self.check(result) {
                regressions.push(report);
            }
        }

        regressions
    }

    /// Get threshold
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Set threshold
    pub fn set_threshold(&mut self, threshold: f64) {
        self.threshold = threshold;
    }
}

// ============================================================================
// REGRESSION REPORT
// ============================================================================

/// Report of a regression
#[derive(Debug, Clone)]
pub struct RegressionReport {
    /// Benchmark name
    pub name: String,
    /// Baseline mean
    pub baseline_mean: f64,
    /// Current mean
    pub current_mean: f64,
    /// Slowdown factor
    pub slowdown: f64,
    /// Threshold that was exceeded
    pub threshold: f64,
}

impl RegressionReport {
    /// Get summary
    pub fn summary(&self) -> String {
        format!(
            "REGRESSION: {} - {:.1}x slower (was {:.0} cycles, now {:.0} cycles, threshold {:.0}%)",
            self.name,
            self.slowdown,
            self.baseline_mean,
            self.current_mean,
            self.threshold * 100.0
        )
    }
}

// ============================================================================
// QUICK BENCH UTILITIES
// ============================================================================

/// Quick benchmarking utility
pub fn quick_bench(name: &str, iterations: u64, mut func: impl FnMut()) -> BenchmarkResult {
    let mut samples = Vec::with_capacity(iterations as usize);

    // Warmup
    for _ in 0..10.min(iterations) {
        func();
    }

    // Measure
    for _ in 0..iterations {
        let start = NexusTimestamp::now();
        func();
        let end = NexusTimestamp::now();
        samples.push(end.duration_since(start));
    }

    BenchmarkResult::from_samples(name, &samples)
}

/// Measure a single execution
pub fn measure<R>(mut func: impl FnMut() -> R) -> (R, u64) {
    let start = NexusTimestamp::now();
    let result = func();
    let end = NexusTimestamp::now();
    (result, end.duration_since(start))
}
