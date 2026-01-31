//! # Dependency Analysis
//!
//! Analyzes dependencies between code elements.
//! Tracks imports, uses, and module relationships.
//!
//! Part of Year 2 COGNITION - Q1: Code Understanding

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// DEPENDENCY TYPES
// ============================================================================

/// Dependency node
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// Node ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Node type
    pub node_type: NodeType,
    /// Path
    pub path: String,
    /// Dependencies
    pub dependencies: Vec<DependencyEdge>,
    /// Dependents
    pub dependents: Vec<u64>,
    /// Created
    pub created: Timestamp,
}

/// Node type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeType {
    Module,
    Crate,
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    Macro,
    Constant,
    File,
}

/// Dependency edge
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Target node ID
    pub target: u64,
    /// Edge type
    pub edge_type: EdgeType,
    /// Optional
    pub optional: bool,
    /// Context
    pub context: String,
}

/// Edge type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Import,
    Use,
    Call,
    Inherit,
    Implement,
    Compose,
    Reference,
    Type,
}

/// Dependency cycle
#[derive(Debug, Clone)]
pub struct DependencyCycle {
    /// Nodes in cycle
    pub nodes: Vec<u64>,
    /// Edge types
    pub edges: Vec<EdgeType>,
}

/// Dependency analysis result
#[derive(Debug, Clone)]
pub struct DependencyAnalysis {
    /// Total nodes
    pub total_nodes: usize,
    /// Total edges
    pub total_edges: usize,
    /// Cycles detected
    pub cycles: Vec<DependencyCycle>,
    /// Most depended on
    pub most_depended: Vec<(u64, usize)>,
    /// Most depending
    pub most_depending: Vec<(u64, usize)>,
    /// Orphan nodes
    pub orphans: Vec<u64>,
    /// Timestamp
    pub timestamp: Timestamp,
}

// ============================================================================
// DEPENDENCY GRAPH
// ============================================================================

/// Dependency graph
pub struct DependencyGraph {
    /// Nodes
    nodes: BTreeMap<u64, DependencyNode>,
    /// Name to ID mapping
    name_to_id: BTreeMap<String, u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: GraphConfig,
    /// Statistics
    stats: GraphStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct GraphConfig {
    /// Detect cycles
    pub detect_cycles: bool,
    /// Maximum cycle length
    pub max_cycle_length: usize,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            detect_cycles: true,
            max_cycle_length: 10,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    /// Nodes added
    pub nodes_added: u64,
    /// Edges added
    pub edges_added: u64,
    /// Analyses performed
    pub analyses: u64,
}

impl DependencyGraph {
    /// Create new graph
    pub fn new(config: GraphConfig) -> Self {
        Self {
            nodes: BTreeMap::new(),
            name_to_id: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: GraphStats::default(),
        }
    }

    /// Add node
    pub fn add_node(&mut self, name: &str, node_type: NodeType, path: &str) -> u64 {
        // Check if exists
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let node = DependencyNode {
            id,
            name: name.into(),
            node_type,
            path: path.into(),
            dependencies: Vec::new(),
            dependents: Vec::new(),
            created: Timestamp::now(),
        };

        self.name_to_id.insert(name.into(), id);
        self.nodes.insert(id, node);
        self.stats.nodes_added += 1;

        id
    }

    /// Add dependency
    pub fn add_dependency(
        &mut self,
        from: u64,
        to: u64,
        edge_type: EdgeType,
        optional: bool,
        context: &str,
    ) {
        // Add forward edge
        if let Some(node) = self.nodes.get_mut(&from) {
            let edge = DependencyEdge {
                target: to,
                edge_type,
                optional,
                context: context.into(),
            };
            node.dependencies.push(edge);
        }

        // Add back reference
        if let Some(target) = self.nodes.get_mut(&to) {
            if !target.dependents.contains(&from) {
                target.dependents.push(from);
            }
        }

        self.stats.edges_added += 1;
    }

    /// Get node
    pub fn get(&self, id: u64) -> Option<&DependencyNode> {
        self.nodes.get(&id)
    }

    /// Get by name
    pub fn get_by_name(&self, name: &str) -> Option<&DependencyNode> {
        self.name_to_id.get(name)
            .and_then(|id| self.nodes.get(id))
    }

    /// Get dependencies
    pub fn dependencies(&self, id: u64) -> Vec<&DependencyNode> {
        self.nodes.get(&id)
            .map(|node| {
                node.dependencies.iter()
                    .filter_map(|edge| self.nodes.get(&edge.target))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get dependents
    pub fn dependents(&self, id: u64) -> Vec<&DependencyNode> {
        self.nodes.get(&id)
            .map(|node| {
                node.dependents.iter()
                    .filter_map(|dep_id| self.nodes.get(dep_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get transitive dependencies
    pub fn transitive_dependencies(&self, id: u64) -> Vec<u64> {
        let mut visited = BTreeSet::new();
        let mut result = Vec::new();

        self.collect_dependencies(id, &mut visited, &mut result);

        result
    }

    fn collect_dependencies(&self, id: u64, visited: &mut BTreeSet<u64>, result: &mut Vec<u64>) {
        if visited.contains(&id) {
            return;
        }
        visited.insert(id);

        if let Some(node) = self.nodes.get(&id) {
            for edge in &node.dependencies {
                if !visited.contains(&edge.target) {
                    result.push(edge.target);
                    self.collect_dependencies(edge.target, visited, result);
                }
            }
        }
    }

    /// Detect cycles
    pub fn detect_cycles(&self) -> Vec<DependencyCycle> {
        let mut cycles = Vec::new();

        for &start_id in self.nodes.keys() {
            self.find_cycles_from(start_id, &mut cycles);
        }

        cycles
    }

    fn find_cycles_from(&self, start: u64, cycles: &mut Vec<DependencyCycle>) {
        let mut path = Vec::new();
        let mut visited = BTreeSet::new();
        let mut edge_types = Vec::new();

        self.dfs_cycles(start, start, &mut path, &mut edge_types, &mut visited, cycles);
    }

    fn dfs_cycles(
        &self,
        current: u64,
        target: u64,
        path: &mut Vec<u64>,
        edge_types: &mut Vec<EdgeType>,
        visited: &mut BTreeSet<u64>,
        cycles: &mut Vec<DependencyCycle>,
    ) {
        if path.len() > self.config.max_cycle_length {
            return;
        }

        path.push(current);
        visited.insert(current);

        if let Some(node) = self.nodes.get(&current) {
            for edge in &node.dependencies {
                edge_types.push(edge.edge_type);

                if edge.target == target && path.len() > 1 {
                    // Found cycle
                    cycles.push(DependencyCycle {
                        nodes: path.clone(),
                        edges: edge_types.clone(),
                    });
                } else if !visited.contains(&edge.target) {
                    self.dfs_cycles(edge.target, target, path, edge_types, visited, cycles);
                }

                edge_types.pop();
            }
        }

        path.pop();
        visited.remove(&current);
    }

    /// Analyze graph
    pub fn analyze(&mut self) -> DependencyAnalysis {
        self.stats.analyses += 1;

        let total_nodes = self.nodes.len();
        let total_edges = self.nodes.values()
            .map(|n| n.dependencies.len())
            .sum();

        // Detect cycles
        let cycles = if self.config.detect_cycles {
            self.detect_cycles()
        } else {
            Vec::new()
        };

        // Most depended on
        let mut depended: Vec<(u64, usize)> = self.nodes.iter()
            .map(|(&id, node)| (id, node.dependents.len()))
            .collect();
        depended.sort_by(|a, b| b.1.cmp(&a.1));
        let most_depended: Vec<_> = depended.into_iter().take(10).collect();

        // Most depending
        let mut depending: Vec<(u64, usize)> = self.nodes.iter()
            .map(|(&id, node)| (id, node.dependencies.len()))
            .collect();
        depending.sort_by(|a, b| b.1.cmp(&a.1));
        let most_depending: Vec<_> = depending.into_iter().take(10).collect();

        // Orphans
        let orphans: Vec<u64> = self.nodes.iter()
            .filter(|(_, n)| n.dependencies.is_empty() && n.dependents.is_empty())
            .map(|(&id, _)| id)
            .collect();

        DependencyAnalysis {
            total_nodes,
            total_edges,
            cycles,
            most_depended,
            most_depending,
            orphans,
            timestamp: Timestamp::now(),
        }
    }

    /// Topological sort
    pub fn topological_sort(&self) -> Option<Vec<u64>> {
        let mut in_degree: BTreeMap<u64, usize> = BTreeMap::new();
        let mut result = Vec::new();
        let mut queue = Vec::new();

        // Calculate in-degrees
        for (&id, _) in &self.nodes {
            in_degree.insert(id, 0);
        }

        for node in self.nodes.values() {
            for edge in &node.dependencies {
                *in_degree.entry(edge.target).or_insert(0) += 1;
            }
        }

        // Find nodes with zero in-degree
        for (&id, &degree) in &in_degree {
            if degree == 0 {
                queue.push(id);
            }
        }

        // Process
        while let Some(id) = queue.pop() {
            result.push(id);

            if let Some(node) = self.nodes.get(&id) {
                for edge in &node.dependencies {
                    if let Some(degree) = in_degree.get_mut(&edge.target) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            queue.push(edge.target);
                        }
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None // Has cycles
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &GraphStats {
        &self.stats
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new(GraphConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_node() {
        let mut graph = DependencyGraph::default();

        let id = graph.add_node("test", NodeType::Module, "/test");
        assert!(graph.get(id).is_some());
    }

    #[test]
    fn test_add_dependency() {
        let mut graph = DependencyGraph::default();

        let a = graph.add_node("a", NodeType::Module, "/a");
        let b = graph.add_node("b", NodeType::Module, "/b");

        graph.add_dependency(a, b, EdgeType::Import, false, "use b");

        let deps = graph.dependencies(a);
        assert_eq!(deps.len(), 1);

        let dependents = graph.dependents(b);
        assert_eq!(dependents.len(), 1);
    }

    #[test]
    fn test_transitive() {
        let mut graph = DependencyGraph::default();

        let a = graph.add_node("a", NodeType::Module, "/a");
        let b = graph.add_node("b", NodeType::Module, "/b");
        let c = graph.add_node("c", NodeType::Module, "/c");

        graph.add_dependency(a, b, EdgeType::Import, false, "");
        graph.add_dependency(b, c, EdgeType::Import, false, "");

        let trans = graph.transitive_dependencies(a);
        assert_eq!(trans.len(), 2);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::default();

        let a = graph.add_node("a", NodeType::Module, "/a");
        let b = graph.add_node("b", NodeType::Module, "/b");

        graph.add_dependency(a, b, EdgeType::Import, false, "");
        graph.add_dependency(b, a, EdgeType::Import, false, "");

        let cycles = graph.detect_cycles();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = DependencyGraph::default();

        let a = graph.add_node("a", NodeType::Module, "/a");
        let b = graph.add_node("b", NodeType::Module, "/b");
        let c = graph.add_node("c", NodeType::Module, "/c");

        graph.add_dependency(a, b, EdgeType::Import, false, "");
        graph.add_dependency(b, c, EdgeType::Import, false, "");

        let sorted = graph.topological_sort();
        assert!(sorted.is_some());
    }

    #[test]
    fn test_analysis() {
        let mut graph = DependencyGraph::default();

        let a = graph.add_node("a", NodeType::Module, "/a");
        let b = graph.add_node("b", NodeType::Module, "/b");

        graph.add_dependency(a, b, EdgeType::Import, false, "");

        let analysis = graph.analyze();
        assert_eq!(analysis.total_nodes, 2);
        assert_eq!(analysis.total_edges, 1);
    }
}
