//! Allocation zones for NUMA-aware and locality-based allocation.
//!
//! Zones divide the disk into regions for:
//! - NUMA locality (allocate near CPU)
//! - Metadata vs data separation
//! - Wear leveling for flash storage

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of zones
pub const MAX_ZONES: usize = 256;

/// Default zone size (256MB in 4K blocks)
pub const DEFAULT_ZONE_BLOCKS: u64 = 65536;

/// Minimum zone size
pub const MIN_ZONE_BLOCKS: u64 = 1024;

// ============================================================================
// Zone Types
// ============================================================================

/// Zone type classification
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ZoneType {
    /// General purpose data zone
    Data     = 0,
    /// Metadata zone (inodes, directories)
    Metadata = 1,
    /// Journal zone
    Journal  = 2,
    /// Snapshot zone
    Snapshot = 3,
    /// Hot data (frequently accessed)
    Hot      = 4,
    /// Cold data (infrequently accessed)
    Cold     = 5,
    /// Reserved for system use
    Reserved = 6,
}

impl ZoneType {
    /// From byte
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Data),
            1 => Some(Self::Metadata),
            2 => Some(Self::Journal),
            3 => Some(Self::Snapshot),
            4 => Some(Self::Hot),
            5 => Some(Self::Cold),
            6 => Some(Self::Reserved),
            _ => None,
        }
    }

    /// Get priority for allocation preference
    pub fn priority(&self) -> u8 {
        match self {
            Self::Journal => 0, // Highest (dedicated)
            Self::Metadata => 1,
            Self::Hot => 2,
            Self::Data => 3,
            Self::Cold => 4,
            Self::Snapshot => 5,
            Self::Reserved => 255, // Never allocate
        }
    }
}

// ============================================================================
// Zone State
// ============================================================================

/// Zone state
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ZoneState {
    /// Zone is active and can accept allocations
    Active        = 0,
    /// Zone is full
    Full          = 1,
    /// Zone is being defragmented
    Defragmenting = 2,
    /// Zone is read-only (for snapshots)
    ReadOnly      = 3,
    /// Zone is offline/disabled
    Offline       = 4,
}

impl ZoneState {
    /// From byte
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Active),
            1 => Some(Self::Full),
            2 => Some(Self::Defragmenting),
            3 => Some(Self::ReadOnly),
            4 => Some(Self::Offline),
            _ => None,
        }
    }

    /// Can allocate in this state?
    #[inline]
    pub fn can_allocate(&self) -> bool {
        *self == Self::Active
    }

    /// Can write in this state?
    #[inline]
    pub fn can_write(&self) -> bool {
        matches!(self, Self::Active | Self::Defragmenting)
    }
}

// ============================================================================
// Zone Descriptor
// ============================================================================

/// On-disk zone descriptor.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct ZoneDescriptor {
    /// Zone ID
    pub zone_id: u32,
    /// Zone type
    pub zone_type: u8,
    /// Zone state
    pub state: u8,
    /// NUMA node affinity (0xFF = any)
    pub numa_node: u8,
    /// Padding
    pub _pad: u8,
    /// First block of zone
    pub start_block: u64,
    /// Number of blocks in zone
    pub block_count: u64,
    /// Free blocks in zone
    pub free_blocks: u64,
    /// Largest contiguous free extent
    pub largest_free: u32,
    /// Fragmentation score (0-100)
    pub fragmentation: u8,
    /// Write amplification factor * 10
    pub write_amp: u8,
    /// Reserved (8 + 8 + 8 + 4 + 4 + 4 + 1 + 1 = 38; need 64-38=26 bytes)
    pub _reserved: [u8; 26],
}

impl ZoneDescriptor {
    /// Size in bytes
    pub const SIZE: usize = 64;

    /// Create new zone descriptor
    pub fn new(zone_id: u32, zone_type: ZoneType, start_block: u64, block_count: u64) -> Self {
        Self {
            zone_id,
            zone_type: zone_type as u8,
            state: ZoneState::Active as u8,
            numa_node: 0xFF,
            _pad: 0,
            start_block,
            block_count,
            free_blocks: block_count,
            largest_free: block_count.min(u32::MAX as u64) as u32,
            fragmentation: 0,
            write_amp: 10, // 1.0x
            _reserved: [0; 26],
        }
    }

    /// Get zone type
    #[inline]
    pub fn zone_type(&self) -> ZoneType {
        ZoneType::from_u8(self.zone_type).unwrap_or(ZoneType::Data)
    }

    /// Get zone state
    #[inline]
    pub fn zone_state(&self) -> ZoneState {
        ZoneState::from_u8(self.state).unwrap_or(ZoneState::Offline)
    }

    /// Get end block (exclusive)
    #[inline]
    pub fn end_block(&self) -> u64 {
        self.start_block + self.block_count
    }

    /// Check if block is in this zone
    #[inline]
    pub fn contains(&self, block: u64) -> bool {
        block >= self.start_block && block < self.end_block()
    }

    /// Get usage percentage
    #[inline]
    pub fn usage_percent(&self) -> u8 {
        if self.block_count == 0 {
            return 100;
        }
        let used = self.block_count - self.free_blocks;
        ((used * 100) / self.block_count) as u8
    }

    /// Check if zone can accept allocation
    pub fn can_allocate(&self, count: u32) -> bool {
        self.zone_state().can_allocate()
            && self.free_blocks >= count as u64
            && self.largest_free >= count
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<ZoneDescriptor>() == ZoneDescriptor::SIZE);

// ============================================================================
// Zone Runtime State
// ============================================================================

/// Runtime zone state with atomic counters.
pub struct Zone {
    /// Zone descriptor
    desc: ZoneDescriptor,
    /// Free blocks (atomic)
    free_blocks: AtomicU64,
    /// Allocations in progress
    pending_allocs: AtomicU32,
    /// Is locked for defrag
    locked: AtomicBool,
    /// Total allocations
    total_allocs: AtomicU64,
    /// Total frees
    total_frees: AtomicU64,
    /// Write count (for wear leveling)
    write_count: AtomicU64,
}

impl Zone {
    /// Create from descriptor
    pub fn from_descriptor(desc: ZoneDescriptor) -> Self {
        Self {
            free_blocks: AtomicU64::new(desc.free_blocks),
            pending_allocs: AtomicU32::new(0),
            locked: AtomicBool::new(false),
            total_allocs: AtomicU64::new(0),
            total_frees: AtomicU64::new(0),
            write_count: AtomicU64::new(0),
            desc,
        }
    }

    /// Get zone ID
    #[inline]
    pub fn id(&self) -> u32 {
        self.desc.zone_id
    }

    /// Get zone type
    #[inline]
    pub fn zone_type(&self) -> ZoneType {
        self.desc.zone_type()
    }

    /// Get start block
    #[inline]
    pub fn start_block(&self) -> u64 {
        self.desc.start_block
    }

    /// Get block count
    #[inline]
    pub fn block_count(&self) -> u64 {
        self.desc.block_count
    }

    /// Get free blocks
    #[inline]
    pub fn free_blocks(&self) -> u64 {
        self.free_blocks.load(Ordering::Relaxed)
    }

    /// Check if zone contains block
    #[inline]
    pub fn contains(&self, block: u64) -> bool {
        self.desc.contains(block)
    }

    /// Try to reserve blocks for allocation
    pub fn try_reserve(&self, count: u32) -> bool {
        if self.locked.load(Ordering::Relaxed) {
            return false;
        }

        let mut current = self.free_blocks.load(Ordering::Relaxed);
        loop {
            if current < count as u64 {
                return false;
            }

            match self.free_blocks.compare_exchange_weak(
                current,
                current - count as u64,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.pending_allocs.fetch_add(1, Ordering::Relaxed);
                    self.total_allocs.fetch_add(1, Ordering::Relaxed);
                    return true;
                },
                Err(c) => current = c,
            }
        }
    }

    /// Complete allocation (decrement pending)
    pub fn complete_alloc(&self) {
        self.pending_allocs.fetch_sub(1, Ordering::Relaxed);
    }

    /// Cancel reservation
    pub fn cancel_reserve(&self, count: u32) {
        self.free_blocks.fetch_add(count as u64, Ordering::Relaxed);
        self.pending_allocs.fetch_sub(1, Ordering::Relaxed);
    }

    /// Free blocks
    pub fn free(&self, count: u32) {
        self.free_blocks.fetch_add(count as u64, Ordering::Relaxed);
        self.total_frees.fetch_add(1, Ordering::Relaxed);
    }

    /// Record write
    pub fn record_write(&self, blocks: u64) {
        self.write_count.fetch_add(blocks, Ordering::Relaxed);
    }

    /// Lock zone for defrag
    pub fn lock_for_defrag(&self) -> bool {
        !self.locked.swap(true, Ordering::Relaxed)
    }

    /// Unlock zone
    pub fn unlock(&self) {
        self.locked.store(false, Ordering::Relaxed);
    }

    /// Check if locked
    #[inline]
    pub fn is_locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    /// Get write count
    #[inline]
    pub fn write_count(&self) -> u64 {
        self.write_count.load(Ordering::Relaxed)
    }

    /// Update descriptor from runtime state
    pub fn sync_descriptor(&mut self) {
        self.desc.free_blocks = self.free_blocks.load(Ordering::Relaxed);
    }

    /// Get descriptor
    pub fn descriptor(&self) -> &ZoneDescriptor {
        &self.desc
    }
}

// ============================================================================
// Zone Manager
// ============================================================================

/// Manages allocation zones.
pub struct ZoneManager {
    /// Number of active zones
    zone_count: u32,
    /// Total blocks across all zones
    total_blocks: u64,
    /// Total free blocks
    free_blocks: AtomicU64,
    /// Preferred zone for next allocation per type
    preferred: [AtomicU32; 8],
}

impl ZoneManager {
    /// Create new zone manager
    pub fn new(zone_count: u32, total_blocks: u64) -> Self {
        #[allow(clippy::declare_interior_mutable_const)]
        const ZERO: AtomicU32 = AtomicU32::new(0);

        Self {
            zone_count,
            total_blocks,
            free_blocks: AtomicU64::new(total_blocks),
            preferred: [ZERO; 8],
        }
    }

    /// Get zone count
    #[inline]
    pub fn zone_count(&self) -> u32 {
        self.zone_count
    }

    /// Get total blocks
    #[inline]
    pub fn total_blocks(&self) -> u64 {
        self.total_blocks
    }

    /// Get free blocks
    #[inline]
    pub fn free_blocks(&self) -> u64 {
        self.free_blocks.load(Ordering::Relaxed)
    }

    /// Get preferred zone for type
    #[inline]
    pub fn preferred_zone(&self, zone_type: ZoneType) -> u32 {
        let idx = (zone_type as usize).min(7);
        self.preferred[idx].load(Ordering::Relaxed)
    }

    /// Set preferred zone for type
    pub fn set_preferred(&self, zone_type: ZoneType, zone_id: u32) {
        let idx = (zone_type as usize).min(7);
        self.preferred[idx].store(zone_id, Ordering::Relaxed);
    }

    /// Find zone containing block
    pub fn find_zone(&self, block: u64, block_size: u64) -> Option<u32> {
        // Simple calculation assuming uniform zones
        if block >= self.total_blocks {
            return None;
        }

        let zone_blocks = self.total_blocks / self.zone_count as u64;
        Some((block / zone_blocks) as u32)
    }

    /// Consume blocks from total
    pub fn consume(&self, count: u64) -> bool {
        let mut current = self.free_blocks.load(Ordering::Relaxed);
        loop {
            if current < count {
                return false;
            }
            match self.free_blocks.compare_exchange_weak(
                current,
                current - count,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(c) => current = c,
            }
        }
    }

    /// Release blocks back to total
    pub fn release(&self, count: u64) {
        self.free_blocks.fetch_add(count, Ordering::Relaxed);
    }
}

// ============================================================================
// Zone Selection
// ============================================================================

/// Zone selection criteria
#[derive(Clone, Copy, Default)]
pub struct ZoneSelector {
    /// Preferred zone type
    pub zone_type: Option<ZoneType>,
    /// Required free blocks
    pub min_free: u32,
    /// NUMA node preference
    pub numa_node: Option<u8>,
    /// Avoid these zones
    pub avoid_zones: u32, // Bitmap of zone IDs to avoid
    /// Prefer zone containing this block (for locality)
    pub near_block: Option<u64>,
}

impl ZoneSelector {
    /// Create for zone type
    pub fn for_type(zone_type: ZoneType) -> Self {
        Self {
            zone_type: Some(zone_type),
            ..Default::default()
        }
    }

    /// With minimum free blocks
    pub fn with_min_free(mut self, min: u32) -> Self {
        self.min_free = min;
        self
    }

    /// With NUMA preference
    pub fn with_numa(mut self, node: u8) -> Self {
        self.numa_node = Some(node);
        self
    }

    /// Near specific block
    pub fn near(mut self, block: u64) -> Self {
        self.near_block = Some(block);
        self
    }

    /// Avoid zone
    pub fn avoiding(mut self, zone_id: u32) -> Self {
        if zone_id < 32 {
            self.avoid_zones |= 1 << zone_id;
        }
        self
    }
}

/// Result of zone selection
#[derive(Clone, Copy, Debug)]
pub struct ZoneSelection {
    /// Selected zone ID
    pub zone_id: u32,
    /// Start block of zone
    pub start_block: u64,
    /// Free blocks in zone
    pub free_blocks: u64,
    /// Distance from goal (for locality scoring)
    pub distance: u64,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zone_descriptor() {
        let desc = ZoneDescriptor::new(0, ZoneType::Data, 1000, 10000);

        assert_eq!(desc.zone_id, 0);
        assert_eq!(desc.zone_type(), ZoneType::Data);
        assert_eq!(desc.zone_state(), ZoneState::Active);
        assert_eq!(desc.start_block, 1000);
        assert_eq!(desc.block_count, 10000);
        assert_eq!(desc.end_block(), 11000);

        assert!(desc.contains(1000));
        assert!(desc.contains(10999));
        assert!(!desc.contains(11000));
    }

    #[test]
    fn test_zone_runtime() {
        let desc = ZoneDescriptor::new(1, ZoneType::Metadata, 0, 1000);
        let zone = Zone::from_descriptor(desc);

        assert_eq!(zone.id(), 1);
        assert_eq!(zone.free_blocks(), 1000);

        assert!(zone.try_reserve(100));
        assert_eq!(zone.free_blocks(), 900);

        zone.complete_alloc();

        zone.free(50);
        assert_eq!(zone.free_blocks(), 950);
    }

    #[test]
    fn test_zone_manager() {
        let manager = ZoneManager::new(4, 10000);

        assert_eq!(manager.zone_count(), 4);
        assert_eq!(manager.free_blocks(), 10000);

        assert!(manager.consume(500));
        assert_eq!(manager.free_blocks(), 9500);

        manager.release(200);
        assert_eq!(manager.free_blocks(), 9700);
    }

    #[test]
    fn test_zone_selector() {
        let selector = ZoneSelector::for_type(ZoneType::Data)
            .with_min_free(100)
            .near(5000)
            .avoiding(0);

        assert_eq!(selector.zone_type, Some(ZoneType::Data));
        assert_eq!(selector.min_free, 100);
        assert_eq!(selector.near_block, Some(5000));
        assert_eq!(selector.avoid_zones, 1);
    }
}
