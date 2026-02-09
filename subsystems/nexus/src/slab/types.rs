//! Core types for Slab Allocator Intelligence
//!
//! This module provides fundamental identifiers and enumerations for slab management.

/// Unique slab cache identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(align(64))]
pub struct SlabCacheId(pub u64);

impl SlabCacheId {
    /// Create a new cache ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Unique slab identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SlabId(pub u64);

impl SlabId {
    /// Create a new slab ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// NUMA node identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Create a new node ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// CPU identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpuId(pub u32);

impl CpuId {
    /// Create a new CPU ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// Slab cache flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlabFlags(pub u32);

impl SlabFlags {
    /// No special flags
    pub const NONE: Self = Self(0);
    /// Hardware cache aligned
    pub const HWCACHE_ALIGN: Self = Self(1 << 0);
    /// Panic on allocation failure
    pub const PANIC: Self = Self(1 << 1);
    /// Reclaim accounting
    pub const RECLAIM_ACCOUNT: Self = Self(1 << 2);
    /// Destroy by RCU
    pub const TYPESAFE_BY_RCU: Self = Self(1 << 3);
    /// Spread across NUMA nodes
    pub const MEM_SPREAD: Self = Self(1 << 4);
    /// Store user tracking info
    pub const STORE_USER: Self = Self(1 << 5);
    /// Red zone debugging
    pub const RED_ZONE: Self = Self(1 << 6);
    /// Poison objects
    pub const POISON: Self = Self(1 << 7);
    /// Trace allocations
    pub const TRACE: Self = Self(1 << 8);

    /// Check if flag is set
    #[inline(always)]
    pub fn contains(&self, flag: SlabFlags) -> bool {
        (self.0 & flag.0) != 0
    }

    /// Add flag
    #[inline(always)]
    pub fn add(&mut self, flag: SlabFlags) {
        self.0 |= flag.0;
    }

    /// Remove flag
    #[inline(always)]
    pub fn remove(&mut self, flag: SlabFlags) {
        self.0 &= !flag.0;
    }
}

/// Slab allocator type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabAllocatorType {
    /// SLAB allocator
    Slab,
    /// SLUB allocator
    Slub,
    /// SLOB allocator (compact)
    Slob,
}

impl SlabAllocatorType {
    /// Get allocator name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Slab => "slab",
            Self::Slub => "slub",
            Self::Slob => "slob",
        }
    }
}

/// Cache state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    /// Active and healthy
    Active,
    /// Growing (allocating new slabs)
    Growing,
    /// Shrinking (releasing slabs)
    Shrinking,
    /// Under memory pressure
    Pressure,
    /// Destroyed/inactive
    Destroyed,
}
