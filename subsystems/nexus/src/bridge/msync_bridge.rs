// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Msync (memory sync bridge)

extern crate alloc;
use alloc::vec::Vec;

/// Msync flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeMsyncFlag { Async, Sync, Invalidate }

/// Msync record
#[derive(Debug, Clone)]
pub struct BridgeMsyncRecord { pub addr: u64, pub length: u64, pub flag: BridgeMsyncFlag, pub pages_synced: u32 }

/// Msync stats
#[derive(Debug, Clone)]
pub struct BridgeMsyncStats { pub total_ops: u64, pub async_syncs: u64, pub sync_syncs: u64, pub invalidates: u64, pub total_bytes_synced: u64 }

/// Manager for msync bridge
pub struct BridgeMsyncManager {
    history: Vec<BridgeMsyncRecord>,
    stats: BridgeMsyncStats,
}

impl BridgeMsyncManager {
    pub fn new() -> Self {
        Self { history: Vec::new(), stats: BridgeMsyncStats { total_ops: 0, async_syncs: 0, sync_syncs: 0, invalidates: 0, total_bytes_synced: 0 } }
    }

    pub fn msync(&mut self, addr: u64, length: u64, flag: BridgeMsyncFlag) {
        self.stats.total_ops += 1;
        match flag { BridgeMsyncFlag::Async => self.stats.async_syncs += 1, BridgeMsyncFlag::Sync => self.stats.sync_syncs += 1, BridgeMsyncFlag::Invalidate => self.stats.invalidates += 1 }
        let pages = ((length + 4095) / 4096) as u32;
        self.stats.total_bytes_synced += length;
        self.history.push(BridgeMsyncRecord { addr, length, flag, pages_synced: pages });
    }

    pub fn stats(&self) -> &BridgeMsyncStats { &self.stats }
}
