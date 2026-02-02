//! Directory entry management.
//!
//! Provides efficient directory indexing with hash-based lookup
//! for O(1) name resolution and sorted iteration.

use crate::core::error::{HfsError, HfsResult};
use crate::core::hash::*;
use crate::core::types::*;

// ============================================================================
// Constants
// ============================================================================

/// Maximum filename length
pub const MAX_NAME_LEN: usize = 255;

/// Maximum inline directory entries in inode
pub const MAX_INLINE_ENTRIES: usize = 4;

/// Directory hash table load factor threshold
pub const HASH_LOAD_FACTOR: f32 = 0.75;

/// Minimum hash table size
pub const MIN_HASH_SIZE: usize = 16;

/// Maximum hash table size
pub const MAX_HASH_SIZE: usize = 65536;

/// Entries per directory block
pub const ENTRIES_PER_BLOCK: usize = 32;

// ============================================================================
// Directory Entry Types
// ============================================================================

/// File type in directory entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DirFileType {
    /// Unknown type
    Unknown   = 0,
    /// Regular file
    Regular   = 1,
    /// Directory
    Directory = 2,
    /// Symbolic link
    Symlink   = 3,
    /// Block device
    BlockDev  = 4,
    /// Character device
    CharDev   = 5,
    /// Named pipe (FIFO)
    Fifo      = 6,
    /// Socket
    Socket    = 7,
    /// Whiteout (for overlay filesystems)
    Whiteout  = 8,
}

impl DirFileType {
    /// From raw value
    pub fn from_raw(raw: u8) -> Self {
        match raw {
            1 => Self::Regular,
            2 => Self::Directory,
            3 => Self::Symlink,
            4 => Self::BlockDev,
            5 => Self::CharDev,
            6 => Self::Fifo,
            7 => Self::Socket,
            8 => Self::Whiteout,
            _ => Self::Unknown,
        }
    }

    /// To FileType
    pub fn to_file_type(self) -> FileType {
        match self {
            Self::Regular => FileType::Regular,
            Self::Directory => FileType::Directory,
            Self::Symlink => FileType::Symlink,
            Self::BlockDev => FileType::BlockDevice,
            Self::CharDev => FileType::CharDevice,
            Self::Fifo => FileType::Fifo,
            Self::Socket => FileType::Socket,
            _ => FileType::Regular,
        }
    }

    /// From FileType
    pub fn from_file_type(ft: FileType) -> Self {
        match ft {
            FileType::Regular => Self::Regular,
            FileType::Directory => Self::Directory,
            FileType::Symlink => Self::Symlink,
            FileType::BlockDevice => Self::BlockDev,
            FileType::CharDevice => Self::CharDev,
            FileType::Fifo => Self::Fifo,
            FileType::Socket => Self::Socket,
            FileType::Unknown | FileType::Snapshot | FileType::Clone => Self::Unknown,
        }
    }
}

impl Default for DirFileType {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================================
// Directory Entry (On-disk)
// ============================================================================

/// On-disk directory entry.
///
/// Size: 128 bytes (fits 32 per 4KB block)
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct DirEntryRaw {
    /// Inode number (0 = deleted)
    pub ino: u64,
    /// Name hash (for fast lookup)
    pub name_hash: u64,
    /// Entry generation
    pub generation: u32,
    /// File type
    pub file_type: u8,
    /// Name length
    pub name_len: u8,
    /// Flags
    pub flags: u16,
    /// Next entry with same hash (for collision chain)
    pub next_hash: u32,
    /// Name (null-terminated)
    pub name: [u8; 100],
}

impl DirEntryRaw {
    /// Entry size
    pub const SIZE: usize = 128;

    /// Create empty entry
    pub const fn empty() -> Self {
        Self {
            ino: 0,
            name_hash: 0,
            generation: 0,
            file_type: 0,
            name_len: 0,
            flags: 0,
            next_hash: 0,
            name: [0; 100],
        }
    }

    /// Create new entry
    pub fn new(ino: u64, name: &[u8], file_type: DirFileType) -> HfsResult<Self> {
        if name.len() > 100 {
            return Err(HfsError::NameTooLong);
        }

        let mut entry = Self::empty();
        entry.ino = ino;
        entry.file_type = file_type as u8;
        entry.name_len = name.len() as u8;
        entry.name[..name.len()].copy_from_slice(name);
        entry.name_hash = hash_name(name);

        Ok(entry)
    }

    /// Check if deleted
    #[inline]
    pub fn is_deleted(&self) -> bool {
        self.ino == 0
    }

    /// Check if valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.ino != 0 && self.name_len > 0
    }

    /// Get name as slice
    pub fn name(&self) -> &[u8] {
        let len = (self.name_len as usize).min(100);
        &self.name[..len]
    }

    /// Get file type
    pub fn file_type(&self) -> DirFileType {
        DirFileType::from_raw(self.file_type)
    }

    /// Check if entry is "."
    pub fn is_dot(&self) -> bool {
        self.name_len == 1 && self.name[0] == b'.'
    }

    /// Check if entry is ".."
    pub fn is_dotdot(&self) -> bool {
        self.name_len == 2 && self.name[0] == b'.' && self.name[1] == b'.'
    }

    /// Mark as deleted
    pub fn delete(&mut self) {
        self.ino = 0;
        self.flags |= 1; // DELETED flag
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<DirEntryRaw>() == 128);

// ============================================================================
// Inline Directory Entry (Small)
// ============================================================================

/// Small inline entry for small directories.
///
/// Size: 32 bytes
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct DirEntrySmall {
    /// Inode number
    pub ino: u64,
    /// File type
    pub file_type: u8,
    /// Name length
    pub name_len: u8,
    /// Name (short)
    pub name: [u8; 22],
}

impl DirEntrySmall {
    /// Entry size
    pub const SIZE: usize = 32;

    /// Maximum name length
    pub const MAX_NAME: usize = 22;

    /// Create empty
    pub const fn empty() -> Self {
        Self {
            ino: 0,
            file_type: 0,
            name_len: 0,
            name: [0; 22],
        }
    }

    /// Create from raw entry (if name fits)
    pub fn from_raw(raw: &DirEntryRaw) -> Option<Self> {
        if raw.name_len as usize > Self::MAX_NAME {
            return None;
        }

        let mut entry = Self::empty();
        entry.ino = raw.ino;
        entry.file_type = raw.file_type;
        entry.name_len = raw.name_len;

        let len = raw.name_len as usize;
        entry.name[..len].copy_from_slice(&raw.name[..len]);

        Some(entry)
    }

    /// To raw entry
    pub fn to_raw(&self) -> DirEntryRaw {
        let mut raw = DirEntryRaw::empty();
        raw.ino = self.ino;
        raw.file_type = self.file_type;
        raw.name_len = self.name_len;

        let len = self.name_len as usize;
        raw.name[..len].copy_from_slice(&self.name[..len]);
        raw.name_hash = hash_name(&self.name[..len]);

        raw
    }

    /// Check if valid
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.ino != 0 && self.name_len > 0
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<DirEntrySmall>() == 32);

// ============================================================================
// Directory Block
// ============================================================================

/// Directory block header.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct DirBlockHeader {
    /// Magic number
    pub magic: u32,
    /// Block type
    pub block_type: u8,
    /// Entry count
    pub count: u8,
    /// Flags
    pub flags: u16,
    /// Next block in chain
    pub next: u64,
    /// Previous block
    pub prev: u64,
    /// Parent inode
    pub parent_ino: u64,
    /// Free space offset
    pub free_offset: u16,
    /// Padding
    pub _pad: [u8; 6],
    /// Checksum
    pub checksum: u32,
    /// Reserved
    pub _reserved: [u8; 12],
}

impl DirBlockHeader {
    /// Header size
    pub const SIZE: usize = 64;

    /// Magic number
    pub const MAGIC: u32 = 0x44495242; // "DIRB"

    /// Create new header
    pub fn new(parent_ino: u64) -> Self {
        Self {
            magic: Self::MAGIC,
            block_type: 0,
            count: 0,
            flags: 0,
            next: 0,
            prev: 0,
            parent_ino,
            free_offset: Self::SIZE as u16,
            _pad: [0; 6],
            checksum: 0,
            _reserved: [0; 12],
        }
    }

    /// Validate header
    pub fn validate(&self) -> HfsResult<()> {
        if self.magic != Self::MAGIC {
            return Err(HfsError::DirCorruption);
        }
        Ok(())
    }
}

/// Directory block (entries block).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct DirBlock {
    /// Header
    pub header: DirBlockHeader,
    /// Entries (up to 31 per block with header)
    pub entries: [DirEntryRaw; 31],
    /// Trailing padding
    pub _pad: [u8; 32],
}

impl DirBlock {
    /// Size in bytes
    pub const SIZE: usize = 4096;

    /// Maximum entries
    pub const MAX_ENTRIES: usize = 31;

    /// Create new block
    pub fn new(parent_ino: u64) -> Self {
        Self {
            header: DirBlockHeader::new(parent_ino),
            entries: [DirEntryRaw::empty(); 31],
            _pad: [0; 32],
        }
    }

    /// Find entry by name
    pub fn find(&self, name: &[u8]) -> Option<(usize, &DirEntryRaw)> {
        let hash = hash_name(name);

        for (i, entry) in self.entries.iter().enumerate() {
            if entry.is_valid() && entry.name_hash == hash && entry.name() == name {
                return Some((i, entry));
            }
        }
        None
    }

    /// Find free slot
    pub fn find_free(&self) -> Option<usize> {
        for (i, entry) in self.entries.iter().enumerate() {
            if !entry.is_valid() {
                return Some(i);
            }
        }
        None
    }

    /// Insert entry
    pub fn insert(&mut self, entry: DirEntryRaw) -> HfsResult<usize> {
        let slot = self.find_free().ok_or(HfsError::DirectoryFull)?;
        self.entries[slot] = entry;
        self.header.count += 1;
        Ok(slot)
    }

    /// Remove entry at index
    pub fn remove(&mut self, index: usize) -> HfsResult<DirEntryRaw> {
        if index >= Self::MAX_ENTRIES {
            return Err(HfsError::InvalidParameter);
        }

        let entry = self.entries[index];
        if !entry.is_valid() {
            return Err(HfsError::NotFound);
        }

        self.entries[index].delete();
        self.header.count = self.header.count.saturating_sub(1);

        Ok(entry)
    }

    /// Check if full
    #[inline]
    pub fn is_full(&self) -> bool {
        self.header.count as usize >= Self::MAX_ENTRIES
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.header.count == 0
    }

    /// Get entry count
    #[inline]
    pub fn count(&self) -> usize {
        self.header.count as usize
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<DirBlock>() <= 4096);

// ============================================================================
// Hash Table Directory
// ============================================================================

/// Directory hash table bucket.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct HashBucket {
    /// First entry block
    pub block: u64,
    /// Entry index in block
    pub index: u16,
    /// Chain length
    pub chain_len: u16,
    /// Reserved
    pub _reserved: u32,
}

impl HashBucket {
    /// Size in bytes
    pub const SIZE: usize = 16;

    /// Create empty bucket
    pub const fn empty() -> Self {
        Self {
            block: 0,
            index: 0,
            chain_len: 0,
            _reserved: 0,
        }
    }

    /// Check if empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.block == 0
    }

    /// Create with entry
    pub fn with_entry(block: u64, index: u16) -> Self {
        Self {
            block,
            index,
            chain_len: 1,
            _reserved: 0,
        }
    }
}

/// Directory hash table header.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct DirHashHeader {
    /// Magic number
    pub magic: u32,
    /// Version
    pub version: u8,
    /// Log2 of table size
    pub size_log2: u8,
    /// Flags
    pub flags: u16,
    /// Entry count
    pub count: u32,
    /// Max chain length
    pub max_chain: u16,
    /// Average chain length (fixed point)
    pub avg_chain: u16,
    /// Parent inode
    pub parent_ino: u64,
    /// Checksum
    pub checksum: u32,
    /// Reserved
    pub _reserved: [u8; 8],
}

impl DirHashHeader {
    /// Header size
    pub const SIZE: usize = 40;

    /// Magic number
    pub const MAGIC: u32 = 0x44485348; // "DHSH"

    /// Create new header
    pub fn new(size_log2: u8, parent_ino: u64) -> Self {
        Self {
            magic: Self::MAGIC,
            version: 1,
            size_log2,
            flags: 0,
            count: 0,
            max_chain: 0,
            avg_chain: 0,
            parent_ino,
            checksum: 0,
            _reserved: [0; 8],
        }
    }

    /// Get table size
    #[inline]
    pub fn table_size(&self) -> usize {
        1 << self.size_log2
    }

    /// Calculate load factor
    pub fn load_factor(&self) -> f32 {
        let size = self.table_size();
        if size == 0 {
            return 0.0;
        }
        self.count as f32 / size as f32
    }

    /// Check if needs resize
    pub fn needs_resize(&self) -> bool {
        self.load_factor() > HASH_LOAD_FACTOR
    }
}

/// Directory hash table block.
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct DirHashBlock {
    /// Header
    pub header: DirHashHeader,
    /// Buckets (252 per block)
    pub buckets: [HashBucket; 252],
    /// Padding
    pub _pad: [u8; 24],
}

impl DirHashBlock {
    /// Size in bytes
    pub const SIZE: usize = 4096;

    /// Buckets per block
    pub const BUCKETS_PER_BLOCK: usize = 252;

    /// Create new hash block
    pub fn new(size_log2: u8, parent_ino: u64) -> Self {
        Self {
            header: DirHashHeader::new(size_log2, parent_ino),
            buckets: [HashBucket::empty(); 252],
            _pad: [0; 24],
        }
    }

    /// Get bucket for hash
    pub fn bucket(&self, hash: u64) -> &HashBucket {
        let idx = (hash as usize) & ((1 << self.header.size_log2) - 1);
        &self.buckets[idx % Self::BUCKETS_PER_BLOCK]
    }

    /// Get mutable bucket for hash
    pub fn bucket_mut(&mut self, hash: u64) -> &mut HashBucket {
        let idx = (hash as usize) & ((1 << self.header.size_log2) - 1);
        &mut self.buckets[idx % Self::BUCKETS_PER_BLOCK]
    }
}

// Verify size
const _: () = assert!(core::mem::size_of::<DirHashBlock>() <= 4096);

// ============================================================================
// Directory Index Entry
// ============================================================================

/// Directory index entry (for B-tree indexing).
///
/// Key: name hash + inode number for uniqueness
/// Value: block + offset
#[derive(Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct DirIndexEntry {
    /// Name hash
    pub name_hash: u64,
    /// Entry block
    pub block: u64,
    /// Entry offset within block
    pub offset: u16,
    /// Name length
    pub name_len: u8,
    /// File type
    pub file_type: u8,
    /// Reserved
    pub _reserved: u32,
}

impl DirIndexEntry {
    /// Size in bytes
    pub const SIZE: usize = 24;

    /// Create empty entry
    pub const fn empty() -> Self {
        Self {
            name_hash: 0,
            block: 0,
            offset: 0,
            name_len: 0,
            file_type: 0,
            _reserved: 0,
        }
    }

    /// Create entry
    pub fn new(name: &[u8], block: u64, offset: u16, file_type: DirFileType) -> Self {
        Self {
            name_hash: hash_name(name),
            block,
            offset,
            name_len: name.len().min(255) as u8,
            file_type: file_type as u8,
            _reserved: 0,
        }
    }
}

// ============================================================================
// Directory Operations Interface
// ============================================================================

/// Directory operation types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirOpType {
    /// Lookup entry by name
    Lookup,
    /// Create new entry
    Create,
    /// Remove entry
    Remove,
    /// Rename entry
    Rename,
    /// Read directory entries
    ReadDir,
    /// Get entry by index
    GetByIndex,
}

/// Directory operation result.
#[derive(Clone, Copy, Debug)]
pub struct DirOpResult {
    /// Inode number (for lookup/create)
    pub ino: u64,
    /// Entry generation
    pub generation: u32,
    /// File type
    pub file_type: DirFileType,
    /// Number of entries (for readdir)
    pub count: u32,
    /// More entries available
    pub more: bool,
}

impl DirOpResult {
    /// Empty result
    pub const fn empty() -> Self {
        Self {
            ino: 0,
            generation: 0,
            file_type: DirFileType::Unknown,
            count: 0,
            more: false,
        }
    }

    /// Result with inode
    pub fn with_ino(ino: u64, file_type: DirFileType) -> Self {
        Self {
            ino,
            generation: 0,
            file_type,
            count: 0,
            more: false,
        }
    }
}

/// Directory statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct DirStats {
    /// Number of entries
    pub entry_count: u64,
    /// Number of blocks used
    pub block_count: u64,
    /// Number of hash collisions
    pub hash_collisions: u64,
    /// Maximum chain length
    pub max_chain: u32,
    /// Has hash table
    pub has_hash: bool,
    /// Hash table size
    pub hash_size: u32,
}

// ============================================================================
// Directory State
// ============================================================================

/// Directory format type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirFormat {
    /// Inline (small directory, in inode)
    Inline,
    /// Linear (sequential blocks)
    Linear,
    /// Hashed (hash table + blocks)
    Hashed,
    /// B-tree indexed (large directories)
    BTree,
}

/// Directory runtime state.
#[derive(Clone, Debug)]
pub struct DirState {
    /// Parent inode
    pub parent_ino: u64,
    /// Directory inode
    pub ino: u64,
    /// Current format
    pub format: DirFormat,
    /// Entry count
    pub count: u64,
    /// Hash table block (if hashed)
    pub hash_block: Option<BlockNum>,
    /// First entry block (if not inline)
    pub first_block: Option<BlockNum>,
    /// B-tree root (if btree format)
    pub btree_root: Option<BlockNum>,
    /// Statistics
    pub stats: DirStats,
}

impl DirState {
    /// Create new state
    pub fn new(ino: u64, parent_ino: u64) -> Self {
        Self {
            parent_ino,
            ino,
            format: DirFormat::Inline,
            count: 0,
            hash_block: None,
            first_block: None,
            btree_root: None,
            stats: DirStats::default(),
        }
    }

    /// Check if should upgrade format
    pub fn should_upgrade(&self) -> bool {
        match self.format {
            DirFormat::Inline if self.count > MAX_INLINE_ENTRIES as u64 => true,
            DirFormat::Linear if self.count > 128 => true,
            DirFormat::Hashed if self.count > 4096 => true,
            _ => false,
        }
    }

    /// Get next format after upgrade
    pub fn next_format(&self) -> DirFormat {
        match self.format {
            DirFormat::Inline => DirFormat::Linear,
            DirFormat::Linear => DirFormat::Hashed,
            DirFormat::Hashed => DirFormat::BTree,
            DirFormat::BTree => DirFormat::BTree,
        }
    }

    /// Is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        // Note: . and .. don't count
        self.count <= 2
    }
}

// ============================================================================
// Name Hash Function
// ============================================================================

/// Hash a filename for directory lookup.
pub fn hash_name(name: &[u8]) -> u64 {
    // Use XXHash64 for fast hashing
    let mut hasher = XxHash64::with_seed(0x4448_4653); // "DHFS"
    hasher.write(name);
    hasher.finish()
}

/// Hash with case folding (for case-insensitive lookup).
pub fn hash_name_ci(name: &[u8]) -> u64 {
    let mut hasher = XxHash64::with_seed(0x4448_4653);

    for &b in name {
        // Simple ASCII case folding
        let c = if b >= b'A' && b <= b'Z' { b + 32 } else { b };
        hasher.write(&[c]);
    }

    hasher.finish()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dir_file_type() {
        assert_eq!(DirFileType::from_raw(1), DirFileType::Regular);
        assert_eq!(DirFileType::from_raw(2), DirFileType::Directory);
        assert_eq!(DirFileType::from_raw(99), DirFileType::Unknown);
    }

    #[test]
    fn test_dir_entry_raw() {
        let entry = DirEntryRaw::new(100, b"test.txt", DirFileType::Regular).unwrap();

        assert!(entry.is_valid());
        assert!(!entry.is_deleted());
        assert_eq!(entry.ino, 100);
        assert_eq!(entry.name(), b"test.txt");
        assert_eq!(entry.file_type(), DirFileType::Regular);
    }

    #[test]
    fn test_dir_entry_dot() {
        let dot = DirEntryRaw::new(1, b".", DirFileType::Directory).unwrap();
        let dotdot = DirEntryRaw::new(1, b"..", DirFileType::Directory).unwrap();

        assert!(dot.is_dot());
        assert!(!dot.is_dotdot());
        assert!(dotdot.is_dotdot());
    }

    #[test]
    fn test_dir_entry_small() {
        let raw = DirEntryRaw::new(100, b"short", DirFileType::Regular).unwrap();
        let small = DirEntrySmall::from_raw(&raw).unwrap();

        assert!(small.is_valid());
        assert_eq!(small.ino, 100);
        assert_eq!(small.name_len, 5);
    }

    #[test]
    fn test_dir_block() {
        let mut block = DirBlock::new(1);

        assert!(block.is_empty());
        assert_eq!(block.count(), 0);

        let entry = DirEntryRaw::new(100, b"file1.txt", DirFileType::Regular).unwrap();
        let idx = block.insert(entry).unwrap();

        assert_eq!(block.count(), 1);

        let found = block.find(b"file1.txt");
        assert!(found.is_some());
        assert_eq!(found.unwrap().1.ino, 100);

        block.remove(idx).unwrap();
        assert!(block.is_empty());
    }

    #[test]
    fn test_hash_name() {
        let h1 = hash_name(b"test.txt");
        let h2 = hash_name(b"test.txt");
        let h3 = hash_name(b"test.doc");

        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_hash_name_ci() {
        let h1 = hash_name_ci(b"Test.TXT");
        let h2 = hash_name_ci(b"test.txt");

        assert_eq!(h1, h2);
    }

    #[test]
    fn test_dir_state() {
        let state = DirState::new(100, 1);

        assert_eq!(state.format, DirFormat::Inline);
        assert!(state.is_empty());
        assert!(!state.should_upgrade());
    }

    #[test]
    fn test_dir_state_upgrade() {
        let mut state = DirState::new(100, 1);
        state.count = 10;

        assert!(state.should_upgrade());
        assert_eq!(state.next_format(), DirFormat::Linear);
    }
}
