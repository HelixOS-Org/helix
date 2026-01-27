//! Directory Operations
//!
//! High-level directory operations: mkdir, rmdir, readdir.

use crate::core::error::{HfsError, HfsResult};
use super::InodeType;

// ============================================================================
// Directory Entry
// ============================================================================

/// Maximum directory entries in a single readdir
pub const MAX_READDIR_ENTRIES: usize = 256;

/// Directory entry type (d_type).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DirEntryType {
    /// Unknown
    Unknown = 0,
    /// FIFO
    Fifo = 1,
    /// Character device
    Char = 2,
    /// Directory
    Dir = 4,
    /// Block device
    Block = 6,
    /// Regular file
    Regular = 8,
    /// Symbolic link
    Link = 10,
    /// Socket
    Socket = 12,
    /// Whiteout
    Whiteout = 14,
}

impl DirEntryType {
    /// From inode type
    pub fn from_inode_type(typ: InodeType) -> Self {
        match typ {
            InodeType::Regular => Self::Regular,
            InodeType::Directory => Self::Dir,
            InodeType::CharDevice => Self::Char,
            InodeType::BlockDevice => Self::Block,
            InodeType::Fifo => Self::Fifo,
            InodeType::Socket => Self::Socket,
            InodeType::Symlink => Self::Link,
        }
    }
    
    /// From mode
    pub fn from_mode(mode: u16) -> Self {
        match InodeType::from_mode(mode) {
            Some(typ) => Self::from_inode_type(typ),
            None => Self::Unknown,
        }
    }
    
    /// Is directory
    pub fn is_dir(self) -> bool {
        self == Self::Dir
    }
    
    /// Is regular file
    pub fn is_file(self) -> bool {
        self == Self::Regular
    }
    
    /// Is symlink
    pub fn is_symlink(self) -> bool {
        self == Self::Link
    }
}

impl Default for DirEntryType {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Directory entry.
#[derive(Clone, Copy)]
pub struct DirEntry {
    /// Inode number
    pub ino: u64,
    /// Entry type
    pub d_type: DirEntryType,
    /// Entry name
    pub name: [u8; 256],
    /// Name length
    pub name_len: u8,
    /// Offset (cookie for next entry)
    pub offset: u64,
}

impl DirEntry {
    /// Create new directory entry
    pub fn new(ino: u64, d_type: DirEntryType, name: &[u8], offset: u64) -> Self {
        let mut entry = Self {
            ino,
            d_type,
            name: [0; 256],
            name_len: 0,
            offset,
        };
        let len = core::cmp::min(name.len(), 255);
        entry.name[..len].copy_from_slice(&name[..len]);
        entry.name_len = len as u8;
        entry
    }
    
    /// Create "." entry
    pub fn dot(ino: u64, offset: u64) -> Self {
        Self::new(ino, DirEntryType::Dir, b".", offset)
    }
    
    /// Create ".." entry
    pub fn dotdot(parent_ino: u64, offset: u64) -> Self {
        Self::new(parent_ino, DirEntryType::Dir, b"..", offset)
    }
    
    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
    
    /// Is dot entry
    pub fn is_dot(&self) -> bool {
        self.name_len == 1 && self.name[0] == b'.'
    }
    
    /// Is dotdot entry
    pub fn is_dotdot(&self) -> bool {
        self.name_len == 2 && self.name[0] == b'.' && self.name[1] == b'.'
    }
    
    /// Is special entry (. or ..)
    pub fn is_special(&self) -> bool {
        self.is_dot() || self.is_dotdot()
    }
}

impl Default for DirEntry {
    fn default() -> Self {
        Self {
            ino: 0,
            d_type: DirEntryType::Unknown,
            name: [0; 256],
            name_len: 0,
            offset: 0,
        }
    }
}

// ============================================================================
// Readdir Context
// ============================================================================

/// Readdir result.
pub struct ReaddirResult {
    /// Entries
    pub entries: [DirEntry; MAX_READDIR_ENTRIES],
    /// Number of entries
    pub count: usize,
    /// End of directory reached
    pub eof: bool,
    /// Next offset
    pub next_offset: u64,
}

impl ReaddirResult {
    /// Create new result
    pub fn new() -> Self {
        Self {
            entries: [DirEntry::default(); MAX_READDIR_ENTRIES],
            count: 0,
            eof: false,
            next_offset: 0,
        }
    }
    
    /// Add entry
    pub fn add(&mut self, entry: DirEntry) -> bool {
        if self.count >= MAX_READDIR_ENTRIES {
            return false;
        }
        self.entries[self.count] = entry;
        self.count += 1;
        true
    }
    
    /// Is full
    pub fn is_full(&self) -> bool {
        self.count >= MAX_READDIR_ENTRIES
    }
    
    /// Iterate entries
    pub fn iter(&self) -> impl Iterator<Item = &DirEntry> {
        self.entries[..self.count].iter()
    }
}

impl Default for ReaddirResult {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Directory Operations
// ============================================================================

/// Mkdir parameters.
#[derive(Clone, Copy)]
pub struct MkdirParams {
    /// Parent inode
    pub parent_ino: u64,
    /// Directory name
    pub name: [u8; 256],
    /// Name length
    pub name_len: u8,
    /// Mode (permissions)
    pub mode: u16,
    /// Owner UID
    pub uid: u32,
    /// Owner GID
    pub gid: u32,
}

impl MkdirParams {
    /// Create new mkdir params
    pub fn new(parent_ino: u64, name: &[u8], mode: u16, uid: u32, gid: u32) -> Self {
        let mut params = Self {
            parent_ino,
            name: [0; 256],
            name_len: 0,
            mode,
            uid,
            gid,
        };
        let len = core::cmp::min(name.len(), 255);
        params.name[..len].copy_from_slice(&name[..len]);
        params.name_len = len as u8;
        params
    }
    
    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

/// Rmdir parameters.
#[derive(Clone, Copy)]
pub struct RmdirParams {
    /// Parent inode
    pub parent_ino: u64,
    /// Directory name
    pub name: [u8; 256],
    /// Name length
    pub name_len: u8,
}

impl RmdirParams {
    /// Create new rmdir params
    pub fn new(parent_ino: u64, name: &[u8]) -> Self {
        let mut params = Self {
            parent_ino,
            name: [0; 256],
            name_len: 0,
        };
        let len = core::cmp::min(name.len(), 255);
        params.name[..len].copy_from_slice(&name[..len]);
        params.name_len = len as u8;
        params
    }
    
    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

/// Readdir parameters.
#[derive(Clone, Copy)]
pub struct ReaddirParams {
    /// Directory inode
    pub ino: u64,
    /// Start offset (cookie)
    pub offset: u64,
    /// Maximum entries to return
    pub max_entries: usize,
    /// Include file attributes
    pub plus: bool,
}

impl ReaddirParams {
    /// Create new readdir params
    pub fn new(ino: u64, offset: u64) -> Self {
        Self {
            ino,
            offset,
            max_entries: MAX_READDIR_ENTRIES,
            plus: false,
        }
    }
    
    /// With max entries
    pub fn with_max(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }
    
    /// With plus (include attributes)
    pub fn plus(mut self) -> Self {
        self.plus = true;
        self
    }
}

// ============================================================================
// Directory Handle
// ============================================================================

/// Open directory handle.
#[derive(Clone, Copy)]
pub struct OpenDir {
    /// Device ID
    pub dev: u64,
    /// Directory inode
    pub ino: u64,
    /// Current offset
    pub offset: u64,
    /// End of directory
    pub eof: bool,
    /// Reference count
    pub refcount: u32,
    /// Owner process ID
    pub owner_pid: u32,
}

impl OpenDir {
    /// Create new open directory
    pub fn new(dev: u64, ino: u64) -> Self {
        Self {
            dev,
            ino,
            offset: 0,
            eof: false,
            refcount: 1,
            owner_pid: 0,
        }
    }
    
    /// Reset position
    pub fn rewind(&mut self) {
        self.offset = 0;
        self.eof = false;
    }
    
    /// Seek to position
    pub fn seek(&mut self, offset: u64) {
        self.offset = offset;
        self.eof = false;
    }
    
    /// Acquire reference
    pub fn acquire(&mut self) -> u32 {
        self.refcount += 1;
        self.refcount
    }
    
    /// Release reference
    pub fn release(&mut self) -> u32 {
        self.refcount = self.refcount.saturating_sub(1);
        self.refcount
    }
}

impl Default for OpenDir {
    fn default() -> Self {
        Self {
            dev: 0,
            ino: 0,
            offset: 0,
            eof: false,
            refcount: 0,
            owner_pid: 0,
        }
    }
}

// ============================================================================
// Directory Table
// ============================================================================

/// Maximum open directories
const MAX_OPEN_DIRS: usize = 4096;

/// Directory table.
pub struct DirTable {
    /// Open directories
    dirs: [OpenDir; MAX_OPEN_DIRS],
    /// Used bitmap
    used: [bool; MAX_OPEN_DIRS],
    /// Count
    count: usize,
}

impl DirTable {
    /// Create new directory table
    pub const fn new() -> Self {
        Self {
            dirs: [OpenDir {
                dev: 0,
                ino: 0,
                offset: 0,
                eof: false,
                refcount: 0,
                owner_pid: 0,
            }; MAX_OPEN_DIRS],
            used: [false; MAX_OPEN_DIRS],
            count: 0,
        }
    }
    
    /// Allocate directory handle
    pub fn alloc(&mut self, dev: u64, ino: u64) -> HfsResult<usize> {
        for i in 0..MAX_OPEN_DIRS {
            if !self.used[i] {
                self.used[i] = true;
                self.dirs[i] = OpenDir::new(dev, ino);
                self.count += 1;
                return Ok(i);
            }
        }
        Err(HfsError::TooManyOpenFiles)
    }
    
    /// Free directory handle
    pub fn free(&mut self, idx: usize) -> HfsResult<()> {
        if idx >= MAX_OPEN_DIRS || !self.used[idx] {
            return Err(HfsError::BadFileDescriptor);
        }
        
        self.used[idx] = false;
        self.dirs[idx] = OpenDir::default();
        self.count -= 1;
        Ok(())
    }
    
    /// Get directory
    pub fn get(&self, idx: usize) -> Option<&OpenDir> {
        if idx < MAX_OPEN_DIRS && self.used[idx] {
            Some(&self.dirs[idx])
        } else {
            None
        }
    }
    
    /// Get mutable directory
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut OpenDir> {
        if idx < MAX_OPEN_DIRS && self.used[idx] {
            Some(&mut self.dirs[idx])
        } else {
            None
        }
    }
    
    /// Count
    pub fn count(&self) -> usize {
        self.count
    }
}

impl Default for DirTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Directory Utilities
// ============================================================================

/// Check if directory is empty.
pub fn is_dir_empty(entries: &[DirEntry]) -> bool {
    for entry in entries {
        if !entry.is_special() {
            return false;
        }
    }
    true
}

/// Count non-special entries.
pub fn count_entries(entries: &[DirEntry]) -> usize {
    entries.iter().filter(|e| !e.is_special()).count()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dir_entry_type() {
        let typ = DirEntryType::from_mode(0o040755);
        assert!(typ.is_dir());
        
        let typ = DirEntryType::from_mode(0o100644);
        assert!(typ.is_file());
    }
    
    #[test]
    fn test_dir_entry() {
        let entry = DirEntry::new(100, DirEntryType::Regular, b"file.txt", 0);
        assert_eq!(entry.name(), b"file.txt");
        assert!(!entry.is_special());
        
        let dot = DirEntry::dot(100, 0);
        assert!(dot.is_dot());
        assert!(dot.is_special());
        
        let dotdot = DirEntry::dotdot(99, 1);
        assert!(dotdot.is_dotdot());
        assert!(dotdot.is_special());
    }
    
    #[test]
    fn test_readdir_result() {
        let mut result = ReaddirResult::new();
        
        assert!(result.add(DirEntry::dot(100, 0)));
        assert!(result.add(DirEntry::dotdot(99, 1)));
        assert!(result.add(DirEntry::new(101, DirEntryType::Regular, b"file.txt", 2)));
        
        assert_eq!(result.count, 3);
        
        let entries: Vec<_> = result.iter().collect();
        assert_eq!(entries.len(), 3);
    }
    
    #[test]
    fn test_dir_table() {
        let mut table = DirTable::new();
        
        let idx = table.alloc(1, 100).unwrap();
        assert_eq!(table.count(), 1);
        
        let dir = table.get_mut(idx).unwrap();
        dir.offset = 10;
        
        assert_eq!(table.get(idx).unwrap().offset, 10);
        
        table.free(idx).unwrap();
        assert_eq!(table.count(), 0);
    }
    
    #[test]
    fn test_is_dir_empty() {
        let entries = [
            DirEntry::dot(100, 0),
            DirEntry::dotdot(99, 1),
        ];
        assert!(is_dir_empty(&entries));
        
        let entries = [
            DirEntry::dot(100, 0),
            DirEntry::dotdot(99, 1),
            DirEntry::new(101, DirEntryType::Regular, b"file.txt", 2),
        ];
        assert!(!is_dir_empty(&entries));
    }
}
