//! # Cognitive Tracing System
//!
//! Distributed tracing for cognitive operations.
//! Tracks causality and timing across domains.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// TRACE TYPES
// ============================================================================

/// A trace span
#[derive(Debug, Clone)]
pub struct Span {
    /// Span ID
    pub id: u64,
    /// Trace ID
    pub trace_id: u64,
    /// Parent span ID
    pub parent_id: Option<u64>,
    /// Operation name
    pub operation: String,
    /// Domain
    pub domain: DomainId,
    /// Start time
    pub start_time: Timestamp,
    /// End time
    pub end_time: Option<Timestamp>,
    /// Status
    pub status: SpanStatus,
    /// Tags
    pub tags: BTreeMap<String, String>,
    /// Logs
    pub logs: Vec<SpanLog>,
    /// Baggage (propagated data)
    pub baggage: BTreeMap<String, String>,
}

/// Span status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// In progress
    Active,
    /// Completed successfully
    Ok,
    /// Completed with error
    Error,
    /// Cancelled
    Cancelled,
    /// Timed out
    Timeout,
}

/// Span log entry
#[derive(Debug, Clone)]
pub struct SpanLog {
    /// Timestamp
    pub timestamp: Timestamp,
    /// Log level
    pub level: LogLevel,
    /// Message
    pub message: String,
    /// Fields
    pub fields: BTreeMap<String, String>,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// A complete trace
#[derive(Debug, Clone)]
pub struct Trace {
    /// Trace ID
    pub id: u64,
    /// Root span ID
    pub root_span_id: u64,
    /// All spans
    pub spans: Vec<Span>,
    /// Start time
    pub start_time: Timestamp,
    /// End time
    pub end_time: Option<Timestamp>,
    /// Is complete
    pub complete: bool,
}

/// Trace context for propagation
#[derive(Debug, Clone)]
pub struct TraceContext {
    /// Trace ID
    pub trace_id: u64,
    /// Current span ID
    pub span_id: u64,
    /// Sampling decision
    pub sampled: bool,
    /// Baggage
    pub baggage: BTreeMap<String, String>,
}

// ============================================================================
// TRACER
// ============================================================================

/// Distributed tracer
pub struct Tracer {
    /// Active spans
    active_spans: BTreeMap<u64, Span>,
    /// Completed spans by trace
    trace_spans: BTreeMap<u64, Vec<Span>>,
    /// Next span ID
    next_span_id: AtomicU64,
    /// Next trace ID
    next_trace_id: AtomicU64,
    /// Configuration
    config: TracerConfig,
    /// Statistics
    stats: TracerStats,
}

/// Tracer configuration
#[derive(Debug, Clone)]
pub struct TracerConfig {
    /// Maximum spans per trace
    pub max_spans_per_trace: usize,
    /// Maximum logs per span
    pub max_logs_per_span: usize,
    /// Maximum trace retention (count)
    pub max_traces: usize,
    /// Sampling rate (0.0 - 1.0)
    pub sampling_rate: f32,
    /// Enable baggage propagation
    pub enable_baggage: bool,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            max_spans_per_trace: 1000,
            max_logs_per_span: 100,
            max_traces: 1000,
            sampling_rate: 1.0,
            enable_baggage: true,
        }
    }
}

/// Tracer statistics
#[derive(Debug, Clone, Default)]
pub struct TracerStats {
    /// Total traces started
    pub total_traces: u64,
    /// Total spans created
    pub total_spans: u64,
    /// Active spans
    pub active_spans: u64,
    /// Completed traces
    pub completed_traces: u64,
    /// Error spans
    pub error_spans: u64,
    /// Average trace duration (ns)
    pub avg_trace_duration_ns: f64,
    /// Average spans per trace
    pub avg_spans_per_trace: f32,
}

impl Tracer {
    /// Create a new tracer
    pub fn new(config: TracerConfig) -> Self {
        Self {
            active_spans: BTreeMap::new(),
            trace_spans: BTreeMap::new(),
            next_span_id: AtomicU64::new(1),
            next_trace_id: AtomicU64::new(1),
            config,
            stats: TracerStats::default(),
        }
    }

    /// Start a new trace
    pub fn start_trace(&mut self, operation: &str, domain: DomainId) -> TraceContext {
        let trace_id = self.next_trace_id.fetch_add(1, Ordering::Relaxed);
        let span_id = self.next_span_id.fetch_add(1, Ordering::Relaxed);

        let span = Span {
            id: span_id,
            trace_id,
            parent_id: None,
            operation: operation.into(),
            domain,
            start_time: Timestamp::now(),
            end_time: None,
            status: SpanStatus::Active,
            tags: BTreeMap::new(),
            logs: Vec::new(),
            baggage: BTreeMap::new(),
        };

        self.active_spans.insert(span_id, span);
        self.trace_spans.insert(trace_id, Vec::new());
        self.stats.total_traces += 1;
        self.stats.total_spans += 1;
        self.stats.active_spans = self.active_spans.len() as u64;

        TraceContext {
            trace_id,
            span_id,
            sampled: true,
            baggage: BTreeMap::new(),
        }
    }

    /// Start a child span
    pub fn start_span(
        &mut self,
        context: &TraceContext,
        operation: &str,
        domain: DomainId,
    ) -> TraceContext {
        let span_id = self.next_span_id.fetch_add(1, Ordering::Relaxed);

        // Get baggage from parent
        let baggage = if self.config.enable_baggage {
            context.baggage.clone()
        } else {
            BTreeMap::new()
        };

        let span = Span {
            id: span_id,
            trace_id: context.trace_id,
            parent_id: Some(context.span_id),
            operation: operation.into(),
            domain,
            start_time: Timestamp::now(),
            end_time: None,
            status: SpanStatus::Active,
            tags: BTreeMap::new(),
            logs: Vec::new(),
            baggage: baggage.clone(),
        };

        self.active_spans.insert(span_id, span);
        self.stats.total_spans += 1;
        self.stats.active_spans = self.active_spans.len() as u64;

        TraceContext {
            trace_id: context.trace_id,
            span_id,
            sampled: context.sampled,
            baggage,
        }
    }

    /// Finish a span
    pub fn finish_span(&mut self, span_id: u64, status: SpanStatus) {
        if let Some(mut span) = self.active_spans.remove(&span_id) {
            span.end_time = Some(Timestamp::now());
            span.status = status;

            if status == SpanStatus::Error {
                self.stats.error_spans += 1;
            }

            // Add to trace spans
            if let Some(spans) = self.trace_spans.get_mut(&span.trace_id) {
                spans.push(span);
            }

            self.stats.active_spans = self.active_spans.len() as u64;
        }
    }

    /// Finish span with OK status
    pub fn finish_span_ok(&mut self, span_id: u64) {
        self.finish_span(span_id, SpanStatus::Ok);
    }

    /// Finish span with error
    pub fn finish_span_error(&mut self, span_id: u64, error: &str) {
        if let Some(span) = self.active_spans.get_mut(&span_id) {
            span.tags.insert("error".into(), "true".into());
            span.tags.insert("error.message".into(), error.into());
        }
        self.finish_span(span_id, SpanStatus::Error);
    }

    /// Add tag to span
    pub fn tag(&mut self, span_id: u64, key: &str, value: &str) {
        if let Some(span) = self.active_spans.get_mut(&span_id) {
            span.tags.insert(key.into(), value.into());
        }
    }

    /// Add log to span
    pub fn log(&mut self, span_id: u64, level: LogLevel, message: &str) {
        if let Some(span) = self.active_spans.get_mut(&span_id) {
            if span.logs.len() < self.config.max_logs_per_span {
                span.logs.push(SpanLog {
                    timestamp: Timestamp::now(),
                    level,
                    message: message.into(),
                    fields: BTreeMap::new(),
                });
            }
        }
    }

    /// Set baggage
    pub fn set_baggage(&mut self, context: &mut TraceContext, key: &str, value: &str) {
        if self.config.enable_baggage {
            context.baggage.insert(key.into(), value.into());

            // Update active span baggage
            if let Some(span) = self.active_spans.get_mut(&context.span_id) {
                span.baggage.insert(key.into(), value.into());
            }
        }
    }

    /// Get baggage
    pub fn get_baggage(&self, context: &TraceContext, key: &str) -> Option<&String> {
        context.baggage.get(key)
    }

    /// Complete a trace
    pub fn complete_trace(&mut self, trace_id: u64) -> Option<Trace> {
        // Finish any remaining active spans for this trace
        let active_for_trace: Vec<_> = self
            .active_spans
            .iter()
            .filter(|(_, span)| span.trace_id == trace_id)
            .map(|(id, _)| *id)
            .collect();

        for span_id in active_for_trace {
            self.finish_span(span_id, SpanStatus::Cancelled);
        }

        // Build trace
        let spans = self.trace_spans.remove(&trace_id)?;

        if spans.is_empty() {
            return None;
        }

        // Find root span
        let root = spans.iter().find(|s| s.parent_id.is_none()).cloned()?;

        let start_time = root.start_time;
        let end_time = spans.iter().filter_map(|s| s.end_time).max();

        let trace = Trace {
            id: trace_id,
            root_span_id: root.id,
            spans,
            start_time,
            end_time,
            complete: true,
        };

        // Update stats
        self.stats.completed_traces += 1;
        if let Some(end) = end_time {
            let duration = end.elapsed_since(start_time);
            self.stats.avg_trace_duration_ns = (self.stats.avg_trace_duration_ns
                * (self.stats.completed_traces - 1) as f64
                + duration as f64)
                / self.stats.completed_traces as f64;
        }
        self.stats.avg_spans_per_trace = (self.stats.avg_spans_per_trace
            * (self.stats.completed_traces - 1) as f32
            + trace.spans.len() as f32)
            / self.stats.completed_traces as f32;

        Some(trace)
    }

    /// Get active span
    pub fn get_span(&self, span_id: u64) -> Option<&Span> {
        self.active_spans.get(&span_id)
    }

    /// Get statistics
    pub fn stats(&self) -> &TracerStats {
        &self.stats
    }
}

// ============================================================================
// SPAN BUILDER
// ============================================================================

/// Builder for creating spans
pub struct SpanBuilder<'a> {
    tracer: &'a mut Tracer,
    context: TraceContext,
    tags: BTreeMap<String, String>,
}

impl<'a> SpanBuilder<'a> {
    /// Create a new span builder
    pub fn new(tracer: &'a mut Tracer, context: TraceContext) -> Self {
        Self {
            tracer,
            context,
            tags: BTreeMap::new(),
        }
    }

    /// Add a tag
    pub fn tag(mut self, key: &str, value: &str) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Apply tags to span
    pub fn apply(self) -> TraceContext {
        for (key, value) in self.tags {
            self.tracer.tag(self.context.span_id, &key, &value);
        }
        self.context
    }
}

// ============================================================================
// TRACE ANALYZER
// ============================================================================

/// Analyzes traces for patterns
pub struct TraceAnalyzer {
    /// Traces
    traces: Vec<Trace>,
    /// Maximum traces to keep
    max_traces: usize,
}

impl TraceAnalyzer {
    /// Create a new analyzer
    pub fn new(max_traces: usize) -> Self {
        Self {
            traces: Vec::new(),
            max_traces,
        }
    }

    /// Add a trace
    pub fn add_trace(&mut self, trace: Trace) {
        if self.traces.len() >= self.max_traces {
            self.traces.remove(0);
        }
        self.traces.push(trace);
    }

    /// Get average trace duration
    pub fn avg_duration(&self) -> f64 {
        if self.traces.is_empty() {
            return 0.0;
        }

        let total: u64 = self
            .traces
            .iter()
            .filter_map(|t| t.end_time.map(|end| end.elapsed_since(t.start_time)))
            .sum();

        total as f64 / self.traces.len() as f64
    }

    /// Get slowest traces
    pub fn slowest(&self, n: usize) -> Vec<&Trace> {
        let mut traces: Vec<_> = self.traces.iter().collect();
        traces.sort_by(|a, b| {
            let dur_a = a
                .end_time
                .map(|e| e.elapsed_since(a.start_time))
                .unwrap_or(0);
            let dur_b = b
                .end_time
                .map(|e| e.elapsed_since(b.start_time))
                .unwrap_or(0);
            dur_b.cmp(&dur_a)
        });
        traces.truncate(n);
        traces
    }

    /// Get traces with errors
    pub fn with_errors(&self) -> Vec<&Trace> {
        self.traces
            .iter()
            .filter(|t| t.spans.iter().any(|s| s.status == SpanStatus::Error))
            .collect()
    }

    /// Get spans by operation
    pub fn spans_by_operation(&self, operation: &str) -> Vec<&Span> {
        self.traces
            .iter()
            .flat_map(|t| t.spans.iter())
            .filter(|s| s.operation == operation)
            .collect()
    }

    /// Get critical path for a trace
    pub fn critical_path(&self, trace_id: u64) -> Vec<&Span> {
        let trace = match self.traces.iter().find(|t| t.id == trace_id) {
            Some(t) => t,
            None => return Vec::new(),
        };

        // Simple implementation: find longest path
        let mut path = Vec::new();
        let mut current = trace.spans.iter().find(|s| s.parent_id.is_none());

        while let Some(span) = current {
            path.push(span);

            // Find longest child
            current = trace
                .spans
                .iter()
                .filter(|s| s.parent_id == Some(span.id))
                .max_by_key(|s| {
                    s.end_time
                        .map(|e| e.elapsed_since(s.start_time))
                        .unwrap_or(0)
                });
        }

        path
    }

    /// Get trace count
    pub fn count(&self) -> usize {
        self.traces.len()
    }

    /// Clear traces
    pub fn clear(&mut self) {
        self.traces.clear();
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_creation() {
        let config = TracerConfig::default();
        let mut tracer = Tracer::new(config);

        let domain = DomainId::new(1);
        let ctx = tracer.start_trace("test_operation", domain);

        assert!(ctx.trace_id > 0);
        assert!(ctx.span_id > 0);
    }

    #[test]
    fn test_child_span() {
        let config = TracerConfig::default();
        let mut tracer = Tracer::new(config);

        let domain = DomainId::new(1);
        let parent = tracer.start_trace("parent", domain);
        let child = tracer.start_span(&parent, "child", domain);

        assert_eq!(child.trace_id, parent.trace_id);
        assert_ne!(child.span_id, parent.span_id);
    }

    #[test]
    fn test_complete_trace() {
        let config = TracerConfig::default();
        let mut tracer = Tracer::new(config);

        let domain = DomainId::new(1);
        let ctx = tracer.start_trace("test", domain);

        // Add tag and log
        tracer.tag(ctx.span_id, "key", "value");
        tracer.log(ctx.span_id, LogLevel::Info, "Test message");

        // Finish span
        tracer.finish_span_ok(ctx.span_id);

        // Complete trace
        let trace = tracer.complete_trace(ctx.trace_id);
        assert!(trace.is_some());

        let trace = trace.unwrap();
        assert_eq!(trace.spans.len(), 1);
        assert!(trace.complete);
    }

    #[test]
    fn test_baggage_propagation() {
        let config = TracerConfig {
            enable_baggage: true,
            ..Default::default()
        };
        let mut tracer = Tracer::new(config);

        let domain = DomainId::new(1);
        let mut parent = tracer.start_trace("parent", domain);

        // Set baggage
        tracer.set_baggage(&mut parent, "user_id", "123");

        // Create child - should inherit baggage
        let child = tracer.start_span(&parent, "child", domain);
        assert_eq!(child.baggage.get("user_id"), Some(&"123".into()));
    }

    #[test]
    fn test_analyzer() {
        let config = TracerConfig::default();
        let mut tracer = Tracer::new(config);
        let mut analyzer = TraceAnalyzer::new(100);

        let domain = DomainId::new(1);

        // Create and complete a trace
        let ctx = tracer.start_trace("test", domain);
        tracer.finish_span_ok(ctx.span_id);

        if let Some(trace) = tracer.complete_trace(ctx.trace_id) {
            analyzer.add_trace(trace);
        }

        assert_eq!(analyzer.count(), 1);
    }
}
