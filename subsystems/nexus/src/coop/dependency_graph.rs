// SPDX-License-Identifier: GPL-2.0
//! Coop dependency_graph — task dependency tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Node type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepNodeType {
    Task,
    Resource,
    Mutex,
    Event,
    Barrier,
}

/// Edge type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepEdgeType {
    DependsOn,
    BlockedBy,
    WaitsFor,
    ProducesFor,
}

/// Graph node
#[derive(Debug)]
pub struct DepNode {
    pub id: u64,
    pub node_type: DepNodeType,
    pub name_hash: u64,
    pub in_edges: Vec<u64>,
    pub out_edges: Vec<u64>,
    pub completed: bool,
    pub depth: u32,
}

impl DepNode {
    pub fn new(id: u64, ntype: DepNodeType) -> Self {
        Self { id, node_type: ntype, name_hash: id, in_edges: Vec::new(), out_edges: Vec::new(), completed: false, depth: 0 }
    }

    pub fn in_degree(&self) -> u32 { self.in_edges.len() as u32 }
    pub fn out_degree(&self) -> u32 { self.out_edges.len() as u32 }
    pub fn is_source(&self) -> bool { self.in_edges.is_empty() }
    pub fn is_sink(&self) -> bool { self.out_edges.is_empty() }
}

/// Graph edge
#[derive(Debug)]
pub struct DepEdge {
    pub id: u64,
    pub from: u64,
    pub to: u64,
    pub edge_type: DepEdgeType,
    pub weight: u32,
}

/// Cycle info
#[derive(Debug, Clone)]
pub struct CycleInfo {
    pub nodes: Vec<u64>,
    pub detected_at: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct DepGraphStats {
    pub total_nodes: u32,
    pub total_edges: u32,
    pub completed_nodes: u32,
    pub max_depth: u32,
    pub cycles_detected: u64,
    pub ready_nodes: u32,
}

/// Main dependency graph
pub struct CoopDependencyGraph {
    nodes: BTreeMap<u64, DepNode>,
    edges: BTreeMap<u64, DepEdge>,
    cycles: Vec<CycleInfo>,
    next_node_id: u64,
    next_edge_id: u64,
}

impl CoopDependencyGraph {
    pub fn new() -> Self { Self { nodes: BTreeMap::new(), edges: BTreeMap::new(), cycles: Vec::new(), next_node_id: 1, next_edge_id: 1 } }

    pub fn add_node(&mut self, ntype: DepNodeType) -> u64 {
        let id = self.next_node_id; self.next_node_id += 1;
        self.nodes.insert(id, DepNode::new(id, ntype));
        id
    }

    pub fn add_edge(&mut self, from: u64, to: u64, etype: DepEdgeType) -> u64 {
        let id = self.next_edge_id; self.next_edge_id += 1;
        self.edges.insert(id, DepEdge { id, from, to, edge_type: etype, weight: 1 });
        if let Some(n) = self.nodes.get_mut(&from) { n.out_edges.push(to); }
        if let Some(n) = self.nodes.get_mut(&to) { n.in_edges.push(from); }
        id
    }

    pub fn complete_node(&mut self, id: u64) {
        if let Some(n) = self.nodes.get_mut(&id) { n.completed = true; }
    }

    pub fn ready_nodes(&self) -> Vec<u64> {
        self.nodes.values().filter(|n| !n.completed && n.in_edges.iter().all(|dep| self.nodes.get(dep).map(|d| d.completed).unwrap_or(true))).map(|n| n.id).collect()
    }

    pub fn stats(&self) -> DepGraphStats {
        let completed = self.nodes.values().filter(|n| n.completed).count() as u32;
        let max_depth = self.nodes.values().map(|n| n.depth).max().unwrap_or(0);
        let ready = self.ready_nodes().len() as u32;
        DepGraphStats { total_nodes: self.nodes.len() as u32, total_edges: self.edges.len() as u32, completed_nodes: completed, max_depth, cycles_detected: self.cycles.len() as u64, ready_nodes: ready }
    }
}
