//! # Distributed Monitoring
//!
//! Year 3 EVOLUTION - Monitoring and observability for distributed systems

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============================================================================
// MONITORING TYPES
// ============================================================================

/// Metric ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MetricId(pub u64);

static METRIC_COUNTER: AtomicU64 = AtomicU64::new(1);

impl MetricId {
    pub fn generate() -> Self {
        Self(METRIC_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Node ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

/// Span ID (for tracing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpanId(pub u64);

static SPAN_COUNTER: AtomicU64 = AtomicU64::new(1);

impl SpanId {
    pub fn generate() -> Self {
        Self(SPAN_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Trace ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceId(pub u64);

static TRACE_COUNTER: AtomicU64 = AtomicU64::new(1);

impl TraceId {
    pub fn generate() -> Self {
        Self(TRACE_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

// ============================================================================
// METRICS
// ============================================================================

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Metric definition
#[derive(Debug, Clone)]
pub struct MetricDef {
    /// ID
    pub id: MetricId,
    /// Name
    pub name: String,
    /// Type
    pub metric_type: MetricType,
    /// Description
    pub description: String,
    /// Unit
    pub unit: Option<String>,
    /// Labels
    pub labels: Vec<String>,
}

/// Counter metric
pub struct Counter {
    /// Definition
    def: MetricDef,
    /// Values by label set
    values: BTreeMap<String, AtomicU64>,
}

impl Counter {
    pub fn new(name: String, description: String) -> Self {
        Self {
            def: MetricDef {
                id: MetricId::generate(),
                name,
                metric_type: MetricType::Counter,
                description,
                unit: None,
                labels: Vec::new(),
            },
            values: BTreeMap::new(),
        }
    }

    /// Increment
    pub fn inc(&self, labels: &str) {
        self.add(labels, 1);
    }

    /// Add value
    pub fn add(&self, labels: &str, value: u64) {
        if let Some(counter) = self.values.get(labels) {
            counter.fetch_add(value, Ordering::Relaxed);
        }
    }

    /// Get value
    pub fn get(&self, labels: &str) -> u64 {
        self.values
            .get(labels)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Register labels
    pub fn register_labels(&mut self, labels: &str) {
        if !self.values.contains_key(labels) {
            self.values.insert(labels.to_string(), AtomicU64::new(0));
        }
    }
}

/// Gauge metric
pub struct Gauge {
    /// Definition
    def: MetricDef,
    /// Values by label set
    values: BTreeMap<String, AtomicU64>,
}

impl Gauge {
    pub fn new(name: String, description: String) -> Self {
        Self {
            def: MetricDef {
                id: MetricId::generate(),
                name,
                metric_type: MetricType::Gauge,
                description,
                unit: None,
                labels: Vec::new(),
            },
            values: BTreeMap::new(),
        }
    }

    /// Set value
    pub fn set(&self, labels: &str, value: f64) {
        if let Some(gauge) = self.values.get(labels) {
            gauge.store(value.to_bits(), Ordering::Relaxed);
        }
    }

    /// Increment
    pub fn inc(&self, labels: &str) {
        self.add(labels, 1.0);
    }

    /// Decrement
    pub fn dec(&self, labels: &str) {
        self.add(labels, -1.0);
    }

    /// Add value
    pub fn add(&self, labels: &str, delta: f64) {
        if let Some(gauge) = self.values.get(labels) {
            loop {
                let current = gauge.load(Ordering::Relaxed);
                let current_f = f64::from_bits(current);
                let new = current_f + delta;
                if gauge
                    .compare_exchange(current, new.to_bits(), Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
            }
        }
    }

    /// Get value
    pub fn get(&self, labels: &str) -> f64 {
        self.values
            .get(labels)
            .map(|g| f64::from_bits(g.load(Ordering::Relaxed)))
            .unwrap_or(0.0)
    }

    /// Register labels
    pub fn register_labels(&mut self, labels: &str) {
        if !self.values.contains_key(labels) {
            self.values.insert(labels.to_string(), AtomicU64::new(0));
        }
    }
}

/// Histogram metric
pub struct Histogram {
    /// Definition
    def: MetricDef,
    /// Bucket boundaries
    buckets: Vec<f64>,
    /// Bucket counts by label set
    bucket_counts: BTreeMap<String, Vec<AtomicU64>>,
    /// Sum by label set
    sums: BTreeMap<String, AtomicU64>,
    /// Count by label set
    counts: BTreeMap<String, AtomicU64>,
}

impl Histogram {
    pub fn new(name: String, description: String, buckets: Vec<f64>) -> Self {
        Self {
            def: MetricDef {
                id: MetricId::generate(),
                name,
                metric_type: MetricType::Histogram,
                description,
                unit: None,
                labels: Vec::new(),
            },
            buckets,
            bucket_counts: BTreeMap::new(),
            sums: BTreeMap::new(),
            counts: BTreeMap::new(),
        }
    }

    /// Observe value
    pub fn observe(&self, labels: &str, value: f64) {
        // Update bucket counts
        if let Some(buckets) = self.bucket_counts.get(labels) {
            for (i, &boundary) in self.buckets.iter().enumerate() {
                if value <= boundary {
                    buckets[i].fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }
            // +Inf bucket
            if value > *self.buckets.last().unwrap_or(&f64::INFINITY) {
                if let Some(last) = buckets.last() {
                    last.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Update sum
        if let Some(sum) = self.sums.get(labels) {
            loop {
                let current = sum.load(Ordering::Relaxed);
                let current_f = f64::from_bits(current);
                let new = current_f + value;
                if sum
                    .compare_exchange(current, new.to_bits(), Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    break;
                }
            }
        }

        // Update count
        if let Some(count) = self.counts.get(labels) {
            count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Register labels
    pub fn register_labels(&mut self, labels: &str) {
        if !self.bucket_counts.contains_key(labels) {
            let bucket_atomics: Vec<_> = (0..=self.buckets.len())
                .map(|_| AtomicU64::new(0))
                .collect();
            self.bucket_counts
                .insert(labels.to_string(), bucket_atomics);
            self.sums.insert(labels.to_string(), AtomicU64::new(0));
            self.counts.insert(labels.to_string(), AtomicU64::new(0));
        }
    }

    /// Get count
    pub fn count(&self, labels: &str) -> u64 {
        self.counts
            .get(labels)
            .map(|c| c.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// Get sum
    pub fn sum(&self, labels: &str) -> f64 {
        self.sums
            .get(labels)
            .map(|s| f64::from_bits(s.load(Ordering::Relaxed)))
            .unwrap_or(0.0)
    }
}

// ============================================================================
// TRACING
// ============================================================================

/// Span
#[derive(Debug, Clone)]
pub struct Span {
    /// Span ID
    pub id: SpanId,
    /// Trace ID
    pub trace_id: TraceId,
    /// Parent span ID
    pub parent_id: Option<SpanId>,
    /// Operation name
    pub operation: String,
    /// Start time (ticks)
    pub start_time: u64,
    /// End time (ticks)
    pub end_time: Option<u64>,
    /// Tags
    pub tags: BTreeMap<String, String>,
    /// Logs
    pub logs: Vec<SpanLog>,
    /// Status
    pub status: SpanStatus,
    /// Node
    pub node: NodeId,
}

/// Span log entry
#[derive(Debug, Clone)]
pub struct SpanLog {
    /// Timestamp
    pub timestamp: u64,
    /// Message
    pub message: String,
    /// Level
    pub level: LogLevel,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    Ok,
    Error,
    Cancelled,
}

impl Span {
    pub fn new(operation: String, trace_id: TraceId, node: NodeId, start_time: u64) -> Self {
        Self {
            id: SpanId::generate(),
            trace_id,
            parent_id: None,
            operation,
            start_time,
            end_time: None,
            tags: BTreeMap::new(),
            logs: Vec::new(),
            status: SpanStatus::Ok,
            node,
        }
    }

    pub fn with_parent(
        operation: String,
        trace_id: TraceId,
        parent_id: SpanId,
        node: NodeId,
        start_time: u64,
    ) -> Self {
        Self {
            id: SpanId::generate(),
            trace_id,
            parent_id: Some(parent_id),
            operation,
            start_time,
            end_time: None,
            tags: BTreeMap::new(),
            logs: Vec::new(),
            status: SpanStatus::Ok,
            node,
        }
    }

    /// Set tag
    pub fn set_tag(&mut self, key: &str, value: &str) {
        self.tags.insert(key.to_string(), value.to_string());
    }

    /// Log message
    pub fn log(&mut self, level: LogLevel, message: String, timestamp: u64) {
        self.logs.push(SpanLog {
            timestamp,
            message,
            level,
        });
    }

    /// Finish span
    pub fn finish(&mut self, end_time: u64) {
        self.end_time = Some(end_time);
    }

    /// Set error
    pub fn set_error(&mut self) {
        self.status = SpanStatus::Error;
    }

    /// Duration
    pub fn duration(&self) -> Option<u64> {
        self.end_time.map(|end| end - self.start_time)
    }
}

/// Tracer
pub struct Tracer {
    /// Node ID
    node_id: NodeId,
    /// Active spans
    active_spans: BTreeMap<SpanId, Span>,
    /// Completed spans
    completed_spans: Vec<Span>,
    /// Max completed spans to keep
    max_completed: usize,
    /// Current tick
    tick: AtomicU64,
    /// Sampling rate
    sampling_rate: f64,
    /// Random state
    random_state: AtomicU64,
}

impl Tracer {
    pub fn new(node_id: NodeId, max_completed: usize) -> Self {
        Self {
            node_id,
            active_spans: BTreeMap::new(),
            completed_spans: Vec::new(),
            max_completed,
            tick: AtomicU64::new(0),
            sampling_rate: 1.0,
            random_state: AtomicU64::new(0xDEADBEEF),
        }
    }

    fn should_sample(&self) -> bool {
        if self.sampling_rate >= 1.0 {
            return true;
        }

        let mut x = self.random_state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.random_state.store(x, Ordering::Relaxed);

        (x as f64) / (u64::MAX as f64) < self.sampling_rate
    }

    /// Start trace
    pub fn start_trace(&mut self, operation: String) -> Option<SpanId> {
        if !self.should_sample() {
            return None;
        }

        let tick = self.tick.load(Ordering::Relaxed);
        let trace_id = TraceId::generate();
        let span = Span::new(operation, trace_id, self.node_id, tick);
        let id = span.id;

        self.active_spans.insert(id, span);
        Some(id)
    }

    /// Start child span
    pub fn start_span(&mut self, operation: String, parent: SpanId) -> Option<SpanId> {
        let parent_span = self.active_spans.get(&parent)?;
        let trace_id = parent_span.trace_id;
        let tick = self.tick.load(Ordering::Relaxed);

        let span = Span::with_parent(operation, trace_id, parent, self.node_id, tick);
        let id = span.id;

        self.active_spans.insert(id, span);
        Some(id)
    }

    /// Get span
    pub fn get_span(&mut self, id: SpanId) -> Option<&mut Span> {
        self.active_spans.get_mut(&id)
    }

    /// Finish span
    pub fn finish_span(&mut self, id: SpanId) {
        let tick = self.tick.load(Ordering::Relaxed);

        if let Some(mut span) = self.active_spans.remove(&id) {
            span.finish(tick);
            self.completed_spans.push(span);

            // Trim completed
            if self.completed_spans.len() > self.max_completed {
                self.completed_spans.drain(0..self.max_completed / 2);
            }
        }
    }

    /// Set sampling rate
    pub fn set_sampling_rate(&mut self, rate: f64) {
        self.sampling_rate = rate.clamp(0.0, 1.0);
    }

    /// Tick
    pub fn tick(&self) {
        self.tick.fetch_add(1, Ordering::Relaxed);
    }

    /// Get completed spans for trace
    pub fn get_trace(&self, trace_id: TraceId) -> Vec<&Span> {
        self.completed_spans
            .iter()
            .filter(|s| s.trace_id == trace_id)
            .collect()
    }
}

// ============================================================================
// ALERTS
// ============================================================================

/// Alert ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AlertId(pub u64);

static ALERT_COUNTER: AtomicU64 = AtomicU64::new(1);

impl AlertId {
    pub fn generate() -> Self {
        Self(ALERT_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Alert rule
#[derive(Debug, Clone)]
pub struct AlertRule {
    /// ID
    pub id: AlertId,
    /// Name
    pub name: String,
    /// Condition
    pub condition: AlertCondition,
    /// Severity
    pub severity: AlertSeverity,
    /// Duration (ticks condition must be true)
    pub for_duration: u64,
    /// Enabled
    pub enabled: bool,
    /// Labels
    pub labels: BTreeMap<String, String>,
    /// Annotations
    pub annotations: BTreeMap<String, String>,
}

/// Alert condition
#[derive(Debug, Clone)]
pub enum AlertCondition {
    MetricAbove {
        metric: String,
        threshold: f64,
    },
    MetricBelow {
        metric: String,
        threshold: f64,
    },
    MetricAbsent {
        metric: String,
        duration: u64,
    },
    RateOfChange {
        metric: String,
        threshold: f64,
        window: u64,
    },
    And(Vec<AlertCondition>),
    Or(Vec<AlertCondition>),
}

/// Alert severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

/// Alert instance
#[derive(Debug, Clone)]
pub struct AlertInstance {
    /// Rule ID
    pub rule_id: AlertId,
    /// State
    pub state: AlertState,
    /// Started at
    pub started_at: u64,
    /// Resolved at
    pub resolved_at: Option<u64>,
    /// Value that triggered
    pub value: f64,
    /// Labels
    pub labels: BTreeMap<String, String>,
}

/// Alert state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertState {
    Pending,
    Firing,
    Resolved,
}

/// Alert manager
pub struct AlertManager {
    /// Rules
    rules: Vec<AlertRule>,
    /// Active alerts
    active: BTreeMap<AlertId, AlertInstance>,
    /// Alert history
    history: Vec<AlertInstance>,
    /// Max history
    max_history: usize,
    /// Current tick
    tick: AtomicU64,
    /// Pending conditions (rule_id -> start_tick)
    pending: BTreeMap<AlertId, u64>,
}

impl AlertManager {
    pub fn new(max_history: usize) -> Self {
        Self {
            rules: Vec::new(),
            active: BTreeMap::new(),
            history: Vec::new(),
            max_history,
            tick: AtomicU64::new(0),
            pending: BTreeMap::new(),
        }
    }

    /// Add rule
    pub fn add_rule(&mut self, rule: AlertRule) {
        self.rules.push(rule);
    }

    /// Remove rule
    pub fn remove_rule(&mut self, id: AlertId) {
        self.rules.retain(|r| r.id != id);
    }

    /// Evaluate rules
    pub fn evaluate(&mut self, metrics: &BTreeMap<String, f64>) -> Vec<AlertInstance> {
        let tick = self.tick.load(Ordering::Relaxed);
        let mut new_alerts = Vec::new();

        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            let condition_met = self.check_condition(&rule.condition, metrics);

            if condition_met {
                if let Some(&start) = self.pending.get(&rule.id) {
                    // Check if for_duration has passed
                    if tick - start >= rule.for_duration {
                        if !self.active.contains_key(&rule.id) {
                            let alert = AlertInstance {
                                rule_id: rule.id,
                                state: AlertState::Firing,
                                started_at: start,
                                resolved_at: None,
                                value: 0.0,
                                labels: rule.labels.clone(),
                            };

                            self.active.insert(rule.id, alert.clone());
                            new_alerts.push(alert);
                        }
                    }
                } else {
                    self.pending.insert(rule.id, tick);
                }
            } else {
                self.pending.remove(&rule.id);

                // Resolve if was active
                if let Some(mut alert) = self.active.remove(&rule.id) {
                    alert.state = AlertState::Resolved;
                    alert.resolved_at = Some(tick);
                    self.history.push(alert);

                    // Trim history
                    if self.history.len() > self.max_history {
                        self.history.drain(0..self.max_history / 2);
                    }
                }
            }
        }

        new_alerts
    }

    fn check_condition(&self, condition: &AlertCondition, metrics: &BTreeMap<String, f64>) -> bool {
        match condition {
            AlertCondition::MetricAbove { metric, threshold } => metrics
                .get(metric)
                .map(|&v| v > *threshold)
                .unwrap_or(false),
            AlertCondition::MetricBelow { metric, threshold } => metrics
                .get(metric)
                .map(|&v| v < *threshold)
                .unwrap_or(false),
            AlertCondition::MetricAbsent {
                metric,
                duration: _,
            } => !metrics.contains_key(metric),
            AlertCondition::RateOfChange {
                metric: _,
                threshold: _,
                window: _,
            } => {
                // Would need historical data
                false
            },
            AlertCondition::And(conditions) => {
                conditions.iter().all(|c| self.check_condition(c, metrics))
            },
            AlertCondition::Or(conditions) => {
                conditions.iter().any(|c| self.check_condition(c, metrics))
            },
        }
    }

    /// Tick
    pub fn tick(&self) {
        self.tick.fetch_add(1, Ordering::Relaxed);
    }

    /// Get active alerts
    pub fn active_alerts(&self) -> Vec<&AlertInstance> {
        self.active.values().collect()
    }

    /// Get alerts by severity
    pub fn alerts_by_severity(&self, severity: AlertSeverity) -> Vec<&AlertRule> {
        self.rules
            .iter()
            .filter(|r| r.severity == severity && self.active.contains_key(&r.id))
            .collect()
    }
}

// ============================================================================
// HEALTH CHECKS
// ============================================================================

/// Health check
pub struct HealthCheck {
    /// Name
    pub name: String,
    /// Check function result
    pub status: HealthStatus,
    /// Last check time
    pub last_check: u64,
    /// Details
    pub details: BTreeMap<String, String>,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

/// Health checker
pub struct HealthChecker {
    /// Checks
    checks: BTreeMap<String, HealthCheck>,
    /// Overall status
    overall: HealthStatus,
    /// Tick
    tick: AtomicU64,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            checks: BTreeMap::new(),
            overall: HealthStatus::Unknown,
            tick: AtomicU64::new(0),
        }
    }

    /// Register check
    pub fn register(&mut self, name: &str) {
        self.checks.insert(name.to_string(), HealthCheck {
            name: name.to_string(),
            status: HealthStatus::Unknown,
            last_check: 0,
            details: BTreeMap::new(),
        });
    }

    /// Update check
    pub fn update(&mut self, name: &str, status: HealthStatus, details: BTreeMap<String, String>) {
        let tick = self.tick.load(Ordering::Relaxed);

        if let Some(check) = self.checks.get_mut(name) {
            check.status = status;
            check.last_check = tick;
            check.details = details;
        }

        self.update_overall();
    }

    fn update_overall(&mut self) {
        let mut has_unhealthy = false;
        let mut has_degraded = false;
        let mut has_unknown = false;

        for check in self.checks.values() {
            match check.status {
                HealthStatus::Unhealthy => has_unhealthy = true,
                HealthStatus::Degraded => has_degraded = true,
                HealthStatus::Unknown => has_unknown = true,
                HealthStatus::Healthy => {},
            }
        }

        self.overall = if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else if has_unknown {
            HealthStatus::Unknown
        } else {
            HealthStatus::Healthy
        };
    }

    /// Get overall status
    pub fn overall(&self) -> HealthStatus {
        self.overall
    }

    /// Get all checks
    pub fn checks(&self) -> &BTreeMap<String, HealthCheck> {
        &self.checks
    }

    /// Tick
    pub fn tick(&self) {
        self.tick.fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter() {
        let mut counter = Counter::new(
            String::from("requests_total"),
            String::from("Total requests"),
        );

        counter.register_labels("method=GET");
        counter.inc("method=GET");
        counter.inc("method=GET");
        counter.add("method=GET", 3);

        assert_eq!(counter.get("method=GET"), 5);
    }

    #[test]
    fn test_tracer() {
        let mut tracer = Tracer::new(NodeId(1), 100);

        let span_id = tracer.start_trace(String::from("request")).unwrap();

        if let Some(span) = tracer.get_span(span_id) {
            span.set_tag("http.method", "GET");
            span.log(LogLevel::Info, String::from("Processing request"), 0);
        }

        let child_id = tracer
            .start_span(String::from("database_query"), span_id)
            .unwrap();
        tracer.finish_span(child_id);
        tracer.finish_span(span_id);

        assert_eq!(tracer.completed_spans.len(), 2);
    }

    #[test]
    fn test_alert_manager() {
        let mut am = AlertManager::new(100);

        am.add_rule(AlertRule {
            id: AlertId::generate(),
            name: String::from("high_cpu"),
            condition: AlertCondition::MetricAbove {
                metric: String::from("cpu_usage"),
                threshold: 80.0,
            },
            severity: AlertSeverity::Warning,
            for_duration: 0,
            enabled: true,
            labels: BTreeMap::new(),
            annotations: BTreeMap::new(),
        });

        let mut metrics = BTreeMap::new();
        metrics.insert(String::from("cpu_usage"), 90.0);

        let alerts = am.evaluate(&metrics);
        assert_eq!(alerts.len(), 1);
    }

    #[test]
    fn test_health_checker() {
        let mut hc = HealthChecker::new();

        hc.register("database");
        hc.register("cache");

        hc.update("database", HealthStatus::Healthy, BTreeMap::new());
        hc.update("cache", HealthStatus::Healthy, BTreeMap::new());

        assert_eq!(hc.overall(), HealthStatus::Healthy);

        hc.update("cache", HealthStatus::Degraded, BTreeMap::new());
        assert_eq!(hc.overall(), HealthStatus::Degraded);
    }
}
