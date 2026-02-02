//! Mount Point Management
//!
//! Handles filesystem mounting, unmounting, and mount options.

use super::vfs::PathBuf;
use super::{Credentials, FsStats, ROOT_INODE};
use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Constants
// ============================================================================

/// Maximum mount points
pub const MAX_MOUNT_POINTS: usize = 256;

/// Maximum mount source length
pub const MAX_SOURCE_LEN: usize = 256;

/// Maximum mount options length
pub const MAX_OPTIONS_LEN: usize = 1024;

// ============================================================================
// Mount Flags
// ============================================================================

/// Mount flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct MountFlags(pub u64);

impl MountFlags {
    // Standard mount flags
    /// Read-only mount
    pub const MS_RDONLY: u64 = 1 << 0;
    /// Don't allow setuid/setgid
    pub const MS_NOSUID: u64 = 1 << 1;
    /// Don't interpret special files
    pub const MS_NODEV: u64 = 1 << 2;
    /// Don't allow execution
    pub const MS_NOEXEC: u64 = 1 << 3;
    /// Synchronous writes
    pub const MS_SYNCHRONOUS: u64 = 1 << 4;
    /// Remount existing mount
    pub const MS_REMOUNT: u64 = 1 << 5;
    /// Allow mandatory locks
    pub const MS_MANDLOCK: u64 = 1 << 6;
    /// Directory sync
    pub const MS_DIRSYNC: u64 = 1 << 7;
    /// Don't update access times
    pub const MS_NOATIME: u64 = 1 << 10;
    /// Don't update directory access times
    pub const MS_NODIRATIME: u64 = 1 << 11;
    /// Bind mount
    pub const MS_BIND: u64 = 1 << 12;
    /// Move mount point
    pub const MS_MOVE: u64 = 1 << 13;
    /// Recursive mount
    pub const MS_REC: u64 = 1 << 14;
    /// Silent mount (don't print errors)
    pub const MS_SILENT: u64 = 1 << 15;
    /// VFS doesn't apply umask
    pub const MS_POSIXACL: u64 = 1 << 16;
    /// Unbindable mount
    pub const MS_UNBINDABLE: u64 = 1 << 17;
    /// Private mount
    pub const MS_PRIVATE: u64 = 1 << 18;
    /// Slave mount
    pub const MS_SLAVE: u64 = 1 << 19;
    /// Shared mount
    pub const MS_SHARED: u64 = 1 << 20;
    /// Update atime relative to mtime/ctime
    pub const MS_RELATIME: u64 = 1 << 21;
    /// Lazy time updates
    pub const MS_LAZYTIME: u64 = 1 << 25;

    /// Create empty flags
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Check flag
    #[inline]
    pub fn has(&self, flag: u64) -> bool {
        self.0 & flag != 0
    }

    /// Set flag
    #[inline]
    pub fn set(&mut self, flag: u64) {
        self.0 |= flag;
    }

    /// Clear flag
    #[inline]
    pub fn clear(&mut self, flag: u64) {
        self.0 &= !flag;
    }

    /// Is read-only
    #[inline]
    pub fn is_readonly(&self) -> bool {
        self.has(Self::MS_RDONLY)
    }

    /// Is noatime
    #[inline]
    pub fn is_noatime(&self) -> bool {
        self.has(Self::MS_NOATIME)
    }

    /// Is sync
    #[inline]
    pub fn is_sync(&self) -> bool {
        self.has(Self::MS_SYNCHRONOUS)
    }
}

// ============================================================================
// Mount Options
// ============================================================================

/// HelixFS-specific mount options.
#[derive(Clone, Copy, Debug)]
pub struct MountOptions {
    /// Mount flags
    pub flags: MountFlags,
    /// Maximum file size (0 = unlimited)
    pub max_file_size: u64,
    /// Commit interval in seconds
    pub commit_interval: u32,
    /// Enable compression
    pub compress: bool,
    /// Compression algorithm
    pub compress_algo: CompressionAlgo,
    /// Compression level (1-22)
    pub compress_level: u8,
    /// Enable encryption
    pub encrypt: bool,
    /// Enable integrity checking
    pub integrity: bool,
    /// Enable snapshots
    pub snapshots: bool,
    /// Enable deduplication
    pub dedup: bool,
    /// Cache mode
    pub cache_mode: CacheMode,
    /// Error behavior
    pub errors: ErrorBehavior,
    /// Default permissions mode
    pub default_mode: u32,
    /// Default UID
    pub default_uid: u32,
    /// Default GID
    pub default_gid: u32,
}

impl MountOptions {
    /// Create default mount options
    pub const fn new() -> Self {
        Self {
            flags: MountFlags(0),
            max_file_size: 0,
            commit_interval: 5,
            compress: false,
            compress_algo: CompressionAlgo::None,
            compress_level: 3,
            encrypt: false,
            integrity: true,
            snapshots: true,
            dedup: false,
            cache_mode: CacheMode::Normal,
            errors: ErrorBehavior::Continue,
            default_mode: 0o755,
            default_uid: 0,
            default_gid: 0,
        }
    }

    /// Read-only mount
    pub fn readonly(mut self) -> Self {
        self.flags.set(MountFlags::MS_RDONLY);
        self
    }

    /// Enable compression
    pub fn with_compression(mut self, algo: CompressionAlgo, level: u8) -> Self {
        self.compress = true;
        self.compress_algo = algo;
        self.compress_level = level;
        self
    }

    /// Enable encryption
    pub fn with_encryption(mut self) -> Self {
        self.encrypt = true;
        self
    }

    /// Set commit interval
    pub fn commit_interval(mut self, secs: u32) -> Self {
        self.commit_interval = secs;
        self
    }
}

impl Default for MountOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Compression algorithm for mount options.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionAlgo {
    /// No compression
    None = 0,
    /// LZ4 (fast)
    Lz4  = 1,
    /// ZSTD (balanced)
    Zstd = 2,
    /// LZO
    Lzo  = 3,
}

impl Default for CompressionAlgo {
    fn default() -> Self {
        Self::None
    }
}

/// Cache mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CacheMode {
    /// Normal caching
    Normal       = 0,
    /// Write-through cache
    WriteThrough = 1,
    /// No caching
    NoCache      = 2,
    /// Metadata only caching
    MetadataOnly = 3,
}

impl Default for CacheMode {
    fn default() -> Self {
        Self::Normal
    }
}

/// Error behavior on mount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ErrorBehavior {
    /// Continue on errors
    Continue  = 0,
    /// Remount read-only
    RemountRo = 1,
    /// Panic on error
    Panic     = 2,
}

impl Default for ErrorBehavior {
    fn default() -> Self {
        Self::Continue
    }
}

// ============================================================================
// Mount Point
// ============================================================================

/// Mount state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MountState {
    /// Mount point not active
    Unmounted  = 0,
    /// Currently mounting
    Mounting   = 1,
    /// Mounted and active
    Mounted    = 2,
    /// Currently unmounting
    Unmounting = 3,
    /// Mount failed
    Failed     = 4,
}

impl Default for MountState {
    fn default() -> Self {
        Self::Unmounted
    }
}

/// Mount point entry.
#[derive(Clone, Debug)]
pub struct MountPoint {
    /// Mount ID
    pub id: u32,
    /// Parent mount ID
    pub parent_id: u32,
    /// Device major number
    pub dev_major: u32,
    /// Device minor number
    pub dev_minor: u32,
    /// Root inode
    pub root_ino: u64,
    /// Mount path
    pub mount_path: PathBuf,
    /// Source device/path
    pub source: [u8; MAX_SOURCE_LEN],
    /// Source length
    pub source_len: usize,
    /// Mount options
    pub options: MountOptions,
    /// Mount state
    pub state: MountState,
    /// Mount time
    pub mount_time: u64,
    /// Reference count
    pub refcount: u32,
    /// Filesystem stats
    pub stats: FsStats,
}

impl MountPoint {
    /// Create new mount point
    pub fn new(id: u32, mount_path: &[u8]) -> Self {
        Self {
            id,
            parent_id: 0,
            dev_major: 0,
            dev_minor: 0,
            root_ino: ROOT_INODE,
            mount_path: PathBuf::from_slice(mount_path),
            source: [0; MAX_SOURCE_LEN],
            source_len: 0,
            options: MountOptions::new(),
            state: MountState::Unmounted,
            mount_time: 0,
            refcount: 0,
            stats: FsStats::new(),
        }
    }

    /// Set source
    pub fn set_source(&mut self, source: &[u8]) {
        let len = core::cmp::min(source.len(), MAX_SOURCE_LEN);
        self.source[..len].copy_from_slice(&source[..len]);
        self.source_len = len;
    }

    /// Get source
    pub fn source(&self) -> &[u8] {
        &self.source[..self.source_len]
    }

    /// Is mounted
    #[inline]
    pub fn is_mounted(&self) -> bool {
        self.state == MountState::Mounted
    }

    /// Is read-only
    #[inline]
    pub fn is_readonly(&self) -> bool {
        self.options.flags.is_readonly()
    }

    /// Get device ID
    pub fn device_id(&self) -> u64 {
        ((self.dev_major as u64) << 32) | (self.dev_minor as u64)
    }
}

impl Default for MountPoint {
    fn default() -> Self {
        Self::new(0, b"/")
    }
}

// ============================================================================
// Mount Table
// ============================================================================

/// Mount table managing all mount points.
pub struct MountTable {
    /// Mount points
    mounts: [Option<MountPoint>; MAX_MOUNT_POINTS],
    /// Number of active mounts
    count: usize,
    /// Next mount ID
    next_id: u32,
}

impl MountTable {
    /// Create new mount table
    pub const fn new() -> Self {
        const NONE: Option<MountPoint> = None;
        Self {
            mounts: [NONE; MAX_MOUNT_POINTS],
            count: 0,
            next_id: 1,
        }
    }

    /// Add mount point
    pub fn add(&mut self, source: &[u8], target: &[u8], options: MountOptions) -> HfsResult<u32> {
        // Find free slot
        let slot = self
            .mounts
            .iter()
            .position(|m| m.is_none())
            .ok_or(HfsError::NoSpace)?;

        let id = self.next_id;
        self.next_id += 1;

        let mut mount = MountPoint::new(id, target);
        mount.set_source(source);
        mount.options = options;
        mount.state = MountState::Mounting;

        self.mounts[slot] = Some(mount);
        self.count += 1;

        Ok(id)
    }

    /// Remove mount point
    pub fn remove(&mut self, id: u32) -> HfsResult<()> {
        let slot = self
            .mounts
            .iter()
            .position(|m| m.as_ref().map_or(false, |mp| mp.id == id))
            .ok_or(HfsError::NotFound)?;

        let mount = self.mounts[slot].as_ref().unwrap();

        if mount.refcount > 0 {
            return Err(HfsError::Busy);
        }

        self.mounts[slot] = None;
        self.count -= 1;

        Ok(())
    }

    /// Get mount by ID
    pub fn get(&self, id: u32) -> Option<&MountPoint> {
        self.mounts
            .iter()
            .filter_map(|m| m.as_ref())
            .find(|m| m.id == id)
    }

    /// Get mount by ID (mutable)
    pub fn get_mut(&mut self, id: u32) -> Option<&mut MountPoint> {
        self.mounts
            .iter_mut()
            .filter_map(|m| m.as_mut())
            .find(|m| m.id == id)
    }

    /// Find mount by path
    pub fn find_by_path(&self, path: &[u8]) -> Option<&MountPoint> {
        let mut best_match: Option<&MountPoint> = None;
        let mut best_len = 0;

        for mount in self.mounts.iter().filter_map(|m| m.as_ref()) {
            if !mount.is_mounted() {
                continue;
            }

            let mount_path = mount.mount_path.as_slice();

            if path.starts_with(mount_path) {
                let len = mount_path.len();
                // Ensure it's a proper prefix (at directory boundary)
                if len == path.len() || path[len] == b'/' || mount_path.ends_with(b"/") {
                    if len > best_len {
                        best_match = Some(mount);
                        best_len = len;
                    }
                }
            }
        }

        best_match
    }

    /// Iterate over mounts
    pub fn iter(&self) -> impl Iterator<Item = &MountPoint> {
        self.mounts.iter().filter_map(|m| m.as_ref())
    }

    /// Count active mounts
    pub fn count(&self) -> usize {
        self.count
    }

    /// Set mount state
    pub fn set_state(&mut self, id: u32, state: MountState) -> HfsResult<()> {
        let mount = self.get_mut(id).ok_or(HfsError::NotFound)?;
        mount.state = state;
        Ok(())
    }

    /// Increment refcount
    pub fn acquire(&mut self, id: u32) -> HfsResult<()> {
        let mount = self.get_mut(id).ok_or(HfsError::NotFound)?;
        mount.refcount = mount.refcount.checked_add(1).ok_or(HfsError::Overflow)?;
        Ok(())
    }

    /// Decrement refcount
    pub fn release(&mut self, id: u32) -> HfsResult<()> {
        let mount = self.get_mut(id).ok_or(HfsError::NotFound)?;
        mount.refcount = mount.refcount.checked_sub(1).ok_or(HfsError::Underflow)?;
        Ok(())
    }
}

impl Default for MountTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Mount Request
// ============================================================================

/// Mount request.
#[derive(Clone, Debug)]
pub struct MountRequest {
    /// Source device or path
    pub source: [u8; MAX_SOURCE_LEN],
    /// Source length
    pub source_len: usize,
    /// Target mount point
    pub target: PathBuf,
    /// Filesystem type
    pub fstype: FsType,
    /// Mount options
    pub options: MountOptions,
    /// Credentials
    pub cred: Credentials,
}

impl MountRequest {
    /// Create new mount request
    pub fn new(source: &[u8], target: &[u8]) -> Self {
        let mut req = Self {
            source: [0; MAX_SOURCE_LEN],
            source_len: 0,
            target: PathBuf::from_slice(target),
            fstype: FsType::HelixFs,
            options: MountOptions::new(),
            cred: Credentials::root(),
        };

        let len = core::cmp::min(source.len(), MAX_SOURCE_LEN);
        req.source[..len].copy_from_slice(&source[..len]);
        req.source_len = len;

        req
    }

    /// Set filesystem type
    pub fn fstype(mut self, fstype: FsType) -> Self {
        self.fstype = fstype;
        self
    }

    /// Set options
    pub fn options(mut self, options: MountOptions) -> Self {
        self.options = options;
        self
    }

    /// Set credentials
    pub fn credentials(mut self, cred: Credentials) -> Self {
        self.cred = cred;
        self
    }

    /// Get source
    pub fn source(&self) -> &[u8] {
        &self.source[..self.source_len]
    }

    /// Validate request
    pub fn validate(&self) -> HfsResult<()> {
        if self.source_len == 0 {
            return Err(HfsError::InvalidArgument);
        }

        if !self.target.is_absolute() {
            return Err(HfsError::InvalidPath);
        }

        Ok(())
    }
}

/// Filesystem type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FsType {
    /// HelixFS
    HelixFs = 0,
    /// Unknown/auto-detect
    Auto    = 1,
}

impl Default for FsType {
    fn default() -> Self {
        Self::HelixFs
    }
}

// ============================================================================
// Unmount Request
// ============================================================================

/// Unmount flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct UnmountFlags(pub u32);

impl UnmountFlags {
    /// Force unmount
    pub const MNT_FORCE: u32 = 1 << 0;
    /// Lazy unmount
    pub const MNT_DETACH: u32 = 1 << 1;
    /// Mark mount as expired
    pub const MNT_EXPIRE: u32 = 1 << 2;

    /// Check flag
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }

    /// Is force
    #[inline]
    pub fn is_force(&self) -> bool {
        self.has(Self::MNT_FORCE)
    }

    /// Is lazy
    #[inline]
    pub fn is_lazy(&self) -> bool {
        self.has(Self::MNT_DETACH)
    }
}

/// Unmount request.
#[derive(Clone, Debug)]
pub struct UnmountRequest {
    /// Target mount point
    pub target: PathBuf,
    /// Unmount flags
    pub flags: UnmountFlags,
    /// Credentials
    pub cred: Credentials,
}

impl UnmountRequest {
    /// Create new unmount request
    pub fn new(target: &[u8]) -> Self {
        Self {
            target: PathBuf::from_slice(target),
            flags: UnmountFlags::default(),
            cred: Credentials::root(),
        }
    }

    /// Force unmount
    pub fn force(mut self) -> Self {
        self.flags = UnmountFlags(self.flags.0 | UnmountFlags::MNT_FORCE);
        self
    }

    /// Lazy unmount
    pub fn lazy(mut self) -> Self {
        self.flags = UnmountFlags(self.flags.0 | UnmountFlags::MNT_DETACH);
        self
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_flags() {
        let mut flags = MountFlags::empty();

        flags.set(MountFlags::MS_RDONLY);
        flags.set(MountFlags::MS_NOATIME);

        assert!(flags.is_readonly());
        assert!(flags.is_noatime());
        assert!(!flags.is_sync());
    }

    #[test]
    fn test_mount_options() {
        let opts = MountOptions::new()
            .readonly()
            .with_compression(CompressionAlgo::Zstd, 3)
            .commit_interval(10);

        assert!(opts.flags.is_readonly());
        assert!(opts.compress);
        assert_eq!(opts.compress_algo, CompressionAlgo::Zstd);
        assert_eq!(opts.commit_interval, 10);
    }

    #[test]
    fn test_mount_table() {
        let mut table = MountTable::new();

        let id = table
            .add(b"/dev/sda1", b"/mnt/test", MountOptions::new())
            .unwrap();
        table.set_state(id, MountState::Mounted).unwrap();

        assert_eq!(table.count(), 1);

        let mount = table.get(id).unwrap();
        assert!(mount.is_mounted());

        // Find by path
        let found = table.find_by_path(b"/mnt/test/subdir");
        assert!(found.is_some());
    }

    #[test]
    fn test_mount_request() {
        let req =
            MountRequest::new(b"/dev/sda1", b"/mnt/disk").options(MountOptions::new().readonly());

        assert!(req.validate().is_ok());
        assert!(req.options.flags.is_readonly());
    }

    #[test]
    fn test_unmount_flags() {
        let flags = UnmountFlags(UnmountFlags::MNT_FORCE | UnmountFlags::MNT_DETACH);

        assert!(flags.is_force());
        assert!(flags.is_lazy());
    }
}
