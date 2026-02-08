// SPDX-License-Identifier: GPL-2.0
//! Apps readlink_app — symbolic link resolution.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Readlink result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadlinkResult {
    Success,
    NotSymlink,
    NotFound,
    PermissionDenied,
    LoopDetected,
    TooLong,
}

/// Symlink resolution entry
#[derive(Debug)]
pub struct SymlinkResolution {
    pub path_hash: u64,
    pub target_hash: u64,
    pub depth: u32,
    pub result: ReadlinkResult,
    pub timestamp: u64,
}

/// Symlink cache
#[derive(Debug)]
pub struct SymlinkCache {
    pub entries: BTreeMap<u64, u64>,
    pub max_entries: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
}

impl SymlinkCache {
    pub fn new(max: usize) -> Self { Self { entries: BTreeMap::new(), max_entries: max, hits: 0, misses: 0, evictions: 0 } }

    pub fn lookup(&mut self, path_hash: u64) -> Option<u64> {
        if let Some(&target) = self.entries.get(&path_hash) { self.hits += 1; Some(target) }
        else { self.misses += 1; None }
    }

    pub fn insert(&mut self, path_hash: u64, target_hash: u64) {
        if self.entries.len() >= self.max_entries {
            if let Some(&first) = self.entries.keys().next() { self.entries.remove(&first); self.evictions += 1; }
        }
        self.entries.insert(path_hash, target_hash);
    }

    pub fn hit_rate(&self) -> f64 { let total = self.hits + self.misses; if total == 0 { 0.0 } else { self.hits as f64 / total as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct ReadlinkAppStats {
    pub total_resolutions: u64,
    pub cache_hit_rate: f64,
    pub max_depth_seen: u32,
    pub loop_count: u64,
}

/// Main readlink app
pub struct AppReadlink {
    cache: SymlinkCache,
    total_resolutions: u64,
    max_depth: u32,
    loops: u64,
}

impl AppReadlink {
    pub fn new(cache_size: usize) -> Self { Self { cache: SymlinkCache::new(cache_size), total_resolutions: 0, max_depth: 0, loops: 0 } }

    pub fn resolve(&mut self, path_hash: u64, target_hash: u64, depth: u32, result: ReadlinkResult) {
        self.total_resolutions += 1;
        if depth > self.max_depth { self.max_depth = depth; }
        if result == ReadlinkResult::LoopDetected { self.loops += 1; }
        if result == ReadlinkResult::Success { self.cache.insert(path_hash, target_hash); }
    }

    pub fn stats(&self) -> ReadlinkAppStats {
        ReadlinkAppStats { total_resolutions: self.total_resolutions, cache_hit_rate: self.cache.hit_rate(), max_depth_seen: self.max_depth, loop_count: self.loops }
    }
}

// ============================================================================
// Merged from readlink_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadlinkV2Result {
    Success,
    NotSymlink,
    NotFound,
    PermissionDenied,
    BufferTooSmall,
    Error,
}

/// Readlink record
#[derive(Debug, Clone)]
pub struct ReadlinkV2Record {
    pub path_hash: u64,
    pub result: ReadlinkV2Result,
    pub target_hash: u64,
    pub target_len: u32,
    pub buf_size: u32,
    pub cached: bool,
    pub duration_ns: u64,
}

impl ReadlinkV2Record {
    pub fn new(path: &[u8], buf_size: u32) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self {
            path_hash: h,
            result: ReadlinkV2Result::Success,
            target_hash: 0,
            target_len: 0,
            buf_size,
            cached: false,
            duration_ns: 0,
        }
    }

    pub fn was_truncated(&self) -> bool {
        self.target_len > self.buf_size
    }
}

/// Readlink cache entry
#[derive(Debug, Clone)]
pub struct ReadlinkCacheEntry {
    pub path_hash: u64,
    pub target_hash: u64,
    pub target_len: u32,
    pub hit_count: u64,
    pub last_used_ns: u64,
    pub valid: bool,
}

impl ReadlinkCacheEntry {
    pub fn new(path_hash: u64, target_hash: u64, target_len: u32) -> Self {
        Self { path_hash, target_hash, target_len, hit_count: 0, last_used_ns: 0, valid: true }
    }

    pub fn touch(&mut self, ts_ns: u64) {
        self.hit_count += 1;
        self.last_used_ns = ts_ns;
    }

    pub fn invalidate(&mut self) {
        self.valid = false;
    }
}

/// Readlink v2 app stats
#[derive(Debug, Clone)]
pub struct ReadlinkV2AppStats {
    pub total_ops: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub truncations: u64,
    pub failures: u64,
}

/// Main app readlink v2
#[derive(Debug)]
pub struct AppReadlinkV2 {
    pub cache: BTreeMap<u64, ReadlinkCacheEntry>,
    pub stats: ReadlinkV2AppStats,
    pub max_cache: u32,
}

impl AppReadlinkV2 {
    pub fn new(max_cache: u32) -> Self {
        Self {
            cache: BTreeMap::new(),
            stats: ReadlinkV2AppStats {
                total_ops: 0,
                cache_hits: 0,
                cache_misses: 0,
                truncations: 0,
                failures: 0,
            },
            max_cache,
        }
    }

    pub fn record(&mut self, record: &ReadlinkV2Record, ts_ns: u64) {
        self.stats.total_ops += 1;
        match record.result {
            ReadlinkV2Result::Success => {
                if record.cached {
                    self.stats.cache_hits += 1;
                    if let Some(entry) = self.cache.get_mut(&record.path_hash) {
                        entry.touch(ts_ns);
                    }
                } else {
                    self.stats.cache_misses += 1;
                    if self.cache.len() < self.max_cache as usize {
                        self.cache.insert(record.path_hash,
                            ReadlinkCacheEntry::new(record.path_hash, record.target_hash, record.target_len));
                    }
                }
                if record.was_truncated() {
                    self.stats.truncations += 1;
                }
            }
            _ => self.stats.failures += 1,
        }
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.stats.cache_hits + self.stats.cache_misses;
        if total == 0 { 0.0 } else { self.stats.cache_hits as f64 / total as f64 }
    }
}
