// SPDX-License-Identifier: GPL-2.0
//! Coop epoll — cooperative epoll interest set sharing

extern crate alloc;

/// Epoll coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpollCoopEvent { InterestMerge, WakeupBatch, FdSetShare, EventCoalesce }

/// Epoll coop record
#[derive(Debug, Clone)]
pub struct EpollCoopRecord {
    pub event: EpollCoopEvent,
    pub epoll_instances: u32,
    pub shared_fds: u32,
    pub coalesced_events: u32,
}

impl EpollCoopRecord {
    pub fn new(event: EpollCoopEvent) -> Self { Self { event, epoll_instances: 0, shared_fds: 0, coalesced_events: 0 } }
}

/// Epoll coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EpollCoopStats { pub total_events: u64, pub merges: u64, pub batches: u64, pub coalesced: u64 }

/// Main coop epoll
#[derive(Debug)]
pub struct CoopEpoll { pub stats: EpollCoopStats }

impl CoopEpoll {
    pub fn new() -> Self { Self { stats: EpollCoopStats { total_events: 0, merges: 0, batches: 0, coalesced: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &EpollCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            EpollCoopEvent::InterestMerge | EpollCoopEvent::FdSetShare => self.stats.merges += 1,
            EpollCoopEvent::WakeupBatch => self.stats.batches += 1,
            EpollCoopEvent::EventCoalesce => self.stats.coalesced += rec.coalesced_events as u64,
        }
    }
}
