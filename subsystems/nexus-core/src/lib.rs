//! # NEXUS Core
//!
//! Core types, traits, and the main NEXUS orchestrator.
//!
//! This crate provides the fundamental building blocks for the NEXUS
//! cognitive kernel subsystem, including:
//!
//! - Component identifiers and lifecycle management
//! - Decision types and outcomes
//! - NEXUS operational levels and states
//! - Timestamp utilities

#![no_std]
#![allow(dead_code)]

extern crate alloc;

mod component;
mod decision;
mod level;
mod state;
mod timestamp;

// Re-export everything
pub use component::ComponentId;
pub use decision::{DecisionKind, DecisionOutcome, NexusDecision};
pub use level::NexusLevel;
pub use state::NexusState;
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
        assert!(!NexusState::Failed.is_operational());
    }
}
