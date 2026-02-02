//! Tracepoint Intelligence Module
//!
//! This module provides AI-powered tracepoint analysis and optimization for the NEXUS subsystem.
//! It includes tracepoint registration, event filtering, data collection, performance analysis,
//! and intelligent trace optimization based on system behavior patterns.
//!
//! ## Modules
//!
//! - [`types`] - Core tracepoint types
//! - [`definition`] - Tracepoint and field definitions
//! - [`event`] - Event data structures
//! - [`filter`] - Event filtering engine
//! - [`buffer`] - Ring buffer for events
//! - [`probe`] - Probe management
//! - [`analyzer`] - Performance analysis and tracepoint management
//! - [`intelligence`] - Main intelligence engine

#![no_std]

extern crate alloc;
use alloc::vec;

pub mod types;
pub mod definition;
pub mod event;
pub mod filter;
pub mod buffer;
pub mod probe;
pub mod analyzer;
pub mod intelligence;

// Re-export types
pub use types::{
    TracepointId, ProbeId, EventId, TracepointSubsystem, TracepointState, FieldType,
};

// Re-export definition
pub use definition::{EventField, TracepointDef};

// Re-export event
pub use event::EventData;

// Re-export filter
pub use filter::{FilterOp, FilterPredicate, FilterExpr, EventFilter};

// Re-export buffer
pub use buffer::EventRingBuffer;

// Re-export probe
pub use probe::{ProbeType, ProbeInfo, ProbeManager};

// Re-export analyzer
pub use analyzer::{TraceSample, TraceStats, PerformanceAnalyzer, TracepointManager};

// Re-export intelligence
pub use intelligence::{
    TracepointAnalysis, TraceIssue, TraceIssueType, TraceRecommendation, TraceAction,
    TracepointIntelligence,
};

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn test_tracepoint_subsystem() {
        assert_eq!(TracepointSubsystem::Sched.name(), "sched");
        assert_eq!(TracepointSubsystem::all().len(), 15);
    }

    #[test]
    fn test_event_data() {
        let mut event = EventData::new(EventId::new(1), TracepointId::new(1), 1000, 0, 1, 1);
        event.data = alloc::vec![0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];

        assert_eq!(event.read_u8(0), Some(0x12));
        assert_eq!(event.read_u16(0), Some(0x3412));
        assert_eq!(event.read_u32(0), Some(0x78563412));
    }

    #[test]
    fn test_filter_predicate() {
        let pred = FilterPredicate::numeric(String::from("pid"), 0, 4, FilterOp::Eq, 1234);

        let mut event = EventData::new(EventId::new(1), TracepointId::new(1), 1000, 0, 1, 1);
        event.data = alloc::vec![0xD2, 0x04, 0x00, 0x00]; // 1234 in little-endian

        assert!(pred.evaluate(&event));
    }

    #[test]
    fn test_ring_buffer() {
        let mut buffer = EventRingBuffer::new(3);

        for i in 0..5 {
            let event = EventData::new(EventId::new(i), TracepointId::new(1), i * 100, 0, 1, 1);
            buffer.write(event);
        }

        assert_eq!(buffer.events_written(), 5);
        assert!(buffer.events_lost() > 0);
    }

    #[test]
    fn test_tracepoint_intelligence() {
        let mut intel = TracepointIntelligence::new(0, 1000);

        let tp_id = intel.register_tracepoint(
            String::from("sched:sched_switch"),
            TracepointSubsystem::Sched,
            1000,
        );

        assert!(intel.enable_tracepoint(tp_id));

        let event = EventData::new(EventId::new(1), tp_id, 2000, 0, 1, 1);
        intel.record_event(event, 500);

        let analysis = intel.analyze(tp_id);
        assert!(analysis.is_some());
    }
}
