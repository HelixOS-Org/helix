//! HelixFS API Layer
//!
//! Provides the public interface for filesystem operations,
//! VFS integration, and mount management.
//!
//! # Modules
//! - `vfs`: Virtual File System interface
//! - `ops`: File and directory operations
//! - `mount`: Mount point management
//! - `handle`: File handles and descriptors

#![allow(dead_code)]

pub mod handle;
pub mod mount;
pub mod ops;
pub mod vfs;

// ============================================================================
// Constants
// ============================================================================

/// Maximum path length
pub const MAX_PATH_LEN: usize = 4096;

/// Maximum filename length
pub const MAX_NAME_LEN: usize = 255;

/// Maximum symbolic link depth
pub const MAX_SYMLINK_DEPTH: usize = 40;

/// Maximum open files per process
pub const MAX_OPEN_FILES: usize = 1024;

/// Root inode number
pub const ROOT_INODE: u64 = 2;

// ============================================================================
// File Type
// ============================================================================

/// File type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FileType {
    /// Unknown type
    Unknown     = 0,
    /// Regular file
    Regular     = 1,
    /// Directory
    Directory   = 2,
    /// Symbolic link
    Symlink     = 3,
    /// Block device
    BlockDevice = 4,
    /// Character device
    CharDevice  = 5,
    /// FIFO/named pipe
    Fifo        = 6,
    /// Socket
    Socket      = 7,
}

impl FileType {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Regular,
            2 => Self::Directory,
            3 => Self::Symlink,
            4 => Self::BlockDevice,
            5 => Self::CharDevice,
            6 => Self::Fifo,
            7 => Self::Socket,
            _ => Self::Unknown,
        }
    }

    /// From mode bits (S_IFMT)
    pub fn from_mode(mode: u32) -> Self {
        match mode & 0o170000 {
            0o100000 => Self::Regular,
            0o040000 => Self::Directory,
            0o120000 => Self::Symlink,
            0o060000 => Self::BlockDevice,
            0o020000 => Self::CharDevice,
            0o010000 => Self::Fifo,
            0o140000 => Self::Socket,
            _ => Self::Unknown,
        }
    }

    /// To mode bits
    pub fn to_mode(&self) -> u32 {
        match self {
            Self::Regular => 0o100000,
            Self::Directory => 0o040000,
            Self::Symlink => 0o120000,
            Self::BlockDevice => 0o060000,
            Self::CharDevice => 0o020000,
            Self::Fifo => 0o010000,
            Self::Socket => 0o140000,
            Self::Unknown => 0,
        }
    }

    /// Is regular file
    #[inline]
    pub fn is_file(&self) -> bool {
        *self == Self::Regular
    }

    /// Is directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        *self == Self::Directory
    }

    /// Is symbolic link
    #[inline]
    pub fn is_symlink(&self) -> bool {
        *self == Self::Symlink
    }
}

impl Default for FileType {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================================
// Open Flags
// ============================================================================

/// File open flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct OpenFlags(pub u32);

impl OpenFlags {
    // Access modes
    /// Read only
    pub const O_RDONLY: u32 = 0o0;
    /// Write only
    pub const O_WRONLY: u32 = 0o1;
    /// Read/write
    pub const O_RDWR: u32 = 0o2;

    // Creation flags
    /// Create if not exists
    pub const O_CREAT: u32 = 0o100;
    /// Exclusive creation
    pub const O_EXCL: u32 = 0o200;
    /// Don't follow symlinks
    pub const O_NOFOLLOW: u32 = 0o400000;
    /// Truncate to zero
    pub const O_TRUNC: u32 = 0o1000;

    // Operation flags
    /// Append mode
    pub const O_APPEND: u32 = 0o2000;
    /// Non-blocking
    pub const O_NONBLOCK: u32 = 0o4000;
    /// Synchronous I/O
    pub const O_SYNC: u32 = 0o4010000;
    /// Direct I/O (bypass cache)
    pub const O_DIRECT: u32 = 0o40000;

    // Directory flags
    /// Directory only
    pub const O_DIRECTORY: u32 = 0o200000;

    /// Create empty flags
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Check flag
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }

    /// Get access mode
    #[inline]
    pub fn access_mode(&self) -> u32 {
        self.0 & 0o3
    }

    /// Is read
    #[inline]
    pub fn is_read(&self) -> bool {
        let mode = self.access_mode();
        mode == Self::O_RDONLY || mode == Self::O_RDWR
    }

    /// Is write
    #[inline]
    pub fn is_write(&self) -> bool {
        let mode = self.access_mode();
        mode == Self::O_WRONLY || mode == Self::O_RDWR
    }

    /// Is create
    #[inline]
    pub fn is_create(&self) -> bool {
        self.has(Self::O_CREAT)
    }

    /// Is truncate
    #[inline]
    pub fn is_truncate(&self) -> bool {
        self.has(Self::O_TRUNC)
    }

    /// Is append
    #[inline]
    pub fn is_append(&self) -> bool {
        self.has(Self::O_APPEND)
    }

    /// Is direct I/O
    #[inline]
    pub fn is_direct(&self) -> bool {
        self.has(Self::O_DIRECT)
    }
}

// ============================================================================
// Seek Whence
// ============================================================================

/// Seek origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SeekWhence {
    /// From beginning
    Set  = 0,
    /// From current position
    Cur  = 1,
    /// From end
    End  = 2,
    /// To next data
    Data = 3,
    /// To next hole
    Hole = 4,
}

impl SeekWhence {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            0 => Self::Set,
            1 => Self::Cur,
            2 => Self::End,
            3 => Self::Data,
            4 => Self::Hole,
            _ => Self::Set,
        }
    }
}

// ============================================================================
// File Stat
// ============================================================================

/// File status information.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FileStat {
    /// Device ID
    pub st_dev: u64,
    /// Inode number
    pub st_ino: u64,
    /// File mode
    pub st_mode: u32,
    /// Number of hard links
    pub st_nlink: u32,
    /// User ID
    pub st_uid: u32,
    /// Group ID
    pub st_gid: u32,
    /// Device ID (if special file)
    pub st_rdev: u64,
    /// File size in bytes
    pub st_size: u64,
    /// Block size for I/O
    pub st_blksize: u32,
    /// Number of 512-byte blocks allocated
    pub st_blocks: u64,
    /// Access time (seconds)
    pub st_atime: u64,
    /// Access time (nanoseconds)
    pub st_atime_nsec: u32,
    /// Modification time (seconds)
    pub st_mtime: u64,
    /// Modification time (nanoseconds)
    pub st_mtime_nsec: u32,
    /// Status change time (seconds)
    pub st_ctime: u64,
    /// Status change time (nanoseconds)
    pub st_ctime_nsec: u32,
}

impl FileStat {
    /// Create new empty stat
    pub const fn new() -> Self {
        Self {
            st_dev: 0,
            st_ino: 0,
            st_mode: 0,
            st_nlink: 0,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            st_size: 0,
            st_blksize: 4096,
            st_blocks: 0,
            st_atime: 0,
            st_atime_nsec: 0,
            st_mtime: 0,
            st_mtime_nsec: 0,
            st_ctime: 0,
            st_ctime_nsec: 0,
        }
    }

    /// Get file type
    pub fn file_type(&self) -> FileType {
        FileType::from_mode(self.st_mode)
    }

    /// Is directory
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.file_type().is_dir()
    }

    /// Is regular file
    #[inline]
    pub fn is_file(&self) -> bool {
        self.file_type().is_file()
    }

    /// Is symlink
    #[inline]
    pub fn is_symlink(&self) -> bool {
        self.file_type().is_symlink()
    }
}

// ============================================================================
// Directory Entry
// ============================================================================

/// Directory entry.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DirEntry {
    /// Inode number
    pub d_ino: u64,
    /// Offset to next entry
    pub d_off: u64,
    /// Length of this entry
    pub d_reclen: u16,
    /// File type
    pub d_type: FileType,
    /// Name length
    pub d_namelen: u8,
    /// Name (null-terminated)
    pub d_name: [u8; MAX_NAME_LEN + 1],
}

impl DirEntry {
    /// Create new entry
    pub fn new(ino: u64, ftype: FileType, name: &[u8]) -> Self {
        let mut entry = Self {
            d_ino: ino,
            d_off: 0,
            d_reclen: 0,
            d_type: ftype,
            d_namelen: 0,
            d_name: [0; MAX_NAME_LEN + 1],
        };

        let len = core::cmp::min(name.len(), MAX_NAME_LEN);
        entry.d_name[..len].copy_from_slice(&name[..len]);
        entry.d_namelen = len as u8;
        entry.d_reclen = (24 + len + 1) as u16; // Align to 8 bytes
        entry.d_reclen = ((entry.d_reclen + 7) / 8) * 8;

        entry
    }

    /// Get name as slice
    pub fn name(&self) -> &[u8] {
        &self.d_name[..self.d_namelen as usize]
    }
}

impl Default for DirEntry {
    fn default() -> Self {
        Self {
            d_ino: 0,
            d_off: 0,
            d_reclen: 0,
            d_type: FileType::Unknown,
            d_namelen: 0,
            d_name: [0; MAX_NAME_LEN + 1],
        }
    }
}

// ============================================================================
// Filesystem Stats
// ============================================================================

/// Filesystem statistics (statvfs).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FsStats {
    /// Block size
    pub f_bsize: u64,
    /// Fragment size
    pub f_frsize: u64,
    /// Total blocks
    pub f_blocks: u64,
    /// Free blocks
    pub f_bfree: u64,
    /// Available blocks (non-root)
    pub f_bavail: u64,
    /// Total inodes
    pub f_files: u64,
    /// Free inodes
    pub f_ffree: u64,
    /// Available inodes (non-root)
    pub f_favail: u64,
    /// Filesystem ID
    pub f_fsid: u64,
    /// Mount flags
    pub f_flag: u64,
    /// Maximum filename length
    pub f_namemax: u64,
}

impl FsStats {
    /// Create new empty stats
    pub const fn new() -> Self {
        Self {
            f_bsize: 4096,
            f_frsize: 4096,
            f_blocks: 0,
            f_bfree: 0,
            f_bavail: 0,
            f_files: 0,
            f_ffree: 0,
            f_favail: 0,
            f_fsid: 0,
            f_flag: 0,
            f_namemax: MAX_NAME_LEN as u64,
        }
    }

    /// Total size in bytes
    pub fn total_size(&self) -> u64 {
        self.f_blocks * self.f_bsize
    }

    /// Free size in bytes
    pub fn free_size(&self) -> u64 {
        self.f_bfree * self.f_bsize
    }

    /// Used size in bytes
    pub fn used_size(&self) -> u64 {
        (self.f_blocks - self.f_bfree) * self.f_bsize
    }

    /// Usage percentage
    pub fn usage_percent(&self) -> f32 {
        if self.f_blocks == 0 {
            return 0.0;
        }
        ((self.f_blocks - self.f_bfree) as f32 / self.f_blocks as f32) * 100.0
    }
}

// ============================================================================
// Credentials
// ============================================================================

/// User credentials for permission checking.
#[derive(Clone, Copy, Debug, Default)]
pub struct Credentials {
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Supplementary groups
    pub groups: [u32; 16],
    /// Number of supplementary groups
    pub ngroups: u8,
    /// Is privileged (root)
    pub privileged: bool,
}

impl Credentials {
    /// Root credentials
    pub const fn root() -> Self {
        Self {
            uid: 0,
            gid: 0,
            groups: [0; 16],
            ngroups: 0,
            privileged: true,
        }
    }

    /// User credentials
    pub fn user(uid: u32, gid: u32) -> Self {
        Self {
            uid,
            gid,
            groups: [gid, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            ngroups: 1,
            privileged: uid == 0,
        }
    }

    /// Check if in group
    pub fn is_in_group(&self, gid: u32) -> bool {
        if self.gid == gid {
            return true;
        }

        for i in 0..self.ngroups as usize {
            if self.groups[i] == gid {
                return true;
            }
        }

        false
    }

    /// Check permission
    pub fn check_permission(&self, mode: u32, owner_uid: u32, owner_gid: u32, want: u32) -> bool {
        if self.privileged {
            return true;
        }

        let perm = if self.uid == owner_uid {
            (mode >> 6) & 0o7
        } else if self.is_in_group(owner_gid) {
            (mode >> 3) & 0o7
        } else {
            mode & 0o7
        };

        (perm & want) == want
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type() {
        assert_eq!(FileType::from_mode(0o100644), FileType::Regular);
        assert_eq!(FileType::from_mode(0o040755), FileType::Directory);
        assert_eq!(FileType::from_mode(0o120777), FileType::Symlink);
    }

    #[test]
    fn test_open_flags() {
        let flags = OpenFlags(OpenFlags::O_RDWR | OpenFlags::O_CREAT);

        assert!(flags.is_read());
        assert!(flags.is_write());
        assert!(flags.is_create());
        assert!(!flags.is_append());
    }

    #[test]
    fn test_file_stat() {
        let mut stat = FileStat::new();
        stat.st_mode = 0o100644;
        stat.st_size = 1024;

        assert!(stat.is_file());
        assert!(!stat.is_dir());
    }

    #[test]
    fn test_dir_entry() {
        let entry = DirEntry::new(100, FileType::Regular, b"test.txt");

        assert_eq!(entry.d_ino, 100);
        assert_eq!(entry.d_type, FileType::Regular);
        assert_eq!(entry.name(), b"test.txt");
    }

    #[test]
    fn test_fs_stats() {
        let mut stats = FsStats::new();
        stats.f_blocks = 1000;
        stats.f_bfree = 500;

        assert_eq!(stats.usage_percent(), 50.0);
    }

    #[test]
    fn test_credentials() {
        let root = Credentials::root();
        assert!(root.privileged);
        assert!(root.check_permission(0o600, 1000, 1000, 0o4));

        let user = Credentials::user(1000, 1000);
        assert!(user.check_permission(0o644, 1000, 1000, 0o6)); // Owner
        assert!(!user.check_permission(0o600, 1001, 1001, 0o4)); // Other
    }
}
