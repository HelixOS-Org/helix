//! Superblock Operations
//!
//! Manages filesystem superblock and global state.

use crate::core::error::{HfsError, HfsResult};
use crate::api::FsStats as ApiFsStats;
use super::ROOT_INO;

// ============================================================================
// Superblock State
// ============================================================================

/// Superblock state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SuperblockState {
    /// Not mounted
    Unmounted = 0,
    /// Clean mount
    Clean = 1,
    /// Dirty (has uncommitted changes)
    Dirty = 2,
    /// Error state
    Error = 3,
    /// Mounting in progress
    Mounting = 4,
    /// Unmounting in progress
    Unmounting = 5,
}

impl Default for SuperblockState {
    fn default() -> Self {
        Self::Unmounted
    }
}

// ============================================================================
// Superblock Flags
// ============================================================================

/// Superblock flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct SuperblockFlags(pub u32);

impl SuperblockFlags {
    /// Read-only mount
    pub const SB_RDONLY: u32 = 1 << 0;
    /// Mandatory locking
    pub const SB_MANDLOCK: u32 = 1 << 1;
    /// Directory sync
    pub const SB_DIRSYNC: u32 = 1 << 2;
    /// No atime updates
    pub const SB_NOATIME: u32 = 1 << 3;
    /// No diratime updates
    pub const SB_NODIRATIME: u32 = 1 << 4;
    /// Silent mount
    pub const SB_SILENT: u32 = 1 << 5;
    /// POSIX ACL support
    pub const SB_POSIXACL: u32 = 1 << 6;
    /// Synchronous mount
    pub const SB_SYNCHRONOUS: u32 = 1 << 7;
    /// Lazy time
    pub const SB_LAZYTIME: u32 = 1 << 8;
    
    /// Check flag
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
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
    pub fn is_readonly(&self) -> bool {
        self.has(Self::SB_RDONLY)
    }
}

// ============================================================================
// VFS Superblock
// ============================================================================

/// VFS superblock.
#[derive(Clone, Copy)]
pub struct VfsSuperblock {
    /// Device ID
    pub dev: u64,
    /// Block size
    pub block_size: u32,
    /// Maximum file size
    pub max_file_size: u64,
    /// Filesystem type magic
    pub magic: u32,
    /// Flags
    pub flags: SuperblockFlags,
    /// State
    pub state: SuperblockState,
    /// Root inode number
    pub root_ino: u64,
    /// Total blocks
    pub total_blocks: u64,
    /// Free blocks
    pub free_blocks: u64,
    /// Total inodes
    pub total_inodes: u64,
    /// Free inodes
    pub free_inodes: u64,
    /// Mount count
    pub mount_count: u32,
    /// Maximum mount count before fsck
    pub max_mount_count: u32,
    /// Last mount time
    pub mount_time: u64,
    /// Last write time
    pub write_time: u64,
    /// Last fsck time
    pub fsck_time: u64,
    /// Filesystem UUID
    pub uuid: [u8; 16],
    /// Volume label
    pub label: [u8; 64],
    /// Label length
    pub label_len: u8,
}

impl VfsSuperblock {
    /// Create new superblock
    pub fn new(dev: u64, block_size: u32) -> Self {
        Self {
            dev,
            block_size,
            max_file_size: u64::MAX,
            magic: 0x48454C58, // "HELX"
            flags: SuperblockFlags::default(),
            state: SuperblockState::Unmounted,
            root_ino: ROOT_INO,
            total_blocks: 0,
            free_blocks: 0,
            total_inodes: 0,
            free_inodes: 0,
            mount_count: 0,
            max_mount_count: 50,
            mount_time: 0,
            write_time: 0,
            fsck_time: 0,
            uuid: [0; 16],
            label: [0; 64],
            label_len: 0,
        }
    }
    
    /// Set label
    pub fn set_label(&mut self, label: &[u8]) {
        let len = core::cmp::min(label.len(), 64);
        self.label[..len].copy_from_slice(&label[..len]);
        self.label_len = len as u8;
    }
    
    /// Get label
    pub fn label(&self) -> &[u8] {
        &self.label[..self.label_len as usize]
    }
    
    /// Is read-only
    #[inline]
    pub fn is_readonly(&self) -> bool {
        self.flags.is_readonly()
    }
    
    /// Is clean
    #[inline]
    pub fn is_clean(&self) -> bool {
        self.state == SuperblockState::Clean
    }
    
    /// Mark dirty
    pub fn mark_dirty(&mut self) {
        if self.state == SuperblockState::Clean {
            self.state = SuperblockState::Dirty;
        }
    }
    
    /// Mark clean
    pub fn mark_clean(&mut self) {
        if self.state == SuperblockState::Dirty {
            self.state = SuperblockState::Clean;
        }
    }
    
    /// Convert to FsStats
    pub fn to_stats(&self) -> ApiFsStats {
        ApiFsStats {
            f_bsize: self.block_size as u64,
            f_frsize: self.block_size as u64,
            f_blocks: self.total_blocks,
            f_bfree: self.free_blocks,
            f_bavail: self.free_blocks,
            f_files: self.total_inodes,
            f_ffree: self.free_inodes,
            f_favail: self.free_inodes,
            f_fsid: self.dev,
            f_flag: self.flags.0 as u64,
            f_namemax: 255,
        }
    }
    
    /// Used blocks
    pub fn used_blocks(&self) -> u64 {
        self.total_blocks.saturating_sub(self.free_blocks)
    }
    
    /// Used inodes
    pub fn used_inodes(&self) -> u64 {
        self.total_inodes.saturating_sub(self.free_inodes)
    }
    
    /// Usage percentage
    pub fn usage_percent(&self) -> u32 {
        if self.total_blocks == 0 {
            return 0;
        }
        ((self.used_blocks() * 100) / self.total_blocks) as u32
    }
}

impl Default for VfsSuperblock {
    fn default() -> Self {
        Self::new(0, 4096)
    }
}

// ============================================================================
// Superblock Table
// ============================================================================

/// Maximum mounted filesystems
const MAX_SUPERBLOCKS: usize = 64;

/// Superblock table.
pub struct SuperblockTable {
    /// Superblocks
    superblocks: [Option<VfsSuperblock>; MAX_SUPERBLOCKS],
    /// Count
    count: usize,
}

impl SuperblockTable {
    /// Create new superblock table
    pub const fn new() -> Self {
        const NONE: Option<VfsSuperblock> = None;
        Self {
            superblocks: [NONE; MAX_SUPERBLOCKS],
            count: 0,
        }
    }
    
    /// Register superblock
    pub fn register(&mut self, sb: VfsSuperblock) -> HfsResult<usize> {
        for i in 0..MAX_SUPERBLOCKS {
            if self.superblocks[i].is_none() {
                self.superblocks[i] = Some(sb);
                self.count += 1;
                return Ok(i);
            }
        }
        
        Err(HfsError::NoSpace)
    }
    
    /// Unregister superblock
    pub fn unregister(&mut self, idx: usize) -> HfsResult<VfsSuperblock> {
        if idx >= MAX_SUPERBLOCKS {
            return Err(HfsError::InvalidArgument);
        }
        
        match self.superblocks[idx].take() {
            Some(sb) => {
                self.count -= 1;
                Ok(sb)
            }
            None => Err(HfsError::NotFound),
        }
    }
    
    /// Get superblock
    pub fn get(&self, idx: usize) -> Option<&VfsSuperblock> {
        self.superblocks.get(idx).and_then(|s| s.as_ref())
    }
    
    /// Get mutable superblock
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut VfsSuperblock> {
        self.superblocks.get_mut(idx).and_then(|s| s.as_mut())
    }
    
    /// Find by device
    pub fn find_by_dev(&self, dev: u64) -> Option<(usize, &VfsSuperblock)> {
        for i in 0..MAX_SUPERBLOCKS {
            if let Some(sb) = &self.superblocks[i] {
                if sb.dev == dev {
                    return Some((i, sb));
                }
            }
        }
        None
    }
    
    /// Iterate over superblocks
    pub fn iter(&self) -> impl Iterator<Item = (usize, &VfsSuperblock)> {
        self.superblocks.iter()
            .enumerate()
            .filter_map(|(i, s)| s.as_ref().map(|sb| (i, sb)))
    }
    
    /// Count
    pub fn count(&self) -> usize {
        self.count
    }
    
    /// Sync all dirty superblocks
    pub fn sync_all(&mut self) {
        for sb in self.superblocks.iter_mut().flatten() {
            if sb.state == SuperblockState::Dirty {
                // Mark clean (actual I/O would be done by caller)
                sb.mark_clean();
            }
        }
    }
}

impl Default for SuperblockTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_superblock() {
        let mut sb = VfsSuperblock::new(1, 4096);
        
        assert_eq!(sb.block_size, 4096);
        assert!(sb.state == SuperblockState::Unmounted);
        
        sb.set_label(b"TestVolume");
        assert_eq!(sb.label(), b"TestVolume");
    }
    
    #[test]
    fn test_superblock_dirty() {
        let mut sb = VfsSuperblock::new(1, 4096);
        sb.state = SuperblockState::Clean;
        
        assert!(sb.is_clean());
        
        sb.mark_dirty();
        assert!(!sb.is_clean());
        
        sb.mark_clean();
        assert!(sb.is_clean());
    }
    
    #[test]
    fn test_superblock_stats() {
        let mut sb = VfsSuperblock::new(1, 4096);
        sb.total_blocks = 1000;
        sb.free_blocks = 500;
        sb.total_inodes = 100;
        sb.free_inodes = 50;
        
        let stats = sb.to_stats();
        assert_eq!(stats.f_blocks, 1000);
        assert_eq!(stats.f_bfree, 500);
        assert_eq!(sb.usage_percent(), 50);
    }
    
    #[test]
    fn test_superblock_table() {
        let mut table = SuperblockTable::new();
        
        let sb = VfsSuperblock::new(1, 4096);
        let idx = table.register(sb).unwrap();
        
        assert_eq!(table.count(), 1);
        
        let sb = table.get(idx).unwrap();
        assert_eq!(sb.dev, 1);
        
        let (found_idx, _) = table.find_by_dev(1).unwrap();
        assert_eq!(found_idx, idx);
        
        table.unregister(idx).unwrap();
        assert_eq!(table.count(), 0);
    }
}
