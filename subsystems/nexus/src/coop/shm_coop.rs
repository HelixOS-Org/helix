// SPDX-License-Identifier: GPL-2.0
//! Coop shm — cooperative shared memory region management

extern crate alloc;

/// Shm coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmCoopEvent { RegionShare, CowFork, PageMigrate, NumaRebalance }

/// Shm coop record
#[derive(Debug, Clone)]
pub struct ShmCoopRecord {
    pub event: ShmCoopEvent,
    pub shmid: i32,
    pub pages: u64,
    pub participants: u32,
}

impl ShmCoopRecord {
    pub fn new(event: ShmCoopEvent) -> Self { Self { event, shmid: -1, pages: 0, participants: 0 } }
}

/// Shm coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ShmCoopStats { pub total_events: u64, pub shares: u64, pub migrations: u64, pub cow_forks: u64 }

/// Main coop shm
#[derive(Debug)]
pub struct CoopShm { pub stats: ShmCoopStats }

impl CoopShm {
    pub fn new() -> Self { Self { stats: ShmCoopStats { total_events: 0, shares: 0, migrations: 0, cow_forks: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &ShmCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            ShmCoopEvent::RegionShare => self.stats.shares += 1,
            ShmCoopEvent::PageMigrate | ShmCoopEvent::NumaRebalance => self.stats.migrations += 1,
            ShmCoopEvent::CowFork => self.stats.cow_forks += 1,
        }
    }
}
