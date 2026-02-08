// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop MCS lock — Mellor-Crummey & Scott queue-based spinlock
//!
//! Implements the MCS lock for scalable NUMA-friendly mutual exclusion.
//! Each waiter spins on a local variable, avoiding cache-line bouncing.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// MCS node state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McsNodeState {
    Waiting,
    Granted,
    Released,
}

/// An MCS queue node.
#[derive(Debug, Clone)]
pub struct McsNode {
    pub node_id: u64,
    pub pid: u64,
    pub cpu: u32,
    pub state: McsNodeState,
    pub next: Option<u64>,
    pub spin_count: u64,
}

impl McsNode {
    pub fn new(node_id: u64, pid: u64, cpu: u32) -> Self {
        Self {
            node_id,
            pid,
            cpu,
            state: McsNodeState::Waiting,
            next: None,
            spin_count: 0,
        }
    }
}

/// An MCS lock instance.
#[derive(Debug, Clone)]
pub struct McsLockInstance {
    pub lock_id: u64,
    pub tail_node: Option<u64>,
    pub holder_node: Option<u64>,
    pub nodes: BTreeMap<u64, McsNode>,
    pub next_node_id: u64,
    pub total_acquires: u64,
    pub total_spins: u64,
    pub max_queue_len: u32,
    pub current_queue_len: u32,
}

impl McsLockInstance {
    pub fn new(lock_id: u64) -> Self {
        Self {
            lock_id,
            tail_node: None,
            holder_node: None,
            nodes: BTreeMap::new(),
            next_node_id: 1,
            total_acquires: 0,
            total_spins: 0,
            max_queue_len: 0,
            current_queue_len: 0,
        }
    }

    pub fn enqueue(&mut self, pid: u64, cpu: u32) -> u64 {
        let nid = self.next_node_id;
        self.next_node_id += 1;
        let node = McsNode::new(nid, pid, cpu);
        if let Some(tail_id) = self.tail_node {
            if let Some(tail) = self.nodes.get_mut(&tail_id) {
                tail.next = Some(nid);
            }
        } else {
            // Lock is free, grant immediately
            let mut granted = node.clone();
            granted.state = McsNodeState::Granted;
            self.nodes.insert(nid, granted);
            self.holder_node = Some(nid);
            self.tail_node = Some(nid);
            self.total_acquires += 1;
            self.current_queue_len = 1;
            return nid;
        }
        self.nodes.insert(nid, node);
        self.tail_node = Some(nid);
        self.current_queue_len += 1;
        if self.current_queue_len as u32 > self.max_queue_len {
            self.max_queue_len = self.current_queue_len as u32;
        }
        nid
    }

    pub fn release(&mut self, node_id: u64) -> Option<u64> {
        if let Some(node) = self.nodes.get(&node_id) {
            let next = node.next;
            self.nodes.remove(&node_id);
            self.current_queue_len = self.current_queue_len.saturating_sub(1);
            if let Some(next_id) = next {
                if let Some(next_node) = self.nodes.get_mut(&next_id) {
                    next_node.state = McsNodeState::Granted;
                }
                self.holder_node = Some(next_id);
                self.total_acquires += 1;
                return Some(next_id);
            } else {
                self.holder_node = None;
                self.tail_node = None;
            }
        }
        None
    }

    pub fn avg_spin(&self) -> f64 {
        if self.total_acquires == 0 {
            return 0.0;
        }
        self.total_spins as f64 / self.total_acquires as f64
    }
}

/// Statistics for MCS lock.
#[derive(Debug, Clone)]
pub struct McsLockStats {
    pub total_locks: u64,
    pub total_acquires: u64,
    pub total_spins: u64,
    pub max_queue: u32,
}

/// Main coop MCS lock manager.
pub struct CoopMcsLock {
    pub locks: BTreeMap<u64, McsLockInstance>,
    pub next_lock_id: u64,
    pub stats: McsLockStats,
}

impl CoopMcsLock {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            next_lock_id: 1,
            stats: McsLockStats {
                total_locks: 0,
                total_acquires: 0,
                total_spins: 0,
                max_queue: 0,
            },
        }
    }

    pub fn create_lock(&mut self) -> u64 {
        let id = self.next_lock_id;
        self.next_lock_id += 1;
        let lock = McsLockInstance::new(id);
        self.locks.insert(id, lock);
        self.stats.total_locks += 1;
        id
    }

    pub fn lock_count(&self) -> usize {
        self.locks.len()
    }
}
