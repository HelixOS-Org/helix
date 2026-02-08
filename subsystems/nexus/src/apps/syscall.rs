//! # Apps Syscall V2
//!
//! Enhanced syscall profiling with deep analysis:
//! - Syscall chains and sequences
//! - Argument pattern analysis
//! - Error code frequency profiling
//! - Inter-syscall latency correlation
//! - Predictive next-syscall estimation
//! - Hotspot identification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Syscall result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallV2Result {
    /// Success with return value
    Success(i64),
    /// Error with errno
    Error(i32),
    /// Interrupted
    Interrupted,
    /// Timed out
    TimedOut,
    /// Restarted
    Restarted,
}

/// Syscall argument summary
#[derive(Debug, Clone)]
pub struct SyscallArgPattern {
    pub arg_index: u8,
    pub min_value: u64,
    pub max_value: u64,
    pub most_common: u64,
    pub most_common_count: u64,
    pub distinct_values: u32,
}

impl SyscallArgPattern {
    pub fn new(arg_index: u8) -> Self {
        Self {
            arg_index,
            min_value: u64::MAX,
            max_value: 0,
            most_common: 0,
            most_common_count: 0,
            distinct_values: 0,
        }
    }

    pub fn record(&mut self, val: u64) {
        if val < self.min_value {
            self.min_value = val;
        }
        if val > self.max_value {
            self.max_value = val;
        }
    }

    pub fn range(&self) -> u64 {
        if self.max_value >= self.min_value {
            self.max_value - self.min_value
        } else {
            0
        }
    }
}

/// Per-syscall-number statistics
#[derive(Debug, Clone)]
pub struct SyscallV2Stats {
    pub syscall_nr: u32,
    pub count: u64,
    pub total_latency_ns: u64,
    pub min_latency_ns: u64,
    pub max_latency_ns: u64,
    pub success_count: u64,
    pub error_count: u64,
    /// Error frequency by errno
    error_freq: BTreeMap<i32, u64>,
    /// Latency histogram buckets (log2)
    pub latency_buckets: [u64; 20],
    /// Argument patterns (up to 6 args)
    arg_patterns: Vec<SyscallArgPattern>,
}

impl SyscallV2Stats {
    pub fn new(syscall_nr: u32) -> Self {
        let mut arg_patterns = Vec::new();
        for i in 0..6 {
            arg_patterns.push(SyscallArgPattern::new(i));
        }
        Self {
            syscall_nr,
            count: 0,
            total_latency_ns: 0,
            min_latency_ns: u64::MAX,
            max_latency_ns: 0,
            success_count: 0,
            error_count: 0,
            error_freq: BTreeMap::new(),
            latency_buckets: [0; 20],
            arg_patterns,
        }
    }

    pub fn record(&mut self, latency_ns: u64, result: SyscallV2Result, args: &[u64]) {
        self.count += 1;
        self.total_latency_ns += latency_ns;
        if latency_ns < self.min_latency_ns {
            self.min_latency_ns = latency_ns;
        }
        if latency_ns > self.max_latency_ns {
            self.max_latency_ns = latency_ns;
        }

        match result {
            SyscallV2Result::Success(_) => self.success_count += 1,
            SyscallV2Result::Error(errno) => {
                self.error_count += 1;
                *self.error_freq.entry(errno).or_insert(0) += 1;
            }
            _ => {}
        }

        // Latency histogram
        let bucket = if latency_ns == 0 {
            0
        } else {
            let log = (latency_ns as f64).log2() as usize;
            if log >= 20 { 19 } else { log }
        };
        self.latency_buckets[bucket] += 1;

        // Argument patterns
        for (i, &val) in args.iter().enumerate().take(6) {
            if i < self.arg_patterns.len() {
                self.arg_patterns[i].record(val);
            }
        }
    }

    pub fn avg_latency_ns(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.total_latency_ns as f64 / self.count as f64 }
    }

    pub fn error_rate(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.error_count as f64 / self.count as f64 }
    }

    /// Top errors
    pub fn top_errors(&self, n: usize) -> Vec<(i32, u64)> {
        let mut errors: Vec<(i32, u64)> = self.error_freq.iter()
            .map(|(&errno, &count)| (errno, count))
            .collect();
        errors.sort_by(|a, b| b.1.cmp(&a.1));
        errors.truncate(n);
        errors
    }

    /// Latency P50 estimate from histogram
    pub fn p50_latency_ns(&self) -> u64 {
        if self.count == 0 {
            return 0;
        }
        let target = self.count / 2;
        let mut cumulative = 0u64;
        for (i, &count) in self.latency_buckets.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return 1u64 << i;
            }
        }
        self.max_latency_ns
    }

    /// Latency P99 estimate
    pub fn p99_latency_ns(&self) -> u64 {
        if self.count == 0 {
            return 0;
        }
        let target = self.count * 99 / 100;
        let mut cumulative = 0u64;
        for (i, &count) in self.latency_buckets.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return 1u64 << i;
            }
        }
        self.max_latency_ns
    }
}

/// Syscall sequence (bigram)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SyscallBigram {
    pub first: u32,
    pub second: u32,
}

/// Per-process syscall V2 profile
#[derive(Debug)]
pub struct ProcessSyscallV2Profile {
    pub pid: u64,
    pub name: String,
    /// Per-syscall stats
    syscall_stats: BTreeMap<u32, SyscallV2Stats>,
    /// Bigram frequencies
    bigrams: BTreeMap<SyscallBigram, u64>,
    /// Last syscall for bigram tracking
    last_syscall: Option<u32>,
    pub total_syscalls: u64,
    pub total_errors: u64,
}

impl ProcessSyscallV2Profile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            name: String::new(),
            syscall_stats: BTreeMap::new(),
            bigrams: BTreeMap::new(),
            last_syscall: None,
            total_syscalls: 0,
            total_errors: 0,
        }
    }

    pub fn record(
        &mut self,
        syscall_nr: u32,
        latency_ns: u64,
        result: SyscallV2Result,
        args: &[u64],
    ) {
        self.total_syscalls += 1;
        if matches!(result, SyscallV2Result::Error(_)) {
            self.total_errors += 1;
        }

        self.syscall_stats.entry(syscall_nr)
            .or_insert_with(|| SyscallV2Stats::new(syscall_nr))
            .record(latency_ns, result, args);

        // Track bigram
        if let Some(prev) = self.last_syscall {
            let bg = SyscallBigram { first: prev, second: syscall_nr };
            *self.bigrams.entry(bg).or_insert(0) += 1;
        }
        self.last_syscall = Some(syscall_nr);
    }

    /// Predict next syscall
    pub fn predict_next(&self) -> Option<u32> {
        let last = self.last_syscall?;
        self.bigrams.iter()
            .filter(|(bg, _)| bg.first == last)
            .max_by_key(|(_, &count)| count)
            .map(|(bg, _)| bg.second)
    }

    /// Hottest syscalls
    pub fn hottest(&self, n: usize) -> Vec<(u32, u64)> {
        let mut syscalls: Vec<(u32, u64)> = self.syscall_stats.iter()
            .map(|(&nr, stats)| (nr, stats.count))
            .collect();
        syscalls.sort_by(|a, b| b.1.cmp(&a.1));
        syscalls.truncate(n);
        syscalls
    }

    /// Slowest syscalls by avg latency
    pub fn slowest(&self, n: usize) -> Vec<(u32, f64)> {
        let mut syscalls: Vec<(u32, f64)> = self.syscall_stats.iter()
            .filter(|(_, s)| s.count > 10)
            .map(|(&nr, stats)| (nr, stats.avg_latency_ns()))
            .collect();
        syscalls.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        syscalls.truncate(n);
        syscalls
    }

    /// Most error-prone syscalls
    pub fn error_prone(&self, n: usize) -> Vec<(u32, f64)> {
        let mut syscalls: Vec<(u32, f64)> = self.syscall_stats.iter()
            .filter(|(_, s)| s.count > 10)
            .map(|(&nr, stats)| (nr, stats.error_rate()))
            .collect();
        syscalls.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        syscalls.truncate(n);
        syscalls
    }

    /// Common syscall patterns (top bigrams)
    pub fn common_patterns(&self, n: usize) -> Vec<(SyscallBigram, u64)> {
        let mut patterns: Vec<(SyscallBigram, u64)> = self.bigrams.iter()
            .map(|(&bg, &count)| (bg, count))
            .collect();
        patterns.sort_by(|a, b| b.1.cmp(&a.1));
        patterns.truncate(n);
        patterns
    }

    /// Overall error rate
    pub fn overall_error_rate(&self) -> f64 {
        if self.total_syscalls == 0 { 0.0 } else {
            self.total_errors as f64 / self.total_syscalls as f64
        }
    }
}

/// Global stats
#[derive(Debug, Clone, Default)]
pub struct AppSyscallV2GlobalStats {
    pub tracked_processes: usize,
    pub total_syscalls: u64,
    pub total_errors: u64,
    pub distinct_syscalls: usize,
    pub avg_error_rate: f64,
}

/// App Syscall V2 Profiler
pub struct AppSyscallV2Profiler {
    processes: BTreeMap<u64, ProcessSyscallV2Profile>,
    stats: AppSyscallV2GlobalStats,
}

impl AppSyscallV2Profiler {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppSyscallV2GlobalStats::default(),
        }
    }

    pub fn record(
        &mut self,
        pid: u64,
        syscall_nr: u32,
        latency_ns: u64,
        result: SyscallV2Result,
        args: &[u64],
    ) {
        self.processes.entry(pid)
            .or_insert_with(|| ProcessSyscallV2Profile::new(pid))
            .record(syscall_nr, latency_ns, result, args);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_syscalls = self.processes.values().map(|p| p.total_syscalls).sum();
        self.stats.total_errors = self.processes.values().map(|p| p.total_errors).sum();
        if !self.processes.is_empty() {
            self.stats.avg_error_rate = self.processes.values()
                .map(|p| p.overall_error_rate())
                .sum::<f64>() / self.processes.len() as f64;
        }
    }

    pub fn stats(&self) -> &AppSyscallV2GlobalStats {
        &self.stats
    }
}
