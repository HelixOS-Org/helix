//! Virtual File System Interface
//!
//! Defines the VFS traits that HelixFS implements, allowing
//! integration with kernel VFS layers.

use crate::core::error::{HfsError, HfsResult};
use super::{FileType, FileStat, FsStats, DirEntry, Credentials, OpenFlags, SeekWhence, MAX_PATH_LEN};

// ============================================================================
// VFS Operations Trait
// ============================================================================

/// Main filesystem operations trait.
///
/// This trait defines all operations a filesystem must implement
/// to be usable by the VFS layer.
pub trait FileSystemOps {
    // ========================================================================
    // Filesystem-level operations
    // ========================================================================
    
    /// Mount the filesystem
    fn mount(&mut self, device: u64, flags: u64) -> HfsResult<()>;
    
    /// Unmount the filesystem
    fn unmount(&mut self) -> HfsResult<()>;
    
    /// Sync all pending data
    fn sync(&mut self) -> HfsResult<()>;
    
    /// Get filesystem statistics
    fn statfs(&self) -> HfsResult<FsStats>;
    
    // ========================================================================
    // Inode operations
    // ========================================================================
    
    /// Lookup inode by name in parent directory
    fn lookup(&self, parent: u64, name: &[u8], cred: &Credentials) -> HfsResult<u64>;
    
    /// Get inode attributes
    fn getattr(&self, ino: u64) -> HfsResult<FileStat>;
    
    /// Set inode attributes
    fn setattr(&mut self, ino: u64, attr: &SetAttr, cred: &Credentials) -> HfsResult<FileStat>;
    
    /// Read symbolic link
    fn readlink(&self, ino: u64) -> HfsResult<PathBuf>;
    
    // ========================================================================
    // File operations
    // ========================================================================
    
    /// Create regular file
    fn create(
        &mut self, 
        parent: u64, 
        name: &[u8], 
        mode: u32, 
        flags: OpenFlags,
        cred: &Credentials
    ) -> HfsResult<(u64, FileHandle)>;
    
    /// Open existing file
    fn open(&mut self, ino: u64, flags: OpenFlags, cred: &Credentials) -> HfsResult<FileHandle>;
    
    /// Read from file
    fn read(
        &self, 
        ino: u64, 
        handle: &FileHandle,
        offset: u64, 
        buf: &mut [u8]
    ) -> HfsResult<usize>;
    
    /// Write to file
    fn write(
        &mut self, 
        ino: u64, 
        handle: &FileHandle,
        offset: u64, 
        buf: &[u8]
    ) -> HfsResult<usize>;
    
    /// Flush file data
    fn flush(&mut self, ino: u64, handle: &FileHandle) -> HfsResult<()>;
    
    /// Release file handle
    fn release(&mut self, ino: u64, handle: FileHandle) -> HfsResult<()>;
    
    /// Sync file data
    fn fsync(&mut self, ino: u64, handle: &FileHandle, datasync: bool) -> HfsResult<()>;
    
    // ========================================================================
    // Directory operations
    // ========================================================================
    
    /// Create directory
    fn mkdir(&mut self, parent: u64, name: &[u8], mode: u32, cred: &Credentials) -> HfsResult<u64>;
    
    /// Remove directory
    fn rmdir(&mut self, parent: u64, name: &[u8], cred: &Credentials) -> HfsResult<()>;
    
    /// Open directory
    fn opendir(&mut self, ino: u64, cred: &Credentials) -> HfsResult<DirHandle>;
    
    /// Read directory entries
    fn readdir(
        &self, 
        ino: u64, 
        handle: &DirHandle,
        offset: u64,
        buf: &mut [DirEntry]
    ) -> HfsResult<usize>;
    
    /// Release directory handle
    fn releasedir(&mut self, ino: u64, handle: DirHandle) -> HfsResult<()>;
    
    // ========================================================================
    // Link operations
    // ========================================================================
    
    /// Create hard link
    fn link(
        &mut self, 
        ino: u64, 
        newparent: u64, 
        newname: &[u8], 
        cred: &Credentials
    ) -> HfsResult<()>;
    
    /// Remove file (unlink)
    fn unlink(&mut self, parent: u64, name: &[u8], cred: &Credentials) -> HfsResult<()>;
    
    /// Create symbolic link
    fn symlink(
        &mut self, 
        parent: u64, 
        name: &[u8], 
        target: &[u8], 
        cred: &Credentials
    ) -> HfsResult<u64>;
    
    /// Rename file/directory
    fn rename(
        &mut self, 
        oldparent: u64, 
        oldname: &[u8], 
        newparent: u64, 
        newname: &[u8], 
        flags: u32,
        cred: &Credentials
    ) -> HfsResult<()>;
    
    // ========================================================================
    // Special operations
    // ========================================================================
    
    /// Create special file (device, fifo, socket)
    fn mknod(
        &mut self, 
        parent: u64, 
        name: &[u8], 
        mode: u32, 
        rdev: u64,
        cred: &Credentials
    ) -> HfsResult<u64>;
    
    /// Allocate space
    fn fallocate(
        &mut self, 
        ino: u64, 
        mode: u32, 
        offset: u64, 
        length: u64
    ) -> HfsResult<()>;
}

// ============================================================================
// Set Attributes
// ============================================================================

/// Attributes to set on inode.
#[derive(Clone, Copy, Debug, Default)]
pub struct SetAttr {
    /// Valid fields mask
    pub valid: SetAttrValid,
    /// New mode
    pub mode: u32,
    /// New user ID
    pub uid: u32,
    /// New group ID
    pub gid: u32,
    /// New size
    pub size: u64,
    /// New access time
    pub atime: u64,
    /// New modification time
    pub mtime: u64,
    /// Set atime to now
    pub atime_now: bool,
    /// Set mtime to now
    pub mtime_now: bool,
}

/// Valid fields for setattr.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct SetAttrValid(pub u32);

impl SetAttrValid {
    pub const MODE: u32 = 1 << 0;
    pub const UID: u32 = 1 << 1;
    pub const GID: u32 = 1 << 2;
    pub const SIZE: u32 = 1 << 3;
    pub const ATIME: u32 = 1 << 4;
    pub const MTIME: u32 = 1 << 5;
    pub const ATIME_NOW: u32 = 1 << 6;
    pub const MTIME_NOW: u32 = 1 << 7;
    
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }
}

// ============================================================================
// File Handle
// ============================================================================

/// File handle for open files.
#[derive(Clone, Copy, Debug)]
pub struct FileHandle {
    /// Handle ID
    pub id: u64,
    /// Inode number
    pub ino: u64,
    /// Open flags
    pub flags: OpenFlags,
    /// Current position
    pub pos: u64,
    /// Generation for validation
    pub generation: u32,
}

impl FileHandle {
    /// Create new file handle
    pub fn new(id: u64, ino: u64, flags: OpenFlags) -> Self {
        Self {
            id,
            ino,
            flags,
            pos: 0,
            generation: 0,
        }
    }
    
    /// Is readable
    #[inline]
    pub fn is_readable(&self) -> bool {
        self.flags.is_read()
    }
    
    /// Is writable
    #[inline]
    pub fn is_writable(&self) -> bool {
        self.flags.is_write()
    }
}

// ============================================================================
// Directory Handle
// ============================================================================

/// Directory handle for open directories.
#[derive(Clone, Copy, Debug)]
pub struct DirHandle {
    /// Handle ID
    pub id: u64,
    /// Inode number
    pub ino: u64,
    /// Current position
    pub pos: u64,
    /// Generation for validation
    pub generation: u32,
}

impl DirHandle {
    /// Create new directory handle
    pub fn new(id: u64, ino: u64) -> Self {
        Self {
            id,
            ino,
            pos: 0,
            generation: 0,
        }
    }
}

// ============================================================================
// Path Buffer
// ============================================================================

/// Fixed-size path buffer.
#[derive(Clone, Debug)]
pub struct PathBuf {
    /// Path data
    pub data: [u8; MAX_PATH_LEN],
    /// Length
    pub len: usize,
}

impl PathBuf {
    /// Create empty path
    pub const fn new() -> Self {
        Self {
            data: [0; MAX_PATH_LEN],
            len: 0,
        }
    }
    
    /// Create from slice
    pub fn from_slice(s: &[u8]) -> Self {
        let mut path = Self::new();
        let len = core::cmp::min(s.len(), MAX_PATH_LEN);
        path.data[..len].copy_from_slice(&s[..len]);
        path.len = len;
        path
    }
    
    /// Get as slice
    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }
    
    /// Push component
    pub fn push(&mut self, component: &[u8]) -> bool {
        if self.len + 1 + component.len() > MAX_PATH_LEN {
            return false;
        }
        
        if self.len > 0 && self.data[self.len - 1] != b'/' {
            self.data[self.len] = b'/';
            self.len += 1;
        }
        
        self.data[self.len..self.len + component.len()].copy_from_slice(component);
        self.len += component.len();
        
        true
    }
    
    /// Get parent
    pub fn parent(&self) -> Option<PathBuf> {
        if self.len == 0 {
            return None;
        }
        
        let mut end = self.len;
        
        // Skip trailing slashes
        while end > 0 && self.data[end - 1] == b'/' {
            end -= 1;
        }
        
        // Find last slash
        while end > 0 && self.data[end - 1] != b'/' {
            end -= 1;
        }
        
        if end == 0 {
            return None;
        }
        
        let mut parent = Self::new();
        parent.data[..end].copy_from_slice(&self.data[..end]);
        parent.len = end;
        
        Some(parent)
    }
    
    /// Get filename
    pub fn filename(&self) -> &[u8] {
        if self.len == 0 {
            return &[];
        }
        
        let mut start = self.len;
        
        // Skip trailing slashes
        let mut end = self.len;
        while end > 0 && self.data[end - 1] == b'/' {
            end -= 1;
        }
        
        // Find last slash
        start = end;
        while start > 0 && self.data[start - 1] != b'/' {
            start -= 1;
        }
        
        &self.data[start..end]
    }
    
    /// Is absolute
    pub fn is_absolute(&self) -> bool {
        self.len > 0 && self.data[0] == b'/'
    }
}

impl Default for PathBuf {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// VFS Node
// ============================================================================

/// VFS inode representation.
#[derive(Clone, Copy, Debug)]
pub struct VfsInode {
    /// Inode number
    pub ino: u64,
    /// Generation number
    pub generation: u64,
    /// File type
    pub ftype: FileType,
    /// Mode (permissions)
    pub mode: u32,
    /// Number of links
    pub nlink: u32,
    /// Owner UID
    pub uid: u32,
    /// Owner GID
    pub gid: u32,
    /// Size in bytes
    pub size: u64,
    /// Blocks allocated
    pub blocks: u64,
    /// Access time (seconds)
    pub atime: u64,
    /// Modification time (seconds)
    pub mtime: u64,
    /// Change time (seconds)
    pub ctime: u64,
    /// Creation time (seconds)
    pub crtime: u64,
}

impl VfsInode {
    /// Create new VFS inode
    pub fn new(ino: u64, ftype: FileType) -> Self {
        Self {
            ino,
            generation: 0,
            ftype,
            mode: 0o644,
            nlink: 1,
            uid: 0,
            gid: 0,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            crtime: 0,
        }
    }
    
    /// Convert to FileStat
    pub fn to_stat(&self, dev: u64) -> FileStat {
        FileStat {
            st_dev: dev,
            st_ino: self.ino,
            st_mode: self.ftype.to_mode() | (self.mode & 0o7777),
            st_nlink: self.nlink,
            st_uid: self.uid,
            st_gid: self.gid,
            st_rdev: 0,
            st_size: self.size,
            st_blksize: 4096,
            st_blocks: self.blocks * 8, // 512-byte blocks
            st_atime: self.atime,
            st_atime_nsec: 0,
            st_mtime: self.mtime,
            st_mtime_nsec: 0,
            st_ctime: self.ctime,
            st_ctime_nsec: 0,
        }
    }
}

impl Default for VfsInode {
    fn default() -> Self {
        Self::new(0, FileType::Unknown)
    }
}

// ============================================================================
// Extended Attributes
// ============================================================================

/// Extended attribute namespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum XattrNamespace {
    /// User namespace
    User = 0,
    /// System namespace
    System = 1,
    /// Security namespace
    Security = 2,
    /// Trusted namespace
    Trusted = 3,
}

impl XattrNamespace {
    /// From prefix string
    pub fn from_prefix(name: &[u8]) -> Option<Self> {
        if name.starts_with(b"user.") {
            Some(Self::User)
        } else if name.starts_with(b"system.") {
            Some(Self::System)
        } else if name.starts_with(b"security.") {
            Some(Self::Security)
        } else if name.starts_with(b"trusted.") {
            Some(Self::Trusted)
        } else {
            None
        }
    }
    
    /// Get prefix
    pub fn prefix(&self) -> &'static [u8] {
        match self {
            Self::User => b"user.",
            Self::System => b"system.",
            Self::Security => b"security.",
            Self::Trusted => b"trusted.",
        }
    }
}

/// Extended attribute operations trait.
pub trait XattrOps {
    /// Get extended attribute
    fn getxattr(&self, ino: u64, name: &[u8], buf: &mut [u8]) -> HfsResult<usize>;
    
    /// Set extended attribute
    fn setxattr(&mut self, ino: u64, name: &[u8], value: &[u8], flags: u32) -> HfsResult<()>;
    
    /// List extended attributes
    fn listxattr(&self, ino: u64, buf: &mut [u8]) -> HfsResult<usize>;
    
    /// Remove extended attribute
    fn removexattr(&mut self, ino: u64, name: &[u8]) -> HfsResult<()>;
}

// ============================================================================
// Lock Operations
// ============================================================================

/// File lock type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum LockType {
    /// Read lock (shared)
    Read = 0,
    /// Write lock (exclusive)
    Write = 1,
    /// Unlock
    Unlock = 2,
}

/// File lock structure.
#[derive(Clone, Copy, Debug)]
pub struct FileLock {
    /// Lock type
    pub ltype: LockType,
    /// Start offset
    pub start: u64,
    /// Length (0 = to end of file)
    pub len: u64,
    /// Process ID
    pub pid: u32,
}

impl FileLock {
    /// Create read lock
    pub fn read(start: u64, len: u64, pid: u32) -> Self {
        Self {
            ltype: LockType::Read,
            start,
            len,
            pid,
        }
    }
    
    /// Create write lock
    pub fn write(start: u64, len: u64, pid: u32) -> Self {
        Self {
            ltype: LockType::Write,
            start,
            len,
            pid,
        }
    }
    
    /// Create unlock
    pub fn unlock(start: u64, len: u64, pid: u32) -> Self {
        Self {
            ltype: LockType::Unlock,
            start,
            len,
            pid,
        }
    }
    
    /// Check if overlaps with another lock
    pub fn overlaps(&self, other: &FileLock) -> bool {
        let self_end = if self.len == 0 { u64::MAX } else { self.start + self.len };
        let other_end = if other.len == 0 { u64::MAX } else { other.start + other.len };
        
        self.start < other_end && other.start < self_end
    }
    
    /// Check if conflicts with another lock
    pub fn conflicts(&self, other: &FileLock) -> bool {
        if !self.overlaps(other) {
            return false;
        }
        
        // Same process doesn't conflict
        if self.pid == other.pid {
            return false;
        }
        
        // Write lock conflicts with any lock
        self.ltype == LockType::Write || other.ltype == LockType::Write
    }
}

/// File locking operations trait.
pub trait LockOps {
    /// Get lock status
    fn getlk(&self, ino: u64, lock: &mut FileLock) -> HfsResult<()>;
    
    /// Set lock (blocking)
    fn setlk(&mut self, ino: u64, lock: &FileLock, block: bool) -> HfsResult<()>;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_path_buf() {
        let mut path = PathBuf::from_slice(b"/home/user");
        assert!(path.is_absolute());
        assert_eq!(path.filename(), b"user");
        
        path.push(b"documents");
        assert_eq!(path.as_slice(), b"/home/user/documents");
        
        let parent = path.parent().unwrap();
        assert_eq!(parent.as_slice(), b"/home/user/");
    }
    
    #[test]
    fn test_file_handle() {
        let handle = FileHandle::new(1, 100, OpenFlags(OpenFlags::O_RDWR));
        
        assert!(handle.is_readable());
        assert!(handle.is_writable());
    }
    
    #[test]
    fn test_vfs_inode() {
        let inode = VfsInode::new(100, FileType::Regular);
        let stat = inode.to_stat(1);
        
        assert_eq!(stat.st_ino, 100);
        assert!(stat.is_file());
    }
    
    #[test]
    fn test_file_lock() {
        let lock1 = FileLock::read(0, 100, 1);
        let lock2 = FileLock::read(50, 100, 2);
        let lock3 = FileLock::write(200, 50, 3);
        
        assert!(lock1.overlaps(&lock2));
        assert!(!lock1.conflicts(&lock2)); // Both read
        assert!(!lock1.overlaps(&lock3));
    }
    
    #[test]
    fn test_xattr_namespace() {
        assert_eq!(XattrNamespace::from_prefix(b"user.test"), Some(XattrNamespace::User));
        assert_eq!(XattrNamespace::from_prefix(b"security.selinux"), Some(XattrNamespace::Security));
        assert_eq!(XattrNamespace::from_prefix(b"invalid"), None);
    }
}
