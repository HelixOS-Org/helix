//! # Dependency Graph and Resolution
//!
//! This module implements a directed acyclic graph (DAG) for tracking and
//! resolving subsystem dependencies. It provides topological sorting, cycle
//! detection, and parallel execution planning.
//!
//! ## Dependency Graph Structure
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                        DEPENDENCY GRAPH                                      │
//! │                                                                              │
//! │                         ┌─────────┐                                          │
//! │                         │ firmware│                                          │
//! │                         └────┬────┘                                          │
//! │                              │                                               │
//! │                              ▼                                               │
//! │                         ┌─────────┐                                          │
//! │                         │boot_info│                                          │
//! │                         └────┬────┘                                          │
//! │                              │                                               │
//! │              ┌───────────────┼───────────────┐                               │
//! │              │               │               │                               │
//! │              ▼               ▼               ▼                               │
//! │         ┌────────┐     ┌────────┐      ┌────────┐                           │
//! │         │  pmm   │     │  cpu   │      │console │                           │
//! │         └───┬────┘     └───┬────┘      └────────┘                           │
//! │             │              │                                                 │
//! │             ▼              ▼                                                 │
//! │         ┌────────┐    ┌──────────┐                                          │
//! │         │  vmm   │    │interrupts│                                          │
//! │         └───┬────┘    └────┬─────┘                                          │
//! │             │              │                                                 │
//! │             └──────┬───────┘                                                 │
//! │                    │                                                         │
//! │                    ▼                                                         │
//! │               ┌────────┐                                                     │
//! │               │  heap  │                                                     │
//! │               └───┬────┘                                                     │
//! │                   │                                                          │
//! │       ┌───────────┼───────────┐                                              │
//! │       │           │           │                                              │
//! │       ▼           ▼           ▼                                              │
//! │  ┌─────────┐ ┌────────┐ ┌───────┐                                           │
//! │  │scheduler│ │  ipc   │ │timers │                                           │
//! │  └─────────┘ └────────┘ └───────┘                                           │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Topological Sort Algorithm
//!
//! Uses Kahn's algorithm with priority ordering within each "layer":
//!
//! 1. Compute in-degree for each node
//! 2. Add all zero in-degree nodes to queue (sorted by priority)
//! 3. Pop from queue, add to result, decrease in-degree of neighbors
//! 4. Repeat until queue is empty
//! 5. If not all nodes visited → cycle exists

use core::cmp::Ordering;
use core::fmt;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering as AtomicOrdering};

use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::InitPhase;
use crate::subsystem::{Dependency, DependencyKind, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::collections::{BTreeMap, BTreeSet, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// DEPENDENCY NODE
// =============================================================================

/// A node in the dependency graph
#[derive(Debug)]
pub struct DependencyNode {
    /// Subsystem ID
    pub id: SubsystemId,

    /// Subsystem name
    pub name: &'static str,

    /// Initialization phase
    pub phase: InitPhase,

    /// Priority within phase
    pub priority: i32,

    /// Outgoing edges (subsystems that depend on this one)
    pub dependents: Vec<SubsystemId>,

    /// Incoming edges (subsystems this one depends on)
    pub dependencies: Vec<DependencyEdge>,

    /// In-degree (number of unsatisfied dependencies)
    pub in_degree: AtomicU32,

    /// Whether this node has been processed
    pub processed: AtomicBool,

    /// Whether this node is essential
    pub essential: bool,
}

impl DependencyNode {
    /// Create new node from subsystem info
    pub fn from_info(info: &SubsystemInfo) -> Self {
        Self {
            id: info.id,
            name: info.name,
            phase: info.phase,
            priority: info.priority,
            dependents: Vec::new(),
            dependencies: Vec::new(),
            in_degree: AtomicU32::new(0),
            processed: AtomicBool::new(false),
            essential: info.essential,
        }
    }

    /// Get current in-degree
    pub fn current_in_degree(&self) -> u32 {
        self.in_degree.load(AtomicOrdering::Acquire)
    }

    /// Decrement in-degree, return new value
    pub fn decrement_in_degree(&self) -> u32 {
        self.in_degree
            .fetch_sub(1, AtomicOrdering::AcqRel)
            .saturating_sub(1)
    }

    /// Mark as processed
    pub fn mark_processed(&self) {
        self.processed.store(true, AtomicOrdering::Release);
    }

    /// Check if processed
    pub fn is_processed(&self) -> bool {
        self.processed.load(AtomicOrdering::Acquire)
    }

    /// Reset for re-processing
    pub fn reset(&self) {
        self.processed.store(false, AtomicOrdering::Release);
        self.in_degree
            .store(self.dependencies.len() as u32, AtomicOrdering::Release);
    }
}

impl PartialEq for DependencyNode {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for DependencyNode {}

impl PartialOrd for DependencyNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DependencyNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // First by phase, then by priority (descending), then by name
        self.phase
            .cmp(&other.phase)
            .then_with(|| other.priority.cmp(&self.priority))
            .then_with(|| self.name.cmp(other.name))
    }
}

// =============================================================================
// DEPENDENCY EDGE
// =============================================================================

/// An edge in the dependency graph
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Source node (the dependency)
    pub from: SubsystemId,

    /// Target node (the dependent)
    pub to: SubsystemId,

    /// Kind of dependency
    pub kind: DependencyKind,

    /// Minimum version constraint
    pub min_version: Option<(u16, u16, u16)>,

    /// Maximum version constraint
    pub max_version: Option<(u16, u16, u16)>,

    /// Whether this edge is satisfied
    pub satisfied: bool,
}

impl DependencyEdge {
    /// Create edge from dependency specification
    pub fn from_dependency(dep: &Dependency, dependent: SubsystemId) -> Self {
        Self {
            from: dep.id,
            to: dependent,
            kind: dep.kind,
            min_version: dep.min_version,
            max_version: dep.max_version,
            satisfied: false,
        }
    }

    /// Check if this is a hard dependency
    pub fn is_required(&self) -> bool {
        self.kind == DependencyKind::Required
    }

    /// Check if this is optional
    pub fn is_optional(&self) -> bool {
        matches!(self.kind, DependencyKind::Optional | DependencyKind::Weak)
    }
}

// =============================================================================
// DEPENDENCY GRAPH
// =============================================================================

/// The dependency graph for all subsystems
pub struct DependencyGraph {
    /// All nodes by ID
    nodes: BTreeMap<SubsystemId, DependencyNode>,

    /// Nodes grouped by phase
    by_phase: [Vec<SubsystemId>; 5],

    /// Cached topological order
    topo_order: Option<Vec<SubsystemId>>,

    /// Whether graph has been validated
    validated: bool,

    /// Detected conflicts
    conflicts: Vec<(SubsystemId, SubsystemId)>,

    /// Statistics
    stats: GraphStats,
}

/// Graph statistics
#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    /// Total nodes
    pub total_nodes: usize,
    /// Total edges
    pub total_edges: usize,
    /// Maximum depth
    pub max_depth: usize,
    /// Critical path length
    pub critical_path: usize,
    /// Parallelizable nodes per phase
    pub parallel_potential: [usize; 5],
}

impl DependencyGraph {
    /// Create empty graph
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            by_phase: Default::default(),
            topo_order: None,
            validated: false,
            conflicts: Vec::new(),
            stats: GraphStats::default(),
        }
    }

    /// Add a subsystem to the graph
    pub fn add_subsystem(&mut self, info: &SubsystemInfo) -> InitResult<()> {
        // Check for duplicate
        if self.nodes.contains_key(&info.id) {
            return Err(
                InitError::new(ErrorKind::AlreadyExists, "Subsystem already in graph")
                    .with_subsystem(info.id),
            );
        }

        // Create node
        let node = DependencyNode::from_info(info);

        // Add edges for dependencies
        let edges: Vec<DependencyEdge> = info
            .dependencies
            .iter()
            .map(|dep| DependencyEdge::from_dependency(dep, info.id))
            .collect();

        // Insert node
        self.nodes.insert(info.id, node);

        // Update phase grouping
        self.by_phase[info.phase as usize].push(info.id);

        // Add dependency edges
        for edge in edges {
            self.add_edge(edge)?;
        }

        // Invalidate cached order
        self.topo_order = None;
        self.validated = false;
        self.stats.total_nodes = self.nodes.len();

        Ok(())
    }

    /// Add an edge to the graph
    fn add_edge(&mut self, edge: DependencyEdge) -> InitResult<()> {
        // Get the dependent node
        if let Some(node) = self.nodes.get_mut(&edge.to) {
            node.dependencies.push(edge.clone());
            node.in_degree.fetch_add(1, AtomicOrdering::Release);
        }

        // Add to dependents list of source
        if let Some(source) = self.nodes.get_mut(&edge.from) {
            source.dependents.push(edge.to);
        }

        self.stats.total_edges += 1;
        Ok(())
    }

    /// Remove a subsystem from the graph
    pub fn remove_subsystem(&mut self, id: SubsystemId) -> InitResult<()> {
        // Remove from nodes
        let node = self.nodes.remove(&id).ok_or_else(|| {
            InitError::new(ErrorKind::NotFound, "Subsystem not in graph").with_subsystem(id)
        })?;

        // Remove from phase grouping
        self.by_phase[node.phase as usize].retain(|&x| x != id);

        // Remove edges pointing to this node
        for dep_id in &node.dependents {
            if let Some(dep_node) = self.nodes.get_mut(dep_id) {
                dep_node.dependencies.retain(|e| e.from != id);
                dep_node.in_degree.fetch_sub(1, AtomicOrdering::Release);
            }
        }

        // Remove edges from dependencies
        for edge in &node.dependencies {
            if let Some(src_node) = self.nodes.get_mut(&edge.from) {
                src_node.dependents.retain(|&x| x != id);
            }
        }

        // Invalidate cache
        self.topo_order = None;
        self.validated = false;

        Ok(())
    }

    /// Validate the graph (check for cycles, missing deps, conflicts)
    pub fn validate(&mut self) -> InitResult<()> {
        // Check for missing required dependencies
        let mut missing = Vec::new();
        for node in self.nodes.values() {
            for edge in &node.dependencies {
                if edge.is_required() && !self.nodes.contains_key(&edge.from) {
                    missing.push((node.id, edge.from, node.name));
                }
            }
        }

        if !missing.is_empty() {
            let msg = alloc::format!(
                "Missing dependencies: {}",
                missing
                    .iter()
                    .map(|(_, _, name)| *name)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            return Err(InitError::new(
                ErrorKind::MissingDependency,
                "Missing required dependencies",
            )
            .with_details(msg));
        }

        // Check for cycles using topological sort
        match self.compute_topological_order() {
            Ok(order) => {
                self.topo_order = Some(order);
            },
            Err(e) => {
                return Err(e);
            },
        }

        // Check for conflicts
        self.check_conflicts()?;

        // Compute statistics
        self.compute_stats();

        self.validated = true;
        Ok(())
    }

    /// Check for conflicting subsystems
    fn check_conflicts(&mut self) -> InitResult<()> {
        self.conflicts.clear();

        for node in self.nodes.values() {
            for edge in &node.dependencies {
                if edge.kind == DependencyKind::Conflict {
                    if self.nodes.contains_key(&edge.from) {
                        self.conflicts.push((node.id, edge.from));
                    }
                }
            }
        }

        if !self.conflicts.is_empty() {
            return Err(InitError::new(
                ErrorKind::DependencyFailed,
                "Conflicting subsystems detected",
            ));
        }

        Ok(())
    }

    /// Compute topological order using Kahn's algorithm
    fn compute_topological_order(&self) -> InitResult<Vec<SubsystemId>> {
        let mut result = Vec::with_capacity(self.nodes.len());
        let mut in_degree: BTreeMap<SubsystemId, u32> = BTreeMap::new();

        // Initialize in-degrees
        for (id, node) in &self.nodes {
            // Only count required dependencies
            let required_count = node
                .dependencies
                .iter()
                .filter(|e| e.is_required() && self.nodes.contains_key(&e.from))
                .count() as u32;
            in_degree.insert(*id, required_count);
        }

        // Priority queue (sorted by phase, then priority)
        let mut queue: Vec<SubsystemId> = Vec::new();

        // Add all nodes with zero in-degree
        for (&id, &degree) in &in_degree {
            if degree == 0 {
                queue.push(id);
            }
        }

        while !queue.is_empty() {
            // Sort by phase and priority
            queue.sort_by(|a, b| {
                let node_a = &self.nodes[a];
                let node_b = &self.nodes[b];
                node_a.cmp(node_b)
            });

            // Take the first (highest priority)
            let current = queue.remove(0);
            result.push(current);

            // Decrease in-degree of dependents
            if let Some(node) = self.nodes.get(&current) {
                for &dependent in &node.dependents {
                    if let Some(degree) = in_degree.get_mut(&dependent) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0
                            && !result.contains(&dependent)
                            && !queue.contains(&dependent)
                        {
                            queue.push(dependent);
                        }
                    }
                }
            }
        }

        // Check for cycle
        if result.len() != self.nodes.len() {
            // Find nodes in cycle
            let not_visited: Vec<_> = self
                .nodes
                .keys()
                .filter(|id| !result.contains(id))
                .copied()
                .collect();

            let cycle_names: Vec<&str> = not_visited
                .iter()
                .filter_map(|id| self.nodes.get(id))
                .map(|n| n.name)
                .collect();

            return Err(InitError::new(
                ErrorKind::CircularDependency,
                "Circular dependency detected",
            )
            .with_details(alloc::format!("Involved: {}", cycle_names.join(" -> "))));
        }

        Ok(result)
    }

    /// Compute graph statistics
    fn compute_stats(&mut self) {
        self.stats.total_nodes = self.nodes.len();
        self.stats.total_edges = self.nodes.values().map(|n| n.dependencies.len()).sum();

        // Compute max depth and parallel potential per phase
        for (phase_idx, phase_nodes) in self.by_phase.iter().enumerate() {
            // Nodes with zero in-phase dependencies can run in parallel
            let parallel = phase_nodes
                .iter()
                .filter(|id| {
                    self.nodes
                        .get(id)
                        .map(|n| {
                            n.dependencies.iter().filter(|e| e.is_required()).all(|e| {
                                self.nodes
                                    .get(&e.from)
                                    .map(|src| src.phase != n.phase)
                                    .unwrap_or(true)
                            })
                        })
                        .unwrap_or(false)
                })
                .count();
            self.stats.parallel_potential[phase_idx] = parallel;
        }

        // Compute critical path (longest path through the graph)
        if let Some(ref order) = self.topo_order {
            let mut distances: BTreeMap<SubsystemId, usize> = BTreeMap::new();

            for &id in order {
                let max_dep = self
                    .nodes
                    .get(&id)
                    .map(|n| {
                        n.dependencies
                            .iter()
                            .filter(|e| e.is_required())
                            .filter_map(|e| distances.get(&e.from))
                            .max()
                            .copied()
                            .unwrap_or(0)
                    })
                    .unwrap_or(0);
                distances.insert(id, max_dep + 1);
            }

            self.stats.critical_path = distances.values().max().copied().unwrap_or(0);
            self.stats.max_depth = self.stats.critical_path;
        }
    }

    /// Get topological order
    pub fn topological_order(&self) -> Option<&[SubsystemId]> {
        self.topo_order.as_deref()
    }

    /// Get order for a specific phase
    pub fn phase_order(&self, phase: InitPhase) -> Vec<SubsystemId> {
        if let Some(ref order) = self.topo_order {
            order
                .iter()
                .filter(|id| {
                    self.nodes
                        .get(id)
                        .map(|n| n.phase == phase)
                        .unwrap_or(false)
                })
                .copied()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get nodes that can be initialized in parallel
    ///
    /// Returns nodes whose all required dependencies are satisfied.
    pub fn get_ready_nodes(&self, satisfied: &BTreeSet<SubsystemId>) -> Vec<SubsystemId> {
        self.nodes
            .values()
            .filter(|node| !node.is_processed())
            .filter(|node| {
                node.dependencies
                    .iter()
                    .filter(|e| e.is_required())
                    .all(|e| satisfied.contains(&e.from) || !self.nodes.contains_key(&e.from))
            })
            .map(|n| n.id)
            .collect()
    }

    /// Get parallel execution batches for a phase
    pub fn get_parallel_batches(&self, phase: InitPhase) -> Vec<Vec<SubsystemId>> {
        let mut batches = Vec::new();
        let mut satisfied: BTreeSet<SubsystemId> = BTreeSet::new();

        // Include nodes from previous phases as satisfied
        for p in 0..(phase as usize) {
            for id in &self.by_phase[p] {
                satisfied.insert(*id);
            }
        }

        let phase_nodes: BTreeSet<SubsystemId> =
            self.by_phase[phase as usize].iter().copied().collect();

        let mut remaining: BTreeSet<SubsystemId> = phase_nodes.clone();

        while !remaining.is_empty() {
            // Find nodes whose dependencies are all satisfied
            let ready: Vec<SubsystemId> = remaining
                .iter()
                .filter(|id| {
                    self.nodes
                        .get(id)
                        .map(|n| {
                            n.dependencies.iter().filter(|e| e.is_required()).all(|e| {
                                satisfied.contains(&e.from) || !phase_nodes.contains(&e.from)
                            })
                        })
                        .unwrap_or(false)
                })
                .copied()
                .collect();

            if ready.is_empty() {
                // No progress possible - shouldn't happen if graph is valid
                break;
            }

            // Sort by priority
            let mut batch = ready;
            batch.sort_by(|a, b| {
                let na = self.nodes.get(a);
                let nb = self.nodes.get(b);
                match (na, nb) {
                    (Some(a), Some(b)) => b.priority.cmp(&a.priority),
                    _ => Ordering::Equal,
                }
            });

            // Mark as satisfied
            for id in &batch {
                satisfied.insert(*id);
                remaining.remove(id);
            }

            batches.push(batch);
        }

        batches
    }

    /// Get a node by ID
    pub fn get_node(&self, id: SubsystemId) -> Option<&DependencyNode> {
        self.nodes.get(&id)
    }

    /// Get all nodes
    pub fn nodes(&self) -> impl Iterator<Item = &DependencyNode> {
        self.nodes.values()
    }

    /// Get nodes in a phase
    pub fn nodes_in_phase(&self, phase: InitPhase) -> Vec<&DependencyNode> {
        self.by_phase[phase as usize]
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &GraphStats {
        &self.stats
    }

    /// Check if graph is validated
    pub fn is_validated(&self) -> bool {
        self.validated
    }

    /// Get number of nodes
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Reset all nodes for re-processing
    pub fn reset(&self) {
        for node in self.nodes.values() {
            node.reset();
        }
    }

    /// Get all dependencies of a subsystem
    pub fn get_dependencies(&self, id: SubsystemId) -> Vec<SubsystemId> {
        self.nodes
            .get(&id)
            .map(|n| n.dependencies.iter().map(|e| e.from).collect())
            .unwrap_or_default()
    }

    /// Get all dependents of a subsystem
    pub fn get_dependents(&self, id: SubsystemId) -> Vec<SubsystemId> {
        self.nodes
            .get(&id)
            .map(|n| n.dependents.clone())
            .unwrap_or_default()
    }

    /// Check if one subsystem depends on another (transitively)
    pub fn depends_on(&self, subsystem: SubsystemId, dependency: SubsystemId) -> bool {
        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(subsystem);

        while let Some(current) = queue.pop_front() {
            if current == dependency {
                return true;
            }

            if visited.insert(current) {
                if let Some(node) = self.nodes.get(&current) {
                    for edge in &node.dependencies {
                        if edge.is_required() {
                            queue.push_back(edge.from);
                        }
                    }
                }
            }
        }

        false
    }

    /// Generate DOT format for visualization
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph dependencies {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box];\n\n");

        // Group by phase
        for (phase_idx, phase) in InitPhase::all().iter().enumerate() {
            dot.push_str(&alloc::format!("  subgraph cluster_{} {{\n", phase_idx));
            dot.push_str(&alloc::format!("    label=\"{}\";\n", phase.name()));

            for id in &self.by_phase[phase_idx] {
                if let Some(node) = self.nodes.get(id) {
                    dot.push_str(&alloc::format!("    \"{}\";\n", node.name));
                }
            }

            dot.push_str("  }\n\n");
        }

        // Edges
        for node in self.nodes.values() {
            for edge in &node.dependencies {
                if let Some(from_node) = self.nodes.get(&edge.from) {
                    let style = match edge.kind {
                        DependencyKind::Required => "",
                        DependencyKind::Optional => " [style=dashed]",
                        DependencyKind::Weak => " [style=dotted]",
                        DependencyKind::Conflict => " [style=bold,color=red]",
                    };
                    dot.push_str(&alloc::format!(
                        "  \"{}\" -> \"{}\"{};\n",
                        from_node.name,
                        node.name,
                        style
                    ));
                }
            }
        }

        dot.push_str("}\n");
        dot
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DependencyGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DependencyGraph")
            .field("nodes", &self.nodes.len())
            .field("validated", &self.validated)
            .field("stats", &self.stats)
            .finish()
    }
}

// =============================================================================
// DEPENDENCY RESOLVER
// =============================================================================

/// Resolves dependencies and determines initialization order
pub struct DependencyResolver {
    /// The graph
    graph: DependencyGraph,

    /// Current position in order
    position: usize,

    /// Satisfied subsystems
    satisfied: BTreeSet<SubsystemId>,
}

impl DependencyResolver {
    /// Create resolver from graph
    pub fn new(graph: DependencyGraph) -> InitResult<Self> {
        let mut g = graph;
        g.validate()?;

        Ok(Self {
            graph: g,
            position: 0,
            satisfied: BTreeSet::new(),
        })
    }

    /// Get next subsystem to initialize
    pub fn next(&mut self) -> Option<SubsystemId> {
        let order = self.graph.topological_order()?;
        if self.position < order.len() {
            let id = order[self.position];
            self.position += 1;
            Some(id)
        } else {
            None
        }
    }

    /// Mark a subsystem as satisfied
    pub fn mark_satisfied(&mut self, id: SubsystemId) {
        self.satisfied.insert(id);
    }

    /// Mark a subsystem as failed
    pub fn mark_failed(&mut self, id: SubsystemId) -> Vec<SubsystemId> {
        // Get all dependents that will also fail
        let mut affected = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(id);

        while let Some(current) = queue.pop_front() {
            affected.push(current);
            for dependent in self.graph.get_dependents(current) {
                if !affected.contains(&dependent) {
                    queue.push_back(dependent);
                }
            }
        }

        affected
    }

    /// Check if all dependencies are satisfied
    pub fn is_ready(&self, id: SubsystemId) -> bool {
        self.graph
            .get_node(id)
            .map(|n| {
                n.dependencies
                    .iter()
                    .filter(|e| e.is_required())
                    .all(|e| self.satisfied.contains(&e.from))
            })
            .unwrap_or(false)
    }

    /// Get subsystems ready for parallel initialization
    pub fn get_ready(&self) -> Vec<SubsystemId> {
        self.graph.get_ready_nodes(&self.satisfied)
    }

    /// Get the underlying graph
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Reset resolver
    pub fn reset(&mut self) {
        self.position = 0;
        self.satisfied.clear();
        self.graph.reset();
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase::PhaseCapabilities;

    fn make_info(
        name: &'static str,
        phase: InitPhase,
        deps: &'static [Dependency],
    ) -> SubsystemInfo {
        SubsystemInfo {
            id: SubsystemId::from_name(name),
            name,
            description: "",
            version: (1, 0, 0),
            phase,
            priority: 0,
            dependencies: deps,
            provides: PhaseCapabilities::empty(),
            requires: PhaseCapabilities::empty(),
            essential: false,
            hot_reloadable: false,
            suspendable: true,
            estimated_init_us: 1000,
            timeout_us: 10_000_000,
            author: "",
            license: "",
        }
    }

    #[test]
    fn test_simple_graph() {
        let mut graph = DependencyGraph::new();

        static DEPS_A: [Dependency; 0] = [];
        static DEPS_B: [Dependency; 1] = [Dependency::required("a")];

        let info_a = make_info("a", InitPhase::Boot, &DEPS_A);
        let info_b = make_info("b", InitPhase::Boot, &DEPS_B);

        graph.add_subsystem(&info_a).unwrap();
        graph.add_subsystem(&info_b).unwrap();

        graph.validate().unwrap();

        let order = graph.topological_order().unwrap();
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], SubsystemId::from_name("a"));
        assert_eq!(order[1], SubsystemId::from_name("b"));
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        static DEPS_A: [Dependency; 1] = [Dependency::required("b")];
        static DEPS_B: [Dependency; 1] = [Dependency::required("a")];

        let info_a = make_info("a", InitPhase::Boot, &DEPS_A);
        let info_b = make_info("b", InitPhase::Boot, &DEPS_B);

        graph.add_subsystem(&info_a).unwrap();
        graph.add_subsystem(&info_b).unwrap();

        let result = graph.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), ErrorKind::CircularDependency);
    }

    #[test]
    fn test_parallel_batches() {
        let mut graph = DependencyGraph::new();

        static NO_DEPS: [Dependency; 0] = [];
        static DEPS_D: [Dependency; 2] = [Dependency::required("a"), Dependency::required("b")];

        // a, b, c can run in parallel
        // d depends on a and b
        graph
            .add_subsystem(&make_info("a", InitPhase::Early, &NO_DEPS))
            .unwrap();
        graph
            .add_subsystem(&make_info("b", InitPhase::Early, &NO_DEPS))
            .unwrap();
        graph
            .add_subsystem(&make_info("c", InitPhase::Early, &NO_DEPS))
            .unwrap();
        graph
            .add_subsystem(&make_info("d", InitPhase::Early, &DEPS_D))
            .unwrap();

        graph.validate().unwrap();

        let batches = graph.get_parallel_batches(InitPhase::Early);

        // First batch: a, b, c
        assert_eq!(batches[0].len(), 3);
        // Second batch: d
        assert_eq!(batches[1].len(), 1);
        assert!(batches[1].contains(&SubsystemId::from_name("d")));
    }

    #[test]
    fn test_optional_dependency() {
        let mut graph = DependencyGraph::new();

        static NO_DEPS: [Dependency; 0] = [];
        static OPT_DEPS: [Dependency; 1] = [Dependency::optional("missing")];

        graph
            .add_subsystem(&make_info("a", InitPhase::Boot, &NO_DEPS))
            .unwrap();
        graph
            .add_subsystem(&make_info("b", InitPhase::Boot, &OPT_DEPS))
            .unwrap();

        // Should not fail even though "missing" doesn't exist
        graph.validate().unwrap();
    }

    #[test]
    fn test_depends_on_transitive() {
        let mut graph = DependencyGraph::new();

        static NO_DEPS: [Dependency; 0] = [];
        static DEPS_B: [Dependency; 1] = [Dependency::required("a")];
        static DEPS_C: [Dependency; 1] = [Dependency::required("b")];

        graph
            .add_subsystem(&make_info("a", InitPhase::Boot, &NO_DEPS))
            .unwrap();
        graph
            .add_subsystem(&make_info("b", InitPhase::Boot, &DEPS_B))
            .unwrap();
        graph
            .add_subsystem(&make_info("c", InitPhase::Boot, &DEPS_C))
            .unwrap();

        graph.validate().unwrap();

        assert!(graph.depends_on(SubsystemId::from_name("c"), SubsystemId::from_name("a")));
        assert!(!graph.depends_on(SubsystemId::from_name("a"), SubsystemId::from_name("c")));
    }
}
