//! # Bridge Dependency Tracker
//!
//! Track syscall dependencies and ordering:
//! - Dependency graph construction
//! - Topological ordering
//! - Cycle detection
//! - Critical path analysis
//! - Dependency-aware batching

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// DEPENDENCY TYPES
// ============================================================================

/// Dependency type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// Data dependency (output of A feeds B)
    Data,
    /// Control dependency (result of A determines B)
    Control,
    /// Resource dependency (A and B share resource)
    Resource,
    /// Order dependency (A must precede B)
    Order,
    /// Anti-dependency (B writes what A reads)
    Anti,
}

/// Dependency strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DependencyStrength {
    /// Weak (hint)
    Weak,
    /// Normal
    Normal,
    /// Strong (required)
    Strong,
    /// Absolute (must not violate)
    Absolute,
}

/// Dependency edge
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Source node
    pub from: u64,
    /// Target node
    pub to: u64,
    /// Type
    pub dep_type: DependencyType,
    /// Strength
    pub strength: DependencyStrength,
    /// Latency cost (ns)
    pub latency_ns: u64,
}

// ============================================================================
// DEPENDENCY NODE
// ============================================================================

/// Node in dependency graph
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// Node id (syscall id or batch id)
    pub id: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Process id
    pub pid: u64,
    /// Execution cost estimate (ns)
    pub cost_ns: u64,
    /// Incoming edges
    pub predecessors: Vec<u64>,
    /// Outgoing edges
    pub successors: Vec<u64>,
    /// Earliest start time
    pub earliest_start: u64,
    /// Latest start time
    pub latest_start: u64,
    /// Completed?
    pub completed: bool,
}

impl DependencyNode {
    pub fn new(id: u64, syscall_nr: u32, pid: u64, cost_ns: u64) -> Self {
        Self {
            id,
            syscall_nr,
            pid,
            cost_ns,
            predecessors: Vec::new(),
            successors: Vec::new(),
            earliest_start: 0,
            latest_start: u64::MAX,
            completed: false,
        }
    }

    /// Slack (latest - earliest)
    pub fn slack(&self) -> u64 {
        self.latest_start.saturating_sub(self.earliest_start)
    }

    /// Is on critical path?
    pub fn is_critical(&self) -> bool {
        self.slack() == 0
    }

    /// Can start? (all predecessors completed)
    pub fn can_start(&self, completed: &[u64]) -> bool {
        self.predecessors.iter().all(|p| completed.contains(p))
    }
}

// ============================================================================
// DEPENDENCY GRAPH
// ============================================================================

/// Dependency graph
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Nodes
    pub nodes: BTreeMap<u64, DependencyNode>,
    /// Edges
    pub edges: Vec<DependencyEdge>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
        }
    }

    /// Add node
    pub fn add_node(&mut self, node: DependencyNode) {
        self.nodes.insert(node.id, node);
    }

    /// Add edge
    pub fn add_edge(&mut self, edge: DependencyEdge) {
        if let Some(from) = self.nodes.get_mut(&edge.from) {
            if !from.successors.contains(&edge.to) {
                from.successors.push(edge.to);
            }
        }
        if let Some(to) = self.nodes.get_mut(&edge.to) {
            if !to.predecessors.contains(&edge.from) {
                to.predecessors.push(edge.from);
            }
        }
        self.edges.push(edge);
    }

    /// Topological sort (Kahn's algorithm)
    pub fn topological_sort(&self) -> Option<Vec<u64>> {
        let mut in_degree: BTreeMap<u64, usize> = BTreeMap::new();
        for (&id, node) in &self.nodes {
            in_degree.entry(id).or_insert(0);
            for &succ in &node.successors {
                *in_degree.entry(succ).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<u64> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        queue.sort();

        let mut result = Vec::new();

        while let Some(node_id) = queue.pop() {
            result.push(node_id);
            if let Some(node) = self.nodes.get(&node_id) {
                for &succ in &node.successors {
                    if let Some(deg) = in_degree.get_mut(&succ) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(succ);
                            queue.sort();
                        }
                    }
                }
            }
        }

        if result.len() == self.nodes.len() {
            Some(result)
        } else {
            None // cycle detected
        }
    }

    /// Detect cycles (returns true if cycle exists)
    pub fn has_cycle(&self) -> bool {
        self.topological_sort().is_none()
    }

    /// Compute earliest start times (forward pass)
    pub fn compute_earliest_starts(&mut self) {
        if let Some(order) = self.topological_sort() {
            for &id in &order {
                let earliest = if let Some(node) = self.nodes.get(&id) {
                    let mut max_pred = 0u64;
                    for &pred_id in &node.predecessors {
                        if let Some(pred) = self.nodes.get(&pred_id) {
                            let finish = pred.earliest_start + pred.cost_ns;
                            if finish > max_pred {
                                max_pred = finish;
                            }
                        }
                    }
                    max_pred
                } else {
                    0
                };
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.earliest_start = earliest;
                }
            }
        }
    }

    /// Compute latest start times (backward pass)
    pub fn compute_latest_starts(&mut self) {
        if let Some(order) = self.topological_sort() {
            // Find makespan
            let makespan: u64 = self
                .nodes
                .values()
                .map(|n| n.earliest_start + n.cost_ns)
                .max()
                .unwrap_or(0);

            for &id in order.iter().rev() {
                let latest = if let Some(node) = self.nodes.get(&id) {
                    if node.successors.is_empty() {
                        makespan - node.cost_ns
                    } else {
                        let mut min_succ = u64::MAX;
                        for &succ_id in &node.successors {
                            if let Some(succ) = self.nodes.get(&succ_id) {
                                if succ.latest_start < min_succ {
                                    min_succ = succ.latest_start;
                                }
                            }
                        }
                        min_succ.saturating_sub(node.cost_ns)
                    }
                } else {
                    0
                };
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.latest_start = latest;
                }
            }
        }
    }

    /// Critical path
    pub fn critical_path(&mut self) -> Vec<u64> {
        self.compute_earliest_starts();
        self.compute_latest_starts();

        let mut path: Vec<u64> = self
            .nodes
            .values()
            .filter(|n| n.is_critical())
            .map(|n| n.id)
            .collect();
        path.sort_by_key(|&id| self.nodes.get(&id).map(|n| n.earliest_start).unwrap_or(0));
        path
    }

    /// Makespan (total time)
    pub fn makespan(&self) -> u64 {
        self.nodes
            .values()
            .map(|n| n.earliest_start + n.cost_ns)
            .max()
            .unwrap_or(0)
    }

    /// Ready nodes (predecessors all completed)
    pub fn ready_nodes(&self) -> Vec<u64> {
        let completed: Vec<u64> = self
            .nodes
            .values()
            .filter(|n| n.completed)
            .map(|n| n.id)
            .collect();

        self.nodes
            .values()
            .filter(|n| !n.completed && n.can_start(&completed))
            .map(|n| n.id)
            .collect()
    }

    /// Node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

// ============================================================================
// DEPENDENCY TRACKER
// ============================================================================

/// Dependency tracker stats
#[derive(Debug, Clone, Default)]
pub struct DependencyTrackerStats {
    /// Graphs tracked
    pub graphs: usize,
    /// Total nodes
    pub total_nodes: u64,
    /// Cycles detected
    pub cycles_detected: u64,
    /// Critical path length (avg ns)
    pub avg_critical_path_ns: f64,
}

/// Bridge dependency tracker
pub struct BridgeDependencyTracker {
    /// Active graphs
    graphs: BTreeMap<u64, DependencyGraph>,
    /// Next graph id
    next_id: u64,
    /// Stats
    stats: DependencyTrackerStats,
}

impl BridgeDependencyTracker {
    pub fn new() -> Self {
        Self {
            graphs: BTreeMap::new(),
            next_id: 1,
            stats: DependencyTrackerStats::default(),
        }
    }

    /// Create new dependency graph
    pub fn create_graph(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.graphs.insert(id, DependencyGraph::new());
        self.stats.graphs = self.graphs.len();
        id
    }

    /// Add node to graph
    pub fn add_node(&mut self, graph_id: u64, node: DependencyNode) {
        if let Some(g) = self.graphs.get_mut(&graph_id) {
            g.add_node(node);
            self.stats.total_nodes += 1;
        }
    }

    /// Add edge
    pub fn add_edge(&mut self, graph_id: u64, edge: DependencyEdge) -> bool {
        if let Some(g) = self.graphs.get_mut(&graph_id) {
            g.add_edge(edge);
            if g.has_cycle() {
                self.stats.cycles_detected += 1;
                return false;
            }
            return true;
        }
        false
    }

    /// Get execution order
    pub fn execution_order(&self, graph_id: u64) -> Option<Vec<u64>> {
        self.graphs.get(&graph_id)?.topological_sort()
    }

    /// Get critical path
    pub fn critical_path(&mut self, graph_id: u64) -> Vec<u64> {
        if let Some(g) = self.graphs.get_mut(&graph_id) {
            g.critical_path()
        } else {
            Vec::new()
        }
    }

    /// Remove completed graph
    pub fn remove_graph(&mut self, graph_id: u64) {
        self.graphs.remove(&graph_id);
        self.stats.graphs = self.graphs.len();
    }

    /// Stats
    pub fn stats(&self) -> &DependencyTrackerStats {
        &self.stats
    }
}
