//! # Filesystem Subsystem
//!
//! Virtual Filesystem (VFS) and filesystem driver initialization.
//! Late phase subsystem for filesystem support.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

// =============================================================================
// VFS TYPES
// =============================================================================

/// Inode number
pub type InodeNum = u64;

/// File descriptor
pub type FileDescriptor = i32;

/// Device number (major:minor)
pub type DeviceNum = u64;

/// File mode (permissions + type)
pub type FileMode = u32;

/// File offset
pub type FileOffset = i64;

// =============================================================================
// FILE TYPES
// =============================================================================

/// File type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
    CharDevice,
    BlockDevice,
    Fifo,
    Socket,
    Unknown,
}

impl Default for FileType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl From<u32> for FileType {
    fn from(mode: u32) -> Self {
        match (mode >> 12) & 0xF {
            0x1 => FileType::Fifo,
            0x2 => FileType::CharDevice,
            0x4 => FileType::Directory,
            0x6 => FileType::BlockDevice,
            0x8 => FileType::Regular,
            0xA => FileType::Symlink,
            0xC => FileType::Socket,
            _ => FileType::Unknown,
        }
    }
}

/// File permissions
#[derive(Debug, Clone, Copy, Default)]
pub struct FilePermissions {
    pub owner_read: bool,
    pub owner_write: bool,
    pub owner_exec: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub group_exec: bool,
    pub other_read: bool,
    pub other_write: bool,
    pub other_exec: bool,
    pub setuid: bool,
    pub setgid: bool,
    pub sticky: bool,
}

impl From<u32> for FilePermissions {
    fn from(mode: u32) -> Self {
        Self {
            owner_read: (mode & 0o400) != 0,
            owner_write: (mode & 0o200) != 0,
            owner_exec: (mode & 0o100) != 0,
            group_read: (mode & 0o040) != 0,
            group_write: (mode & 0o020) != 0,
            group_exec: (mode & 0o010) != 0,
            other_read: (mode & 0o004) != 0,
            other_write: (mode & 0o002) != 0,
            other_exec: (mode & 0o001) != 0,
            setuid: (mode & 0o4000) != 0,
            setgid: (mode & 0o2000) != 0,
            sticky: (mode & 0o1000) != 0,
        }
    }
}

/// Open flags
#[derive(Debug, Clone, Copy, Default)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub truncate: bool,
    pub append: bool,
    pub exclusive: bool,
    pub nonblock: bool,
    pub directory: bool,
    pub sync: bool,
}

impl OpenFlags {
    pub const READ: Self = Self {
        read: true,
        write: false,
        create: false,
        truncate: false,
        append: false,
        exclusive: false,
        nonblock: false,
        directory: false,
        sync: false,
    };

    pub const WRITE: Self = Self {
        read: false,
        write: true,
        create: false,
        truncate: false,
        append: false,
        exclusive: false,
        nonblock: false,
        directory: false,
        sync: false,
    };

    pub const RDWR: Self = Self {
        read: true,
        write: true,
        create: false,
        truncate: false,
        append: false,
        exclusive: false,
        nonblock: false,
        directory: false,
        sync: false,
    };
}

/// Seek origin
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

// =============================================================================
// INODE
// =============================================================================

/// Inode (index node)
#[derive(Debug, Clone)]
pub struct Inode {
    pub ino: InodeNum,
    pub file_type: FileType,
    pub mode: FileMode,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub size: u64,
    pub blksize: u32,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub dev: DeviceNum,
    pub rdev: DeviceNum,
}

impl Default for Inode {
    fn default() -> Self {
        Self {
            ino: 0,
            file_type: FileType::Unknown,
            mode: 0,
            nlink: 0,
            uid: 0,
            gid: 0,
            size: 0,
            blksize: 4096,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            dev: 0,
            rdev: 0,
        }
    }
}

/// Directory entry
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub ino: InodeNum,
    pub name: String,
    pub file_type: FileType,
}

// =============================================================================
// VFS OPERATIONS
// =============================================================================

/// Filesystem operations trait
pub trait FileSystemOps: Send + Sync {
    /// Get filesystem type name
    fn name(&self) -> &str;

    /// Mount filesystem
    fn mount(&mut self, device: Option<DeviceNum>, options: &str) -> InitResult<()>;

    /// Unmount filesystem
    fn unmount(&mut self) -> InitResult<()>;

    /// Get root inode
    fn root(&self) -> InitResult<InodeNum>;

    /// Lookup inode by name in parent
    fn lookup(&self, parent: InodeNum, name: &str) -> InitResult<InodeNum>;

    /// Get inode info
    fn stat(&self, ino: InodeNum) -> InitResult<Inode>;

    /// Read directory entries
    fn readdir(&self, ino: InodeNum, offset: u64) -> InitResult<Vec<DirEntry>>;

    /// Read file data
    fn read(&self, ino: InodeNum, offset: u64, size: usize) -> InitResult<Vec<u8>>;

    /// Write file data
    fn write(&mut self, ino: InodeNum, offset: u64, data: &[u8]) -> InitResult<usize>;

    /// Create file
    fn create(&mut self, parent: InodeNum, name: &str, mode: FileMode) -> InitResult<InodeNum>;

    /// Create directory
    fn mkdir(&mut self, parent: InodeNum, name: &str, mode: FileMode) -> InitResult<InodeNum>;

    /// Remove file
    fn unlink(&mut self, parent: InodeNum, name: &str) -> InitResult<()>;

    /// Remove directory
    fn rmdir(&mut self, parent: InodeNum, name: &str) -> InitResult<()>;

    /// Rename file/directory
    fn rename(
        &mut self,
        old_parent: InodeNum,
        old_name: &str,
        new_parent: InodeNum,
        new_name: &str,
    ) -> InitResult<()>;

    /// Sync filesystem
    fn sync(&mut self) -> InitResult<()>;

    /// Get filesystem stats
    fn statfs(&self) -> InitResult<FsStats>;
}

/// Filesystem statistics
#[derive(Debug, Clone, Default)]
pub struct FsStats {
    pub block_size: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub available_blocks: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub fs_type: u64,
    pub max_name_len: u64,
}

// =============================================================================
// MOUNT
// =============================================================================

/// Mount point
pub struct MountPoint {
    pub path: String,
    pub device: Option<DeviceNum>,
    pub fs_type: String,
    pub options: String,
    pub filesystem: Box<dyn FileSystemOps>,
    pub root_ino: InodeNum,
    pub mounted_at: u64,
}

impl MountPoint {
    /// Create new mount point
    pub fn new(
        path: String,
        device: Option<DeviceNum>,
        fs_type: String,
        filesystem: Box<dyn FileSystemOps>,
    ) -> Self {
        Self {
            path,
            device,
            fs_type,
            options: String::new(),
            filesystem,
            root_ino: 0,
            mounted_at: 0,
        }
    }
}

// =============================================================================
// OPEN FILE
// =============================================================================

/// Open file handle
pub struct OpenFile {
    pub fd: FileDescriptor,
    pub path: String,
    pub ino: InodeNum,
    pub mount_idx: usize,
    pub flags: OpenFlags,
    pub offset: AtomicU64,
    pub refcount: AtomicU64,
}

impl OpenFile {
    /// Read current offset
    pub fn offset(&self) -> u64 {
        self.offset.load(Ordering::SeqCst)
    }

    /// Set offset
    pub fn set_offset(&self, off: u64) {
        self.offset.store(off, Ordering::SeqCst);
    }

    /// Seek to position
    pub fn seek(&self, from: SeekFrom, size: u64) -> InitResult<u64> {
        let new_offset = match from {
            SeekFrom::Start(off) => off,
            SeekFrom::End(off) => {
                if off >= 0 {
                    size.saturating_add(off as u64)
                } else {
                    size.saturating_sub((-off) as u64)
                }
            },
            SeekFrom::Current(off) => {
                let current = self.offset();
                if off >= 0 {
                    current.saturating_add(off as u64)
                } else {
                    current.saturating_sub((-off) as u64)
                }
            },
        };

        self.set_offset(new_offset);
        Ok(new_offset)
    }
}

// =============================================================================
// FILESYSTEM SUBSYSTEM
// =============================================================================

/// Filesystem Subsystem
///
/// Manages VFS and mounted filesystems.
pub struct FilesystemSubsystem {
    info: SubsystemInfo,

    // Mount points
    mounts: Vec<MountPoint>,

    // Open files
    open_files: BTreeMap<FileDescriptor, OpenFile>,
    next_fd: AtomicU64,

    // Registered filesystem types
    fs_types: Vec<String>,

    // Root path
    root_mounted: bool,
}

static FS_DEPS: [Dependency; 2] = [
    Dependency::required("drivers"),
    Dependency::required("heap"),
];

impl FilesystemSubsystem {
    /// Create new filesystem subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("filesystem", InitPhase::Late)
                .with_priority(800)
                .with_description("Virtual filesystem")
                .with_dependencies(&FS_DEPS)
                .provides(PhaseCapabilities::FILESYSTEM),
            mounts: Vec::new(),
            open_files: BTreeMap::new(),
            next_fd: AtomicU64::new(3), // 0, 1, 2 reserved for stdin/stdout/stderr
            fs_types: Vec::new(),
            root_mounted: false,
        }
    }

    /// Register filesystem type
    pub fn register_fs_type(&mut self, name: &str) {
        if !self.fs_types.iter().any(|t| t == name) {
            self.fs_types.push(String::from(name));
        }
    }

    /// Get registered filesystem types
    pub fn fs_types(&self) -> &[String] {
        &self.fs_types
    }

    /// Mount filesystem
    pub fn mount(
        &mut self,
        path: &str,
        device: Option<DeviceNum>,
        fs_type: &str,
        mut filesystem: Box<dyn FileSystemOps>,
        options: &str,
    ) -> InitResult<()> {
        // Check if already mounted
        if self.mounts.iter().any(|m| m.path == path) {
            return Err(InitError::new(
                ErrorKind::AlreadyInitialized,
                "Already mounted",
            ));
        }

        // Mount the filesystem
        filesystem.mount(device, options)?;

        // Get root inode
        let root_ino = filesystem.root()?;

        let mut mount = MountPoint::new(
            String::from(path),
            device,
            String::from(fs_type),
            filesystem,
        );
        mount.root_ino = root_ino;
        mount.options = String::from(options);

        self.mounts.push(mount);

        if path == "/" {
            self.root_mounted = true;
        }

        Ok(())
    }

    /// Unmount filesystem
    pub fn unmount(&mut self, path: &str) -> InitResult<()> {
        let idx = self
            .mounts
            .iter()
            .position(|m| m.path == path)
            .ok_or_else(|| InitError::new(ErrorKind::NotFound, "Not mounted"))?;

        // Check for open files
        let has_open_files = self.open_files.values().any(|f| f.mount_idx == idx);
        if has_open_files {
            return Err(InitError::new(ErrorKind::Busy, "Filesystem busy"));
        }

        let mut mount = self.mounts.remove(idx);
        mount.filesystem.unmount()?;

        if path == "/" {
            self.root_mounted = false;
        }

        Ok(())
    }

    /// Find mount point for path
    fn find_mount(&self, path: &str) -> Option<(usize, &str)> {
        let mut best_match = None;
        let mut best_len = 0;

        for (idx, mount) in self.mounts.iter().enumerate() {
            if path.starts_with(&mount.path) {
                let mount_len = mount.path.len();
                if mount_len > best_len {
                    best_len = mount_len;
                    let relative = if mount.path == "/" {
                        path
                    } else {
                        &path[mount_len..]
                    };
                    best_match = Some((idx, relative));
                }
            }
        }

        best_match
    }

    /// Resolve path to inode
    pub fn resolve_path(&self, path: &str) -> InitResult<(usize, InodeNum)> {
        let (mount_idx, relative) = self
            .find_mount(path)
            .ok_or_else(|| InitError::new(ErrorKind::NotFound, "No mount point"))?;

        let mount = &self.mounts[mount_idx];
        let mut ino = mount.root_ino;

        // Walk path components
        for component in relative.split('/').filter(|s| !s.is_empty()) {
            if component == "." {
                continue;
            }
            // Note: ".." handling would need parent tracking

            ino = mount.filesystem.lookup(ino, component)?;
        }

        Ok((mount_idx, ino))
    }

    /// Open file
    pub fn open(&mut self, path: &str, flags: OpenFlags) -> InitResult<FileDescriptor> {
        let (mount_idx, ino) = self.resolve_path(path)?;

        let fd = self.next_fd.fetch_add(1, Ordering::SeqCst) as FileDescriptor;

        let open_file = OpenFile {
            fd,
            path: String::from(path),
            ino,
            mount_idx,
            flags,
            offset: AtomicU64::new(0),
            refcount: AtomicU64::new(1),
        };

        self.open_files.insert(fd, open_file);

        Ok(fd)
    }

    /// Close file
    pub fn close(&mut self, fd: FileDescriptor) -> InitResult<()> {
        if let Some(file) = self.open_files.get(&fd) {
            let count = file.refcount.fetch_sub(1, Ordering::SeqCst);
            if count == 1 {
                self.open_files.remove(&fd);
            }
        }
        Ok(())
    }

    /// Read from file
    pub fn read(&self, fd: FileDescriptor, buf: &mut [u8]) -> InitResult<usize> {
        let file = self
            .open_files
            .get(&fd)
            .ok_or_else(|| InitError::new(ErrorKind::InvalidArgument, "Bad fd"))?;

        if !file.flags.read {
            return Err(InitError::new(ErrorKind::PermissionDenied, "Not readable"));
        }

        let mount = &self.mounts[file.mount_idx];
        let offset = file.offset();

        let data = mount.filesystem.read(file.ino, offset, buf.len())?;
        let bytes_read = data.len().min(buf.len());
        buf[..bytes_read].copy_from_slice(&data[..bytes_read]);

        file.set_offset(offset + bytes_read as u64);

        Ok(bytes_read)
    }

    /// Write to file
    pub fn write(&mut self, fd: FileDescriptor, buf: &[u8]) -> InitResult<usize> {
        let file = self
            .open_files
            .get(&fd)
            .ok_or_else(|| InitError::new(ErrorKind::InvalidArgument, "Bad fd"))?;

        if !file.flags.write {
            return Err(InitError::new(ErrorKind::PermissionDenied, "Not writable"));
        }

        let offset = file.offset();
        let ino = file.ino;
        let mount_idx = file.mount_idx;

        let mount = &mut self.mounts[mount_idx];
        let bytes_written = mount.filesystem.write(ino, offset, buf)?;

        if let Some(file) = self.open_files.get(&fd) {
            file.set_offset(offset + bytes_written as u64);
        }

        Ok(bytes_written)
    }

    /// Stat file
    pub fn stat(&self, path: &str) -> InitResult<Inode> {
        let (mount_idx, ino) = self.resolve_path(path)?;
        self.mounts[mount_idx].filesystem.stat(ino)
    }

    /// List directory
    pub fn readdir(&self, path: &str) -> InitResult<Vec<DirEntry>> {
        let (mount_idx, ino) = self.resolve_path(path)?;
        self.mounts[mount_idx].filesystem.readdir(ino, 0)
    }

    /// Create directory
    pub fn mkdir(&mut self, path: &str, mode: FileMode) -> InitResult<()> {
        // Get parent path and name
        let (parent_path, name) = path
            .rsplit_once('/')
            .ok_or_else(|| InitError::new(ErrorKind::InvalidArgument, "Invalid path"))?;

        let parent_path = if parent_path.is_empty() {
            "/"
        } else {
            parent_path
        };

        let (mount_idx, parent_ino) = self.resolve_path(parent_path)?;
        self.mounts[mount_idx]
            .filesystem
            .mkdir(parent_ino, name, mode)?;

        Ok(())
    }

    /// Sync all filesystems
    pub fn sync_all(&mut self) -> InitResult<()> {
        for mount in &mut self.mounts {
            mount.filesystem.sync()?;
        }
        Ok(())
    }

    /// Get mount info
    pub fn mounts(&self) -> Vec<MountInfo> {
        self.mounts
            .iter()
            .map(|m| MountInfo {
                path: m.path.clone(),
                fs_type: m.fs_type.clone(),
                device: m.device,
                options: m.options.clone(),
            })
            .collect()
    }
}

/// Mount information
#[derive(Debug, Clone)]
pub struct MountInfo {
    pub path: String,
    pub fs_type: String,
    pub device: Option<DeviceNum>,
    pub options: String,
}

impl Default for FilesystemSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for FilesystemSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing filesystem subsystem");

        // Register built-in filesystem types
        self.register_fs_type("ramfs");
        self.register_fs_type("tmpfs");
        self.register_fs_type("devfs");
        self.register_fs_type("procfs");
        self.register_fs_type("sysfs");

        ctx.debug(alloc::format!(
            "Registered filesystem types: {:?}",
            self.fs_types
        ));

        // In real kernel: mount root filesystem
        // self.mount("/", Some(root_dev), "ext4", Box::new(Ext4Fs::new()), "")?;

        ctx.info("Filesystem subsystem initialized");

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Filesystem subsystem shutdown");

        // Sync all filesystems
        self.sync_all()?;

        // Close all open files
        self.open_files.clear();

        // Unmount all (reverse order)
        while let Some(mount) = self.mounts.pop() {
            ctx.debug(alloc::format!("Unmounting {}", mount.path));
            // mount.filesystem.unmount() handled by drop
        }

        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_subsystem() {
        let sub = FilesystemSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Late);
        assert!(sub.info().provides.contains(PhaseCapabilities::FILESYSTEM));
    }

    #[test]
    fn test_file_type() {
        assert_eq!(FileType::from(0o0100644), FileType::Regular);
        assert_eq!(FileType::from(0o0040755), FileType::Directory);
        assert_eq!(FileType::from(0o0120777), FileType::Symlink);
    }

    #[test]
    fn test_permissions() {
        let perms = FilePermissions::from(0o755);
        assert!(perms.owner_read);
        assert!(perms.owner_write);
        assert!(perms.owner_exec);
        assert!(perms.group_read);
        assert!(!perms.group_write);
        assert!(perms.group_exec);
    }

    #[test]
    fn test_open_file_seek() {
        let file = OpenFile {
            fd: 3,
            path: String::from("/test"),
            ino: 1,
            mount_idx: 0,
            flags: OpenFlags::READ,
            offset: AtomicU64::new(0),
            refcount: AtomicU64::new(1),
        };

        assert_eq!(file.seek(SeekFrom::Start(100), 1000).unwrap(), 100);
        assert_eq!(file.offset(), 100);

        assert_eq!(file.seek(SeekFrom::Current(50), 1000).unwrap(), 150);
        assert_eq!(file.seek(SeekFrom::End(-100), 1000).unwrap(), 900);
    }
}
