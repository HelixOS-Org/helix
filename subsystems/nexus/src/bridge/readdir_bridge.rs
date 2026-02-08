// SPDX-License-Identifier: GPL-2.0
//! Bridge readdir — directory read operation bridging

extern crate alloc;

/// Readdir bridge event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaddirBridgeEvent { Getdents, Readdir, Seekdir, Telldir }

/// Readdir bridge record
#[derive(Debug, Clone)]
pub struct ReaddirBridgeRecord {
    pub event: ReaddirBridgeEvent,
    pub fd: i32,
    pub entries: u32,
    pub dir_inode: u64,
}

impl ReaddirBridgeRecord {
    pub fn new(event: ReaddirBridgeEvent, fd: i32) -> Self { Self { event, fd, entries: 0, dir_inode: 0 } }
}

/// Readdir bridge stats
#[derive(Debug, Clone)]
pub struct ReaddirBridgeStats { pub total_ops: u64, pub entries_read: u64, pub seeks: u64, pub large_dirs: u64 }

/// Main bridge readdir
#[derive(Debug)]
pub struct BridgeReaddir { pub stats: ReaddirBridgeStats }

impl BridgeReaddir {
    pub fn new() -> Self { Self { stats: ReaddirBridgeStats { total_ops: 0, entries_read: 0, seeks: 0, large_dirs: 0 } } }
    pub fn record(&mut self, rec: &ReaddirBridgeRecord) {
        self.stats.total_ops += 1;
        self.stats.entries_read += rec.entries as u64;
        match rec.event {
            ReaddirBridgeEvent::Seekdir | ReaddirBridgeEvent::Telldir => self.stats.seeks += 1,
            _ => {}
        }
        if rec.entries > 1000 { self.stats.large_dirs += 1; }
    }
}
