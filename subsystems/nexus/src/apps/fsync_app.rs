// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Fsync (filesystem sync operations)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Sync operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppSyncType {
    Fsync,
    Fdatasync,
    SyncFs,
    SyncRange,
    MsyncSync,
    MsyncAsync,
}

/// Sync request descriptor
#[derive(Debug, Clone)]
pub struct AppSyncRequest {
    pub fd: u64,
    pub sync_type: AppSyncType,
    pub offset: u64,
    pub length: u64,
    pub timestamp: u64,
}

/// Sync completion result
#[derive(Debug, Clone)]
pub struct AppSyncCompletion {
    pub request_id: u64,
    pub latency_us: u64,
    pub bytes_synced: u64,
    pub success: bool,
}

/// Statistics for sync operations
#[derive(Debug, Clone)]
pub struct AppSyncStats {
    pub total_syncs: u64,
    pub fsync_count: u64,
    pub fdatasync_count: u64,
    pub syncfs_count: u64,
    pub avg_sync_latency_us: u64,
    pub total_bytes_synced: u64,
    pub sync_errors: u64,
}

/// Manager for filesystem sync operations
pub struct AppFsyncManager {
    pending_syncs: BTreeMap<u64, AppSyncRequest>,
    completions: Vec<AppSyncCompletion>,
    next_id: u64,
    stats: AppSyncStats,
}

impl AppFsyncManager {
    pub fn new() -> Self {
        Self {
            pending_syncs: BTreeMap::new(),
            completions: Vec::new(),
            next_id: 1,
            stats: AppSyncStats {
                total_syncs: 0,
                fsync_count: 0,
                fdatasync_count: 0,
                syncfs_count: 0,
                avg_sync_latency_us: 0,
                total_bytes_synced: 0,
                sync_errors: 0,
            },
        }
    }

    pub fn submit_sync(&mut self, fd: u64, sync_type: AppSyncType, offset: u64, length: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let req = AppSyncRequest {
            fd,
            sync_type,
            offset,
            length,
            timestamp: id.wrapping_mul(59),
        };
        self.pending_syncs.insert(id, req);
        self.stats.total_syncs += 1;
        match sync_type {
            AppSyncType::Fsync => self.stats.fsync_count += 1,
            AppSyncType::Fdatasync => self.stats.fdatasync_count += 1,
            AppSyncType::SyncFs => self.stats.syncfs_count += 1,
            _ => {}
        }
        id
    }

    pub fn complete_sync(&mut self, request_id: u64, bytes_synced: u64) -> bool {
        if let Some(_req) = self.pending_syncs.remove(&request_id) {
            let completion = AppSyncCompletion {
                request_id,
                latency_us: 200,
                bytes_synced,
                success: true,
            };
            self.completions.push(completion);
            self.stats.total_bytes_synced += bytes_synced;
            true
        } else {
            false
        }
    }

    pub fn sync_all(&mut self) -> usize {
        let ids: Vec<u64> = self.pending_syncs.keys().cloned().collect();
        let count = ids.len();
        for id in ids {
            if let Some(_req) = self.pending_syncs.remove(&id) {
                let completion = AppSyncCompletion {
                    request_id: id,
                    latency_us: 500,
                    bytes_synced: 0,
                    success: true,
                };
                self.completions.push(completion);
            }
        }
        count
    }

    pub fn pending_count(&self) -> usize {
        self.pending_syncs.len()
    }

    pub fn stats(&self) -> &AppSyncStats {
        &self.stats
    }
}
