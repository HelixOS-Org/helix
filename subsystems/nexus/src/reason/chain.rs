//! Causal Chain Builder
//!
//! This module provides causal chain construction from cause to effect.

#![allow(clippy::excessive_nesting)]
#![allow(clippy::map_entry)]

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{CausalEdgeId, CausalGraph, CausalNodeId, ChainId};

/// Causal chain (path from cause to effect)
#[derive(Debug, Clone)]
pub struct CausalChain {
    /// Chain ID
    pub id: ChainId,
    /// Nodes in the chain (ordered cause → effect)
    pub nodes: Vec<CausalNodeId>,
    /// Edges in the chain
    pub edges: Vec<CausalEdgeId>,
    /// Total confidence (product of edge confidences)
    pub total_confidence: f32,
    /// Total time span
    pub time_span: u64,
    /// Chain strength
    pub strength: f32,
}

impl CausalChain {
    /// Create new chain
    pub fn new(id: ChainId) -> Self {
        Self {
            id,
            nodes: Vec::new(),
            edges: Vec::new(),
            total_confidence: 1.0,
            time_span: 0,
            strength: 0.0,
        }
    }

    /// Add node
    pub fn add_node(&mut self, node_id: CausalNodeId) {
        self.nodes.push(node_id);
    }

    /// Add edge
    pub fn add_edge(&mut self, edge_id: CausalEdgeId, confidence: f32, strength: f32) {
        self.edges.push(edge_id);
        self.total_confidence *= confidence;
        self.strength += strength;
    }

    /// Chain length
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Root cause (first node)
    pub fn root_cause(&self) -> Option<CausalNodeId> {
        self.nodes.first().copied()
    }

    /// Terminal effect (last node)
    pub fn terminal_effect(&self) -> Option<CausalNodeId> {
        self.nodes.last().copied()
    }

    /// Average strength
    pub fn average_strength(&self) -> f32 {
        if self.edges.is_empty() {
            return 0.0;
        }
        self.strength / self.edges.len() as f32
    }

    /// Set time span
    pub fn set_time_span(&mut self, span: u64) {
        self.time_span = span;
    }
}

/// Causal chain builder
pub struct CausalChainBuilder {
    /// Chain counter
    chain_counter: AtomicU64,
}

impl CausalChainBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            chain_counter: AtomicU64::new(0),
        }
    }

    /// Build chain from cause to effect
    pub fn build_chain(
        &self,
        graph: &CausalGraph,
        from: CausalNodeId,
        to: CausalNodeId,
    ) -> Option<CausalChain> {
        // BFS to find path
        let mut queue = Vec::new();
        let mut visited = BTreeMap::new();
        let mut parent = BTreeMap::new();
        let mut parent_edge = BTreeMap::new();

        queue.push(from);
        visited.insert(from, true);

        while let Some(current) = queue.pop() {
            if current == to {
                // Found path, reconstruct chain
                let id = ChainId(self.chain_counter.fetch_add(1, Ordering::Relaxed));
                let mut chain = CausalChain::new(id);

                let mut path = Vec::new();
                let mut edges = Vec::new();
                let mut node = to;

                while node != from {
                    path.push(node);
                    if let Some(&edge_id) = parent_edge.get(&node) {
                        edges.push(edge_id);
                    }
                    if let Some(&prev) = parent.get(&node) {
                        node = prev;
                    } else {
                        break;
                    }
                }
                path.push(from);

                // Reverse to get cause → effect order
                path.reverse();
                edges.reverse();

                for node_id in path {
                    chain.add_node(node_id);
                }

                for edge_id in edges {
                    if let Some(edge) = graph.get_edge(edge_id) {
                        chain.add_edge(edge_id, edge.confidence, edge.combined_strength());
                    }
                }

                // Calculate time span
                if let (Some(first), Some(last)) = (
                    graph.get_node(from).map(|n| n.event.timestamp),
                    graph.get_node(to).map(|n| n.event.timestamp),
                ) {
                    chain.set_time_span(last.saturating_sub(first));
                }

                return Some(chain);
            }

            if let Some(node) = graph.get_node(current) {
                for &edge_id in &node.effects {
                    if let Some(edge) = graph.get_edge(edge_id) {
                        let next = edge.target;
                        if !visited.contains_key(&next) {
                            visited.insert(next, true);
                            parent.insert(next, current);
                            parent_edge.insert(next, edge_id);
                            queue.push(next);
                        }
                    }
                }
            }
        }

        None
    }

    /// Build all chains to a target
    pub fn build_all_chains_to(
        &self,
        graph: &CausalGraph,
        to: CausalNodeId,
        max_chains: usize,
    ) -> Vec<CausalChain> {
        let roots = graph.find_root_causes(to);
        let mut chains = Vec::new();

        for root in roots.into_iter().take(max_chains) {
            if let Some(chain) = self.build_chain(graph, root, to) {
                chains.push(chain);
            }
        }

        chains
    }

    /// Build chain count
    pub fn chain_count(&self) -> u64 {
        self.chain_counter.load(Ordering::Relaxed)
    }
}

impl Default for CausalChainBuilder {
    fn default() -> Self {
        Self::new()
    }
}
