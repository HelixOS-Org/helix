//! # Application Syscall Profile
//!
//! Per-application syscall pattern analysis:
//! - Syscall frequency tracking
//! - Syscall latency profiling
//! - Pattern detection
//! - Bottleneck identification
//! - Syscall cost breakdown

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SYSCALL CATEGORY
// ============================================================================

/// Syscall category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyscallCategory {
    /// File I/O
    FileIo,
    /// Network I/O
    NetworkIo,
    /// Memory management
    Memory,
    /// Process management
    Process,
    /// Synchronization
    Sync,
    /// Signal handling
    Signal,
    /// Timer/clock
    Timer,
    /// IPC
    Ipc,
    /// Device I/O
    DeviceIo,
    /// Other
    Other,
}

/// Syscall descriptor
#[derive(Debug, Clone)]
pub struct SyscallDescriptor {
    /// Syscall number
    pub number: u32,
    /// Category
    pub category: SyscallCategory,
    /// Expected cost class
    pub cost_class: SyscallCostClass,
}

/// Cost class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallCostClass {
    /// Fast path (< 1μs)
    Fast,
    /// Medium (1-100μs)
    Medium,
    /// Slow (100μs-10ms)
    Slow,
    /// Very slow (> 10ms)
    VerySlow,
    /// Variable
    Variable,
}

// ============================================================================
// SYSCALL COUNTER
// ============================================================================

/// Per-syscall counter
#[derive(Debug, Clone)]
pub struct SyscallCounter {
    /// Syscall number
    pub number: u32,
    /// Total invocations
    pub count: u64,
    /// Total latency (ns)
    pub total_latency_ns: u64,
    /// Min latency (ns)
    pub min_latency_ns: u64,
    /// Max latency (ns)
    pub max_latency_ns: u64,
    /// Error count
    pub error_count: u64,
    /// Sum of squares (for variance)
    sum_sq: f64,
}

impl SyscallCounter {
    pub fn new(number: u32) -> Self {
        Self {
            number,
            count: 0,
            total_latency_ns: 0,
            min_latency_ns: u64::MAX,
            max_latency_ns: 0,
            error_count: 0,
            sum_sq: 0.0,
        }
    }

    /// Record invocation
    pub fn record(&mut self, latency_ns: u64, is_error: bool) {
        self.count += 1;
        self.total_latency_ns += latency_ns;
        if latency_ns < self.min_latency_ns {
            self.min_latency_ns = latency_ns;
        }
        if latency_ns > self.max_latency_ns {
            self.max_latency_ns = latency_ns;
        }
        if is_error {
            self.error_count += 1;
        }
        self.sum_sq += (latency_ns as f64) * (latency_ns as f64);
    }

    /// Average latency
    pub fn avg_latency_ns(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.total_latency_ns as f64 / self.count as f64
    }

    /// Variance
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        let mean = self.avg_latency_ns();
        self.sum_sq / self.count as f64 - mean * mean
    }

    /// Std deviation
    pub fn std_dev(&self) -> f64 {
        let var = self.variance();
        if var <= 0.0 {
            return 0.0;
        }
        libm::sqrt(var)
    }

    /// Error rate
    pub fn error_rate(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.error_count as f64 / self.count as f64
    }
}

// ============================================================================
// SYSCALL PATTERN
// ============================================================================

/// Detected syscall pattern
#[derive(Debug, Clone)]
pub struct SyscallPattern {
    /// Pattern sequence
    pub sequence: Vec<u32>,
    /// Occurrence count
    pub occurrences: u64,
    /// Average total latency for the pattern
    pub avg_latency_ns: f64,
    /// Pattern type
    pub pattern_type: PatternType,
}

/// Pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// Read-write cycle
    ReadWrite,
    /// Poll loop
    PollLoop,
    /// Allocate-free cycle
    AllocFree,
    /// Open-read-close
    OpenReadClose,
    /// Lock-unlock
    LockUnlock,
    /// Generic sequence
    Generic,
}

/// Simple n-gram pattern detector
#[derive(Debug, Clone)]
pub struct PatternDetector {
    /// Recent syscalls
    recent: Vec<u32>,
    /// Max window
    max_window: usize,
    /// N-gram counts (hash -> count)
    ngram_counts: BTreeMap<u64, u64>,
    /// N-gram to sequence
    ngram_sequences: BTreeMap<u64, Vec<u32>>,
    /// N-gram size
    ngram_size: usize,
}

impl PatternDetector {
    pub fn new(ngram_size: usize) -> Self {
        Self {
            recent: Vec::new(),
            max_window: 256,
            ngram_counts: BTreeMap::new(),
            ngram_sequences: BTreeMap::new(),
            ngram_size,
        }
    }

    /// Record syscall
    pub fn record(&mut self, syscall: u32) {
        self.recent.push(syscall);
        if self.recent.len() > self.max_window {
            self.recent.remove(0);
        }

        if self.recent.len() >= self.ngram_size {
            let start = self.recent.len() - self.ngram_size;
            let gram = &self.recent[start..];
            let hash = self.hash_sequence(gram);
            *self.ngram_counts.entry(hash).or_insert(0) += 1;
            self.ngram_sequences
                .entry(hash)
                .or_insert_with(|| gram.to_vec());
        }
    }

    fn hash_sequence(&self, seq: &[u32]) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for &s in seq {
            h ^= s as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    /// Get top patterns
    pub fn top_patterns(&self, limit: usize) -> Vec<SyscallPattern> {
        let mut entries: Vec<_> = self.ngram_counts.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1));

        entries
            .iter()
            .take(limit)
            .filter_map(|(&hash, &count)| {
                let seq = self.ngram_sequences.get(&hash)?;
                Some(SyscallPattern {
                    sequence: seq.clone(),
                    occurrences: count,
                    avg_latency_ns: 0.0,
                    pattern_type: PatternType::Generic,
                })
            })
            .collect()
    }
}

// ============================================================================
// BOTTLENECK
// ============================================================================

/// Syscall bottleneck
#[derive(Debug, Clone)]
pub struct SyscallBottleneck {
    /// Syscall number
    pub syscall: u32,
    /// Category
    pub category: SyscallCategory,
    /// Total time spent (ns)
    pub total_time_ns: u64,
    /// Fraction of total syscall time
    pub time_fraction: f64,
    /// Average latency
    pub avg_latency_ns: f64,
    /// Bottleneck type
    pub bottleneck_type: BottleneckType,
}

/// Bottleneck type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottleneckType {
    /// High frequency
    HighFrequency,
    /// High latency
    HighLatency,
    /// High error rate
    HighErrorRate,
    /// High variance
    HighVariance,
}

// ============================================================================
// APP SYSCALL PROFILER
// ============================================================================

/// Syscall profiler stats
#[derive(Debug, Clone, Default)]
pub struct AppSyscallProfileStats {
    /// Tracked processes
    pub process_count: usize,
    /// Total syscalls recorded
    pub total_syscalls: u64,
    /// Total syscall time (ns)
    pub total_time_ns: u64,
    /// Unique syscalls seen
    pub unique_syscalls: usize,
    /// Bottlenecks found
    pub bottleneck_count: usize,
}

/// Per-process syscall profile
#[derive(Debug, Clone)]
pub struct ProcessSyscallProfile {
    /// Process ID
    pub pid: u64,
    /// Per-syscall counters
    pub counters: BTreeMap<u32, SyscallCounter>,
    /// Pattern detector
    pub pattern_detector: PatternDetector,
    /// Category totals (ns)
    pub category_time: BTreeMap<u8, u64>,
    /// Total time
    pub total_time_ns: u64,
    /// Total calls
    pub total_calls: u64,
}

impl ProcessSyscallProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            counters: BTreeMap::new(),
            pattern_detector: PatternDetector::new(3),
            category_time: BTreeMap::new(),
            total_time_ns: 0,
            total_calls: 0,
        }
    }

    /// Record syscall
    pub fn record(&mut self, number: u32, category: SyscallCategory, latency_ns: u64, error: bool) {
        let counter = self
            .counters
            .entry(number)
            .or_insert_with(|| SyscallCounter::new(number));
        counter.record(latency_ns, error);
        self.pattern_detector.record(number);
        *self.category_time.entry(category as u8).or_insert(0) += latency_ns;
        self.total_time_ns += latency_ns;
        self.total_calls += 1;
    }

    /// Find bottlenecks
    pub fn find_bottlenecks(&self) -> Vec<SyscallBottleneck> {
        let mut bottlenecks = Vec::new();

        for (&number, counter) in &self.counters {
            let time_frac = if self.total_time_ns > 0 {
                counter.total_latency_ns as f64 / self.total_time_ns as f64
            } else {
                0.0
            };

            let bottleneck_type = if time_frac > 0.3 {
                Some(BottleneckType::HighFrequency)
            } else if counter.avg_latency_ns() > 1_000_000.0 {
                Some(BottleneckType::HighLatency)
            } else if counter.error_rate() > 0.1 {
                Some(BottleneckType::HighErrorRate)
            } else if counter.std_dev() > counter.avg_latency_ns() * 2.0 {
                Some(BottleneckType::HighVariance)
            } else {
                None
            };

            if let Some(bt) = bottleneck_type {
                bottlenecks.push(SyscallBottleneck {
                    syscall: number,
                    category: SyscallCategory::Other,
                    total_time_ns: counter.total_latency_ns,
                    time_fraction: time_frac,
                    avg_latency_ns: counter.avg_latency_ns(),
                    bottleneck_type: bt,
                });
            }
        }

        bottlenecks.sort_by(|a, b| {
            b.total_time_ns
                .partial_cmp(&a.total_time_ns)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        bottlenecks
    }
}

/// Application syscall profiler
pub struct AppSyscallProfiler {
    /// Per-process profiles
    profiles: BTreeMap<u64, ProcessSyscallProfile>,
    /// Syscall descriptors
    descriptors: BTreeMap<u32, SyscallDescriptor>,
    /// Stats
    stats: AppSyscallProfileStats,
}

impl AppSyscallProfiler {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            descriptors: BTreeMap::new(),
            stats: AppSyscallProfileStats::default(),
        }
    }

    /// Register syscall descriptor
    pub fn register_syscall(&mut self, desc: SyscallDescriptor) {
        self.descriptors.insert(desc.number, desc);
    }

    /// Record syscall
    pub fn record(
        &mut self,
        pid: u64,
        number: u32,
        latency_ns: u64,
        error: bool,
    ) {
        let category = self
            .descriptors
            .get(&number)
            .map(|d| d.category)
            .unwrap_or(SyscallCategory::Other);

        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(|| ProcessSyscallProfile::new(pid));
        profile.record(number, category, latency_ns, error);

        self.stats.total_syscalls += 1;
        self.stats.total_time_ns += latency_ns;
        self.stats.process_count = self.profiles.len();
        self.stats.unique_syscalls = self
            .profiles
            .values()
            .flat_map(|p| p.counters.keys())
            .collect::<Vec<_>>()
            .len();
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessSyscallProfile> {
        self.profiles.get(&pid)
    }

    /// Find bottlenecks for process
    pub fn find_bottlenecks(&self, pid: u64) -> Vec<SyscallBottleneck> {
        self.profiles
            .get(&pid)
            .map(|p| p.find_bottlenecks())
            .unwrap_or_default()
    }

    /// Stats
    pub fn stats(&self) -> &AppSyscallProfileStats {
        &self.stats
    }
}
