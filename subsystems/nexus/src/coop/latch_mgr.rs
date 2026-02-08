// SPDX-License-Identifier: GPL-2.0
//! Coop latch_mgr — countdown latch manager.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Latch state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatchState {
    Waiting,
    Released,
    TimedOut,
}

/// Countdown latch
#[derive(Debug)]
pub struct CountdownLatch {
    pub id: u64,
    pub initial_count: u32,
    pub current_count: u32,
    pub state: LatchState,
    pub waiters: u32,
    pub created_at: u64,
    pub released_at: u64,
    pub countdown_events: u64,
}

impl CountdownLatch {
    pub fn new(id: u64, count: u32, now: u64) -> Self {
        Self { id, initial_count: count, current_count: count, state: LatchState::Waiting, waiters: 0, created_at: now, released_at: 0, countdown_events: 0 }
    }

    pub fn count_down(&mut self, now: u64) -> bool {
        if self.current_count == 0 { return false; }
        self.current_count -= 1;
        self.countdown_events += 1;
        if self.current_count == 0 { self.state = LatchState::Released; self.released_at = now; true }
        else { false }
    }

    pub fn is_released(&self) -> bool { self.state == LatchState::Released }
    pub fn progress(&self) -> f64 { if self.initial_count == 0 { 1.0 } else { (self.initial_count - self.current_count) as f64 / self.initial_count as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct LatchMgrStats {
    pub total_latches: u32,
    pub waiting_latches: u32,
    pub released_latches: u32,
    pub total_waiters: u32,
    pub total_countdowns: u64,
}

/// Main latch manager
pub struct CoopLatchMgr {
    latches: BTreeMap<u64, CountdownLatch>,
    next_id: u64,
}

impl CoopLatchMgr {
    pub fn new() -> Self { Self { latches: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, count: u32, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.latches.insert(id, CountdownLatch::new(id, count, now));
        id
    }

    pub fn count_down(&mut self, id: u64, now: u64) -> bool {
        self.latches.get_mut(&id).map_or(false, |l| l.count_down(now))
    }

    pub fn stats(&self) -> LatchMgrStats {
        let waiting = self.latches.values().filter(|l| l.state == LatchState::Waiting).count() as u32;
        let released = self.latches.values().filter(|l| l.is_released()).count() as u32;
        let waiters: u32 = self.latches.values().map(|l| l.waiters).sum();
        let countdowns: u64 = self.latches.values().map(|l| l.countdown_events).sum();
        LatchMgrStats { total_latches: self.latches.len() as u32, waiting_latches: waiting, released_latches: released, total_waiters: waiters, total_countdowns: countdowns }
    }
}
