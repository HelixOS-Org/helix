//! # Application Profiling
//!
//! Continuously learns application behavior from syscall patterns,
//! resource usage, and I/O characteristics. Builds rich profiles
//! that drive optimization decisions.

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// APPLICATION CLASSIFICATION
// ============================================================================

/// High-level classification of an application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppClass {
    /// CPU-bound computation (scientific, encoding, compilation)
    CpuIntensive,
    /// I/O-bound (databases, file servers, log processors)
    IoIntensive,
    /// Network-bound (web servers, proxies, streaming)
    NetworkIntensive,
    /// Memory-bound (caches, in-memory databases)
    MemoryIntensive,
    /// Interactive (GUI apps, editors, terminals)
    Interactive,
    /// Batch processing (cron jobs, ETL pipelines)
    Batch,
    /// Real-time (audio, video, gaming)
    RealTime,
    /// Mixed workload
    Mixed,
    /// Unknown / insufficient data
    Unknown,
}

impl AppClass {
    /// Whether this class benefits from syscall prediction
    #[inline]
    pub fn benefits_from_prediction(&self) -> bool {
        matches!(
            self,
            Self::IoIntensive | Self::NetworkIntensive | Self::Batch | Self::RealTime
        )
    }

    /// Whether this class benefits from batching
    #[inline]
    pub fn benefits_from_batching(&self) -> bool {
        matches!(
            self,
            Self::IoIntensive | Self::Batch | Self::NetworkIntensive
        )
    }

    /// Whether this class is latency-sensitive
    #[inline(always)]
    pub fn is_latency_sensitive(&self) -> bool {
        matches!(self, Self::Interactive | Self::RealTime)
    }
}

// ============================================================================
// RESOURCE USAGE PATTERNS
// ============================================================================

/// Observed resource usage pattern
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ResourceUsagePattern {
    /// Average CPU usage (0.0 - 1.0)
    pub avg_cpu: f64,
    /// Peak CPU usage
    pub peak_cpu: f64,
    /// Average memory usage (bytes)
    pub avg_memory: usize,
    /// Peak memory usage
    pub peak_memory: usize,
    /// I/O read bytes per second
    pub io_read_bps: u64,
    /// I/O write bytes per second
    pub io_write_bps: u64,
    /// Network bytes per second
    pub net_bps: u64,
    /// Syscalls per second
    pub syscalls_per_sec: f64,
    /// Whether usage is bursty
    pub bursty: bool,
    /// Whether usage is periodic
    pub periodic: bool,
    /// Period length if periodic (ms)
    pub period_ms: u64,
}

impl ResourceUsagePattern {
    pub fn new() -> Self {
        Self {
            avg_cpu: 0.0,
            peak_cpu: 0.0,
            avg_memory: 0,
            peak_memory: 0,
            io_read_bps: 0,
            io_write_bps: 0,
            net_bps: 0,
            syscalls_per_sec: 0.0,
            bursty: false,
            periodic: false,
            period_ms: 0,
        }
    }

    /// Dominant resource dimension
    pub fn dominant_resource(&self) -> &'static str {
        let cpu_score = self.avg_cpu;
        let io_score = (self.io_read_bps + self.io_write_bps) as f64 / 1_000_000_000.0;
        let net_score = self.net_bps as f64 / 1_000_000_000.0;
        let mem_score = self.avg_memory as f64 / (4 * 1024 * 1024 * 1024) as f64;

        let max = cpu_score.max(io_score).max(net_score).max(mem_score);

        if (max - cpu_score).abs() < f64::EPSILON {
            "cpu"
        } else if (max - io_score).abs() < f64::EPSILON {
            "io"
        } else if (max - net_score).abs() < f64::EPSILON {
            "network"
        } else {
            "memory"
        }
    }
}

// ============================================================================
// APPLICATION BEHAVIOR
// ============================================================================

/// Detailed behavior observations for a process
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppBehavior {
    /// Syscall type frequency distribution
    pub syscall_freq: LinearMap<u64, 64>,
    /// Total syscalls observed
    pub total_syscalls: u64,
    /// Average data size per I/O syscall
    pub avg_io_size: usize,
    /// Whether reads are sequential
    pub sequential_reads: bool,
    /// Whether writes are sequential
    pub sequential_writes: bool,
    /// Average time between syscalls (ns)
    pub avg_inter_syscall_ns: u64,
    /// Standard deviation of inter-syscall time
    pub inter_syscall_stddev: f64,
    /// Number of unique file descriptors used
    pub unique_fds: usize,
    /// Number of concurrent threads observed
    pub thread_count: usize,
    /// Whether the app uses async I/O
    pub uses_async_io: bool,
    /// Whether the app uses memory-mapped I/O
    pub uses_mmap: bool,
}

impl AppBehavior {
    pub fn new() -> Self {
        Self {
            syscall_freq: LinearMap::new(),
            total_syscalls: 0,
            avg_io_size: 0,
            sequential_reads: false,
            sequential_writes: false,
            avg_inter_syscall_ns: 0,
            inter_syscall_stddev: 0.0,
            unique_fds: 0,
            thread_count: 1,
            uses_async_io: false,
            uses_mmap: false,
        }
    }

    /// Top N most frequent syscall types
    pub fn top_syscalls(&self, n: usize) -> Vec<(SyscallType, f64)> {
        let mut entries: Vec<_> = self.syscall_freq.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));

        entries
            .into_iter()
            .take(n)
            .map(|(key, count)| {
                let pct = *count as f64 / self.total_syscalls.max(1) as f64;
                (SyscallType::from_number(*key), pct)
            })
            .collect()
    }

    /// I/O ratio (fraction of syscalls that are I/O)
    #[inline]
    pub fn io_ratio(&self) -> f64 {
        let io_types = [0u64, 1, 2, 3, 8, 4, 78, 74, 16]; // read, write, open, close, seek, stat, readdir, fsync, ioctl
        let io_count: u64 = io_types
            .iter()
            .filter_map(|t| self.syscall_freq.get(t))
            .sum();
        io_count as f64 / self.total_syscalls.max(1) as f64
    }

    /// Network ratio
    #[inline]
    pub fn network_ratio(&self) -> f64 {
        let net_types = [41u64, 49, 50, 43, 42, 44, 45]; // socket, bind, listen, accept, connect, send, recv
        let net_count: u64 = net_types
            .iter()
            .filter_map(|t| self.syscall_freq.get(t))
            .sum();
        net_count as f64 / self.total_syscalls.max(1) as f64
    }

    /// Memory ratio
    #[inline]
    pub fn memory_ratio(&self) -> f64 {
        let mem_types = [9u64, 11, 10, 12]; // mmap, munmap, mprotect, brk
        let mem_count: u64 = mem_types
            .iter()
            .filter_map(|t| self.syscall_freq.get(t))
            .sum();
        mem_count as f64 / self.total_syscalls.max(1) as f64
    }
}

// ============================================================================
// APPLICATION PROFILE
// ============================================================================

/// Complete profile for an application, built from observations
#[derive(Debug, Clone)]
pub struct AppProfile {
    /// Process ID
    pub pid: u64,
    /// Application name (if known)
    pub name: Option<String>,
    /// Dominant classification
    pub dominant_class: AppClass,
    /// Secondary classification (if mixed)
    pub secondary_class: Option<AppClass>,
    /// Behavior observations
    pub behavior: AppBehavior,
    /// Resource usage patterns
    pub resource_pattern: ResourceUsagePattern,
    /// Confidence in this profile (0.0 - 1.0)
    pub confidence: f64,
    /// Number of observation windows
    pub observation_count: u64,
    /// Recommended optimizations
    pub recommendations: Vec<ProfileRecommendation>,
}

/// A recommendation derived from profiling
#[derive(Debug, Clone)]
pub struct ProfileRecommendation {
    /// What to optimize
    pub category: &'static str,
    /// Specific recommendation
    pub description: String,
    /// Expected improvement (0.0 - 1.0)
    pub expected_improvement: f64,
}

// ============================================================================
// APPLICATION PROFILER
// ============================================================================

/// The profiler — observes syscall patterns and builds application profiles.
#[repr(align(64))]
pub struct AppProfiler {
    /// Syscall type observations
    syscall_counts: LinearMap<u64, 64>,
    /// Total syscalls observed
    total_syscalls: u64,
    /// Total I/O data size
    total_io_bytes: usize,
    /// I/O syscall count (for average size)
    io_syscall_count: u64,
    /// Recent syscall types (for sequence detection)
    recent_types: VecDeque<SyscallType>,
    /// Max recent history
    max_recent: usize,
    /// Last syscall timestamp
    last_timestamp: u64,
    /// Inter-syscall time accumulator
    inter_times: Vec<u64>,
}

impl AppProfiler {
    /// Create a new profiler
    pub fn new(max_recent: usize) -> Self {
        Self {
            syscall_counts: LinearMap::new(),
            total_syscalls: 0,
            total_io_bytes: 0,
            io_syscall_count: 0,
            recent_types: Vec::with_capacity(max_recent),
            max_recent,
            last_timestamp: 0,
            inter_times: Vec::new(),
        }
    }

    /// Record a syscall observation
    pub fn record_syscall(&mut self, syscall_type: SyscallType, data_size: usize) {
        let key = syscall_type.from_number_reverse_pub();
        self.syscall_counts.add(key, 1);
        self.total_syscalls += 1;

        if syscall_type.is_io() {
            self.total_io_bytes += data_size;
            self.io_syscall_count += 1;
        }

        // Track recent types
        if self.recent_types.len() >= self.max_recent {
            self.recent_types.pop_front();
        }
        self.recent_types.push_back(syscall_type);
    }

    /// Record a timestamp for inter-syscall timing
    #[inline]
    pub fn record_timestamp(&mut self, timestamp: u64) {
        if self.last_timestamp > 0 && timestamp > self.last_timestamp {
            let delta = timestamp - self.last_timestamp;
            if self.inter_times.len() < 1000 {
                self.inter_times.push(delta);
            }
        }
        self.last_timestamp = timestamp;
    }

    /// Build a complete profile from observations
    pub fn build_profile(&self) -> AppProfile {
        let behavior = self.build_behavior();
        let dominant_class = self.classify(&behavior);
        let confidence = self.compute_confidence();
        let recommendations = self.generate_recommendations(dominant_class, &behavior);

        AppProfile {
            pid: 0,
            name: None,
            dominant_class,
            secondary_class: None,
            behavior,
            resource_pattern: ResourceUsagePattern::new(),
            confidence,
            observation_count: self.total_syscalls,
            recommendations,
        }
    }

    /// Build behavior from observations
    fn build_behavior(&self) -> AppBehavior {
        let mut behavior = AppBehavior::new();
        behavior.syscall_freq = self.syscall_counts.clone();
        behavior.total_syscalls = self.total_syscalls;

        if self.io_syscall_count > 0 {
            behavior.avg_io_size = self.total_io_bytes / self.io_syscall_count as usize;
        }

        // Detect sequential patterns
        if self.recent_types.len() >= 5 {
            let read_count = self
                .recent_types
                .iter()
                .filter(|t| **t == SyscallType::Read)
                .count();
            let write_count = self
                .recent_types
                .iter()
                .filter(|t| **t == SyscallType::Write)
                .count();

            behavior.sequential_reads = read_count as f64 / self.recent_types.len() as f64 > 0.6;
            behavior.sequential_writes = write_count as f64 / self.recent_types.len() as f64 > 0.6;
        }

        // Detect mmap usage
        behavior.uses_mmap = self.syscall_counts.get(9).copied().unwrap_or(0) > 0;

        // Inter-syscall timing
        if !self.inter_times.is_empty() {
            let sum: u64 = self.inter_times.iter().sum();
            behavior.avg_inter_syscall_ns = sum / self.inter_times.len() as u64;

            let mean = behavior.avg_inter_syscall_ns as f64;
            let variance: f64 = self
                .inter_times
                .iter()
                .map(|t| {
                    let diff = *t as f64 - mean;
                    diff * diff
                })
                .sum::<f64>()
                / self.inter_times.len() as f64;
            behavior.inter_syscall_stddev = libm::sqrt(variance);
        }

        behavior
    }

    /// Classify an application based on behavior
    fn classify(&self, behavior: &AppBehavior) -> AppClass {
        let io_ratio = behavior.io_ratio();
        let net_ratio = behavior.network_ratio();
        let mem_ratio = behavior.memory_ratio();

        // Dominant behavior classification
        if io_ratio > 0.5 {
            AppClass::IoIntensive
        } else if net_ratio > 0.4 {
            AppClass::NetworkIntensive
        } else if mem_ratio > 0.3 {
            AppClass::MemoryIntensive
        } else if behavior.avg_inter_syscall_ns > 10_000_000 {
            // Long gaps between syscalls → CPU-bound
            AppClass::CpuIntensive
        } else if behavior.avg_inter_syscall_ns < 100_000
            && behavior.inter_syscall_stddev < 50_000.0
        {
            // Very regular, fast syscalls → real-time
            AppClass::RealTime
        } else if behavior.inter_syscall_stddev > behavior.avg_inter_syscall_ns as f64 * 2.0 {
            // Very irregular → interactive
            AppClass::Interactive
        } else if self.total_syscalls < 50 {
            AppClass::Unknown
        } else {
            AppClass::Mixed
        }
    }

    /// Compute confidence in the profile
    fn compute_confidence(&self) -> f64 {
        // Confidence increases with observations
        let obs_factor = (self.total_syscalls as f64 / 100.0).min(1.0);
        // Confidence increases with diversity of observations
        let diversity = self.syscall_counts.len() as f64 / 10.0;
        let diversity_factor = diversity.min(1.0);

        (obs_factor * 0.7 + diversity_factor * 0.3).min(1.0)
    }

    /// Generate optimization recommendations
    fn generate_recommendations(
        &self,
        class: AppClass,
        behavior: &AppBehavior,
    ) -> Vec<ProfileRecommendation> {
        let mut recs = Vec::new();

        match class {
            AppClass::IoIntensive => {
                if behavior.sequential_reads {
                    recs.push(ProfileRecommendation {
                        category: "prefetch",
                        description: String::from(
                            "Enable aggressive read-ahead for sequential I/O pattern",
                        ),
                        expected_improvement: 0.35,
                    });
                }
                if behavior.avg_io_size < 4096 {
                    recs.push(ProfileRecommendation {
                        category: "batching",
                        description: String::from(
                            "Small I/O detected — batch reads into larger transfers",
                        ),
                        expected_improvement: 0.25,
                    });
                }
            },
            AppClass::NetworkIntensive => {
                recs.push(ProfileRecommendation {
                    category: "batching",
                    description: String::from("Batch network sends for throughput improvement"),
                    expected_improvement: 0.20,
                });
            },
            AppClass::Interactive => {
                recs.push(ProfileRecommendation {
                    category: "latency",
                    description: String::from(
                        "Use fast-path routing for latency-sensitive operations",
                    ),
                    expected_improvement: 0.15,
                });
            },
            AppClass::RealTime => {
                recs.push(ProfileRecommendation {
                    category: "prediction",
                    description: String::from(
                        "Enable syscall prediction for deterministic real-time patterns",
                    ),
                    expected_improvement: 0.30,
                });
            },
            _ => {},
        }

        // General recommendations
        if behavior.uses_mmap {
            recs.push(ProfileRecommendation {
                category: "memory",
                description: String::from("Use large pages for memory-mapped regions"),
                expected_improvement: 0.10,
            });
        }

        recs
    }

    /// Reset the profiler
    #[inline]
    pub fn reset(&mut self) {
        self.syscall_counts.clear();
        self.total_syscalls = 0;
        self.total_io_bytes = 0;
        self.io_syscall_count = 0;
        self.recent_types.clear();
        self.last_timestamp = 0;
        self.inter_times.clear();
    }
}
