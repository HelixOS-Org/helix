//! Superblock structure and operations.
//!
//! The superblock is the root of all filesystem metadata, containing global
//! configuration, pointers to key structures, and filesystem state.

use crate::core::types::*;
use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::Crc32c;
use crate::{HFS_MAGIC, BLOCK_SIZE};
use core::mem::size_of;

/// Superblock size in bytes (512 bytes, replicated to fill a block)
pub const SUPERBLOCK_SIZE: usize = 512;

/// Number of superblock replicas per block
pub const SUPERBLOCK_REPLICAS_PER_BLOCK: usize = BLOCK_SIZE / SUPERBLOCK_SIZE;

/// Number of primary superblock copies
pub const PRIMARY_SUPERBLOCK_COPIES: usize = 8;

/// Number of backup superblock copies
pub const BACKUP_SUPERBLOCK_COPIES: usize = 8;

/// Superblock version for compatibility checking
pub const SUPERBLOCK_VERSION: u32 = 1;

/// Superblock flags
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(transparent)]
pub struct SuperblockFlags(pub u64);

impl SuperblockFlags {
    /// Filesystem is read-only
    pub const READONLY: u64 = 1 << 0;
    /// Filesystem is being resized
    pub const RESIZING: u64 = 1 << 1;
    /// Encryption is enabled
    pub const ENCRYPTED: u64 = 1 << 2;
    /// Compression is enabled
    pub const COMPRESSED: u64 = 1 << 3;
    /// Integrity checking is enabled
    pub const INTEGRITY: u64 = 1 << 4;
    /// Filesystem was not cleanly unmounted
    pub const DIRTY: u64 = 1 << 5;
    /// Journal replay is needed
    pub const JOURNAL_REPLAY: u64 = 1 << 6;
    /// Snapshots are enabled
    pub const SNAPSHOTS: u64 = 1 << 7;
    /// Deduplication is enabled
    pub const DEDUP: u64 = 1 << 8;
    /// Large files (>2TB) supported
    pub const LARGE_FILES: u64 = 1 << 9;
    /// Case-insensitive names
    pub const CASEFOLD: u64 = 1 << 10;
    /// Extended attributes enabled
    pub const XATTR: u64 = 1 << 11;
    /// ACLs enabled
    pub const ACL: u64 = 1 << 12;
    /// Quotas enabled
    pub const QUOTA: u64 = 1 << 13;
    
    pub const EMPTY: Self = Self(0);
    
    #[inline]
    pub const fn new(flags: u64) -> Self {
        Self(flags)
    }
    
    #[inline]
    pub const fn contains(self, flag: u64) -> bool {
        (self.0 & flag) != 0
    }
    
    #[inline]
    pub fn set(&mut self, flag: u64) {
        self.0 |= flag;
    }
    
    #[inline]
    pub fn clear(&mut self, flag: u64) {
        self.0 &= !flag;
    }
}

/// Mount state enumeration
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum MountState {
    /// Filesystem is cleanly unmounted
    Clean = 0,
    /// Filesystem is mounted
    Mounted = 1,
    /// Filesystem has errors
    Errors = 2,
    /// Filesystem is being recovered
    Recovering = 3,
}

impl MountState {
    pub fn from_raw(v: u16) -> Self {
        match v {
            0 => Self::Clean,
            1 => Self::Mounted,
            2 => Self::Errors,
            3 => Self::Recovering,
            _ => Self::Errors,
        }
    }
}

/// Error behavior on corruption
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u16)]
pub enum ErrorBehavior {
    /// Continue operation
    Continue = 0,
    /// Remount read-only
    RemountReadonly = 1,
    /// Panic
    Panic = 2,
}

impl ErrorBehavior {
    pub fn from_raw(v: u16) -> Self {
        match v {
            0 => Self::Continue,
            1 => Self::RemountReadonly,
            2 => Self::Panic,
            _ => Self::Continue,
        }
    }
}

/// On-disk superblock structure.
///
/// This is the primary filesystem metadata structure, replicated multiple
/// times across the disk for redundancy. Contains all critical configuration
/// and pointers to key data structures.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct SuperblockRaw {
    /* 0x000 */ /// Magic number ("HELIXFS1")
    pub magic: u64,
    
    /* 0x008 */ /// On-disk format version
    pub version: u32,
    
    /* 0x00C */ /// Feature flags (required)
    pub features_compat: u32,
    
    /* 0x010 */ /// Feature flags (incompatible)
    pub features_incompat: u32,
    
    /* 0x014 */ /// Feature flags (read-only compatible)
    pub features_ro_compat: u32,
    
    /* 0x018 */ /// Filesystem UUID
    pub uuid: [u8; 16],
    
    /* 0x028 */ /// Volume label (null-terminated)
    pub label: [u8; 32],
    
    /* 0x048 */ /// Block size (power of 2, typically 4096)
    pub block_size: u32,
    
    /* 0x04C */ /// Log2 of block size
    pub block_shift: u8,
    
    /* 0x04D */ /// Reserved
    pub _reserved1: [u8; 3],
    
    /* 0x050 */ /// Total blocks in filesystem
    pub total_blocks: u64,
    
    /* 0x058 */ /// Free blocks
    pub free_blocks: u64,
    
    /* 0x060 */ /// Reserved blocks (for root)
    pub reserved_blocks: u64,
    
    /* 0x068 */ /// Total inodes
    pub total_inodes: u64,
    
    /* 0x070 */ /// Free inodes
    pub free_inodes: u64,
    
    /* 0x078 */ /// Root directory inode
    pub root_inode: u64,
    
    /* 0x080 */ /// First data block
    pub first_data_block: u64,
    
    /* 0x088 */ /// Allocation bitmap start block
    pub alloc_bitmap_block: u64,
    
    /* 0x090 */ /// Allocation bitmap size in blocks
    pub alloc_bitmap_size: u64,
    
    /* 0x098 */ /// Inode tree root block
    pub inode_tree_root: u64,
    
    /* 0x0A0 */ /// Extent tree root block
    pub extent_tree_root: u64,
    
    /* 0x0A8 */ /// Directory tree root block
    pub dir_tree_root: u64,
    
    /* 0x0B0 */ /// Journal start block
    pub journal_start: u64,
    
    /* 0x0B8 */ /// Journal size in blocks
    pub journal_size: u64,
    
    /* 0x0C0 */ /// Current journal sequence number
    pub journal_sequence: u64,
    
    /* 0x0C8 */ /// Snapshot tree root block
    pub snapshot_tree_root: u64,
    
    /* 0x0D0 */ /// Current snapshot ID
    pub current_snapshot: u64,
    
    /* 0x0D8 */ /// Number of snapshots
    pub snapshot_count: u32,
    
    /* 0x0DC */ /// Maximum snapshots allowed
    pub max_snapshots: u32,
    
    /* 0x0E0 */ /// Encryption key block (encrypted master key)
    pub crypto_key_block: u64,
    
    /* 0x0E8 */ /// Encryption algorithm
    pub crypto_algorithm: u8,
    
    /* 0x0E9 */ /// Hash algorithm for integrity
    pub hash_algorithm: u8,
    
    /* 0x0EA */ /// Compression algorithm
    pub compress_algorithm: u8,
    
    /* 0x0EB */ /// Compression level (0-9)
    pub compress_level: u8,
    
    /* 0x0EC */ /// Reserved
    pub _reserved2: u32,
    
    /* 0x0F0 */ /// Mount count
    pub mount_count: u32,
    
    /* 0x0F4 */ /// Maximum mounts before fsck
    pub max_mount_count: u32,
    
    /* 0x0F8 */ /// Mount state
    pub state: u16,
    
    /* 0x0FA */ /// Error behavior
    pub errors: u16,
    
    /* 0x0FC */ /// Reserved
    pub _reserved3: u32,
    
    /* 0x100 */ /// Last mount time
    pub mount_time: u64,
    
    /* 0x108 */ /// Last write time
    pub write_time: u64,
    
    /* 0x110 */ /// Last check time
    pub check_time: u64,
    
    /* 0x118 */ /// Creation time
    pub create_time: u64,
    
    /* 0x120 */ /// Filesystem flags
    pub flags: u64,
    
    /* 0x128 */ /// Next inode number to allocate
    pub next_inode: u64,
    
    /* 0x130 */ /// Next transaction ID
    pub next_txn: u64,
    
    /* 0x138 */ /// Merkle root hash of filesystem
    pub merkle_root: [u8; 32],
    
    /* 0x158 */ /// Reserved for future use
    pub _reserved4: [u8; 160],
    
    /* 0x1F8 */ /// CRC32C of superblock (excludes this field)
    pub checksum: u32,
    
    /* 0x1FC */ /// Secondary magic for validation
    pub magic2: u32,
}

// Compile-time size check
const _: () = assert!(size_of::<SuperblockRaw>() == SUPERBLOCK_SIZE);

impl SuperblockRaw {
    /// Secondary magic value
    pub const MAGIC2: u32 = 0x48465321; // "HFS!"
    
    /// Create a new empty superblock
    pub const fn new() -> Self {
        Self {
            magic: HFS_MAGIC,
            version: SUPERBLOCK_VERSION,
            features_compat: 0,
            features_incompat: 0,
            features_ro_compat: 0,
            uuid: [0; 16],
            label: [0; 32],
            block_size: BLOCK_SIZE as u32,
            block_shift: 12,
            _reserved1: [0; 3],
            total_blocks: 0,
            free_blocks: 0,
            reserved_blocks: 0,
            total_inodes: 0,
            free_inodes: 0,
            root_inode: 1,
            first_data_block: 0,
            alloc_bitmap_block: 0,
            alloc_bitmap_size: 0,
            inode_tree_root: 0,
            extent_tree_root: 0,
            dir_tree_root: 0,
            journal_start: 0,
            journal_size: 0,
            journal_sequence: 0,
            snapshot_tree_root: 0,
            current_snapshot: 0,
            snapshot_count: 0,
            max_snapshots: 1024,
            crypto_key_block: 0,
            crypto_algorithm: 0,
            hash_algorithm: 0,
            compress_algorithm: 0,
            compress_level: 0,
            _reserved2: 0,
            mount_count: 0,
            max_mount_count: 100,
            state: MountState::Clean as u16,
            errors: ErrorBehavior::Continue as u16,
            _reserved3: 0,
            mount_time: 0,
            write_time: 0,
            check_time: 0,
            create_time: 0,
            flags: 0,
            next_inode: 2, // 1 is root
            next_txn: 1,
            merkle_root: [0; 32],
            _reserved4: [0; 160],
            checksum: 0,
            magic2: Self::MAGIC2,
        }
    }
    
    /// Calculate checksum of superblock
    pub fn calculate_checksum(&self) -> u32 {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                SUPERBLOCK_SIZE - 8, // Exclude checksum and magic2
            )
        };
        Crc32c::hash(bytes)
    }
    
    /// Update checksum field
    pub fn update_checksum(&mut self) {
        self.checksum = self.calculate_checksum();
    }
    
    /// Validate superblock
    pub fn validate(&self) -> HfsResult<()> {
        // Check magic
        if self.magic != HFS_MAGIC {
            return Err(HfsError::BadMagic);
        }
        
        // Check secondary magic
        if self.magic2 != Self::MAGIC2 {
            return Err(HfsError::BadMagic);
        }
        
        // Check version
        if self.version > SUPERBLOCK_VERSION {
            return Err(HfsError::IncompatibleVersion);
        }
        
        // Check checksum
        if self.checksum != self.calculate_checksum() {
            return Err(HfsError::ChecksumMismatch);
        }
        
        // Check block size is power of 2
        if !self.block_size.is_power_of_two() || self.block_size < 512 {
            return Err(HfsError::SuperblockCorruption);
        }
        
        // Check block shift
        if (1u32 << self.block_shift) != self.block_size {
            return Err(HfsError::SuperblockCorruption);
        }
        
        // Check free blocks <= total blocks
        if self.free_blocks > self.total_blocks {
            return Err(HfsError::SuperblockCorruption);
        }
        
        Ok(())
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; SUPERBLOCK_SIZE] {
        unsafe { core::mem::transmute_copy(self) }
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; SUPERBLOCK_SIZE]) -> Self {
        unsafe { core::ptr::read(bytes.as_ptr() as *const Self) }
    }
    
    /// Get label as string
    pub fn label_str(&self) -> &str {
        let len = self.label.iter().position(|&b| b == 0).unwrap_or(32);
        core::str::from_utf8(&self.label[..len]).unwrap_or("")
    }
    
    /// Set label from string
    pub fn set_label(&mut self, label: &str) {
        let bytes = label.as_bytes();
        let len = core::cmp::min(bytes.len(), 31);
        self.label[..len].copy_from_slice(&bytes[..len]);
        self.label[len] = 0;
    }
}

impl Default for SuperblockRaw {
    fn default() -> Self {
        Self::new()
    }
}

/// High-level superblock wrapper with caching and validation.
pub struct Superblock {
    /// Raw on-disk superblock
    raw: SuperblockRaw,
    /// Block number where this superblock is stored
    block_num: BlockNum,
    /// Whether the superblock has been modified
    dirty: bool,
}

impl Superblock {
    /// Create new superblock
    pub fn new(raw: SuperblockRaw, block_num: BlockNum) -> Self {
        Self {
            raw,
            block_num,
            dirty: false,
        }
    }
    
    /// Create a fresh superblock for mkfs
    pub fn create(
        total_blocks: u64,
        block_size: u32,
        uuid: [u8; 16],
        label: &str,
    ) -> Self {
        let mut raw = SuperblockRaw::new();
        
        raw.block_size = block_size;
        raw.block_shift = block_size.trailing_zeros() as u8;
        raw.total_blocks = total_blocks;
        raw.free_blocks = total_blocks; // Will be updated during mkfs
        raw.uuid = uuid;
        raw.set_label(label);
        
        // Calculate layout
        let layout = crate::disk::layout::DiskLayout::calculate(total_blocks, block_size);
        
        raw.alloc_bitmap_block = layout.alloc_bitmap_start;
        raw.alloc_bitmap_size = layout.alloc_bitmap_blocks;
        raw.journal_start = layout.journal_start;
        raw.journal_size = layout.journal_blocks;
        raw.first_data_block = layout.data_start;
        
        raw.update_checksum();
        
        Self {
            raw,
            block_num: BlockNum::new(0),
            dirty: true,
        }
    }
    
    /// Get raw superblock reference
    #[inline]
    pub fn raw(&self) -> &SuperblockRaw {
        &self.raw
    }
    
    /// Get mutable raw superblock (marks dirty)
    #[inline]
    pub fn raw_mut(&mut self) -> &mut SuperblockRaw {
        self.dirty = true;
        &mut self.raw
    }
    
    /// Check if dirty
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    
    /// Mark as clean
    #[inline]
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }
    
    /// Get block size
    #[inline]
    pub fn block_size(&self) -> u32 {
        self.raw.block_size
    }
    
    /// Get block shift
    #[inline]
    pub fn block_shift(&self) -> u32 {
        self.raw.block_shift as u32
    }
    
    /// Get total blocks
    #[inline]
    pub fn total_blocks(&self) -> u64 {
        self.raw.total_blocks
    }
    
    /// Get free blocks
    #[inline]
    pub fn free_blocks(&self) -> u64 {
        self.raw.free_blocks
    }
    
    /// Get root inode number
    #[inline]
    pub fn root_inode(&self) -> InodeNum {
        InodeNum::new(self.raw.root_inode)
    }
    
    /// Get flags
    #[inline]
    pub fn flags(&self) -> SuperblockFlags {
        SuperblockFlags::new(self.raw.flags)
    }
    
    /// Check if read-only
    #[inline]
    pub fn is_readonly(&self) -> bool {
        self.flags().contains(SuperblockFlags::READONLY)
    }
    
    /// Check if dirty (needs journal replay)
    #[inline]
    pub fn needs_recovery(&self) -> bool {
        self.raw.state == MountState::Mounted as u16 ||
        self.flags().contains(SuperblockFlags::JOURNAL_REPLAY)
    }
    
    /// Update free block count
    pub fn update_free_blocks(&mut self, count: u64) {
        self.raw.free_blocks = count;
        self.dirty = true;
    }
    
    /// Allocate next inode number
    pub fn alloc_inode(&mut self) -> InodeNum {
        let ino = InodeNum::new(self.raw.next_inode);
        self.raw.next_inode += 1;
        self.raw.free_inodes = self.raw.free_inodes.saturating_sub(1);
        self.dirty = true;
        ino
    }
    
    /// Allocate next transaction ID
    pub fn alloc_txn(&mut self) -> TxnId {
        let txn = TxnId::new(self.raw.next_txn);
        self.raw.next_txn += 1;
        self.dirty = true;
        txn
    }
    
    /// Set mount state
    pub fn set_state(&mut self, state: MountState) {
        self.raw.state = state as u16;
        self.dirty = true;
    }
    
    /// Update timestamps
    pub fn update_times(&mut self, now: u64) {
        self.raw.write_time = now;
        self.dirty = true;
    }
    
    /// Increment mount count
    pub fn increment_mount(&mut self, now: u64) {
        self.raw.mount_count += 1;
        self.raw.mount_time = now;
        self.raw.state = MountState::Mounted as u16;
        self.dirty = true;
    }
    
    /// Prepare for sync (update checksum)
    pub fn prepare_sync(&mut self) {
        self.raw.update_checksum();
    }
    
    /// Get serialized bytes for writing
    pub fn to_bytes(&self) -> [u8; SUPERBLOCK_SIZE] {
        self.raw.to_bytes()
    }
}

/// Superblock location strategy for redundancy.
pub struct SuperblockLocations {
    /// Primary copies (blocks 0-7)
    pub primary: [BlockNum; PRIMARY_SUPERBLOCK_COPIES],
    /// Backup copies (spread across disk)
    pub backup: [BlockNum; BACKUP_SUPERBLOCK_COPIES],
}

impl SuperblockLocations {
    /// Calculate superblock locations for a filesystem
    pub fn calculate(total_blocks: u64) -> Self {
        let mut primary = [BlockNum::NULL; PRIMARY_SUPERBLOCK_COPIES];
        let mut backup = [BlockNum::NULL; BACKUP_SUPERBLOCK_COPIES];
        
        // Primary: first 8 blocks
        for i in 0..PRIMARY_SUPERBLOCK_COPIES {
            primary[i] = BlockNum::new(i as u64);
        }
        
        // Backup: spread across disk at power-of-2 intervals
        if total_blocks >= 1024 {
            backup[0] = BlockNum::new(1024);
        }
        if total_blocks >= 32768 {
            backup[1] = BlockNum::new(32768);
        }
        if total_blocks >= 1048576 {
            backup[2] = BlockNum::new(1048576);
        }
        if total_blocks >= 33554432 {
            backup[3] = BlockNum::new(33554432);
        }
        
        // Additional backups near end of disk
        if total_blocks > 1024 {
            backup[4] = BlockNum::new(total_blocks - 8);
            backup[5] = BlockNum::new(total_blocks - 16);
            backup[6] = BlockNum::new(total_blocks - 32);
            backup[7] = BlockNum::new(total_blocks - 64);
        }
        
        Self { primary, backup }
    }
    
    /// Get all valid superblock locations
    pub fn all_locations(&self) -> impl Iterator<Item = BlockNum> + '_ {
        self.primary.iter()
            .chain(self.backup.iter())
            .filter(|b| b.is_valid())
            .copied()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_superblock_size() {
        assert_eq!(size_of::<SuperblockRaw>(), 512);
    }
    
    #[test]
    fn test_superblock_checksum() {
        let mut sb = SuperblockRaw::new();
        sb.total_blocks = 1000000;
        sb.update_checksum();
        
        assert!(sb.validate().is_ok());
        
        // Corrupt and verify detection
        sb.total_blocks = 999999;
        assert!(matches!(sb.validate(), Err(HfsError::ChecksumMismatch)));
    }
    
    #[test]
    fn test_superblock_label() {
        let mut sb = SuperblockRaw::new();
        sb.set_label("My Volume");
        assert_eq!(sb.label_str(), "My Volume");
        
        // Long label truncation
        sb.set_label("This is a very long volume label that exceeds 32 bytes");
        assert_eq!(sb.label_str().len(), 31);
    }
    
    #[test]
    fn test_superblock_locations() {
        let locs = SuperblockLocations::calculate(1_000_000);
        
        // Primary at start
        assert_eq!(locs.primary[0].get(), 0);
        assert_eq!(locs.primary[7].get(), 7);
        
        // Backups spread out
        assert_eq!(locs.backup[0].get(), 1024);
        assert_eq!(locs.backup[1].get(), 32768);
    }
}
