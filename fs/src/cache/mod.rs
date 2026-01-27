//! HelixFS Cache Subsystem
//!
//! High-performance caching with ARC (Adaptive Replacement Cache) and
//! CLOCK algorithms for optimal hit rates and memory efficiency.
//!
//! # Features
//! - ARC for balanced recency/frequency caching
//! - CLOCK for low-overhead page tracking
//! - Buffer cache for block I/O
//! - Inode cache for metadata
//! - Page cache for file data
//! - Write-back with coalescing
//! - NUMA-aware allocation

#![allow(dead_code)]

pub mod buffer;
pub mod arc;
pub mod page;
pub mod inode;

use crate::core::types::*;
use crate::core::error::HfsError;

// ============================================================================
// Constants
// ============================================================================

/// Default buffer cache size (256 MB)
pub const DEFAULT_BUFFER_CACHE_SIZE: usize = 256 * 1024 * 1024;

/// Default inode cache entries
pub const DEFAULT_INODE_CACHE_SIZE: usize = 65536;

/// Default page cache size (1 GB)
pub const DEFAULT_PAGE_CACHE_SIZE: usize = 1024 * 1024 * 1024;

/// Cache line size (for alignment)
pub const CACHE_LINE_SIZE: usize = 64;

/// Maximum dirty ratio before flush
pub const MAX_DIRTY_RATIO: u32 = 20; // 20%

/// Writeback interval (ms)
pub const WRITEBACK_INTERVAL_MS: u64 = 5000;

/// Maximum buffer age before forced writeback (ms)
pub const MAX_BUFFER_AGE_MS: u64 = 30000;

// ============================================================================
// Cache Entry State
// ============================================================================

/// State of a cache entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CacheState {
    /// Entry is free/unused
    Free = 0,
    /// Entry is valid and clean
    Clean = 1,
    /// Entry is valid and dirty
    Dirty = 2,
    /// Entry is being read from disk
    Reading = 3,
    /// Entry is being written to disk
    Writing = 4,
    /// Entry is locked for exclusive access
    Locked = 5,
    /// Entry is invalid (needs reread)
    Invalid = 6,
    /// Entry is being evicted
    Evicting = 7,
}

impl CacheState {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::Free,
            1 => Self::Clean,
            2 => Self::Dirty,
            3 => Self::Reading,
            4 => Self::Writing,
            5 => Self::Locked,
            6 => Self::Invalid,
            7 => Self::Evicting,
            _ => Self::Invalid,
        }
    }
    
    /// Is this entry usable
    #[inline]
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Clean | Self::Dirty)
    }
    
    /// Is this entry busy
    #[inline]
    pub fn is_busy(&self) -> bool {
        matches!(self, Self::Reading | Self::Writing | Self::Locked | Self::Evicting)
    }
    
    /// Is this entry dirty
    #[inline]
    pub fn is_dirty(&self) -> bool {
        *self == Self::Dirty
    }
}

impl Default for CacheState {
    fn default() -> Self {
        Self::Free
    }
}

// ============================================================================
// Cache Entry Flags
// ============================================================================

/// Flags for cache entries.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct CacheFlags(pub u16);

impl CacheFlags {
    /// Entry is pinned (cannot be evicted)
    pub const PINNED: u16 = 1 << 0;
    /// Entry was recently accessed
    pub const ACCESSED: u16 = 1 << 1;
    /// Entry is metadata
    pub const METADATA: u16 = 1 << 2;
    /// Entry has I/O pending
    pub const IO_PENDING: u16 = 1 << 3;
    /// Entry is in ARC T1 (recency)
    pub const ARC_T1: u16 = 1 << 4;
    /// Entry is in ARC T2 (frequency)
    pub const ARC_T2: u16 = 1 << 5;
    /// Entry is in ARC B1 (ghost recency)
    pub const ARC_B1: u16 = 1 << 6;
    /// Entry is in ARC B2 (ghost frequency)
    pub const ARC_B2: u16 = 1 << 7;
    /// Entry is sequentially accessed
    pub const SEQUENTIAL: u16 = 1 << 8;
    /// Entry should be prefetched
    pub const PREFETCH: u16 = 1 << 9;
    /// Entry is high priority
    pub const HIGH_PRIORITY: u16 = 1 << 10;
    /// Entry is write-through
    pub const WRITE_THROUGH: u16 = 1 << 11;
    
    /// Create empty flags
    pub const fn empty() -> Self {
        Self(0)
    }
    
    /// Check if flag is set
    #[inline]
    pub fn has(&self, flag: u16) -> bool {
        self.0 & flag != 0
    }
    
    /// Set flag
    #[inline]
    pub fn set(&mut self, flag: u16) {
        self.0 |= flag;
    }
    
    /// Clear flag
    #[inline]
    pub fn clear(&mut self, flag: u16) {
        self.0 &= !flag;
    }
    
    /// Is pinned
    #[inline]
    pub fn is_pinned(&self) -> bool {
        self.has(Self::PINNED)
    }
    
    /// Is metadata
    #[inline]
    pub fn is_metadata(&self) -> bool {
        self.has(Self::METADATA)
    }
}

// ============================================================================
// Cache Key
// ============================================================================

/// Key for identifying cache entries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Device ID
    pub device: u32,
    /// Block number
    pub block: u64,
}

impl CacheKey {
    /// Create new key
    pub const fn new(device: u32, block: u64) -> Self {
        Self { device, block }
    }
    
    /// Hash for lookup
    pub fn hash(&self) -> u64 {
        // FNV-1a hash
        let mut hash: u64 = 0xcbf29ce484222325;
        
        // Hash device
        for b in self.device.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        
        // Hash block
        for b in self.block.to_le_bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        
        hash
    }
}

// ============================================================================
// Cache Handle
// ============================================================================

/// Handle to a cache entry.
#[derive(Clone, Copy, Debug)]
pub struct CacheHandle {
    /// Index in cache
    pub index: u32,
    /// Generation for validation
    pub generation: u32,
}

impl CacheHandle {
    /// Create new handle
    pub const fn new(index: u32, generation: u32) -> Self {
        Self { index, generation }
    }
    
    /// Invalid handle
    pub const fn invalid() -> Self {
        Self {
            index: u32::MAX,
            generation: 0,
        }
    }
    
    /// Check if valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.index != u32::MAX
    }
}

// ============================================================================
// Cache Statistics
// ============================================================================

/// Cache statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct CacheStats {
    /// Total lookups
    pub lookups: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Insertions
    pub inserts: u64,
    /// Evictions
    pub evictions: u64,
    /// Dirty evictions (required I/O)
    pub dirty_evictions: u64,
    /// Writebacks
    pub writebacks: u64,
    /// Current entries
    pub current_entries: u64,
    /// Current dirty entries
    pub dirty_entries: u64,
    /// Memory used (bytes)
    pub memory_used: u64,
    /// Prefetch hits
    pub prefetch_hits: u64,
    /// Prefetch misses
    pub prefetch_misses: u64,
}

impl CacheStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            lookups: 0,
            hits: 0,
            misses: 0,
            inserts: 0,
            evictions: 0,
            dirty_evictions: 0,
            writebacks: 0,
            current_entries: 0,
            dirty_entries: 0,
            memory_used: 0,
            prefetch_hits: 0,
            prefetch_misses: 0,
        }
    }
    
    /// Hit rate
    pub fn hit_rate(&self) -> f32 {
        if self.lookups == 0 {
            return 0.0;
        }
        (self.hits as f32 / self.lookups as f32) * 100.0
    }
    
    /// Dirty ratio
    pub fn dirty_ratio(&self) -> f32 {
        if self.current_entries == 0 {
            return 0.0;
        }
        (self.dirty_entries as f32 / self.current_entries as f32) * 100.0
    }
    
    /// Record hit
    #[inline]
    pub fn record_hit(&mut self) {
        self.lookups += 1;
        self.hits += 1;
    }
    
    /// Record miss
    #[inline]
    pub fn record_miss(&mut self) {
        self.lookups += 1;
        self.misses += 1;
    }
}

// ============================================================================
// Cache Configuration
// ============================================================================

/// Cache configuration.
#[derive(Clone, Copy, Debug)]
pub struct CacheConfig {
    /// Maximum memory for buffer cache
    pub buffer_cache_size: usize,
    /// Maximum inode cache entries
    pub inode_cache_size: usize,
    /// Maximum memory for page cache
    pub page_cache_size: usize,
    /// Maximum dirty ratio (percentage)
    pub max_dirty_ratio: u32,
    /// Writeback interval (ms)
    pub writeback_interval_ms: u64,
    /// Maximum buffer age (ms)
    pub max_buffer_age_ms: u64,
    /// Enable ARC algorithm
    pub use_arc: bool,
    /// Enable prefetching
    pub prefetch_enabled: bool,
    /// Prefetch window (blocks)
    pub prefetch_window: u32,
}

impl CacheConfig {
    /// Default configuration
    pub const fn default() -> Self {
        Self {
            buffer_cache_size: DEFAULT_BUFFER_CACHE_SIZE,
            inode_cache_size: DEFAULT_INODE_CACHE_SIZE,
            page_cache_size: DEFAULT_PAGE_CACHE_SIZE,
            max_dirty_ratio: MAX_DIRTY_RATIO,
            writeback_interval_ms: WRITEBACK_INTERVAL_MS,
            max_buffer_age_ms: MAX_BUFFER_AGE_MS,
            use_arc: true,
            prefetch_enabled: true,
            prefetch_window: 32,
        }
    }
    
    /// Minimal configuration
    pub const fn minimal() -> Self {
        Self {
            buffer_cache_size: 16 * 1024 * 1024, // 16 MB
            inode_cache_size: 4096,
            page_cache_size: 64 * 1024 * 1024, // 64 MB
            max_dirty_ratio: 30,
            writeback_interval_ms: 10000,
            max_buffer_age_ms: 60000,
            use_arc: false,
            prefetch_enabled: false,
            prefetch_window: 0,
        }
    }
    
    /// High-performance configuration
    pub const fn high_performance() -> Self {
        Self {
            buffer_cache_size: 1024 * 1024 * 1024, // 1 GB
            inode_cache_size: 262144,
            page_cache_size: 4 * 1024 * 1024 * 1024, // 4 GB
            max_dirty_ratio: 40,
            writeback_interval_ms: 2000,
            max_buffer_age_ms: 15000,
            use_arc: true,
            prefetch_enabled: true,
            prefetch_window: 128,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::default()
    }
}

// ============================================================================
// Cache Operation
// ============================================================================

/// Type of cache operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CacheOp {
    /// Lookup (no read)
    Lookup = 1,
    /// Read (load if not present)
    Read = 2,
    /// Write (mark dirty)
    Write = 3,
    /// Invalidate
    Invalidate = 4,
    /// Flush (write dirty)
    Flush = 5,
    /// Pin (prevent eviction)
    Pin = 6,
    /// Unpin
    Unpin = 7,
    /// Prefetch
    Prefetch = 8,
}

/// Result of cache operation.
#[derive(Clone, Copy, Debug)]
pub struct CacheOpResult {
    /// Success
    pub success: bool,
    /// Handle (if applicable)
    pub handle: Option<CacheHandle>,
    /// Was a hit
    pub was_hit: bool,
    /// Error
    pub error: Option<HfsError>,
}

impl CacheOpResult {
    /// Success with hit
    pub fn hit(handle: CacheHandle) -> Self {
        Self {
            success: true,
            handle: Some(handle),
            was_hit: true,
            error: None,
        }
    }
    
    /// Success with miss
    pub fn miss(handle: CacheHandle) -> Self {
        Self {
            success: true,
            handle: Some(handle),
            was_hit: false,
            error: None,
        }
    }
    
    /// Failure
    pub fn failure(error: HfsError) -> Self {
        Self {
            success: false,
            handle: None,
            was_hit: false,
            error: Some(error),
        }
    }
}

// ============================================================================
// Writeback Control
// ============================================================================

/// Writeback urgency level.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum WritebackUrgency {
    /// Normal writeback
    Normal = 0,
    /// High priority
    High = 1,
    /// Urgent (low on space)
    Urgent = 2,
    /// Critical (must free now)
    Critical = 3,
}

/// Writeback request.
#[derive(Clone, Copy, Debug)]
pub struct WritebackRequest {
    /// Minimum buffers to write
    pub min_buffers: u32,
    /// Maximum buffers to write
    pub max_buffers: u32,
    /// Urgency level
    pub urgency: WritebackUrgency,
    /// Sync after write
    pub sync: bool,
}

impl WritebackRequest {
    /// Normal writeback
    pub const fn normal(count: u32) -> Self {
        Self {
            min_buffers: 1,
            max_buffers: count,
            urgency: WritebackUrgency::Normal,
            sync: false,
        }
    }
    
    /// Urgent writeback
    pub const fn urgent(count: u32) -> Self {
        Self {
            min_buffers: count,
            max_buffers: count * 2,
            urgency: WritebackUrgency::Urgent,
            sync: false,
        }
    }
    
    /// Sync all dirty
    pub const fn sync_all() -> Self {
        Self {
            min_buffers: 0,
            max_buffers: u32::MAX,
            urgency: WritebackUrgency::Critical,
            sync: true,
        }
    }
}

// ============================================================================
// Writeback Result
// ============================================================================

/// Writeback result.
#[derive(Clone, Copy, Debug, Default)]
pub struct WritebackResult {
    /// Buffers written
    pub buffers_written: u32,
    /// Bytes written
    pub bytes_written: u64,
    /// Errors encountered
    pub errors: u32,
}

impl WritebackResult {
    /// Create new result
    pub const fn new() -> Self {
        Self {
            buffers_written: 0,
            bytes_written: 0,
            errors: 0,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cache_state() {
        assert!(CacheState::Clean.is_usable());
        assert!(CacheState::Dirty.is_dirty());
        assert!(CacheState::Reading.is_busy());
        assert!(!CacheState::Free.is_busy());
    }
    
    #[test]
    fn test_cache_flags() {
        let mut flags = CacheFlags::empty();
        assert!(!flags.is_pinned());
        
        flags.set(CacheFlags::PINNED);
        assert!(flags.is_pinned());
        
        flags.clear(CacheFlags::PINNED);
        assert!(!flags.is_pinned());
    }
    
    #[test]
    fn test_cache_key() {
        let key1 = CacheKey::new(0, 100);
        let key2 = CacheKey::new(0, 200);
        
        assert_ne!(key1.hash(), key2.hash());
        assert_eq!(key1, CacheKey::new(0, 100));
    }
    
    #[test]
    fn test_cache_handle() {
        let handle = CacheHandle::new(42, 1);
        assert!(handle.is_valid());
        
        let invalid = CacheHandle::invalid();
        assert!(!invalid.is_valid());
    }
    
    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::new();
        stats.record_hit();
        stats.record_hit();
        stats.record_miss();
        
        assert_eq!(stats.lookups, 3);
        assert_eq!(stats.hits, 2);
        assert!((stats.hit_rate() - 66.66).abs() < 1.0);
    }
    
    #[test]
    fn test_cache_config() {
        let config = CacheConfig::default();
        assert!(config.use_arc);
        assert!(config.prefetch_enabled);
        
        let minimal = CacheConfig::minimal();
        assert!(!minimal.use_arc);
    }
}
