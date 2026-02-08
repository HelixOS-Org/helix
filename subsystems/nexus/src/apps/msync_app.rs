// SPDX-License-Identifier: GPL-2.0
//! Apps msync_app — msync memory synchronization application layer.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Msync flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsyncFlag {
    Async,
    Sync,
    Invalidate,
}

/// Msync operation
#[derive(Debug)]
pub struct MsyncOp {
    pub addr: u64,
    pub length: u64,
    pub flags: MsyncFlag,
    pub started_at: u64,
    pub completed_at: u64,
    pub pages_synced: u32,
}

/// Mapping sync tracker
#[derive(Debug)]
pub struct MappingSyncTracker {
    pub addr: u64,
    pub length: u64,
    pub total_syncs: u64,
    pub total_pages_synced: u64,
    pub total_bytes: u64,
    pub last_sync: u64,
    pub dirty_pages: u32,
}

impl MappingSyncTracker {
    pub fn new(addr: u64, len: u64) -> Self {
        Self { addr, length: len, total_syncs: 0, total_pages_synced: 0, total_bytes: 0, last_sync: 0, dirty_pages: 0 }
    }

    pub fn record_sync(&mut self, pages: u32, ts: u64) {
        self.total_syncs += 1;
        self.total_pages_synced += pages as u64;
        self.total_bytes += (pages as u64) * 4096;
        self.last_sync = ts;
        if self.dirty_pages >= pages { self.dirty_pages -= pages; }
        else { self.dirty_pages = 0; }
    }

    pub fn mark_dirty(&mut self, pages: u32) { self.dirty_pages += pages; }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MsyncAppStats {
    pub tracked_mappings: u32,
    pub total_syncs: u64,
    pub total_pages_synced: u64,
    pub total_dirty_pages: u32,
    pub sync_ops: u64,
    pub async_ops: u64,
}

/// Main app msync
pub struct AppMsync {
    mappings: BTreeMap<u64, MappingSyncTracker>,
    sync_ops: u64,
    async_ops: u64,
}

impl AppMsync {
    pub fn new() -> Self { Self { mappings: BTreeMap::new(), sync_ops: 0, async_ops: 0 } }

    pub fn track_mapping(&mut self, addr: u64, len: u64) { self.mappings.insert(addr, MappingSyncTracker::new(addr, len)); }

    pub fn sync(&mut self, addr: u64, pages: u32, flag: MsyncFlag, ts: u64) {
        match flag { MsyncFlag::Sync => self.sync_ops += 1, MsyncFlag::Async => self.async_ops += 1, _ => {} }
        if let Some(m) = self.mappings.get_mut(&addr) { m.record_sync(pages, ts); }
    }

    pub fn mark_dirty(&mut self, addr: u64, pages: u32) {
        if let Some(m) = self.mappings.get_mut(&addr) { m.mark_dirty(pages); }
    }

    pub fn untrack(&mut self, addr: u64) { self.mappings.remove(&addr); }

    pub fn stats(&self) -> MsyncAppStats {
        let syncs: u64 = self.mappings.values().map(|m| m.total_syncs).sum();
        let pages: u64 = self.mappings.values().map(|m| m.total_pages_synced).sum();
        let dirty: u32 = self.mappings.values().map(|m| m.dirty_pages).sum();
        MsyncAppStats { tracked_mappings: self.mappings.len() as u32, total_syncs: syncs, total_pages_synced: pages, total_dirty_pages: dirty, sync_ops: self.sync_ops, async_ops: self.async_ops }
    }
}
