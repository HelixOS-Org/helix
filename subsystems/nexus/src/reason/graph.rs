//! Causal Graph
//!
//! This module provides the causal DAG (Directed Acyclic Graph) structure.

#![allow(clippy::excessive_nesting)]

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{CausalEdgeId, CausalEvent, CausalEventId, CausalNodeId};

/// Causal relationship type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalRelationType {
    /// Direct cause (A directly caused B)
    DirectCause,
    /// Indirect cause (A led to conditions for B)
    IndirectCause,
    /// Contributing factor (A contributed to B)
    ContributingFactor,
    /// Necessary condition (B couldn't happen without A)
    NecessaryCondition,
    /// Sufficient condition (A alone can cause B)
    SufficientCondition,
    /// Temporal precedence (A happened before B)
    TemporalPrecedence,
    /// Correlation (A and B are correlated)
    Correlation,
    /// Inhibition (A prevented or reduced B)
    Inhibition,
}

impl CausalRelationType {
    /// Get relation name
    pub fn name(&self) -> &'static str {
        match self {
            Self::DirectCause => "direct_cause",
            Self::IndirectCause => "indirect_cause",
            Self::ContributingFactor => "contributing_factor",
            Self::NecessaryCondition => "necessary_condition",
            Self::SufficientCondition => "sufficient_condition",
            Self::TemporalPrecedence => "temporal_precedence",
            Self::Correlation => "correlation",
            Self::Inhibition => "inhibition",
        }
    }

    /// Is causal (not just correlation)
    pub fn is_causal(&self) -> bool {
        !matches!(self, Self::Correlation | Self::TemporalPrecedence)
    }

    /// Causal strength (0.0 - 1.0)
    pub fn strength(&self) -> f32 {
        match self {
            Self::DirectCause => 1.0,
            Self::SufficientCondition => 0.95,
            Self::NecessaryCondition => 0.9,
            Self::IndirectCause => 0.7,
            Self::ContributingFactor => 0.5,
            Self::Inhibition => 0.4,
            Self::TemporalPrecedence => 0.2,
            Self::Correlation => 0.1,
        }
    }
}

/// Causal node (event in the causal graph)
#[derive(Debug, Clone)]
pub struct CausalNode {
    /// Node ID
    pub id: CausalNodeId,
    /// Associated event
    pub event: CausalEvent,
    /// Incoming edges (causes)
    pub causes: Vec<CausalEdgeId>,
    /// Outgoing edges (effects)
    pub effects: Vec<CausalEdgeId>,
    /// Is root cause
    pub is_root_cause: bool,
    /// Is terminal effect
    pub is_terminal: bool,
    /// Causal depth (distance from root)
    pub depth: u32,
}

impl CausalNode {
    /// Create new node
    pub fn new(id: CausalNodeId, event: CausalEvent) -> Self {
        Self {
            id,
            event,
            causes: Vec::new(),
            effects: Vec::new(),
            is_root_cause: false,
            is_terminal: false,
            depth: 0,
        }
    }

    /// Has causes
    pub fn has_causes(&self) -> bool {
        !self.causes.is_empty()
    }

    /// Has effects
    pub fn has_effects(&self) -> bool {
        !self.effects.is_empty()
    }
}

/// Causal edge (relationship between events)
#[derive(Debug, Clone)]
pub struct CausalEdge {
    /// Edge ID
    pub id: CausalEdgeId,
    /// Source node (cause)
    pub source: CausalNodeId,
    /// Target node (effect)
    pub target: CausalNodeId,
    /// Relation type
    pub relation: CausalRelationType,
    /// Confidence (0.0 - 1.0)
    pub confidence: f32,
    /// Time delta (effect timestamp - cause timestamp)
    pub time_delta: u64,
    /// Evidence
    pub evidence: Vec<String>,
}

impl CausalEdge {
    /// Create new edge
    pub fn new(
        id: CausalEdgeId,
        source: CausalNodeId,
        target: CausalNodeId,
        relation: CausalRelationType,
    ) -> Self {
        Self {
            id,
            source,
            target,
            relation,
            confidence: 0.5,
            time_delta: 0,
            evidence: Vec::new(),
        }
    }

    /// With confidence
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// With time delta
    pub fn with_time_delta(mut self, delta: u64) -> Self {
        self.time_delta = delta;
        self
    }

    /// Add evidence
    pub fn add_evidence(&mut self, evidence: String) {
        self.evidence.push(evidence);
    }

    /// Combined strength (relation strength * confidence)
    pub fn combined_strength(&self) -> f32 {
        self.relation.strength() * self.confidence
    }
}

/// Causal graph (DAG of causal relationships)
#[derive(Debug)]
pub struct CausalGraph {
    /// Nodes
    pub(crate) nodes: BTreeMap<CausalNodeId, CausalNode>,
    /// Edges
    pub(crate) edges: BTreeMap<CausalEdgeId, CausalEdge>,
    /// Node counter
    node_counter: AtomicU64,
    /// Edge counter
    edge_counter: AtomicU64,
    /// Event to node mapping
    event_to_node: BTreeMap<CausalEventId, CausalNodeId>,
}

impl CausalGraph {
    /// Create new graph
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            node_counter: AtomicU64::new(0),
            edge_counter: AtomicU64::new(0),
            event_to_node: BTreeMap::new(),
        }
    }

    /// Add event to graph
    pub fn add_event(&mut self, event: CausalEvent) -> CausalNodeId {
        let event_id = event.id;
        let node_id = CausalNodeId(self.node_counter.fetch_add(1, Ordering::Relaxed));
        let node = CausalNode::new(node_id, event);
        self.nodes.insert(node_id, node);
        self.event_to_node.insert(event_id, node_id);
        node_id
    }

    /// Add causal relationship
    pub fn add_relationship(
        &mut self,
        cause: CausalNodeId,
        effect: CausalNodeId,
        relation: CausalRelationType,
        confidence: f32,
    ) -> Option<CausalEdgeId> {
        // Check nodes exist
        if !self.nodes.contains_key(&cause) || !self.nodes.contains_key(&effect) {
            return None;
        }

        // Calculate time delta
        let time_delta = {
            let cause_time = self.nodes.get(&cause)?.event.timestamp;
            let effect_time = self.nodes.get(&effect)?.event.timestamp;
            effect_time.saturating_sub(cause_time)
        };

        // Create edge
        let edge_id = CausalEdgeId(self.edge_counter.fetch_add(1, Ordering::Relaxed));
        let edge = CausalEdge::new(edge_id, cause, effect, relation)
            .with_confidence(confidence)
            .with_time_delta(time_delta);

        // Update nodes
        if let Some(cause_node) = self.nodes.get_mut(&cause) {
            cause_node.effects.push(edge_id);
        }
        if let Some(effect_node) = self.nodes.get_mut(&effect) {
            effect_node.causes.push(edge_id);
        }

        self.edges.insert(edge_id, edge);
        Some(edge_id)
    }

    /// Get node
    pub fn get_node(&self, id: CausalNodeId) -> Option<&CausalNode> {
        self.nodes.get(&id)
    }

    /// Get node mutably
    pub fn get_node_mut(&mut self, id: CausalNodeId) -> Option<&mut CausalNode> {
        self.nodes.get_mut(&id)
    }

    /// Get edge
    pub fn get_edge(&self, id: CausalEdgeId) -> Option<&CausalEdge> {
        self.edges.get(&id)
    }

    /// Get node by event
    pub fn get_node_by_event(&self, event_id: CausalEventId) -> Option<&CausalNode> {
        let node_id = self.event_to_node.get(&event_id)?;
        self.nodes.get(node_id)
    }

    /// Find root causes for a node
    pub fn find_root_causes(&self, node_id: CausalNodeId) -> Vec<CausalNodeId> {
        let mut roots = Vec::new();
        let mut visited = Vec::new();
        self.find_roots_recursive(node_id, &mut roots, &mut visited);
        roots
    }

    fn find_roots_recursive(
        &self,
        node_id: CausalNodeId,
        roots: &mut Vec<CausalNodeId>,
        visited: &mut Vec<CausalNodeId>,
    ) {
        if visited.contains(&node_id) {
            return;
        }
        visited.push(node_id);

        if let Some(node) = self.nodes.get(&node_id) {
            if node.causes.is_empty() {
                roots.push(node_id);
            } else {
                for &edge_id in &node.causes {
                    if let Some(edge) = self.edges.get(&edge_id) {
                        self.find_roots_recursive(edge.source, roots, visited);
                    }
                }
            }
        }
    }

    /// Find effects of a node
    pub fn find_effects(&self, node_id: CausalNodeId) -> Vec<CausalNodeId> {
        let mut effects = Vec::new();
        let mut visited = Vec::new();
        self.find_effects_recursive(node_id, &mut effects, &mut visited);
        effects
    }

    fn find_effects_recursive(
        &self,
        node_id: CausalNodeId,
        effects: &mut Vec<CausalNodeId>,
        visited: &mut Vec<CausalNodeId>,
    ) {
        if visited.contains(&node_id) {
            return;
        }
        visited.push(node_id);

        if let Some(node) = self.nodes.get(&node_id) {
            for &edge_id in &node.effects {
                if let Some(edge) = self.edges.get(&edge_id) {
                    effects.push(edge.target);
                    self.find_effects_recursive(edge.target, effects, visited);
                }
            }
        }
    }

    /// Calculate causal depth for all nodes
    pub fn calculate_depths(&mut self) {
        // Find root nodes (no causes)
        let roots: Vec<CausalNodeId> = self
            .nodes
            .values()
            .filter(|n| n.causes.is_empty())
            .map(|n| n.id)
            .collect();

        // Mark roots
        for &root_id in &roots {
            if let Some(node) = self.nodes.get_mut(&root_id) {
                node.is_root_cause = true;
                node.depth = 0;
            }
        }

        // BFS to calculate depths
        let mut queue = roots;
        while let Some(node_id) = queue.pop() {
            let depth = self.nodes.get(&node_id).map(|n| n.depth).unwrap_or(0);
            let effects: Vec<CausalEdgeId> = self
                .nodes
                .get(&node_id)
                .map(|n| n.effects.clone())
                .unwrap_or_default();

            for edge_id in effects {
                if let Some(edge) = self.edges.get(&edge_id) {
                    let target = edge.target;
                    if let Some(target_node) = self.nodes.get_mut(&target) {
                        if target_node.depth < depth + 1 {
                            target_node.depth = depth + 1;
                            queue.push(target);
                        }
                    }
                }
            }
        }

        // Find terminal nodes (no effects)
        let terminals: Vec<CausalNodeId> = self
            .nodes
            .values()
            .filter(|n| n.effects.is_empty())
            .map(|n| n.id)
            .collect();

        for terminal_id in terminals {
            if let Some(node) = self.nodes.get_mut(&terminal_id) {
                node.is_terminal = true;
            }
        }
    }

    /// Node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get all root causes
    pub fn root_causes(&self) -> Vec<&CausalNode> {
        self.nodes.values().filter(|n| n.is_root_cause).collect()
    }

    /// Get all terminal effects
    pub fn terminal_effects(&self) -> Vec<&CausalNode> {
        self.nodes.values().filter(|n| n.is_terminal).collect()
    }

    /// Iterate all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &CausalNode> {
        self.nodes.values()
    }

    /// Iterate all edges
    pub fn edges_iter(&self) -> impl Iterator<Item = &CausalEdge> {
        self.edges.values()
    }
}

impl Default for CausalGraph {
    fn default() -> Self {
        Self::new()
    }
}
