//! Span definitions

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::types::{SpanId, TraceId, TraceLevel};
use crate::core::{ComponentId, NexusTimestamp};

// ============================================================================
// SPAN VALUE
// ============================================================================

/// A value in a span attribute
#[derive(Debug, Clone)]
pub enum SpanValue {
    /// Integer value
    Int(i64),
    /// Unsigned integer
    Uint(u64),
    /// Float value
    Float(f64),
    /// Boolean value
    Bool(bool),
    /// String value
    String(String),
}

impl From<i64> for SpanValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<u64> for SpanValue {
    fn from(v: u64) -> Self {
        Self::Uint(v)
    }
}

impl From<f64> for SpanValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<bool> for SpanValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

impl From<&str> for SpanValue {
    fn from(v: &str) -> Self {
        Self::String(v.into())
    }
}

impl From<String> for SpanValue {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}

// ============================================================================
// SPAN EVENT
// ============================================================================

/// An event within a span
#[derive(Debug, Clone)]
pub struct SpanEvent {
    /// Event name
    pub name: &'static str,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Attributes
    pub attributes: Vec<(String, SpanValue)>,
}

// ============================================================================
// SPAN
// ============================================================================

/// A trace span
#[derive(Debug, Clone)]
pub struct Span {
    /// Span ID
    pub id: SpanId,
    /// Parent span ID
    pub parent: Option<SpanId>,
    /// Trace ID
    pub trace_id: TraceId,
    /// Span name
    pub name: &'static str,
    /// Start timestamp
    pub start: NexusTimestamp,
    /// End timestamp
    pub end: Option<NexusTimestamp>,
    /// Component
    pub component: Option<ComponentId>,
    /// Level
    pub level: TraceLevel,
    /// Attributes
    pub attributes: Vec<(String, SpanValue)>,
    /// Events within this span
    pub events: Vec<SpanEvent>,
}

impl Span {
    /// Create a new span
    pub fn new(name: &'static str, trace_id: TraceId, parent: Option<SpanId>) -> Self {
        Self {
            id: SpanId::new(),
            parent,
            trace_id,
            name,
            start: NexusTimestamp::now(),
            end: None,
            component: None,
            level: TraceLevel::Info,
            attributes: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Create a root span
    #[inline(always)]
    pub fn root(name: &'static str) -> Self {
        Self::new(name, TraceId::new(), None)
    }

    /// Create a child span
    #[inline(always)]
    pub fn child(&self, name: &'static str) -> Self {
        Self::new(name, self.trace_id, Some(self.id))
    }

    /// Set component
    #[inline(always)]
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.component = Some(component);
        self
    }

    /// Set level
    #[inline(always)]
    pub fn with_level(mut self, level: TraceLevel) -> Self {
        self.level = level;
        self
    }

    /// Add attribute
    #[inline(always)]
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<SpanValue>) -> Self {
        self.attributes.push((key.into(), value.into()));
        self
    }

    /// Add an event
    #[inline]
    pub fn add_event(&mut self, name: &'static str) {
        self.events.push(SpanEvent {
            name,
            timestamp: NexusTimestamp::now(),
            attributes: Vec::new(),
        });
    }

    /// End the span
    #[inline]
    pub fn end(&mut self) {
        if self.end.is_none() {
            self.end = Some(NexusTimestamp::now());
        }
    }

    /// Get duration in cycles
    #[inline]
    pub fn duration(&self) -> u64 {
        match self.end {
            Some(end) => end.duration_since(self.start),
            None => NexusTimestamp::now().duration_since(self.start),
        }
    }

    /// Is span ended?
    #[inline(always)]
    pub fn is_ended(&self) -> bool {
        self.end.is_some()
    }
}
