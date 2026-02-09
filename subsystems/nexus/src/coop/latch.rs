// SPDX-License-Identifier: GPL-2.0
//! Coop latch — single-use gate synchronization primitive.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Latch state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatchState {
    Closed,
    Open,
    TimedOut,
}

/// Latch
#[derive(Debug)]
pub struct Latch {
    pub id: u64,
    pub state: LatchState,
    pub waiters: u32,
    pub created_at: u64,
    pub opened_at: u64,
    pub total_waits: u64,
    pub max_waiters: u32,
}

impl Latch {
    pub fn new(id: u64, now: u64) -> Self {
        Self { id, state: LatchState::Closed, waiters: 0, created_at: now, opened_at: 0, total_waits: 0, max_waiters: 0 }
    }

    #[inline]
    pub fn open(&mut self, now: u64) -> u32 {
        self.state = LatchState::Open;
        self.opened_at = now;
        let released = self.waiters;
        self.waiters = 0;
        released
    }

    #[inline]
    pub fn wait(&mut self) -> bool {
        if self.state == LatchState::Open { return true; }
        self.waiters += 1;
        self.total_waits += 1;
        if self.waiters > self.max_waiters { self.max_waiters = self.waiters; }
        false
    }

    #[inline(always)]
    pub fn is_open(&self) -> bool { self.state == LatchState::Open }

    #[inline(always)]
    pub fn latency_ns(&self) -> u64 {
        if self.opened_at > self.created_at { self.opened_at - self.created_at } else { 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LatchStats {
    pub total_latches: u32,
    pub open_count: u32,
    pub closed_count: u32,
    pub total_waits: u64,
    pub avg_latency_ns: u64,
}

/// Main coop latch manager
pub struct CoopLatch {
    latches: BTreeMap<u64, Latch>,
    next_id: u64,
}

impl CoopLatch {
    pub fn new() -> Self { Self { latches: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.latches.insert(id, Latch::new(id, now));
        id
    }

    #[inline(always)]
    pub fn open(&mut self, id: u64, now: u64) -> u32 {
        if let Some(l) = self.latches.get_mut(&id) { l.open(now) }
        else { 0 }
    }

    #[inline(always)]
    pub fn wait(&mut self, id: u64) -> bool {
        if let Some(l) = self.latches.get_mut(&id) { l.wait() }
        else { false }
    }

    #[inline(always)]
    pub fn destroy(&mut self, id: u64) { self.latches.remove(&id); }

    #[inline]
    pub fn stats(&self) -> LatchStats {
        let open = self.latches.values().filter(|l| l.is_open()).count() as u32;
        let closed = self.latches.values().filter(|l| !l.is_open()).count() as u32;
        let waits: u64 = self.latches.values().map(|l| l.total_waits).sum();
        let lat_sum: u64 = self.latches.values().filter(|l| l.is_open()).map(|l| l.latency_ns()).sum();
        let open_count = self.latches.values().filter(|l| l.is_open()).count() as u64;
        let avg = if open_count == 0 { 0 } else { lat_sum / open_count };
        LatchStats { total_latches: self.latches.len() as u32, open_count: open, closed_count: closed, total_waits: waits, avg_latency_ns: avg }
    }
}

// ============================================================================
// Merged from latch_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LatchV2State {
    Counting,
    Released,
    Reset,
}

/// A latch V2 instance
#[derive(Debug, Clone)]
pub struct LatchV2Instance {
    pub id: u64,
    pub initial_count: u32,
    pub current_count: u32,
    pub state: LatchV2State,
    pub waiters: Vec<u64>,
    pub countdown_ops: u64,
    pub reset_count: u64,
    pub release_count: u64,
}

impl LatchV2Instance {
    pub fn new(id: u64, count: u32) -> Self {
        Self {
            id, initial_count: count, current_count: count,
            state: LatchV2State::Counting,
            waiters: Vec::new(),
            countdown_ops: 0, reset_count: 0, release_count: 0,
        }
    }

    pub fn countdown(&mut self) -> bool {
        if self.current_count > 0 {
            self.current_count -= 1;
            self.countdown_ops += 1;
            if self.current_count == 0 {
                self.state = LatchV2State::Released;
                self.release_count += 1;
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn add_waiter(&mut self, tid: u64) {
        if self.state != LatchV2State::Released {
            self.waiters.push(tid);
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.current_count = self.initial_count;
        self.state = LatchV2State::Reset;
        self.waiters.clear();
        self.reset_count += 1;
    }

    #[inline(always)]
    pub fn is_released(&self) -> bool {
        self.state == LatchV2State::Released
    }
}

/// Statistics for latch V2
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LatchV2Stats {
    pub latches_created: u64,
    pub countdowns: u64,
    pub releases: u64,
    pub resets: u64,
    pub total_waiters: u64,
}

/// Main latch V2 coop manager
#[derive(Debug)]
pub struct CoopLatchV2 {
    latches: BTreeMap<u64, LatchV2Instance>,
    next_id: u64,
    stats: LatchV2Stats,
}

impl CoopLatchV2 {
    pub fn new() -> Self {
        Self {
            latches: BTreeMap::new(),
            next_id: 1,
            stats: LatchV2Stats {
                latches_created: 0, countdowns: 0,
                releases: 0, resets: 0, total_waiters: 0,
            },
        }
    }

    #[inline]
    pub fn create(&mut self, count: u32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.latches.insert(id, LatchV2Instance::new(id, count));
        self.stats.latches_created += 1;
        id
    }

    #[inline]
    pub fn countdown(&mut self, latch_id: u64) -> bool {
        if let Some(latch) = self.latches.get_mut(&latch_id) {
            self.stats.countdowns += 1;
            if latch.countdown() {
                self.stats.releases += 1;
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn wait(&mut self, latch_id: u64, tid: u64) -> bool {
        if let Some(latch) = self.latches.get_mut(&latch_id) {
            if latch.is_released() { return true; }
            latch.add_waiter(tid);
            self.stats.total_waiters += 1;
            false
        } else { false }
    }

    #[inline]
    pub fn reset(&mut self, latch_id: u64) -> bool {
        if let Some(latch) = self.latches.get_mut(&latch_id) {
            latch.reset();
            self.stats.resets += 1;
            true
        } else { false }
    }

    #[inline(always)]
    pub fn stats(&self) -> &LatchV2Stats {
        &self.stats
    }
}
