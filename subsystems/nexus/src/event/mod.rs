//! # NEXUS Event System
//!
//! High-performance event system for NEXUS with priority queuing and handlers.
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Core types (EventId, EventPriority, AnomalyEventKind)
//! - `kind`: Event kind definitions
//! - `event`: NexusEvent struct
//! - `handler`: Event handler trait and subscriptions
//! - `queue`: Priority event queue
//! - `bus`: Event bus for distribution

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod bus;
pub mod event;
pub mod handler;
pub mod kind;
pub mod queue;
pub mod types;

// Re-export types
// Re-export bus
pub use bus::{EventBus, EventBusStats};
// Re-export event
pub use event::NexusEvent;
// Re-export handler
pub use handler::{EventHandler, EventHandlerResult, EventSubscription};
// Re-export kind
pub use kind::NexusEventKind;
// Re-export queue
pub use queue::EventQueue;
pub use types::{AnomalyEventKind, EventId, EventPriority};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComponentId;

    #[test]
    fn test_event_id_uniqueness() {
        let id1 = EventId::new();
        let id2 = EventId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_event_priority_ordering() {
        assert!(EventPriority::Background < EventPriority::Low);
        assert!(EventPriority::Low < EventPriority::Normal);
        assert!(EventPriority::Normal < EventPriority::High);
        assert!(EventPriority::High < EventPriority::Critical);
        assert!(EventPriority::Critical < EventPriority::Emergency);
    }

    #[test]
    fn test_event_queue() {
        let mut queue = EventQueue::new(100);

        queue.push(NexusEvent::new(NexusEventKind::Heartbeat));
        queue.push(NexusEvent::with_priority(
            NexusEventKind::SystemShutdown,
            EventPriority::Critical,
        ));

        // Should get critical event first
        let event = queue.pop().unwrap();
        assert_eq!(event.priority, EventPriority::Critical);
    }

    #[test]
    fn test_event_default_priority() {
        let crash_event = NexusEvent::new(NexusEventKind::CrashPredicted {
            confidence: 0.95,
            time_to_crash_ms: 5000,
            component: ComponentId::MEMORY,
        });
        assert_eq!(crash_event.priority, EventPriority::Emergency);

        let heartbeat = NexusEvent::new(NexusEventKind::Heartbeat);
        assert_eq!(heartbeat.priority, EventPriority::Low);
    }

    #[test]
    fn test_event_predicates() {
        let prediction = NexusEvent::new(NexusEventKind::CrashPredicted {
            confidence: 0.8,
            time_to_crash_ms: 10000,
            component: ComponentId::SCHEDULER,
        });
        assert!(prediction.is_prediction());
        assert!(prediction.is_critical());

        let healing = NexusEvent::new(NexusEventKind::HealingStarted {
            component: ComponentId::MEMORY,
            strategy: "soft_reset".into(),
        });
        assert!(healing.is_healing());
    }
}
