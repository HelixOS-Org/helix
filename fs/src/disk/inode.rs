//! On-disk inode structure and operations.
//!
//! Inodes are the fundamental metadata structure for files and directories.
//! Each inode contains file attributes, ownership, timestamps, and pointers
//! to the file's data extents.

use crate::core::types::*;
use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::Crc32c;
use core::mem::size_of;

/// Inode size on disk (256 bytes, power of 2 for alignment)
pub const INODE_SIZE: usize = 256;

/// Maximum inline data size (for small files stored directly in inode)
pub const MAX_INLINE_DATA: usize = 64;

/// Maximum inline extents in inode
pub const MAX_INLINE_EXTENTS: usize = 4;

/// Maximum inline symlink length
pub const MAX_INLINE_SYMLINK: usize = 176;

/// Inode flags for on-disk storage
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(transparent)]
pub struct OnDiskInodeFlags(pub u32);

impl OnDiskInodeFlags {
    /// Immutable file
    pub const IMMUTABLE: u32 = 1 << 0;
    /// Append-only file
    pub const APPEND_ONLY: u32 = 1 << 1;
    /// No dump
    pub const NO_DUMP: u32 = 1 << 2;
    /// No atime updates
    pub const NO_ATIME: u32 = 1 << 3;
    /// Synchronous updates
    pub const SYNC: u32 = 1 << 4;
    /// Directory sync
    pub const DIR_SYNC: u32 = 1 << 5;
    /// Compressed content
    pub const COMPRESSED: u32 = 1 << 6;
    /// Encrypted content
    pub const ENCRYPTED: u32 = 1 << 7;
    /// Has inline data
    pub const INLINE_DATA: u32 = 1 << 8;
    /// Uses extent tree
    pub const EXTENT_TREE: u32 = 1 << 9;
    /// Is a snapshot
    pub const SNAPSHOT: u32 = 1 << 10;
    /// Is a clone
    pub const CLONE: u32 = 1 << 11;
    /// Has extended attributes
    pub const HAS_XATTR: u32 = 1 << 12;
    /// Has ACL
    pub const HAS_ACL: u32 = 1 << 13;
    /// Has integrity verification
    pub const HAS_INTEGRITY: u32 = 1 << 14;
    /// Marked for deletion
    pub const ORPHAN: u32 = 1 << 15;
    /// Has inline extents
    pub const INLINE_EXTENTS: u32 = 1 << 16;
    /// Has inline symlink
    pub const INLINE_SYMLINK: u32 = 1 << 17;
}

/// On-disk inode structure.
///
/// This is the primary metadata structure for files and directories.
/// It contains all file attributes and either inline data/extents or
/// pointers to external extent trees.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct InodeRaw {
    /* 0x00 */ /// Inode number
    pub ino: u64,
    
    /* 0x08 */ /// Parent inode (for orphan recovery)
    pub parent_ino: u64,
    
    /* 0x10 */ /// File mode (type + permissions)
    pub mode: u32,
    
    /* 0x14 */ /// Inode flags
    pub flags: u32,
    
    /* 0x18 */ /// Owner user ID
    pub uid: u32,
    
    /* 0x1C */ /// Owner group ID
    pub gid: u32,
    
    /* 0x20 */ /// File size in bytes
    pub size: u64,
    
    /* 0x28 */ /// Allocated blocks (in filesystem blocks)
    pub blocks: u64,
    
    /* 0x30 */ /// Hard link count
    pub nlink: u32,
    
    /* 0x34 */ /// Generation number (for NFS)
    pub generation: u32,
    
    /* 0x38 */ /// Access time (nanoseconds since epoch)
    pub atime: u64,
    
    /* 0x40 */ /// Modification time
    pub mtime: u64,
    
    /* 0x48 */ /// Change time (inode change)
    pub ctime: u64,
    
    /* 0x50 */ /// Creation time
    pub crtime: u64,
    
    /* 0x58 */ /// Snapshot ID this inode belongs to
    pub snapshot_id: u64,
    
    /* 0x60 */ /// Transaction ID of last modification
    pub txn_id: u64,
    
    /* 0x68 */ /// Extent tree root block (if not inline)
    pub extent_root: u64,
    
    /* 0x70 */ /// Extended attribute block
    pub xattr_block: u64,
    
    /* 0x78 */ /// ACL block
    pub acl_block: u64,
    
    /* 0x80 */ /// Encryption key ID
    pub crypto_key_id: u64,
    
    /* 0x88 */ /// Content hash (for integrity)
    pub content_hash: [u8; 32],
    
    /* 0xA8 */ /// Compression type
    pub compress_type: u8,
    
    /* 0xA9 */ /// Encryption type
    pub encrypt_type: u8,
    
    /* 0xAA */ /// Hash type for integrity
    pub hash_type: u8,
    
    /* 0xAB */ /// Number of inline extents
    pub inline_extent_count: u8,
    
    /* 0xAC */ /// Device number (for device files)
    pub rdev: u32,
    
    /* 0xB0 */ /// Inline data/extents/symlink (64 bytes)
    /// Layout depends on inode type and flags:
    /// - Regular file with INLINE_DATA: raw file data
    /// - Regular file with INLINE_EXTENTS: up to 4 extent entries
    /// - Symlink with INLINE_SYMLINK: target path (extends to reserved)
    /// - Directory: first entry or root of dir entries
    pub inline: [u8; 64],
    
    /* 0xF0 */ /// Checksum of inode
    pub checksum: u32,
    
    /* 0xF4 */ /// Reserved for future use
    pub _reserved: [u8; 12],
}

// Compile-time size check
const _: () = assert!(size_of::<InodeRaw>() == INODE_SIZE);

impl InodeRaw {
    /// Create a new empty inode
    pub const fn new() -> Self {
        Self {
            ino: 0,
            parent_ino: 0,
            mode: 0,
            flags: 0,
            uid: 0,
            gid: 0,
            size: 0,
            blocks: 0,
            nlink: 0,
            generation: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            crtime: 0,
            snapshot_id: 0,
            txn_id: 0,
            extent_root: 0,
            xattr_block: 0,
            acl_block: 0,
            crypto_key_id: 0,
            content_hash: [0; 32],
            compress_type: 0,
            encrypt_type: 0,
            hash_type: 0,
            inline_extent_count: 0,
            rdev: 0,
            inline: [0; 64],
            checksum: 0,
            _reserved: [0; 12],
        }
    }
    
    /// Create inode for a regular file
    pub fn new_file(ino: u64, mode: u32, uid: u32, gid: u32, now: u64) -> Self {
        let mut inode = Self::new();
        inode.ino = ino;
        inode.mode = FileMode::S_IFREG | (mode & 0o7777);
        inode.uid = uid;
        inode.gid = gid;
        inode.nlink = 1;
        inode.atime = now;
        inode.mtime = now;
        inode.ctime = now;
        inode.crtime = now;
        inode
    }
    
    /// Create inode for a directory
    pub fn new_dir(ino: u64, parent_ino: u64, mode: u32, uid: u32, gid: u32, now: u64) -> Self {
        let mut inode = Self::new();
        inode.ino = ino;
        inode.parent_ino = parent_ino;
        inode.mode = FileMode::S_IFDIR | (mode & 0o7777);
        inode.uid = uid;
        inode.gid = gid;
        inode.nlink = 2; // . and parent's link
        inode.atime = now;
        inode.mtime = now;
        inode.ctime = now;
        inode.crtime = now;
        inode
    }
    
    /// Create inode for a symbolic link
    pub fn new_symlink(ino: u64, target: &[u8], uid: u32, gid: u32, now: u64) -> Self {
        let mut inode = Self::new();
        inode.ino = ino;
        inode.mode = FileMode::S_IFLNK | 0o777;
        inode.uid = uid;
        inode.gid = gid;
        inode.nlink = 1;
        inode.size = target.len() as u64;
        inode.atime = now;
        inode.mtime = now;
        inode.ctime = now;
        inode.crtime = now;
        
        // Store inline if small enough
        if target.len() <= MAX_INLINE_SYMLINK {
            inode.flags |= OnDiskInodeFlags::INLINE_SYMLINK;
            let len = target.len().min(64);
            inode.inline[..len].copy_from_slice(&target[..len]);
            // Extended symlink data goes in reserved area for longer links
        }
        
        inode
    }
    
    /// Create inode for a device file
    pub fn new_device(
        ino: u64,
        mode: u32,
        rdev: u32,
        uid: u32,
        gid: u32,
        now: u64,
    ) -> Self {
        let mut inode = Self::new();
        inode.ino = ino;
        inode.mode = mode; // Should include S_IFCHR or S_IFBLK
        inode.uid = uid;
        inode.gid = gid;
        inode.nlink = 1;
        inode.rdev = rdev;
        inode.atime = now;
        inode.mtime = now;
        inode.ctime = now;
        inode.crtime = now;
        inode
    }
    
    /// Calculate checksum
    pub fn calculate_checksum(&self) -> u32 {
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                INODE_SIZE - 16, // Exclude checksum and reserved
            )
        };
        Crc32c::hash(bytes)
    }
    
    /// Update checksum
    pub fn update_checksum(&mut self) {
        self.checksum = self.calculate_checksum();
    }
    
    /// Validate inode
    pub fn validate(&self) -> HfsResult<()> {
        // Check checksum
        if self.checksum != self.calculate_checksum() {
            return Err(HfsError::ChecksumMismatch);
        }
        
        // Check inode number is valid
        if self.ino == 0 {
            return Err(HfsError::InodeCorruption);
        }
        
        // Check mode has valid file type
        let file_type = self.mode & FileMode::S_IFMT;
        if file_type == 0 {
            return Err(HfsError::InodeCorruption);
        }
        
        // Check nlink for regular files/dirs
        if self.file_type() != FileType::Unknown && self.nlink == 0 {
            return Err(HfsError::InodeCorruption);
        }
        
        Ok(())
    }
    
    /// Get file type
    #[inline]
    pub fn file_type(&self) -> FileType {
        FileMode::new(self.mode).file_type()
    }
    
    /// Check if this is a regular file
    #[inline]
    pub fn is_file(&self) -> bool {
        (self.mode & FileMode::S_IFMT) == FileMode::S_IFREG
    }
    
    /// Check if this is a directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        (self.mode & FileMode::S_IFMT) == FileMode::S_IFDIR
    }
    
    /// Check if this is a symbolic link
    #[inline]
    pub fn is_symlink(&self) -> bool {
        (self.mode & FileMode::S_IFMT) == FileMode::S_IFLNK
    }
    
    /// Check if this has inline data
    #[inline]
    pub fn has_inline_data(&self) -> bool {
        (self.flags & OnDiskInodeFlags::INLINE_DATA) != 0
    }
    
    /// Check if this uses inline extents
    #[inline]
    pub fn has_inline_extents(&self) -> bool {
        (self.flags & OnDiskInodeFlags::INLINE_EXTENTS) != 0
    }
    
    /// Check if this uses an extent tree
    #[inline]
    pub fn has_extent_tree(&self) -> bool {
        (self.flags & OnDiskInodeFlags::EXTENT_TREE) != 0
    }
    
    /// Get inline data (if present)
    pub fn inline_data(&self) -> Option<&[u8]> {
        if self.has_inline_data() {
            let len = (self.size as usize).min(MAX_INLINE_DATA);
            Some(&self.inline[..len])
        } else {
            None
        }
    }
    
    /// Get inline extents (if present)
    pub fn inline_extents(&self) -> Option<&[Extent]> {
        if self.has_inline_extents() && self.inline_extent_count > 0 {
            let count = (self.inline_extent_count as usize).min(MAX_INLINE_EXTENTS);
            // SAFETY: inline is properly aligned and sized for Extent
            let extents = unsafe {
                core::slice::from_raw_parts(
                    self.inline.as_ptr() as *const Extent,
                    count,
                )
            };
            Some(extents)
        } else {
            None
        }
    }
    
    /// Set inline data
    pub fn set_inline_data(&mut self, data: &[u8]) -> HfsResult<()> {
        if data.len() > MAX_INLINE_DATA {
            return Err(HfsError::FileTooLarge);
        }
        
        self.inline[..data.len()].copy_from_slice(data);
        self.size = data.len() as u64;
        self.flags |= OnDiskInodeFlags::INLINE_DATA;
        self.flags &= !OnDiskInodeFlags::INLINE_EXTENTS;
        self.flags &= !OnDiskInodeFlags::EXTENT_TREE;
        
        Ok(())
    }
    
    /// Set inline extents
    pub fn set_inline_extents(&mut self, extents: &[Extent]) -> HfsResult<()> {
        if extents.len() > MAX_INLINE_EXTENTS {
            return Err(HfsError::ExtentTreeFull);
        }
        
        self.inline_extent_count = extents.len() as u8;
        
        // Copy extents to inline storage
        let extent_bytes = unsafe {
            core::slice::from_raw_parts(
                extents.as_ptr() as *const u8,
                extents.len() * Extent::SIZE,
            )
        };
        self.inline[..extent_bytes.len()].copy_from_slice(extent_bytes);
        
        self.flags |= OnDiskInodeFlags::INLINE_EXTENTS;
        self.flags &= !OnDiskInodeFlags::INLINE_DATA;
        
        Ok(())
    }
    
    /// Get symlink target (if inline)
    pub fn symlink_target(&self) -> Option<&[u8]> {
        if self.is_symlink() && (self.flags & OnDiskInodeFlags::INLINE_SYMLINK) != 0 {
            let len = (self.size as usize).min(64);
            Some(&self.inline[..len])
        } else {
            None
        }
    }
    
    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; INODE_SIZE] {
        unsafe { core::mem::transmute_copy(self) }
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; INODE_SIZE]) -> Self {
        unsafe { core::ptr::read(bytes.as_ptr() as *const Self) }
    }
    
    /// Convert to FileStat
    pub fn to_stat(&self) -> FileStat {
        FileStat {
            ino: InodeNum::new(self.ino),
            mode: FileMode::new(self.mode),
            nlink: self.nlink,
            uid: self.uid,
            gid: self.gid,
            rdev: self.rdev as u64,
            size: self.size,
            blksize: 4096, // Default block size
            blocks: self.blocks * 8, // Convert to 512-byte blocks
            atime: self.atime,
            mtime: self.mtime,
            ctime: self.ctime,
            crtime: self.crtime,
            generation: Generation::new(self.generation as u64),
            flags: InodeFlags::new(self.flags as u64),
        }
    }
}

impl Default for InodeRaw {
    fn default() -> Self {
        Self::new()
    }
}

/// In-memory inode with additional state.
///
/// This wraps the on-disk inode with runtime state like dirty flags,
/// locks, and cached data.
pub struct Inode {
    /// On-disk data
    raw: InodeRaw,
    /// Block number where inode is stored (if any)
    disk_location: Option<BlockNum>,
    /// Dirty flag
    dirty: bool,
    /// Reference count
    ref_count: u32,
    /// Open file count
    open_count: u32,
}

impl Inode {
    /// Create from raw inode
    pub fn from_raw(raw: InodeRaw, location: Option<BlockNum>) -> Self {
        Self {
            raw,
            disk_location: location,
            dirty: false,
            ref_count: 1,
            open_count: 0,
        }
    }
    
    /// Create new inode (not yet on disk)
    pub fn new(raw: InodeRaw) -> Self {
        Self::from_raw(raw, None)
    }
    
    /// Get raw inode
    #[inline]
    pub fn raw(&self) -> &InodeRaw {
        &self.raw
    }
    
    /// Get mutable raw inode (marks dirty)
    #[inline]
    pub fn raw_mut(&mut self) -> &mut InodeRaw {
        self.dirty = true;
        &mut self.raw
    }
    
    /// Get inode number
    #[inline]
    pub fn ino(&self) -> InodeNum {
        InodeNum::new(self.raw.ino)
    }
    
    /// Get file type
    #[inline]
    pub fn file_type(&self) -> FileType {
        self.raw.file_type()
    }
    
    /// Get file size
    #[inline]
    pub fn size(&self) -> u64 {
        self.raw.size
    }
    
    /// Set file size
    #[inline]
    pub fn set_size(&mut self, size: u64) {
        self.raw.size = size;
        self.dirty = true;
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
    
    /// Increment reference count
    #[inline]
    pub fn add_ref(&mut self) {
        self.ref_count += 1;
    }
    
    /// Decrement reference count, return true if last reference
    #[inline]
    pub fn release(&mut self) -> bool {
        self.ref_count = self.ref_count.saturating_sub(1);
        self.ref_count == 0
    }
    
    /// Increment open count
    #[inline]
    pub fn add_open(&mut self) {
        self.open_count += 1;
    }
    
    /// Decrement open count
    #[inline]
    pub fn close(&mut self) -> bool {
        self.open_count = self.open_count.saturating_sub(1);
        self.open_count == 0
    }
    
    /// Check if file is open
    #[inline]
    pub fn is_open(&self) -> bool {
        self.open_count > 0
    }
    
    /// Update access time
    pub fn touch_atime(&mut self, now: u64) {
        if (self.raw.flags & OnDiskInodeFlags::NO_ATIME) == 0 {
            self.raw.atime = now;
            self.dirty = true;
        }
    }
    
    /// Update modification time
    pub fn touch_mtime(&mut self, now: u64) {
        self.raw.mtime = now;
        self.raw.ctime = now;
        self.dirty = true;
    }
    
    /// Update change time
    pub fn touch_ctime(&mut self, now: u64) {
        self.raw.ctime = now;
        self.dirty = true;
    }
    
    /// Convert to stat
    #[inline]
    pub fn stat(&self) -> FileStat {
        self.raw.to_stat()
    }
    
    /// Prepare for sync
    pub fn prepare_sync(&mut self) {
        self.raw.update_checksum();
    }
}

// ============================================================================
// Inode Key for B-tree
// ============================================================================

/// Key for inode lookup in B-tree.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(C, packed)]
pub struct InodeKey {
    /// Snapshot ID (for versioning)
    pub snapshot_id: u64,
    /// Inode number
    pub ino: u64,
}

impl InodeKey {
    /// Create new key
    #[inline]
    pub const fn new(snapshot_id: u64, ino: u64) -> Self {
        Self { snapshot_id, ino }
    }
    
    /// Create key for current (live) version
    #[inline]
    pub const fn current(ino: u64) -> Self {
        Self::new(u64::MAX, ino)
    }
    
    /// Size in bytes
    pub const SIZE: usize = 16;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_inode_size() {
        assert_eq!(size_of::<InodeRaw>(), 256);
    }
    
    #[test]
    fn test_inode_file() {
        let inode = InodeRaw::new_file(42, 0o644, 1000, 1000, 1234567890);
        
        assert_eq!(inode.ino, 42);
        assert!(inode.is_file());
        assert!(!inode.is_dir());
        assert_eq!(inode.uid, 1000);
        assert_eq!(inode.nlink, 1);
    }
    
    #[test]
    fn test_inode_dir() {
        let inode = InodeRaw::new_dir(43, 1, 0o755, 1000, 1000, 1234567890);
        
        assert!(inode.is_dir());
        assert_eq!(inode.nlink, 2);
        assert_eq!(inode.parent_ino, 1);
    }
    
    #[test]
    fn test_inode_checksum() {
        let mut inode = InodeRaw::new_file(42, 0o644, 1000, 1000, 1234567890);
        inode.update_checksum();
        
        assert!(inode.validate().is_ok());
        
        // Corrupt and verify
        inode.size = 9999;
        assert!(matches!(inode.validate(), Err(HfsError::ChecksumMismatch)));
    }
    
    #[test]
    fn test_inline_data() {
        let mut inode = InodeRaw::new_file(42, 0o644, 1000, 1000, 1234567890);
        
        let data = b"Hello, World!";
        assert!(inode.set_inline_data(data).is_ok());
        assert!(inode.has_inline_data());
        assert_eq!(inode.inline_data(), Some(&data[..]));
    }
    
    #[test]
    fn test_symlink() {
        let target = b"/path/to/target";
        let inode = InodeRaw::new_symlink(44, target, 1000, 1000, 1234567890);
        
        assert!(inode.is_symlink());
        assert_eq!(inode.symlink_target(), Some(&target[..]));
    }
}
