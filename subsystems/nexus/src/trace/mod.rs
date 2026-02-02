//! # Ultra-Low Overhead Tracing
//!
//! High-performance tracing with <1% overhead even under heavy load.
//!
//! ## Key Innovations
//!
//! - **Lock-Free Ring Buffer**: No contention on trace writes
//! - **Binary Format**: Minimal serialization overhead
//! - **Sampling**: Adaptive sampling to control overhead
//! - **Causal Links**: Track causality between events
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Core types (TraceId, SpanId, TraceLevel)
//! - `span`: Span definitions and values
//! - `record`: Binary trace records
//! - `buffer`: Lock-free ring buffer
//! - `tracer`: Main tracer implementation
//! - `guard`: RAII span guards

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod buffer;
pub mod guard;
pub mod record;
pub mod span;
pub mod tracer;
pub mod types;

// Re-export types
// Re-export buffer
pub use buffer::TraceRingBuffer;
// Re-export guard
pub use guard::SpanGuard;
// Re-export record
pub use record::TraceRecord;
// Re-export span types
pub use span::{Span, SpanEvent, SpanValue};
// Re-export tracer
pub use tracer::{Tracer, TracerConfig, TracerStats};
pub use types::{SpanId, TraceId, TraceLevel};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id() {
        let id1 = TraceId::new();
        let id2 = TraceId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_span() {
        let span = Span::root("test")
            .with_level(TraceLevel::Debug)
            .with_attribute("key", "value");

        assert_eq!(span.name, "test");
        assert_eq!(span.level, TraceLevel::Debug);
        assert_eq!(span.attributes.len(), 1);
    }

    #[test]
    fn test_ring_buffer() {
        let mut buffer = TraceRingBuffer::new(10);

        assert!(buffer.is_empty());

        // Write some records
        for i in 0..5 {
            let record = TraceRecord {
                timestamp: i,
                trace_id: 1,
                span_id: i,
                parent_id: 0,
                component_id: 0,
                event_type: 0,
                level: 0,
                flags: 0,
                payload: 0,
            };
            buffer.write(record);
        }

        assert_eq!(buffer.len(), 5);

        // Read them back
        for _ in 0..5 {
            assert!(buffer.read().is_some());
        }

        assert!(buffer.is_empty());
    }

    #[test]
    fn test_tracer() {
        let mut tracer = Tracer::default();

        let span = Span::root("test_span");
        tracer.start_span(&span);

        let mut span = span;
        span.end();
        tracer.end_span(&span);

        let records = tracer.drain();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_child_span() {
        let parent = Span::root("parent");
        let child = parent.child("child");

        assert_eq!(child.trace_id, parent.trace_id);
        assert_eq!(child.parent, Some(parent.id));
    }
}
