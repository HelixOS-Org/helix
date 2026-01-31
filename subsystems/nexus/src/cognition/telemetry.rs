//! # Cognitive Telemetry
//!
//! Telemetry collection and export for cognitive systems.
//! Supports metrics, traces, and logs aggregation.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// TELEMETRY TYPES
// ============================================================================

/// A telemetry point
#[derive(Debug, Clone)]
pub struct TelemetryPoint {
    /// Point ID
    pub id: u64,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Metric name
    pub name: String,
    /// Value
    pub value: TelemetryValue,
    /// Tags
    pub tags: BTreeMap<String, String>,
    /// Source domain
    pub source: DomainId,
}

/// Telemetry value
#[derive(Debug, Clone)]
pub enum TelemetryValue {
    /// Counter (monotonically increasing)
    Counter(u64),
    /// Gauge (can go up or down)
    Gauge(f64),
    /// Histogram bucket
    Histogram {
        sum: f64,
        count: u64,
        buckets: Vec<(f64, u64)>,
    },
    /// Summary with quantiles
    Summary {
        sum: f64,
        count: u64,
        quantiles: Vec<(f32, f64)>,
    },
    /// Boolean
    Bool(bool),
    /// String
    String(String),
}

/// Telemetry event
#[derive(Debug, Clone)]
pub struct TelemetryEvent {
    /// Event ID
    pub id: u64,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Event name
    pub name: String,
    /// Severity
    pub severity: EventSeverity,
    /// Message
    pub message: String,
    /// Attributes
    pub attributes: BTreeMap<String, String>,
    /// Source domain
    pub source: DomainId,
}

/// Event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Resource information
#[derive(Debug, Clone)]
pub struct Resource {
    /// Resource type
    pub resource_type: String,
    /// Attributes
    pub attributes: BTreeMap<String, String>,
}

// ============================================================================
// TELEMETRY COLLECTOR
// ============================================================================

/// Collects telemetry data
pub struct TelemetryCollector {
    /// Metric points
    points: Vec<TelemetryPoint>,
    /// Events
    events: Vec<TelemetryEvent>,
    /// Metric series (aggregated)
    series: BTreeMap<String, MetricSeries>,
    /// Next point ID
    next_point_id: AtomicU64,
    /// Next event ID
    next_event_id: AtomicU64,
    /// Configuration
    config: TelemetryConfig,
    /// Resource
    resource: Resource,
    /// Statistics
    stats: TelemetryStats,
}

/// Metric time series
#[derive(Debug, Clone)]
pub struct MetricSeries {
    /// Series name
    pub name: String,
    /// Latest value
    pub latest: TelemetryValue,
    /// Sample count
    pub samples: u64,
    /// First sample time
    pub first_time: Timestamp,
    /// Last sample time
    pub last_time: Timestamp,
    /// Minimum (for gauge)
    pub min: Option<f64>,
    /// Maximum (for gauge)
    pub max: Option<f64>,
    /// Sum (for aggregation)
    pub sum: f64,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Maximum points to buffer
    pub max_points: usize,
    /// Maximum events to buffer
    pub max_events: usize,
    /// Default tags
    pub default_tags: BTreeMap<String, String>,
    /// Enable aggregation
    pub enable_aggregation: bool,
    /// Flush interval (nanoseconds)
    pub flush_interval_ns: u64,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            max_points: 100000,
            max_events: 10000,
            default_tags: BTreeMap::new(),
            enable_aggregation: true,
            flush_interval_ns: 10_000_000_000, // 10 seconds
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct TelemetryStats {
    /// Total points collected
    pub total_points: u64,
    /// Total events collected
    pub total_events: u64,
    /// Points dropped
    pub points_dropped: u64,
    /// Events dropped
    pub events_dropped: u64,
    /// Flushes
    pub flushes: u64,
    /// Last flush time
    pub last_flush: Option<Timestamp>,
}

impl TelemetryCollector {
    /// Create a new collector
    pub fn new(config: TelemetryConfig, resource: Resource) -> Self {
        Self {
            points: Vec::new(),
            events: Vec::new(),
            series: BTreeMap::new(),
            next_point_id: AtomicU64::new(1),
            next_event_id: AtomicU64::new(1),
            config,
            resource,
            stats: TelemetryStats::default(),
        }
    }

    /// Record a metric point
    pub fn record_metric(
        &mut self,
        name: &str,
        value: TelemetryValue,
        tags: BTreeMap<String, String>,
        source: DomainId,
    ) -> u64 {
        let id = self.next_point_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        // Check buffer limit
        if self.points.len() >= self.config.max_points {
            self.points.remove(0);
            self.stats.points_dropped += 1;
        }

        // Merge default tags
        let mut all_tags = self.config.default_tags.clone();
        all_tags.extend(tags);

        let point = TelemetryPoint {
            id,
            timestamp: now,
            name: name.into(),
            value: value.clone(),
            tags: all_tags,
            source,
        };

        self.points.push(point);
        self.stats.total_points += 1;

        // Update series if aggregation enabled
        if self.config.enable_aggregation {
            self.update_series(name, &value, now);
        }

        id
    }

    /// Update metric series
    fn update_series(&mut self, name: &str, value: &TelemetryValue, timestamp: Timestamp) {
        let series = self.series.entry(name.into()).or_insert_with(|| {
            MetricSeries {
                name: name.into(),
                latest: value.clone(),
                samples: 0,
                first_time: timestamp,
                last_time: timestamp,
                min: None,
                max: None,
                sum: 0.0,
            }
        });

        series.latest = value.clone();
        series.samples += 1;
        series.last_time = timestamp;

        // Update min/max/sum for gauges
        if let TelemetryValue::Gauge(v) = value {
            series.sum += v;
            series.min = Some(series.min.map(|m| m.min(*v)).unwrap_or(*v));
            series.max = Some(series.max.map(|m| m.max(*v)).unwrap_or(*v));
        }
    }

    /// Record a counter increment
    pub fn increment_counter(&mut self, name: &str, amount: u64, source: DomainId) -> u64 {
        // Get current value
        let current = self.series.get(name)
            .and_then(|s| {
                if let TelemetryValue::Counter(v) = s.latest {
                    Some(v)
                } else {
                    None
                }
            })
            .unwrap_or(0);

        self.record_metric(
            name,
            TelemetryValue::Counter(current + amount),
            BTreeMap::new(),
            source,
        )
    }

    /// Record a gauge value
    pub fn record_gauge(&mut self, name: &str, value: f64, source: DomainId) -> u64 {
        self.record_metric(
            name,
            TelemetryValue::Gauge(value),
            BTreeMap::new(),
            source,
        )
    }

    /// Record an event
    pub fn record_event(
        &mut self,
        name: &str,
        severity: EventSeverity,
        message: &str,
        attributes: BTreeMap<String, String>,
        source: DomainId,
    ) -> u64 {
        let id = self.next_event_id.fetch_add(1, Ordering::Relaxed);

        // Check buffer limit
        if self.events.len() >= self.config.max_events {
            self.events.remove(0);
            self.stats.events_dropped += 1;
        }

        let event = TelemetryEvent {
            id,
            timestamp: Timestamp::now(),
            name: name.into(),
            severity,
            message: message.into(),
            attributes,
            source,
        };

        self.events.push(event);
        self.stats.total_events += 1;

        id
    }

    /// Record info event
    pub fn info(&mut self, name: &str, message: &str, source: DomainId) -> u64 {
        self.record_event(name, EventSeverity::Info, message, BTreeMap::new(), source)
    }

    /// Record warning event
    pub fn warn(&mut self, name: &str, message: &str, source: DomainId) -> u64 {
        self.record_event(name, EventSeverity::Warn, message, BTreeMap::new(), source)
    }

    /// Record error event
    pub fn error(&mut self, name: &str, message: &str, source: DomainId) -> u64 {
        self.record_event(name, EventSeverity::Error, message, BTreeMap::new(), source)
    }

    /// Get metric series
    pub fn get_series(&self, name: &str) -> Option<&MetricSeries> {
        self.series.get(name)
    }

    /// Get all series
    pub fn all_series(&self) -> Vec<&MetricSeries> {
        self.series.values().collect()
    }

    /// Get recent points
    pub fn recent_points(&self, count: usize) -> Vec<&TelemetryPoint> {
        self.points.iter().rev().take(count).collect()
    }

    /// Get recent events
    pub fn recent_events(&self, count: usize) -> Vec<&TelemetryEvent> {
        self.events.iter().rev().take(count).collect()
    }

    /// Get events by severity
    pub fn events_by_severity(&self, min_severity: EventSeverity) -> Vec<&TelemetryEvent> {
        self.events.iter()
            .filter(|e| e.severity >= min_severity)
            .collect()
    }

    /// Flush and return all buffered data
    pub fn flush(&mut self) -> TelemetryBatch {
        self.stats.flushes += 1;
        self.stats.last_flush = Some(Timestamp::now());

        TelemetryBatch {
            points: core::mem::take(&mut self.points),
            events: core::mem::take(&mut self.events),
            resource: self.resource.clone(),
            timestamp: Timestamp::now(),
        }
    }

    /// Clear all data
    pub fn clear(&mut self) {
        self.points.clear();
        self.events.clear();
        self.series.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> &TelemetryStats {
        &self.stats
    }

    /// Get resource
    pub fn resource(&self) -> &Resource {
        &self.resource
    }
}

/// A batch of telemetry data
#[derive(Debug, Clone)]
pub struct TelemetryBatch {
    /// Metric points
    pub points: Vec<TelemetryPoint>,
    /// Events
    pub events: Vec<TelemetryEvent>,
    /// Resource
    pub resource: Resource,
    /// Batch timestamp
    pub timestamp: Timestamp,
}

// ============================================================================
// TELEMETRY EXPORTER
// ============================================================================

/// Telemetry export format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON
    Json,
    /// Protocol Buffers (placeholder)
    Protobuf,
    /// OpenTelemetry (placeholder)
    Otlp,
    /// Prometheus text format
    Prometheus,
}

/// Telemetry exporter
pub struct TelemetryExporter {
    /// Export format
    format: ExportFormat,
    /// Export buffer
    buffer: Vec<u8>,
    /// Batches exported
    batches_exported: u64,
}

impl TelemetryExporter {
    /// Create a new exporter
    pub fn new(format: ExportFormat) -> Self {
        Self {
            format,
            buffer: Vec::new(),
            batches_exported: 0,
        }
    }

    /// Export a batch
    pub fn export(&mut self, batch: &TelemetryBatch) -> Vec<u8> {
        self.batches_exported += 1;
        self.buffer.clear();

        match self.format {
            ExportFormat::Json => self.export_json(batch),
            ExportFormat::Prometheus => self.export_prometheus(batch),
            _ => self.export_json(batch), // Fallback to JSON
        }

        self.buffer.clone()
    }

    /// Export as JSON (simplified)
    fn export_json(&mut self, batch: &TelemetryBatch) {
        // Simple JSON format
        self.buffer.extend_from_slice(b"{\"points\":[");

        for (i, point) in batch.points.iter().enumerate() {
            if i > 0 {
                self.buffer.push(b',');
            }
            self.export_point_json(point);
        }

        self.buffer.extend_from_slice(b"],\"events\":[");

        for (i, event) in batch.events.iter().enumerate() {
            if i > 0 {
                self.buffer.push(b',');
            }
            self.export_event_json(event);
        }

        self.buffer.extend_from_slice(b"]}");
    }

    /// Export point as JSON
    fn export_point_json(&mut self, point: &TelemetryPoint) {
        self.buffer.extend_from_slice(b"{\"name\":\"");
        self.buffer.extend_from_slice(point.name.as_bytes());
        self.buffer.extend_from_slice(b"\",\"timestamp\":");
        self.buffer.extend_from_slice(format!("{}", point.timestamp.raw()).as_bytes());
        self.buffer.extend_from_slice(b",\"value\":");

        match &point.value {
            TelemetryValue::Counter(v) => {
                self.buffer.extend_from_slice(format!("{}", v).as_bytes());
            }
            TelemetryValue::Gauge(v) => {
                self.buffer.extend_from_slice(format!("{}", v).as_bytes());
            }
            _ => {
                self.buffer.extend_from_slice(b"0");
            }
        }

        self.buffer.push(b'}');
    }

    /// Export event as JSON
    fn export_event_json(&mut self, event: &TelemetryEvent) {
        self.buffer.extend_from_slice(b"{\"name\":\"");
        self.buffer.extend_from_slice(event.name.as_bytes());
        self.buffer.extend_from_slice(b"\",\"severity\":\"");
        let severity_str = match event.severity {
            EventSeverity::Trace => "trace",
            EventSeverity::Debug => "debug",
            EventSeverity::Info => "info",
            EventSeverity::Warn => "warn",
            EventSeverity::Error => "error",
            EventSeverity::Fatal => "fatal",
        };
        self.buffer.extend_from_slice(severity_str.as_bytes());
        self.buffer.extend_from_slice(b"\",\"message\":\"");
        self.buffer.extend_from_slice(event.message.as_bytes());
        self.buffer.extend_from_slice(b"\"}");
    }

    /// Export as Prometheus format
    fn export_prometheus(&mut self, batch: &TelemetryBatch) {
        for point in &batch.points {
            // # HELP
            self.buffer.extend_from_slice(b"# HELP ");
            self.buffer.extend_from_slice(point.name.as_bytes());
            self.buffer.extend_from_slice(b"\n# TYPE ");
            self.buffer.extend_from_slice(point.name.as_bytes());

            let type_str = match &point.value {
                TelemetryValue::Counter(_) => " counter\n",
                TelemetryValue::Gauge(_) => " gauge\n",
                TelemetryValue::Histogram { .. } => " histogram\n",
                TelemetryValue::Summary { .. } => " summary\n",
                _ => " untyped\n",
            };
            self.buffer.extend_from_slice(type_str.as_bytes());

            // Metric line
            self.buffer.extend_from_slice(point.name.as_bytes());

            // Labels
            if !point.tags.is_empty() {
                self.buffer.push(b'{');
                for (i, (k, v)) in point.tags.iter().enumerate() {
                    if i > 0 {
                        self.buffer.push(b',');
                    }
                    self.buffer.extend_from_slice(k.as_bytes());
                    self.buffer.extend_from_slice(b"=\"");
                    self.buffer.extend_from_slice(v.as_bytes());
                    self.buffer.push(b'"');
                }
                self.buffer.push(b'}');
            }

            self.buffer.push(b' ');

            // Value
            match &point.value {
                TelemetryValue::Counter(v) => {
                    self.buffer.extend_from_slice(format!("{}", v).as_bytes());
                }
                TelemetryValue::Gauge(v) => {
                    self.buffer.extend_from_slice(format!("{}", v).as_bytes());
                }
                _ => {
                    self.buffer.extend_from_slice(b"0");
                }
            }

            self.buffer.push(b'\n');
        }
    }

    /// Get batches exported count
    pub fn batches_exported(&self) -> u64 {
        self.batches_exported
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_collector() -> TelemetryCollector {
        let config = TelemetryConfig::default();
        let resource = Resource {
            resource_type: "test".into(),
            attributes: BTreeMap::new(),
        };
        TelemetryCollector::new(config, resource)
    }

    #[test]
    fn test_record_counter() {
        let mut collector = create_collector();
        let domain = DomainId::new(1);

        collector.increment_counter("requests", 1, domain);
        collector.increment_counter("requests", 5, domain);

        let series = collector.get_series("requests").unwrap();
        assert!(matches!(series.latest, TelemetryValue::Counter(6)));
        assert_eq!(series.samples, 2);
    }

    #[test]
    fn test_record_gauge() {
        let mut collector = create_collector();
        let domain = DomainId::new(1);

        collector.record_gauge("cpu_usage", 45.5, domain);
        collector.record_gauge("cpu_usage", 55.0, domain);
        collector.record_gauge("cpu_usage", 30.0, domain);

        let series = collector.get_series("cpu_usage").unwrap();
        assert!(matches!(series.latest, TelemetryValue::Gauge(30.0)));
        assert_eq!(series.min, Some(30.0));
        assert_eq!(series.max, Some(55.0));
    }

    #[test]
    fn test_record_event() {
        let mut collector = create_collector();
        let domain = DomainId::new(1);

        collector.info("startup", "System started", domain);
        collector.warn("memory", "Low memory", domain);
        collector.error("failure", "Critical error", domain);

        let errors = collector.events_by_severity(EventSeverity::Error);
        assert_eq!(errors.len(), 1);

        let warnings = collector.events_by_severity(EventSeverity::Warn);
        assert_eq!(warnings.len(), 2);
    }

    #[test]
    fn test_flush() {
        let mut collector = create_collector();
        let domain = DomainId::new(1);

        collector.increment_counter("test", 1, domain);
        collector.info("test_event", "Test", domain);

        let batch = collector.flush();
        assert_eq!(batch.points.len(), 1);
        assert_eq!(batch.events.len(), 1);

        // After flush, buffers should be empty
        assert!(collector.points.is_empty());
        assert!(collector.events.is_empty());
    }

    #[test]
    fn test_export_prometheus() {
        let mut collector = create_collector();
        let domain = DomainId::new(1);

        collector.record_gauge("cpu_usage", 45.5, domain);

        let batch = collector.flush();
        let mut exporter = TelemetryExporter::new(ExportFormat::Prometheus);
        let output = exporter.export(&batch);

        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("cpu_usage"));
        assert!(output_str.contains("gauge"));
    }
}
