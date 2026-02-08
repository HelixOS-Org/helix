// SPDX-License-Identifier: GPL-2.0
//! Coop lock_free_list — lock-free linked list structure.

extern crate alloc;

use alloc::vec::Vec;

/// Node state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LfNodeState {
    Active,
    Marked,
    Removed,
}

/// Lock-free list node
#[derive(Debug)]
pub struct LfNode {
    pub key: u64,
    pub value_hash: u64,
    pub state: LfNodeState,
    pub next_key: Option<u64>,
    pub insert_epoch: u64,
}

impl LfNode {
    pub fn new(key: u64, value_hash: u64, epoch: u64) -> Self {
        Self { key, value_hash, state: LfNodeState::Active, next_key: None, insert_epoch: epoch }
    }
}

/// Lock-free list
#[derive(Debug)]
pub struct LockFreeList {
    pub nodes: Vec<LfNode>,
    pub head_key: Option<u64>,
    pub epoch: u64,
    pub insertions: u64,
    pub deletions: u64,
    pub traversals: u64,
    pub cas_failures: u64,
}

impl LockFreeList {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), head_key: None, epoch: 0, insertions: 0, deletions: 0, traversals: 0, cas_failures: 0 }
    }

    pub fn insert(&mut self, key: u64, value_hash: u64) -> bool {
        self.epoch += 1;
        if self.nodes.iter().any(|n| n.key == key && n.state == LfNodeState::Active) { return false; }
        let mut node = LfNode::new(key, value_hash, self.epoch);
        node.next_key = self.head_key;
        self.head_key = Some(key);
        self.nodes.push(node);
        self.insertions += 1;
        true
    }

    pub fn remove(&mut self, key: u64) -> bool {
        if let Some(n) = self.nodes.iter_mut().find(|n| n.key == key && n.state == LfNodeState::Active) {
            n.state = LfNodeState::Marked;
            self.deletions += 1;
            true
        } else { false }
    }

    pub fn contains(&mut self, key: u64) -> bool {
        self.traversals += 1;
        self.nodes.iter().any(|n| n.key == key && n.state == LfNodeState::Active)
    }

    pub fn len(&self) -> usize { self.nodes.iter().filter(|n| n.state == LfNodeState::Active).count() }

    pub fn cleanup(&mut self) {
        self.nodes.retain(|n| n.state == LfNodeState::Active);
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct LockFreeListStats {
    pub active_nodes: u32,
    pub total_insertions: u64,
    pub total_deletions: u64,
    pub total_traversals: u64,
    pub cas_failures: u64,
}

/// Main coop lock-free list
pub struct CoopLockFreeList {
    list: LockFreeList,
}

impl CoopLockFreeList {
    pub fn new() -> Self { Self { list: LockFreeList::new() } }
    pub fn insert(&mut self, key: u64, val: u64) -> bool { self.list.insert(key, val) }
    pub fn remove(&mut self, key: u64) -> bool { self.list.remove(key) }
    pub fn contains(&mut self, key: u64) -> bool { self.list.contains(key) }
    pub fn len(&self) -> usize { self.list.len() }

    pub fn stats(&self) -> LockFreeListStats {
        LockFreeListStats { active_nodes: self.list.len() as u32, total_insertions: self.list.insertions, total_deletions: self.list.deletions, total_traversals: self.list.traversals, cas_failures: self.list.cas_failures }
    }
}
