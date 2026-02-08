// SPDX-License-Identifier: GPL-2.0
//! Coop spin_barrier — spinning barrier for thread synchronization.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

/// Barrier phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarrierPhase {
    Arriving,
    Released,
}

/// Spin barrier instance
#[derive(Debug)]
pub struct SpinBarrier {
    pub id: u64,
    pub parties: u32,
    pub count: AtomicU32,
    pub generation: u64,
    pub total_waits: u64,
    pub total_completions: u64,
}

impl SpinBarrier {
    pub fn new(id: u64, parties: u32) -> Self {
        Self { id, parties, count: AtomicU32::new(0), generation: 0, total_waits: 0, total_completions: 0 }
    }

    pub fn arrive(&mut self) -> bool {
        let prev = self.count.fetch_add(1, Ordering::AcqRel);
        self.total_waits += 1;
        if prev + 1 >= self.parties {
            self.count.store(0, Ordering::Release);
            self.generation += 1;
            self.total_completions += 1;
            true
        } else { false }
    }

    pub fn reset(&mut self) {
        self.count.store(0, Ordering::Release);
        self.generation += 1;
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SpinBarrierStats {
    pub total_barriers: u32,
    pub total_waits: u64,
    pub total_completions: u64,
}

/// Main coop spin barrier manager
pub struct CoopSpinBarrier {
    barriers: Vec<SpinBarrier>,
    next_id: u64,
}

impl CoopSpinBarrier {
    pub fn new() -> Self { Self { barriers: Vec::new(), next_id: 1 } }

    pub fn create(&mut self, parties: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.barriers.push(SpinBarrier::new(id, parties));
        id
    }

    pub fn arrive(&mut self, idx: usize) -> bool {
        if idx < self.barriers.len() { self.barriers[idx].arrive() } else { false }
    }

    pub fn stats(&self) -> SpinBarrierStats {
        let waits: u64 = self.barriers.iter().map(|b| b.total_waits).sum();
        let comps: u64 = self.barriers.iter().map(|b| b.total_completions).sum();
        SpinBarrierStats { total_barriers: self.barriers.len() as u32, total_waits: waits, total_completions: comps }
    }
}
