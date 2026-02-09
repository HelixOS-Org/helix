//! # Application Dependency Tracker
//!
//! Inter-process dependency analysis:
//! - Dependency graph construction
//! - Circular dependency detection
//! - Critical path analysis
//! - Dependency health monitoring
//! - Impact assessment

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// DEPENDENCY TYPES
// ============================================================================

/// Dependency type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppDepType {
    /// IPC communication
    Ipc,
    /// Shared memory
    SharedMemory,
    /// File lock
    FileLock,
    /// Socket
    Socket,
    /// Signal
    Signal,
    /// Pipe
    Pipe,
    /// Semaphore
    Semaphore,
    /// Parent-child
    ParentChild,
}

/// Dependency strength
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DepStrength {
    /// Weak (occasional)
    Weak,
    /// Normal
    Normal,
    /// Strong (frequent)
    Strong,
    /// Critical (blocking)
    Critical,
}

/// Dependency state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepState {
    /// Active
    Active,
    /// Idle (no recent activity)
    Idle,
    /// Blocked (waiting)
    Blocked,
    /// Broken (target unavailable)
    Broken,
}

// ============================================================================
// DEPENDENCY EDGE
// ============================================================================

/// A dependency edge
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Source pid
    pub source: u64,
    /// Target pid
    pub target: u64,
    /// Type
    pub dep_type: AppDepType,
    /// Strength
    pub strength: DepStrength,
    /// State
    pub state: DepState,
    /// Communication count
    pub comm_count: u64,
    /// Total bytes transferred
    pub bytes_transferred: u64,
    /// Average latency (ns)
    pub avg_latency_ns: f64,
    /// Last activity timestamp
    pub last_activity: u64,
}

impl DependencyEdge {
    pub fn new(source: u64, target: u64, dep_type: AppDepType) -> Self {
        Self {
            source,
            target,
            dep_type,
            strength: DepStrength::Normal,
            state: DepState::Active,
            comm_count: 0,
            bytes_transferred: 0,
            avg_latency_ns: 0.0,
            last_activity: 0,
        }
    }

    /// Record communication
    pub fn record(&mut self, bytes: u64, latency_ns: u64, now: u64) {
        self.comm_count += 1;
        self.bytes_transferred += bytes;
        let alpha = 0.1;
        self.avg_latency_ns = alpha * latency_ns as f64 + (1.0 - alpha) * self.avg_latency_ns;
        self.last_activity = now;

        // Update strength based on frequency
        if self.comm_count > 1000 {
            self.strength = DepStrength::Critical;
        } else if self.comm_count > 100 {
            self.strength = DepStrength::Strong;
        } else if self.comm_count > 10 {
            self.strength = DepStrength::Normal;
        }
    }

    /// Bandwidth (bytes/sec) estimate
    #[inline]
    pub fn bandwidth_estimate(&self) -> f64 {
        if self.avg_latency_ns <= 0.0 || self.comm_count == 0 {
            return 0.0;
        }
        let avg_bytes = self.bytes_transferred as f64 / self.comm_count as f64;
        avg_bytes / (self.avg_latency_ns / 1_000_000_000.0)
    }
}

// ============================================================================
// DEPENDENCY GRAPH
// ============================================================================

/// Dependency graph
#[derive(Debug)]
pub struct DependencyGraph {
    /// Edges keyed by FNV hash of (source, target)
    edges: BTreeMap<u64, DependencyEdge>,
    /// Outgoing edges per node
    outgoing: BTreeMap<u64, Vec<u64>>,
    /// Incoming edges per node
    incoming: BTreeMap<u64, Vec<u64>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            edges: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
        }
    }

    fn edge_key(source: u64, target: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= source;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= target;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Add or update edge
    pub fn add_edge(&mut self, edge: DependencyEdge) {
        let key = Self::edge_key(edge.source, edge.target);
        let src = edge.source;
        let tgt = edge.target;
        self.edges.insert(key, edge);
        let outgoing = self.outgoing.entry(src).or_insert_with(Vec::new);
        if !outgoing.contains(&tgt) {
            outgoing.push(tgt);
        }
        let incoming = self.incoming.entry(tgt).or_insert_with(Vec::new);
        if !incoming.contains(&src) {
            incoming.push(src);
        }
    }

    /// Get edge
    #[inline(always)]
    pub fn edge(&self, source: u64, target: u64) -> Option<&DependencyEdge> {
        let key = Self::edge_key(source, target);
        self.edges.get(&key)
    }

    /// Get mutable edge
    #[inline(always)]
    pub fn edge_mut(&mut self, source: u64, target: u64) -> Option<&mut DependencyEdge> {
        let key = Self::edge_key(source, target);
        self.edges.get_mut(&key)
    }

    /// Remove node (and all its edges)
    pub fn remove_node(&mut self, pid: u64) {
        // Remove outgoing
        if let Some(targets) = self.outgoing.remove(&pid) {
            for tgt in &targets {
                let key = Self::edge_key(pid, *tgt);
                self.edges.remove(&key);
                if let Some(inc) = self.incoming.get_mut(tgt) {
                    inc.retain(|&s| s != pid);
                }
            }
        }
        // Remove incoming
        if let Some(sources) = self.incoming.remove(&pid) {
            for src in &sources {
                let key = Self::edge_key(*src, pid);
                self.edges.remove(&key);
                if let Some(out) = self.outgoing.get_mut(src) {
                    out.retain(|&t| t != pid);
                }
            }
        }
    }

    /// Dependencies of pid (outgoing)
    #[inline(always)]
    pub fn dependencies(&self, pid: u64) -> Vec<u64> {
        self.outgoing.get(&pid).cloned().unwrap_or_default()
    }

    /// Dependents of pid (incoming)
    #[inline(always)]
    pub fn dependents(&self, pid: u64) -> Vec<u64> {
        self.incoming.get(&pid).cloned().unwrap_or_default()
    }

    /// Node count
    #[inline]
    pub fn node_count(&self) -> usize {
        let mut nodes = alloc::collections::BTreeSet::new();
        for edge in self.edges.values() {
            nodes.insert(edge.source);
            nodes.insert(edge.target);
        }
        nodes.len()
    }

    /// Edge count
    #[inline(always)]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Detect simple cycles (2-node mutual dependencies)
    #[inline]
    pub fn detect_cycles(&self) -> Vec<(u64, u64)> {
        let mut cycles = Vec::new();
        for edge in self.edges.values() {
            let reverse_key = Self::edge_key(edge.target, edge.source);
            if self.edges.contains_key(&reverse_key) && edge.source < edge.target {
                cycles.push((edge.source, edge.target));
            }
        }
        cycles
    }

    /// Critical edges (blocking or critical strength)
    #[inline]
    pub fn critical_edges(&self) -> Vec<&DependencyEdge> {
        self.edges.values()
            .filter(|e| e.strength == DepStrength::Critical || e.state == DepState::Blocked)
            .collect()
    }
}

// ============================================================================
// DEPENDENCY ANALYZER
// ============================================================================

/// Dependency stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppDependencyStats {
    /// Nodes
    pub node_count: usize,
    /// Edges
    pub edge_count: usize,
    /// Cycles detected
    pub cycle_count: usize,
    /// Broken dependencies
    pub broken_count: usize,
}

/// App dependency analyzer
pub struct AppDependencyAnalyzer {
    /// Graph
    graph: DependencyGraph,
    /// Stats
    stats: AppDependencyStats,
}

impl AppDependencyAnalyzer {
    pub fn new() -> Self {
        Self {
            graph: DependencyGraph::new(),
            stats: AppDependencyStats::default(),
        }
    }

    /// Record dependency
    #[inline]
    pub fn record(&mut self, source: u64, target: u64, dep_type: AppDepType, bytes: u64, latency_ns: u64, now: u64) {
        if let Some(edge) = self.graph.edge_mut(source, target) {
            edge.record(bytes, latency_ns, now);
        } else {
            let mut edge = DependencyEdge::new(source, target, dep_type);
            edge.record(bytes, latency_ns, now);
            self.graph.add_edge(edge);
        }
        self.update_stats();
    }

    /// Remove process
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.graph.remove_node(pid);
        self.update_stats();
    }

    /// Impact of losing pid
    #[inline(always)]
    pub fn impact_of_loss(&self, pid: u64) -> Vec<u64> {
        self.graph.dependents(pid)
    }

    /// Graph ref
    #[inline(always)]
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    fn update_stats(&mut self) {
        self.stats.node_count = self.graph.node_count();
        self.stats.edge_count = self.graph.edge_count();
        self.stats.cycle_count = self.graph.detect_cycles().len();
        self.stats.broken_count = self.graph.edges.values()
            .filter(|e| e.state == DepState::Broken).count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppDependencyStats {
        &self.stats
    }
}
