//! # NEXUS Core
//!
//! Core types, traits, and the main NEXUS orchestrator.

#![allow(dead_code)]

extern crate alloc;

mod component;
mod decision;
mod level;
mod nexus;
mod state;
mod subsystem;
mod timestamp;

// Re-export level
// Re-export component
pub use component::ComponentId;
// Re-export decision types
pub use decision::{DecisionKind, DecisionOutcome, NexusDecision};
pub use level::NexusLevel;
// Re-export main nexus struct
pub use nexus::Nexus;
// Re-export state
pub use state::NexusState;
// Re-export subsystem trait
pub use subsystem::NexusSubsystem;
// Re-export timestamp
pub use timestamp::NexusTimestamp;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nexus_level_ordering() {
        assert!(NexusLevel::Disabled < NexusLevel::Monitoring);
        assert!(NexusLevel::Monitoring < NexusLevel::Detection);
        assert!(NexusLevel::Detection < NexusLevel::Prediction);
        assert!(NexusLevel::Prediction < NexusLevel::Correction);
        assert!(NexusLevel::Correction < NexusLevel::Healing);
        assert!(NexusLevel::Healing < NexusLevel::Autonomous);
    }

    #[test]
    fn test_nexus_level_from_u8() {
        assert_eq!(NexusLevel::from_u8(0), Some(NexusLevel::Disabled));
        assert_eq!(NexusLevel::from_u8(6), Some(NexusLevel::Autonomous));
        assert_eq!(NexusLevel::from_u8(7), None);
    }

    #[test]
    fn test_nexus_state_operational() {
        assert!(!NexusState::Uninitialized.is_operational());
        assert!(NexusState::Running.is_operational());
        assert!(NexusState::Degraded.is_operational());
        assert!(NexusState::Healing.is_operational());
        assert!(!NexusState::Stopped.is_operational());
    }

    #[test]
    fn test_component_id() {
        assert_eq!(ComponentId::SCHEDULER.raw(), 1);
        assert_eq!(ComponentId::MEMORY.raw(), 2);
    }

    #[test]
    fn test_decision() {
        let decision = NexusDecision::new(DecisionKind::SoftRecover, 0.85)
            .with_reason("Memory pressure detected")
            .with_reason("Initiating soft recovery");

        assert_eq!(decision.kind, DecisionKind::SoftRecover);
        assert_eq!(decision.confidence, 0.85);
        assert_eq!(decision.reasoning.len(), 2);
    }
}
