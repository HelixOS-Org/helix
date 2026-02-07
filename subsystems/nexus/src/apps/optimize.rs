//! # Per-Application Optimization Strategies
//!
//! Generates specific optimization strategies for each application based
//! on its profile, classification, and predicted future behavior.

use alloc::string::String;
use alloc::vec::Vec;

use super::classify::WorkloadCategory;
use super::profile::ProcessProfile;

// ============================================================================
// OPTIMIZATION TYPES
// ============================================================================

/// A tuning knob that can be adjusted
#[derive(Debug, Clone)]
pub struct TuningKnob {
    /// Knob name
    pub name: &'static str,
    /// Current value
    pub current: f64,
    /// Recommended value
    pub recommended: f64,
    /// Minimum allowed value
    pub min: f64,
    /// Maximum allowed value
    pub max: f64,
    /// Impact score (how much this affects performance)
    pub impact: f64,
}

/// Scheduler hint to give for a process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerHint {
    /// Prefer latency over throughput
    LatencyOptimized,
    /// Prefer throughput over latency
    ThroughputOptimized,
    /// Balanced mode
    Balanced,
    /// Real-time — minimize jitter
    RealTime,
    /// Energy efficient — allow deeper sleep
    PowerSaver,
    /// Burst-aware — expect intermittent CPU spikes
    BurstAware,
}

/// Optimization strategy for a process
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationStrategy {
    /// Aggressive optimization for performance
    Aggressive,
    /// Moderate optimizations
    Moderate,
    /// Conservative — minimize risk
    Conservative,
    /// No optimization (passthrough)
    None,
}

/// A specific optimization recommendation
#[derive(Debug, Clone)]
pub struct AppOptimization {
    /// Strategy used
    pub strategy: OptimizationStrategy,
    /// Description
    pub description: String,
    /// Target category
    pub category: &'static str,
    /// Expected improvement (0.0 - 1.0)
    pub expected_improvement: f64,
    /// Scheduler hint
    pub scheduler_hint: Option<SchedulerHint>,
    /// Tuning knobs to adjust
    pub knobs: Vec<TuningKnob>,
    /// Risk level (0.0 = safe, 1.0 = risky)
    pub risk: f64,
}

// ============================================================================
// OPTIMIZATION ENGINE
// ============================================================================

/// The optimization engine — generates per-process optimization plans.
pub struct OptimizationEngine {
    /// Active optimizations per process
    active_count: u64,
    /// Strategy preference
    default_strategy: OptimizationStrategy,
    /// Whether to enable experimental optimizations
    experimental: bool,
}

impl OptimizationEngine {
    pub fn new() -> Self {
        Self {
            active_count: 0,
            default_strategy: OptimizationStrategy::Moderate,
            experimental: false,
        }
    }

    /// Enable experimental optimizations
    pub fn enable_experimental(&mut self) {
        self.experimental = true;
    }

    /// Set default strategy
    pub fn set_strategy(&mut self, strategy: OptimizationStrategy) {
        self.default_strategy = strategy;
    }

    /// Generate optimizations for a process
    pub fn optimize(&mut self, profile: &ProcessProfile) -> Vec<AppOptimization> {
        let mut opts = Vec::new();

        // CPU optimizations
        opts.extend(self.cpu_optimizations(profile));

        // Memory optimizations
        opts.extend(self.memory_optimizations(profile));

        // I/O optimizations
        opts.extend(self.io_optimizations(profile));

        // Network optimizations
        opts.extend(self.network_optimizations(profile));

        self.active_count += opts.len() as u64;
        opts
    }

    fn cpu_optimizations(&self, profile: &ProcessProfile) -> Vec<AppOptimization> {
        let mut opts = Vec::new();

        if profile.cpu.is_compute_bound {
            let hint = if profile.cpu.is_bursty {
                SchedulerHint::BurstAware
            } else {
                SchedulerHint::ThroughputOptimized
            };

            opts.push(AppOptimization {
                strategy: self.default_strategy,
                description: String::from("CPU-bound: optimize for compute throughput"),
                category: "cpu",
                expected_improvement: 0.15,
                scheduler_hint: Some(hint),
                knobs: alloc::vec![
                    TuningKnob {
                        name: "time_slice_us",
                        current: 4000.0,
                        recommended: 10000.0,
                        min: 1000.0,
                        max: 100000.0,
                        impact: 0.7,
                    },
                    TuningKnob {
                        name: "migration_cost_us",
                        current: 250.0,
                        recommended: 500.0,
                        min: 100.0,
                        max: 5000.0,
                        impact: 0.4,
                    },
                ],
                risk: 0.1,
            });
        }

        if profile.cpu.cache_miss_rate > 0.2 {
            opts.push(AppOptimization {
                strategy: self.default_strategy,
                description: String::from("High cache miss rate: increase cache allocation"),
                category: "cache",
                expected_improvement: 0.12,
                scheduler_hint: None,
                knobs: alloc::vec![TuningKnob {
                    name: "cache_ways_allocated",
                    current: 4.0,
                    recommended: 8.0,
                    min: 1.0,
                    max: 16.0,
                    impact: 0.6,
                }],
                risk: 0.15,
            });
        }

        if profile.cpu.avg_usage > 0.9 && profile.cpu.typical_thread_count > 2 {
            opts.push(AppOptimization {
                strategy: self.default_strategy,
                description: String::from("Saturated multi-threaded: enable group scheduling"),
                category: "scheduler",
                expected_improvement: 0.10,
                scheduler_hint: Some(SchedulerHint::ThroughputOptimized),
                knobs: alloc::vec![TuningKnob {
                    name: "group_scheduling_bandwidth_pct",
                    current: 100.0,
                    recommended: 95.0,
                    min: 50.0,
                    max: 100.0,
                    impact: 0.3,
                }],
                risk: 0.05,
            });
        }

        opts
    }

    fn memory_optimizations(&self, profile: &ProcessProfile) -> Vec<AppOptimization> {
        let mut opts = Vec::new();

        if profile.memory.should_use_huge_pages() {
            opts.push(AppOptimization {
                strategy: self.default_strategy,
                description: String::from("Large working set: enable transparent huge pages"),
                category: "memory",
                expected_improvement: 0.15,
                scheduler_hint: None,
                knobs: alloc::vec![TuningKnob {
                    name: "thp_enabled",
                    current: 0.0,
                    recommended: 1.0,
                    min: 0.0,
                    max: 1.0,
                    impact: 0.8,
                }],
                risk: 0.1,
            });
        }

        if profile.memory.page_fault_rate > 500.0 {
            opts.push(AppOptimization {
                strategy: self.default_strategy,
                description: String::from("High fault rate: prefault pages on mmap"),
                category: "memory",
                expected_improvement: 0.20,
                scheduler_hint: None,
                knobs: alloc::vec![TuningKnob {
                    name: "mmap_prefault",
                    current: 0.0,
                    recommended: 1.0,
                    min: 0.0,
                    max: 1.0,
                    impact: 0.7,
                }],
                risk: 0.15,
            });
        }

        opts
    }

    fn io_optimizations(&self, profile: &ProcessProfile) -> Vec<AppOptimization> {
        let mut opts = Vec::new();

        if profile.io.sequential_reads {
            let readahead = profile.io.optimal_readahead();
            opts.push(AppOptimization {
                strategy: self.default_strategy,
                description: alloc::format!(
                    "Sequential reads: set readahead to {}KB",
                    readahead / 1024
                ),
                category: "io",
                expected_improvement: 0.25,
                scheduler_hint: None,
                knobs: alloc::vec![TuningKnob {
                    name: "readahead_kb",
                    current: 128.0,
                    recommended: (readahead / 1024) as f64,
                    min: 4.0,
                    max: 2048.0,
                    impact: 0.8,
                }],
                risk: 0.05,
            });
        }

        if profile.io.frequent_fsync {
            opts.push(AppOptimization {
                strategy: self.default_strategy,
                description: String::from("Frequent fsync: enable write coalescing"),
                category: "io",
                expected_improvement: 0.30,
                scheduler_hint: None,
                knobs: alloc::vec![TuningKnob {
                    name: "fsync_coalesce_ms",
                    current: 0.0,
                    recommended: 5.0,
                    min: 0.0,
                    max: 50.0,
                    impact: 0.9,
                }],
                risk: 0.20,
            });
        }

        opts
    }

    fn network_optimizations(&self, profile: &ProcessProfile) -> Vec<AppOptimization> {
        let mut opts = Vec::new();

        if profile.network.is_server && profile.network.connection_rate > 100.0 {
            opts.push(AppOptimization {
                strategy: self.default_strategy,
                description: String::from("High-rate server: enable connection optimization"),
                category: "network",
                expected_improvement: 0.15,
                scheduler_hint: Some(SchedulerHint::LatencyOptimized),
                knobs: alloc::vec![
                    TuningKnob {
                        name: "tcp_fastopen",
                        current: 0.0,
                        recommended: 1.0,
                        min: 0.0,
                        max: 1.0,
                        impact: 0.5,
                    },
                    TuningKnob {
                        name: "so_reuseport",
                        current: 0.0,
                        recommended: 1.0,
                        min: 0.0,
                        max: 1.0,
                        impact: 0.4,
                    },
                ],
                risk: 0.05,
            });
        }

        if profile.network.active_connections > 500 {
            opts.push(AppOptimization {
                strategy: self.default_strategy,
                description: String::from("Many connections: increase socket buffer"),
                category: "network",
                expected_improvement: 0.10,
                scheduler_hint: None,
                knobs: alloc::vec![TuningKnob {
                    name: "socket_buffer_kb",
                    current: 128.0,
                    recommended: 512.0,
                    min: 64.0,
                    max: 4096.0,
                    impact: 0.5,
                }],
                risk: 0.1,
            });
        }

        opts
    }
}
