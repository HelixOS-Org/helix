//! Benchmark Engine - Core execution and configuration
//!
//! Manages benchmark execution, configuration, and coordination.

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

// =============================================================================
// Configuration
// =============================================================================

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    /// Number of warmup iterations
    pub warmup_iterations: u32,
    /// Number of main iterations
    pub iterations: u32,
    /// CPU frequency for time conversion (MHz)
    pub cpu_freq_mhz: u64,
    /// Whether to collect all samples or just statistics
    pub collect_all_samples: bool,
    /// Maximum samples to store (if collect_all_samples)
    pub max_samples: usize,
    /// Enable outlier detection
    pub detect_outliers: bool,
    /// Outlier threshold (standard deviations)
    pub outlier_threshold: f32,
    /// Enable verbose output
    pub verbose: bool,
    /// Target architecture
    pub target_arch: TargetArch,
    /// Execution mode
    pub exec_mode: ExecutionMode,
}

impl BenchmarkConfig {
    /// Create with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Set number of iterations
    pub fn iterations(mut self, n: u32) -> Self {
        self.iterations = n;
        self
    }

    /// Set warmup iterations
    pub fn warmup(mut self, n: u32) -> Self {
        self.warmup_iterations = n;
        self
    }

    /// Set CPU frequency
    pub fn cpu_freq_mhz(mut self, freq: u64) -> Self {
        self.cpu_freq_mhz = freq;
        self
    }

    /// Enable collecting all samples
    pub fn collect_samples(mut self, enable: bool) -> Self {
        self.collect_all_samples = enable;
        self
    }

    /// Set verbose mode
    pub fn verbose(mut self, enable: bool) -> Self {
        self.verbose = enable;
        self
    }

    /// Set target architecture
    pub fn arch(mut self, arch: TargetArch) -> Self {
        self.target_arch = arch;
        self
    }

    /// Set execution mode
    pub fn mode(mut self, mode: ExecutionMode) -> Self {
        self.exec_mode = mode;
        self
    }
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            warmup_iterations: 1_000,
            iterations: 10_000,
            cpu_freq_mhz: 2_500,
            collect_all_samples: true,
            max_samples: 100_000,
            detect_outliers: true,
            outlier_threshold: 3.0,
            verbose: false,
            target_arch: TargetArch::X86_64,
            exec_mode: ExecutionMode::Virtualized,
        }
    }
}

/// Target architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArch {
    X86_64,
    Arm64,
    RiscV64,
}

impl TargetArch {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Arm64 => "aarch64",
            Self::RiscV64 => "riscv64",
        }
    }
}

/// Execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Running on bare metal
    BareMetal,
    /// Running in VM (QEMU, etc.)
    Virtualized,
    /// Running in emulator
    Emulated,
}

impl ExecutionMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BareMetal => "bare-metal",
            Self::Virtualized => "virtualized",
            Self::Emulated => "emulated",
        }
    }
}

// =============================================================================
// Benchmark Engine
// =============================================================================

/// Core benchmark execution engine
pub struct BenchmarkEngine {
    /// Configuration
    config: BenchmarkConfig,
    /// Current iteration
    current_iteration: AtomicU32,
    /// Total cycles measured
    total_cycles: AtomicU64,
    /// Minimum cycles
    min_cycles: AtomicU64,
    /// Maximum cycles
    max_cycles: AtomicU64,
    /// Running sum for mean
    sum_cycles: AtomicU64,
    /// Running sum of squares for variance
    sum_squares: AtomicU64,
}

impl BenchmarkEngine {
    /// Create new engine
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            current_iteration: AtomicU32::new(0),
            total_cycles: AtomicU64::new(0),
            min_cycles: AtomicU64::new(u64::MAX),
            max_cycles: AtomicU64::new(0),
            sum_cycles: AtomicU64::new(0),
            sum_squares: AtomicU64::new(0),
        }
    }

    /// Reset engine state
    pub fn reset(&self) {
        self.current_iteration.store(0, Ordering::SeqCst);
        self.total_cycles.store(0, Ordering::SeqCst);
        self.min_cycles.store(u64::MAX, Ordering::SeqCst);
        self.max_cycles.store(0, Ordering::SeqCst);
        self.sum_cycles.store(0, Ordering::SeqCst);
        self.sum_squares.store(0, Ordering::SeqCst);
    }

    /// Record a measurement
    pub fn record(&self, cycles: u64) {
        self.current_iteration.fetch_add(1, Ordering::SeqCst);
        self.total_cycles.fetch_add(cycles, Ordering::SeqCst);
        self.sum_cycles.fetch_add(cycles, Ordering::SeqCst);
        self.sum_squares
            .fetch_add(cycles * cycles, Ordering::SeqCst);

        // Update min
        loop {
            let current_min = self.min_cycles.load(Ordering::SeqCst);
            if cycles >= current_min {
                break;
            }
            if self
                .min_cycles
                .compare_exchange(current_min, cycles, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }

        // Update max
        loop {
            let current_max = self.max_cycles.load(Ordering::SeqCst);
            if cycles <= current_max {
                break;
            }
            if self
                .max_cycles
                .compare_exchange(current_max, cycles, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Get current iteration count
    pub fn iteration(&self) -> u32 {
        self.current_iteration.load(Ordering::SeqCst)
    }

    /// Get total cycles
    pub fn total_cycles(&self) -> u64 {
        self.total_cycles.load(Ordering::SeqCst)
    }

    /// Get minimum cycles
    pub fn min_cycles(&self) -> u64 {
        let min = self.min_cycles.load(Ordering::SeqCst);
        if min == u64::MAX {
            0
        } else {
            min
        }
    }

    /// Get maximum cycles
    pub fn max_cycles(&self) -> u64 {
        self.max_cycles.load(Ordering::SeqCst)
    }

    /// Calculate mean
    pub fn mean_cycles(&self) -> u64 {
        let sum = self.sum_cycles.load(Ordering::SeqCst);
        let count = self.current_iteration.load(Ordering::SeqCst) as u64;
        if count == 0 {
            0
        } else {
            sum / count
        }
    }

    /// Calculate variance (approximate)
    pub fn variance(&self) -> u64 {
        let sum = self.sum_cycles.load(Ordering::SeqCst);
        let sum_sq = self.sum_squares.load(Ordering::SeqCst);
        let count = self.current_iteration.load(Ordering::SeqCst) as u64;

        if count <= 1 {
            return 0;
        }

        // Var = E[X²] - E[X]²
        let mean_sq = (sum * sum) / (count * count);
        let mean_of_sq = sum_sq / count;

        mean_of_sq.saturating_sub(mean_sq)
    }

    /// Calculate standard deviation (integer approximation)
    pub fn std_dev(&self) -> u64 {
        // Integer square root approximation
        let var = self.variance();
        if var == 0 {
            return 0;
        }

        // Newton's method for integer sqrt
        let mut x = var;
        let mut y = (x + 1) / 2;
        while y < x {
            x = y;
            y = (x + var / x) / 2;
        }
        x
    }

    /// Get config
    pub fn config(&self) -> &BenchmarkConfig {
        &self.config
    }

    /// Convert cycles to nanoseconds
    pub fn cycles_to_ns(&self, cycles: u64) -> u64 {
        cycles * 1000 / self.config.cpu_freq_mhz
    }

    /// Convert cycles to microseconds
    pub fn cycles_to_us(&self, cycles: u64) -> u64 {
        cycles / self.config.cpu_freq_mhz
    }
}

impl Default for BenchmarkEngine {
    fn default() -> Self {
        Self::new(BenchmarkConfig::default())
    }
}

// =============================================================================
// Benchmark Runner
// =============================================================================

/// Type for benchmark function
pub type BenchFn = fn() -> u64;

/// Simple benchmark runner
pub struct BenchmarkRunner {
    engine: BenchmarkEngine,
    samples: Vec<u64>,
}

impl BenchmarkRunner {
    pub fn new(config: BenchmarkConfig) -> Self {
        let collect = config.collect_all_samples;
        let max = config.max_samples;
        Self {
            engine: BenchmarkEngine::new(config),
            samples: if collect {
                Vec::with_capacity(max)
            } else {
                Vec::new()
            },
        }
    }

    /// Run a benchmark function
    pub fn run(&mut self, func: BenchFn) -> RunResult {
        self.engine.reset();
        self.samples.clear();

        let config = self.engine.config().clone();

        // Warmup
        for _ in 0..config.warmup_iterations {
            let _ = func();
        }

        // Main iterations
        for _ in 0..config.iterations {
            let cycles = func();
            self.engine.record(cycles);

            if config.collect_all_samples && self.samples.len() < config.max_samples {
                self.samples.push(cycles);
            }
        }

        // Compute results
        let mut result = RunResult {
            iterations: config.iterations,
            total_cycles: self.engine.total_cycles(),
            min_cycles: self.engine.min_cycles(),
            max_cycles: self.engine.max_cycles(),
            mean_cycles: self.engine.mean_cycles(),
            std_dev_cycles: self.engine.std_dev(),
            median_cycles: 0,
            p50_cycles: 0,
            p95_cycles: 0,
            p99_cycles: 0,
            throughput_ops_per_sec: 0,
            samples: if config.collect_all_samples {
                Some(self.samples.clone())
            } else {
                None
            },
        };

        // Compute percentiles if we have samples
        if !self.samples.is_empty() {
            self.samples.sort_unstable();
            let len = self.samples.len();

            result.median_cycles = self.samples[len / 2];
            result.p50_cycles = self.samples[len / 2];
            result.p95_cycles = self.samples[len * 95 / 100];
            result.p99_cycles = self.samples[len * 99 / 100];
        }

        // Compute throughput
        if result.mean_cycles > 0 {
            // ops/sec = cycles_per_sec / cycles_per_op
            let cycles_per_sec = config.cpu_freq_mhz * 1_000_000;
            result.throughput_ops_per_sec = cycles_per_sec / result.mean_cycles;
        }

        result
    }
}

/// Result of a benchmark run
#[derive(Debug, Clone)]
pub struct RunResult {
    pub iterations: u32,
    pub total_cycles: u64,
    pub min_cycles: u64,
    pub max_cycles: u64,
    pub mean_cycles: u64,
    pub std_dev_cycles: u64,
    pub median_cycles: u64,
    pub p50_cycles: u64,
    pub p95_cycles: u64,
    pub p99_cycles: u64,
    pub throughput_ops_per_sec: u64,
    pub samples: Option<Vec<u64>>,
}

impl RunResult {
    /// Jitter (max - min)
    pub fn jitter(&self) -> u64 {
        self.max_cycles.saturating_sub(self.min_cycles)
    }

    /// Coefficient of variation (std_dev / mean * 100)
    pub fn cv_percent(&self) -> u64 {
        if self.mean_cycles == 0 {
            0
        } else {
            self.std_dev_cycles * 100 / self.mean_cycles
        }
    }
}
