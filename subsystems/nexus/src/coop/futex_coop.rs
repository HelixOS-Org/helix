// SPDX-License-Identifier: GPL-2.0
//! Coop futex — cooperative futex wait queue management

extern crate alloc;

/// Futex coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexCoopEvent { WaitqMerge, WakeCoalesce, PriorityInherit, RequeueOptimize }

/// Futex coop record
#[derive(Debug, Clone)]
pub struct FutexCoopRecord {
    pub event: FutexCoopEvent,
    pub waiters: u32,
    pub woken: u32,
    pub futex_addr: u64,
}

impl FutexCoopRecord {
    pub fn new(event: FutexCoopEvent) -> Self { Self { event, waiters: 0, woken: 0, futex_addr: 0 } }
}

/// Futex coop stats
#[derive(Debug, Clone)]
pub struct FutexCoopStats { pub total_events: u64, pub merges: u64, pub coalesced_wakes: u64, pub inherits: u64 }

/// Main coop futex
#[derive(Debug)]
pub struct CoopFutex { pub stats: FutexCoopStats }

impl CoopFutex {
    pub fn new() -> Self { Self { stats: FutexCoopStats { total_events: 0, merges: 0, coalesced_wakes: 0, inherits: 0 } } }
    pub fn record(&mut self, rec: &FutexCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            FutexCoopEvent::WaitqMerge => self.stats.merges += 1,
            FutexCoopEvent::WakeCoalesce | FutexCoopEvent::RequeueOptimize => self.stats.coalesced_wakes += 1,
            FutexCoopEvent::PriorityInherit => self.stats.inherits += 1,
        }
    }
}
