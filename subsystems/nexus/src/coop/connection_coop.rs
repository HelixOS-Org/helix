// SPDX-License-Identifier: GPL-2.0
//! Coop connection — cooperative connection tracking

extern crate alloc;

/// Connection coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionCoopEvent { TrackingShare, NatSync, StateReplicate, TimeoutCoord }

/// Connection coop record
#[derive(Debug, Clone)]
pub struct ConnectionCoopRecord {
    pub event: ConnectionCoopEvent,
    pub tracked_conns: u32,
    pub nat_entries: u32,
    pub replicated: u32,
}

impl ConnectionCoopRecord {
    pub fn new(event: ConnectionCoopEvent) -> Self { Self { event, tracked_conns: 0, nat_entries: 0, replicated: 0 } }
}

/// Connection coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ConnectionCoopStats { pub total_events: u64, pub shares: u64, pub nat_syncs: u64, pub replications: u64 }

/// Main coop connection
#[derive(Debug)]
pub struct CoopConnection { pub stats: ConnectionCoopStats }

impl CoopConnection {
    pub fn new() -> Self { Self { stats: ConnectionCoopStats { total_events: 0, shares: 0, nat_syncs: 0, replications: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &ConnectionCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            ConnectionCoopEvent::TrackingShare => self.stats.shares += 1,
            ConnectionCoopEvent::NatSync => self.stats.nat_syncs += 1,
            ConnectionCoopEvent::StateReplicate | ConnectionCoopEvent::TimeoutCoord => self.stats.replications += 1,
        }
    }
}
