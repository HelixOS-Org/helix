// SPDX-License-Identifier: GPL-2.0
//! Coop wait_group — goroutine-style wait group.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Wait group state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitGroupState {
    Active,
    Waiting,
    Done,
}

/// Wait group
#[derive(Debug)]
pub struct WaitGroup {
    pub id: u64,
    pub counter: i64,
    pub state: WaitGroupState,
    pub waiters: u32,
    pub add_count: u64,
    pub done_count: u64,
    pub created_at: u64,
    pub completed_at: u64,
}

impl WaitGroup {
    pub fn new(id: u64, now: u64) -> Self {
        Self { id, counter: 0, state: WaitGroupState::Active, waiters: 0, add_count: 0, done_count: 0, created_at: now, completed_at: 0 }
    }

    pub fn add(&mut self, delta: i64) {
        self.counter += delta;
        if delta > 0 { self.add_count += delta as u64; }
    }

    pub fn done(&mut self, now: u64) {
        self.counter -= 1;
        self.done_count += 1;
        if self.counter <= 0 { self.state = WaitGroupState::Done; self.completed_at = now; }
    }

    pub fn is_done(&self) -> bool { self.counter <= 0 }
    pub fn duration(&self) -> u64 { if self.completed_at > 0 { self.completed_at - self.created_at } else { 0 } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct WaitGroupStats {
    pub total_groups: u32,
    pub active_groups: u32,
    pub done_groups: u32,
    pub total_adds: u64,
    pub total_dones: u64,
    pub total_waiters: u32,
}

/// Main wait group manager
pub struct CoopWaitGroup {
    groups: BTreeMap<u64, WaitGroup>,
    next_id: u64,
}

impl CoopWaitGroup {
    pub fn new() -> Self { Self { groups: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.groups.insert(id, WaitGroup::new(id, now));
        id
    }

    pub fn add(&mut self, id: u64, delta: i64) {
        if let Some(g) = self.groups.get_mut(&id) { g.add(delta); }
    }

    pub fn done(&mut self, id: u64, now: u64) {
        if let Some(g) = self.groups.get_mut(&id) { g.done(now); }
    }

    pub fn stats(&self) -> WaitGroupStats {
        let active = self.groups.values().filter(|g| g.state == WaitGroupState::Active).count() as u32;
        let done = self.groups.values().filter(|g| g.is_done()).count() as u32;
        let adds: u64 = self.groups.values().map(|g| g.add_count).sum();
        let dones: u64 = self.groups.values().map(|g| g.done_count).sum();
        let waiters: u32 = self.groups.values().map(|g| g.waiters).sum();
        WaitGroupStats { total_groups: self.groups.len() as u32, active_groups: active, done_groups: done, total_adds: adds, total_dones: dones, total_waiters: waiters }
    }
}
