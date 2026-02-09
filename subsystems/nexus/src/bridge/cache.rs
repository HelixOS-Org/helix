//! # Syscall Result Cache
//!
//! Caches results of idempotent and deterministic syscalls to avoid
//! redundant kernel entries. Particularly effective for:
//! - `stat()` / `fstat()` calls on unchanged files
//! - `getpid()`, `getuid()`, `gettimeofday()` (within tolerance)
//! - `readlink()` on stable symlinks
//! - Repeated `ioctl()` status queries
//! - Config file reads

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// CACHE KEY & ENTRY
// ============================================================================

/// Hash of syscall parameters used as cache key
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(align(64))]
pub struct CacheKey(pub u64);

impl CacheKey {
    /// Create a cache key from syscall type + arguments using FNV-1a
    pub fn from_args(syscall_type: SyscallType, args: &[u64]) -> Self {
        let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
        hash ^= syscall_type as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime

        for &arg in args {
            hash ^= arg;
            hash = hash.wrapping_mul(0x100000001b3);
        }

        Self(hash)
    }
}

/// Cacheability classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cacheability {
    /// Always cacheable (e.g., getpid)
    AlwaysCacheable,
    /// Cacheable with TTL (e.g., stat within modification window)
    TimeBounded { ttl_ms: u64 },
    /// Cacheable until an invalidation event
    EventInvalidated,
    /// Never cacheable
    NeverCacheable,
}

/// A cached syscall result
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CachedResult {
    /// Cache key
    pub key: CacheKey,
    /// Syscall type
    pub syscall_type: SyscallType,
    /// Process ID that populated this entry
    pub source_pid: u64,
    /// Return value
    pub return_value: i64,
    /// Output buffer (if any, limited size)
    pub output_data: Vec<u8>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last access timestamp
    pub last_accessed: u64,
    /// Number of cache hits
    pub hit_count: u64,
    /// TTL in milliseconds (0 = infinite)
    pub ttl_ms: u64,
    /// Size of this entry in bytes (approximate)
    pub size_bytes: usize,
}

impl CachedResult {
    pub fn new(
        key: CacheKey,
        syscall_type: SyscallType,
        source_pid: u64,
        return_value: i64,
        ttl_ms: u64,
    ) -> Self {
        Self {
            key,
            syscall_type,
            source_pid,
            return_value,
            output_data: Vec::new(),
            created_at: 0,
            last_accessed: 0,
            hit_count: 0,
            ttl_ms,
            size_bytes: 64, // base overhead
        }
    }

    #[inline]
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.size_bytes += data.len();
        self.output_data = data;
        self
    }

    #[inline]
    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.created_at = ts;
        self.last_accessed = ts;
        self
    }

    /// Check if this entry has expired
    #[inline]
    pub fn is_expired(&self, current_time: u64) -> bool {
        if self.ttl_ms == 0 {
            return false;
        }
        current_time.saturating_sub(self.created_at) > self.ttl_ms
    }

    /// Record a cache hit
    #[inline(always)]
    pub fn record_hit(&mut self, timestamp: u64) {
        self.hit_count += 1;
        self.last_accessed = timestamp;
    }

    /// LRU score (lower = better candidate for eviction)
    #[inline(always)]
    pub fn lru_score(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.last_accessed)
    }
}

// ============================================================================
// INVALIDATION
// ============================================================================

/// Event that invalidates cache entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationEvent {
    /// File was modified
    FileModified { inode: u64 },
    /// File was deleted
    FileDeleted { inode: u64 },
    /// Directory contents changed
    DirectoryChanged { inode: u64 },
    /// Process state changed (e.g., setuid)
    ProcessStateChanged { pid: u64 },
    /// Mount table changed
    MountChanged,
    /// Network configuration changed
    NetworkChanged,
    /// Time jumped (for time-based caches)
    TimeJump,
    /// Full cache flush requested
    FlushAll,
}

/// Invalidation subscription
#[derive(Debug, Clone)]
struct InvalidationSub {
    /// Cache keys to invalidate
    keys: Vec<CacheKey>,
    /// Event type to watch
    event_type: u8, // simplified discriminant
}

// ============================================================================
// PER-PROCESS CACHE
// ============================================================================

/// Per-process cache partition
#[derive(Debug)]
struct ProcessCache {
    pid: u64,
    entries: BTreeMap<CacheKey, CachedResult>,
    max_entries: usize,
    max_bytes: usize,
    current_bytes: usize,
    hits: u64,
    misses: u64,
}

impl ProcessCache {
    fn new(pid: u64, max_entries: usize, max_bytes: usize) -> Self {
        Self {
            pid,
            entries: BTreeMap::new(),
            max_entries,
            max_bytes,
            current_bytes: 0,
            hits: 0,
            misses: 0,
        }
    }

    fn get(&mut self, key: &CacheKey, current_time: u64) -> Option<&CachedResult> {
        // Check if exists and not expired
        let expired = self
            .entries
            .get(key)
            .map(|e| e.is_expired(current_time))
            .unwrap_or(true);

        if expired {
            if self.entries.contains_key(key) {
                let entry = self.entries.remove(key).unwrap();
                self.current_bytes -= entry.size_bytes;
            }
            self.misses += 1;
            return None;
        }

        if let Some(entry) = self.entries.get_mut(key) {
            entry.record_hit(current_time);
            self.hits += 1;
            Some(entry)
        } else {
            self.misses += 1;
            None
        }
    }

    fn insert(&mut self, entry: CachedResult, current_time: u64) {
        // Evict if at capacity
        while self.entries.len() >= self.max_entries
            || self.current_bytes + entry.size_bytes > self.max_bytes
        {
            if !self.evict_one(current_time) {
                break;
            }
        }

        self.current_bytes += entry.size_bytes;
        self.entries.insert(entry.key, entry);
    }

    fn evict_one(&mut self, current_time: u64) -> bool {
        // Find LRU entry
        let lru_key = self
            .entries
            .iter()
            .max_by_key(|(_, e)| e.lru_score(current_time))
            .map(|(k, _)| *k);

        if let Some(key) = lru_key {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_bytes -= entry.size_bytes;
                return true;
            }
        }
        false
    }

    fn invalidate(&mut self, key: &CacheKey) -> bool {
        if let Some(entry) = self.entries.remove(key) {
            self.current_bytes -= entry.size_bytes;
            true
        } else {
            false
        }
    }

    fn purge_expired(&mut self, current_time: u64) -> usize {
        let expired_keys: Vec<CacheKey> = self
            .entries
            .iter()
            .filter(|(_, e)| e.is_expired(current_time))
            .map(|(k, _)| *k)
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            if let Some(entry) = self.entries.remove(&key) {
                self.current_bytes -= entry.size_bytes;
            }
        }
        count
    }

    fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

// ============================================================================
// GLOBAL SYSCALL CACHE
// ============================================================================

/// Cacheability rules per syscall type
fn classify_cacheability(syscall_type: SyscallType) -> Cacheability {
    match syscall_type {
        // Always cacheable (process-invariant for its lifetime)
        SyscallType::Stat => Cacheability::TimeBounded { ttl_ms: 1000 },

        // Most syscalls are not cacheable
        SyscallType::Read | SyscallType::Write | SyscallType::Open | SyscallType::Close => {
            Cacheability::NeverCacheable
        },
        SyscallType::Mmap | SyscallType::Munmap | SyscallType::Brk => Cacheability::NeverCacheable,
        SyscallType::Fork | SyscallType::Exec | SyscallType::Exit => Cacheability::NeverCacheable,
        SyscallType::Connect | SyscallType::Accept | SyscallType::Send | SyscallType::Recv => {
            Cacheability::NeverCacheable
        },

        // Poll/Ioctl might be cacheable in specific contexts
        SyscallType::Ioctl => Cacheability::TimeBounded { ttl_ms: 100 },
        SyscallType::Poll => Cacheability::NeverCacheable,
        SyscallType::Seek => Cacheability::NeverCacheable,
        SyscallType::Fsync => Cacheability::NeverCacheable,

        // Default: not cacheable
        _ => Cacheability::NeverCacheable,
    }
}

/// Configuration for the syscall cache
#[derive(Debug, Clone, Copy)]
#[repr(align(64))]
pub struct SyscallCacheConfig {
    /// Max entries per process
    pub max_entries_per_process: usize,
    /// Max bytes per process
    pub max_bytes_per_process: usize,
    /// Max total processes
    pub max_processes: usize,
    /// Global cache enabled
    pub enabled: bool,
    /// Purge interval (ms)
    pub purge_interval_ms: u64,
}

impl Default for SyscallCacheConfig {
    fn default() -> Self {
        Self {
            max_entries_per_process: 256,
            max_bytes_per_process: 256 * 1024, // 256KB per process
            max_processes: 1024,
            enabled: true,
            purge_interval_ms: 5000,
        }
    }
}

/// Global syscall result cache
#[repr(align(64))]
pub struct SyscallCache {
    /// Per-process caches
    process_caches: BTreeMap<u64, ProcessCache>,
    /// Configuration
    config: SyscallCacheConfig,
    /// Global hit count
    global_hits: u64,
    /// Global miss count
    global_misses: u64,
    /// Global eviction count
    global_evictions: u64,
    /// Invalidation subscriptions (simplified)
    inode_to_keys: BTreeMap<u64, Vec<(u64, CacheKey)>>, // inode → [(pid, key)]
    /// Last purge timestamp
    last_purge: u64,
}

impl SyscallCache {
    pub fn new(config: SyscallCacheConfig) -> Self {
        Self {
            process_caches: BTreeMap::new(),
            config,
            global_hits: 0,
            global_misses: 0,
            global_evictions: 0,
            inode_to_keys: BTreeMap::new(),
            last_purge: 0,
        }
    }

    /// Look up a cached result
    pub fn lookup(
        &mut self,
        pid: u64,
        syscall_type: SyscallType,
        args: &[u64],
        current_time: u64,
    ) -> Option<&CachedResult> {
        if !self.config.enabled {
            return None;
        }

        // Check cacheability
        if classify_cacheability(syscall_type) == Cacheability::NeverCacheable {
            return None;
        }

        let key = CacheKey::from_args(syscall_type, args);
        let cache = self.process_caches.get_mut(&pid)?;

        match cache.get(&key, current_time) {
            Some(_) => {
                self.global_hits += 1;
                // Re-borrow as immutable
                cache.entries.get(&key)
            },
            None => {
                self.global_misses += 1;
                None
            },
        }
    }

    /// Store a result in the cache
    pub fn store(
        &mut self,
        pid: u64,
        syscall_type: SyscallType,
        args: &[u64],
        return_value: i64,
        output: Option<Vec<u8>>,
        current_time: u64,
    ) -> bool {
        if !self.config.enabled {
            return false;
        }

        let cacheability = classify_cacheability(syscall_type);
        let ttl = match cacheability {
            Cacheability::AlwaysCacheable => 0, // infinite
            Cacheability::TimeBounded { ttl_ms } => ttl_ms,
            Cacheability::EventInvalidated => 0,
            Cacheability::NeverCacheable => return false,
        };

        let key = CacheKey::from_args(syscall_type, args);
        let mut entry = CachedResult::new(key, syscall_type, pid, return_value, ttl)
            .with_timestamp(current_time);

        if let Some(data) = output {
            entry = entry.with_data(data);
        }

        // Ensure process cache exists
        if !self.process_caches.contains_key(&pid) {
            if self.process_caches.len() >= self.config.max_processes {
                return false;
            }
            self.process_caches.insert(
                pid,
                ProcessCache::new(
                    pid,
                    self.config.max_entries_per_process,
                    self.config.max_bytes_per_process,
                ),
            );
        }

        let cache = self.process_caches.get_mut(&pid).unwrap();
        cache.insert(entry, current_time);
        true
    }

    /// Handle an invalidation event
    pub fn invalidate(&mut self, event: InvalidationEvent) {
        match event {
            InvalidationEvent::FileModified { inode }
            | InvalidationEvent::FileDeleted { inode }
            | InvalidationEvent::DirectoryChanged { inode } => {
                if let Some(entries) = self.inode_to_keys.remove(&inode) {
                    for (pid, key) in entries {
                        if let Some(cache) = self.process_caches.get_mut(&pid) {
                            cache.invalidate(&key);
                            self.global_evictions += 1;
                        }
                    }
                }
            },
            InvalidationEvent::ProcessStateChanged { pid } => {
                self.process_caches.remove(&pid);
            },
            InvalidationEvent::FlushAll => {
                self.process_caches.clear();
                self.inode_to_keys.clear();
            },
            _ => {},
        }
    }

    /// Periodic purge of expired entries
    pub fn purge_expired(&mut self, current_time: u64) -> usize {
        if current_time.saturating_sub(self.last_purge) < self.config.purge_interval_ms {
            return 0;
        }
        self.last_purge = current_time;

        let mut total_purged = 0;
        for cache in self.process_caches.values_mut() {
            total_purged += cache.purge_expired(current_time);
        }
        self.global_evictions += total_purged as u64;
        total_purged
    }

    /// Remove a process from the cache
    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) {
        self.process_caches.remove(&pid);
    }

    /// Global hit rate
    #[inline]
    pub fn global_hit_rate(&self) -> f64 {
        let total = self.global_hits + self.global_misses;
        if total == 0 {
            0.0
        } else {
            self.global_hits as f64 / total as f64
        }
    }

    /// Per-process hit rate
    #[inline]
    pub fn process_hit_rate(&self, pid: u64) -> f64 {
        self.process_caches
            .get(&pid)
            .map(|c| c.hit_rate())
            .unwrap_or(0.0)
    }

    /// Total cached entries
    #[inline(always)]
    pub fn total_entries(&self) -> usize {
        self.process_caches.values().map(|c| c.entries.len()).sum()
    }

    /// Total memory used
    #[inline(always)]
    pub fn total_memory(&self) -> usize {
        self.process_caches.values().map(|c| c.current_bytes).sum()
    }

    /// Get statistics
    #[inline]
    pub fn stats(&self) -> SyscallCacheStats {
        SyscallCacheStats {
            total_entries: self.total_entries(),
            total_memory_bytes: self.total_memory(),
            process_count: self.process_caches.len(),
            global_hits: self.global_hits,
            global_misses: self.global_misses,
            global_evictions: self.global_evictions,
            hit_rate: self.global_hit_rate(),
        }
    }
}

/// Cache statistics summary
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SyscallCacheStats {
    pub total_entries: usize,
    pub total_memory_bytes: usize,
    pub process_count: usize,
    pub global_hits: u64,
    pub global_misses: u64,
    pub global_evictions: u64,
    pub hit_rate: f64,
}
