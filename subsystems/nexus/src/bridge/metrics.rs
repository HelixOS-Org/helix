//! # Syscall Performance Metrics
//!
//! Comprehensive performance tracking for the syscall layer including:
//! - Per-syscall-type latency histograms
//! - Per-process syscall profiles
//! - Throughput tracking
//! - Error rate monitoring
//! - Trend analysis

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// LATENCY HISTOGRAM
// ============================================================================

/// Fixed-bucket histogram for latency tracking
/// Buckets: [0-1µs, 1-10µs, 10-100µs, 100µs-1ms, 1-10ms, 10-100ms, 100ms-1s, >1s]
const NUM_BUCKETS: usize = 8;
const BUCKET_BOUNDARIES_NS: [u64; 8] = [
    1_000,         // 1µs
    10_000,        // 10µs
    100_000,       // 100µs
    1_000_000,     // 1ms
    10_000_000,    // 10ms
    100_000_000,   // 100ms
    1_000_000_000, // 1s
    u64::MAX,      // >1s
];

/// Latency histogram
#[derive(Debug, Clone)]
pub struct LatencyHistogram {
    /// Bucket counts
    pub buckets: [u64; NUM_BUCKETS],
    /// Total samples
    pub count: u64,
    /// Sum of all latencies (ns)
    pub sum_ns: u64,
    /// Minimum latency (ns)
    pub min_ns: u64,
    /// Maximum latency (ns)
    pub max_ns: u64,
    /// Sum of squares (for variance calculation)
    sum_sq: f64,
}

impl LatencyHistogram {
    pub fn new() -> Self {
        Self {
            buckets: [0; NUM_BUCKETS],
            count: 0,
            sum_ns: 0,
            min_ns: u64::MAX,
            max_ns: 0,
            sum_sq: 0.0,
        }
    }

    /// Record a latency sample
    pub fn record(&mut self, latency_ns: u64) {
        self.count += 1;
        self.sum_ns += latency_ns;
        if latency_ns < self.min_ns {
            self.min_ns = latency_ns;
        }
        if latency_ns > self.max_ns {
            self.max_ns = latency_ns;
        }
        self.sum_sq += (latency_ns as f64) * (latency_ns as f64);

        // Find bucket
        for (i, &boundary) in BUCKET_BOUNDARIES_NS.iter().enumerate() {
            if latency_ns <= boundary {
                self.buckets[i] += 1;
                return;
            }
        }
        self.buckets[NUM_BUCKETS - 1] += 1;
    }

    /// Average latency (ns)
    pub fn avg_ns(&self) -> u64 {
        if self.count == 0 {
            0
        } else {
            self.sum_ns / self.count
        }
    }

    /// Standard deviation (ns)
    pub fn std_dev_ns(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        let mean = self.sum_ns as f64 / self.count as f64;
        let variance = (self.sum_sq / self.count as f64) - (mean * mean);
        if variance > 0.0 {
            libm::sqrt(variance)
        } else {
            0.0
        }
    }

    /// Approximate percentile (p50, p90, p95, p99)
    pub fn percentile(&self, pct: f64) -> u64 {
        if self.count == 0 {
            return 0;
        }
        let target = (self.count as f64 * pct / 100.0) as u64;
        let mut cumulative = 0u64;
        for (i, &count) in self.buckets.iter().enumerate() {
            cumulative += count;
            if cumulative >= target {
                return BUCKET_BOUNDARIES_NS[i];
            }
        }
        BUCKET_BOUNDARIES_NS[NUM_BUCKETS - 1]
    }

    /// p50
    pub fn p50(&self) -> u64 {
        self.percentile(50.0)
    }

    /// p90
    pub fn p90(&self) -> u64 {
        self.percentile(90.0)
    }

    /// p95
    pub fn p95(&self) -> u64 {
        self.percentile(95.0)
    }

    /// p99
    pub fn p99(&self) -> u64 {
        self.percentile(99.0)
    }

    /// Reset the histogram
    pub fn reset(&mut self) {
        self.buckets = [0; NUM_BUCKETS];
        self.count = 0;
        self.sum_ns = 0;
        self.min_ns = u64::MAX;
        self.max_ns = 0;
        self.sum_sq = 0.0;
    }
}

// ============================================================================
// THROUGHPUT TRACKER
// ============================================================================

/// Window-based throughput tracker
#[derive(Debug, Clone)]
pub struct ThroughputTracker {
    /// Window size in milliseconds
    window_ms: u64,
    /// Samples per window
    windows: Vec<WindowSample>,
    /// Max windows to keep
    max_windows: usize,
    /// Current window start
    current_window_start: u64,
    /// Current window count
    current_count: u64,
    /// Current window bytes
    current_bytes: u64,
}

/// A throughput sample for one window
#[derive(Debug, Clone, Copy)]
pub struct WindowSample {
    /// Window start timestamp
    pub start: u64,
    /// Operations count
    pub ops: u64,
    /// Bytes processed
    pub bytes: u64,
    /// Errors
    pub errors: u64,
}

impl ThroughputTracker {
    pub fn new(window_ms: u64) -> Self {
        Self {
            window_ms,
            windows: Vec::new(),
            max_windows: 60, // Keep 1 minute of history at 1s windows
            current_window_start: 0,
            current_count: 0,
            current_bytes: 0,
        }
    }

    /// Record an operation
    pub fn record(&mut self, bytes: u64, current_time: u64) {
        self.maybe_rotate(current_time);
        self.current_count += 1;
        self.current_bytes += bytes;
    }

    fn maybe_rotate(&mut self, current_time: u64) {
        if self.current_window_start == 0 {
            self.current_window_start = current_time;
            return;
        }

        if current_time.saturating_sub(self.current_window_start) >= self.window_ms {
            // Close current window
            let sample = WindowSample {
                start: self.current_window_start,
                ops: self.current_count,
                bytes: self.current_bytes,
                errors: 0,
            };

            if self.windows.len() >= self.max_windows {
                self.windows.remove(0);
            }
            self.windows.push(sample);

            self.current_window_start = current_time;
            self.current_count = 0;
            self.current_bytes = 0;
        }
    }

    /// Operations per second (average over recent windows)
    pub fn ops_per_sec(&self) -> f64 {
        if self.windows.is_empty() {
            return 0.0;
        }
        let recent = &self.windows[self.windows.len().saturating_sub(5)..];
        let total_ops: u64 = recent.iter().map(|w| w.ops).sum();
        let total_time_ms = recent.len() as u64 * self.window_ms;
        if total_time_ms == 0 {
            return 0.0;
        }
        total_ops as f64 / (total_time_ms as f64 / 1000.0)
    }

    /// Bytes per second (average over recent windows)
    pub fn bytes_per_sec(&self) -> f64 {
        if self.windows.is_empty() {
            return 0.0;
        }
        let recent = &self.windows[self.windows.len().saturating_sub(5)..];
        let total_bytes: u64 = recent.iter().map(|w| w.bytes).sum();
        let total_time_ms = recent.len() as u64 * self.window_ms;
        if total_time_ms == 0 {
            return 0.0;
        }
        total_bytes as f64 / (total_time_ms as f64 / 1000.0)
    }

    /// Get throughput trend (positive = increasing, negative = decreasing)
    pub fn trend(&self) -> f64 {
        if self.windows.len() < 4 {
            return 0.0;
        }
        let n = self.windows.len();
        let first_half: f64 = self.windows[..n / 2]
            .iter()
            .map(|w| w.ops as f64)
            .sum::<f64>()
            / (n / 2) as f64;
        let second_half: f64 = self.windows[n / 2..]
            .iter()
            .map(|w| w.ops as f64)
            .sum::<f64>()
            / (n - n / 2) as f64;

        if first_half < 0.001 {
            return 0.0;
        }
        (second_half - first_half) / first_half
    }
}

// ============================================================================
// ERROR TRACKER
// ============================================================================

/// Per-syscall-type error tracking
#[derive(Debug, Clone)]
pub struct ErrorTracker {
    /// Error counts per error code
    error_counts: BTreeMap<i32, u64>,
    /// Total errors
    total_errors: u64,
    /// Total successful calls
    total_success: u64,
    /// Recent error timestamps (for rate calculation)
    recent_errors: Vec<u64>,
    /// Max recent errors to store
    max_recent: usize,
}

impl ErrorTracker {
    pub fn new() -> Self {
        Self {
            error_counts: BTreeMap::new(),
            total_errors: 0,
            total_success: 0,
            recent_errors: Vec::new(),
            max_recent: 100,
        }
    }

    /// Record a result
    pub fn record(&mut self, return_value: i64, timestamp: u64) {
        if return_value < 0 {
            self.total_errors += 1;
            *self.error_counts.entry(return_value as i32).or_insert(0) += 1;
            if self.recent_errors.len() >= self.max_recent {
                self.recent_errors.remove(0);
            }
            self.recent_errors.push(timestamp);
        } else {
            self.total_success += 1;
        }
    }

    /// Error rate (0.0 - 1.0)
    pub fn error_rate(&self) -> f64 {
        let total = self.total_errors + self.total_success;
        if total == 0 {
            0.0
        } else {
            self.total_errors as f64 / total as f64
        }
    }

    /// Errors per second (over recent window)
    pub fn errors_per_sec(&self, current_time: u64) -> f64 {
        if self.recent_errors.is_empty() {
            return 0.0;
        }
        let window_start = current_time.saturating_sub(1000);
        let recent_count = self
            .recent_errors
            .iter()
            .filter(|&&t| t >= window_start)
            .count();
        recent_count as f64
    }

    /// Most common error code
    pub fn most_common_error(&self) -> Option<(i32, u64)> {
        self.error_counts
            .iter()
            .max_by_key(|(_, &v)| v)
            .map(|(&k, &v)| (k, v))
    }
}

// ============================================================================
// COMPREHENSIVE METRICS COLLECTOR
// ============================================================================

/// Per-syscall-type metrics
#[derive(Debug, Clone)]
pub struct SyscallTypeMetrics {
    /// Syscall type
    pub syscall_type: SyscallType,
    /// Latency histogram
    pub latency: LatencyHistogram,
    /// Error tracking
    pub errors: ErrorTracker,
    /// Throughput
    pub throughput: ThroughputTracker,
    /// Total invocations
    pub invocations: u64,
    /// Total bytes transferred
    pub bytes_transferred: u64,
}

impl SyscallTypeMetrics {
    pub fn new(syscall_type: SyscallType) -> Self {
        Self {
            syscall_type,
            latency: LatencyHistogram::new(),
            errors: ErrorTracker::new(),
            throughput: ThroughputTracker::new(1000),
            invocations: 0,
            bytes_transferred: 0,
        }
    }

    /// Record a syscall completion
    pub fn record(&mut self, latency_ns: u64, return_value: i64, bytes: u64, timestamp: u64) {
        self.invocations += 1;
        self.bytes_transferred += bytes;
        self.latency.record(latency_ns);
        self.errors.record(return_value, timestamp);
        self.throughput.record(bytes, timestamp);
    }
}

/// Per-process metrics
#[derive(Debug, Clone)]
pub struct ProcessSyscallMetrics {
    /// Process ID
    pub pid: u64,
    /// Total syscalls
    pub total_syscalls: u64,
    /// Total latency (ns)
    pub total_latency_ns: u64,
    /// Per-type breakdown
    pub per_type: BTreeMap<u8, u64>,
    /// First syscall timestamp
    pub first_seen: u64,
    /// Last syscall timestamp
    pub last_seen: u64,
    /// Total errors
    pub total_errors: u64,
}

impl ProcessSyscallMetrics {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            total_syscalls: 0,
            total_latency_ns: 0,
            per_type: BTreeMap::new(),
            first_seen: 0,
            last_seen: 0,
            total_errors: 0,
        }
    }

    pub fn record(
        &mut self,
        syscall_type: SyscallType,
        latency_ns: u64,
        success: bool,
        timestamp: u64,
    ) {
        self.total_syscalls += 1;
        self.total_latency_ns += latency_ns;
        *self.per_type.entry(syscall_type as u8).or_insert(0) += 1;
        if self.first_seen == 0 {
            self.first_seen = timestamp;
        }
        self.last_seen = timestamp;
        if !success {
            self.total_errors += 1;
        }
    }

    /// Average latency
    pub fn avg_latency_ns(&self) -> u64 {
        if self.total_syscalls == 0 {
            0
        } else {
            self.total_latency_ns / self.total_syscalls
        }
    }

    /// Syscalls per second
    pub fn rate(&self) -> f64 {
        let duration_ms = self.last_seen.saturating_sub(self.first_seen);
        if duration_ms == 0 {
            return 0.0;
        }
        self.total_syscalls as f64 / (duration_ms as f64 / 1000.0)
    }

    /// Top N most used syscall types
    pub fn top_types(&self, n: usize) -> Vec<(u8, u64)> {
        let mut types: Vec<(u8, u64)> = self.per_type.iter().map(|(&k, &v)| (k, v)).collect();
        types.sort_by(|a, b| b.1.cmp(&a.1));
        types.truncate(n);
        types
    }
}

// ============================================================================
// GLOBAL METRICS REGISTRY
// ============================================================================

/// Global syscall metrics registry
pub struct MetricsRegistry {
    /// Per-syscall-type metrics
    type_metrics: BTreeMap<u8, SyscallTypeMetrics>,
    /// Per-process metrics
    process_metrics: BTreeMap<u64, ProcessSyscallMetrics>,
    /// Global totals
    pub global_syscalls: u64,
    pub global_latency_ns: u64,
    pub global_errors: u64,
    pub global_bytes: u64,
    /// Max processes to track
    max_processes: usize,
}

impl MetricsRegistry {
    pub fn new(max_processes: usize) -> Self {
        Self {
            type_metrics: BTreeMap::new(),
            process_metrics: BTreeMap::new(),
            global_syscalls: 0,
            global_latency_ns: 0,
            global_errors: 0,
            global_bytes: 0,
            max_processes,
        }
    }

    /// Record a syscall completion
    pub fn record(
        &mut self,
        pid: u64,
        syscall_type: SyscallType,
        latency_ns: u64,
        return_value: i64,
        bytes: u64,
        timestamp: u64,
    ) {
        self.global_syscalls += 1;
        self.global_latency_ns += latency_ns;
        self.global_bytes += bytes;
        let success = return_value >= 0;
        if !success {
            self.global_errors += 1;
        }

        // Per-type
        self.type_metrics
            .entry(syscall_type as u8)
            .or_insert_with(|| SyscallTypeMetrics::new(syscall_type))
            .record(latency_ns, return_value, bytes, timestamp);

        // Per-process
        if self.process_metrics.len() < self.max_processes
            || self.process_metrics.contains_key(&pid)
        {
            self.process_metrics
                .entry(pid)
                .or_insert_with(|| ProcessSyscallMetrics::new(pid))
                .record(syscall_type, latency_ns, success, timestamp);
        }
    }

    /// Get type metrics
    pub fn get_type_metrics(&self, syscall_type: SyscallType) -> Option<&SyscallTypeMetrics> {
        self.type_metrics.get(&(syscall_type as u8))
    }

    /// Get process metrics
    pub fn get_process_metrics(&self, pid: u64) -> Option<&ProcessSyscallMetrics> {
        self.process_metrics.get(&pid)
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.process_metrics.remove(&pid);
    }

    /// Global average latency
    pub fn global_avg_latency_ns(&self) -> u64 {
        if self.global_syscalls == 0 {
            0
        } else {
            self.global_latency_ns / self.global_syscalls
        }
    }

    /// Global error rate
    pub fn global_error_rate(&self) -> f64 {
        if self.global_syscalls == 0 {
            0.0
        } else {
            self.global_errors as f64 / self.global_syscalls as f64
        }
    }

    /// Top N processes by syscall count
    pub fn top_processes(&self, n: usize) -> Vec<(u64, u64)> {
        let mut procs: Vec<(u64, u64)> = self
            .process_metrics
            .iter()
            .map(|(&pid, m)| (pid, m.total_syscalls))
            .collect();
        procs.sort_by(|a, b| b.1.cmp(&a.1));
        procs.truncate(n);
        procs
    }

    /// Top N syscall types by invocation count
    pub fn top_types(&self, n: usize) -> Vec<(u8, u64)> {
        let mut types: Vec<(u8, u64)> = self
            .type_metrics
            .iter()
            .map(|(&t, m)| (t, m.invocations))
            .collect();
        types.sort_by(|a, b| b.1.cmp(&a.1));
        types.truncate(n);
        types
    }

    /// Process count being tracked
    pub fn tracked_processes(&self) -> usize {
        self.process_metrics.len()
    }
}
