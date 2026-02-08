// SPDX-License-Identifier: GPL-2.0
//! NEXUS Apps — Stat App (file status and metadata tracking)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// File type from stat
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatFileType {
    Regular,
    Directory,
    Symlink,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    Unknown,
}

/// A stat result structure
#[derive(Debug, Clone)]
pub struct StatResult {
    pub dev: u64,
    pub ino: u64,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u64,
    pub size: u64,
    pub blksize: u32,
    pub blocks: u64,
    pub atime_ns: u64,
    pub mtime_ns: u64,
    pub ctime_ns: u64,
    pub file_type: StatFileType,
}

impl StatResult {
    pub fn new(ino: u64, mode: u32, size: u64, file_type: StatFileType) -> Self {
        Self {
            dev: 0, ino, mode, nlink: 1,
            uid: 0, gid: 0, rdev: 0,
            size, blksize: 4096, blocks: (size + 511) / 512,
            atime_ns: 0, mtime_ns: 0, ctime_ns: 0,
            file_type,
        }
    }

    pub fn is_regular(&self) -> bool { self.file_type == StatFileType::Regular }
    pub fn is_directory(&self) -> bool { self.file_type == StatFileType::Directory }
    pub fn is_symlink(&self) -> bool { self.file_type == StatFileType::Symlink }
}

/// Stat cache entry
#[derive(Debug, Clone)]
pub struct StatCacheEntry {
    pub path_hash: u64,
    pub result: StatResult,
    pub cached_tick: u64,
    pub hits: u64,
}

/// Statistics for stat app
#[derive(Debug, Clone)]
pub struct StatAppStats {
    pub stat_calls: u64,
    pub lstat_calls: u64,
    pub fstat_calls: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub not_found: u64,
    pub permission_denied: u64,
}

/// Main stat app manager
#[derive(Debug)]
pub struct AppStat {
    cache: BTreeMap<u64, StatCacheEntry>,
    cache_capacity: usize,
    stats: StatAppStats,
}

impl AppStat {
    pub fn new(cache_capacity: usize) -> Self {
        Self {
            cache: BTreeMap::new(),
            cache_capacity,
            stats: StatAppStats {
                stat_calls: 0, lstat_calls: 0, fstat_calls: 0,
                cache_hits: 0, cache_misses: 0,
                not_found: 0, permission_denied: 0,
            },
        }
    }

    fn hash_path(path: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn stat(&mut self, path: &str, tick: u64) -> Option<&StatResult> {
        self.stats.stat_calls += 1;
        let hash = Self::hash_path(path);
        if let Some(entry) = self.cache.get_mut(&hash) {
            entry.hits += 1;
            self.stats.cache_hits += 1;
            return Some(&entry.result);
        }
        self.stats.cache_misses += 1;
        None
    }

    pub fn cache_result(&mut self, path: &str, result: StatResult, tick: u64) {
        let hash = Self::hash_path(path);
        if self.cache.len() >= self.cache_capacity {
            if let Some(oldest) = self.cache.keys().next().copied() {
                self.cache.remove(&oldest);
            }
        }
        self.cache.insert(hash, StatCacheEntry {
            path_hash: hash,
            result,
            cached_tick: tick,
            hits: 0,
        });
    }

    pub fn invalidate(&mut self, path: &str) {
        let hash = Self::hash_path(path);
        self.cache.remove(&hash);
    }

    pub fn stats(&self) -> &StatAppStats {
        &self.stats
    }
}

// ============================================================================
// Merged from stat_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatV2Call {
    Stat,
    Fstat,
    Lstat,
    Fstatat,
    Statx,
    NewStat,
}

/// Stat v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatV2Result {
    Success,
    NotFound,
    PermissionDenied,
    NameTooLong,
    NotDir,
    Loop,
    Fault,
    Error,
}

/// Stat v2 record
#[derive(Debug, Clone)]
pub struct StatV2Record {
    pub call: StatV2Call,
    pub result: StatV2Result,
    pub path_hash: u64,
    pub inode: u64,
    pub size: u64,
    pub blocks: u64,
    pub nlink: u32,
    pub mode: u32,
    pub latency_ns: u64,
}

impl StatV2Record {
    pub fn new(call: StatV2Call, path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { call, result: StatV2Result::Success, path_hash: h, inode: 0, size: 0, blocks: 0, nlink: 1, mode: 0, latency_ns: 0 }
    }

    pub fn is_symlink_aware(&self) -> bool {
        matches!(self.call, StatV2Call::Lstat | StatV2Call::Fstatat | StatV2Call::Statx)
    }
}

/// Stat v2 app stats
#[derive(Debug, Clone)]
pub struct StatV2AppStats {
    pub total_calls: u64,
    pub stat_calls: u64,
    pub statx_calls: u64,
    pub errors: u64,
    pub total_latency_ns: u64,
}

/// Main app stat v2
#[derive(Debug)]
pub struct AppStatV2 {
    pub stats: StatV2AppStats,
}

impl AppStatV2 {
    pub fn new() -> Self {
        Self { stats: StatV2AppStats { total_calls: 0, stat_calls: 0, statx_calls: 0, errors: 0, total_latency_ns: 0 } }
    }

    pub fn record(&mut self, rec: &StatV2Record) {
        self.stats.total_calls += 1;
        self.stats.total_latency_ns += rec.latency_ns;
        match rec.call {
            StatV2Call::Stat | StatV2Call::Fstat | StatV2Call::Lstat | StatV2Call::Fstatat | StatV2Call::NewStat => self.stats.stat_calls += 1,
            StatV2Call::Statx => self.stats.statx_calls += 1,
        }
        if rec.result != StatV2Result::Success { self.stats.errors += 1; }
    }

    pub fn avg_latency_ns(&self) -> u64 {
        if self.stats.total_calls == 0 { 0 } else { self.stats.total_latency_ns / self.stats.total_calls }
    }
}

// ============================================================================
// Merged from stat_v3_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppFileTypeV3 {
    Regular,
    Directory,
    Symlink,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    Unknown,
}

/// File stat information
#[derive(Debug, Clone)]
pub struct AppStatInfo {
    pub inode: u64,
    pub file_type: AppFileTypeV3,
    pub permissions: u32,
    pub hard_links: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub block_size: u32,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub dev: u64,
    pub rdev: u64,
}

/// Statistics for stat operations
#[derive(Debug, Clone)]
pub struct AppStatOpStats {
    pub total_stats: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub stat_errors: u64,
    pub lstat_calls: u64,
    pub fstat_calls: u64,
}

/// Manager for stat application operations
pub struct AppStatV3Manager {
    stat_cache: BTreeMap<u64, AppStatInfo>,
    path_to_inode: BTreeMap<u64, u64>,
    stats: AppStatOpStats,
}

impl AppStatV3Manager {
    pub fn new() -> Self {
        Self {
            stat_cache: BTreeMap::new(),
            path_to_inode: BTreeMap::new(),
            stats: AppStatOpStats {
                total_stats: 0,
                cache_hits: 0,
                cache_misses: 0,
                stat_errors: 0,
                lstat_calls: 0,
                fstat_calls: 0,
            },
        }
    }

    fn hash_path(path: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn cache_stat(&mut self, path: &str, info: AppStatInfo) {
        let hash = Self::hash_path(path);
        self.path_to_inode.insert(hash, info.inode);
        self.stat_cache.insert(info.inode, info);
    }

    pub fn stat_path(&mut self, path: &str) -> Option<&AppStatInfo> {
        self.stats.total_stats += 1;
        let hash = Self::hash_path(path);
        if let Some(&inode) = self.path_to_inode.get(&hash) {
            self.stats.cache_hits += 1;
            self.stat_cache.get(&inode)
        } else {
            self.stats.cache_misses += 1;
            None
        }
    }

    pub fn fstat(&mut self, inode: u64) -> Option<&AppStatInfo> {
        self.stats.fstat_calls += 1;
        self.stats.total_stats += 1;
        if self.stat_cache.contains_key(&inode) {
            self.stats.cache_hits += 1;
            self.stat_cache.get(&inode)
        } else {
            self.stats.cache_misses += 1;
            None
        }
    }

    pub fn lstat(&mut self, path: &str) -> Option<&AppStatInfo> {
        self.stats.lstat_calls += 1;
        self.stat_path(path)
    }

    pub fn invalidate(&mut self, path: &str) {
        let hash = Self::hash_path(path);
        if let Some(inode) = self.path_to_inode.remove(&hash) {
            self.stat_cache.remove(&inode);
        }
    }

    pub fn stats(&self) -> &AppStatOpStats {
        &self.stats
    }
}
