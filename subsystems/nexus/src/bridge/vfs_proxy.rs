//! # Bridge Vfs Proxy
//!
//! Virtual filesystem syscall optimization:
//! - Path lookup caching (dentry cache proxy)
//! - Stat result caching
//! - Directory read-ahead
//! - Negative dentry tracking
//! - Filesystem type awareness

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Filesystem operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsOp {
    Open,
    Close,
    Read,
    Write,
    Stat,
    Lstat,
    Readdir,
    Mkdir,
    Rmdir,
    Unlink,
    Rename,
    Symlink,
    Readlink,
    Chmod,
    Chown,
    Access,
    Getattr,
}

/// Cache result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheResult {
    Hit,
    Miss,
    NegativeHit,
    Stale,
}

/// Dentry cache entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DentryCacheEntry {
    /// Path hash (FNV-1a)
    pub path_hash: u64,
    /// Inode number (if found)
    pub inode: Option<u64>,
    /// Is negative (path doesn't exist)?
    pub negative: bool,
    /// Cached timestamp (ns)
    pub cached_ns: u64,
    /// TTL (ns)
    pub ttl_ns: u64,
    /// Hit count
    pub hits: u64,
    /// Filesystem type hash
    pub fs_type_hash: u64,
}

impl DentryCacheEntry {
    pub fn new(path: &str, inode: Option<u64>, now_ns: u64, ttl_ns: u64) -> Self {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in path.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        Self {
            path_hash: hash,
            inode,
            negative: inode.is_none(),
            cached_ns: now_ns,
            ttl_ns,
            hits: 0,
            fs_type_hash: 0,
        }
    }

    /// Is valid?
    #[inline(always)]
    pub fn is_valid(&self, now_ns: u64) -> bool {
        now_ns.saturating_sub(self.cached_ns) < self.ttl_ns
    }
}

/// Stat cache entry
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StatCacheEntry {
    /// Path hash
    pub path_hash: u64,
    /// File size
    pub size: u64,
    /// Mode
    pub mode: u32,
    /// UID
    pub uid: u32,
    /// GID
    pub gid: u32,
    /// Modification time (ns)
    pub mtime_ns: u64,
    /// Cached at (ns)
    pub cached_ns: u64,
    /// TTL (ns)
    pub ttl_ns: u64,
    /// Hits
    pub hits: u64,
}

impl StatCacheEntry {
    pub fn new(path_hash: u64, size: u64, mode: u32, now_ns: u64) -> Self {
        Self {
            path_hash,
            size,
            mode,
            uid: 0,
            gid: 0,
            mtime_ns: now_ns,
            cached_ns: now_ns,
            ttl_ns: 2_000_000_000, // 2s default
            hits: 0,
        }
    }

    #[inline(always)]
    pub fn is_valid(&self, now_ns: u64) -> bool {
        now_ns.saturating_sub(self.cached_ns) < self.ttl_ns
    }
}

/// Per-process VFS stats
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessVfsProfile {
    /// PID
    pub pid: u64,
    /// Operation counts
    op_counts: BTreeMap<u8, u64>,
    /// Current working directory hash
    pub cwd_hash: u64,
    /// Root directory hash
    pub root_hash: u64,
    /// Open files count
    pub open_files: u32,
    /// Total path lookups
    pub total_lookups: u64,
    /// Cache hits
    pub cache_hits: u64,
}

impl ProcessVfsProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            op_counts: BTreeMap::new(),
            cwd_hash: 0,
            root_hash: 0,
            open_files: 0,
            total_lookups: 0,
            cache_hits: 0,
        }
    }

    /// Record operation
    #[inline(always)]
    pub fn record_op(&mut self, op: VfsOp) {
        *self.op_counts.entry(op as u8).or_insert(0) += 1;
    }

    /// Hit ratio
    #[inline]
    pub fn hit_ratio(&self) -> f64 {
        if self.total_lookups == 0 {
            return 0.0;
        }
        self.cache_hits as f64 / self.total_lookups as f64
    }

    /// Most common operation
    #[inline]
    pub fn most_common_op(&self) -> Option<u8> {
        self.op_counts.iter()
            .max_by_key(|&(_, &count)| count)
            .map(|(&op, _)| op)
    }
}

/// VFS proxy stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeVfsProxyStats {
    pub dentry_cache_size: usize,
    pub stat_cache_size: usize,
    pub negative_entries: usize,
    pub total_lookups: u64,
    pub cache_hits: u64,
    pub hit_ratio: f64,
    pub tracked_processes: usize,
}

/// Bridge VFS proxy
#[repr(align(64))]
pub struct BridgeVfsProxy {
    /// Dentry cache
    dentry_cache: BTreeMap<u64, DentryCacheEntry>,
    /// Stat cache
    stat_cache: BTreeMap<u64, StatCacheEntry>,
    /// Process profiles
    processes: BTreeMap<u64, ProcessVfsProfile>,
    /// Max cache size
    max_dentry_cache: usize,
    max_stat_cache: usize,
    /// Total lookups
    total_lookups: u64,
    /// Cache hits
    cache_hits: u64,
    /// Stats
    stats: BridgeVfsProxyStats,
}

impl BridgeVfsProxy {
    pub fn new() -> Self {
        Self {
            dentry_cache: BTreeMap::new(),
            stat_cache: BTreeMap::new(),
            processes: BTreeMap::new(),
            max_dentry_cache: 4096,
            max_stat_cache: 2048,
            total_lookups: 0,
            cache_hits: 0,
            stats: BridgeVfsProxyStats::default(),
        }
    }

    /// Hash a path (FNV-1a)
    fn hash_path(path: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in path.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Lookup dentry
    pub fn lookup_dentry(&mut self, path: &str, now_ns: u64) -> CacheResult {
        self.total_lookups += 1;
        let hash = Self::hash_path(path);

        if let Some(entry) = self.dentry_cache.get_mut(&hash) {
            if entry.is_valid(now_ns) {
                entry.hits += 1;
                self.cache_hits += 1;
                self.update_stats();
                if entry.negative {
                    return CacheResult::NegativeHit;
                }
                return CacheResult::Hit;
            }
            self.update_stats();
            return CacheResult::Stale;
        }
        self.update_stats();
        CacheResult::Miss
    }

    /// Insert dentry
    #[inline]
    pub fn insert_dentry(&mut self, path: &str, inode: Option<u64>, now_ns: u64) {
        if self.dentry_cache.len() >= self.max_dentry_cache {
            self.evict_dentry();
        }
        let entry = DentryCacheEntry::new(path, inode, now_ns, 5_000_000_000);
        self.dentry_cache.insert(entry.path_hash, entry);
        self.update_stats();
    }

    /// Lookup stat
    #[inline]
    pub fn lookup_stat(&mut self, path: &str, now_ns: u64) -> Option<&StatCacheEntry> {
        let hash = Self::hash_path(path);
        if let Some(entry) = self.stat_cache.get_mut(&hash) {
            if entry.is_valid(now_ns) {
                entry.hits += 1;
                return Some(entry);
            }
        }
        None
    }

    /// Insert stat
    #[inline]
    pub fn insert_stat(&mut self, path: &str, size: u64, mode: u32, now_ns: u64) {
        if self.stat_cache.len() >= self.max_stat_cache {
            self.evict_stat();
        }
        let hash = Self::hash_path(path);
        self.stat_cache.insert(hash, StatCacheEntry::new(hash, size, mode, now_ns));
        self.update_stats();
    }

    /// Invalidate path
    #[inline]
    pub fn invalidate(&mut self, path: &str) {
        let hash = Self::hash_path(path);
        self.dentry_cache.remove(&hash);
        self.stat_cache.remove(&hash);
        self.update_stats();
    }

    fn evict_dentry(&mut self) {
        // Evict least-hit entry
        if let Some(&key) = self.dentry_cache.iter()
            .min_by_key(|(_, v)| v.hits)
            .map(|(k, _)| k)
        {
            self.dentry_cache.remove(&key);
        }
    }

    fn evict_stat(&mut self) {
        if let Some(&key) = self.stat_cache.iter()
            .min_by_key(|(_, v)| v.hits)
            .map(|(k, _)| k)
        {
            self.stat_cache.remove(&key);
        }
    }

    fn update_stats(&mut self) {
        self.stats.dentry_cache_size = self.dentry_cache.len();
        self.stats.stat_cache_size = self.stat_cache.len();
        self.stats.negative_entries = self.dentry_cache.values().filter(|e| e.negative).count();
        self.stats.total_lookups = self.total_lookups;
        self.stats.cache_hits = self.cache_hits;
        self.stats.hit_ratio = if self.total_lookups > 0 {
            self.cache_hits as f64 / self.total_lookups as f64
        } else {
            0.0
        };
        self.stats.tracked_processes = self.processes.len();
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeVfsProxyStats {
        &self.stats
    }
}
