// SPDX-License-Identifier: GPL-2.0
//! Holistic page cache — page cache efficiency and eviction analysis

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Page cache operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageCacheOp {
    Add,
    Lookup,
    Evict,
    Activate,
    Deactivate,
    Writeback,
    Invalidate,
    Readahead,
}

/// Page cache list
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageCacheList {
    Active,
    Inactive,
    Unevictable,
}

/// Per-file page cache entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FileCacheProfile {
    pub inode: u64,
    pub cached_pages: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub readahead_pages: u64,
}

impl FileCacheProfile {
    pub fn new(inode: u64) -> Self {
        Self {
            inode,
            cached_pages: 0,
            hits: 0,
            misses: 0,
            evictions: 0,
            readahead_pages: 0,
        }
    }

    #[inline(always)]
    pub fn add_page(&mut self) {
        self.cached_pages += 1;
    }
    #[inline]
    pub fn evict_page(&mut self) {
        if self.cached_pages > 0 {
            self.cached_pages -= 1;
        }
        self.evictions += 1;
    }
    #[inline(always)]
    pub fn hit(&mut self) {
        self.hits += 1;
    }
    #[inline(always)]
    pub fn miss(&mut self) {
        self.misses += 1;
    }

    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Holistic page cache stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticPageCacheStats {
    pub total_pages: u64,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub readahead_pages: u64,
    pub active_pages: u64,
    pub inactive_pages: u64,
}

/// Main holistic page cache
#[derive(Debug)]
#[repr(align(64))]
pub struct HolisticPageCache {
    pub files: BTreeMap<u64, FileCacheProfile>,
    pub stats: HolisticPageCacheStats,
}

impl HolisticPageCache {
    pub fn new() -> Self {
        Self {
            files: BTreeMap::new(),
            stats: HolisticPageCacheStats {
                total_pages: 0,
                hits: 0,
                misses: 0,
                evictions: 0,
                readahead_pages: 0,
                active_pages: 0,
                inactive_pages: 0,
            },
        }
    }

    pub fn record(&mut self, op: PageCacheOp, inode: u64) {
        let profile = self
            .files
            .entry(inode)
            .or_insert_with(|| FileCacheProfile::new(inode));
        match op {
            PageCacheOp::Add => {
                profile.add_page();
                self.stats.total_pages += 1;
                self.stats.inactive_pages += 1;
            },
            PageCacheOp::Lookup => {
                profile.hit();
                self.stats.hits += 1;
            },
            PageCacheOp::Evict => {
                profile.evict_page();
                self.stats.evictions += 1;
            },
            PageCacheOp::Activate => {
                self.stats.active_pages += 1;
                self.stats.inactive_pages = self.stats.inactive_pages.saturating_sub(1);
            },
            PageCacheOp::Readahead => {
                profile.readahead_pages += 1;
                self.stats.readahead_pages += 1;
            },
            _ => {},
        }
    }

    #[inline]
    pub fn hit_rate(&self) -> f64 {
        let total = self.stats.hits + self.stats.misses;
        if total == 0 {
            0.0
        } else {
            self.stats.hits as f64 / total as f64
        }
    }
}
