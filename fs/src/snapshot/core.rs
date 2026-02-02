//! Core snapshot operations.
//!
//! Provides snapshot creation, deletion, and management.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::core::error::{HfsError, HfsResult};
use crate::core::types::*;
use crate::snapshot::{
    SnapshotDescriptor, SnapshotFlags, SnapshotId, SnapshotState, SnapshotStats,
    INVALID_SNAPSHOT_ID, MAX_SNAPSHOTS, ROOT_SNAPSHOT_ID,
};

// ============================================================================
// Snapshot Reference
// ============================================================================

/// Snapshot reference (lightweight handle).
#[derive(Clone, Copy, Debug)]
pub struct SnapshotRef {
    /// Snapshot ID
    pub id: SnapshotId,
    /// Generation (for staleness detection)
    pub generation: u32,
    /// Parent ID
    pub parent_id: SnapshotId,
    /// State
    pub state: SnapshotState,
    /// Flags
    pub flags: SnapshotFlags,
}

impl SnapshotRef {
    /// Create from descriptor
    pub fn from_desc(desc: &SnapshotDescriptor) -> Self {
        Self {
            id: desc.id,
            generation: desc.generation,
            parent_id: desc.parent_id,
            state: desc.state(),
            flags: desc.flags(),
        }
    }

    /// Check if valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.id != INVALID_SNAPSHOT_ID && self.state.is_usable()
    }

    /// Check if read-only
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.flags.is_read_only() || self.state == SnapshotState::Archived
    }

    /// Check if root
    #[inline]
    pub fn is_root(&self) -> bool {
        self.id == ROOT_SNAPSHOT_ID
    }
}

impl Default for SnapshotRef {
    fn default() -> Self {
        Self {
            id: INVALID_SNAPSHOT_ID,
            generation: 0,
            parent_id: INVALID_SNAPSHOT_ID,
            state: SnapshotState::Invalid,
            flags: SnapshotFlags::default(),
        }
    }
}

// ============================================================================
// Snapshot Entry
// ============================================================================

/// Snapshot entry in snapshot table.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SnapshotEntry {
    /// Snapshot ID
    pub id: SnapshotId,
    /// Parent ID
    pub parent_id: SnapshotId,
    /// First child ID
    pub first_child: SnapshotId,
    /// Next sibling ID
    pub next_sibling: SnapshotId,
    /// State
    pub state: u8,
    /// Flags
    pub flags: u8,
    /// Reference count
    pub refcount: u16,
    /// Generation
    pub generation: u32,
    /// Create time
    pub create_time: u64,
    /// Descriptor block
    pub desc_block: BlockNum,
    /// Root tree block
    pub root_tree: BlockNum,
    /// Exclusive blocks
    pub exclusive_blocks: u64,
}

impl SnapshotEntry {
    /// Size in bytes
    pub const SIZE: usize = 72;

    /// Create new entry
    pub fn new(id: SnapshotId, parent_id: SnapshotId) -> Self {
        Self {
            id,
            parent_id,
            first_child: INVALID_SNAPSHOT_ID,
            next_sibling: INVALID_SNAPSHOT_ID,
            state: SnapshotState::Creating as u8,
            flags: 0,
            refcount: 0,
            generation: 0,
            create_time: 0,
            desc_block: BlockNum::NULL,
            root_tree: BlockNum::NULL,
            exclusive_blocks: 0,
        }
    }

    /// Check if valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.id != INVALID_SNAPSHOT_ID
    }

    /// Get state
    #[inline]
    pub fn state(&self) -> SnapshotState {
        SnapshotState::from_raw(self.state)
    }

    /// Has children
    #[inline]
    pub fn has_children(&self) -> bool {
        self.first_child != INVALID_SNAPSHOT_ID
    }

    /// Has siblings
    #[inline]
    pub fn has_siblings(&self) -> bool {
        self.next_sibling != INVALID_SNAPSHOT_ID
    }
}

// ============================================================================
// Snapshot Table
// ============================================================================

/// Snapshot table header.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SnapshotTableHeader {
    /// Magic
    pub magic: u32,
    /// Version
    pub version: u8,
    /// Flags
    pub flags: u8,
    /// Reserved
    pub _reserved: u16,
    /// Entry count
    pub count: u32,
    /// Maximum entries
    pub max_entries: u32,
    /// Next snapshot ID
    pub next_id: SnapshotId,
    /// Active snapshot ID (current)
    pub active_id: SnapshotId,
    /// Root snapshot ID
    pub root_id: SnapshotId,
    /// Generation
    pub generation: u64,
    /// Checksum
    pub checksum: u32,
    /// Padding
    pub _pad: [u8; 12],
}

impl SnapshotTableHeader {
    /// Size
    pub const SIZE: usize = 64;

    /// Magic
    pub const MAGIC: u32 = 0x534E5442; // "SNTB"

    /// Create new header
    pub fn new() -> Self {
        Self {
            magic: Self::MAGIC,
            version: 1,
            flags: 0,
            _reserved: 0,
            count: 0,
            max_entries: MAX_SNAPSHOTS as u32,
            next_id: 2, // 1 is root
            active_id: ROOT_SNAPSHOT_ID,
            root_id: ROOT_SNAPSHOT_ID,
            generation: 0,
            checksum: 0,
            _pad: [0; 12],
        }
    }

    /// Validate
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != Self::MAGIC {
            return Err(HfsError::SnapshotCorrupted);
        }
        Ok(())
    }
}

impl Default for SnapshotTableHeader {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Snapshot Manager
// ============================================================================

/// Maximum in-memory snapshot cache
const SNAPSHOT_CACHE_SIZE: usize = 64;

/// Snapshot manager runtime state.
pub struct SnapshotManager {
    /// Next snapshot ID
    pub next_id: AtomicU64,
    /// Active snapshot ID
    pub active_id: AtomicU64,
    /// Snapshot count
    pub count: AtomicU64,
    /// Operation in progress
    pub op_in_progress: AtomicBool,
    /// Cached entries
    pub cache: [SnapshotEntry; SNAPSHOT_CACHE_SIZE],
    /// Cache count
    pub cache_count: usize,
    /// Statistics
    pub stats: SnapshotStats,
}

impl SnapshotManager {
    /// Create new manager
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(2),
            active_id: AtomicU64::new(ROOT_SNAPSHOT_ID),
            count: AtomicU64::new(1), // Root
            op_in_progress: AtomicBool::new(false),
            cache: [SnapshotEntry::new(INVALID_SNAPSHOT_ID, INVALID_SNAPSHOT_ID);
                SNAPSHOT_CACHE_SIZE],
            cache_count: 0,
            stats: SnapshotStats::new(),
        }
    }

    /// Get active snapshot ID
    #[inline]
    pub fn active_id(&self) -> SnapshotId {
        self.active_id.load(Ordering::Acquire)
    }

    /// Set active snapshot
    pub fn set_active(&self, id: SnapshotId) {
        self.active_id.store(id, Ordering::Release);
    }

    /// Allocate new snapshot ID
    pub fn alloc_id(&self) -> SnapshotId {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Get snapshot count
    #[inline]
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Try to start operation
    pub fn try_start_op(&self) -> bool {
        self.op_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    /// End operation
    pub fn end_op(&self) {
        self.op_in_progress.store(false, Ordering::Release);
    }

    /// Add to cache
    pub fn cache_add(&mut self, entry: SnapshotEntry) {
        if self.cache_count < SNAPSHOT_CACHE_SIZE {
            self.cache[self.cache_count] = entry;
            self.cache_count += 1;
        } else {
            // Replace oldest (simple LRU)
            // In a real implementation, use proper LRU
            self.cache[0] = entry;
        }
    }

    /// Find in cache
    pub fn cache_find(&self, id: SnapshotId) -> Option<&SnapshotEntry> {
        for i in 0..self.cache_count {
            if self.cache[i].id == id {
                return Some(&self.cache[i]);
            }
        }
        None
    }

    /// Remove from cache
    pub fn cache_remove(&mut self, id: SnapshotId) {
        for i in 0..self.cache_count {
            if self.cache[i].id == id {
                // Swap with last
                self.cache[i] = self.cache[self.cache_count - 1];
                self.cache[self.cache_count - 1] =
                    SnapshotEntry::new(INVALID_SNAPSHOT_ID, INVALID_SNAPSHOT_ID);
                self.cache_count -= 1;
                return;
            }
        }
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Snapshot Creation Parameters
// ============================================================================

/// Parameters for snapshot creation.
#[derive(Clone, Debug)]
pub struct SnapshotCreateParams {
    /// Parent snapshot (default: active)
    pub parent: Option<SnapshotId>,
    /// Name
    pub name: [u8; 64],
    /// Name length
    pub name_len: usize,
    /// Description
    pub description: [u8; 128],
    /// Description length
    pub desc_len: usize,
    /// Flags
    pub flags: SnapshotFlags,
    /// Make active after creation
    pub make_active: bool,
}

impl SnapshotCreateParams {
    /// Create with name
    pub fn with_name(name: &[u8]) -> Self {
        let mut params = Self::default();
        let len = name.len().min(64);
        params.name[..len].copy_from_slice(&name[..len]);
        params.name_len = len;
        params
    }

    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len]
    }

    /// Get description
    pub fn description(&self) -> &[u8] {
        &self.description[..self.desc_len]
    }
}

impl Default for SnapshotCreateParams {
    fn default() -> Self {
        Self {
            parent: None,
            name: [0; 64],
            name_len: 0,
            description: [0; 128],
            desc_len: 0,
            flags: SnapshotFlags::new(SnapshotFlags::READ_ONLY),
            make_active: false,
        }
    }
}

// ============================================================================
// Snapshot Iterator
// ============================================================================

/// Iterator over snapshots.
pub struct SnapshotIterator {
    /// Current snapshot ID
    pub current: SnapshotId,
    /// Start ID
    pub start: SnapshotId,
    /// End ID
    pub end: SnapshotId,
    /// Include children
    pub include_children: bool,
    /// Only active
    pub only_active: bool,
    /// Count
    pub count: usize,
}

impl SnapshotIterator {
    /// Create iterator for all snapshots
    pub fn all() -> Self {
        Self {
            current: ROOT_SNAPSHOT_ID,
            start: ROOT_SNAPSHOT_ID,
            end: SnapshotId::MAX,
            include_children: true,
            only_active: false,
            count: 0,
        }
    }

    /// Create iterator for children of parent
    pub fn children_of(parent: SnapshotId) -> Self {
        Self {
            current: INVALID_SNAPSHOT_ID, // Will be set to first child
            start: parent,
            end: SnapshotId::MAX,
            include_children: false,
            only_active: false,
            count: 0,
        }
    }

    /// Filter to active only
    pub fn active_only(mut self) -> Self {
        self.only_active = true;
        self
    }
}

// ============================================================================
// Snapshot Path
// ============================================================================

/// Path from one snapshot to another.
#[derive(Clone, Debug)]
pub struct SnapshotPath {
    /// Path elements (snapshot IDs from source to target)
    pub path: [SnapshotId; 32],
    /// Path length
    pub length: usize,
    /// Common ancestor
    pub common_ancestor: SnapshotId,
}

impl SnapshotPath {
    /// Maximum path length
    pub const MAX_LENGTH: usize = 32;

    /// Create empty path
    pub const fn empty() -> Self {
        Self {
            path: [INVALID_SNAPSHOT_ID; 32],
            length: 0,
            common_ancestor: INVALID_SNAPSHOT_ID,
        }
    }

    /// Push snapshot to path
    pub fn push(&mut self, id: SnapshotId) -> HfsResult<()> {
        if self.length >= Self::MAX_LENGTH {
            return Err(HfsError::SnapshotDepthExceeded);
        }

        self.path[self.length] = id;
        self.length += 1;
        Ok(())
    }

    /// Get path as slice
    pub fn as_slice(&self) -> &[SnapshotId] {
        &self.path[..self.length]
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl Default for SnapshotPath {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_ref() {
        let mut desc = SnapshotDescriptor::new(5, ROOT_SNAPSHOT_ID);
        desc.set_state(SnapshotState::Active);

        let ref_ = SnapshotRef::from_desc(&desc);

        assert_eq!(ref_.id, 5);
        assert!(ref_.is_valid());
        assert!(!ref_.is_root());
    }

    #[test]
    fn test_snapshot_entry() {
        let entry = SnapshotEntry::new(10, 5);

        assert!(entry.is_valid());
        assert!(!entry.has_children());
        assert!(!entry.has_siblings());
    }

    #[test]
    fn test_snapshot_table_header() {
        let header = SnapshotTableHeader::new();

        assert_eq!(header.magic, SnapshotTableHeader::MAGIC);
        assert!(header.validate().is_ok());
    }

    #[test]
    fn test_snapshot_manager() {
        let manager = SnapshotManager::new();

        assert_eq!(manager.active_id(), ROOT_SNAPSHOT_ID);
        assert_eq!(manager.count(), 1);

        let id1 = manager.alloc_id();
        let id2 = manager.alloc_id();
        assert_eq!(id2, id1 + 1);

        assert!(manager.try_start_op());
        assert!(!manager.try_start_op());
        manager.end_op();
        assert!(manager.try_start_op());
    }

    #[test]
    fn test_snapshot_create_params() {
        let params = SnapshotCreateParams::with_name(b"test-snap");

        assert_eq!(params.name(), b"test-snap");
        assert!(params.flags.is_read_only());
    }

    #[test]
    fn test_snapshot_path() {
        let mut path = SnapshotPath::empty();

        assert!(path.is_empty());

        path.push(1).unwrap();
        path.push(5).unwrap();
        path.push(10).unwrap();

        assert_eq!(path.length, 3);
        assert_eq!(path.as_slice(), &[1, 5, 10]);
    }
}
