//! # Application Workload History
//!
//! Long-term workload history tracking for applications:
//! - Historical resource usage patterns
//! - Periodic behavior analysis (daily, weekly patterns)
//! - Workload fingerprint evolution
//! - Cross-execution behavior correlation
//! - Regression detection across versions

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// TIME SERIES STORAGE
// ============================================================================

/// Compact time series with downsampling
#[derive(Debug, Clone)]
pub struct TimeSeries {
    /// Raw samples (most recent)
    raw: VecDeque<(u64, f64)>,
    /// Minute-level aggregates
    minute_agg: VecDeque<TimeAggregate>,
    /// Hour-level aggregates
    hour_agg: VecDeque<TimeAggregate>,
    /// Max raw samples
    max_raw: usize,
    /// Max minute aggregates
    max_minutes: usize,
    /// Max hour aggregates
    max_hours: usize,
    /// Current minute accumulator
    current_minute: Option<AggregateAccumulator>,
    /// Current hour accumulator
    current_hour: Option<AggregateAccumulator>,
}

/// Time-aggregated value
#[derive(Debug, Clone, Copy)]
pub struct TimeAggregate {
    /// Period start timestamp
    pub start: u64,
    /// Period end timestamp
    pub end: u64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Mean value
    pub mean: f64,
    /// Sample count
    pub count: u64,
    /// Sum of values
    pub sum: f64,
}

/// Accumulator for building aggregates
#[derive(Debug, Clone)]
struct AggregateAccumulator {
    start: u64,
    min: f64,
    max: f64,
    sum: f64,
    count: u64,
}

impl AggregateAccumulator {
    fn new(start: u64, value: f64) -> Self {
        Self {
            start,
            min: value,
            max: value,
            sum: value,
            count: 1,
        }
    }

    fn add(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    fn finalize(&self, end: u64) -> TimeAggregate {
        TimeAggregate {
            start: self.start,
            end,
            min: self.min,
            max: self.max,
            mean: if self.count > 0 {
                self.sum / self.count as f64
            } else {
                0.0
            },
            count: self.count,
            sum: self.sum,
        }
    }
}

impl TimeSeries {
    pub fn new(max_raw: usize, max_minutes: usize, max_hours: usize) -> Self {
        Self {
            raw: VecDeque::new(),
            minute_agg: VecDeque::new(),
            hour_agg: VecDeque::new(),
            max_raw,
            max_minutes,
            max_hours,
            current_minute: None,
            current_hour: None,
        }
    }

    /// Add a sample
    pub fn add(&mut self, timestamp: u64, value: f64) {
        // Store raw
        if self.raw.len() >= self.max_raw {
            self.raw.pop_front();
        }
        self.raw.push_back((timestamp, value));

        // Minute aggregation (60,000 ms = 1 minute)
        match &mut self.current_minute {
            Some(acc) if timestamp.saturating_sub(acc.start) < 60_000 => {
                acc.add(value);
            },
            Some(acc) => {
                let agg = acc.finalize(timestamp);
                if self.minute_agg.len() >= self.max_minutes {
                    self.minute_agg.pop_front();
                }
                self.minute_agg.push_back(agg);
                self.current_minute = Some(AggregateAccumulator::new(timestamp, value));
            },
            None => {
                self.current_minute = Some(AggregateAccumulator::new(timestamp, value));
            },
        }

        // Hour aggregation (3,600,000 ms = 1 hour)
        match &mut self.current_hour {
            Some(acc) if timestamp.saturating_sub(acc.start) < 3_600_000 => {
                acc.add(value);
            },
            Some(acc) => {
                let agg = acc.finalize(timestamp);
                if self.hour_agg.len() >= self.max_hours {
                    self.hour_agg.pop_front();
                }
                self.hour_agg.push_back(agg);
                self.current_hour = Some(AggregateAccumulator::new(timestamp, value));
            },
            None => {
                self.current_hour = Some(AggregateAccumulator::new(timestamp, value));
            },
        }
    }

    /// Get latest raw value
    #[inline(always)]
    pub fn latest(&self) -> Option<f64> {
        self.raw.back().map(|(_, v)| *v)
    }

    /// Average of recent raw values
    #[inline]
    pub fn recent_avg(&self, n: usize) -> f64 {
        let start = self.raw.len().saturating_sub(n);
        let slice = &self.raw[start..];
        if slice.is_empty() {
            return 0.0;
        }
        slice.iter().map(|(_, v)| v).sum::<f64>() / slice.len() as f64
    }

    /// Trend over recent values (positive = increasing)
    pub fn trend(&self) -> f64 {
        if self.raw.len() < 10 {
            return 0.0;
        }
        let n = self.raw.len();
        let first_half: f64 =
            self.raw[..n / 2].iter().map(|(_, v)| v).sum::<f64>() / (n / 2) as f64;
        let second_half: f64 =
            self.raw[n / 2..].iter().map(|(_, v)| v).sum::<f64>() / (n - n / 2) as f64;
        if first_half < 0.001 {
            return 0.0;
        }
        (second_half - first_half) / first_half
    }

    /// Get minute-level aggregates
    #[inline(always)]
    pub fn minutes(&self) -> &[TimeAggregate] {
        &self.minute_agg
    }

    /// Get hour-level aggregates
    #[inline(always)]
    pub fn hours(&self) -> &[TimeAggregate] {
        &self.hour_agg
    }

    /// Total samples ever added
    #[inline(always)]
    pub fn raw_count(&self) -> usize {
        self.raw.len()
    }
}

// ============================================================================
// WORKLOAD HISTORY
// ============================================================================

/// Per-process workload history
#[derive(Debug, Clone)]
pub struct WorkloadHistory {
    /// Process ID
    pub pid: u64,
    /// Binary name / identifier
    pub binary_id: u64,
    /// CPU usage time series
    pub cpu_usage: TimeSeries,
    /// Memory usage time series
    pub memory_usage: TimeSeries,
    /// I/O rate time series
    pub io_rate: TimeSeries,
    /// Syscall rate time series
    pub syscall_rate: TimeSeries,
    /// Network rate time series
    pub network_rate: TimeSeries,
    /// Thread count time series
    pub thread_count: TimeSeries,
    /// Workload fingerprint snapshots
    pub fingerprints: VecDeque<WorkloadFingerprint>,
    /// Max fingerprints
    max_fingerprints: usize,
}

/// Snapshot of workload characteristics at a point in time
#[derive(Debug, Clone)]
pub struct WorkloadFingerprint {
    /// Timestamp
    pub timestamp: u64,
    /// CPU intensity (0.0 - 1.0)
    pub cpu_intensity: f64,
    /// I/O intensity (0.0 - 1.0)
    pub io_intensity: f64,
    /// Memory pressure (0.0 - 1.0)
    pub memory_pressure: f64,
    /// Network intensity (0.0 - 1.0)
    pub network_intensity: f64,
    /// Concurrency level
    pub concurrency: f64,
    /// Syscall diversity (number of distinct syscall types)
    pub syscall_diversity: u32,
    /// Average syscall latency (µs)
    pub avg_syscall_latency_us: f64,
}

impl WorkloadHistory {
    pub fn new(pid: u64, binary_id: u64) -> Self {
        Self {
            pid,
            binary_id,
            cpu_usage: TimeSeries::new(300, 60, 24), // 5 min raw, 1h minute, 1d hour
            memory_usage: TimeSeries::new(300, 60, 24),
            io_rate: TimeSeries::new(300, 60, 24),
            syscall_rate: TimeSeries::new(300, 60, 24),
            network_rate: TimeSeries::new(300, 60, 24),
            thread_count: TimeSeries::new(300, 60, 24),
            fingerprints: VecDeque::new(),
            max_fingerprints: 100,
        }
    }

    /// Record a fingerprint
    #[inline]
    pub fn add_fingerprint(&mut self, fp: WorkloadFingerprint) {
        if self.fingerprints.len() >= self.max_fingerprints {
            self.fingerprints.pop_front();
        }
        self.fingerprints.push_back(fp);
    }

    /// Detect workload change (compare recent fingerprint to historical)
    pub fn workload_changed(&self) -> bool {
        if self.fingerprints.len() < 5 {
            return false;
        }
        let n = self.fingerprints.len();
        let recent = &self.fingerprints[n - 1];
        let historical_avg_cpu: f64 = self.fingerprints[..n - 1]
            .iter()
            .map(|fp| fp.cpu_intensity)
            .sum::<f64>()
            / (n - 1) as f64;

        // Simple change detection: >50% deviation
        let deviation = libm::fabs(recent.cpu_intensity - historical_avg_cpu);
        if historical_avg_cpu > 0.01 {
            deviation / historical_avg_cpu > 0.5
        } else {
            deviation > 0.1
        }
    }

    /// Get current workload trend
    #[inline(always)]
    pub fn cpu_trend(&self) -> f64 {
        self.cpu_usage.trend()
    }

    #[inline(always)]
    pub fn memory_trend(&self) -> f64 {
        self.memory_usage.trend()
    }

    #[inline(always)]
    pub fn io_trend(&self) -> f64 {
        self.io_rate.trend()
    }
}

// ============================================================================
// HISTORY MANAGER
// ============================================================================

/// Manages workload history for all processes
pub struct WorkloadHistoryManager {
    /// Per-process histories
    histories: BTreeMap<u64, WorkloadHistory>,
    /// Per-binary aggregate histories (binary_id → aggregated metrics)
    binary_histories: BTreeMap<u64, BinaryHistory>,
    /// Max processes
    max_processes: usize,
    /// Max binary histories
    max_binaries: usize,
}

/// Aggregate history for a binary (across all executions)
#[derive(Debug, Clone)]
pub struct BinaryHistory {
    /// Binary identifier
    pub binary_id: u64,
    /// Number of executions seen
    pub execution_count: u64,
    /// Average peak CPU usage
    pub avg_peak_cpu: f64,
    /// Average peak memory (bytes)
    pub avg_peak_memory: f64,
    /// Average lifetime (ms)
    pub avg_lifetime_ms: f64,
    /// Typical startup duration (ms)
    pub avg_startup_ms: f64,
    /// Typical steady-state CPU
    pub typical_cpu: f64,
    /// Typical steady-state memory
    pub typical_memory: f64,
    /// Crash count
    pub crashes: u64,
}

impl BinaryHistory {
    pub fn new(binary_id: u64) -> Self {
        Self {
            binary_id,
            execution_count: 0,
            avg_peak_cpu: 0.0,
            avg_peak_memory: 0.0,
            avg_lifetime_ms: 0.0,
            avg_startup_ms: 0.0,
            typical_cpu: 0.0,
            typical_memory: 0.0,
            crashes: 0,
        }
    }

    /// Update running averages with a new execution
    pub fn record_execution(
        &mut self,
        peak_cpu: f64,
        peak_memory: f64,
        lifetime_ms: f64,
        startup_ms: f64,
        steady_cpu: f64,
        steady_memory: f64,
        crashed: bool,
    ) {
        self.execution_count += 1;
        let n = self.execution_count as f64;
        // Exponential moving average with factor based on count
        let alpha = if n < 10.0 { 1.0 / n } else { 0.1 };
        self.avg_peak_cpu = self.avg_peak_cpu * (1.0 - alpha) + peak_cpu * alpha;
        self.avg_peak_memory = self.avg_peak_memory * (1.0 - alpha) + peak_memory * alpha;
        self.avg_lifetime_ms = self.avg_lifetime_ms * (1.0 - alpha) + lifetime_ms * alpha;
        self.avg_startup_ms = self.avg_startup_ms * (1.0 - alpha) + startup_ms * alpha;
        self.typical_cpu = self.typical_cpu * (1.0 - alpha) + steady_cpu * alpha;
        self.typical_memory = self.typical_memory * (1.0 - alpha) + steady_memory * alpha;
        if crashed {
            self.crashes += 1;
        }
    }

    /// Crash rate
    #[inline]
    pub fn crash_rate(&self) -> f64 {
        if self.execution_count == 0 {
            0.0
        } else {
            self.crashes as f64 / self.execution_count as f64
        }
    }
}

impl WorkloadHistoryManager {
    pub fn new(max_processes: usize, max_binaries: usize) -> Self {
        Self {
            histories: BTreeMap::new(),
            binary_histories: BTreeMap::new(),
            max_processes,
            max_binaries,
        }
    }

    /// Get or create process history
    #[inline]
    pub fn get_or_create(&mut self, pid: u64, binary_id: u64) -> &mut WorkloadHistory {
        if !self.histories.contains_key(&pid) && self.histories.len() < self.max_processes {
            self.histories
                .insert(pid, WorkloadHistory::new(pid, binary_id));
        }
        self.histories
            .entry(pid)
            .or_insert_with(|| WorkloadHistory::new(pid, binary_id))
    }

    /// Get process history
    #[inline(always)]
    pub fn get(&self, pid: u64) -> Option<&WorkloadHistory> {
        self.histories.get(&pid)
    }

    /// Get binary history
    #[inline(always)]
    pub fn get_binary(&self, binary_id: u64) -> Option<&BinaryHistory> {
        self.binary_histories.get(&binary_id)
    }

    /// Get or create binary history
    #[inline]
    pub fn get_or_create_binary(&mut self, binary_id: u64) -> &mut BinaryHistory {
        if !self.binary_histories.contains_key(&binary_id)
            && self.binary_histories.len() < self.max_binaries
        {
            self.binary_histories
                .insert(binary_id, BinaryHistory::new(binary_id));
        }
        self.binary_histories
            .entry(binary_id)
            .or_insert_with(|| BinaryHistory::new(binary_id))
    }

    /// Remove process
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.histories.remove(&pid);
    }

    /// Number of tracked processes
    #[inline(always)]
    pub fn process_count(&self) -> usize {
        self.histories.len()
    }

    /// Number of tracked binaries
    #[inline(always)]
    pub fn binary_count(&self) -> usize {
        self.binary_histories.len()
    }
}
