// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop CLH lock — Craig, Landin, and Hagersten queue lock
//!
//! Implements the CLH lock where each thread spins on its predecessor's
//! node, providing implicit FIFO ordering and good NUMA behavior.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// CLH node state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClhNodeState {
    Locked,
    Unlocked,
}

/// A CLH queue node.
#[derive(Debug, Clone)]
pub struct ClhNode {
    pub node_id: u64,
    pub pid: u64,
    pub state: ClhNodeState,
    pub predecessor: Option<u64>,
    pub spin_count: u64,
    pub cpu: u32,
}

impl ClhNode {
    pub fn new(node_id: u64, pid: u64, cpu: u32) -> Self {
        Self {
            node_id,
            pid,
            state: ClhNodeState::Locked,
            predecessor: None,
            spin_count: 0,
            cpu,
        }
    }
}

/// A CLH lock instance.
#[derive(Debug, Clone)]
pub struct ClhLockInstance {
    pub lock_id: u64,
    pub tail_node: Option<u64>,
    pub sentinel_id: u64,
    pub nodes: BTreeMap<u64, ClhNode>,
    pub next_node_id: u64,
    pub total_acquires: u64,
    pub total_spins: u64,
    pub max_queue_depth: u32,
}

impl ClhLockInstance {
    pub fn new(lock_id: u64) -> Self {
        // Create sentinel node (always unlocked)
        let mut nodes = BTreeMap::new();
        let sentinel = ClhNode {
            node_id: 0,
            pid: 0,
            state: ClhNodeState::Unlocked,
            predecessor: None,
            spin_count: 0,
            cpu: 0,
        };
        nodes.insert(0, sentinel);
        Self {
            lock_id,
            tail_node: Some(0),
            sentinel_id: 0,
            nodes,
            next_node_id: 1,
            total_acquires: 0,
            total_spins: 0,
            max_queue_depth: 0,
        }
    }

    pub fn enqueue(&mut self, pid: u64, cpu: u32) -> u64 {
        let nid = self.next_node_id;
        self.next_node_id += 1;
        let mut node = ClhNode::new(nid, pid, cpu);
        node.predecessor = self.tail_node;
        self.nodes.insert(nid, node);
        self.tail_node = Some(nid);
        // Check if predecessor is unlocked (immediate acquire)
        if let Some(pred_id) = self.nodes.get(&nid).and_then(|n| n.predecessor) {
            if let Some(pred) = self.nodes.get(&pred_id) {
                if pred.state == ClhNodeState::Unlocked {
                    self.total_acquires += 1;
                }
            }
        }
        let depth = self.nodes.len() as u32;
        if depth > self.max_queue_depth {
            self.max_queue_depth = depth;
        }
        nid
    }

    pub fn release(&mut self, node_id: u64) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.state = ClhNodeState::Unlocked;
        }
        // Cleanup old nodes that are unlocked and have no successors
        let mut to_remove = Vec::new();
        for (id, node) in &self.nodes {
            if node.state == ClhNodeState::Unlocked && *id != self.tail_node.unwrap_or(0) {
                let has_successor = self.nodes.values().any(|n| n.predecessor == Some(*id));
                if !has_successor && *id != 0 {
                    to_remove.push(*id);
                }
            }
        }
        for id in to_remove {
            self.nodes.remove(&id);
        }
    }

    #[inline(always)]
    pub fn queue_depth(&self) -> usize {
        self.nodes.len().saturating_sub(1) // exclude sentinel
    }
}

/// Statistics for CLH lock.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ClhLockStats {
    pub total_locks: u64,
    pub total_acquires: u64,
    pub total_spins: u64,
    pub max_queue: u32,
}

/// Main coop CLH lock manager.
pub struct CoopClhLock {
    pub locks: BTreeMap<u64, ClhLockInstance>,
    pub next_lock_id: u64,
    pub stats: ClhLockStats,
}

impl CoopClhLock {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            next_lock_id: 1,
            stats: ClhLockStats {
                total_locks: 0,
                total_acquires: 0,
                total_spins: 0,
                max_queue: 0,
            },
        }
    }

    #[inline]
    pub fn create_lock(&mut self) -> u64 {
        let id = self.next_lock_id;
        self.next_lock_id += 1;
        let lock = ClhLockInstance::new(id);
        self.locks.insert(id, lock);
        self.stats.total_locks += 1;
        id
    }

    #[inline(always)]
    pub fn lock_count(&self) -> usize {
        self.locks.len()
    }
}
