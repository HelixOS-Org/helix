// SPDX-License-Identifier: GPL-2.0
//! App fstat — per-fd stat tracking with cache

extern crate alloc;
use alloc::collections::BTreeMap;

/// Fstat result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FstatResult {
    Success,
    BadFd,
    Fault,
    Error,
}

/// Fstat record
#[derive(Debug, Clone)]
pub struct FstatRecord {
    pub fd: i32,
    pub result: FstatResult,
    pub inode: u64,
    pub size: u64,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub latency_ns: u64,
}

impl FstatRecord {
    pub fn new(fd: i32) -> Self {
        Self {
            fd,
            result: FstatResult::Success,
            inode: 0,
            size: 0,
            mode: 0,
            nlink: 1,
            uid: 0,
            gid: 0,
            latency_ns: 0,
        }
    }
}

/// Fstat cache entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FstatCacheEntry {
    pub inode: u64,
    pub size: u64,
    pub mode: u32,
    pub hits: u64,
    pub last_refresh_ns: u64,
}

impl FstatCacheEntry {
    pub fn new(inode: u64, size: u64, mode: u32) -> Self {
        Self {
            inode,
            size,
            mode,
            hits: 0,
            last_refresh_ns: 0,
        }
    }

    #[inline(always)]
    pub fn hit(&mut self) {
        self.hits += 1;
    }

    #[inline]
    pub fn refresh(&mut self, size: u64, mode: u32, now_ns: u64) {
        self.size = size;
        self.mode = mode;
        self.last_refresh_ns = now_ns;
    }
}

/// Fstat app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FstatAppStats {
    pub total_calls: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub errors: u64,
}

/// Main app fstat
#[derive(Debug)]
pub struct AppFstat {
    pub cache: BTreeMap<i32, FstatCacheEntry>,
    pub stats: FstatAppStats,
}

impl AppFstat {
    pub fn new() -> Self {
        Self {
            cache: BTreeMap::new(),
            stats: FstatAppStats {
                total_calls: 0,
                cache_hits: 0,
                cache_misses: 0,
                errors: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &FstatRecord) {
        self.stats.total_calls += 1;
        if rec.result != FstatResult::Success {
            self.stats.errors += 1;
            return;
        }
        if let Some(entry) = self.cache.get_mut(&rec.fd) {
            entry.hit();
            self.stats.cache_hits += 1;
        } else {
            self.cache
                .insert(rec.fd, FstatCacheEntry::new(rec.inode, rec.size, rec.mode));
            self.stats.cache_misses += 1;
        }
    }

    #[inline]
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.stats.cache_hits + self.stats.cache_misses;
        if total == 0 {
            0.0
        } else {
            self.stats.cache_hits as f64 / total as f64
        }
    }
}
