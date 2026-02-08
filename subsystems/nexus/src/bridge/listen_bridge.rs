// SPDX-License-Identifier: GPL-2.0
//! Bridge listen — socket listen backlog bridging

extern crate alloc;

/// Listen bridge event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenBridgeEvent { Start, BacklogFull, BacklogDrain, OverflowDrop }

/// Listen bridge record
#[derive(Debug, Clone)]
pub struct ListenBridgeRecord {
    pub event: ListenBridgeEvent,
    pub fd: i32,
    pub backlog: u32,
    pub pending: u32,
}

impl ListenBridgeRecord {
    pub fn new(event: ListenBridgeEvent, fd: i32) -> Self { Self { event, fd, backlog: 128, pending: 0 } }
}

/// Listen bridge stats
#[derive(Debug, Clone)]
pub struct ListenBridgeStats { pub total_events: u64, pub starts: u64, pub overflows: u64, pub peak_pending: u32 }

/// Main bridge listen
#[derive(Debug)]
pub struct BridgeListen { pub stats: ListenBridgeStats }

impl BridgeListen {
    pub fn new() -> Self { Self { stats: ListenBridgeStats { total_events: 0, starts: 0, overflows: 0, peak_pending: 0 } } }
    pub fn record(&mut self, rec: &ListenBridgeRecord) {
        self.stats.total_events += 1;
        match rec.event {
            ListenBridgeEvent::Start => self.stats.starts += 1,
            ListenBridgeEvent::BacklogFull | ListenBridgeEvent::OverflowDrop => self.stats.overflows += 1,
            _ => {}
        }
        if rec.pending > self.stats.peak_pending { self.stats.peak_pending = rec.pending; }
    }
}
