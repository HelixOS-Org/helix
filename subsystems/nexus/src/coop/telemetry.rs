//! # Coop Telemetry V2
//!
//! Advanced telemetry for cooperative protocol monitoring:
//! - Distributed tracing with span context propagation
//! - Metric aggregation across cooperation domains
//! - Event correlation engine
//! - Adaptive sampling
//! - Telemetry export pipeline

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// TELEMETRY TYPES
// ============================================================================

/// Telemetry metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricKind {
    /// Counter (monotonically increasing)
    Counter,
    /// Gauge (can go up and down)
    Gauge,
    /// Histogram (distribution)
    Histogram,
    /// Summary (percentiles)
    Summary,
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// Ok
    Ok,
    /// Error
    Error,
    /// Unset
    Unset,
}

/// Sampling decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingDecision {
    /// Always sample
    AlwaysSample,
    /// Never sample
    NeverSample,
    /// Probabilistic
    Probabilistic,
    /// Rate-limited
    RateLimited,
}

// ============================================================================
// TRACE SPAN
// ============================================================================

/// Trace context (W3C-like)
#[derive(Debug, Clone, Copy)]
pub struct TraceContext {
    /// Trace ID (high bits)
    pub trace_id_high: u64,
    /// Trace ID (low bits)
    pub trace_id_low: u64,
    /// Span ID
    pub span_id: u64,
    /// Parent span ID (0 = root)
    pub parent_span_id: u64,
    /// Sampled flag
    pub sampled: bool,
}

/// Trace span
#[derive(Debug, Clone)]
pub struct TraceSpan {
    /// Context
    pub context: TraceContext,
    /// Operation name hash (FNV-1a)
    pub operation_hash: u64,
    /// PID
    pub pid: u64,
    /// Start time (ns)
    pub start_ns: u64,
    /// End time (ns, 0 = in progress)
    pub end_ns: u64,
    /// Status
    pub status: SpanStatus,
    /// Attributes
    pub attributes: BTreeMap<u64, u64>,
    /// Events count
    pub events: u32,
}

impl TraceSpan {
    pub fn new(context: TraceContext, operation_hash: u64, pid: u64, now: u64) -> Self {
        Self {
            context,
            operation_hash,
            pid,
            start_ns: now,
            end_ns: 0,
            status: SpanStatus::Unset,
            attributes: BTreeMap::new(),
            events: 0,
        }
    }

    /// End span
    pub fn end(&mut self, status: SpanStatus, now: u64) {
        self.end_ns = now;
        self.status = status;
    }

    /// Duration (ns)
    pub fn duration_ns(&self) -> u64 {
        if self.end_ns == 0 {
            return 0;
        }
        self.end_ns.saturating_sub(self.start_ns)
    }

    /// Is root span
    pub fn is_root(&self) -> bool {
        self.context.parent_span_id == 0
    }

    /// Set attribute
    pub fn set_attribute(&mut self, key_hash: u64, value: u64) {
        self.attributes.insert(key_hash, value);
    }
}

// ============================================================================
// METRIC POINT
// ============================================================================

/// Metric data point
#[derive(Debug, Clone)]
pub struct MetricPoint {
    /// Metric name hash (FNV-1a)
    pub name_hash: u64,
    /// Kind
    pub kind: MetricKind,
    /// Value
    pub value: f64,
    /// Timestamp (ns)
    pub timestamp_ns: u64,
    /// Labels (key_hash -> value_hash)
    pub labels: BTreeMap<u64, u64>,
}

/// Metric aggregation
#[derive(Debug, Clone)]
pub struct MetricAggregation {
    /// Name hash
    pub name_hash: u64,
    /// Kind
    pub kind: MetricKind,
    /// Count
    pub count: u64,
    /// Sum
    pub sum: f64,
    /// Min
    pub min: f64,
    /// Max
    pub max: f64,
    /// EMA
    pub ema: f64,
    /// Histogram buckets (boundary -> count)
    pub buckets: BTreeMap<u64, u64>,
}

impl MetricAggregation {
    pub fn new(name_hash: u64, kind: MetricKind) -> Self {
        Self {
            name_hash,
            kind,
            count: 0,
            sum: 0.0,
            min: f64::MAX,
            max: f64::MIN,
            ema: 0.0,
            buckets: BTreeMap::new(),
        }
    }

    /// Record value
    pub fn record(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
        if value < self.min {
            self.min = value;
        }
        if value > self.max {
            self.max = value;
        }
        self.ema = 0.95 * self.ema + 0.05 * value;

        // Histogram bucket (round to nearest power of 2)
        if self.kind == MetricKind::Histogram {
            let bucket = if value <= 0.0 { 0 } else { value as u64 };
            let bucket_key = bucket.next_power_of_two();
            *self.buckets.entry(bucket_key).or_insert(0) += 1;
        }
    }

    /// Mean
    pub fn mean(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        self.sum / self.count as f64
    }
}

// ============================================================================
// ADAPTIVE SAMPLER
// ============================================================================

/// Adaptive sampler
#[derive(Debug)]
pub struct AdaptiveSampler {
    /// Base sample rate (0..1)
    pub base_rate: f64,
    /// Current effective rate
    pub effective_rate: f64,
    /// Target samples per second
    pub target_sps: f64,
    /// Actual samples in window
    pub window_samples: u64,
    /// Window start (ns)
    pub window_start_ns: u64,
    /// Window duration (ns)
    pub window_ns: u64,
    /// PRNG state
    rng_state: u64,
}

impl AdaptiveSampler {
    pub fn new(base_rate: f64, target_sps: f64) -> Self {
        Self {
            base_rate,
            effective_rate: base_rate,
            target_sps,
            window_samples: 0,
            window_start_ns: 0,
            window_ns: 1_000_000_000, // 1s
            rng_state: 0x12345678_9abcdef0,
        }
    }

    /// Should sample?
    pub fn should_sample(&mut self, now: u64) -> bool {
        // Check window
        if now.saturating_sub(self.window_start_ns) >= self.window_ns {
            self.adapt(now);
        }

        // xorshift64
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;

        let rand_val = (self.rng_state % 10000) as f64 / 10000.0;
        if rand_val < self.effective_rate {
            self.window_samples += 1;
            true
        } else {
            false
        }
    }

    fn adapt(&mut self, now: u64) {
        let elapsed = now.saturating_sub(self.window_start_ns) as f64 / 1_000_000_000.0;
        if elapsed > 0.0 {
            let actual_sps = self.window_samples as f64 / elapsed;
            if actual_sps > self.target_sps * 1.2 {
                self.effective_rate *= 0.8;
            } else if actual_sps < self.target_sps * 0.8 {
                self.effective_rate = (self.effective_rate * 1.2).min(1.0);
            }
        }
        self.window_samples = 0;
        self.window_start_ns = now;
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Telemetry V2 stats
#[derive(Debug, Clone, Default)]
pub struct CoopTelemetryV2Stats {
    /// Active spans
    pub active_spans: usize,
    /// Completed spans
    pub completed_spans: u64,
    /// Metric series
    pub metric_series: usize,
    /// Total data points
    pub total_points: u64,
    /// Sample rate
    pub effective_sample_rate: f64,
}

/// Coop telemetry V2 engine
pub struct CoopTelemetryV2 {
    /// Active spans
    spans: BTreeMap<u64, TraceSpan>,
    /// Metric aggregations
    metrics: BTreeMap<u64, MetricAggregation>,
    /// Sampler
    pub sampler: AdaptiveSampler,
    /// Stats
    stats: CoopTelemetryV2Stats,
    /// PRNG for span IDs
    span_rng: u64,
}

impl CoopTelemetryV2 {
    pub fn new() -> Self {
        Self {
            spans: BTreeMap::new(),
            metrics: BTreeMap::new(),
            sampler: AdaptiveSampler::new(0.1, 100.0),
            stats: CoopTelemetryV2Stats::default(),
            span_rng: 0xdeadbeef_cafebabe,
        }
    }

    fn next_span_id(&mut self) -> u64 {
        self.span_rng ^= self.span_rng << 13;
        self.span_rng ^= self.span_rng >> 7;
        self.span_rng ^= self.span_rng << 17;
        self.span_rng
    }

    /// Start trace span
    pub fn start_span(&mut self, operation: &str, pid: u64, parent: Option<TraceContext>, now: u64) -> u64 {
        let span_id = self.next_span_id();
        let context = match parent {
            Some(p) => TraceContext {
                trace_id_high: p.trace_id_high,
                trace_id_low: p.trace_id_low,
                span_id,
                parent_span_id: p.span_id,
                sampled: p.sampled,
            },
            None => TraceContext {
                trace_id_high: self.next_span_id(),
                trace_id_low: self.next_span_id(),
                span_id,
                parent_span_id: 0,
                sampled: self.sampler.should_sample(now),
            },
        };

        // FNV-1a for operation name
        let mut op_hash: u64 = 0xcbf29ce484222325;
        for b in operation.as_bytes() {
            op_hash ^= *b as u64;
            op_hash = op_hash.wrapping_mul(0x100000001b3);
        }

        let span = TraceSpan::new(context, op_hash, pid, now);
        self.spans.insert(span_id, span);
        self.update_stats();
        span_id
    }

    /// End span
    pub fn end_span(&mut self, span_id: u64, status: SpanStatus, now: u64) {
        if let Some(span) = self.spans.get_mut(&span_id) {
            span.end(status, now);
            self.stats.completed_spans += 1;
        }
    }

    /// Record metric
    pub fn record_metric(&mut self, name: &str, kind: MetricKind, value: f64, _now: u64) {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let agg = self.metrics.entry(hash)
            .or_insert_with(|| MetricAggregation::new(hash, kind));
        agg.record(value);
        self.stats.total_points += 1;
    }

    /// Cleanup completed spans
    pub fn cleanup_completed(&mut self) {
        self.spans.retain(|_, s| s.end_ns == 0);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.active_spans = self.spans.values().filter(|s| s.end_ns == 0).count();
        self.stats.metric_series = self.metrics.len();
        self.stats.effective_sample_rate = self.sampler.effective_rate;
    }

    /// Stats
    pub fn stats(&self) -> &CoopTelemetryV2Stats {
        &self.stats
    }
}
