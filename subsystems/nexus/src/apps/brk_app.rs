// SPDX-License-Identifier: GPL-2.0
//! Apps brk_app — program break (heap) management.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Brk region state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrkState {
    Uninitialized,
    Active,
    Expanding,
    Shrinking,
}

/// Process heap
#[derive(Debug)]
pub struct ProcessHeap {
    pub pid: u64,
    pub start: u64,
    pub current: u64,
    pub max_ever: u64,
    pub state: BrkState,
    pub expand_count: u64,
    pub shrink_count: u64,
    pub total_expanded: u64,
    pub total_shrunk: u64,
    pub page_faults: u64,
}

impl ProcessHeap {
    pub fn new(pid: u64, start: u64) -> Self {
        Self { pid, start, current: start, max_ever: start, state: BrkState::Active, expand_count: 0, shrink_count: 0, total_expanded: 0, total_shrunk: 0, page_faults: 0 }
    }

    pub fn size(&self) -> u64 { self.current - self.start }
    pub fn pages(&self) -> u64 { (self.size() + 4095) / 4096 }

    pub fn brk(&mut self, new_brk: u64) -> bool {
        if new_brk < self.start { return false; }
        if new_brk > self.current { let delta = new_brk - self.current; self.total_expanded += delta; self.expand_count += 1; self.state = BrkState::Expanding; }
        else if new_brk < self.current { let delta = self.current - new_brk; self.total_shrunk += delta; self.shrink_count += 1; self.state = BrkState::Shrinking; }
        self.current = new_brk;
        if new_brk > self.max_ever { self.max_ever = new_brk; }
        self.state = BrkState::Active;
        true
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct BrkAppStats {
    pub tracked_processes: u32,
    pub total_heap_bytes: u64,
    pub total_heap_pages: u64,
    pub total_expands: u64,
    pub total_shrinks: u64,
}

/// Main brk app
pub struct AppBrk {
    heaps: BTreeMap<u64, ProcessHeap>,
}

impl AppBrk {
    pub fn new() -> Self { Self { heaps: BTreeMap::new() } }
    pub fn init_heap(&mut self, pid: u64, start: u64) { self.heaps.insert(pid, ProcessHeap::new(pid, start)); }
    pub fn remove(&mut self, pid: u64) { self.heaps.remove(&pid); }

    pub fn brk(&mut self, pid: u64, new_brk: u64) -> bool {
        if let Some(h) = self.heaps.get_mut(&pid) { h.brk(new_brk) } else { false }
    }

    pub fn stats(&self) -> BrkAppStats {
        let bytes: u64 = self.heaps.values().map(|h| h.size()).sum();
        let pages: u64 = self.heaps.values().map(|h| h.pages()).sum();
        let expands: u64 = self.heaps.values().map(|h| h.expand_count).sum();
        let shrinks: u64 = self.heaps.values().map(|h| h.shrink_count).sum();
        BrkAppStats { tracked_processes: self.heaps.len() as u32, total_heap_bytes: bytes, total_heap_pages: pages, total_expands: expands, total_shrinks: shrinks }
    }
}
