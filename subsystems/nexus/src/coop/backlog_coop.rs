// SPDX-License-Identifier: GPL-2.0
//! Coop backlog — cooperative socket backlog sharing

extern crate alloc;

/// Backlog coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacklogCoopEvent { QueueShare, OverflowRedirect, SynCookiePool, BackpressureSync }

/// Backlog coop record
#[derive(Debug, Clone)]
pub struct BacklogCoopRecord {
    pub event: BacklogCoopEvent,
    pub queue_depth: u32,
    pub redirected: u32,
    pub listeners: u32,
}

impl BacklogCoopRecord {
    pub fn new(event: BacklogCoopEvent) -> Self { Self { event, queue_depth: 0, redirected: 0, listeners: 0 } }
}

/// Backlog coop stats
#[derive(Debug, Clone)]
pub struct BacklogCoopStats { pub total_events: u64, pub shares: u64, pub redirects: u64, pub syncs: u64 }

/// Main coop backlog
#[derive(Debug)]
pub struct CoopBacklog { pub stats: BacklogCoopStats }

impl CoopBacklog {
    pub fn new() -> Self { Self { stats: BacklogCoopStats { total_events: 0, shares: 0, redirects: 0, syncs: 0 } } }
    pub fn record(&mut self, rec: &BacklogCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            BacklogCoopEvent::QueueShare => self.stats.shares += 1,
            BacklogCoopEvent::OverflowRedirect | BacklogCoopEvent::SynCookiePool => self.stats.redirects += 1,
            BacklogCoopEvent::BackpressureSync => self.stats.syncs += 1,
        }
    }
}
