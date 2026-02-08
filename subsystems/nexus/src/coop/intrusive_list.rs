// SPDX-License-Identifier: GPL-2.0
//! Coop intrusive_list — intrusive doubly-linked list for wait queues.

extern crate alloc;

use alloc::vec::Vec;

/// Node state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntrusiveNodeState {
    Unlinked,
    Linked,
    Removed,
}

/// Intrusive list node
#[derive(Debug)]
pub struct IntrusiveNode {
    pub data: u64,
    pub prev: Option<usize>,
    pub next: Option<usize>,
    pub state: IntrusiveNodeState,
}

impl IntrusiveNode {
    pub fn new(data: u64) -> Self {
        Self { data, prev: None, next: None, state: IntrusiveNodeState::Unlinked }
    }
}

/// Intrusive list
#[derive(Debug)]
pub struct IntrusiveList {
    nodes: Vec<IntrusiveNode>,
    head: Option<usize>,
    tail: Option<usize>,
    len: u32,
    free_list: Vec<usize>,
}

impl IntrusiveList {
    pub fn new() -> Self {
        Self { nodes: Vec::new(), head: None, tail: None, len: 0, free_list: Vec::new() }
    }

    pub fn push_back(&mut self, data: u64) -> usize {
        let idx = if let Some(free) = self.free_list.pop() {
            self.nodes[free] = IntrusiveNode::new(data);
            free
        } else {
            let idx = self.nodes.len();
            self.nodes.push(IntrusiveNode::new(data));
            idx
        };

        self.nodes[idx].state = IntrusiveNodeState::Linked;

        if let Some(tail) = self.tail {
            self.nodes[tail].next = Some(idx);
            self.nodes[idx].prev = Some(tail);
        } else {
            self.head = Some(idx);
        }
        self.tail = Some(idx);
        self.len += 1;
        idx
    }

    pub fn pop_front(&mut self) -> Option<u64> {
        let head = self.head?;
        let data = self.nodes[head].data;
        let next = self.nodes[head].next;

        self.nodes[head].state = IntrusiveNodeState::Removed;
        self.nodes[head].prev = None;
        self.nodes[head].next = None;
        self.free_list.push(head);

        if let Some(n) = next { self.nodes[n].prev = None; }
        else { self.tail = None; }
        self.head = next;
        self.len -= 1;
        Some(data)
    }

    pub fn remove(&mut self, idx: usize) -> Option<u64> {
        if idx >= self.nodes.len() || self.nodes[idx].state != IntrusiveNodeState::Linked { return None; }
        let data = self.nodes[idx].data;
        let prev = self.nodes[idx].prev;
        let next = self.nodes[idx].next;

        if let Some(p) = prev { self.nodes[p].next = next; } else { self.head = next; }
        if let Some(n) = next { self.nodes[n].prev = prev; } else { self.tail = prev; }

        self.nodes[idx].state = IntrusiveNodeState::Removed;
        self.free_list.push(idx);
        self.len -= 1;
        Some(data)
    }

    pub fn len(&self) -> u32 { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }
}

/// Stats
#[derive(Debug, Clone)]
pub struct IntrusiveListStats {
    pub total_lists: u32,
    pub total_nodes: u32,
    pub free_nodes: u32,
}

/// Main coop intrusive list manager
pub struct CoopIntrusiveList {
    lists: Vec<IntrusiveList>,
}

impl CoopIntrusiveList {
    pub fn new() -> Self { Self { lists: Vec::new() } }

    pub fn create(&mut self) -> usize {
        let idx = self.lists.len();
        self.lists.push(IntrusiveList::new());
        idx
    }

    pub fn push_back(&mut self, list: usize, data: u64) -> Option<usize> {
        if list < self.lists.len() { Some(self.lists[list].push_back(data)) } else { None }
    }

    pub fn pop_front(&mut self, list: usize) -> Option<u64> {
        if list < self.lists.len() { self.lists[list].pop_front() } else { None }
    }

    pub fn stats(&self) -> IntrusiveListStats {
        let nodes: u32 = self.lists.iter().map(|l| l.len).sum();
        let free: u32 = self.lists.iter().map(|l| l.free_list.len() as u32).sum();
        IntrusiveListStats { total_lists: self.lists.len() as u32, total_nodes: nodes, free_nodes: free }
    }
}
