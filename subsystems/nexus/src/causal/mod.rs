//! # Causal Graph Construction
//!
//! Build causal dependency graphs to understand the chain of events.
//!
//! ## Key Features
//!
//! - **Happens-Before Tracking**: Track causal relationships
//! - **Critical Path Analysis**: Find the critical path in execution
//! - **Root Cause Detection**: Trace errors back to root cause
//! - **Visualizable Output**: Generate graphs for debugging
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//! - `node`: Causal node definitions
//! - `edge`: Causal edge definitions
//! - `graph`: Main causal graph implementation
//! - `tracker`: Runtime causality tracking

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

// Submodules
pub mod node;
pub mod edge;
pub mod graph;
pub mod tracker;

// Re-export node types
pub use node::{CausalNode, CausalNodeType};

// Re-export edge types
pub use edge::{CausalEdge, CausalEdgeType};

// Re-export graph
pub use graph::CausalGraph;

// Re-export tracker
pub use tracker::CausalTracker;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::ComponentId;

    #[test]
    fn test_causal_node() {
        let node = CausalNode::new(CausalNodeType::Event, "test")
            .with_component(ComponentId::MEMORY)
            .with_metadata("key", "value");

        assert_eq!(node.name, "test");
        assert_eq!(node.component, Some(ComponentId::MEMORY));
        assert!(node.metadata.contains_key("key"));
    }

    #[test]
    fn test_causal_graph() {
        let mut graph = CausalGraph::new();

        let n1 = graph.add_node(CausalNode::new(CausalNodeType::Event, "start"));
        let n2 = graph.add_node(CausalNode::new(CausalNodeType::Event, "middle"));
        let n3 = graph.add_node(CausalNode::new(CausalNodeType::Event, "end"));

        graph.link(n1, n2, CausalEdgeType::Sequential);
        graph.link(n2, n3, CausalEdgeType::Sequential);

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
        assert_eq!(graph.roots(), &[n1]);
    }

    #[test]
    fn test_path_to() {
        let mut graph = CausalGraph::new();

        let n1 = graph.add_node(CausalNode::new(CausalNodeType::Event, "a"));
        let n2 = graph.add_node(CausalNode::new(CausalNodeType::Event, "b"));
        let n3 = graph.add_node(CausalNode::new(CausalNodeType::Event, "c"));

        graph.link(n1, n2, CausalEdgeType::Sequential);
        graph.link(n2, n3, CausalEdgeType::Sequential);

        let path = graph.path_to(n3);
        assert_eq!(path, vec![n1, n2, n3]);
    }

    #[test]
    fn test_critical_path() {
        let mut graph = CausalGraph::new();

        let n1 = graph.add_node(CausalNode::new(CausalNodeType::Event, "start"));
        let n2 = graph.add_node(CausalNode::new(CausalNodeType::Event, "branch1"));
        let n3 = graph.add_node(CausalNode::new(CausalNodeType::Event, "branch2"));
        let n4 = graph.add_node(CausalNode::new(CausalNodeType::Event, "end"));

        // Branch 1 has higher weight
        graph.add_edge(CausalEdge::new(n1, n2, CausalEdgeType::Fork).with_weight(10.0));
        graph.add_edge(CausalEdge::new(n2, n4, CausalEdgeType::Sequential).with_weight(10.0));

        // Branch 2 has lower weight
        graph.add_edge(CausalEdge::new(n1, n3, CausalEdgeType::Fork).with_weight(1.0));
        graph.add_edge(CausalEdge::new(n3, n4, CausalEdgeType::Sequential).with_weight(1.0));

        let critical = graph.critical_path();
        assert!(critical.contains(&n2)); // Should go through branch 1
    }

    #[test]
    fn test_causal_tracker() {
        let mut tracker = CausalTracker::default();

        let _e1 = tracker.record_event(ComponentId::SCHEDULER, CausalNodeType::Event, "start");
        let _e2 = tracker.record_event(ComponentId::SCHEDULER, CausalNodeType::Event, "process");
        let e3 = tracker.record_error(ComponentId::SCHEDULER, "error occurred");

        assert_eq!(tracker.graph().node_count(), 3);

        let root = tracker.find_root_cause(e3);
        assert!(root.is_some());
    }
}
