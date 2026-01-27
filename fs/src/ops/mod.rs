//! Filesystem Operations Module
//!
//! High-level filesystem operations that integrate all subsystems.
//! This is the main entry point for all filesystem operations.

#![allow(dead_code)]

use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Submodules
// ============================================================================

pub mod file;
pub mod dir;
pub mod io;
pub mod link;
pub mod xattr;

pub use file::*;
pub use dir::*;
pub use io::*;
pub use link::*;
pub use xattr::*;

// ============================================================================
// Constants
// ============================================================================

/// Maximum concurrent operations
pub const MAX_CONCURRENT_OPS: usize = 256;

/// Maximum path components
pub const MAX_PATH_COMPONENTS: usize = 128;

/// Maximum symlink loops
pub const MAX_SYMLINK_LOOPS: usize = 40;

/// Maximum I/O size
pub const MAX_IO_SIZE: usize = 128 * 1024 * 1024; // 128MB

/// Default I/O chunk size
pub const DEFAULT_IO_CHUNK: usize = 1024 * 1024; // 1MB

// ============================================================================
// Operation Context
// ============================================================================

/// Operation type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OpType {
    /// Read operation
    Read = 0,
    /// Write operation
    Write = 1,
    /// Create operation
    Create = 2,
    /// Delete operation
    Delete = 3,
    /// Rename operation
    Rename = 4,
    /// Link operation
    Link = 5,
    /// Stat operation
    Stat = 6,
    /// Sync operation
    Sync = 7,
    /// Truncate operation
    Truncate = 8,
    /// Xattr operation
    Xattr = 9,
}

/// Operation flags.
#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct OpFlags(pub u32);

impl OpFlags {
    /// Synchronous operation
    pub const O_SYNC: u32 = 1 << 0;
    /// Direct I/O (bypass cache)
    pub const O_DIRECT: u32 = 1 << 1;
    /// Non-blocking operation
    pub const O_NONBLOCK: u32 = 1 << 2;
    /// Atomic operation
    pub const O_ATOMIC: u32 = 1 << 3;
    /// Don't update atime
    pub const O_NOATIME: u32 = 1 << 4;
    /// Follow symlinks
    pub const O_FOLLOW: u32 = 1 << 5;
    /// Exclusive lock
    pub const O_EXCL: u32 = 1 << 6;
    /// Internal operation
    pub const O_INTERNAL: u32 = 1 << 7;
    
    #[inline]
    pub fn has(&self, flag: u32) -> bool {
        self.0 & flag != 0
    }
    
    #[inline]
    pub fn set(&mut self, flag: u32) {
        self.0 |= flag;
    }
}

/// Operation context.
#[derive(Clone, Copy, Debug)]
pub struct OpContext {
    /// Operation type
    pub op_type: OpType,
    /// Operation flags
    pub flags: OpFlags,
    /// Credentials (UID)
    pub uid: u32,
    /// Credentials (GID)
    pub gid: u32,
    /// Namespace ID
    pub ns_id: u32,
    /// Process ID
    pub pid: u32,
    /// Thread ID
    pub tid: u32,
    /// Start timestamp
    pub start_time: u64,
    /// Deadline (0 = no deadline)
    pub deadline: u64,
    /// Priority (0 = highest)
    pub priority: u8,
}

impl OpContext {
    /// Create new operation context
    pub fn new(op_type: OpType, uid: u32, gid: u32) -> Self {
        Self {
            op_type,
            flags: OpFlags::default(),
            uid,
            gid,
            ns_id: 0,
            pid: 0,
            tid: 0,
            start_time: 0,
            deadline: 0,
            priority: 4,
        }
    }
    
    /// Create read context
    pub fn read(uid: u32, gid: u32) -> Self {
        Self::new(OpType::Read, uid, gid)
    }
    
    /// Create write context
    pub fn write(uid: u32, gid: u32) -> Self {
        Self::new(OpType::Write, uid, gid)
    }
    
    /// Set sync flag
    pub fn sync(mut self) -> Self {
        self.flags.set(OpFlags::O_SYNC);
        self
    }
    
    /// Set direct flag
    pub fn direct(mut self) -> Self {
        self.flags.set(OpFlags::O_DIRECT);
        self
    }
    
    /// Is synchronous
    #[inline]
    pub fn is_sync(&self) -> bool {
        self.flags.has(OpFlags::O_SYNC)
    }
    
    /// Is direct I/O
    #[inline]
    pub fn is_direct(&self) -> bool {
        self.flags.has(OpFlags::O_DIRECT)
    }
    
    /// Is past deadline
    pub fn is_past_deadline(&self, now: u64) -> bool {
        self.deadline > 0 && now > self.deadline
    }
}

impl Default for OpContext {
    fn default() -> Self {
        Self::new(OpType::Read, 0, 0)
    }
}

// ============================================================================
// Operation Result
// ============================================================================

/// Operation statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpStats {
    /// Bytes transferred
    pub bytes: u64,
    /// I/O operations performed
    pub io_ops: u32,
    /// Cache hits
    pub cache_hits: u32,
    /// Cache misses
    pub cache_misses: u32,
    /// Time spent (nanoseconds)
    pub time_ns: u64,
}

impl OpStats {
    /// Add bytes
    #[inline]
    pub fn add_bytes(&mut self, bytes: u64) {
        self.bytes += bytes;
    }
    
    /// Record I/O
    #[inline]
    pub fn record_io(&mut self, is_hit: bool) {
        self.io_ops += 1;
        if is_hit {
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }
    }
    
    /// Record time
    #[inline]
    pub fn record_time(&mut self, start: u64, end: u64) {
        self.time_ns = end.saturating_sub(start);
    }
    
    /// Hit rate
    pub fn hit_rate(&self) -> f32 {
        let total = self.cache_hits + self.cache_misses;
        if total == 0 {
            return 0.0;
        }
        self.cache_hits as f32 / total as f32
    }
    
    /// Throughput (bytes/sec)
    pub fn throughput(&self) -> u64 {
        if self.time_ns == 0 {
            return 0;
        }
        (self.bytes * 1_000_000_000) / self.time_ns
    }
}

// ============================================================================
// Path Operations
// ============================================================================

/// Path component.
#[derive(Clone, Copy)]
pub struct PathComponent {
    /// Component bytes
    pub name: [u8; 256],
    /// Length
    pub len: u8,
}

impl PathComponent {
    /// Create from slice
    pub fn from_slice(s: &[u8]) -> Self {
        let mut name = [0u8; 256];
        let len = core::cmp::min(s.len(), 255);
        name[..len].copy_from_slice(&s[..len]);
        Self {
            name,
            len: len as u8,
        }
    }
    
    /// Get as slice
    pub fn as_slice(&self) -> &[u8] {
        &self.name[..self.len as usize]
    }
    
    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    /// Is "."
    pub fn is_dot(&self) -> bool {
        self.len == 1 && self.name[0] == b'.'
    }
    
    /// Is ".."
    pub fn is_dotdot(&self) -> bool {
        self.len == 2 && self.name[0] == b'.' && self.name[1] == b'.'
    }
}

impl Default for PathComponent {
    fn default() -> Self {
        Self {
            name: [0; 256],
            len: 0,
        }
    }
}

/// Split path into components
pub fn split_path(path: &[u8]) -> ([PathComponent; MAX_PATH_COMPONENTS], usize) {
    let mut components = [PathComponent::default(); MAX_PATH_COMPONENTS];
    let mut count = 0;
    
    let mut start = 0;
    let path = if path.first() == Some(&b'/') {
        &path[1..]
    } else {
        path
    };
    
    for (i, &c) in path.iter().enumerate() {
        if c == b'/' {
            if i > start && count < MAX_PATH_COMPONENTS {
                components[count] = PathComponent::from_slice(&path[start..i]);
                count += 1;
            }
            start = i + 1;
        }
    }
    
    // Last component
    if start < path.len() && count < MAX_PATH_COMPONENTS {
        components[count] = PathComponent::from_slice(&path[start..]);
        count += 1;
    }
    
    (components, count)
}

// ============================================================================
// Permission Checking
// ============================================================================

/// Permission flags.
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Permission {
    /// Read permission
    Read = 4,
    /// Write permission
    Write = 2,
    /// Execute permission
    Execute = 1,
}

/// Check permission.
pub fn check_permission(
    mode: u16,
    owner_uid: u32,
    owner_gid: u32,
    ctx: &OpContext,
    perm: Permission,
) -> bool {
    // Root can do anything
    if ctx.uid == 0 {
        return true;
    }
    
    let perm = perm as u16;
    
    // Check owner permissions
    if ctx.uid == owner_uid {
        return (mode >> 6) & perm != 0;
    }
    
    // Check group permissions
    if ctx.gid == owner_gid {
        return (mode >> 3) & perm != 0;
    }
    
    // Check other permissions
    mode & perm != 0
}

// ============================================================================
// Inode Type
// ============================================================================

/// Inode type (for S_IFMT).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum InodeType {
    /// Regular file
    Regular = 0o100000,
    /// Directory
    Directory = 0o040000,
    /// Character device
    CharDevice = 0o020000,
    /// Block device
    BlockDevice = 0o060000,
    /// FIFO
    Fifo = 0o010000,
    /// Socket
    Socket = 0o140000,
    /// Symbolic link
    Symlink = 0o120000,
}

impl InodeType {
    /// From mode
    pub fn from_mode(mode: u16) -> Option<Self> {
        match mode & 0o170000 {
            0o100000 => Some(Self::Regular),
            0o040000 => Some(Self::Directory),
            0o020000 => Some(Self::CharDevice),
            0o060000 => Some(Self::BlockDevice),
            0o010000 => Some(Self::Fifo),
            0o140000 => Some(Self::Socket),
            0o120000 => Some(Self::Symlink),
            _ => None,
        }
    }
    
    /// To mode mask
    pub fn to_mode(self) -> u16 {
        self as u16
    }
    
    /// Is regular file
    pub fn is_file(self) -> bool {
        self == Self::Regular
    }
    
    /// Is directory
    pub fn is_dir(self) -> bool {
        self == Self::Directory
    }
    
    /// Is symlink
    pub fn is_symlink(self) -> bool {
        self == Self::Symlink
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_op_context() {
        let ctx = OpContext::read(1000, 1000).sync().direct();
        
        assert_eq!(ctx.op_type, OpType::Read);
        assert!(ctx.is_sync());
        assert!(ctx.is_direct());
    }
    
    #[test]
    fn test_op_stats() {
        let mut stats = OpStats::default();
        
        stats.add_bytes(1024);
        stats.record_io(true);
        stats.record_io(false);
        
        assert_eq!(stats.bytes, 1024);
        assert_eq!(stats.cache_hits, 1);
        assert_eq!(stats.cache_misses, 1);
        assert!((stats.hit_rate() - 0.5).abs() < 0.01);
    }
    
    #[test]
    fn test_split_path() {
        let (components, count) = split_path(b"/home/user/file.txt");
        
        assert_eq!(count, 3);
        assert_eq!(components[0].as_slice(), b"home");
        assert_eq!(components[1].as_slice(), b"user");
        assert_eq!(components[2].as_slice(), b"file.txt");
    }
    
    #[test]
    fn test_path_component() {
        let dot = PathComponent::from_slice(b".");
        assert!(dot.is_dot());
        
        let dotdot = PathComponent::from_slice(b"..");
        assert!(dotdot.is_dotdot());
    }
    
    #[test]
    fn test_permission() {
        let mode = 0o644;
        let owner = 1000;
        let group = 1000;
        
        // Owner can read/write
        let ctx = OpContext::new(OpType::Read, 1000, 1000);
        assert!(check_permission(mode, owner, group, &ctx, Permission::Read));
        assert!(check_permission(mode, owner, group, &ctx, Permission::Write));
        
        // Others can only read
        let ctx = OpContext::new(OpType::Read, 2000, 2000);
        assert!(check_permission(mode, owner, group, &ctx, Permission::Read));
        assert!(!check_permission(mode, owner, group, &ctx, Permission::Write));
        
        // Root can do anything
        let ctx = OpContext::new(OpType::Write, 0, 0);
        assert!(check_permission(mode, owner, group, &ctx, Permission::Write));
    }
    
    #[test]
    fn test_inode_type() {
        let mode = 0o100644;
        let typ = InodeType::from_mode(mode).unwrap();
        assert!(typ.is_file());
        
        let mode = 0o040755;
        let typ = InodeType::from_mode(mode).unwrap();
        assert!(typ.is_dir());
    }
}
