//! # State Reconstruction
//!
//! Reconstruct component state from logs and events.
//!
//! ## Key Features
//!
//! - **Event Sourcing**: Reconstruct state from events
//! - **Log Replay**: Replay operation logs
//! - **Consistency Verification**: Verify reconstructed state
//! - **Partial Reconstruction**: Reconstruct subset of state

#![allow(dead_code)]

extern crate alloc;

mod engine;
mod event;
mod log;
mod reconstructor;
mod snapshot;

// Re-export event types
pub use event::{StateEvent, StateEventType};

// Re-export log
pub use log::StateLog;

// Re-export snapshot
pub use snapshot::StateSnapshot;

// Re-export reconstructor
pub use reconstructor::StateReconstructor;

// Re-export engine
pub use engine::{ReplayEngine, ReplayResult};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComponentId;

    #[test]
    fn test_state_event() {
        let mut event = StateEvent::new(ComponentId::MEMORY, StateEventType::Create, "test_key")
            .with_new_value(vec![1, 2, 3]);

        event.calculate_checksum();
        assert!(event.verify_checksum());
    }

    #[test]
    fn test_state_log() {
        let mut log = StateLog::new(100);

        for i in 0..10 {
            let event = StateEvent::new(
                ComponentId::MEMORY,
                StateEventType::Update,
                alloc::format!("key_{}", i),
            );
            log.append(event);
        }

        assert_eq!(log.len(), 10);
    }

    #[test]
    fn test_reconstruction() {
        let mut reconstructor = StateReconstructor::new();

        // Record some changes
        reconstructor.record_change(ComponentId::MEMORY, "key1", None, Some(vec![1, 2, 3]));
        reconstructor.record_change(ComponentId::MEMORY, "key2", None, Some(vec![4, 5, 6]));

        // Reconstruct
        let state = reconstructor.reconstruct_current(ComponentId::MEMORY);
        assert!(state.is_ok());

        let state = state.unwrap();
        assert_eq!(state.len(), 2);
        assert_eq!(state.get("key1"), Some(&vec![1, 2, 3]));
    }

    #[test]
    fn test_snapshot() {
        let mut snapshot = StateSnapshot::new(ComponentId::MEMORY);
        snapshot.set("key1", vec![1, 2, 3]);
        snapshot.set("key2", vec![4, 5, 6]);
        snapshot.calculate_checksum();

        assert!(snapshot.verify_checksum());
    }
}
