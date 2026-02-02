//! Causal tracker for runtime causality tracking

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::edge::CausalEdgeType;
use super::graph::CausalGraph;
use super::node::{CausalNode, CausalNodeType};
use crate::core::ComponentId;

// ============================================================================
// CAUSAL TRACKER
// ============================================================================

/// Tracks causality during execution
pub struct CausalTracker {
    /// The causal graph being built
    graph: CausalGraph,
    /// Current node per component
    current_node: BTreeMap<u64, u64>,
    /// Maximum nodes to keep
    max_nodes: usize,
}

impl CausalTracker {
    /// Create a new tracker
    pub fn new(max_nodes: usize) -> Self {
        Self {
            graph: CausalGraph::new(),
            current_node: BTreeMap::new(),
            max_nodes,
        }
    }

    /// Record an event
    pub fn record_event(
        &mut self,
        component: ComponentId,
        node_type: CausalNodeType,
        name: impl Into<alloc::string::String>,
    ) -> u64 {
        let node = CausalNode::new(node_type, name).with_component(component);

        let node_id = node.id;

        // Link to previous event for this component
        if let Some(&prev_id) = self.current_node.get(&component.raw()) {
            self.graph.add_node(node);
            self.graph
                .link(prev_id, node_id, CausalEdgeType::Sequential);
        } else {
            self.graph.add_node(node);
        }

        self.current_node.insert(component.raw(), node_id);

        // Garbage collect if needed
        if self.graph.node_count() > self.max_nodes {
            self.garbage_collect();
        }

        node_id
    }

    /// Record a message send
    pub fn record_send(
        &mut self,
        from: ComponentId,
        to: ComponentId,
        message: impl Into<alloc::string::String>,
    ) -> (u64, u64) {
        let send_node = CausalNode::new(CausalNodeType::Send, message.into()).with_component(from);
        let send_id = self.graph.add_node(send_node);

        // Link to previous
        if let Some(&prev) = self.current_node.get(&from.raw()) {
            self.graph.link(prev, send_id, CausalEdgeType::Sequential);
        }
        self.current_node.insert(from.raw(), send_id);

        // Create receive placeholder
        let recv_node = CausalNode::new(CausalNodeType::Receive, "receive").with_component(to);
        let recv_id = self.graph.add_node(recv_node);

        // Link send to receive
        self.graph.link(send_id, recv_id, CausalEdgeType::Message);

        // Link to receiver's previous
        if let Some(&prev) = self.current_node.get(&to.raw()) {
            self.graph.link(prev, recv_id, CausalEdgeType::Sequential);
        }
        self.current_node.insert(to.raw(), recv_id);

        (send_id, recv_id)
    }

    /// Record an error
    pub fn record_error(
        &mut self,
        component: ComponentId,
        error: impl Into<alloc::string::String>,
    ) -> u64 {
        self.record_event(component, CausalNodeType::Error, error)
    }

    /// Link two existing nodes
    pub fn link_nodes(&mut self, from: u64, to: u64, edge_type: CausalEdgeType) {
        self.graph.link(from, to, edge_type);
    }

    /// Get the causal graph
    pub fn graph(&self) -> &CausalGraph {
        &self.graph
    }

    /// Get mutable graph
    pub fn graph_mut(&mut self) -> &mut CausalGraph {
        &mut self.graph
    }

    /// Find root cause of an error
    pub fn find_root_cause(&self, error_node: u64) -> Option<u64> {
        self.graph.find_root_cause(error_node)
    }

    /// Get critical path
    pub fn critical_path(&self) -> Vec<u64> {
        self.graph.critical_path()
    }

    /// Simple garbage collection
    fn garbage_collect(&mut self) {
        // Keep only recent nodes by keeping nodes connected to errors
        // or the most recent nodes per component
        // Simplified: just clear old roots
    }
}

impl Default for CausalTracker {
    fn default() -> Self {
        Self::new(10000)
    }
}
