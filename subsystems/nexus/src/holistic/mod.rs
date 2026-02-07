//! # Holistic Optimization — System-Wide Optimization Engine
//!
//! Milestone 4.4: Integrates information from all subsystems to perform
//! global optimizations that no single component could achieve alone.
//! Combines syscall intelligence (bridge), application understanding (apps),
//! and cooperation feedback (coop) into unified optimization decisions.

extern crate alloc;

pub mod balance;
pub mod global;
pub mod orchestrate;
pub mod policy;
pub mod predict;

pub use balance::*;
pub use global::*;
pub use orchestrate::*;
pub use policy::*;
pub use predict::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_snapshot() {
        let snap = SystemSnapshot::new(4, 8 * 1024 * 1024 * 1024);
        assert_eq!(snap.cpu_cores, 4);
        assert!(snap.total_memory > 0);
    }

    #[test]
    fn test_optimization_goal() {
        let goal = OptimizationGoal::Throughput;
        assert_ne!(goal, OptimizationGoal::Latency);
    }

    #[test]
    fn test_policy_rule() {
        let rule = PolicyRule::new(
            PolicyCondition::CpuAbove(0.9),
            PolicyAction::ThrottleLowPriority,
            5,
        );
        assert_eq!(rule.priority, 5);
    }

    #[test]
    fn test_orchestrator() {
        let orch = Orchestrator::new();
        assert_eq!(orch.pending_actions(), 0);
    }

    #[test]
    fn test_resource_balancer() {
        let balancer = ResourceBalancer::new(4, 8 * 1024 * 1024 * 1024);
        let (cpu, mem) = balancer.available_resources();
        assert!(cpu > 0.0);
        assert!(mem > 0);
    }
}
