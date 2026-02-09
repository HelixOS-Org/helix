// SPDX-License-Identifier: GPL-2.0
//! Coop countdown — countdown latch for multi-thread synchronization.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Countdown state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CountdownState {
    Waiting,
    Reached,
    TimedOut,
    Cancelled,
}

/// Countdown latch
#[derive(Debug)]
pub struct CountdownLatch {
    pub id: u64,
    pub initial_count: u32,
    pub current_count: u32,
    pub state: CountdownState,
    pub waiters: u32,
    pub created_at: u64,
    pub reached_at: u64,
    pub decrements: u64,
}

impl CountdownLatch {
    pub fn new(id: u64, count: u32, now: u64) -> Self {
        Self { id, initial_count: count, current_count: count, state: CountdownState::Waiting, waiters: 0, created_at: now, reached_at: 0, decrements: 0 }
    }

    #[inline]
    pub fn count_down(&mut self, now: u64) -> bool {
        if self.current_count == 0 { return true; }
        self.current_count -= 1;
        self.decrements += 1;
        if self.current_count == 0 {
            self.state = CountdownState::Reached;
            self.reached_at = now;
            return true;
        }
        false
    }

    #[inline(always)]
    pub fn add_waiter(&mut self) { self.waiters += 1; }
    #[inline(always)]
    pub fn remove_waiter(&mut self) { if self.waiters > 0 { self.waiters -= 1; } }

    #[inline(always)]
    pub fn cancel(&mut self) { self.state = CountdownState::Cancelled; }

    #[inline(always)]
    pub fn progress(&self) -> f64 {
        if self.initial_count == 0 { return 1.0; }
        1.0 - (self.current_count as f64 / self.initial_count as f64)
    }

    #[inline(always)]
    pub fn elapsed(&self) -> u64 {
        if self.reached_at > 0 { self.reached_at - self.created_at }
        else { 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CountdownStats {
    pub total_latches: u32,
    pub active: u32,
    pub reached: u32,
    pub total_decrements: u64,
    pub avg_progress: f64,
}

/// Main coop countdown manager
pub struct CoopCountdown {
    latches: BTreeMap<u64, CountdownLatch>,
    next_id: u64,
}

impl CoopCountdown {
    pub fn new() -> Self { Self { latches: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, count: u32, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.latches.insert(id, CountdownLatch::new(id, count, now));
        id
    }

    #[inline(always)]
    pub fn count_down(&mut self, id: u64, now: u64) -> bool {
        if let Some(l) = self.latches.get_mut(&id) { l.count_down(now) }
        else { false }
    }

    #[inline(always)]
    pub fn destroy(&mut self, id: u64) { self.latches.remove(&id); }

    #[inline]
    pub fn stats(&self) -> CountdownStats {
        let active = self.latches.values().filter(|l| l.state == CountdownState::Waiting).count() as u32;
        let reached = self.latches.values().filter(|l| l.state == CountdownState::Reached).count() as u32;
        let decs: u64 = self.latches.values().map(|l| l.decrements).sum();
        let avg = if self.latches.is_empty() { 0.0 }
            else { self.latches.values().map(|l| l.progress()).sum::<f64>() / self.latches.len() as f64 };
        CountdownStats { total_latches: self.latches.len() as u32, active, reached, total_decrements: decs, avg_progress: avg }
    }
}
