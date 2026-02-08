//! # Coop Dependency Tracker
//!
//! Track and manage dependencies between cooperating processes:
//! - Dependency graph construction
//! - Circular dependency detection
//! - Critical path analysis
//! - Dependency-aware scheduling hints
//! - Cascading failure prediction

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// DEPENDENCY TYPES
// ============================================================================

/// Dependency type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopDepType {
    /// Data dependency (produces/consumes)
    Data,
    /// Service dependency (calls/provides)
    Service,
    /// Resource dependency (shares resource)
    Resource,
    /// Ordering dependency (must run before)
    Order,
    /// Communication dependency (IPC)
    Communication,
    /// Lock dependency
    Lock,
}

/// Dependency strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopDepStrength {
    /// Optional (can proceed without)
    Optional,
    /// Preferred (degrades without)
    Preferred,
    /// Required (blocks without)
    Required,
    /// Critical (fails without)
    Critical,
}

/// Dependency state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopDepState {
    /// Active and healthy
    Active,
    /// Degraded (latency issues)
    Degraded,
    /// Broken (target unavailable)
    Broken,
    /// Stale (not used recently)
    Stale,
}

// ============================================================================
// DEPENDENCY EDGE
// ============================================================================

/// Dependency edge
#[derive(Debug, Clone)]
pub struct CoopDepEdge {
    /// Source PID
    pub from_pid: u64,
    /// Target PID
    pub to_pid: u64,
    /// Dependency type
    pub dep_type: CoopDepType,
    /// Strength
    pub strength: CoopDepStrength,
    /// State
    pub state: CoopDepState,
    /// Latency (ns, EMA)
    pub latency_ema_ns: f64,
    /// Invocation count
    pub invocations: u64,
    /// Failure count
    pub failures: u64,
    /// Last seen (ns)
    pub last_seen_ns: u64,
}

impl CoopDepEdge {
    pub fn new(from: u64, to: u64, dep_type: CoopDepType, strength: CoopDepStrength, now: u64) -> Self {
        Self {
            from_pid: from,
            to_pid: to,
            dep_type,
            strength,
            state: CoopDepState::Active,
            latency_ema_ns: 0.0,
            invocations: 0,
            failures: 0,
            last_seen_ns: now,
        }
    }

    /// Record invocation
    pub fn record_invocation(&mut self, latency_ns: u64, success: bool, now: u64) {
        self.invocations += 1;
        self.latency_ema_ns = 0.9 * self.latency_ema_ns + 0.1 * latency_ns as f64;
        self.last_seen_ns = now;
        if !success {
            self.failures += 1;
        }
    }

    /// Failure rate
    pub fn failure_rate(&self) -> f64 {
        if self.invocations == 0 {
            return 0.0;
        }
        self.failures as f64 / self.invocations as f64
    }

    /// Health score (0..1)
    pub fn health(&self) -> f64 {
        let fail_penalty = self.failure_rate();
        (1.0 - fail_penalty).max(0.0)
    }

    /// Is stale
    pub fn is_stale(&self, now: u64, timeout_ns: u64) -> bool {
        now.saturating_sub(self.last_seen_ns) > timeout_ns
    }
}

// ============================================================================
// DEPENDENCY GRAPH
// ============================================================================

/// Dependency graph
#[derive(Debug)]
pub struct CoopDepGraph {
    /// Edges: (from, to) -> edge
    edges: BTreeMap<(u64, u64), CoopDepEdge>,
    /// Adjacency: pid -> outgoing targets
    outgoing: BTreeMap<u64, Vec<u64>>,
    /// Reverse adjacency: pid -> incoming sources
    incoming: BTreeMap<u64, Vec<u64>>,
}

impl CoopDepGraph {
    pub fn new() -> Self {
        Self {
            edges: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
        }
    }

    /// Add edge
    pub fn add_edge(&mut self, edge: CoopDepEdge) {
        let from = edge.from_pid;
        let to = edge.to_pid;
        self.edges.insert((from, to), edge);

        let out = self.outgoing.entry(from).or_insert_with(Vec::new);
        if !out.contains(&to) {
            out.push(to);
        }

        let inc = self.incoming.entry(to).or_insert_with(Vec::new);
        if !inc.contains(&from) {
            inc.push(from);
        }
    }

    /// Remove edge
    pub fn remove_edge(&mut self, from: u64, to: u64) {
        self.edges.remove(&(from, to));
        if let Some(out) = self.outgoing.get_mut(&from) {
            out.retain(|&x| x != to);
        }
        if let Some(inc) = self.incoming.get_mut(&to) {
            inc.retain(|&x| x != from);
        }
    }

    /// Get edge
    pub fn get_edge(&self, from: u64, to: u64) -> Option<&CoopDepEdge> {
        self.edges.get(&(from, to))
    }

    /// Get edge mut
    pub fn get_edge_mut(&mut self, from: u64, to: u64) -> Option<&mut CoopDepEdge> {
        self.edges.get_mut(&(from, to))
    }

    /// Dependencies of a process
    pub fn dependencies_of(&self, pid: u64) -> Vec<u64> {
        self.outgoing.get(&pid).cloned().unwrap_or_default()
    }

    /// Dependents on a process
    pub fn dependents_on(&self, pid: u64) -> Vec<u64> {
        self.incoming.get(&pid).cloned().unwrap_or_default()
    }

    /// Detect circular dependencies using DFS
    pub fn detect_cycles(&self) -> Vec<Vec<u64>> {
        let mut cycles = Vec::new();
        let mut visited = BTreeMap::new();
        let mut stack = Vec::new();

        let nodes: Vec<u64> = self.outgoing.keys().cloned().collect();
        for &node in &nodes {
            if !visited.contains_key(&node) {
                self.dfs_cycle(node, &mut visited, &mut stack, &mut cycles);
            }
        }
        cycles
    }

    fn dfs_cycle(
        &self,
        node: u64,
        visited: &mut BTreeMap<u64, u8>, // 0=in-progress, 1=done
        stack: &mut Vec<u64>,
        cycles: &mut Vec<Vec<u64>>,
    ) {
        visited.insert(node, 0);
        stack.push(node);

        if let Some(neighbors) = self.outgoing.get(&node) {
            for &next in neighbors {
                match visited.get(&next) {
                    None => {
                        self.dfs_cycle(next, visited, stack, cycles);
                    }
                    Some(&0) => {
                        // Found cycle
                        if let Some(pos) = stack.iter().position(|&x| x == next) {
                            let cycle: Vec<u64> = stack[pos..].to_vec();
                            if cycles.len() < 32 {
                                cycles.push(cycle);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        stack.pop();
        visited.insert(node, 1);
    }

    /// Critical path: longest chain of required dependencies
    pub fn critical_path(&self) -> Vec<u64> {
        let mut longest: Vec<u64> = Vec::new();
        let nodes: Vec<u64> = self.outgoing.keys().cloned().collect();

        // Find roots (no incoming required edges)
        let roots: Vec<u64> = nodes.iter()
            .filter(|&&n| {
                !self.incoming.get(&n).map(|inc| {
                    inc.iter().any(|&from| {
                        self.edges.get(&(from, n))
                            .map(|e| e.strength >= CoopDepStrength::Required)
                            .unwrap_or(false)
                    })
                }).unwrap_or(false)
            })
            .cloned()
            .collect();

        for root in roots {
            let mut path = Vec::new();
            self.find_longest_path(root, &mut path, &mut longest);
        }

        longest
    }

    fn find_longest_path(&self, node: u64, current: &mut Vec<u64>, longest: &mut Vec<u64>) {
        current.push(node);
        if current.len() > longest.len() {
            *longest = current.clone();
        }

        if let Some(neighbors) = self.outgoing.get(&node) {
            for &next in neighbors {
                if !current.contains(&next) {
                    if let Some(edge) = self.edges.get(&(node, next)) {
                        if edge.strength >= CoopDepStrength::Required {
                            self.find_longest_path(next, current, longest);
                        }
                    }
                }
            }
        }

        current.pop();
    }

    /// Cascade failure analysis: what breaks if pid fails
    pub fn cascade_impact(&self, failed_pid: u64) -> Vec<u64> {
        let mut impacted = Vec::new();
        let mut queue = alloc::vec![failed_pid];
        let mut visited = BTreeMap::new();
        visited.insert(failed_pid, true);

        while let Some(pid) = queue.pop() {
            let dependents = self.dependents_on(pid);
            for dep in dependents {
                if visited.contains_key(&dep) {
                    continue;
                }
                // Check if this is a critical/required dependency
                if let Some(edge) = self.edges.get(&(dep, pid)) {
                    if edge.strength >= CoopDepStrength::Required {
                        impacted.push(dep);
                        visited.insert(dep, true);
                        queue.push(dep);
                    }
                }
            }
        }

        impacted
    }

    /// Remove process from graph
    pub fn remove_process(&mut self, pid: u64) {
        let out = self.outgoing.remove(&pid).unwrap_or_default();
        for target in &out {
            self.edges.remove(&(pid, *target));
            if let Some(inc) = self.incoming.get_mut(target) {
                inc.retain(|&x| x != pid);
            }
        }
        let inc = self.incoming.remove(&pid).unwrap_or_default();
        for source in &inc {
            self.edges.remove(&(*source, pid));
            if let Some(out_list) = self.outgoing.get_mut(source) {
                out_list.retain(|&x| x != pid);
            }
        }
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Dependency tracker stats
#[derive(Debug, Clone, Default)]
pub struct CoopDepTrackerStats {
    /// Total nodes
    pub total_nodes: usize,
    /// Total edges
    pub total_edges: usize,
    /// Detected cycles
    pub detected_cycles: usize,
    /// Critical edges
    pub critical_edges: usize,
}

/// Coop dependency tracker
pub struct CoopDepTracker {
    /// Dependency graph
    pub graph: CoopDepGraph,
    /// Stats
    stats: CoopDepTrackerStats,
}

impl CoopDepTracker {
    pub fn new() -> Self {
        Self {
            graph: CoopDepGraph::new(),
            stats: CoopDepTrackerStats::default(),
        }
    }

    /// Add dependency
    pub fn add_dependency(&mut self, from: u64, to: u64, dep_type: CoopDepType, strength: CoopDepStrength, now: u64) {
        let edge = CoopDepEdge::new(from, to, dep_type, strength, now);
        self.graph.add_edge(edge);
        self.update_stats();
    }

    /// Record invocation
    pub fn record_invocation(&mut self, from: u64, to: u64, latency_ns: u64, success: bool, now: u64) {
        if let Some(edge) = self.graph.get_edge_mut(from, to) {
            edge.record_invocation(latency_ns, success, now);
        }
    }

    /// Detect cycles
    pub fn check_cycles(&mut self) -> Vec<Vec<u64>> {
        let cycles = self.graph.detect_cycles();
        self.stats.detected_cycles = cycles.len();
        cycles
    }

    /// Impact analysis
    pub fn impact_of_failure(&self, pid: u64) -> Vec<u64> {
        self.graph.cascade_impact(pid)
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        self.graph.remove_process(pid);
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.total_nodes = self.graph.outgoing.len();
        self.stats.total_edges = self.graph.edges.len();
        self.stats.critical_edges = self.graph.edges.values()
            .filter(|e| e.strength >= CoopDepStrength::Critical)
            .count();
    }

    /// Stats
    pub fn stats(&self) -> &CoopDepTrackerStats {
        &self.stats
    }
}
