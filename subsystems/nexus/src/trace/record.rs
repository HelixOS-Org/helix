//! Trace record for ring buffer

#![allow(dead_code)]

use super::span::Span;
use crate::core::NexusTimestamp;

// ============================================================================
// TRACE RECORD
// ============================================================================

/// A compact trace record for the ring buffer
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TraceRecord {
    /// Timestamp (cycles)
    pub timestamp: u64,
    /// Trace ID
    pub trace_id: u64,
    /// Span ID
    pub span_id: u64,
    /// Parent span ID (0 = none)
    pub parent_id: u64,
    /// Component ID
    pub component_id: u64,
    /// Event type
    pub event_type: u8,
    /// Level
    pub level: u8,
    /// Flags
    pub flags: u16,
    /// Payload (name hash or value)
    pub payload: u32,
}

impl TraceRecord {
    /// Size in bytes
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Event type: span start
    pub const EVENT_SPAN_START: u8 = 0;
    /// Event type: span end
    pub const EVENT_SPAN_END: u8 = 1;
    /// Event type: event
    pub const EVENT_EVENT: u8 = 2;
    /// Event type: log
    pub const EVENT_LOG: u8 = 3;

    /// Create a span start record
    pub fn span_start(span: &Span, name_hash: u32) -> Self {
        Self {
            timestamp: span.start.ticks(),
            trace_id: span.trace_id.0,
            span_id: span.id.0,
            parent_id: span.parent.map(|p| p.0).unwrap_or(0),
            component_id: span.component.map(|c| c.raw()).unwrap_or(0),
            event_type: Self::EVENT_SPAN_START,
            level: span.level as u8,
            flags: 0,
            payload: name_hash,
        }
    }

    /// Create a span end record
    pub fn span_end(span: &Span) -> Self {
        Self {
            timestamp: span.end.unwrap_or_else(NexusTimestamp::now).ticks(),
            trace_id: span.trace_id.0,
            span_id: span.id.0,
            parent_id: span.parent.map(|p| p.0).unwrap_or(0),
            component_id: span.component.map(|c| c.raw()).unwrap_or(0),
            event_type: Self::EVENT_SPAN_END,
            level: span.level as u8,
            flags: 0,
            payload: (span.duration() / 1000) as u32, // Duration in µs
        }
    }
}
