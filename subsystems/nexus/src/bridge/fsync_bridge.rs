// SPDX-License-Identifier: GPL-2.0
//! Bridge fsync — file sync operation bridging

extern crate alloc;

/// Fsync bridge event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsyncBridgeEvent { Fsync, Fdatasync, SyncRange, SyncFs }

/// Fsync bridge record
#[derive(Debug, Clone)]
pub struct FsyncBridgeRecord {
    pub event: FsyncBridgeEvent,
    pub fd: i32,
    pub offset: u64,
    pub len: u64,
    pub latency_ns: u64,
}

impl FsyncBridgeRecord {
    pub fn new(event: FsyncBridgeEvent, fd: i32) -> Self { Self { event, fd, offset: 0, len: 0, latency_ns: 0 } }
}

/// Fsync bridge stats
#[derive(Debug, Clone)]
pub struct FsyncBridgeStats { pub total_ops: u64, pub full_syncs: u64, pub data_syncs: u64, pub range_syncs: u64 }

/// Main bridge fsync
#[derive(Debug)]
pub struct BridgeFsync { pub stats: FsyncBridgeStats }

impl BridgeFsync {
    pub fn new() -> Self { Self { stats: FsyncBridgeStats { total_ops: 0, full_syncs: 0, data_syncs: 0, range_syncs: 0 } } }
    pub fn record(&mut self, rec: &FsyncBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.event {
            FsyncBridgeEvent::Fsync | FsyncBridgeEvent::SyncFs => self.stats.full_syncs += 1,
            FsyncBridgeEvent::Fdatasync => self.stats.data_syncs += 1,
            FsyncBridgeEvent::SyncRange => self.stats.range_syncs += 1,
        }
    }
}
