//! Core cache types and identifiers.

// ============================================================================
// CACHE TYPES
// ============================================================================

/// Cache type identifier
pub type CacheId = u32;

/// Cache entry key
pub type CacheKey = u64;

/// Cache level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CacheLevel {
    /// L1 cache (fastest, smallest)
    L1     = 1,
    /// L2 cache
    L2     = 2,
    /// L3 cache
    L3     = 3,
    /// Last level cache
    Llc    = 4,
    /// Memory cache (page cache, buffer cache)
    Memory = 5,
    /// Disk cache (SSD cache)
    Disk   = 6,
}

impl CacheLevel {
    /// Get typical access latency in nanoseconds
    pub fn typical_latency_ns(&self) -> u64 {
        match self {
            Self::L1 => 1,
            Self::L2 => 4,
            Self::L3 => 12,
            Self::Llc => 40,
            Self::Memory => 100,
            Self::Disk => 10_000,
        }
    }

    /// Get typical size in bytes
    pub fn typical_size(&self) -> u64 {
        match self {
            Self::L1 => 32 * 1024,                 // 32KB
            Self::L2 => 256 * 1024,                // 256KB
            Self::L3 => 8 * 1024 * 1024,           // 8MB
            Self::Llc => 16 * 1024 * 1024,         // 16MB
            Self::Memory => 1024 * 1024 * 1024,    // 1GB
            Self::Disk => 32 * 1024 * 1024 * 1024, // 32GB
        }
    }
}

/// Cache line state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLineState {
    /// Invalid (not in cache)
    Invalid,
    /// Shared (clean, possibly in other caches)
    Shared,
    /// Exclusive (clean, only in this cache)
    Exclusive,
    /// Modified (dirty)
    Modified,
    /// Owned (dirty, may be in other caches as shared)
    Owned,
}

/// Eviction policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvictionPolicy {
    /// Least Recently Used
    Lru,
    /// Least Frequently Used
    Lfu,
    /// Adaptive Replacement Cache
    Arc,
    /// Clock (second chance)
    Clock,
    /// Random
    Random,
    /// AI-optimized
    AiOptimized,
}
