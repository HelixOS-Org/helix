//! Benchmark suite

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

use super::benchmark::Benchmark;
use super::result::BenchmarkResult;

// ============================================================================
// BENCHMARK SUITE
// ============================================================================

/// A collection of benchmarks
pub struct BenchmarkSuite {
    /// Suite name
    pub name: String,
    /// Benchmarks
    benchmarks: Vec<Benchmark>,
    /// Global setup
    setup: Option<Box<dyn Fn() + Send + Sync>>,
    /// Global teardown
    teardown: Option<Box<dyn Fn() + Send + Sync>>,
}

impl BenchmarkSuite {
    /// Create a new suite
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            benchmarks: Vec::new(),
            setup: None,
            teardown: None,
        }
    }

    /// Add a benchmark
    #[inline(always)]
    pub fn add(&mut self, benchmark: Benchmark) {
        self.benchmarks.push(benchmark);
    }

    /// Add a simple benchmark
    #[inline(always)]
    pub fn bench(&mut self, name: impl Into<String>, func: impl Fn() + Send + Sync + 'static) {
        self.add(Benchmark::new(name, func));
    }

    /// Set global setup
    #[inline(always)]
    pub fn with_setup(mut self, setup: impl Fn() + Send + Sync + 'static) -> Self {
        self.setup = Some(Box::new(setup));
        self
    }

    /// Set global teardown
    #[inline(always)]
    pub fn with_teardown(mut self, teardown: impl Fn() + Send + Sync + 'static) -> Self {
        self.teardown = Some(Box::new(teardown));
        self
    }

    /// Run all benchmarks
    pub fn run(&self) -> SuiteResult {
        let start = NexusTimestamp::now();

        if let Some(ref setup) = self.setup {
            setup();
        }

        let mut results = Vec::new();
        for benchmark in &self.benchmarks {
            results.push(benchmark.run());
        }

        if let Some(ref teardown) = self.teardown {
            teardown();
        }

        let end = NexusTimestamp::now();

        SuiteResult {
            suite_name: self.name.clone(),
            results,
            total_time: end.duration_since(start),
        }
    }

    /// Get benchmark count
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.benchmarks.len()
    }

    /// Is suite empty?
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.benchmarks.is_empty()
    }
}

// ============================================================================
// SUITE RESULT
// ============================================================================

/// Result of running a benchmark suite
#[derive(Debug, Clone)]
pub struct SuiteResult {
    /// Suite name
    pub suite_name: String,
    /// Individual results
    pub results: Vec<BenchmarkResult>,
    /// Total time
    pub total_time: u64,
}

impl SuiteResult {
    /// Get fastest benchmark
    #[inline]
    pub fn fastest(&self) -> Option<&BenchmarkResult> {
        self.results.iter().min_by(|a, b| {
            a.mean
                .partial_cmp(&b.mean)
                .unwrap_or(core::cmp::Ordering::Equal)
        })
    }

    /// Get slowest benchmark
    #[inline]
    pub fn slowest(&self) -> Option<&BenchmarkResult> {
        self.results.iter().max_by(|a, b| {
            a.mean
                .partial_cmp(&b.mean)
                .unwrap_or(core::cmp::Ordering::Equal)
        })
    }

    /// Get summary
    pub fn summary(&self) -> String {
        let mut s = format!(
            "Benchmark Suite: {} ({} benchmarks, {} total cycles)\n",
            self.suite_name,
            self.results.len(),
            self.total_time
        );

        for result in &self.results {
            s.push_str(&format!("  {}\n", result.summary()));
        }

        s
    }
}
