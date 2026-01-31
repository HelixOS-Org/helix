//! Snapshot subsystem for instant point-in-time captures.
//!
//! This module provides:
//! - O(1) snapshot creation using CoW
//! - Snapshot hierarchy (parent/child relationships)
//! - Incremental diff between snapshots
//! - Snapshot rollback and cloning

pub mod core;
pub mod tree;
pub mod diff;
pub mod clone;

pub use self::core::*;
pub use tree::*;
pub use diff::*;
pub use clone::*;

use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::Crc32c;

// ============================================================================
// Snapshot Configuration
// ============================================================================

/// Snapshot magic number
pub const SNAPSHOT_MAGIC: u32 = 0x48534E50; // "HSNP"

/// Snapshot version
pub const SNAPSHOT_VERSION: u8 = 1;

/// Maximum snapshot name length
pub const MAX_SNAPSHOT_NAME: usize = 64;

/// Maximum snapshot depth (parent chain)
pub const MAX_SNAPSHOT_DEPTH: usize = 256;

/// Maximum snapshots per filesystem
pub const MAX_SNAPSHOTS: usize = 65536;

// ============================================================================
// Snapshot ID
// ============================================================================

/// Snapshot ID type.
pub type SnapshotId = u64;

/// Invalid snapshot ID
pub const INVALID_SNAPSHOT_ID: SnapshotId = 0;

/// Root snapshot ID (base filesystem)
pub const ROOT_SNAPSHOT_ID: SnapshotId = 1;

// ============================================================================
// Snapshot State
// ============================================================================

/// Snapshot state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SnapshotState {
    /// Invalid/deleted
    Invalid = 0,
    /// Active (usable)
    Active = 1,
    /// Creating (in progress)
    Creating = 2,
    /// Deleting (in progress)
    Deleting = 3,
    /// Locked (cannot be deleted)
    Locked = 4,
    /// Archived (read-only, optimized storage)
    Archived = 5,
}

impl SnapshotState {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Active,
            2 => Self::Creating,
            3 => Self::Deleting,
            4 => Self::Locked,
            5 => Self::Archived,
            _ => Self::Invalid,
        }
    }
    
    /// Check if usable
    #[inline]
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::Active | Self::Locked | Self::Archived)
    }
    
    /// Check if modifiable
    #[inline]
    pub fn is_modifiable(&self) -> bool {
        *self == Self::Active
    }
    
    /// Check if deletable
    #[inline]
    pub fn is_deletable(&self) -> bool {
        matches!(self, Self::Active | Self::Archived)
    }
}

impl Default for SnapshotState {
    fn default() -> Self {
        Self::Invalid
    }
}

// ============================================================================
// Snapshot Flags
// ============================================================================

/// Snapshot flags.
#[derive(Clone, Copy, Debug, Default)]
pub struct SnapshotFlags(pub u32);

impl SnapshotFlags {
    /// No flags
    pub const NONE: u32 = 0;
    /// Read-only snapshot
    pub const READ_ONLY: u32 = 1 << 0;
    /// Writable snapshot (clone)
    pub const WRITABLE: u32 = 1 << 1;
    /// Auto-created (by timer)
    pub const AUTO: u32 = 1 << 2;
    /// Has exclusive data (not shared)
    pub const EXCLUSIVE: u32 = 1 << 3;
    /// Is base for other snapshots
    pub const IS_BASE: u32 = 1 << 4;
    /// Keep minimum versions
    pub const KEEP_MINIMUM: u32 = 1 << 5;
    /// Archived to secondary storage
    pub const ARCHIVED: u32 = 1 << 6;
    
    /// Create from raw
    pub const fn new(flags: u32) -> Self {
        Self(flags)
    }
    
    /// Check if flag is set
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        (self.0 & flag) != 0
    }
    
    /// Set flag
    #[inline]
    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }
    
    /// Clear flag
    #[inline]
    pub fn clear(&mut self, flag: u32) {
        self.0 &= !flag;
    }
    
    /// Is read-only
    #[inline]
    pub fn is_read_only(&self) -> bool {
        self.has(Self::READ_ONLY)
    }
    
    /// Is writable
    #[inline]
    pub fn is_writable(&self) -> bool {
        self.has(Self::WRITABLE)
    }
}

// ============================================================================
// Snapshot Descriptor (On-disk)
// ============================================================================

/// On-disk snapshot descriptor.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SnapshotDescriptor {
    /// Magic number
    pub magic: u32,
    /// Version
    pub version: u8,
    /// State
    pub state: u8,
    /// Reserved
    pub _reserved1: u16,
    /// Snapshot ID
    pub id: SnapshotId,
    /// Parent snapshot ID
    pub parent_id: SnapshotId,
    /// Creation timestamp
    pub create_time: u64,
    /// Flags
    pub flags: u32,
    /// Generation
    pub generation: u32,
    /// Root tree block
    pub root_tree: u64,
    /// Extent tree block
    pub extent_tree: u64,
    /// Inode count
    pub inode_count: u64,
    /// Block count (exclusive)
    pub exclusive_blocks: u64,
    /// Block count (shared)
    pub shared_blocks: u64,
    /// Name length
    pub name_len: u8,
    /// Padding
    pub _pad: [u8; 7],
    /// Name
    pub name: [u8; MAX_SNAPSHOT_NAME],
    /// Description (optional)
    pub description: [u8; 128],
    /// Reserved
    pub _reserved2: [u8; 228],
    /// Checksum
    pub checksum: u32,
}

impl SnapshotDescriptor {
    /// Size in bytes
    pub const SIZE: usize = 512;
    
    /// Create new descriptor
    pub fn new(id: SnapshotId, parent_id: SnapshotId) -> Self {
        Self {
            magic: SNAPSHOT_MAGIC,
            version: SNAPSHOT_VERSION,
            state: SnapshotState::Creating as u8,
            _reserved1: 0,
            id,
            parent_id,
            create_time: 0,
            flags: 0,
            generation: 0,
            root_tree: 0,
            extent_tree: 0,
            inode_count: 0,
            exclusive_blocks: 0,
            shared_blocks: 0,
            name_len: 0,
            _pad: [0; 7],
            name: [0; MAX_SNAPSHOT_NAME],
            description: [0; 128],
            _reserved2: [0; 228],
            checksum: 0,
        }
    }
    
    /// Validate descriptor
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != SNAPSHOT_MAGIC {
            return Err(HfsError::SnapshotCorrupted);
        }
        
        if self.version != SNAPSHOT_VERSION {
            return Err(HfsError::InvalidVersion);
        }
        
        if self.id == INVALID_SNAPSHOT_ID {
            return Err(HfsError::InvalidSnapshotId);
        }
        
        Ok(())
    }
    
    /// Get state
    #[inline]
    pub fn state(&self) -> SnapshotState {
        SnapshotState::from_raw(self.state)
    }
    
    /// Set state
    #[inline]
    pub fn set_state(&mut self, state: SnapshotState) {
        self.state = state as u8;
    }
    
    /// Get flags
    #[inline]
    pub fn flags(&self) -> SnapshotFlags {
        SnapshotFlags::new(self.flags)
    }
    
    /// Set name
    pub fn set_name(&mut self, name: &[u8]) {
        let len = name.len().min(MAX_SNAPSHOT_NAME);
        self.name[..len].copy_from_slice(&name[..len]);
        self.name_len = len as u8;
    }
    
    /// Get name
    pub fn name(&self) -> &[u8] {
        let len = (self.name_len as usize).min(MAX_SNAPSHOT_NAME);
        &self.name[..len]
    }
    
    /// Set description
    pub fn set_description(&mut self, desc: &[u8]) {
        let len = desc.len().min(128);
        self.description[..len].copy_from_slice(&desc[..len]);
    }
    
    /// Total blocks
    #[inline]
    pub fn total_blocks(&self) -> u64 {
        self.exclusive_blocks + self.shared_blocks
    }
    
    /// Compute checksum
    pub fn compute_checksum(&self) -> u32 {
        let mut hasher = Crc32c::new();
        let bytes = unsafe {
            ::core::slice::from_raw_parts(
                self as *const _ as *const u8,
                Self::SIZE - 4,
            )
        };
        hasher.write(bytes);
        hasher.finish()
    }
    
    /// Update checksum
    pub fn update_checksum(&mut self) {
        self.checksum = self.compute_checksum();
    }
}

// Verify size
const _: () = assert!(::core::mem::size_of::<SnapshotDescriptor>() == 512);

// ============================================================================
// Snapshot Statistics
// ============================================================================

/// Snapshot statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct SnapshotStats {
    /// Total snapshots
    pub total: u64,
    /// Active snapshots
    pub active: u64,
    /// Locked snapshots
    pub locked: u64,
    /// Archived snapshots
    pub archived: u64,
    /// Total exclusive blocks
    pub exclusive_blocks: u64,
    /// Total shared blocks
    pub shared_blocks: u64,
    /// Maximum depth
    pub max_depth: u32,
    /// Snapshots created
    pub created: u64,
    /// Snapshots deleted
    pub deleted: u64,
    /// Clones created
    pub clones_created: u64,
}

impl SnapshotStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            total: 0,
            active: 0,
            locked: 0,
            archived: 0,
            exclusive_blocks: 0,
            shared_blocks: 0,
            max_depth: 0,
            created: 0,
            deleted: 0,
            clones_created: 0,
        }
    }
    
    /// Calculate space savings ratio
    pub fn savings_ratio(&self) -> f32 {
        let total = self.exclusive_blocks + self.shared_blocks;
        if total == 0 {
            return 0.0;
        }
        self.shared_blocks as f32 / total as f32
    }
}

// ============================================================================
// Snapshot Operations Interface
// ============================================================================

/// Snapshot operation types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotOp {
    /// Create snapshot
    Create,
    /// Delete snapshot
    Delete,
    /// Clone snapshot (writable copy)
    Clone,
    /// Rollback to snapshot
    Rollback,
    /// Archive snapshot
    Archive,
    /// Lock snapshot
    Lock,
    /// Unlock snapshot
    Unlock,
    /// Diff between snapshots
    Diff,
}

/// Snapshot operation result.
#[derive(Clone, Copy, Debug)]
pub struct SnapshotOpResult {
    /// Operation type
    pub op: SnapshotOp,
    /// Success
    pub success: bool,
    /// Snapshot ID (for create/clone)
    pub snapshot_id: SnapshotId,
    /// Error (if failed)
    pub error: Option<HfsError>,
    /// Duration (ns)
    pub duration_ns: u64,
}

impl SnapshotOpResult {
    /// Create success result
    pub fn success(op: SnapshotOp, snapshot_id: SnapshotId, duration_ns: u64) -> Self {
        Self {
            op,
            success: true,
            snapshot_id,
            error: None,
            duration_ns,
        }
    }
    
    /// Create failure result
    pub fn failure(op: SnapshotOp, error: HfsError) -> Self {
        Self {
            op,
            success: false,
            snapshot_id: INVALID_SNAPSHOT_ID,
            error: Some(error),
            duration_ns: 0,
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
    fn test_snapshot_state() {
        assert!(SnapshotState::Active.is_usable());
        assert!(SnapshotState::Locked.is_usable());
        assert!(!SnapshotState::Creating.is_usable());
        
        assert!(SnapshotState::Active.is_modifiable());
        assert!(!SnapshotState::Locked.is_modifiable());
        
        assert!(SnapshotState::Active.is_deletable());
        assert!(!SnapshotState::Locked.is_deletable());
    }
    
    #[test]
    fn test_snapshot_flags() {
        let mut flags = SnapshotFlags::new(0);
        
        assert!(!flags.is_read_only());
        flags.set(SnapshotFlags::READ_ONLY);
        assert!(flags.is_read_only());
        
        flags.clear(SnapshotFlags::READ_ONLY);
        assert!(!flags.is_read_only());
    }
    
    #[test]
    fn test_snapshot_descriptor() {
        let mut desc = SnapshotDescriptor::new(1, ROOT_SNAPSHOT_ID);
        
        assert_eq!(desc.id, 1);
        assert_eq!(desc.parent_id, ROOT_SNAPSHOT_ID);
        assert!(desc.validate().is_ok());
        
        desc.set_name(b"test-snapshot");
        assert_eq!(desc.name(), b"test-snapshot");
        
        desc.set_state(SnapshotState::Active);
        assert_eq!(desc.state(), SnapshotState::Active);
    }
    
    #[test]
    fn test_snapshot_stats() {
        let mut stats = SnapshotStats::new();
        stats.exclusive_blocks = 100;
        stats.shared_blocks = 300;
        
        assert_eq!(stats.savings_ratio(), 0.75);
    }
}
