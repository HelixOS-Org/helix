// SPDX-License-Identifier: GPL-2.0
//! Coop async_barrier — async-compatible barrier primitive.

extern crate alloc;

use alloc::vec::Vec;

/// Barrier phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncBarrierPhase {
    Waiting,
    Released,
    TimedOut,
    Cancelled,
}

/// Async waiter
#[derive(Debug)]
pub struct AsyncWaiter {
    pub id: u64,
    pub phase: AsyncBarrierPhase,
    pub arrival_time: u64,
    pub release_time: u64,
    pub wait_ns: u64,
}

impl AsyncWaiter {
    pub fn new(id: u64, now: u64) -> Self {
        Self { id, phase: AsyncBarrierPhase::Waiting, arrival_time: now, release_time: 0, wait_ns: 0 }
    }

    #[inline]
    pub fn release(&mut self, now: u64) {
        self.phase = AsyncBarrierPhase::Released;
        self.release_time = now;
        self.wait_ns = now.saturating_sub(self.arrival_time);
    }
}

/// Async barrier instance
#[derive(Debug)]
pub struct AsyncBarrier {
    pub generation: u64,
    pub threshold: u32,
    pub waiters: Vec<AsyncWaiter>,
    pub completed_generations: u64,
}

impl AsyncBarrier {
    pub fn new(threshold: u32) -> Self {
        Self { generation: 0, threshold, waiters: Vec::new(), completed_generations: 0 }
    }

    #[inline]
    pub fn arrive(&mut self, id: u64, now: u64) -> bool {
        self.waiters.push(AsyncWaiter::new(id, now));
        if self.waiters.len() as u32 >= self.threshold {
            for w in self.waiters.iter_mut() { w.release(now); }
            self.completed_generations += 1;
            self.generation += 1;
            true
        } else { false }
    }

    #[inline]
    pub fn drain_released(&mut self) -> Vec<u64> {
        let ids: Vec<u64> = self.waiters.iter().filter(|w| w.phase == AsyncBarrierPhase::Released).map(|w| w.id).collect();
        self.waiters.retain(|w| w.phase == AsyncBarrierPhase::Waiting);
        ids
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AsyncBarrierStats {
    pub total_generations: u64,
    pub current_waiters: u32,
    pub threshold: u32,
    pub avg_wait_ns: u64,
}

/// Main coop async barrier
pub struct CoopAsyncBarrier {
    barriers: Vec<AsyncBarrier>,
}

impl CoopAsyncBarrier {
    pub fn new() -> Self { Self { barriers: Vec::new() } }

    #[inline]
    pub fn create(&mut self, threshold: u32) -> usize {
        let idx = self.barriers.len();
        self.barriers.push(AsyncBarrier::new(threshold));
        idx
    }

    #[inline(always)]
    pub fn arrive(&mut self, idx: usize, id: u64, now: u64) -> bool {
        if let Some(b) = self.barriers.get_mut(idx) { b.arrive(id, now) } else { false }
    }

    #[inline]
    pub fn stats(&self) -> Vec<AsyncBarrierStats> {
        self.barriers.iter().map(|b| {
            let total_wait: u64 = b.waiters.iter().map(|w| w.wait_ns).sum();
            let avg = if b.waiters.is_empty() { 0 } else { total_wait / b.waiters.len() as u64 };
            AsyncBarrierStats { total_generations: b.completed_generations, current_waiters: b.waiters.len() as u32, threshold: b.threshold, avg_wait_ns: avg }
        }).collect()
    }
}
