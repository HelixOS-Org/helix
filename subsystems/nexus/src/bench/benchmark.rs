//! Benchmark definition and execution

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

use super::result::BenchmarkResult;

// ============================================================================
// BENCHMARK
// ============================================================================

/// A benchmark
pub struct Benchmark {
    /// Benchmark name
    pub name: String,
    /// Function to benchmark
    func: Box<dyn Fn() + Send + Sync>,
    /// Setup function
    setup: Option<Box<dyn Fn() + Send + Sync>>,
    /// Teardown function
    teardown: Option<Box<dyn Fn() + Send + Sync>>,
    /// Minimum iterations
    min_iterations: u64,
    /// Minimum time (cycles)
    min_time: u64,
    /// Warmup iterations
    warmup_iterations: u64,
}

impl Benchmark {
    /// Create a new benchmark
    pub fn new(name: impl Into<String>, func: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            name: name.into(),
            func: Box::new(func),
            setup: None,
            teardown: None,
            min_iterations: 100,
            min_time: 1_000_000_000, // 1 second at 1GHz
            warmup_iterations: 10,
        }
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

    /// Set minimum iterations
    #[inline(always)]
    pub fn with_min_iterations(mut self, n: u64) -> Self {
        self.min_iterations = n;
        self
    }

    /// Set minimum time
    #[inline(always)]
    pub fn with_min_time(mut self, cycles: u64) -> Self {
        self.min_time = cycles;
        self
    }

    /// Set warmup iterations
    #[inline(always)]
    pub fn with_warmup(mut self, n: u64) -> Self {
        self.warmup_iterations = n;
        self
    }

    /// Run the benchmark
    pub fn run(&self) -> BenchmarkResult {
        // Setup
        if let Some(ref setup) = self.setup {
            setup();
        }

        // Warmup
        for _ in 0..self.warmup_iterations {
            (self.func)();
        }

        // Collect samples
        let mut samples = Vec::new();
        let mut total_time = 0u64;

        while samples.len() < self.min_iterations as usize || total_time < self.min_time {
            let start = NexusTimestamp::now();
            (self.func)();
            let end = NexusTimestamp::now();

            let duration = end.duration_since(start);
            samples.push(duration);
            total_time += duration;
        }

        // Teardown
        if let Some(ref teardown) = self.teardown {
            teardown();
        }

        BenchmarkResult::from_samples(&self.name, &samples)
    }

    /// Run with specific iteration count
    pub fn run_iterations(&self, iterations: u64) -> BenchmarkResult {
        if let Some(ref setup) = self.setup {
            setup();
        }

        // Warmup
        for _ in 0..self.warmup_iterations.min(10) {
            (self.func)();
        }

        let mut samples = Vec::with_capacity(iterations as usize);

        for _ in 0..iterations {
            let start = NexusTimestamp::now();
            (self.func)();
            let end = NexusTimestamp::now();
            samples.push(end.duration_since(start));
        }

        if let Some(ref teardown) = self.teardown {
            teardown();
        }

        BenchmarkResult::from_samples(&self.name, &samples)
    }
}
