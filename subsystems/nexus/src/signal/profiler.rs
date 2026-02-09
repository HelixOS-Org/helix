//! Signal Handler Profiler
//!
//! Profiles signal handler execution for performance analysis.

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{ProcessId, SignalNumber};

/// Handler execution sample
#[derive(Debug, Clone, Copy)]
pub struct HandlerSample {
    /// Signal number
    pub signo: SignalNumber,
    /// Execution time (nanoseconds)
    pub duration_ns: u64,
    /// Process ID
    pub pid: ProcessId,
    /// Timestamp
    pub timestamp: u64,
}

/// Handler statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HandlerStats {
    /// Signal number
    pub signo: SignalNumber,
    /// Execution count
    pub execution_count: u64,
    /// Total execution time (nanoseconds)
    pub total_time_ns: u64,
    /// Minimum execution time
    pub min_time_ns: u64,
    /// Maximum execution time
    pub max_time_ns: u64,
    /// Failures/errors count
    pub failure_count: u64,
    /// Async-signal-unsafe calls detected
    pub unsafe_calls: u64,
    /// Nested signal handlers detected
    pub nested_handlers: u64,
}

impl HandlerStats {
    /// Create new handler stats
    pub fn new(signo: SignalNumber) -> Self {
        Self {
            signo,
            execution_count: 0,
            total_time_ns: 0,
            min_time_ns: u64::MAX,
            max_time_ns: 0,
            failure_count: 0,
            unsafe_calls: 0,
            nested_handlers: 0,
        }
    }

    /// Calculate average execution time
    #[inline]
    pub fn avg_time_ns(&self) -> u64 {
        if self.execution_count == 0 {
            return 0;
        }
        self.total_time_ns / self.execution_count
    }

    /// Calculate failure rate
    #[inline]
    pub fn failure_rate(&self) -> f32 {
        if self.execution_count == 0 {
            return 0.0;
        }
        self.failure_count as f32 / self.execution_count as f32
    }
}

/// Signal handler profiler
pub struct HandlerProfiler {
    /// Per-signal statistics
    stats: BTreeMap<SignalNumber, HandlerStats>,
    /// Per-process per-signal statistics
    per_process_stats: BTreeMap<(ProcessId, SignalNumber), HandlerStats>,
    /// Recent samples
    samples: VecDeque<HandlerSample>,
    /// Maximum samples
    max_samples: usize,
    /// Slow handler threshold (nanoseconds)
    slow_threshold_ns: u64,
    /// Slow handler count
    slow_handlers: AtomicU64,
    /// Currently executing handlers (for nesting detection)
    executing: BTreeMap<ProcessId, Vec<SignalNumber>>,
}

impl HandlerProfiler {
    /// Create new handler profiler
    pub fn new() -> Self {
        Self {
            stats: BTreeMap::new(),
            per_process_stats: BTreeMap::new(),
            samples: Vec::with_capacity(1000),
            max_samples: 1000,
            slow_threshold_ns: 10_000_000, // 10ms
            slow_handlers: AtomicU64::new(0),
            executing: BTreeMap::new(),
        }
    }

    /// Record handler entry
    pub fn record_entry(&mut self, pid: ProcessId, signo: SignalNumber) {
        let stack = self.executing.entry(pid).or_default();

        // Check for nested handler
        if !stack.is_empty() {
            self.stats
                .entry(signo)
                .or_insert_with(|| HandlerStats::new(signo))
                .nested_handlers += 1;
        }

        stack.push(signo);
    }

    /// Record handler exit
    pub fn record_exit(
        &mut self,
        pid: ProcessId,
        signo: SignalNumber,
        duration_ns: u64,
        failed: bool,
        timestamp: u64,
    ) {
        // Update executing stack
        if let Some(stack) = self.executing.get_mut(&pid) {
            stack.retain(|s| *s != signo);
        }

        // Update global stats
        let stats = self
            .stats
            .entry(signo)
            .or_insert_with(|| HandlerStats::new(signo));

        stats.execution_count += 1;
        stats.total_time_ns += duration_ns;
        stats.min_time_ns = stats.min_time_ns.min(duration_ns);
        stats.max_time_ns = stats.max_time_ns.max(duration_ns);
        if failed {
            stats.failure_count += 1;
        }

        // Update per-process stats
        let key = (pid, signo);
        let pstats = self
            .per_process_stats
            .entry(key)
            .or_insert_with(|| HandlerStats::new(signo));
        pstats.execution_count += 1;
        pstats.total_time_ns += duration_ns;

        // Check for slow handler
        if duration_ns > self.slow_threshold_ns {
            self.slow_handlers.fetch_add(1, Ordering::Relaxed);
        }

        // Store sample
        let sample = HandlerSample {
            signo,
            duration_ns,
            pid,
            timestamp,
        };
        if self.samples.len() >= self.max_samples {
            self.samples.pop_front();
        }
        self.samples.push_back(sample);
    }

    /// Record async-signal-unsafe call
    #[inline]
    pub fn record_unsafe_call(&mut self, signo: SignalNumber) {
        self.stats
            .entry(signo)
            .or_insert_with(|| HandlerStats::new(signo))
            .unsafe_calls += 1;
    }

    /// Get handler stats
    #[inline(always)]
    pub fn get_stats(&self, signo: SignalNumber) -> Option<&HandlerStats> {
        self.stats.get(&signo)
    }

    /// Get per-process stats
    #[inline(always)]
    pub fn get_process_stats(&self, pid: ProcessId, signo: SignalNumber) -> Option<&HandlerStats> {
        self.per_process_stats.get(&(pid, signo))
    }

    /// Get slow handler count
    #[inline(always)]
    pub fn slow_handler_count(&self) -> u64 {
        self.slow_handlers.load(Ordering::Relaxed)
    }

    /// Get problematic handlers (slow or with issues)
    pub fn get_problematic_handlers(&self) -> Vec<(SignalNumber, &HandlerStats)> {
        self.stats
            .iter()
            .filter(|(_, stats)| {
                stats.avg_time_ns() > self.slow_threshold_ns
                    || stats.failure_rate() > 0.1
                    || stats.unsafe_calls > 0
                    || stats.nested_handlers > 0
            })
            .map(|(sig, stats)| (*sig, stats))
            .collect()
    }

    /// Set slow threshold
    #[inline(always)]
    pub fn set_slow_threshold(&mut self, threshold_ns: u64) {
        self.slow_threshold_ns = threshold_ns;
    }

    /// Get slow threshold
    #[inline(always)]
    pub fn slow_threshold(&self) -> u64 {
        self.slow_threshold_ns
    }

    /// Get total execution count
    #[inline(always)]
    pub fn total_executions(&self) -> u64 {
        self.stats.values().map(|s| s.execution_count).sum()
    }

    /// Get recent samples
    #[inline(always)]
    pub fn get_samples(&self) -> &[HandlerSample] {
        &self.samples
    }
}

impl Default for HandlerProfiler {
    fn default() -> Self {
        Self::new()
    }
}
