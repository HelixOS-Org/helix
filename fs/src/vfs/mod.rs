//! HelixFS VFS Integration Layer
//!
//! Provides integration with kernel VFS subsystems,
//! inode management, and dentry caching.
//!
//! # Components
//! - `inode`: Inode operations and management
//! - `dentry`: Directory entry cache
//! - `super`: Superblock operations
//! - `namespace`: Namespace and mount management

#![allow(dead_code)]

pub mod inode;
pub mod dentry;
pub mod superblock;
pub mod namespace;

use crate::core::error::HfsResult;
use crate::api::Credentials;
use crate::api::{FileType, FileStat};

// Re-export FileType and FileStat from api for VFS use
pub use crate::api::FileType as VfsFileType;
pub use crate::api::FileStat as VfsFileStat;

// ============================================================================
// Constants
// ============================================================================

/// Maximum inode number
pub const MAX_INODE_NR: u64 = u64::MAX - 1;

/// Root inode number
pub const ROOT_INO: u64 = 2;

/// First regular inode
pub const FIRST_INO: u64 = 11;

/// Invalid inode number
pub const INVALID_INO: u64 = 0;

/// Maximum open inodes
pub const MAX_INODES: usize = 65536;

/// Maximum dentries cached
pub const MAX_DENTRIES: usize = 131072;

// ============================================================================
// Inode Flags
// ============================================================================

/// VFS inode flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct VfsInodeFlags(pub u32);

impl VfsInodeFlags {
    /// Inode is dirty
    pub const I_DIRTY: u32 = 1 << 0;
    /// Inode is being synced
    pub const I_SYNC: u32 = 1 << 1;
    /// Inode is new (not yet on disk)
    pub const I_NEW: u32 = 1 << 2;
    /// Inode is being deleted
    pub const I_FREEING: u32 = 1 << 3;
    /// Inode has dirty pages
    pub const I_DIRTY_PAGES: u32 = 1 << 4;
    /// Inode has dirty metadata
    pub const I_DIRTY_META: u32 = 1 << 5;
    /// Inode is locked
    pub const I_LOCK: u32 = 1 << 6;
    /// Inode is referenced
    pub const I_REFERENCED: u32 = 1 << 7;
    /// Inode is immutable
    pub const I_IMMUTABLE: u32 = 1 << 8;
    /// Inode is append-only
    pub const I_APPEND: u32 = 1 << 9;
    
    /// Create empty flags
    pub const fn empty() -> Self {
        Self(0)
    }
    
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
    
    /// Is dirty (any dirty flag)
    #[inline]
    pub fn is_dirty(&self) -> bool {
        self.has(Self::I_DIRTY | Self::I_DIRTY_PAGES | Self::I_DIRTY_META)
    }
}

// ============================================================================
// VFS Inode
// ============================================================================

/// Core VFS inode structure.
#[derive(Clone, Copy, Debug)]
pub struct VfsInode {
    /// Inode number
    pub ino: u64,
    /// Device ID
    pub dev: u64,
    /// File type
    pub ftype: FileType,
    /// Mode (permissions)
    pub mode: u32,
    /// Number of hard links
    pub nlink: u32,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// Device number (for special files)
    pub rdev: u64,
    /// Size in bytes
    pub size: u64,
    /// Block size for I/O
    pub blksize: u32,
    /// Blocks allocated
    pub blocks: u64,
    /// Access time
    pub atime: Timespec,
    /// Modification time
    pub mtime: Timespec,
    /// Change time
    pub ctime: Timespec,
    /// Creation time
    pub crtime: Timespec,
    /// Generation number
    pub generation: u64,
    /// Flags
    pub flags: VfsInodeFlags,
    /// Reference count
    pub refcount: u32,
}

impl VfsInode {
    /// Create new inode
    pub fn new(ino: u64, ftype: FileType) -> Self {
        Self {
            ino,
            dev: 0,
            ftype,
            mode: 0o644,
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            size: 0,
            blksize: 4096,
            blocks: 0,
            atime: Timespec::zero(),
            mtime: Timespec::zero(),
            ctime: Timespec::zero(),
            crtime: Timespec::zero(),
            generation: 0,
            flags: VfsInodeFlags::empty(),
            refcount: 1,
        }
    }
    
    /// Create directory inode
    pub fn new_dir(ino: u64, mode: u32) -> Self {
        let mut inode = Self::new(ino, FileType::Directory);
        inode.mode = mode & 0o7777;
        inode.size = 4096; // One block for empty dir
        inode.nlink = 2; // . and parent
        inode
    }
    
    /// Create regular file inode
    pub fn new_file(ino: u64, mode: u32) -> Self {
        let mut inode = Self::new(ino, FileType::Regular);
        inode.mode = mode & 0o7777;
        inode
    }
    
    /// Create symlink inode
    pub fn new_symlink(ino: u64) -> Self {
        let mut inode = Self::new(ino, FileType::Symlink);
        inode.mode = 0o777;
        inode
    }
    
    /// Convert to FileStat
    pub fn to_stat(&self) -> FileStat {
        FileStat {
            st_dev: self.dev,
            st_ino: self.ino,
            st_mode: self.ftype.to_mode() | (self.mode & 0o7777),
            st_nlink: self.nlink,
            st_uid: self.uid,
            st_gid: self.gid,
            st_rdev: self.rdev,
            st_size: self.size,
            st_blksize: self.blksize,
            st_blocks: self.blocks * (self.blksize as u64 / 512),
            st_atime: self.atime.sec as u64,
            st_atime_nsec: self.atime.nsec,
            st_mtime: self.mtime.sec as u64,
            st_mtime_nsec: self.mtime.nsec,
            st_ctime: self.ctime.sec as u64,
            st_ctime_nsec: self.ctime.nsec,
        }
    }
    
    /// Is directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.ftype == FileType::Directory
    }
    
    /// Is regular file
    #[inline]
    pub fn is_file(&self) -> bool {
        self.ftype == FileType::Regular
    }
    
    /// Is symlink
    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.ftype == FileType::Symlink
    }
    
    /// Mark dirty
    pub fn mark_dirty(&mut self) {
        self.flags.set(VfsInodeFlags::I_DIRTY);
    }
    
    /// Clear dirty
    pub fn clear_dirty(&mut self) {
        self.flags.clear(VfsInodeFlags::I_DIRTY | VfsInodeFlags::I_DIRTY_PAGES | VfsInodeFlags::I_DIRTY_META);
    }
    
    /// Check permission
    pub fn check_permission(&self, cred: &Credentials, want: u32) -> bool {
        cred.check_permission(self.mode, self.uid, self.gid, want)
    }
    
    /// Update timestamps
    pub fn touch(&mut self, now: Timespec) {
        self.atime = now;
        self.mtime = now;
        self.ctime = now;
    }
}

impl Default for VfsInode {
    fn default() -> Self {
        Self::new(INVALID_INO, FileType::Unknown)
    }
}

// ============================================================================
// Timespec
// ============================================================================

/// Time specification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Timespec {
    /// Seconds
    pub sec: i64,
    /// Nanoseconds
    pub nsec: u32,
}

impl Timespec {
    /// Zero time
    pub const fn zero() -> Self {
        Self { sec: 0, nsec: 0 }
    }
    
    /// Create from seconds
    pub const fn from_secs(sec: i64) -> Self {
        Self { sec, nsec: 0 }
    }
    
    /// Create from seconds and nanoseconds
    pub const fn new(sec: i64, nsec: u32) -> Self {
        Self { sec, nsec }
    }
    
    /// Add duration
    pub fn add(&self, sec: i64, nsec: u32) -> Self {
        let mut result = *self;
        result.nsec += nsec;
        result.sec += sec + (result.nsec / 1_000_000_000) as i64;
        result.nsec %= 1_000_000_000;
        result
    }
    
    /// Compare times
    pub fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        match self.sec.cmp(&other.sec) {
            core::cmp::Ordering::Equal => self.nsec.cmp(&other.nsec),
            ord => ord,
        }
    }
}

// ============================================================================
// VFS Operations
// ============================================================================

/// VFS superblock operations.
pub trait VfsSuperOps {
    /// Allocate new inode
    fn alloc_inode(&mut self) -> HfsResult<u64>;
    
    /// Free inode
    fn free_inode(&mut self, ino: u64) -> HfsResult<()>;
    
    /// Read inode from disk
    fn read_inode(&self, ino: u64) -> HfsResult<VfsInode>;
    
    /// Write inode to disk
    fn write_inode(&mut self, inode: &VfsInode) -> HfsResult<()>;
    
    /// Delete inode
    fn delete_inode(&mut self, ino: u64) -> HfsResult<()>;
    
    /// Sync filesystem
    fn sync_fs(&mut self) -> HfsResult<()>;
    
    /// Get filesystem stats
    fn statfs(&self) -> HfsResult<crate::api::FsStats>;
}

/// VFS inode operations.
pub trait VfsInodeOps {
    /// Create file in directory
    fn create(&mut self, parent: &VfsInode, name: &[u8], mode: u32, cred: &Credentials) -> HfsResult<VfsInode>;
    
    /// Lookup name in directory
    fn lookup(&self, dir: &VfsInode, name: &[u8]) -> HfsResult<VfsInode>;
    
    /// Create hard link
    fn link(&mut self, old: &VfsInode, dir: &VfsInode, name: &[u8]) -> HfsResult<()>;
    
    /// Remove file
    fn unlink(&mut self, dir: &VfsInode, name: &[u8]) -> HfsResult<()>;
    
    /// Create symbolic link
    fn symlink(&mut self, dir: &VfsInode, name: &[u8], target: &[u8], cred: &Credentials) -> HfsResult<VfsInode>;
    
    /// Create directory
    fn mkdir(&mut self, parent: &VfsInode, name: &[u8], mode: u32, cred: &Credentials) -> HfsResult<VfsInode>;
    
    /// Remove directory
    fn rmdir(&mut self, parent: &VfsInode, name: &[u8]) -> HfsResult<()>;
    
    /// Rename
    fn rename(
        &mut self,
        old_dir: &VfsInode,
        old_name: &[u8],
        new_dir: &VfsInode,
        new_name: &[u8],
    ) -> HfsResult<()>;
    
    /// Set attributes
    fn setattr(&mut self, inode: &mut VfsInode, attr: &SetAttr) -> HfsResult<()>;
    
    /// Get attributes
    fn getattr(&self, inode: &VfsInode) -> HfsResult<FileStat>;
}

/// Attribute to set.
#[derive(Clone, Copy, Debug, Default)]
pub struct SetAttr {
    /// Valid mask
    pub valid: u32,
    /// Mode
    pub mode: u32,
    /// UID
    pub uid: u32,
    /// GID
    pub gid: u32,
    /// Size
    pub size: u64,
    /// Access time
    pub atime: Timespec,
    /// Modification time
    pub mtime: Timespec,
}

impl SetAttr {
    /// Mode is valid
    pub const ATTR_MODE: u32 = 1 << 0;
    /// UID is valid
    pub const ATTR_UID: u32 = 1 << 1;
    /// GID is valid
    pub const ATTR_GID: u32 = 1 << 2;
    /// Size is valid
    pub const ATTR_SIZE: u32 = 1 << 3;
    /// Atime is valid
    pub const ATTR_ATIME: u32 = 1 << 4;
    /// Mtime is valid
    pub const ATTR_MTIME: u32 = 1 << 5;
    
    /// Check if field is valid
    pub fn has(&self, flag: u32) -> bool {
        self.valid & flag != 0
    }
}

/// VFS file operations.
pub trait VfsFileOps {
    /// Read from file
    fn read(&self, inode: &VfsInode, offset: u64, buf: &mut [u8]) -> HfsResult<usize>;
    
    /// Write to file
    fn write(&mut self, inode: &VfsInode, offset: u64, buf: &[u8]) -> HfsResult<usize>;
    
    /// Sync file data
    fn fsync(&mut self, inode: &VfsInode, datasync: bool) -> HfsResult<()>;
    
    /// Read directory
    fn readdir(&self, dir: &VfsInode, offset: u64, callback: &mut dyn FnMut(&[u8], u64, FileType) -> bool) -> HfsResult<()>;
    
    /// Read link target
    fn readlink(&self, inode: &VfsInode, buf: &mut [u8]) -> HfsResult<usize>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vfs_inode() {
        let inode = VfsInode::new_file(100, 0o644);
        
        assert_eq!(inode.ino, 100);
        assert!(inode.is_file());
        assert_eq!(inode.mode, 0o644);
    }
    
    #[test]
    fn test_vfs_inode_dir() {
        let inode = VfsInode::new_dir(100, 0o755);
        
        assert!(inode.is_dir());
        assert_eq!(inode.nlink, 2);
    }
    
    #[test]
    fn test_vfs_inode_stat() {
        let mut inode = VfsInode::new_file(100, 0o644);
        inode.size = 1024;
        inode.uid = 1000;
        inode.gid = 1000;
        
        let stat = inode.to_stat();
        assert_eq!(stat.st_ino, 100);
        assert_eq!(stat.st_size, 1024);
        assert!(stat.is_file());
    }
    
    #[test]
    fn test_vfs_inode_flags() {
        let mut inode = VfsInode::new_file(100, 0o644);
        
        assert!(!inode.flags.is_dirty());
        
        inode.mark_dirty();
        assert!(inode.flags.is_dirty());
        
        inode.clear_dirty();
        assert!(!inode.flags.is_dirty());
    }
    
    #[test]
    fn test_timespec() {
        let t1 = Timespec::from_secs(100);
        let t2 = t1.add(10, 500_000_000);
        
        assert_eq!(t2.sec, 110);
        assert_eq!(t2.nsec, 500_000_000);
        
        let t3 = t2.add(0, 600_000_000);
        assert_eq!(t3.sec, 111);
        assert_eq!(t3.nsec, 100_000_000);
    }
    
    #[test]
    fn test_permission() {
        let inode = VfsInode::new_file(100, 0o644);
        
        let root = Credentials::root();
        assert!(inode.check_permission(&root, 0o7)); // Root can do anything
        
        let owner = Credentials::user(0, 0);
        assert!(inode.check_permission(&owner, 0o6)); // Owner: rw
        
        let other = Credentials::user(1000, 1000);
        assert!(inode.check_permission(&other, 0o4)); // Other: r
        assert!(!inode.check_permission(&other, 0o2)); // Other: no write
    }
}
