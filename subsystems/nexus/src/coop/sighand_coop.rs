// SPDX-License-Identifier: GPL-2.0
//! Coop sighand — cooperative signal handler table sharing

extern crate alloc;

/// Sighand coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SighandCoopEvent { TableShare, TableCow, HandlerSync, MaskPropagate }

/// Sighand coop record
#[derive(Debug, Clone)]
pub struct SighandCoopRecord {
    pub event: SighandCoopEvent,
    pub thread_count: u32,
    pub signals_shared: u32,
    pub pid: u32,
}

impl SighandCoopRecord {
    pub fn new(event: SighandCoopEvent) -> Self { Self { event, thread_count: 0, signals_shared: 0, pid: 0 } }
}

/// Sighand coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SighandCoopStats { pub total_events: u64, pub shares: u64, pub cows: u64, pub propagations: u64 }

/// Main coop sighand
#[derive(Debug)]
pub struct CoopSighand { pub stats: SighandCoopStats }

impl CoopSighand {
    pub fn new() -> Self { Self { stats: SighandCoopStats { total_events: 0, shares: 0, cows: 0, propagations: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &SighandCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            SighandCoopEvent::TableShare => self.stats.shares += 1,
            SighandCoopEvent::TableCow => self.stats.cows += 1,
            SighandCoopEvent::HandlerSync | SighandCoopEvent::MaskPropagate => self.stats.propagations += 1,
        }
    }
}
