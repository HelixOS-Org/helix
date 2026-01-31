//! # CORTEX Telemetry and Metrics
//!
//! This module provides comprehensive telemetry and metrics collection
//! for the CORTEX intelligence framework. It enables real-time observation
//! of kernel behavior, performance analysis, and anomaly detection.
//!
//! ## Design Philosophy
//!
//! - **Zero-overhead when disabled**: Metrics collection can be compiled out
//! - **Lock-free collection**: Uses atomic operations for performance
//! - **Bounded memory**: Fixed-size ring buffers, no unbounded growth
//! - **Sampling support**: Statistical sampling for high-frequency events
//! - **Hierarchical aggregation**: Per-core → per-CPU → system-wide

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::Timestamp;

// =============================================================================
// METRIC TYPES
// =============================================================================

/// Metric identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MetricId(pub u64);

/// Metric kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    /// Counter - monotonically increasing
    Counter,

    /// Gauge - can go up or down
    Gauge,

    /// Histogram - distribution of values
    Histogram,

    /// Timer - measures duration
    Timer,

    /// Rate - events per time unit
    Rate,
}

/// Metric category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricCategory {
    /// Consciousness metrics
    Consciousness,

    /// Neural engine metrics
    Neural,

    /// Temporal kernel metrics
    Temporal,

    /// Survivability metrics
    Survivability,

    /// Meta-kernel metrics
    Meta,

    /// System metrics
    System,

    /// Memory metrics
    Memory,

    /// Scheduler metrics
    Scheduler,

    /// I/O metrics
    Io,

    /// Custom/extension metrics
    Custom,
}

/// Metric definition
#[derive(Clone)]
pub struct MetricDef {
    /// Metric ID
    pub id: MetricId,

    /// Metric name
    pub name: String,

    /// Description
    pub description: String,

    /// Kind
    pub kind: MetricKind,

    /// Category
    pub category: MetricCategory,

    /// Unit (e.g., "bytes", "microseconds", "operations")
    pub unit: String,

    /// Is metric enabled?
    pub enabled: bool,
}

impl MetricDef {
    /// Create new metric definition
    pub fn new(id: MetricId, name: &str, kind: MetricKind, category: MetricCategory) -> Self {
        Self {
            id,
            name: String::from(name),
            description: String::new(),
            kind,
            category,
            unit: String::new(),
            enabled: true,
        }
    }

    /// Add description
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = String::from(desc);
        self
    }

    /// Set unit
    pub fn with_unit(mut self, unit: &str) -> Self {
        self.unit = String::from(unit);
        self
    }
}

// =============================================================================
// COUNTER
// =============================================================================

/// Atomic counter metric
pub struct Counter {
    /// Current value
    value: AtomicU64,

    /// Metric definition
    def: MetricDef,
}

impl Counter {
    /// Create new counter
    pub fn new(def: MetricDef) -> Self {
        Self {
            value: AtomicU64::new(0),
            def,
        }
    }

    /// Increment by 1
    pub fn inc(&self) {
        self.add(1);
    }

    /// Add value
    pub fn add(&self, n: u64) {
        self.value.fetch_add(n, Ordering::Relaxed);
    }

    /// Get current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset counter
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }

    /// Get definition
    pub fn def(&self) -> &MetricDef {
        &self.def
    }
}

// =============================================================================
// GAUGE
// =============================================================================

/// Atomic gauge metric (can go up or down)
pub struct Gauge {
    /// Current value (stored as bits of f64)
    value: AtomicU64,

    /// Metric definition
    def: MetricDef,
}

impl Gauge {
    /// Create new gauge
    pub fn new(def: MetricDef) -> Self {
        Self {
            value: AtomicU64::new(0),
            def,
        }
    }

    /// Set value
    pub fn set(&self, v: f64) {
        self.value.store(v.to_bits(), Ordering::Relaxed);
    }

    /// Set integer value
    pub fn set_i64(&self, v: i64) {
        self.set(v as f64);
    }

    /// Get current value
    pub fn get(&self) -> f64 {
        f64::from_bits(self.value.load(Ordering::Relaxed))
    }

    /// Increment by value
    pub fn add(&self, delta: f64) {
        loop {
            let current = self.value.load(Ordering::Relaxed);
            let current_f64 = f64::from_bits(current);
            let new = (current_f64 + delta).to_bits();

            if self
                .value
                .compare_exchange_weak(current, new, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }
    }

    /// Decrement by value
    pub fn sub(&self, delta: f64) {
        self.add(-delta);
    }

    /// Get definition
    pub fn def(&self) -> &MetricDef {
        &self.def
    }
}

// =============================================================================
// HISTOGRAM
// =============================================================================

/// Histogram bucket
#[derive(Debug, Clone)]
pub struct Bucket {
    /// Upper bound (exclusive)
    pub le: f64,

    /// Count of observations
    pub count: AtomicU64,
}

impl Bucket {
    /// Create new bucket
    pub fn new(le: f64) -> Self {
        Self {
            le,
            count: AtomicU64::new(0),
        }
    }
}

/// Histogram for measuring distributions
pub struct Histogram {
    /// Buckets
    buckets: Vec<Bucket>,

    /// Total sum
    sum: AtomicU64,

    /// Total count
    count: AtomicU64,

    /// Metric definition
    def: MetricDef,
}

impl Histogram {
    /// Create histogram with default buckets
    pub fn new(def: MetricDef) -> Self {
        Self::with_buckets(def, &[
            0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
        ])
    }

    /// Create histogram with custom buckets
    pub fn with_buckets(def: MetricDef, bounds: &[f64]) -> Self {
        let mut buckets = Vec::with_capacity(bounds.len() + 1);

        for &bound in bounds {
            buckets.push(Bucket::new(bound));
        }

        // +Inf bucket
        buckets.push(Bucket::new(f64::INFINITY));

        Self {
            buckets,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
            def,
        }
    }

    /// Observe a value
    pub fn observe(&self, v: f64) {
        // Update count
        self.count.fetch_add(1, Ordering::Relaxed);

        // Update sum
        loop {
            let current = self.sum.load(Ordering::Relaxed);
            let current_f64 = f64::from_bits(current);
            let new = (current_f64 + v).to_bits();

            if self
                .sum
                .compare_exchange_weak(current, new, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                break;
            }
        }

        // Update buckets
        for bucket in &self.buckets {
            if v <= bucket.le {
                bucket.count.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Get count
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Get sum
    pub fn sum(&self) -> f64 {
        f64::from_bits(self.sum.load(Ordering::Relaxed))
    }

    /// Get mean
    pub fn mean(&self) -> f64 {
        let count = self.count();
        if count == 0 {
            0.0
        } else {
            self.sum() / count as f64
        }
    }

    /// Get bucket counts
    pub fn bucket_counts(&self) -> Vec<(f64, u64)> {
        self.buckets
            .iter()
            .map(|b| (b.le, b.count.load(Ordering::Relaxed)))
            .collect()
    }

    /// Estimate percentile
    pub fn percentile(&self, p: f64) -> f64 {
        let total = self.count();
        if total == 0 {
            return 0.0;
        }

        let target = (p * total as f64) as u64;

        let mut prev_bound = 0.0;
        let mut prev_count = 0;

        for bucket in &self.buckets {
            let count = bucket.count.load(Ordering::Relaxed);
            if count >= target {
                // Linear interpolation
                if count == prev_count {
                    return bucket.le;
                }

                let fraction = (target - prev_count) as f64 / (count - prev_count) as f64;
                return prev_bound + fraction * (bucket.le - prev_bound);
            }
            prev_bound = bucket.le;
            prev_count = count;
        }

        f64::INFINITY
    }

    /// Get definition
    pub fn def(&self) -> &MetricDef {
        &self.def
    }
}

// =============================================================================
// TIMER
// =============================================================================

/// Timer for measuring durations
pub struct Timer {
    /// Underlying histogram
    histogram: Histogram,

    /// Start time for current measurement
    start: AtomicU64,
}

impl Timer {
    /// Create new timer
    pub fn new(def: MetricDef) -> Self {
        // Default buckets for timing (in seconds)
        Self {
            histogram: Histogram::with_buckets(def, &[
                0.000_001, 0.000_01, 0.000_1, 0.001, 0.01, 0.1, 1.0, 10.0,
            ]),
            start: AtomicU64::new(0),
        }
    }

    /// Start timing
    pub fn start(&self) -> TimerGuard<'_> {
        TimerGuard {
            timer: self,
            start: crate::current_timestamp(),
        }
    }

    /// Record duration directly (in seconds)
    pub fn record(&self, duration_secs: f64) {
        self.histogram.observe(duration_secs);
    }

    /// Record duration in nanoseconds
    pub fn record_ns(&self, duration_ns: u64) {
        self.histogram.observe(duration_ns as f64 / 1_000_000_000.0);
    }

    /// Get count
    pub fn count(&self) -> u64 {
        self.histogram.count()
    }

    /// Get mean duration
    pub fn mean(&self) -> f64 {
        self.histogram.mean()
    }

    /// Get percentile
    pub fn percentile(&self, p: f64) -> f64 {
        self.histogram.percentile(p)
    }

    /// Get definition
    pub fn def(&self) -> &MetricDef {
        self.histogram.def()
    }
}

/// RAII guard for timing
pub struct TimerGuard<'a> {
    timer: &'a Timer,
    start: Timestamp,
}

impl<'a> Drop for TimerGuard<'a> {
    fn drop(&mut self) {
        let duration = crate::current_timestamp().saturating_sub(self.start);
        // Assuming 1GHz clock, convert to seconds
        self.timer.record(duration as f64 / 1_000_000_000.0);
    }
}

// =============================================================================
// RATE METER
// =============================================================================

/// Sliding window rate meter
pub struct RateMeter {
    /// Window slots (counts per slot)
    slots: Vec<AtomicU64>,

    /// Current slot index
    current_slot: AtomicU64,

    /// Slot duration (cycles)
    slot_duration: u64,

    /// Last slot update timestamp
    last_update: AtomicU64,

    /// Metric definition
    def: MetricDef,
}

impl RateMeter {
    /// Create new rate meter with window size
    pub fn new(def: MetricDef, num_slots: usize, slot_duration_cycles: u64) -> Self {
        let slots = (0..num_slots).map(|_| AtomicU64::new(0)).collect();

        Self {
            slots,
            current_slot: AtomicU64::new(0),
            slot_duration: slot_duration_cycles,
            last_update: AtomicU64::new(0),
            def,
        }
    }

    /// Mark an event
    pub fn mark(&self, timestamp: Timestamp) {
        self.maybe_rotate(timestamp);

        let slot = self.current_slot.load(Ordering::Relaxed) as usize % self.slots.len();
        self.slots[slot].fetch_add(1, Ordering::Relaxed);
    }

    /// Mark multiple events
    pub fn mark_n(&self, n: u64, timestamp: Timestamp) {
        self.maybe_rotate(timestamp);

        let slot = self.current_slot.load(Ordering::Relaxed) as usize % self.slots.len();
        self.slots[slot].fetch_add(n, Ordering::Relaxed);
    }

    /// Rotate slots if needed
    fn maybe_rotate(&self, timestamp: Timestamp) {
        let last = self.last_update.load(Ordering::Relaxed);

        if timestamp - last >= self.slot_duration {
            // Move to next slot
            let current = self.current_slot.fetch_add(1, Ordering::Relaxed);
            let next_slot = (current + 1) as usize % self.slots.len();

            // Clear the next slot
            self.slots[next_slot].store(0, Ordering::Relaxed);

            self.last_update.store(timestamp, Ordering::Relaxed);
        }
    }

    /// Get current rate (events per second, assuming 1GHz clock)
    pub fn rate(&self) -> f64 {
        let total: u64 = self.slots.iter().map(|s| s.load(Ordering::Relaxed)).sum();

        let window_duration_secs =
            (self.slots.len() as u64 * self.slot_duration) as f64 / 1_000_000_000.0;

        total as f64 / window_duration_secs
    }

    /// Get total count in window
    pub fn total(&self) -> u64 {
        self.slots.iter().map(|s| s.load(Ordering::Relaxed)).sum()
    }

    /// Get definition
    pub fn def(&self) -> &MetricDef {
        &self.def
    }
}

// =============================================================================
// TIME SERIES
// =============================================================================

/// Time series sample
#[derive(Debug, Clone, Copy)]
pub struct Sample {
    /// Timestamp
    pub timestamp: Timestamp,

    /// Value
    pub value: f64,
}

/// Time series buffer (ring buffer)
pub struct TimeSeries {
    /// Samples
    samples: Vec<Sample>,

    /// Write position
    write_pos: usize,

    /// Number of valid samples
    count: usize,

    /// Metric definition
    def: MetricDef,
}

impl TimeSeries {
    /// Create new time series
    pub fn new(def: MetricDef, capacity: usize) -> Self {
        Self {
            samples: vec![
                Sample {
                    timestamp: 0,
                    value: 0.0
                };
                capacity
            ],
            write_pos: 0,
            count: 0,
            def,
        }
    }

    /// Add sample
    pub fn add(&mut self, timestamp: Timestamp, value: f64) {
        self.samples[self.write_pos] = Sample { timestamp, value };
        self.write_pos = (self.write_pos + 1) % self.samples.len();
        if self.count < self.samples.len() {
            self.count += 1;
        }
    }

    /// Get all samples (oldest first)
    pub fn samples(&self) -> Vec<Sample> {
        let mut result = Vec::with_capacity(self.count);

        let start = if self.count == self.samples.len() {
            self.write_pos
        } else {
            0
        };

        for i in 0..self.count {
            let idx = (start + i) % self.samples.len();
            result.push(self.samples[idx]);
        }

        result
    }

    /// Get latest sample
    pub fn latest(&self) -> Option<Sample> {
        if self.count == 0 {
            None
        } else {
            let idx = if self.write_pos == 0 {
                self.samples.len() - 1
            } else {
                self.write_pos - 1
            };
            Some(self.samples[idx])
        }
    }

    /// Get sample count
    pub fn count(&self) -> usize {
        self.count
    }

    /// Get definition
    pub fn def(&self) -> &MetricDef {
        &self.def
    }

    /// Calculate simple moving average
    pub fn sma(&self, window: usize) -> f64 {
        let samples = self.samples();
        let n = samples.len().min(window);

        if n == 0 {
            return 0.0;
        }

        let sum: f64 = samples.iter().rev().take(n).map(|s| s.value).sum();

        sum / n as f64
    }

    /// Calculate exponential moving average
    pub fn ema(&self, alpha: f64) -> f64 {
        let samples = self.samples();

        if samples.is_empty() {
            return 0.0;
        }

        let mut ema = samples[0].value;
        for sample in samples.iter().skip(1) {
            ema = alpha * sample.value + (1.0 - alpha) * ema;
        }

        ema
    }

    /// Calculate trend (slope of linear regression)
    pub fn trend(&self) -> f64 {
        let samples = self.samples();
        let n = samples.len();

        if n < 2 {
            return 0.0;
        }

        let n_f64 = n as f64;

        // Calculate means
        let x_mean = (n_f64 - 1.0) / 2.0;
        let y_mean: f64 = samples.iter().map(|s| s.value).sum::<f64>() / n_f64;

        // Calculate slope
        let mut num = 0.0;
        let mut den = 0.0;

        for (i, sample) in samples.iter().enumerate() {
            let x_diff = i as f64 - x_mean;
            let y_diff = sample.value - y_mean;
            num += x_diff * y_diff;
            den += x_diff * x_diff;
        }

        if den.abs() < 1e-10 {
            0.0
        } else {
            num / den
        }
    }
}

// =============================================================================
// TELEMETRY COLLECTOR
// =============================================================================

/// Telemetry snapshot
#[derive(Clone)]
pub struct TelemetrySnapshot {
    /// Timestamp
    pub timestamp: Timestamp,

    /// Counter values
    pub counters: Vec<(String, u64)>,

    /// Gauge values
    pub gauges: Vec<(String, f64)>,

    /// Histogram summaries
    pub histograms: Vec<(String, f64, f64, f64)>, // name, mean, p50, p99

    /// Rate values
    pub rates: Vec<(String, f64)>,
}

/// Telemetry collector
pub struct TelemetryCollector {
    /// Counters
    counters: Vec<Counter>,

    /// Gauges
    gauges: Vec<Gauge>,

    /// Histograms
    histograms: Vec<Histogram>,

    /// Timers
    timers: Vec<Timer>,

    /// Rate meters
    rates: Vec<RateMeter>,

    /// Time series
    series: Vec<TimeSeries>,

    /// Enabled?
    enabled: bool,

    /// Sampling rate (1.0 = all, 0.1 = 10%)
    sampling_rate: f64,

    /// Sample counter
    sample_counter: AtomicU64,
}

impl TelemetryCollector {
    /// Create new collector
    pub fn new() -> Self {
        Self {
            counters: Vec::new(),
            gauges: Vec::new(),
            histograms: Vec::new(),
            timers: Vec::new(),
            rates: Vec::new(),
            series: Vec::new(),
            enabled: true,
            sampling_rate: 1.0,
            sample_counter: AtomicU64::new(0),
        }
    }

    /// Register counter
    pub fn register_counter(&mut self, def: MetricDef) -> usize {
        let idx = self.counters.len();
        self.counters.push(Counter::new(def));
        idx
    }

    /// Register gauge
    pub fn register_gauge(&mut self, def: MetricDef) -> usize {
        let idx = self.gauges.len();
        self.gauges.push(Gauge::new(def));
        idx
    }

    /// Register histogram
    pub fn register_histogram(&mut self, def: MetricDef) -> usize {
        let idx = self.histograms.len();
        self.histograms.push(Histogram::new(def));
        idx
    }

    /// Register timer
    pub fn register_timer(&mut self, def: MetricDef) -> usize {
        let idx = self.timers.len();
        self.timers.push(Timer::new(def));
        idx
    }

    /// Register rate meter
    pub fn register_rate(&mut self, def: MetricDef, slots: usize, slot_duration: u64) -> usize {
        let idx = self.rates.len();
        self.rates.push(RateMeter::new(def, slots, slot_duration));
        idx
    }

    /// Register time series
    pub fn register_series(&mut self, def: MetricDef, capacity: usize) -> usize {
        let idx = self.series.len();
        self.series.push(TimeSeries::new(def, capacity));
        idx
    }

    /// Get counter
    pub fn counter(&self, idx: usize) -> Option<&Counter> {
        self.counters.get(idx)
    }

    /// Get gauge
    pub fn gauge(&self, idx: usize) -> Option<&Gauge> {
        self.gauges.get(idx)
    }

    /// Get histogram
    pub fn histogram(&self, idx: usize) -> Option<&Histogram> {
        self.histograms.get(idx)
    }

    /// Get timer
    pub fn timer(&self, idx: usize) -> Option<&Timer> {
        self.timers.get(idx)
    }

    /// Get rate meter
    pub fn rate(&self, idx: usize) -> Option<&RateMeter> {
        self.rates.get(idx)
    }

    /// Get time series (mutable for adding samples)
    pub fn series_mut(&mut self, idx: usize) -> Option<&mut TimeSeries> {
        self.series.get_mut(idx)
    }

    /// Should sample this event?
    pub fn should_sample(&self) -> bool {
        if !self.enabled {
            return false;
        }

        if self.sampling_rate >= 1.0 {
            return true;
        }

        let counter = self.sample_counter.fetch_add(1, Ordering::Relaxed);
        let threshold = (self.sampling_rate * 1000.0) as u64;
        (counter % 1000) < threshold
    }

    /// Set sampling rate
    pub fn set_sampling_rate(&mut self, rate: f64) {
        self.sampling_rate = rate.clamp(0.0, 1.0);
    }

    /// Enable/disable collection
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Take snapshot
    pub fn snapshot(&self, timestamp: Timestamp) -> TelemetrySnapshot {
        TelemetrySnapshot {
            timestamp,
            counters: self
                .counters
                .iter()
                .map(|c| (c.def().name.clone(), c.get()))
                .collect(),
            gauges: self
                .gauges
                .iter()
                .map(|g| (g.def().name.clone(), g.get()))
                .collect(),
            histograms: self
                .histograms
                .iter()
                .map(|h| {
                    (
                        h.def().name.clone(),
                        h.mean(),
                        h.percentile(0.50),
                        h.percentile(0.99),
                    )
                })
                .collect(),
            rates: self
                .rates
                .iter()
                .map(|r| (r.def().name.clone(), r.rate()))
                .collect(),
        }
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let counter = Counter::new(MetricDef::new(
            MetricId(1),
            "test",
            MetricKind::Counter,
            MetricCategory::System,
        ));

        assert_eq!(counter.get(), 0);
        counter.inc();
        assert_eq!(counter.get(), 1);
        counter.add(5);
        assert_eq!(counter.get(), 6);
    }

    #[test]
    fn test_gauge() {
        let gauge = Gauge::new(MetricDef::new(
            MetricId(1),
            "test",
            MetricKind::Gauge,
            MetricCategory::System,
        ));

        gauge.set(42.0);
        assert_eq!(gauge.get(), 42.0);

        gauge.add(8.0);
        assert_eq!(gauge.get(), 50.0);
    }

    #[test]
    fn test_histogram() {
        let hist = Histogram::with_buckets(
            MetricDef::new(
                MetricId(1),
                "test",
                MetricKind::Histogram,
                MetricCategory::System,
            ),
            &[1.0, 5.0, 10.0],
        );

        hist.observe(0.5);
        hist.observe(2.0);
        hist.observe(7.0);

        assert_eq!(hist.count(), 3);
    }

    #[test]
    fn test_time_series() {
        let mut ts = TimeSeries::new(
            MetricDef::new(
                MetricId(1),
                "test",
                MetricKind::Gauge,
                MetricCategory::System,
            ),
            10,
        );

        ts.add(100, 1.0);
        ts.add(200, 2.0);
        ts.add(300, 3.0);

        assert_eq!(ts.count(), 3);
        assert_eq!(ts.latest().map(|s| s.value), Some(3.0));
        assert_eq!(ts.sma(3), 2.0);
    }
}
