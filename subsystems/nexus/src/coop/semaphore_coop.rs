// SPDX-License-Identifier: GPL-2.0
//! Coop semaphore — cooperative semaphore deadlock avoidance

extern crate alloc;

/// Semaphore coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemCoopEvent { DeadlockDetect, PriorityInherit, WaiterCoalesce, UndoShare }

/// Semaphore coop record
#[derive(Debug, Clone)]
pub struct SemCoopRecord {
    pub event: SemCoopEvent,
    pub semid: i32,
    pub waiters: u32,
    pub resolved: bool,
}

impl SemCoopRecord {
    pub fn new(event: SemCoopEvent) -> Self { Self { event, semid: -1, waiters: 0, resolved: false } }
}

/// Semaphore coop stats
#[derive(Debug, Clone)]
pub struct SemCoopStats { pub total_events: u64, pub deadlocks: u64, pub inherits: u64, pub coalesced: u64 }

/// Main coop semaphore
#[derive(Debug)]
pub struct CoopSemaphore { pub stats: SemCoopStats }

impl CoopSemaphore {
    pub fn new() -> Self { Self { stats: SemCoopStats { total_events: 0, deadlocks: 0, inherits: 0, coalesced: 0 } } }
    pub fn record(&mut self, rec: &SemCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            SemCoopEvent::DeadlockDetect => self.stats.deadlocks += 1,
            SemCoopEvent::PriorityInherit => self.stats.inherits += 1,
            SemCoopEvent::WaiterCoalesce | SemCoopEvent::UndoShare => self.stats.coalesced += 1,
        }
    }
}
