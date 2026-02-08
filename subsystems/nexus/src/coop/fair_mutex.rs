// SPDX-License-Identifier: GPL-2.0
//! Coop fair_mutex — fair (FIFO) mutex implementation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fair mutex state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FairMutexState {
    Unlocked,
    Locked,
    Contended,
}

/// Fair mutex waiter
#[derive(Debug)]
pub struct FairMutexWaiter {
    pub tid: u64,
    pub enqueue_time: u64,
    pub position: u32,
}

/// Fair mutex instance
#[derive(Debug)]
pub struct FairMutex {
    pub id: u64,
    pub state: FairMutexState,
    pub owner: u64,
    pub waiters: Vec<FairMutexWaiter>,
    pub lock_count: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
}

impl FairMutex {
    pub fn new(id: u64) -> Self {
        Self { id, state: FairMutexState::Unlocked, owner: 0, waiters: Vec::new(), lock_count: 0, total_wait_ns: 0, max_wait_ns: 0 }
    }

    pub fn try_lock(&mut self, tid: u64) -> bool {
        if self.state == FairMutexState::Unlocked {
            self.state = FairMutexState::Locked;
            self.owner = tid;
            self.lock_count += 1;
            true
        } else { false }
    }

    pub fn enqueue(&mut self, tid: u64, now: u64) {
        let pos = self.waiters.len() as u32;
        self.waiters.push(FairMutexWaiter { tid, enqueue_time: now, position: pos });
        self.state = FairMutexState::Contended;
    }

    pub fn unlock(&mut self, now: u64) -> Option<u64> {
        if self.waiters.is_empty() {
            self.state = FairMutexState::Unlocked;
            self.owner = 0;
            None
        } else {
            let next = self.waiters.remove(0);
            let wait = if now > next.enqueue_time { now - next.enqueue_time } else { 0 };
            self.total_wait_ns += wait;
            if wait > self.max_wait_ns { self.max_wait_ns = wait; }
            self.owner = next.tid;
            self.lock_count += 1;
            if self.waiters.is_empty() { self.state = FairMutexState::Locked; }
            Some(next.tid)
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct FairMutexStats {
    pub total_mutexes: u32,
    pub total_locks: u64,
    pub total_waiters: u32,
    pub max_wait_ns: u64,
}

/// Main coop fair mutex manager
pub struct CoopFairMutex {
    mutexes: BTreeMap<u64, FairMutex>,
    next_id: u64,
}

impl CoopFairMutex {
    pub fn new() -> Self { Self { mutexes: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.mutexes.insert(id, FairMutex::new(id));
        id
    }

    pub fn try_lock(&mut self, id: u64, tid: u64) -> bool {
        if let Some(m) = self.mutexes.get_mut(&id) { m.try_lock(tid) } else { false }
    }

    pub fn enqueue(&mut self, id: u64, tid: u64, now: u64) {
        if let Some(m) = self.mutexes.get_mut(&id) { m.enqueue(tid, now); }
    }

    pub fn unlock(&mut self, id: u64, now: u64) -> Option<u64> {
        if let Some(m) = self.mutexes.get_mut(&id) { m.unlock(now) } else { None }
    }

    pub fn destroy(&mut self, id: u64) { self.mutexes.remove(&id); }

    pub fn stats(&self) -> FairMutexStats {
        let locks: u64 = self.mutexes.values().map(|m| m.lock_count).sum();
        let waiters: u32 = self.mutexes.values().map(|m| m.waiters.len() as u32).sum();
        let max_w: u64 = self.mutexes.values().map(|m| m.max_wait_ns).max().unwrap_or(0);
        FairMutexStats { total_mutexes: self.mutexes.len() as u32, total_locks: locks, total_waiters: waiters, max_wait_ns: max_w }
    }
}
