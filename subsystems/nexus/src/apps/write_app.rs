// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Write (filesystem write operations)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Write operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppWriteMode {
    Overwrite,
    Append,
    Positional,
    Vectored,
    DirectIO,
    Buffered,
    Sync,
    Datasync,
}

/// Write request descriptor
#[derive(Debug, Clone)]
pub struct AppWriteRequest {
    pub fd: u64,
    pub offset: u64,
    pub length: usize,
    pub write_mode: AppWriteMode,
    pub priority: u8,
    pub sync_on_complete: bool,
    pub timestamp: u64,
}

/// Write completion result
#[derive(Debug, Clone)]
pub struct AppWriteCompletion {
    pub request_id: u64,
    pub bytes_written: usize,
    pub latency_us: u64,
    pub synced: bool,
    pub error_code: i32,
}

/// Statistics for write operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppWriteStats {
    pub total_writes: u64,
    pub total_bytes_written: u64,
    pub sync_writes: u64,
    pub async_writes: u64,
    pub avg_latency_us: u64,
    pub write_errors: u64,
}

/// Manager for application-level write operations
pub struct AppWriteManager {
    pending_writes: BTreeMap<u64, AppWriteRequest>,
    completions: Vec<AppWriteCompletion>,
    next_id: u64,
    stats: AppWriteStats,
    write_buffer_size: usize,
    dirty_bytes: u64,
}

impl AppWriteManager {
    pub fn new() -> Self {
        Self {
            pending_writes: BTreeMap::new(),
            completions: Vec::new(),
            next_id: 1,
            stats: AppWriteStats {
                total_writes: 0,
                total_bytes_written: 0,
                sync_writes: 0,
                async_writes: 0,
                avg_latency_us: 0,
                write_errors: 0,
            },
            write_buffer_size: 65536,
            dirty_bytes: 0,
        }
    }

    pub fn submit_write(&mut self, fd: u64, offset: u64, length: usize, mode: AppWriteMode) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let sync = matches!(mode, AppWriteMode::Sync | AppWriteMode::Datasync);
        let req = AppWriteRequest {
            fd,
            offset,
            length,
            write_mode: mode,
            priority: 5,
            sync_on_complete: sync,
            timestamp: id.wrapping_mul(37),
        };
        self.pending_writes.insert(id, req);
        self.stats.total_writes += 1;
        self.dirty_bytes += length as u64;
        if sync {
            self.stats.sync_writes += 1;
        } else {
            self.stats.async_writes += 1;
        }
        id
    }

    pub fn complete_write(&mut self, request_id: u64, bytes_written: usize) -> bool {
        if let Some(req) = self.pending_writes.remove(&request_id) {
            let latency = if req.sync_on_complete { 500 } else { 50 };
            let completion = AppWriteCompletion {
                request_id,
                bytes_written,
                latency_us: latency,
                synced: req.sync_on_complete,
                error_code: 0,
            };
            self.completions.push(completion);
            self.stats.total_bytes_written += bytes_written as u64;
            self.dirty_bytes = self.dirty_bytes.saturating_sub(bytes_written as u64);
            true
        } else {
            false
        }
    }

    #[inline]
    pub fn flush_dirty(&mut self) -> u64 {
        let flushed = self.dirty_bytes;
        self.dirty_bytes = 0;
        flushed
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppWriteStats {
        &self.stats
    }
}
