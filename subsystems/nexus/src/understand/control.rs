//! # Control Flow Analysis
//!
//! Analyzes control flow through code.
//! Builds CFGs and identifies paths.
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
// CFG TYPES
// ============================================================================

/// Basic block
#[derive(Debug, Clone)]
pub struct BasicBlock {
    /// Block ID
    pub id: u64,
    /// Label
    pub label: String,
    /// Statements
    pub statements: Vec<Statement>,
    /// Successors
    pub successors: Vec<Edge>,
    /// Predecessors
    pub predecessors: Vec<u64>,
    /// Block type
    pub block_type: BlockType,
    /// Start line
    pub start_line: u32,
    /// End line
    pub end_line: u32,
}

/// Block type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    Entry,
    Exit,
    Normal,
    Conditional,
    Loop,
    Return,
    Unreachable,
}

/// Statement
#[derive(Debug, Clone)]
pub struct Statement {
    /// Statement ID
    pub id: u64,
    /// Kind
    pub kind: StatementKind,
    /// Line
    pub line: u32,
    /// Column
    pub column: u32,
}

/// Statement kind
#[derive(Debug, Clone)]
pub enum StatementKind {
    Assign(String, String),
    Call(String, Vec<String>),
    Return(Option<String>),
    Branch(String),
    Nop,
}

/// Edge
#[derive(Debug, Clone)]
pub struct Edge {
    /// Target block
    pub target: u64,
    /// Edge type
    pub edge_type: EdgeType,
    /// Condition
    pub condition: Option<String>,
}

/// Edge type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Unconditional,
    True,
    False,
    LoopBack,
    LoopExit,
    Exception,
    Fallthrough,
}

/// Control flow graph
#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    /// Graph ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Entry block
    pub entry: u64,
    /// Exit blocks
    pub exits: Vec<u64>,
    /// Blocks
    pub blocks: BTreeMap<u64, BasicBlock>,
    /// Created
    pub created: Timestamp,
}

/// Path
#[derive(Debug, Clone)]
pub struct Path {
    /// Path ID
    pub id: u64,
    /// Blocks in path
    pub blocks: Vec<u64>,
    /// Conditions
    pub conditions: Vec<String>,
    /// Feasible
    pub feasible: bool,
}

/// CFG analysis result
#[derive(Debug, Clone)]
pub struct CFGAnalysis {
    /// Total blocks
    pub total_blocks: usize,
    /// Total edges
    pub total_edges: usize,
    /// Cyclomatic complexity
    pub cyclomatic_complexity: usize,
    /// Loop count
    pub loop_count: usize,
    /// Unreachable blocks
    pub unreachable: Vec<u64>,
    /// Critical paths
    pub critical_paths: Vec<Path>,
}

// ============================================================================
// CFG BUILDER
// ============================================================================

/// CFG builder
pub struct CFGBuilder {
    /// Graphs
    graphs: BTreeMap<u64, ControlFlowGraph>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: CFGConfig,
    /// Statistics
    stats: CFGStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct CFGConfig {
    /// Detect unreachable
    pub detect_unreachable: bool,
    /// Simplify
    pub simplify: bool,
}

impl Default for CFGConfig {
    fn default() -> Self {
        Self {
            detect_unreachable: true,
            simplify: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct CFGStats {
    /// Graphs created
    pub graphs_created: u64,
    /// Blocks created
    pub blocks_created: u64,
    /// Analyses performed
    pub analyses: u64,
}

impl CFGBuilder {
    /// Create new builder
    pub fn new(config: CFGConfig) -> Self {
        Self {
            graphs: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: CFGStats::default(),
        }
    }

    /// Create graph
    pub fn create_graph(&mut self, name: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        // Create entry block
        let entry_id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let entry = BasicBlock {
            id: entry_id,
            label: "entry".into(),
            statements: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
            block_type: BlockType::Entry,
            start_line: 0,
            end_line: 0,
        };

        let mut blocks = BTreeMap::new();
        blocks.insert(entry_id, entry);

        let graph = ControlFlowGraph {
            id,
            name: name.into(),
            entry: entry_id,
            exits: Vec::new(),
            blocks,
            created: Timestamp::now(),
        };

        self.graphs.insert(id, graph);
        self.stats.graphs_created += 1;
        self.stats.blocks_created += 1;

        id
    }

    /// Add block
    pub fn add_block(
        &mut self,
        graph_id: u64,
        label: &str,
        block_type: BlockType,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let block = BasicBlock {
            id,
            label: label.into(),
            statements: Vec::new(),
            successors: Vec::new(),
            predecessors: Vec::new(),
            block_type,
            start_line: 0,
            end_line: 0,
        };

        if let Some(graph) = self.graphs.get_mut(&graph_id) {
            graph.blocks.insert(id, block);

            if block_type == BlockType::Exit {
                graph.exits.push(id);
            }
        }

        self.stats.blocks_created += 1;

        id
    }

    /// Add statement
    pub fn add_statement(&mut self, graph_id: u64, block_id: u64, stmt: Statement) {
        if let Some(graph) = self.graphs.get_mut(&graph_id) {
            if let Some(block) = graph.blocks.get_mut(&block_id) {
                block.statements.push(stmt);
            }
        }
    }

    /// Add edge
    pub fn add_edge(
        &mut self,
        graph_id: u64,
        from: u64,
        to: u64,
        edge_type: EdgeType,
        condition: Option<String>,
    ) {
        if let Some(graph) = self.graphs.get_mut(&graph_id) {
            // Add successor
            if let Some(from_block) = graph.blocks.get_mut(&from) {
                from_block.successors.push(Edge {
                    target: to,
                    edge_type,
                    condition,
                });
            }

            // Add predecessor
            if let Some(to_block) = graph.blocks.get_mut(&to) {
                if !to_block.predecessors.contains(&from) {
                    to_block.predecessors.push(from);
                }
            }
        }
    }

    /// Get graph
    pub fn get(&self, id: u64) -> Option<&ControlFlowGraph> {
        self.graphs.get(&id)
    }

    /// Analyze graph
    pub fn analyze(&mut self, graph_id: u64) -> Option<CFGAnalysis> {
        self.stats.analyses += 1;

        let graph = self.graphs.get(&graph_id)?;

        let total_blocks = graph.blocks.len();
        let total_edges: usize = graph.blocks.values()
            .map(|b| b.successors.len())
            .sum();

        // Cyclomatic complexity: E - N + 2P
        let cyclomatic_complexity = total_edges - total_blocks + 2;

        // Count loops
        let loop_count = graph.blocks.values()
            .flat_map(|b| b.successors.iter())
            .filter(|e| e.edge_type == EdgeType::LoopBack)
            .count();

        // Find unreachable
        let unreachable = if self.config.detect_unreachable {
            self.find_unreachable(graph)
        } else {
            Vec::new()
        };

        // Find critical paths
        let critical_paths = self.find_critical_paths(graph);

        Some(CFGAnalysis {
            total_blocks,
            total_edges,
            cyclomatic_complexity,
            loop_count,
            unreachable,
            critical_paths,
        })
    }

    fn find_unreachable(&self, graph: &ControlFlowGraph) -> Vec<u64> {
        let mut reachable = BTreeSet::new();
        let mut queue = vec![graph.entry];

        while let Some(id) = queue.pop() {
            if reachable.contains(&id) {
                continue;
            }
            reachable.insert(id);

            if let Some(block) = graph.blocks.get(&id) {
                for edge in &block.successors {
                    queue.push(edge.target);
                }
            }
        }

        graph.blocks.keys()
            .filter(|id| !reachable.contains(id))
            .copied()
            .collect()
    }

    fn find_critical_paths(&self, graph: &ControlFlowGraph) -> Vec<Path> {
        let mut paths = Vec::new();
        let mut current_path = Vec::new();
        let mut conditions = Vec::new();
        let mut visited = BTreeSet::new();

        self.dfs_paths(
            graph,
            graph.entry,
            &mut current_path,
            &mut conditions,
            &mut visited,
            &mut paths,
        );

        // Return longest paths
        paths.sort_by(|a, b| b.blocks.len().cmp(&a.blocks.len()));
        paths.into_iter().take(5).collect()
    }

    fn dfs_paths(
        &self,
        graph: &ControlFlowGraph,
        current: u64,
        path: &mut Vec<u64>,
        conditions: &mut Vec<String>,
        visited: &mut BTreeSet<u64>,
        paths: &mut Vec<Path>,
    ) {
        if path.len() > 100 {
            return; // Prevent infinite loops
        }

        path.push(current);
        visited.insert(current);

        let block = match graph.blocks.get(&current) {
            Some(b) => b,
            None => {
                path.pop();
                visited.remove(&current);
                return;
            }
        };

        // Check if exit
        if block.block_type == BlockType::Exit || block.successors.is_empty() {
            paths.push(Path {
                id: paths.len() as u64 + 1,
                blocks: path.clone(),
                conditions: conditions.clone(),
                feasible: true,
            });
        } else {
            for edge in &block.successors {
                if !visited.contains(&edge.target) {
                    if let Some(ref cond) = edge.condition {
                        conditions.push(cond.clone());
                    }

                    self.dfs_paths(graph, edge.target, path, conditions, visited, paths);

                    if edge.condition.is_some() {
                        conditions.pop();
                    }
                }
            }
        }

        path.pop();
        visited.remove(&current);
    }

    /// Get dominators
    pub fn dominators(&self, graph_id: u64) -> BTreeMap<u64, BTreeSet<u64>> {
        let mut doms = BTreeMap::new();

        let graph = match self.graphs.get(&graph_id) {
            Some(g) => g,
            None => return doms,
        };

        // Initialize
        let all_nodes: BTreeSet<u64> = graph.blocks.keys().copied().collect();

        for &id in &all_nodes {
            if id == graph.entry {
                let mut entry_dom = BTreeSet::new();
                entry_dom.insert(id);
                doms.insert(id, entry_dom);
            } else {
                doms.insert(id, all_nodes.clone());
            }
        }

        // Iterate until fixed point
        let mut changed = true;

        while changed {
            changed = false;

            for &id in &all_nodes {
                if id == graph.entry {
                    continue;
                }

                let block = match graph.blocks.get(&id) {
                    Some(b) => b,
                    None => continue,
                };

                let mut new_doms: Option<BTreeSet<u64>> = None;

                for &pred in &block.predecessors {
                    if let Some(pred_doms) = doms.get(&pred) {
                        match &mut new_doms {
                            None => new_doms = Some(pred_doms.clone()),
                            Some(nd) => {
                                *nd = nd.intersection(pred_doms).copied().collect();
                            }
                        }
                    }
                }

                let mut new_doms = new_doms.unwrap_or_default();
                new_doms.insert(id);

                if doms.get(&id) != Some(&new_doms) {
                    doms.insert(id, new_doms);
                    changed = true;
                }
            }
        }

        doms
    }

    /// Get statistics
    pub fn stats(&self) -> &CFGStats {
        &self.stats
    }
}

impl Default for CFGBuilder {
    fn default() -> Self {
        Self::new(CFGConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_graph() {
        let mut builder = CFGBuilder::default();

        let id = builder.create_graph("test");
        assert!(builder.get(id).is_some());
    }

    #[test]
    fn test_add_block() {
        let mut builder = CFGBuilder::default();

        let gid = builder.create_graph("test");
        let bid = builder.add_block(gid, "block1", BlockType::Normal);

        let graph = builder.get(gid).unwrap();
        assert!(graph.blocks.contains_key(&bid));
    }

    #[test]
    fn test_add_edge() {
        let mut builder = CFGBuilder::default();

        let gid = builder.create_graph("test");
        let graph = builder.get(gid).unwrap();
        let entry = graph.entry;

        let b1 = builder.add_block(gid, "block1", BlockType::Normal);

        builder.add_edge(gid, entry, b1, EdgeType::Unconditional, None);

        let graph = builder.get(gid).unwrap();
        let entry_block = graph.blocks.get(&entry).unwrap();
        assert_eq!(entry_block.successors.len(), 1);
    }

    #[test]
    fn test_analyze() {
        let mut builder = CFGBuilder::default();

        let gid = builder.create_graph("test");
        let graph = builder.get(gid).unwrap();
        let entry = graph.entry;

        let b1 = builder.add_block(gid, "block1", BlockType::Normal);
        let exit = builder.add_block(gid, "exit", BlockType::Exit);

        builder.add_edge(gid, entry, b1, EdgeType::Unconditional, None);
        builder.add_edge(gid, b1, exit, EdgeType::Unconditional, None);

        let analysis = builder.analyze(gid).unwrap();
        assert_eq!(analysis.total_blocks, 3);
        assert_eq!(analysis.total_edges, 2);
    }

    #[test]
    fn test_unreachable() {
        let mut builder = CFGBuilder::default();

        let gid = builder.create_graph("test");
        let exit = builder.add_block(gid, "exit", BlockType::Exit);
        let _unreachable = builder.add_block(gid, "unreachable", BlockType::Normal);

        let graph = builder.get(gid).unwrap();
        builder.add_edge(gid, graph.entry, exit, EdgeType::Unconditional, None);

        let analysis = builder.analyze(gid).unwrap();
        assert_eq!(analysis.unreachable.len(), 1);
    }

    #[test]
    fn test_dominators() {
        let mut builder = CFGBuilder::default();

        let gid = builder.create_graph("test");
        let graph = builder.get(gid).unwrap();
        let entry = graph.entry;

        let b1 = builder.add_block(gid, "b1", BlockType::Normal);
        builder.add_edge(gid, entry, b1, EdgeType::Unconditional, None);

        let doms = builder.dominators(gid);

        // Entry dominates itself
        assert!(doms.get(&entry).unwrap().contains(&entry));

        // Entry dominates b1
        assert!(doms.get(&b1).unwrap().contains(&entry));
    }
}
