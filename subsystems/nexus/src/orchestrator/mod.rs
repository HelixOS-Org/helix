//! NEXUS Orchestrator — Central Intelligence Coordinator
//!
//! The Orchestrator is the brain of NEXUS, coordinating all intelligence modules
//! to provide unified decision-making, resource allocation, and system-wide
//! optimization. It implements the core GENESIS capabilities:
//!
//! - **Unified Analysis**: Aggregates insights from all subsystems
//! - **Decision Fusion**: Combines multiple intelligence sources
//! - **Priority Management**: Orchestrates competing demands
//! - **Adaptive Response**: Real-time system-wide adjustments
//! - **Learning Integration**: Cross-module knowledge sharing

// Submodules
mod types;
mod decision;
mod event;
mod policy;
mod manager;
mod genesis;
mod intelligence;

// Re-export core types
pub use types::{
    SubsystemId, subsystems, DecisionId, EventId,
    HealthLevel, SubsystemPriority,
    SubsystemState, SubsystemMetrics,
};

// Re-export decision types
pub use decision::{
    DecisionType, DecisionUrgency, DecisionStatus,
    DecisionAction, Decision,
};

// Re-export event types
pub use event::{OrchestratorEventType, OrchestratorEvent};

// Re-export policy types
pub use policy::{PolicyType, SystemPolicy};

// Re-export manager
pub use manager::OrchestratorManager;

// Re-export genesis
pub use genesis::GenesisSummary;

// Re-export intelligence
pub use intelligence::{
    OrchestratorAnalysis, SubsystemStatus,
    OrchestratorIssue, OrchestratorIssueType,
    OrchestratorRecommendation, OrchestratorAction,
    OrchestratorIntelligence,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_level() {
        assert_eq!(HealthLevel::from_score(5), HealthLevel::Critical);
        assert_eq!(HealthLevel::from_score(30), HealthLevel::Degraded);
        assert_eq!(HealthLevel::from_score(55), HealthLevel::Warning);
        assert_eq!(HealthLevel::from_score(75), HealthLevel::Healthy);
        assert_eq!(HealthLevel::from_score(95), HealthLevel::Optimal);
    }

    #[test]
    fn test_subsystem_state() {
        let state = SubsystemState::new(SubsystemId::new(1), String::from("test"));
        assert!(state.is_enabled());
        assert_eq!(state.health_score(), 75);

        state.set_health_score(90);
        assert_eq!(state.health_score(), 90);
    }

    #[test]
    fn test_decision() {
        let decision = Decision::new(
            DecisionId::new(1),
            DecisionType::PerformanceOptimization,
            SubsystemId::new(1),
            String::from("Optimize memory"),
        )
        .with_confidence(85)
        .with_urgency(DecisionUrgency::Urgent);

        assert_eq!(decision.confidence, 85);
        assert!(decision.is_high_priority());
    }

    #[test]
    fn test_orchestrator_manager() {
        let mut manager = OrchestratorManager::new();

        let state = SubsystemState::new(SubsystemId::new(1), String::from("memory"));
        manager.register_subsystem(state);

        assert_eq!(manager.subsystem_count(), 1);

        let id = manager.create_decision(
            DecisionType::ResourceAllocation,
            SubsystemId::new(1),
            String::from("Allocate more memory"),
            0,
        );

        assert!(manager.get_pending_decision(id).is_some());
        assert!(manager.approve_decision(id));
    }

    #[test]
    fn test_orchestrator_intelligence() {
        let mut intel = OrchestratorIntelligence::new();
        intel.initialize();

        assert_eq!(intel.manager().subsystem_count(), 12);

        intel.report_health(subsystems::MEMORY, 90, 0);

        let analysis = intel.analyze();
        assert!(analysis.health_score >= 80.0);
    }

    #[test]
    fn test_genesis_summary() {
        let summary = GenesisSummary::current();
        assert_eq!(summary.completion_pct, 100.0);
        assert!(!summary.capabilities.is_empty());
    }
}
