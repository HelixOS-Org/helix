//! # Helix Kernel Benchmark Suite
//!
//! Comprehensive benchmarking framework for evaluating Helix kernel performance.
//! Designed for deterministic, reproducible measurements with statistical analysis.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                      HELIX BENCHMARK SUITE                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                  │
//! │  │  SCHEDULER   │  │   MEMORY     │  │     IRQ      │                  │
//! │  │  BENCHMARKS  │  │  BENCHMARKS  │  │  BENCHMARKS  │                  │
//! │  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘                  │
//! │         │                 │                 │                          │
//! │         └────────────┬────┴─────────────────┘                          │
//! │                      │                                                  │
//! │              ┌───────▼───────┐                                         │
//! │              │  BENCHMARK    │                                         │
//! │              │    ENGINE     │                                         │
//! │              └───────┬───────┘                                         │
//! │                      │                                                  │
//! │              ┌───────▼───────┐                                         │
//! │              │   RESULTS     │                                         │
//! │              │   COLLECTOR   │                                         │
//! │              └───────┬───────┘                                         │
//! │                      │                                                  │
//! │              ┌───────▼───────┐                                         │
//! │              │    REPORT     │                                         │
//! │              │   GENERATOR   │                                         │
//! │              └───────────────┘                                         │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use helix_benchmarks::{BenchmarkSuite, BenchmarkConfig};
//!
//! let config = BenchmarkConfig::default()
//!     .iterations(10_000)
//!     .warmup(1_000);
//!
//! let suite = BenchmarkSuite::new(config);
//! let results = suite.run_all();
//!
//! results.print_report();
//! ```

#![no_std]
#![feature(alloc_error_handler)]

extern crate alloc;

pub mod engine;
pub mod ipc;
pub mod irq;
pub mod memory;
pub mod results;
pub mod scheduler;
pub mod stress;
pub mod timing;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// Re-exports
pub use engine::{BenchmarkConfig, BenchmarkEngine, BenchmarkRunner, RunResult};
pub use results::{BenchmarkReport, ReportFormatter, ResultCollector};
use spin::RwLock;
pub use timing::{CycleCounter, TimingSource};

// =============================================================================
// Core Types
// =============================================================================

/// Unique benchmark identifier
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BenchmarkId {
    /// Numeric ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Category
    pub category: BenchmarkCategory,
}

impl BenchmarkId {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            name: String::new(),
            category: BenchmarkCategory::Custom,
        }
    }

    pub fn with_name(id: u64, name: String, category: BenchmarkCategory) -> Self {
        Self { id, name, category }
    }
}

/// Benchmark category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BenchmarkCategory {
    /// Scheduler performance tests
    Scheduler,
    /// Memory management tests
    Memory,
    /// Interrupt handling tests
    Irq,
    /// Inter-process communication tests
    Ipc,
    /// System stress tests
    Stress,
    /// Custom user-defined tests
    Custom,
}

impl BenchmarkCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Scheduler => "scheduler",
            Self::Memory => "memory",
            Self::Irq => "irq",
            Self::Ipc => "ipc",
            Self::Stress => "stress",
            Self::Custom => "custom",
        }
    }
}

/// Time unit for measurements
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    /// CPU cycles
    Cycles,
    /// Nanoseconds
    Nanoseconds,
    /// Microseconds
    Microseconds,
    /// Milliseconds
    Milliseconds,
}

impl TimeUnit {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cycles => "cycles",
            Self::Nanoseconds => "ns",
            Self::Microseconds => "µs",
            Self::Milliseconds => "ms",
        }
    }

    /// Convert cycles to this unit (assuming 2.5 GHz CPU)
    pub fn from_cycles(&self, cycles: u64) -> u64 {
        const CPU_FREQ_MHZ: u64 = 2500; // Adjustable

        match self {
            Self::Cycles => cycles,
            Self::Nanoseconds => cycles * 1000 / CPU_FREQ_MHZ,
            Self::Microseconds => cycles / CPU_FREQ_MHZ,
            Self::Milliseconds => cycles / (CPU_FREQ_MHZ * 1000),
        }
    }
}

/// Benchmark state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkState {
    /// Not started
    Idle,
    /// Warming up
    WarmingUp,
    /// Running main iterations
    Running,
    /// Collecting results
    Collecting,
    /// Completed
    Completed,
    /// Failed
    Failed,
}

/// A single measurement
#[derive(Debug, Clone, Copy)]
pub struct Measurement {
    /// Value in cycles
    pub cycles: u64,
    /// Iteration number
    pub iteration: u32,
    /// Timestamp when taken
    pub timestamp: u64,
}

impl Measurement {
    pub fn new(cycles: u64, iteration: u32, timestamp: u64) -> Self {
        Self {
            cycles,
            iteration,
            timestamp,
        }
    }

    pub fn to_nanoseconds(&self, cpu_freq_mhz: u64) -> u64 {
        self.cycles * 1000 / cpu_freq_mhz
    }
}

/// Benchmark definition
pub struct BenchmarkDef {
    /// Unique ID
    pub id: BenchmarkId,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Category
    pub category: BenchmarkCategory,
    /// Setup function
    pub setup: Option<fn() -> bool>,
    /// Benchmark function (returns cycles)
    pub run: fn() -> u64,
    /// Teardown function
    pub teardown: Option<fn()>,
    /// Expected baseline (for comparison)
    pub baseline_cycles: Option<u64>,
}

// =============================================================================
// Statistics Types
// =============================================================================

/// Statistical results
#[derive(Debug, Clone, Default)]
pub struct Statistics {
    /// Minimum value
    pub min: u64,
    /// Maximum value
    pub max: u64,
    /// Mean (average)
    pub mean: u64,
    /// Median (p50)
    pub p50: u64,
    /// 95th percentile
    pub p95: u64,
    /// 99th percentile
    pub p99: u64,
    /// Standard deviation
    pub std_dev: u64,
    /// Variance
    pub variance: u64,
    /// Jitter (max - min)
    pub jitter: u64,
}

impl Statistics {
    /// Compute from samples
    pub fn from_samples(samples: &mut [u64]) -> Self {
        if samples.is_empty() {
            return Self::default();
        }

        samples.sort_unstable();
        let len = samples.len();

        let min = samples[0];
        let max = samples[len - 1];
        let sum: u64 = samples.iter().sum();
        let mean = sum / len as u64;

        // Percentiles
        let p50 = samples[len / 2];
        let p95 = samples[len * 95 / 100];
        let p99 = samples[len * 99 / 100];

        // Variance and std_dev
        let sum_sq_diff: u64 = samples
            .iter()
            .map(|&x| {
                let diff = if x > mean { x - mean } else { mean - x };
                diff * diff
            })
            .sum();
        let variance = sum_sq_diff / len as u64;

        // Integer sqrt for std_dev
        let std_dev = if variance > 0 {
            let mut x = variance;
            let mut y = (x + 1) / 2;
            while y < x {
                x = y;
                y = (x + variance / x) / 2;
            }
            x
        } else {
            0
        };

        Self {
            min,
            max,
            mean,
            p50,
            p95,
            p99,
            std_dev,
            variance,
            jitter: max - min,
        }
    }
}

/// Benchmark results container
#[derive(Clone)]
pub struct BenchmarkResults {
    /// Benchmark ID
    pub id: BenchmarkId,
    /// Benchmark name
    pub name: String,
    /// Category
    pub category: BenchmarkCategory,
    /// All measurements
    pub measurements: Vec<Measurement>,
    /// Computed statistics
    pub stats: Statistics,
    /// Start timestamp
    pub start_timestamp: u64,
    /// End timestamp
    pub end_timestamp: u64,
    /// Total time in cycles
    pub total_time_cycles: u64,
    /// Baseline for comparison
    pub baseline_cycles: Option<u64>,
    /// Performance vs baseline (%)
    pub vs_baseline_pct: Option<i32>,
    /// Whether failed
    pub failed: bool,
    /// Failure reason
    pub failure_reason: Option<String>,
}

impl BenchmarkResults {
    /// Create new results
    pub fn new(id: BenchmarkId, name: String, category: BenchmarkCategory) -> Self {
        Self {
            id,
            name,
            category,
            measurements: Vec::new(),
            stats: Statistics::default(),
            start_timestamp: timing::read_tsc(),
            end_timestamp: 0,
            total_time_cycles: 0,
            baseline_cycles: None,
            vs_baseline_pct: None,
            failed: false,
            failure_reason: None,
        }
    }

    /// Add a measurement
    pub fn add_measurement(&mut self, m: Measurement) {
        self.measurements.push(m);
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, reason: &str) {
        self.failed = true;
        self.failure_reason = Some(String::from(reason));
    }

    /// Compute statistics from measurements
    pub fn compute_statistics(&mut self) {
        if self.measurements.is_empty() {
            return;
        }

        let mut samples: Vec<u64> = self.measurements.iter().map(|m| m.cycles).collect();

        self.stats = Statistics::from_samples(&mut samples);
    }

    /// Compute comparison to baseline
    pub fn compute_comparison(&mut self) {
        if let Some(baseline) = self.baseline_cycles {
            if baseline > 0 {
                let current = self.stats.mean;
                let diff = (current as i64 - baseline as i64) * 100 / baseline as i64;
                self.vs_baseline_pct = Some(diff as i32);
            }
        }
    }
}

// =============================================================================
// Benchmark Suite
// =============================================================================

/// Main benchmark suite
pub struct BenchmarkSuite {
    /// Configuration
    config: BenchmarkConfig,
    /// Registered benchmarks
    benchmarks: RwLock<Vec<BenchmarkDef>>,
    /// Results storage
    results: RwLock<Vec<BenchmarkResults>>,
    /// Current state
    state: RwLock<BenchmarkState>,
    /// Next benchmark ID
    next_id: AtomicU64,
    /// Running flag
    running: AtomicBool,
}

impl BenchmarkSuite {
    /// Create new benchmark suite
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            benchmarks: RwLock::new(Vec::new()),
            results: RwLock::new(Vec::new()),
            state: RwLock::new(BenchmarkState::Idle),
            next_id: AtomicU64::new(1),
            running: AtomicBool::new(false),
        }
    }

    /// Register a benchmark
    pub fn register(&self, def: BenchmarkDef) -> BenchmarkId {
        let id_num = self.next_id.fetch_add(1, Ordering::SeqCst);
        let id = BenchmarkId::with_name(id_num, def.name.clone(), def.category);
        let mut def = def;
        def.id = id.clone();
        self.benchmarks.write().push(def);
        id
    }

    /// Register all default benchmarks
    pub fn register_defaults(&self) {
        // Scheduler benchmarks
        scheduler::register_benchmarks(self);

        // Memory benchmarks
        memory::register_benchmarks(self);

        // IRQ benchmarks
        irq::register_benchmarks(self);

        // IPC benchmarks
        ipc::register_benchmarks(self);

        // Stress tests
        stress::register_benchmarks(self);
    }

    /// Run all registered benchmarks
    pub fn run_all(&self) -> Vec<BenchmarkResults> {
        self.running.store(true, Ordering::SeqCst);
        *self.state.write() = BenchmarkState::Running;

        let benchmarks = self.benchmarks.read();
        let mut all_results = Vec::with_capacity(benchmarks.len());

        for bench in benchmarks.iter() {
            if !self.running.load(Ordering::SeqCst) {
                break;
            }

            let result = self.run_single(bench);
            all_results.push(result);
        }

        *self.state.write() = BenchmarkState::Completed;
        self.running.store(false, Ordering::SeqCst);

        // Store results
        *self.results.write() = all_results.clone();

        all_results
    }

    /// Run a single benchmark
    fn run_single(&self, bench: &BenchmarkDef) -> BenchmarkResults {
        let mut results =
            BenchmarkResults::new(bench.id.clone(), bench.name.clone(), bench.category);

        // Setup
        if let Some(setup) = bench.setup {
            if !setup() {
                results.mark_failed("Setup failed");
                return results;
            }
        }

        // Warmup phase
        *self.state.write() = BenchmarkState::WarmingUp;
        for _ in 0..self.config.warmup_iterations {
            let _ = (bench.run)();
        }

        // Main iterations
        *self.state.write() = BenchmarkState::Running;
        let start_timestamp = timing::read_tsc();

        for i in 0..self.config.iterations {
            let cycles = (bench.run)();
            let timestamp = timing::read_tsc();
            results.add_measurement(Measurement::new(cycles, i, timestamp));
        }

        results.end_timestamp = timing::read_tsc();
        results.total_time_cycles = results.end_timestamp - start_timestamp;

        // Collect phase
        *self.state.write() = BenchmarkState::Collecting;
        results.compute_statistics();

        // Compare to baseline if available
        if let Some(baseline) = bench.baseline_cycles {
            results.baseline_cycles = Some(baseline);
            results.compute_comparison();
        }

        // Teardown
        if let Some(teardown) = bench.teardown {
            teardown();
        }

        results
    }

    /// Run benchmarks by category
    pub fn run_category(&self, category: BenchmarkCategory) -> Vec<BenchmarkResults> {
        let benchmarks = self.benchmarks.read();
        let filtered: Vec<_> = benchmarks
            .iter()
            .filter(|b| b.category == category)
            .collect();

        let mut results = Vec::new();
        for bench in filtered {
            results.push(self.run_single(bench));
        }
        results
    }

    /// Stop running benchmarks
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Get current state
    pub fn state(&self) -> BenchmarkState {
        *self.state.read()
    }

    /// Get all results
    pub fn get_results(&self) -> Vec<BenchmarkResults> {
        self.results.read().clone()
    }

    /// Generate full report
    pub fn generate_report(&self) -> BenchmarkReport {
        let results = self.results.read().clone();
        BenchmarkReport::from_results(results, &self.config)
    }
}

impl Default for BenchmarkSuite {
    fn default() -> Self {
        Self::new(BenchmarkConfig::default())
    }
}

// =============================================================================
// Quick Benchmark Macros
// =============================================================================

/// Macro for defining a simple benchmark
#[macro_export]
macro_rules! benchmark {
    ($name:expr, $category:expr, $func:expr) => {
        BenchmarkDef {
            id: BenchmarkId::new(0),
            name: alloc::string::String::from($name),
            description: alloc::string::String::new(),
            category: $category,
            setup: None,
            run: $func,
            teardown: None,
            baseline_cycles: None,
        }
    };

    ($name:expr, $category:expr, $func:expr,baseline: $baseline:expr) => {
        BenchmarkDef {
            id: BenchmarkId::new(0),
            name: alloc::string::String::from($name),
            description: alloc::string::String::new(),
            category: $category,
            setup: None,
            run: $func,
            teardown: None,
            baseline_cycles: Some($baseline),
        }
    };
}

/// Macro for measuring a code block
#[macro_export]
macro_rules! measure {
    ($block:block) => {{
        let start = $crate::timing::read_tsc();
        $block
        let end = $crate::timing::read_tsc();
        end - start
    }};
}

// =============================================================================
// Platform Detection
// =============================================================================

/// Detected platform information
#[derive(Debug, Clone)]
pub struct PlatformInfo {
    /// Architecture name
    pub arch: &'static str,
    /// CPU frequency (MHz)
    pub cpu_freq_mhz: u64,
    /// Number of CPUs
    pub num_cpus: u32,
    /// Cache line size
    pub cache_line_size: u32,
    /// Whether TSC is available
    pub has_tsc: bool,
    /// Whether running in VM
    pub is_vm: bool,
}

impl PlatformInfo {
    /// Detect current platform
    pub fn detect() -> Self {
        Self {
            #[cfg(target_arch = "x86_64")]
            arch: "x86_64",
            #[cfg(target_arch = "aarch64")]
            arch: "aarch64",
            #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
            arch: "unknown",
            cpu_freq_mhz: 2500, // Default, should be detected
            num_cpus: 1,        // Default, should be detected from ACPI/DT
            cache_line_size: 64,
            has_tsc: cfg!(target_arch = "x86_64"),
            is_vm: false, // Should be detected via CPUID
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_suite_creation() {
        let config = BenchmarkConfig::default();
        let suite = BenchmarkSuite::new(config);
        assert_eq!(suite.state(), BenchmarkState::Idle);
    }

    #[test]
    fn test_measurement() {
        let m = Measurement::new(1000, 0, 12345);
        assert_eq!(m.cycles, 1000);
        assert_eq!(m.to_nanoseconds(2500), 400);
    }
}
