//! # Holistic Lock Contention Analyzer
//!
//! System-wide lock contention analysis and optimization:
//! - Global lock dependency graph
//! - Lock contention hotspot detection
//! - Priority inversion detection across subsystems
//! - Lock hold-time distribution analysis
//! - Deadlock potential scoring
//! - Lock ordering violation detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Lock type in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemLockType {
    Spinlock,
    Mutex,
    RwLockRead,
    RwLockWrite,
    Semaphore,
    SeqLock,
    RcuRead,
}

/// Contention severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentionLevel {
    None,
    Low,
    Moderate,
    High,
    Severe,
}

/// Lock instance descriptor
#[derive(Debug, Clone)]
pub struct LockInstance {
    pub lock_id: u64,
    pub lock_type: SystemLockType,
    pub name_hash: u64,
    pub subsystem_id: u32,
    pub acquire_count: u64,
    pub contention_count: u64,
    pub total_hold_ns: u64,
    pub max_hold_ns: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
    pub current_holder: Option<u64>,
    pub waiter_count: u32,
}

impl LockInstance {
    pub fn new(lock_id: u64, lock_type: SystemLockType, subsystem_id: u32) -> Self {
        Self {
            lock_id,
            lock_type,
            name_hash: {
                let mut h: u64 = 0xcbf29ce484222325;
                h ^= lock_id;
                h = h.wrapping_mul(0x100000001b3);
                h ^= subsystem_id as u64;
                h = h.wrapping_mul(0x100000001b3);
                h
            },
            subsystem_id,
            acquire_count: 0,
            contention_count: 0,
            total_hold_ns: 0,
            max_hold_ns: 0,
            total_wait_ns: 0,
            max_wait_ns: 0,
            current_holder: None,
            waiter_count: 0,
        }
    }

    pub fn avg_hold_ns(&self) -> u64 {
        if self.acquire_count == 0 { return 0; }
        self.total_hold_ns / self.acquire_count
    }

    pub fn avg_wait_ns(&self) -> u64 {
        if self.contention_count == 0 { return 0; }
        self.total_wait_ns / self.contention_count
    }

    pub fn contention_ratio(&self) -> f64 {
        if self.acquire_count == 0 { return 0.0; }
        self.contention_count as f64 / self.acquire_count as f64
    }

    pub fn severity(&self) -> ContentionLevel {
        let ratio = self.contention_ratio();
        if ratio > 0.5 { ContentionLevel::Severe }
        else if ratio > 0.3 { ContentionLevel::High }
        else if ratio > 0.1 { ContentionLevel::Moderate }
        else if ratio > 0.01 { ContentionLevel::Low }
        else { ContentionLevel::None }
    }
}

/// Lock ordering edge (A acquired before B)
#[derive(Debug, Clone)]
pub struct LockOrderEdgeHolistic {
    pub lock_a: u64,
    pub lock_b: u64,
    pub observed_count: u64,
    pub task_id: u64,
}

/// Priority inversion record
#[derive(Debug, Clone)]
pub struct PriorityInversionHolistic {
    pub lock_id: u64,
    pub holder_prio: i32,
    pub waiter_prio: i32,
    pub inversion_ns: u64,
    pub timestamp_ns: u64,
}

/// Lock contention hotspot
#[derive(Debug, Clone)]
pub struct ContentionHotspot {
    pub lock_id: u64,
    pub contention_score: f64,
    pub impact_ns: u64,
    pub affected_tasks: u32,
}

/// Hold-time distribution bucket
#[derive(Debug, Clone)]
pub struct HoldTimeBucket {
    pub min_ns: u64,
    pub max_ns: u64,
    pub count: u64,
}

/// Lock dependency graph for deadlock analysis
#[derive(Debug, Clone)]
pub struct LockDepGraph {
    edges: Vec<LockOrderEdgeHolistic>,
}

impl LockDepGraph {
    pub fn new() -> Self {
        Self { edges: Vec::new() }
    }

    pub fn add_edge(&mut self, a: u64, b: u64, task: u64) {
        for edge in &mut self.edges {
            if edge.lock_a == a && edge.lock_b == b {
                edge.observed_count += 1;
                return;
            }
        }
        self.edges.push(LockOrderEdgeHolistic {
            lock_a: a,
            lock_b: b,
            observed_count: 1,
            task_id: task,
        });
    }

    /// Check for ordering violations (A→B and B→A both exist)
    pub fn ordering_violations(&self) -> Vec<(u64, u64)> {
        let mut violations = Vec::new();
        for e1 in &self.edges {
            for e2 in &self.edges {
                if e1.lock_a == e2.lock_b && e1.lock_b == e2.lock_a {
                    let pair = if e1.lock_a < e1.lock_b {
                        (e1.lock_a, e1.lock_b)
                    } else {
                        (e1.lock_b, e1.lock_a)
                    };
                    if !violations.contains(&pair) {
                        violations.push(pair);
                    }
                }
            }
        }
        violations
    }

    /// Simple cycle detection using DFS
    pub fn has_cycle(&self) -> bool {
        let mut nodes: Vec<u64> = Vec::new();
        for e in &self.edges {
            if !nodes.contains(&e.lock_a) { nodes.push(e.lock_a); }
            if !nodes.contains(&e.lock_b) { nodes.push(e.lock_b); }
        }

        for &start in &nodes {
            let mut visited: Vec<u64> = Vec::new();
            let mut stack: Vec<u64> = alloc::vec![start];
            while let Some(node) = stack.pop() {
                if visited.contains(&node) { return true; }
                visited.push(node);
                for e in &self.edges {
                    if e.lock_a == node {
                        stack.push(e.lock_b);
                    }
                }
            }
        }
        false
    }
}

/// Holistic Lock Contention stats
#[derive(Debug, Clone, Default)]
pub struct HolisticLockContentionStats {
    pub total_locks: usize,
    pub contended_locks: usize,
    pub severe_hotspots: usize,
    pub ordering_violations: usize,
    pub priority_inversions: u64,
    pub total_contention_ns: u64,
}

/// Holistic Lock Contention Analyzer
pub struct HolisticLockContention {
    locks: BTreeMap<u64, LockInstance>,
    dep_graph: LockDepGraph,
    inversions: Vec<PriorityInversionHolistic>,
    max_inversions: usize,
    stats: HolisticLockContentionStats,
}

impl HolisticLockContention {
    pub fn new(max_inversions: usize) -> Self {
        Self {
            locks: BTreeMap::new(),
            dep_graph: LockDepGraph::new(),
            inversions: Vec::new(),
            max_inversions,
            stats: HolisticLockContentionStats::default(),
        }
    }

    pub fn register_lock(&mut self, lock: LockInstance) {
        self.locks.insert(lock.lock_id, lock);
    }

    pub fn record_acquire(&mut self, lock_id: u64, task_id: u64, waited: bool, wait_ns: u64, ts: u64) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.acquire_count += 1;
            lock.current_holder = Some(task_id);
            if waited {
                lock.contention_count += 1;
                lock.total_wait_ns += wait_ns;
                if wait_ns > lock.max_wait_ns { lock.max_wait_ns = wait_ns; }
            }
            let _ = ts;
        }
    }

    pub fn record_release(&mut self, lock_id: u64, hold_ns: u64) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.current_holder = None;
            lock.total_hold_ns += hold_ns;
            if hold_ns > lock.max_hold_ns { lock.max_hold_ns = hold_ns; }
        }
    }

    pub fn record_order(&mut self, held_lock: u64, acquiring_lock: u64, task: u64) {
        self.dep_graph.add_edge(held_lock, acquiring_lock, task);
    }

    pub fn record_inversion(&mut self, inv: PriorityInversionHolistic) {
        self.inversions.push(inv);
        while self.inversions.len() > self.max_inversions {
            self.inversions.remove(0);
        }
    }

    /// Find contention hotspots
    pub fn hotspots(&self, min_severity: ContentionLevel) -> Vec<ContentionHotspot> {
        self.locks.values()
            .filter(|l| l.severity() >= min_severity)
            .map(|l| ContentionHotspot {
                lock_id: l.lock_id,
                contention_score: l.contention_ratio(),
                impact_ns: l.total_wait_ns,
                affected_tasks: l.waiter_count,
            })
            .collect()
    }

    pub fn recompute(&mut self) {
        self.stats.total_locks = self.locks.len();
        self.stats.contended_locks = self.locks.values()
            .filter(|l| l.contention_count > 0).count();
        self.stats.severe_hotspots = self.locks.values()
            .filter(|l| l.severity() >= ContentionLevel::Severe).count();
        self.stats.ordering_violations = self.dep_graph.ordering_violations().len();
        self.stats.priority_inversions = self.inversions.len() as u64;
        self.stats.total_contention_ns = self.locks.values()
            .map(|l| l.total_wait_ns).sum();
    }

    pub fn lock(&self, id: u64) -> Option<&LockInstance> { self.locks.get(&id) }
    pub fn has_deadlock_risk(&self) -> bool { self.dep_graph.has_cycle() }
    pub fn stats(&self) -> &HolisticLockContentionStats { &self.stats }
}
