//! # Bridge Telemetry
//!
//! Syscall telemetry collection and export:
//! - Metric counters and gauges
//! - Histograms
//! - Span/trace telemetry
//! - Metric labels and dimensions
//! - Aggregation periods
//! - Telemetry export

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// METRIC TYPES
// ============================================================================

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MetricType {
    /// Monotonically increasing counter
    Counter,
    /// Gauge (can go up and down)
    Gauge,
    /// Histogram (distribution)
    Histogram,
    /// Summary (percentiles)
    Summary,
}

/// Metric value
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// Counter or gauge value
    Scalar(f64),
    /// Histogram buckets (upper_bound → count)
    HistogramBuckets(Vec<(f64, u64)>),
    /// Summary quantiles (quantile → value)
    SummaryQuantiles(Vec<(f64, f64)>),
}

// ============================================================================
// COUNTER
// ============================================================================

/// Monotonic counter
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TelemetryCounter {
    /// Name
    pub name: String,
    /// Value
    pub value: u64,
    /// Labels
    pub labels: BTreeMap<String, String>,
}

impl TelemetryCounter {
    pub fn new(name: String) -> Self {
        Self {
            name,
            value: 0,
            labels: BTreeMap::new(),
        }
    }

    #[inline(always)]
    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    #[inline(always)]
    pub fn increment(&mut self) {
        self.value += 1;
    }

    #[inline(always)]
    pub fn add(&mut self, n: u64) {
        self.value += n;
    }
}

// ============================================================================
// GAUGE
// ============================================================================

/// Gauge (up/down metric)
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TelemetryGauge {
    /// Name
    pub name: String,
    /// Current value
    pub value: f64,
    /// Min observed
    pub min: f64,
    /// Max observed
    pub max: f64,
    /// Labels
    pub labels: BTreeMap<String, String>,
}

impl TelemetryGauge {
    pub fn new(name: String) -> Self {
        Self {
            name,
            value: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            labels: BTreeMap::new(),
        }
    }

    #[inline]
    pub fn set(&mut self, value: f64) {
        self.value = value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
    }

    #[inline(always)]
    pub fn increment(&mut self, delta: f64) {
        self.set(self.value + delta);
    }

    #[inline(always)]
    pub fn decrement(&mut self, delta: f64) {
        self.set(self.value - delta);
    }
}

// ============================================================================
// HISTOGRAM
// ============================================================================

/// Telemetry histogram
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TelemetryHistogram {
    /// Name
    pub name: String,
    /// Bucket upper bounds
    pub buckets: Vec<f64>,
    /// Bucket counts
    pub counts: Vec<u64>,
    /// Total sum
    pub sum: f64,
    /// Total count
    pub count: u64,
    /// Labels
    pub labels: BTreeMap<String, String>,
}

impl TelemetryHistogram {
    pub fn new(name: String, buckets: Vec<f64>) -> Self {
        let len = buckets.len();
        Self {
            name,
            buckets,
            counts: alloc::vec![0; len],
            sum: 0.0,
            count: 0,
            labels: BTreeMap::new(),
        }
    }

    /// Default buckets for latency (microseconds)
    #[inline]
    pub fn default_latency_buckets() -> Vec<f64> {
        alloc::vec![
            1.0, 5.0, 10.0, 25.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
        ]
    }

    /// Observe a value
    #[inline]
    pub fn observe(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;

        for (i, &bound) in self.buckets.iter().enumerate() {
            if value <= bound {
                self.counts[i] += 1;
            }
        }
    }

    /// Mean
    #[inline]
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum / self.count as f64
    }

    /// Estimated percentile
    pub fn percentile(&self, p: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let target = (self.count as f64 * p / 100.0) as u64;
        for (i, &count) in self.counts.iter().enumerate() {
            if count >= target {
                return self.buckets[i];
            }
        }
        *self.buckets.last().unwrap_or(&0.0)
    }
}

// ============================================================================
// TELEMETRY SPAN
// ============================================================================

/// Trace span status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// OK
    Ok,
    /// Error
    Error,
    /// Cancelled
    Cancelled,
}

/// Telemetry span
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TelemetrySpan {
    /// Span ID
    pub span_id: u64,
    /// Parent span ID (0 = root)
    pub parent_id: u64,
    /// Trace ID
    pub trace_id: u64,
    /// Operation name
    pub operation: String,
    /// Start time (ns)
    pub start_ns: u64,
    /// End time (ns)
    pub end_ns: u64,
    /// Status
    pub status: SpanStatus,
    /// Attributes
    pub attributes: BTreeMap<String, String>,
}

impl TelemetrySpan {
    pub fn new(span_id: u64, trace_id: u64, operation: String, start_ns: u64) -> Self {
        Self {
            span_id,
            parent_id: 0,
            trace_id,
            operation,
            start_ns,
            end_ns: 0,
            status: SpanStatus::Ok,
            attributes: BTreeMap::new(),
        }
    }

    #[inline(always)]
    pub fn with_parent(mut self, parent_id: u64) -> Self {
        self.parent_id = parent_id;
        self
    }

    #[inline(always)]
    pub fn finish(&mut self, end_ns: u64) {
        self.end_ns = end_ns;
    }

    #[inline(always)]
    pub fn set_error(&mut self) {
        self.status = SpanStatus::Error;
    }

    #[inline(always)]
    pub fn duration_ns(&self) -> u64 {
        self.end_ns.saturating_sub(self.start_ns)
    }

    #[inline(always)]
    pub fn add_attribute(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
    }
}

// ============================================================================
// TELEMETRY MANAGER
// ============================================================================

/// Telemetry stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TelemetryStats {
    /// Counter count
    pub counter_count: usize,
    /// Gauge count
    pub gauge_count: usize,
    /// Histogram count
    pub histogram_count: usize,
    /// Active spans
    pub active_spans: usize,
    /// Total observations
    pub total_observations: u64,
}

/// Bridge telemetry manager
#[repr(align(64))]
pub struct BridgeTelemetryManager {
    /// Counters
    counters: BTreeMap<String, TelemetryCounter>,
    /// Gauges
    gauges: BTreeMap<String, TelemetryGauge>,
    /// Histograms
    histograms: BTreeMap<String, TelemetryHistogram>,
    /// Active spans
    active_spans: BTreeMap<u64, TelemetrySpan>,
    /// Completed spans
    completed_spans: VecDeque<TelemetrySpan>,
    /// Max completed
    max_completed: usize,
    /// Next span ID
    next_span_id: u64,
    /// Stats
    stats: TelemetryStats,
}

impl BridgeTelemetryManager {
    pub fn new() -> Self {
        Self {
            counters: BTreeMap::new(),
            gauges: BTreeMap::new(),
            histograms: BTreeMap::new(),
            active_spans: BTreeMap::new(),
            completed_spans: VecDeque::new(),
            max_completed: 1000,
            next_span_id: 1,
            stats: TelemetryStats::default(),
        }
    }

    /// Get or create counter
    #[inline]
    pub fn counter(&mut self, name: String) -> &mut TelemetryCounter {
        if !self.counters.contains_key(&name) {
            self.counters
                .insert(name.clone(), TelemetryCounter::new(name.clone()));
            self.stats.counter_count = self.counters.len();
        }
        self.counters.get_mut(&name).unwrap()
    }

    /// Get or create gauge
    #[inline]
    pub fn gauge(&mut self, name: String) -> &mut TelemetryGauge {
        if !self.gauges.contains_key(&name) {
            self.gauges
                .insert(name.clone(), TelemetryGauge::new(name.clone()));
            self.stats.gauge_count = self.gauges.len();
        }
        self.gauges.get_mut(&name).unwrap()
    }

    /// Get or create histogram
    #[inline]
    pub fn histogram(&mut self, name: String, buckets: Vec<f64>) -> &mut TelemetryHistogram {
        if !self.histograms.contains_key(&name) {
            self.histograms
                .insert(name.clone(), TelemetryHistogram::new(name.clone(), buckets));
            self.stats.histogram_count = self.histograms.len();
        }
        self.histograms.get_mut(&name).unwrap()
    }

    /// Start span
    #[inline]
    pub fn start_span(&mut self, trace_id: u64, operation: String, start_ns: u64) -> u64 {
        let id = self.next_span_id;
        self.next_span_id += 1;
        let span = TelemetrySpan::new(id, trace_id, operation, start_ns);
        self.active_spans.insert(id, span);
        self.stats.active_spans = self.active_spans.len();
        id
    }

    /// End span
    #[inline]
    pub fn end_span(&mut self, span_id: u64, end_ns: u64) {
        if let Some(mut span) = self.active_spans.remove(&span_id) {
            span.finish(end_ns);
            self.completed_spans.push_back(span);
            if self.completed_spans.len() > self.max_completed {
                self.completed_spans.pop_front();
            }
        }
        self.stats.active_spans = self.active_spans.len();
    }

    /// Record observation (increment total)
    #[inline(always)]
    pub fn observe(&mut self) {
        self.stats.total_observations += 1;
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &TelemetryStats {
        &self.stats
    }
}
