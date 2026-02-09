// SPDX-License-Identifier: GPL-2.0
//! Coop notify — cooperative notification fan-out

extern crate alloc;

/// Notify coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyCoopEvent { FanOut, Coalesce, Filter, Deduplicate }

/// Notify coop record
#[derive(Debug, Clone)]
pub struct NotifyCoopRecord {
    pub event: NotifyCoopEvent,
    pub subscribers: u32,
    pub notifications: u32,
    pub source_hash: u64,
}

impl NotifyCoopRecord {
    pub fn new(event: NotifyCoopEvent) -> Self { Self { event, subscribers: 0, notifications: 0, source_hash: 0 } }
}

/// Notify coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct NotifyCoopStats { pub total_events: u64, pub fanouts: u64, pub coalesced: u64, pub filtered: u64 }

/// Main coop notify
#[derive(Debug)]
pub struct CoopNotify { pub stats: NotifyCoopStats }

impl CoopNotify {
    pub fn new() -> Self { Self { stats: NotifyCoopStats { total_events: 0, fanouts: 0, coalesced: 0, filtered: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &NotifyCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            NotifyCoopEvent::FanOut => self.stats.fanouts += 1,
            NotifyCoopEvent::Coalesce | NotifyCoopEvent::Deduplicate => self.stats.coalesced += 1,
            NotifyCoopEvent::Filter => self.stats.filtered += 1,
        }
    }
}
