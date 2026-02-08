// SPDX-License-Identifier: GPL-2.0
//! Coop signal — cooperative signal delivery coordination

extern crate alloc;

/// Signal coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalCoopEvent { GroupSignal, BroadcastSignal, SignalForward, CoalescedDelivery }

/// Signal coop record
#[derive(Debug, Clone)]
pub struct SignalCoopRecord {
    pub event: SignalCoopEvent,
    pub signal_nr: u32,
    pub source_pid: u32,
    pub targets: u32,
}

impl SignalCoopRecord {
    pub fn new(event: SignalCoopEvent, signal_nr: u32) -> Self {
        Self { event, signal_nr, source_pid: 0, targets: 0 }
    }
}

/// Signal coop stats
#[derive(Debug, Clone)]
pub struct SignalCoopStats { pub total_events: u64, pub group_signals: u64, pub broadcasts: u64, pub coalesced: u64 }

/// Main coop signal
#[derive(Debug)]
pub struct CoopSignal { pub stats: SignalCoopStats }

impl CoopSignal {
    pub fn new() -> Self { Self { stats: SignalCoopStats { total_events: 0, group_signals: 0, broadcasts: 0, coalesced: 0 } } }
    pub fn record(&mut self, rec: &SignalCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            SignalCoopEvent::GroupSignal => self.stats.group_signals += 1,
            SignalCoopEvent::BroadcastSignal => self.stats.broadcasts += 1,
            SignalCoopEvent::CoalescedDelivery => self.stats.coalesced += 1,
            _ => {}
        }
    }
}
