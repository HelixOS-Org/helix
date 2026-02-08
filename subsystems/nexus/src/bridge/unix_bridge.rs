// SPDX-License-Identifier: GPL-2.0
//! Bridge unix — unix domain socket bridging

extern crate alloc;

/// Unix socket bridge event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnixBridgeEvent { StreamCreate, DgramCreate, SeqpacketCreate, PassFd, PassCred }

/// Unix bridge record
#[derive(Debug, Clone)]
pub struct UnixBridgeRecord {
    pub event: UnixBridgeEvent,
    pub path_hash: u64,
    pub bytes: u64,
    pub ancillary_fds: u32,
}

impl UnixBridgeRecord {
    pub fn new(event: UnixBridgeEvent) -> Self { Self { event, path_hash: 0, bytes: 0, ancillary_fds: 0 } }
}

/// Unix bridge stats
#[derive(Debug, Clone)]
pub struct UnixBridgeStats { pub total_events: u64, pub streams: u64, pub dgrams: u64, pub fd_passes: u64 }

/// Main bridge unix
#[derive(Debug)]
pub struct BridgeUnix { pub stats: UnixBridgeStats }

impl BridgeUnix {
    pub fn new() -> Self { Self { stats: UnixBridgeStats { total_events: 0, streams: 0, dgrams: 0, fd_passes: 0 } } }
    pub fn record(&mut self, rec: &UnixBridgeRecord) {
        self.stats.total_events += 1;
        match rec.event {
            UnixBridgeEvent::StreamCreate | UnixBridgeEvent::SeqpacketCreate => self.stats.streams += 1,
            UnixBridgeEvent::DgramCreate => self.stats.dgrams += 1,
            UnixBridgeEvent::PassFd | UnixBridgeEvent::PassCred => self.stats.fd_passes += 1,
        }
    }
}
