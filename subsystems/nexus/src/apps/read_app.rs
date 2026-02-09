// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Read (filesystem read operations)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Read operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppReadType {
    Sequential,
    Random,
    Positional,
    Vectored,
    DirectIO,
    Buffered,
    MmapRead,
    Readahead,
}

/// Read request descriptor
#[derive(Debug, Clone)]
pub struct AppReadRequest {
    pub fd: u64,
    pub offset: u64,
    pub length: usize,
    pub read_type: AppReadType,
    pub priority: u8,
    pub timestamp: u64,
}

/// Read completion info
#[derive(Debug, Clone)]
pub struct AppReadCompletion {
    pub request_id: u64,
    pub bytes_read: usize,
    pub latency_us: u64,
    pub from_cache: bool,
    pub error_code: i32,
}

/// Statistics for read operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppReadStats {
    pub total_reads: u64,
    pub total_bytes_read: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub avg_latency_us: u64,
    pub peak_throughput_mbps: u64,
}

/// Manager for application-level read operations
pub struct AppReadManager {
    pending_reads: BTreeMap<u64, AppReadRequest>,
    completions: Vec<AppReadCompletion>,
    next_id: u64,
    stats: AppReadStats,
    readahead_window: usize,
}

impl AppReadManager {
    pub fn new() -> Self {
        Self {
            pending_reads: BTreeMap::new(),
            completions: Vec::new(),
            next_id: 1,
            stats: AppReadStats {
                total_reads: 0,
                total_bytes_read: 0,
                cache_hits: 0,
                cache_misses: 0,
                avg_latency_us: 0,
                peak_throughput_mbps: 0,
            },
            readahead_window: 4096,
        }
    }

    pub fn submit_read(&mut self, fd: u64, offset: u64, length: usize, read_type: AppReadType) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let req = AppReadRequest {
            fd,
            offset,
            length,
            read_type,
            priority: 5,
            timestamp: id.wrapping_mul(31),
        };
        self.pending_reads.insert(id, req);
        self.stats.total_reads += 1;
        id
    }

    pub fn complete_read(&mut self, request_id: u64, bytes_read: usize, from_cache: bool) -> bool {
        if let Some(_req) = self.pending_reads.remove(&request_id) {
            let latency = if from_cache { 5 } else { 150 };
            let completion = AppReadCompletion {
                request_id,
                bytes_read,
                latency_us: latency,
                from_cache,
                error_code: 0,
            };
            self.completions.push(completion);
            self.stats.total_bytes_read += bytes_read as u64;
            if from_cache {
                self.stats.cache_hits += 1;
            } else {
                self.stats.cache_misses += 1;
            }
            true
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn set_readahead(&mut self, window: usize) {
        self.readahead_window = window;
    }

    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending_reads.len()
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppReadStats {
        &self.stats
    }
}
