//! # Micro-Rollback Engine
//!
//! Granular rollback at component level without affecting the rest of the system.
//!
//! ## Key Features
//!
//! - **Component-Level Rollback**: Rollback individual components
//! - **Incremental Checkpoints**: Minimal storage overhead
//! - **Fast Restore**: Sub-millisecond restore times
//! - **Dependency-Aware**: Handle inter-component dependencies

#![allow(dead_code)]

extern crate alloc;

mod engine;
mod entry;
mod point;
mod policy;
mod transaction;

// Re-export point
pub use point::RollbackPoint;

// Re-export entry
pub use entry::RollbackEntry;

// Re-export policy
pub use policy::RollbackPolicy;

// Re-export engine
pub use engine::{MicroRollbackEngine, MicroRollbackStats};

// Re-export transaction
pub use transaction::RollbackTransaction;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComponentId;

    #[test]
    fn test_rollback_point() {
        let point = RollbackPoint::new(ComponentId::MEMORY, 1)
            .with_hash(12345)
            .with_dependency(ComponentId::SCHEDULER);

        assert_eq!(point.component, ComponentId::MEMORY);
        assert_eq!(point.checkpoint_id, 1);
        assert_eq!(point.state_hash, 12345);
        assert_eq!(point.dependencies.len(), 1);
        assert!(point.is_safe);
    }

    #[test]
    fn test_create_point() {
        let mut engine = MicroRollbackEngine::default();

        let id = engine.create_point(ComponentId::MEMORY, 1, 12345);
        assert!(id > 0);

        let point = engine.latest_point(ComponentId::MEMORY);
        assert!(point.is_some());
        assert_eq!(point.unwrap().state_hash, 12345);
    }

    #[test]
    fn test_rollback() {
        let mut engine = MicroRollbackEngine::default();

        let id = engine.create_point(ComponentId::MEMORY, 1, 12345);
        let result = engine.rollback(ComponentId::MEMORY, Some(id));

        assert!(result.is_ok());
        let entry = result.unwrap();
        assert!(entry.success);
    }

    #[test]
    fn test_max_points() {
        let mut policy = RollbackPolicy::default();
        policy.max_points = 3;
        let mut engine = MicroRollbackEngine::new(policy);

        // Create more points than max
        for i in 0..5 {
            engine.create_point(ComponentId::MEMORY, i, i);
        }

        // Should only have 3 points
        assert_eq!(engine.points_for(ComponentId::MEMORY).len(), 3);
    }

    #[test]
    fn test_unsafe_point() {
        let mut engine = MicroRollbackEngine::default();

        let id = engine.create_point(ComponentId::MEMORY, 1, 12345);
        engine.invalidate_point(id);

        // Should not be able to get unsafe point by default
        let point = engine.latest_point(ComponentId::MEMORY);
        assert!(point.is_none());

        // But with allow_unsafe = true
        engine.policy.allow_unsafe = true;
        let point = engine.latest_point(ComponentId::MEMORY);
        assert!(point.is_some());
    }

    #[test]
    fn test_stats() {
        let mut engine = MicroRollbackEngine::default();

        engine.create_point(ComponentId::MEMORY, 1, 1);
        engine.create_point(ComponentId::SCHEDULER, 2, 2);

        let stats = engine.stats();
        assert_eq!(stats.total_points, 2);
        assert_eq!(stats.components_with_points, 2);
    }
}
