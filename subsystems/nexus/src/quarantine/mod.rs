//! # Component Quarantine System
//!
//! Isolate failing components to prevent cascade failures.
//!
//! ## Key Features
//!
//! - **Isolation Levels**: From partial to complete isolation
//! - **Automatic Release**: Components can be released when healthy
//! - **Dependency Tracking**: Quarantine dependent components
//! - **Fallback Activation**: Switch to backup implementations
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `level`: Quarantine levels and reasons
//! - `entry`: Quarantine entry
//! - `system`: Main quarantine system

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod entry;
pub mod level;
pub mod system;

// Re-export level types
pub use level::{QuarantineLevel, QuarantineReason};

// Re-export entry
pub use entry::QuarantineEntry;

// Re-export system
pub use system::{QuarantineHistoryEntry, QuarantineStats, QuarantineSystem};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComponentId;

    #[test]
    fn test_quarantine_level() {
        assert!(QuarantineLevel::Monitored.can_process());
        assert!(QuarantineLevel::Monitored.can_communicate());
        assert!(!QuarantineLevel::Suspended.can_process());
        assert!(!QuarantineLevel::Isolated.can_communicate());
    }

    #[test]
    fn test_quarantine_reason() {
        let reason = QuarantineReason::RepeatedFailures { count: 5 };
        assert_eq!(reason.recommended_level(), QuarantineLevel::Suspended);

        let reason = QuarantineReason::LowHealth {
            health: 0.4,
            threshold: 0.5,
        };
        assert_eq!(reason.recommended_level(), QuarantineLevel::Degraded);
    }

    #[test]
    fn test_quarantine_entry() {
        let entry = QuarantineEntry::new(
            ComponentId::MEMORY,
            QuarantineReason::RepeatedFailures { count: 3 },
        );

        assert_eq!(entry.component, ComponentId::MEMORY);
        assert_eq!(entry.level, QuarantineLevel::Isolated);
        assert!(entry.duration() >= 0);
    }

    #[test]
    fn test_quarantine_system() {
        let mut system = QuarantineSystem::new();

        let entry = QuarantineEntry::new(
            ComponentId::SCHEDULER,
            QuarantineReason::Manual {
                reason: "Test".into(),
            },
        );

        system.quarantine(entry);

        assert!(system.is_quarantined(ComponentId::SCHEDULER));
        assert_eq!(
            system.get_level(ComponentId::SCHEDULER),
            Some(QuarantineLevel::Isolated)
        );

        system.release(ComponentId::SCHEDULER);
        assert!(!system.is_quarantined(ComponentId::SCHEDULER));
    }

    #[test]
    fn test_escalation() {
        let mut system = QuarantineSystem::new();

        let entry = QuarantineEntry::new(
            ComponentId::MEMORY,
            QuarantineReason::LowHealth {
                health: 0.4,
                threshold: 0.5,
            },
        )
        .with_level(QuarantineLevel::Monitored);

        system.quarantine(entry);

        system.escalate(ComponentId::MEMORY);
        assert_eq!(
            system.get_level(ComponentId::MEMORY),
            Some(QuarantineLevel::Degraded)
        );

        system.escalate(ComponentId::MEMORY);
        assert_eq!(
            system.get_level(ComponentId::MEMORY),
            Some(QuarantineLevel::Restricted)
        );
    }

    #[test]
    fn test_stats() {
        let mut system = QuarantineSystem::new();

        system.quarantine(
            QuarantineEntry::new(
                ComponentId::MEMORY,
                QuarantineReason::Manual {
                    reason: "Test".into(),
                },
            )
            .with_level(QuarantineLevel::Isolated),
        );

        system.quarantine(
            QuarantineEntry::new(
                ComponentId::SCHEDULER,
                QuarantineReason::Manual {
                    reason: "Test".into(),
                },
            )
            .with_level(QuarantineLevel::Suspended),
        );

        let stats = system.stats();
        assert_eq!(stats.total_quarantined, 2);
        assert_eq!(stats.isolated, 1);
        assert_eq!(stats.suspended, 1);
    }
}
