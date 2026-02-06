//! Fundamental type definitions for HelixFS.
//!
//! This module defines all core types used throughout the filesystem,
//! including block addresses, inode numbers, timestamps, and various
//! configuration structures.

use core::fmt;
use core::ops::{Add, BitAnd, BitOr, Not, Sub};

#[cfg(feature = "alloc")]
extern crate alloc as alloc_crate;

// ============================================================================
// Block and Inode Types
// ============================================================================

/// Physical block number on disk.
///
/// A block is the fundamental unit of allocation in HelixFS.
/// With 64-bit addressing and 4KB blocks, we can address up to 64 ZB.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct BlockNum(pub u64);

impl BlockNum {
    pub const NULL: Self = Self(0);
    pub const INVALID: Self = Self(u64::MAX);

    #[inline]
    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    /// Alias for is_null
    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_valid(self) -> bool {
        self.0 != 0 && self.0 != u64::MAX
    }

    /// Convert to byte offset on disk
    #[inline]
    pub const fn to_byte_offset(self, block_shift: u32) -> u64 {
        self.0 << block_shift
    }

    /// Create from byte offset
    #[inline]
    pub const fn from_byte_offset(offset: u64, block_shift: u32) -> Self {
        Self(offset >> block_shift)
    }
}

impl fmt::Debug for BlockNum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Block({})", self.0)
    }
}

impl fmt::Display for BlockNum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add<u64> for BlockNum {
    type Output = Self;
    #[inline]
    fn add(self, rhs: u64) -> Self {
        Self(self.0.wrapping_add(rhs))
    }
}

impl Sub<u64> for BlockNum {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: u64) -> Self {
        Self(self.0.wrapping_sub(rhs))
    }
}

impl Sub<BlockNum> for BlockNum {
    type Output = u64;
    #[inline]
    fn sub(self, rhs: BlockNum) -> u64 {
        self.0.wrapping_sub(rhs.0)
    }
}

/// Inode number uniquely identifying a file or directory.
///
/// Inode 0 is reserved as NULL, inode 1 is the root directory.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct InodeNum(pub u64);

impl InodeNum {
    pub const NULL: Self = Self(0);
    pub const ROOT: Self = Self(1);
    pub const INVALID: Self = Self(u64::MAX);

    #[inline]
    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_root(self) -> bool {
        self.0 == 1
    }

    #[inline]
    pub const fn is_valid(self) -> bool {
        self.0 != 0 && self.0 != u64::MAX
    }
}

impl fmt::Debug for InodeNum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Inode({})", self.0)
    }
}

impl fmt::Display for InodeNum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Transaction ID for journaling and versioning.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct TxnId(pub u64);

impl TxnId {
    pub const NULL: Self = Self(0);

    #[inline]
    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

impl fmt::Debug for TxnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Txn({})", self.0)
    }
}

/// Snapshot ID for versioning.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct SnapId(pub u64);

impl SnapId {
    pub const NULL: Self = Self(0);
    pub const CURRENT: Self = Self(u64::MAX); // Current (live) version

    #[inline]
    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[inline]
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub const fn is_current(self) -> bool {
        self.0 == u64::MAX
    }
}

impl fmt::Debug for SnapId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_current() {
            write!(f, "Snap(CURRENT)")
        } else {
            write!(f, "Snap({})", self.0)
        }
    }
}

/// Generation number for detecting stale references.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
#[repr(transparent)]
pub struct Generation(pub u64);

impl Generation {
    #[inline]
    pub const fn new(n: u64) -> Self {
        Self(n)
    }

    #[inline]
    pub const fn get(self) -> u64 {
        self.0
    }

    #[inline]
    pub fn next(self) -> Self {
        Self(self.0.wrapping_add(1))
    }
}

// ============================================================================
// File Type and Mode
// ============================================================================

/// File type enumeration.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
#[repr(u8)]
pub enum FileType {
    /// Unknown or uninitialized
    Unknown     = 0,
    /// Regular file
    Regular     = 1,
    /// Directory
    Directory   = 2,
    /// Character device
    CharDevice  = 3,
    /// Block device
    BlockDevice = 4,
    /// Named pipe (FIFO)
    Fifo        = 5,
    /// Unix socket
    Socket      = 6,
    /// Symbolic link
    Symlink     = 7,
    /// HelixFS: Snapshot reference
    Snapshot    = 8,
    /// HelixFS: Clone (CoW copy)
    Clone       = 9,
}

impl FileType {
    /// Create from raw type value
    #[inline]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            1 => Self::Regular,
            2 => Self::Directory,
            3 => Self::CharDevice,
            4 => Self::BlockDevice,
            5 => Self::Fifo,
            6 => Self::Socket,
            7 => Self::Symlink,
            8 => Self::Snapshot,
            9 => Self::Clone,
            _ => Self::Unknown,
        }
    }

    /// Convert to raw value
    #[inline]
    pub const fn to_raw(self) -> u8 {
        self as u8
    }

    /// Check if this is a regular file
    #[inline]
    pub const fn is_regular(self) -> bool {
        matches!(self, Self::Regular)
    }

    /// Check if this is a directory
    #[inline]
    pub const fn is_dir(self) -> bool {
        matches!(self, Self::Directory)
    }

    /// Check if this is a symbolic link
    #[inline]
    pub const fn is_symlink(self) -> bool {
        matches!(self, Self::Symlink)
    }

    /// Check if this is a device
    #[inline]
    pub const fn is_device(self) -> bool {
        matches!(self, Self::CharDevice | Self::BlockDevice)
    }

    /// Get DT_* constant for directory entries
    #[inline]
    pub const fn to_dtype(self) -> u8 {
        self as u8
    }
}

impl Default for FileType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// File permission mode (POSIX-compatible).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct FileMode(pub u32);

impl FileMode {
    // File type bits (high nibble of mode)
    pub const S_IFMT: u32 = 0o170000; // Mask for file type
    pub const S_IFSOCK: u32 = 0o140000; // Socket
    pub const S_IFLNK: u32 = 0o120000; // Symbolic link
    pub const S_IFREG: u32 = 0o100000; // Regular file
    pub const S_IFBLK: u32 = 0o060000; // Block device
    pub const S_IFDIR: u32 = 0o040000; // Directory
    pub const S_IFCHR: u32 = 0o020000; // Character device
    pub const S_IFIFO: u32 = 0o010000; // FIFO

    // Permission bits
    pub const S_ISUID: u32 = 0o004000; // Set UID on execution
    pub const S_ISGID: u32 = 0o002000; // Set GID on execution
    pub const S_ISVTX: u32 = 0o001000; // Sticky bit

    pub const S_IRWXU: u32 = 0o000700; // Owner RWX
    pub const S_IRUSR: u32 = 0o000400; // Owner read
    pub const S_IWUSR: u32 = 0o000200; // Owner write
    pub const S_IXUSR: u32 = 0o000100; // Owner execute

    pub const S_IRWXG: u32 = 0o000070; // Group RWX
    pub const S_IRGRP: u32 = 0o000040; // Group read
    pub const S_IWGRP: u32 = 0o000020; // Group write
    pub const S_IXGRP: u32 = 0o000010; // Group execute

    pub const S_IRWXO: u32 = 0o000007; // Other RWX
    pub const S_IROTH: u32 = 0o000004; // Other read
    pub const S_IWOTH: u32 = 0o000002; // Other write
    pub const S_IXOTH: u32 = 0o000001; // Other execute

    /// Default directory mode (rwxr-xr-x)
    pub const DEFAULT_DIR: Self = Self(Self::S_IFDIR | 0o755);

    /// Default file mode (rw-r--r--)
    pub const DEFAULT_FILE: Self = Self(Self::S_IFREG | 0o644);

    #[inline]
    pub const fn new(mode: u32) -> Self {
        Self(mode)
    }

    #[inline]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Get file type from mode
    #[inline]
    pub const fn file_type(self) -> FileType {
        match self.0 & Self::S_IFMT {
            Self::S_IFREG => FileType::Regular,
            Self::S_IFDIR => FileType::Directory,
            Self::S_IFLNK => FileType::Symlink,
            Self::S_IFCHR => FileType::CharDevice,
            Self::S_IFBLK => FileType::BlockDevice,
            Self::S_IFIFO => FileType::Fifo,
            Self::S_IFSOCK => FileType::Socket,
            _ => FileType::Unknown,
        }
    }

    /// Get permission bits only
    #[inline]
    pub const fn permissions(self) -> u32 {
        self.0 & 0o7777
    }

    /// Check read permission for owner
    #[inline]
    pub const fn owner_read(self) -> bool {
        (self.0 & Self::S_IRUSR) != 0
    }

    /// Check write permission for owner
    #[inline]
    pub const fn owner_write(self) -> bool {
        (self.0 & Self::S_IWUSR) != 0
    }

    /// Check execute permission for owner
    #[inline]
    pub const fn owner_exec(self) -> bool {
        (self.0 & Self::S_IXUSR) != 0
    }

    /// Create mode for regular file with permissions
    #[inline]
    pub const fn regular(perms: u32) -> Self {
        Self(Self::S_IFREG | (perms & 0o7777))
    }

    /// Create mode for directory with permissions
    #[inline]
    pub const fn directory(perms: u32) -> Self {
        Self(Self::S_IFDIR | (perms & 0o7777))
    }

    /// Create mode for symlink
    #[inline]
    pub const fn symlink() -> Self {
        Self(Self::S_IFLNK | 0o777)
    }
}

impl fmt::Debug for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Mode({:#06o})", self.0)
    }
}

impl fmt::Display for FileMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display as ls-style string: drwxrwxrwx
        let ft = match self.0 & Self::S_IFMT {
            Self::S_IFDIR => 'd',
            Self::S_IFLNK => 'l',
            Self::S_IFCHR => 'c',
            Self::S_IFBLK => 'b',
            Self::S_IFIFO => 'p',
            Self::S_IFSOCK => 's',
            _ => '-',
        };

        let perms = self.0;
        write!(
            f,
            "{}{}{}{}{}{}{}{}{}{}",
            ft,
            if perms & Self::S_IRUSR != 0 { 'r' } else { '-' },
            if perms & Self::S_IWUSR != 0 { 'w' } else { '-' },
            if perms & Self::S_IXUSR != 0 {
                if perms & Self::S_ISUID != 0 {
                    's'
                } else {
                    'x'
                }
            } else if perms & Self::S_ISUID != 0 {
                'S'
            } else {
                '-'
            },
            if perms & Self::S_IRGRP != 0 { 'r' } else { '-' },
            if perms & Self::S_IWGRP != 0 { 'w' } else { '-' },
            if perms & Self::S_IXGRP != 0 {
                if perms & Self::S_ISGID != 0 {
                    's'
                } else {
                    'x'
                }
            } else if perms & Self::S_ISGID != 0 {
                'S'
            } else {
                '-'
            },
            if perms & Self::S_IROTH != 0 { 'r' } else { '-' },
            if perms & Self::S_IWOTH != 0 { 'w' } else { '-' },
            if perms & Self::S_IXOTH != 0 {
                if perms & Self::S_ISVTX != 0 {
                    't'
                } else {
                    'x'
                }
            } else if perms & Self::S_ISVTX != 0 {
                'T'
            } else {
                '-'
            },
        )
    }
}

impl BitAnd<u32> for FileMode {
    type Output = u32;
    #[inline]
    fn bitand(self, rhs: u32) -> u32 {
        self.0 & rhs
    }
}

impl BitOr<u32> for FileMode {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: u32) -> Self {
        Self(self.0 | rhs)
    }
}

// ============================================================================
// Inode Flags
// ============================================================================

/// Flags for inode behavior and state.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct InodeFlags(pub u64);

impl InodeFlags {
    /// File is immutable (cannot be modified)
    pub const IMMUTABLE: u64 = 1 << 0;
    /// File is append-only
    pub const APPEND_ONLY: u64 = 1 << 1;
    /// Do not dump (backup) this file
    pub const NO_DUMP: u64 = 1 << 2;
    /// Do not update access time
    pub const NO_ATIME: u64 = 1 << 3;
    /// Synchronous updates
    pub const SYNC: u64 = 1 << 4;
    /// Directory is synchronous
    pub const DIR_SYNC: u64 = 1 << 5;
    /// Compressed file
    pub const COMPRESSED: u64 = 1 << 6;
    /// Encrypted file
    pub const ENCRYPTED: u64 = 1 << 7;
    /// File has inline data (small files stored in inode)
    pub const INLINE_DATA: u64 = 1 << 8;
    /// File uses extent-based allocation
    pub const EXTENTS: u64 = 1 << 9;
    /// File is a snapshot
    pub const SNAPSHOT: u64 = 1 << 10;
    /// File is a clone (CoW copy)
    pub const CLONE: u64 = 1 << 11;
    /// File is sparse (has holes)
    pub const SPARSE: u64 = 1 << 12;
    /// File has extended attributes
    pub const HAS_XATTR: u64 = 1 << 13;
    /// File has ACL
    pub const HAS_ACL: u64 = 1 << 14;
    /// File integrity protection enabled
    pub const INTEGRITY: u64 = 1 << 15;
    /// File is being deleted (pending GC)
    pub const ORPHAN: u64 = 1 << 16;
    /// File is a hardlink target
    pub const HARDLINK: u64 = 1 << 17;
    /// File has been modified since snapshot
    pub const COW_PENDING: u64 = 1 << 18;
    /// File is pinned in cache
    pub const PINNED: u64 = 1 << 19;
    /// File is in transaction
    pub const IN_TXN: u64 = 1 << 20;
    /// File needs fsync
    pub const DIRTY: u64 = 1 << 21;

    pub const EMPTY: Self = Self(0);

    #[inline]
    pub const fn new(flags: u64) -> Self {
        Self(flags)
    }

    #[inline]
    pub const fn get(self) -> u64 {
        self.0
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

    #[inline]
    pub const fn is_immutable(self) -> bool {
        self.contains(Self::IMMUTABLE)
    }

    #[inline]
    pub const fn is_compressed(self) -> bool {
        self.contains(Self::COMPRESSED)
    }

    #[inline]
    pub const fn is_encrypted(self) -> bool {
        self.contains(Self::ENCRYPTED)
    }

    #[inline]
    pub const fn is_snapshot(self) -> bool {
        self.contains(Self::SNAPSHOT)
    }

    #[inline]
    pub const fn is_inline(self) -> bool {
        self.contains(Self::INLINE_DATA)
    }
}

impl fmt::Debug for InodeFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "InodeFlags({:#x})", self.0)
    }
}

impl BitAnd for InodeFlags {
    type Output = Self;
    #[inline]
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl BitOr for InodeFlags {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl Not for InodeFlags {
    type Output = Self;
    #[inline]
    fn not(self) -> Self {
        Self(!self.0)
    }
}

// ============================================================================
// Extent Descriptor
// ============================================================================

/// Flags for extent entries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(transparent)]
pub struct ExtentFlags(pub u16);

impl ExtentFlags {
    /// Extent is unwritten (preallocated but not written)
    pub const UNWRITTEN: u16 = 1 << 0;
    /// Preallocated (alias for UNWRITTEN)
    pub const PREALLOC: u16 = 1 << 0;
    /// Extent data is compressed
    pub const COMPRESSED: u16 = 1 << 1;
    /// Extent data is encrypted
    pub const ENCRYPTED: u16 = 1 << 2;
    /// Extent is shared (CoW, referenced by multiple inodes)
    pub const SHARED: u16 = 1 << 3;
    /// Extent is inline (data follows extent header)
    pub const INLINE: u16 = 1 << 4;
    /// Extent has checksum
    pub const CHECKSUM: u16 = 1 << 5;
    /// Extent is a hole (sparse)
    pub const HOLE: u16 = 1 << 6;

    pub const EMPTY: Self = Self(0);

    #[inline]
    pub const fn new(flags: u16) -> Self {
        Self(flags)
    }

    #[inline]
    pub const fn contains(self, flag: u16) -> bool {
        (self.0 & flag) != 0
    }
}

/// Extent descriptor mapping logical file offset to physical blocks.
///
/// This is the core data structure for file block mapping in HelixFS.
/// Each extent maps a contiguous range of logical blocks to physical blocks.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
#[repr(C, packed)]
pub struct Extent {
    /// Logical block offset within file (in blocks)
    pub logical_start: u64,
    /// Physical block number on disk
    pub physical_start: u64,
    /// Number of blocks in this extent
    pub block_count: u32,
    /// Extent flags
    pub flags: u16,
    /// Checksum of extent metadata
    pub checksum: u16,
}

impl Extent {
    /// Size of extent structure on disk
    pub const SIZE: usize = 24;

    /// Create a new extent
    #[inline]
    pub const fn new(logical_start: u64, physical_start: u64, block_count: u32) -> Self {
        Self {
            logical_start,
            physical_start,
            block_count,
            flags: 0,
            checksum: 0,
        }
    }

    /// Create a hole extent (sparse region)
    #[inline]
    pub const fn hole(logical_start: u64, block_count: u32) -> Self {
        Self {
            logical_start,
            physical_start: 0,
            block_count,
            flags: ExtentFlags::HOLE,
            checksum: 0,
        }
    }

    /// Check if this extent contains the given logical block
    #[inline]
    pub const fn contains_logical(&self, block: u64) -> bool {
        block >= self.logical_start && block < self.logical_start + self.block_count as u64
    }

    /// Get physical block for a logical block within this extent
    #[inline]
    pub const fn logical_to_physical(&self, logical: u64) -> Option<u64> {
        if self.contains_logical(logical) && !self.is_hole() {
            Some(self.physical_start + (logical - self.logical_start))
        } else {
            None
        }
    }

    /// Logical end block (exclusive)
    #[inline]
    pub const fn logical_end(&self) -> u64 {
        self.logical_start + self.block_count as u64
    }

    /// Physical end block (exclusive)
    #[inline]
    pub const fn physical_end(&self) -> u64 {
        self.physical_start + self.block_count as u64
    }

    /// Check if this is a hole
    #[inline]
    pub const fn is_hole(&self) -> bool {
        (self.flags & ExtentFlags::HOLE) != 0
    }

    /// Check if this extent is shared (CoW)
    #[inline]
    pub const fn is_shared(&self) -> bool {
        (self.flags & ExtentFlags::SHARED) != 0
    }

    /// Check if this extent is compressed
    #[inline]
    pub const fn is_compressed(&self) -> bool {
        (self.flags & ExtentFlags::COMPRESSED) != 0
    }

    /// Check if this extent is encrypted
    #[inline]
    pub const fn is_encrypted(&self) -> bool {
        (self.flags & ExtentFlags::ENCRYPTED) != 0
    }

    /// Size in bytes
    #[inline]
    pub const fn size_bytes(&self, block_size: u64) -> u64 {
        self.block_count as u64 * block_size
    }
}

impl fmt::Debug for Extent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy packed fields to avoid unaligned reference
        let ls = self.logical_start;
        let le = self.logical_end();
        let ps = self.physical_start;
        let pe = self.physical_end();
        let fl = self.flags;
        write!(
            f,
            "Extent {{ L{}..{} -> P{}..{}, flags={:#x} }}",
            ls, le, ps, pe, fl
        )
    }
}

// ============================================================================
// Compression and Encryption Types
// ============================================================================

/// Compression algorithm selection.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(u8)]
pub enum CompressionType {
    /// No compression
    #[default]
    None     = 0,
    /// LZ4 - Fast compression
    Lz4      = 1,
    /// LZ4 HC - Higher compression ratio
    Lz4Hc    = 2,
    /// Zstd - Balanced
    Zstd     = 3,
    /// Zstd High - Maximum compression
    ZstdHigh = 4,
}

impl CompressionType {
    #[inline]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            1 => Self::Lz4,
            2 => Self::Lz4Hc,
            3 => Self::Zstd,
            4 => Self::ZstdHigh,
            _ => Self::None,
        }
    }
}

/// Encryption algorithm selection.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(u8)]
pub enum EncryptionType {
    /// No encryption
    #[default]
    None             = 0,
    /// AES-256-GCM (AEAD)
    Aes256Gcm        = 1,
    /// ChaCha20-Poly1305 (AEAD)
    ChaCha20Poly1305 = 2,
    /// AES-256-XTS (disk encryption mode)
    Aes256Xts        = 3,
}

impl EncryptionType {
    #[inline]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            1 => Self::Aes256Gcm,
            2 => Self::ChaCha20Poly1305,
            3 => Self::Aes256Xts,
            _ => Self::None,
        }
    }

    /// Get key size in bytes for this algorithm
    #[inline]
    pub const fn key_size(self) -> usize {
        match self {
            Self::None => 0,
            Self::Aes256Gcm => 32,
            Self::ChaCha20Poly1305 => 32,
            Self::Aes256Xts => 64, // Two 256-bit keys
        }
    }

    /// Get nonce/IV size in bytes
    #[inline]
    pub const fn nonce_size(self) -> usize {
        match self {
            Self::None => 0,
            Self::Aes256Gcm => 12,
            Self::ChaCha20Poly1305 => 12,
            Self::Aes256Xts => 16,
        }
    }

    /// Get authentication tag size
    #[inline]
    pub const fn tag_size(self) -> usize {
        match self {
            Self::None => 0,
            Self::Aes256Gcm => 16,
            Self::ChaCha20Poly1305 => 16,
            Self::Aes256Xts => 0, // XTS doesn't have auth tag
        }
    }
}

/// Hash algorithm for integrity checking.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(u8)]
pub enum HashType {
    /// No hashing
    #[default]
    None     = 0,
    /// XXHash64 - Fast non-cryptographic
    XxHash64 = 1,
    /// CRC32C - Hardware accelerated on modern CPUs
    Crc32c   = 2,
    /// SHA-256 - Cryptographic
    Sha256   = 3,
    /// BLAKE3 - Fast cryptographic
    Blake3   = 4,
}

impl HashType {
    #[inline]
    pub const fn from_raw(v: u8) -> Self {
        match v {
            1 => Self::XxHash64,
            2 => Self::Crc32c,
            3 => Self::Sha256,
            4 => Self::Blake3,
            _ => Self::None,
        }
    }

    /// Get hash output size in bytes
    #[inline]
    pub const fn output_size(self) -> usize {
        match self {
            Self::None => 0,
            Self::XxHash64 => 8,
            Self::Crc32c => 4,
            Self::Sha256 => 32,
            Self::Blake3 => 32,
        }
    }

    /// Check if this is a cryptographic hash
    #[inline]
    pub const fn is_cryptographic(self) -> bool {
        matches!(self, Self::Sha256 | Self::Blake3)
    }
}

// ============================================================================
// Allocation Hints
// ============================================================================

/// Hints for the block allocator about expected access patterns.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[repr(u8)]
pub enum AllocationHint {
    /// No specific hint
    #[default]
    None       = 0,
    /// Sequential access expected - allocate contiguously
    Sequential = 1,
    /// Random access expected - spread allocation
    Random     = 2,
    /// Hot data - allocate on fast storage
    Hot        = 3,
    /// Cold data - allocate on slow storage
    Cold       = 4,
    /// Temporary file - may be deleted soon
    Temporary  = 5,
    /// Streaming media - large sequential reads
    Streaming  = 6,
    /// Database workload - mixed access
    Database   = 7,
    /// Metadata - prioritize reliability
    Metadata   = 8,
}

// ============================================================================
// 256-bit Hash Value
// ============================================================================

/// 256-bit hash value used for Merkle tree and integrity checking.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
#[repr(C)]
pub struct Hash256 {
    pub bytes: [u8; 32],
}

impl Hash256 {
    pub const ZERO: Self = Self { bytes: [0; 32] };
    pub const SIZE: usize = 32;

    /// Create from bytes
    #[inline]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    /// Create from slice (must be exactly 32 bytes)
    #[inline]
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() != 32 {
            return None;
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(slice);
        Some(Self { bytes })
    }

    /// Check if hash is all zeros
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.bytes.iter().all(|&b| b == 0)
    }

    /// XOR with another hash
    #[inline]
    pub fn xor(&self, other: &Self) -> Self {
        let mut result = [0u8; 32];
        for ((r, a), b) in result.iter_mut().zip(self.bytes.iter()).zip(other.bytes.iter()) {
            *r = a ^ b;
        }
        Self { bytes: result }
    }

    /// Get as u64 array (for faster comparison)
    #[inline]
    pub fn as_u64_array(&self) -> &[u64; 4] {
        // SAFETY: [u8; 32] has same layout as [u64; 4] on all platforms
        unsafe { &*(self.bytes.as_ptr() as *const [u64; 4]) }
    }
}

impl fmt::Debug for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash256(")?;
        for byte in &self.bytes[0..4] {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, "...")?;
        for byte in &self.bytes[28..32] {
            write!(f, "{:02x}", byte)?;
        }
        write!(f, ")")
    }
}

impl fmt::Display for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.bytes {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}

// ============================================================================
// File Statistics (stat-like)
// ============================================================================

/// File statistics structure (similar to POSIX stat).
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FileStat {
    /// Inode number
    pub ino: InodeNum,
    /// File mode (type + permissions)
    pub mode: FileMode,
    /// Number of hard links
    pub nlink: u32,
    /// Owner user ID
    pub uid: u32,
    /// Owner group ID
    pub gid: u32,
    /// Device ID (for device files)
    pub rdev: u64,
    /// File size in bytes
    pub size: u64,
    /// Block size for filesystem I/O
    pub blksize: u32,
    /// Number of 512-byte blocks allocated
    pub blocks: u64,
    /// Access time (nanoseconds since epoch)
    pub atime: u64,
    /// Modification time (nanoseconds since epoch)
    pub mtime: u64,
    /// Status change time (nanoseconds since epoch)
    pub ctime: u64,
    /// Creation time (nanoseconds since epoch)
    pub crtime: u64,
    /// Inode generation
    pub generation: Generation,
    /// Inode flags
    pub flags: InodeFlags,
}

impl FileStat {
    /// Create empty stat structure
    #[inline]
    pub const fn empty() -> Self {
        Self {
            ino: InodeNum::NULL,
            mode: FileMode(0),
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            size: 0,
            blksize: 4096,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            crtime: 0,
            generation: Generation(0),
            flags: InodeFlags::EMPTY,
        }
    }

    /// Check if this is a directory
    #[inline]
    pub const fn is_dir(&self) -> bool {
        self.mode.file_type().is_dir()
    }

    /// Check if this is a regular file
    #[inline]
    pub const fn is_file(&self) -> bool {
        self.mode.file_type().is_regular()
    }

    /// Check if this is a symlink
    #[inline]
    pub const fn is_symlink(&self) -> bool {
        self.mode.file_type().is_symlink()
    }
}

// ============================================================================
// Directory Entry
// ============================================================================

/// Maximum inline name length in directory entry
pub const DIRENT_INLINE_NAME_LEN: usize = 248;

/// Directory entry structure.
#[derive(Clone)]
pub struct DirEntry {
    /// Inode number of target
    pub ino: InodeNum,
    /// File type
    pub file_type: FileType,
    /// File name
    pub name: DirEntryName,
}

/// Directory entry name storage.
#[derive(Clone)]
pub enum DirEntryName {
    /// Inline name (up to 248 bytes)
    Inline {
        len: u8,
        bytes: [u8; DIRENT_INLINE_NAME_LEN],
    },
    #[cfg(feature = "alloc")]
    /// Heap-allocated name for longer names
    Heap(alloc_crate::vec::Vec<u8>),
}

impl DirEntryName {
    /// Create from slice
    pub fn from_slice(s: &[u8]) -> Self {
        if s.len() <= DIRENT_INLINE_NAME_LEN {
            let mut bytes = [0u8; DIRENT_INLINE_NAME_LEN];
            bytes[..s.len()].copy_from_slice(s);
            Self::Inline {
                len: s.len() as u8,
                bytes,
            }
        } else {
            #[cfg(feature = "alloc")]
            {
                Self::Heap(alloc_crate::vec::Vec::from(s))
            }
            #[cfg(not(feature = "alloc"))]
            {
                // Truncate if no alloc
                let mut bytes = [0u8; DIRENT_INLINE_NAME_LEN];
                bytes.copy_from_slice(&s[..DIRENT_INLINE_NAME_LEN]);
                Self::Inline {
                    len: DIRENT_INLINE_NAME_LEN as u8,
                    bytes,
                }
            }
        }
    }

    /// Get name as slice
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Inline { len, bytes } => &bytes[..*len as usize],
            #[cfg(feature = "alloc")]
            Self::Heap(v) => v.as_slice(),
        }
    }

    /// Get name length
    pub fn len(&self) -> usize {
        match self {
            Self::Inline { len, .. } => *len as usize,
            #[cfg(feature = "alloc")]
            Self::Heap(v) => v.len(),
        }
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl DirEntry {
    /// Create a new directory entry
    pub fn new(ino: InodeNum, file_type: FileType, name: &[u8]) -> Self {
        Self {
            ino,
            file_type,
            name: DirEntryName::from_slice(name),
        }
    }

    /// Get name as byte slice
    pub fn name(&self) -> &[u8] {
        self.name.as_slice()
    }

    /// Check if this is "." entry
    pub fn is_dot(&self) -> bool {
        self.name.as_slice() == b"."
    }

    /// Check if this is ".." entry
    pub fn is_dotdot(&self) -> bool {
        self.name.as_slice() == b".."
    }
}

impl fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Try to display name as UTF-8
        let name = core::str::from_utf8(self.name.as_slice()).unwrap_or("<invalid utf8>");
        write!(
            f,
            "DirEntry {{ ino: {}, type: {:?}, name: {:?} }}",
            self.ino.0, self.file_type, name
        )
    }
}

// ============================================================================
// Filesystem Statistics
// ============================================================================

/// Filesystem-wide statistics (statfs-like).
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct FsStats {
    /// Total blocks in filesystem
    pub total_blocks: u64,
    /// Free blocks available
    pub free_blocks: u64,
    /// Available blocks (may differ from free due to reserved)
    pub avail_blocks: u64,
    /// Total inodes
    pub total_inodes: u64,
    /// Free inodes
    pub free_inodes: u64,
    /// Block size in bytes
    pub block_size: u32,
    /// Maximum filename length
    pub max_name_len: u32,
    /// Filesystem magic number
    pub magic: u64,
    /// Filesystem flags
    pub flags: u64,
    /// Number of snapshots
    pub snapshot_count: u32,
    /// Compression ratio (fixed-point, 1.0 = 1000)
    pub compression_ratio: u32,
}

impl FsStats {
    /// Get total size in bytes
    #[inline]
    pub const fn total_bytes(&self) -> u64 {
        self.total_blocks * self.block_size as u64
    }

    /// Get free size in bytes
    #[inline]
    pub const fn free_bytes(&self) -> u64 {
        self.free_blocks * self.block_size as u64
    }

    /// Get used size in bytes
    #[inline]
    pub const fn used_bytes(&self) -> u64 {
        (self.total_blocks - self.free_blocks) * self.block_size as u64
    }

    /// Get usage percentage (0-100)
    #[inline]
    pub const fn usage_percent(&self) -> u32 {
        if self.total_blocks == 0 {
            0
        } else {
            ((self.total_blocks - self.free_blocks) * 100 / self.total_blocks) as u32
        }
    }
}
