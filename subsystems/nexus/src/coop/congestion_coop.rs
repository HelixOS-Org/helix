// SPDX-License-Identifier: GPL-2.0
//! Coop congestion — cooperative congestion window coordination

extern crate alloc;

/// Congestion coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionCoopEvent { WindowSync, SlowStartGroup, CongAvoidCoord, RecoveryAssist }

/// Congestion coop record
#[derive(Debug, Clone)]
pub struct CongestionCoopRecord {
    pub event: CongestionCoopEvent,
    pub cwnd: u32,
    pub ssthresh: u32,
    pub flows: u32,
}

impl CongestionCoopRecord {
    pub fn new(event: CongestionCoopEvent) -> Self { Self { event, cwnd: 0, ssthresh: 0, flows: 0 } }
}

/// Congestion coop stats
#[derive(Debug, Clone)]
pub struct CongestionCoopStats { pub total_events: u64, pub syncs: u64, pub group_starts: u64, pub recoveries: u64 }

/// Main coop congestion
#[derive(Debug)]
pub struct CoopCongestion { pub stats: CongestionCoopStats }

impl CoopCongestion {
    pub fn new() -> Self { Self { stats: CongestionCoopStats { total_events: 0, syncs: 0, group_starts: 0, recoveries: 0 } } }
    pub fn record(&mut self, rec: &CongestionCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            CongestionCoopEvent::WindowSync | CongestionCoopEvent::CongAvoidCoord => self.stats.syncs += 1,
            CongestionCoopEvent::SlowStartGroup => self.stats.group_starts += 1,
            CongestionCoopEvent::RecoveryAssist => self.stats.recoveries += 1,
        }
    }
}
