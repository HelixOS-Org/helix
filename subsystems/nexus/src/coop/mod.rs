//! # Kernel-App Cooperation Protocol — Year 4 SYMBIOSIS (Q3 2029)
//!
//! Bidirectional communication protocol between kernel and userland
//! applications. Applications can give hints to the kernel, and the kernel
//! can send advisories back to applications.
//!
//! ## Key Innovations
//!
//! - **Bidirectional Hints**: Apps → Kernel hints, Kernel → App advisories
//! - **Resource Negotiation**: Dynamic resource contracts between kernel and apps
//! - **Cooperative Scheduling**: App and kernel collaborate on scheduling decisions
//! - **Feedback Loop**: Continuous improvement from cooperation telemetry
//!
//! ## Submodules
//!
//! - `protocol`: Core protocol definitions and message types
//! - `hints`: Bidirectional hint system (app→kernel, kernel→app)
//! - `negotiate`: Resource negotiation and contract management
//! - `feedback`: Cooperation telemetry and feedback loops

#![allow(dead_code)]

extern crate alloc;

pub mod feedback;
pub mod hints;
pub mod negotiate;
pub mod protocol;

// Re-export core types
pub use feedback::{CoopFeedback, CoopMetrics, FeedbackCollector, FeedbackType};
pub use hints::{AppHint, AppHintType, KernelAdvisory, KernelAdvisoryType, HintBus};
pub use negotiate::{
    Contract, ContractId, ContractState, NegotiationEngine, ResourceDemand, ResourceOffer,
};
pub use protocol::{
    CoopCapability, CoopMessage, CoopMessageType, CoopSession, CoopSessionState, ProtocolVersion,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hint_creation() {
        let hint = AppHint::new(42, AppHintType::ComputeIntensive { duration_ms: 5000 });
        assert_eq!(hint.pid, 42);
        assert!(matches!(hint.hint_type, AppHintType::ComputeIntensive { .. }));
    }

    #[test]
    fn test_advisory_creation() {
        let advisory = KernelAdvisory::new(
            42,
            KernelAdvisoryType::MemoryPressure {
                level: PressureLevel::Medium,
                recommended_release_bytes: 50 * 1024 * 1024,
            },
        );
        assert_eq!(advisory.pid, 42);
    }

    #[test]
    fn test_hint_bus() {
        let mut bus = HintBus::new(100);

        bus.send_hint(AppHint::new(42, AppHintType::LatencySensitive { thread_id: 1 }));
        bus.send_hint(AppHint::new(43, AppHintType::IoHeavy { expected_bytes: 1024 * 1024 }));

        let hints = bus.drain_hints();
        assert_eq!(hints.len(), 2);
    }

    #[test]
    fn test_negotiation() {
        let mut engine = NegotiationEngine::new();

        let demand = ResourceDemand {
            pid: 42,
            cpu_shares: Some(200),
            memory_bytes: Some(256 * 1024 * 1024),
            io_bandwidth_bps: None,
            net_bandwidth_bps: None,
            priority: 5,
            duration_ms: Some(60_000),
        };

        let offer = engine.evaluate_demand(&demand);
        assert!(offer.is_some());
        let offer = offer.unwrap();

        let contract = engine.accept_offer(demand, offer);
        assert_eq!(contract.state, ContractState::Active);
    }

    #[test]
    fn test_feedback_collector() {
        let mut collector = FeedbackCollector::new(1000);

        collector.record(CoopFeedback {
            pid: 42,
            feedback_type: FeedbackType::HintAccuracy { accuracy: 0.92 },
            timestamp: 1000,
        });

        let metrics = collector.compute_metrics();
        assert!(metrics.avg_hint_accuracy > 0.0);
    }

    // Needed for the advisory test
    use hints::PressureLevel;
}
