// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop lockdep — Lock dependency graph and deadlock detection
//!
//! Implements a lock dependency validator that tracks lock ordering,
//! detects potential deadlocks via cycle detection, and reports violations.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Lock class category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockdepClass {
    Mutex,
    Spinlock,
    RwLockRead,
    RwLockWrite,
    Semaphore,
    SeqLock,
    Rcu,
}

/// Dependency edge type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockdepEdgeType {
    Before,       // this lock must be acquired before
    After,        // this lock must be acquired after
    TryLock,      // try-lock (no ordering constraint)
    InterruptCtx, // acquired in interrupt context
}

/// A violation report.
#[derive(Debug, Clone)]
pub struct LockdepViolation {
    pub violation_id: u64,
    pub lock_a: u64,
    pub lock_b: u64,
    pub pid: u64,
    pub kind: LockdepViolationKind,
    pub backtrace_hash: u64,
    pub timestamp: u64,
}

/// Violation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockdepViolationKind {
    OrderInversion,
    PotentialDeadlock,
    RecursiveLock,
    IrqSafety,
    DoubleLock,
}

/// A lock class node in the dependency graph.
#[derive(Debug, Clone)]
pub struct LockdepNode {
    pub lock_id: u64,
    pub class: LockdepClass,
    pub name: Option<String>,
    pub forward_deps: Vec<u64>,  // locks acquired after this
    pub backward_deps: Vec<u64>, // locks acquired before this
    pub acquire_count: u64,
    pub contention_count: u64,
    pub irq_safe: bool,
}

impl LockdepNode {
    pub fn new(lock_id: u64, class: LockdepClass) -> Self {
        Self {
            lock_id,
            class,
            name: None,
            forward_deps: Vec::new(),
            backward_deps: Vec::new(),
            acquire_count: 0,
            contention_count: 0,
            irq_safe: false,
        }
    }

    pub fn add_forward(&mut self, dep: u64) {
        if !self.forward_deps.contains(&dep) {
            self.forward_deps.push(dep);
        }
    }

    pub fn add_backward(&mut self, dep: u64) {
        if !self.backward_deps.contains(&dep) {
            self.backward_deps.push(dep);
        }
    }
}

/// Per-CPU lock hold stack.
#[derive(Debug, Clone)]
pub struct LockdepHoldStack {
    pub cpu: u32,
    pub held_locks: Vec<u64>,
    pub max_depth: usize,
}

impl LockdepHoldStack {
    pub fn new(cpu: u32) -> Self {
        Self {
            cpu,
            held_locks: Vec::new(),
            max_depth: 0,
        }
    }

    pub fn push(&mut self, lock_id: u64) {
        self.held_locks.push(lock_id);
        if self.held_locks.len() > self.max_depth {
            self.max_depth = self.held_locks.len();
        }
    }

    pub fn pop(&mut self, lock_id: u64) -> bool {
        if let Some(pos) = self.held_locks.iter().rposition(|&l| l == lock_id) {
            self.held_locks.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn is_held(&self, lock_id: u64) -> bool {
        self.held_locks.contains(&lock_id)
    }
}

/// Statistics for lockdep.
#[derive(Debug, Clone)]
pub struct LockdepStats {
    pub total_nodes: u64,
    pub total_edges: u64,
    pub total_violations: u64,
    pub deadlocks_detected: u64,
    pub order_inversions: u64,
    pub recursive_locks: u64,
}

/// Main coop lockdep manager.
pub struct CoopLockdep {
    pub graph: BTreeMap<u64, LockdepNode>,
    pub stacks: BTreeMap<u32, LockdepHoldStack>,
    pub violations: Vec<LockdepViolation>,
    pub next_violation_id: u64,
    pub stats: LockdepStats,
}

impl CoopLockdep {
    pub fn new() -> Self {
        Self {
            graph: BTreeMap::new(),
            stacks: BTreeMap::new(),
            violations: Vec::new(),
            next_violation_id: 1,
            stats: LockdepStats {
                total_nodes: 0,
                total_edges: 0,
                total_violations: 0,
                deadlocks_detected: 0,
                order_inversions: 0,
                recursive_locks: 0,
            },
        }
    }

    pub fn register_lock(&mut self, lock_id: u64, class: LockdepClass) {
        if !self.graph.contains_key(&lock_id) {
            let node = LockdepNode::new(lock_id, class);
            self.graph.insert(lock_id, node);
            self.stats.total_nodes += 1;
        }
    }

    pub fn record_acquire(&mut self, cpu: u32, lock_id: u64, pid: u64) {
        let stack = self.stacks.entry(cpu).or_insert_with(|| LockdepHoldStack::new(cpu));
        // Check for double-lock
        if stack.is_held(lock_id) {
            let vid = self.next_violation_id;
            self.next_violation_id += 1;
            self.violations.push(LockdepViolation {
                violation_id: vid,
                lock_a: lock_id,
                lock_b: lock_id,
                pid,
                kind: LockdepViolationKind::DoubleLock,
                backtrace_hash: 0,
                timestamp: 0,
            });
            self.stats.total_violations += 1;
            self.stats.recursive_locks += 1;
        }
        // Record ordering dependencies
        for &held in &stack.held_locks {
            if let Some(node) = self.graph.get_mut(&held) {
                node.add_forward(lock_id);
                self.stats.total_edges += 1;
            }
            if let Some(node) = self.graph.get_mut(&lock_id) {
                node.add_backward(held);
            }
        }
        stack.push(lock_id);
        if let Some(node) = self.graph.get_mut(&lock_id) {
            node.acquire_count += 1;
        }
    }

    pub fn record_release(&mut self, cpu: u32, lock_id: u64) {
        if let Some(stack) = self.stacks.get_mut(&cpu) {
            stack.pop(lock_id);
        }
    }

    pub fn has_cycle(&self, start: u64) -> bool {
        let mut visited = Vec::new();
        let mut stack = alloc::vec![start];
        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                if current == start && visited.len() > 1 {
                    return true;
                }
                continue;
            }
            visited.push(current);
            if let Some(node) = self.graph.get(&current) {
                for &dep in &node.forward_deps {
                    stack.push(dep);
                }
            }
        }
        false
    }

    pub fn node_count(&self) -> usize {
        self.graph.len()
    }
}
