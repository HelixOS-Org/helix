//! NEXUS Causal Reasoning Engine — COGNITION Year 2
//!
//! This module provides causal reasoning capabilities, enabling NEXUS to:
//!
//! - **Understand** why events occurred (root cause analysis)
//! - **Build** causal chains from observations
//! - **Reason** counterfactually ("what if X hadn't happened?")
//! - **Explain** complex system behaviors in human terms
//! - **Query** historical causality with CQL (Causal Query Language)
//!
//! ## Modules
//!
//! - [`types`] - Core identifiers
//! - [`events`] - Causal event types and structures
//! - [`graph`] - Causal DAG structure
//! - [`chain`] - Causal chain building
//! - [`counterfactual`] - "What if" reasoning
//! - [`explanation`] - Human-readable explanations
//! - [`cql`] - Causal Query Language
//! - [`intelligence`] - Main causal reasoning interface

#![no_std]

extern crate alloc;

pub mod types;
pub mod events;
pub mod graph;
pub mod chain;
pub mod counterfactual;
pub mod explanation;
pub mod cql;
pub mod intelligence;

// Re-export types
pub use types::{
    CausalEventId, CausalNodeId, CausalEdgeId, ChainId, QueryId,
};

// Re-export events
pub use events::{
    CausalEventType, EventSeverity, CausalEvent,
};

// Re-export graph
pub use graph::{
    CausalRelationType, CausalNode, CausalEdge, CausalGraph,
};

// Re-export chain
pub use chain::{
    CausalChain, CausalChainBuilder,
};

// Re-export counterfactual
pub use counterfactual::{
    CounterfactualScenario, CounterfactualModification,
    CounterfactualResult, CounterfactualImpact, CounterfactualEngine,
};

// Re-export explanation
pub use explanation::{
    ExplanationType, Explanation, ExplanationGenerator,
};

// Re-export cql
pub use cql::{
    CqlQuery, CqlResult, CqlEngine,
};

// Re-export intelligence
pub use intelligence::{
    CausalReasoningAnalysis, CausalReasoningIntelligence,
};

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn test_causal_event() {
        let event = CausalEvent::new(
            CausalEventId::new(1),
            CausalEventType::ErrorOccurred,
            1000000,
        )
        .with_severity(EventSeverity::Error)
        .with_description(String::from("Test error"));

        assert_eq!(event.severity, EventSeverity::Error);
        assert!(event.event_type.is_error());
    }

    #[test]
    fn test_causal_graph() {
        let mut graph = CausalGraph::new();

        let e1 = CausalEvent::new(CausalEventId::new(1), CausalEventType::MemoryAlloc, 1000);
        let e2 = CausalEvent::new(CausalEventId::new(2), CausalEventType::PageFault, 2000);
        let e3 = CausalEvent::new(CausalEventId::new(3), CausalEventType::ErrorOccurred, 3000);

        let n1 = graph.add_event(e1);
        let n2 = graph.add_event(e2);
        let n3 = graph.add_event(e3);

        graph.add_relationship(n1, n2, CausalRelationType::DirectCause, 0.9);
        graph.add_relationship(n2, n3, CausalRelationType::DirectCause, 0.8);

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);

        let roots = graph.find_root_causes(n3);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0], n1);
    }

    #[test]
    fn test_causal_chain() {
        let mut graph = CausalGraph::new();
        let builder = CausalChainBuilder::new();

        let e1 = CausalEvent::new(CausalEventId::new(1), CausalEventType::Syscall, 1000);
        let e2 = CausalEvent::new(CausalEventId::new(2), CausalEventType::LockAcquire, 2000);
        let e3 = CausalEvent::new(CausalEventId::new(3), CausalEventType::ErrorOccurred, 3000);

        let n1 = graph.add_event(e1);
        let n2 = graph.add_event(e2);
        let n3 = graph.add_event(e3);

        graph.add_relationship(n1, n2, CausalRelationType::DirectCause, 0.9);
        graph.add_relationship(n2, n3, CausalRelationType::DirectCause, 0.8);

        let chain = builder.build_chain(&graph, n1, n3);
        assert!(chain.is_some());

        let chain = chain.unwrap();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain.root_cause(), Some(n1));
        assert_eq!(chain.terminal_effect(), Some(n3));
    }

    #[test]
    fn test_counterfactual() {
        let mut graph = CausalGraph::new();
        let engine = CounterfactualEngine::new();

        let e1 = CausalEvent::new(CausalEventId::new(1), CausalEventType::ConfigChanged, 1000);
        let e2 = CausalEvent::new(CausalEventId::new(2), CausalEventType::ErrorOccurred, 2000);

        let n1 = graph.add_event(e1);
        let n2 = graph.add_event(e2);

        graph.add_relationship(n1, n2, CausalRelationType::DirectCause, 0.95);

        let scenario = engine.create_scenario(
            String::from("What if config hadn't changed?"),
            CausalEventId::new(1),
            CounterfactualModification::Remove,
        );

        let result = engine.simulate(&graph, scenario);
        assert!(!result.prevented_events.is_empty());
    }

    #[test]
    fn test_cql_engine() {
        let mut graph = CausalGraph::new();
        let engine = CqlEngine::new();

        let e1 = CausalEvent::new(CausalEventId::new(1), CausalEventType::Syscall, 1000);
        let e2 = CausalEvent::new(CausalEventId::new(2), CausalEventType::ErrorOccurred, 2000);

        let n1 = graph.add_event(e1);
        let n2 = graph.add_event(e2);

        graph.add_relationship(n1, n2, CausalRelationType::DirectCause, 0.9);

        let result = engine.execute(&graph, CqlQuery::WhyQuery { event: CausalEventId::new(2) });
        assert!(matches!(result, CqlResult::Explanation(_)));
    }

    #[test]
    fn test_causal_intelligence() {
        let mut intel = CausalReasoningIntelligence::new();

        let e1 = intel.record_event(CausalEventType::MemoryAlloc, 1000);
        let e2 = intel.record_event(CausalEventType::ResourceExhausted, 2000);

        intel.add_causality(e1, e2, CausalRelationType::DirectCause, 0.85);

        let analysis = intel.analyze();
        assert_eq!(analysis.total_events, 2);
        assert_eq!(analysis.total_relationships, 1);
    }
}
