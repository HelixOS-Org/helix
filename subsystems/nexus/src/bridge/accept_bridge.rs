// SPDX-License-Identifier: GPL-2.0
//! Bridge accept — socket accept connection bridging

extern crate alloc;

/// Accept bridge event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptBridgeEvent { Accepted, Refused, Timeout, NonBlocking }

/// Accept bridge record
#[derive(Debug, Clone)]
pub struct AcceptBridgeRecord {
    pub event: AcceptBridgeEvent,
    pub listen_fd: i32,
    pub new_fd: i32,
    pub peer_port: u16,
    pub flags: u32,
}

impl AcceptBridgeRecord {
    pub fn new(event: AcceptBridgeEvent, listen_fd: i32) -> Self { Self { event, listen_fd, new_fd: -1, peer_port: 0, flags: 0 } }
}

/// Accept bridge stats
#[derive(Debug, Clone)]
pub struct AcceptBridgeStats { pub total_events: u64, pub accepted: u64, pub refused: u64, pub timeouts: u64 }

/// Main bridge accept
#[derive(Debug)]
pub struct BridgeAccept { pub stats: AcceptBridgeStats }

impl BridgeAccept {
    pub fn new() -> Self { Self { stats: AcceptBridgeStats { total_events: 0, accepted: 0, refused: 0, timeouts: 0 } } }
    pub fn record(&mut self, rec: &AcceptBridgeRecord) {
        self.stats.total_events += 1;
        match rec.event {
            AcceptBridgeEvent::Accepted | AcceptBridgeEvent::NonBlocking => self.stats.accepted += 1,
            AcceptBridgeEvent::Refused => self.stats.refused += 1,
            AcceptBridgeEvent::Timeout => self.stats.timeouts += 1,
        }
    }
}
