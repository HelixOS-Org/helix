//! Causal graph implementation

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use super::edge::{CausalEdge, CausalEdgeType};
use super::node::{CausalNode, CausalNodeType};

// ============================================================================
// CAUSAL GRAPH
// ============================================================================

/// A causal dependency graph
pub struct CausalGraph {
    /// Nodes by ID
    nodes: BTreeMap<u64, CausalNode>,
    /// Edges (from -> to)
    edges: Vec<CausalEdge>,
    /// Adjacency list (from -> [to])
    adjacency: BTreeMap<u64, Vec<u64>>,
    /// Reverse adjacency (to -> [from])
    reverse_adjacency: BTreeMap<u64, Vec<u64>>,
    /// Root nodes (no incoming edges)
    roots: Vec<u64>,
    /// Error nodes
    error_nodes: Vec<u64>,
}

impl CausalGraph {
    /// Create a new causal graph
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            adjacency: BTreeMap::new(),
            reverse_adjacency: BTreeMap::new(),
            roots: Vec::new(),
            error_nodes: Vec::new(),
        }
    }

    /// Add a node
    #[inline]
    pub fn add_node(&mut self, node: CausalNode) -> u64 {
        let id = node.id;

        if node.node_type == CausalNodeType::Error {
            self.error_nodes.push(id);
        }

        self.nodes.insert(id, node);
        self.update_roots();
        id
    }

    /// Add an edge
    #[inline]
    pub fn add_edge(&mut self, edge: CausalEdge) {
        let from = edge.from;
        let to = edge.to;

        self.adjacency.entry(from).or_default().push(to);
        self.reverse_adjacency.entry(to).or_default().push(from);
        self.edges.push(edge);

        self.update_roots();
    }

    /// Add causal link between two nodes
    #[inline]
    pub fn link(&mut self, from: u64, to: u64, edge_type: CausalEdgeType) {
        if let (Some(from_node), Some(to_node)) = (self.nodes.get(&from), self.nodes.get(&to)) {
            let latency = to_node.timestamp.duration_since(from_node.timestamp);
            let edge = CausalEdge::new(from, to, edge_type).with_latency(latency);
            self.add_edge(edge);
        }
    }

    /// Update root nodes
    fn update_roots(&mut self) {
        self.roots = self
            .nodes
            .keys()
            .filter(|id| !self.reverse_adjacency.contains_key(id))
            .copied()
            .collect();
    }

    /// Get a node by ID
    #[inline(always)]
    pub fn get_node(&self, id: u64) -> Option<&CausalNode> {
        self.nodes.get(&id)
    }

    /// Get all nodes
    #[inline(always)]
    pub fn nodes(&self) -> impl Iterator<Item = &CausalNode> {
        self.nodes.values()
    }

    /// Get all edges
    #[inline(always)]
    pub fn edges(&self) -> &[CausalEdge] {
        &self.edges
    }

    /// Get root nodes
    #[inline(always)]
    pub fn roots(&self) -> &[u64] {
        &self.roots
    }

    /// Get error nodes
    #[inline(always)]
    pub fn error_nodes(&self) -> &[u64] {
        &self.error_nodes
    }

    /// Get children of a node
    #[inline]
    pub fn children(&self, node_id: u64) -> &[u64] {
        self.adjacency
            .get(&node_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get parents of a node
    #[inline]
    pub fn parents(&self, node_id: u64) -> &[u64] {
        self.reverse_adjacency
            .get(&node_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Find path from root to a node
    pub fn path_to(&self, target: u64) -> Vec<u64> {
        let mut path = Vec::new();
        let mut current = target;

        while let Some(parents) = self.reverse_adjacency.get(&current) {
            path.push(current);
            if parents.is_empty() {
                break;
            }
            // Follow first parent (simplification)
            current = parents[0];
        }

        path.reverse();
        path
    }

    /// Find root cause of an error
    pub fn find_root_cause(&self, error_node: u64) -> Option<u64> {
        let path = self.path_to(error_node);

        // Walk back to find earliest error or anomaly
        for &node_id in path.iter().rev() {
            if let Some(node) = self.nodes.get(&node_id) {
                if node.node_type == CausalNodeType::Error {
                    return Some(node_id);
                }
            }
        }

        // Return the root of the path
        path.first().copied()
    }

    /// Find critical path (longest path)
    pub fn critical_path(&self) -> Vec<u64> {
        if self.roots.is_empty() {
            return Vec::new();
        }

        // Simple longest path using DFS
        let mut longest_path = Vec::new();
        let mut max_weight = 0.0;

        for &root in &self.roots {
            let (path, weight) = self.dfs_longest_path(root);
            if weight > max_weight {
                max_weight = weight;
                longest_path = path;
            }
        }

        longest_path
    }

    /// DFS to find longest path from a node
    fn dfs_longest_path(&self, start: u64) -> (Vec<u64>, f64) {
        let children = self.children(start);

        if children.is_empty() {
            return (vec![start], 0.0);
        }

        let mut best_path = Vec::new();
        let mut best_weight = 0.0;

        for &child in children {
            let (mut path, weight) = self.dfs_longest_path(child);

            // Find edge weight
            let edge_weight = self
                .edges
                .iter()
                .find(|e| e.from == start && e.to == child)
                .map(|e| e.weight)
                .unwrap_or(1.0);

            let total_weight = weight + edge_weight;
            if total_weight > best_weight {
                best_weight = total_weight;
                path.insert(0, start);
                best_path = path;
            }
        }

        (best_path, best_weight)
    }

    /// Get all paths from any root to a specific node
    #[inline]
    pub fn all_paths_to(&self, target: u64) -> Vec<Vec<u64>> {
        let mut all_paths = Vec::new();

        for &root in &self.roots {
            self.dfs_all_paths(root, target, vec![root], &mut all_paths);
        }

        all_paths
    }

    /// DFS to find all paths
    fn dfs_all_paths(&self, current: u64, target: u64, path: Vec<u64>, paths: &mut Vec<Vec<u64>>) {
        if current == target {
            paths.push(path);
            return;
        }

        for &child in self.children(current) {
            let mut new_path = path.clone();
            new_path.push(child);
            self.dfs_all_paths(child, target, new_path, paths);
        }
    }

    /// Get subgraph containing only nodes reachable from specified roots
    pub fn subgraph_from(&self, roots: &[u64]) -> CausalGraph {
        let mut subgraph = CausalGraph::new();
        let mut visited = Vec::new();
        let mut stack = roots.to_vec();

        while let Some(node_id) = stack.pop() {
            if visited.contains(&node_id) {
                continue;
            }
            visited.push(node_id);

            if let Some(node) = self.nodes.get(&node_id) {
                subgraph.add_node(node.clone());
            }

            for &child in self.children(node_id) {
                stack.push(child);
            }
        }

        // Add edges
        for edge in &self.edges {
            if visited.contains(&edge.from) && visited.contains(&edge.to) {
                subgraph.add_edge(edge.clone());
            }
        }

        subgraph
    }

    /// Get node count
    #[inline(always)]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get edge count
    #[inline(always)]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Clear the graph
    #[inline]
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.adjacency.clear();
        self.reverse_adjacency.clear();
        self.roots.clear();
        self.error_nodes.clear();
    }
}

impl Default for CausalGraph {
    fn default() -> Self {
        Self::new()
    }
}
