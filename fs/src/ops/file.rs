//! File Operations
//!
//! High-level file operations: open, close, create, truncate.

use crate::core::error::{HfsError, HfsResult};
use crate::api::OpenFlags;
use super::InodeType;

// ============================================================================
// File State
// ============================================================================

/// Open file state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FileState {
    /// Closed
    Closed = 0,
    /// Opening
    Opening = 1,
    /// Open
    Open = 2,
    /// Error state
    Error = 3,
    /// Closing
    Closing = 4,
}

impl Default for FileState {
    fn default() -> Self {
        Self::Closed
    }
}

/// File open mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum FileOpenMode {
    /// Read only
    ReadOnly = 0,
    /// Write only
    WriteOnly = 1,
    /// Read-write
    ReadWrite = 2,
}

impl FileOpenMode {
    /// From open flags
    pub fn from_flags(flags: OpenFlags) -> Self {
        if flags.has(OpenFlags::O_RDWR) {
            Self::ReadWrite
        } else if flags.has(OpenFlags::O_WRONLY) {
            Self::WriteOnly
        } else {
            Self::ReadOnly
        }
    }
    
    /// Can read
    pub fn can_read(self) -> bool {
        matches!(self, Self::ReadOnly | Self::ReadWrite)
    }
    
    /// Can write
    pub fn can_write(self) -> bool {
        matches!(self, Self::WriteOnly | Self::ReadWrite)
    }
}

// ============================================================================
// Open File Handle
// ============================================================================

/// Open file handle.
#[derive(Clone, Copy)]
pub struct OpenFile {
    /// Device ID
    pub dev: u64,
    /// Inode number
    pub ino: u64,
    /// File state
    pub state: FileState,
    /// Open mode
    pub mode: FileOpenMode,
    /// Open flags
    pub flags: OpenFlags,
    /// Current position
    pub pos: u64,
    /// File size
    pub size: u64,
    /// Reference count
    pub refcount: u32,
    /// Owner process ID
    pub owner_pid: u32,
    /// Open timestamp
    pub open_time: u64,
    /// Last access time
    pub access_time: u64,
    /// Bytes read
    pub bytes_read: u64,
    /// Bytes written
    pub bytes_written: u64,
    /// Read operations
    pub read_ops: u32,
    /// Write operations
    pub write_ops: u32,
}

impl OpenFile {
    /// Create new open file
    pub fn new(dev: u64, ino: u64, flags: OpenFlags) -> Self {
        Self {
            dev,
            ino,
            state: FileState::Open,
            mode: FileOpenMode::from_flags(flags),
            flags,
            pos: 0,
            size: 0,
            refcount: 1,
            owner_pid: 0,
            open_time: 0,
            access_time: 0,
            bytes_read: 0,
            bytes_written: 0,
            read_ops: 0,
            write_ops: 0,
        }
    }
    
    /// Is open
    pub fn is_open(&self) -> bool {
        self.state == FileState::Open
    }
    
    /// Can read
    pub fn can_read(&self) -> bool {
        self.is_open() && self.mode.can_read()
    }
    
    /// Can write
    pub fn can_write(&self) -> bool {
        self.is_open() && self.mode.can_write()
    }
    
    /// Is append mode
    pub fn is_append(&self) -> bool {
        self.flags.has(OpenFlags::O_APPEND)
    }
    
    /// Is sync mode
    pub fn is_sync(&self) -> bool {
        self.flags.has(OpenFlags::O_SYNC)
    }
    
    /// Seek to position
    pub fn seek(&mut self, pos: u64) -> HfsResult<u64> {
        if pos > self.size {
            // Allow seek past end for writes
        }
        self.pos = pos;
        Ok(pos)
    }
    
    /// Read bytes (update stats)
    pub fn record_read(&mut self, bytes: u64) {
        self.bytes_read += bytes;
        self.read_ops += 1;
        self.pos += bytes;
    }
    
    /// Write bytes (update stats)
    pub fn record_write(&mut self, bytes: u64) {
        self.bytes_written += bytes;
        self.write_ops += 1;
        if self.is_append() {
            self.pos = self.size;
        }
        self.pos += bytes;
        if self.pos > self.size {
            self.size = self.pos;
        }
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

impl Default for OpenFile {
    fn default() -> Self {
        Self {
            dev: 0,
            ino: 0,
            state: FileState::Closed,
            mode: FileOpenMode::ReadOnly,
            flags: OpenFlags::default(),
            pos: 0,
            size: 0,
            refcount: 0,
            owner_pid: 0,
            open_time: 0,
            access_time: 0,
            bytes_read: 0,
            bytes_written: 0,
            read_ops: 0,
            write_ops: 0,
        }
    }
}

// ============================================================================
// File Table
// ============================================================================

/// Maximum open files
const MAX_OPEN_FILES: usize = 16384;

/// File table entry state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
enum FileTableSlot {
    Free,
    Used,
}

/// System-wide file table.
pub struct FileTable {
    /// Open files
    files: [OpenFile; MAX_OPEN_FILES],
    /// Slot states
    slots: [FileTableSlot; MAX_OPEN_FILES],
    /// Open file count
    count: usize,
    /// Free list head
    free_head: usize,
}

impl FileTable {
    /// Create new file table
    pub const fn new() -> Self {
        Self {
            files: [OpenFile {
                dev: 0,
                ino: 0,
                state: FileState::Closed,
                mode: FileOpenMode::ReadOnly,
                flags: OpenFlags(0),
                pos: 0,
                size: 0,
                refcount: 0,
                owner_pid: 0,
                open_time: 0,
                access_time: 0,
                bytes_read: 0,
                bytes_written: 0,
                read_ops: 0,
                write_ops: 0,
            }; MAX_OPEN_FILES],
            slots: [FileTableSlot::Free; MAX_OPEN_FILES],
            count: 0,
            free_head: 0,
        }
    }
    
    /// Allocate file entry
    pub fn alloc(&mut self, dev: u64, ino: u64, flags: OpenFlags) -> HfsResult<usize> {
        // Find free slot
        for i in self.free_head..MAX_OPEN_FILES {
            if self.slots[i] == FileTableSlot::Free {
                self.slots[i] = FileTableSlot::Used;
                self.files[i] = OpenFile::new(dev, ino, flags);
                self.count += 1;
                self.free_head = i + 1;
                return Ok(i);
            }
        }
        
        // Wrap around
        for i in 0..self.free_head {
            if self.slots[i] == FileTableSlot::Free {
                self.slots[i] = FileTableSlot::Used;
                self.files[i] = OpenFile::new(dev, ino, flags);
                self.count += 1;
                self.free_head = i + 1;
                return Ok(i);
            }
        }
        
        Err(HfsError::TooManyOpenFiles)
    }
    
    /// Free file entry
    pub fn free(&mut self, idx: usize) -> HfsResult<()> {
        if idx >= MAX_OPEN_FILES {
            return Err(HfsError::InvalidArgument);
        }
        
        if self.slots[idx] != FileTableSlot::Used {
            return Err(HfsError::BadFileDescriptor);
        }
        
        self.slots[idx] = FileTableSlot::Free;
        self.files[idx] = OpenFile::default();
        self.count -= 1;
        
        if idx < self.free_head {
            self.free_head = idx;
        }
        
        Ok(())
    }
    
    /// Get file
    pub fn get(&self, idx: usize) -> Option<&OpenFile> {
        if idx < MAX_OPEN_FILES && self.slots[idx] == FileTableSlot::Used {
            Some(&self.files[idx])
        } else {
            None
        }
    }
    
    /// Get mutable file
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut OpenFile> {
        if idx < MAX_OPEN_FILES && self.slots[idx] == FileTableSlot::Used {
            Some(&mut self.files[idx])
        } else {
            None
        }
    }
    
    /// Count open files
    pub fn count(&self) -> usize {
        self.count
    }
    
    /// Find file by inode
    pub fn find_by_inode(&self, dev: u64, ino: u64) -> Option<usize> {
        for i in 0..MAX_OPEN_FILES {
            if self.slots[i] == FileTableSlot::Used {
                if self.files[i].dev == dev && self.files[i].ino == ino {
                    return Some(i);
                }
            }
        }
        None
    }
    
    /// Get statistics
    pub fn stats(&self) -> FileTableStats {
        let mut stats = FileTableStats::default();
        
        for i in 0..MAX_OPEN_FILES {
            if self.slots[i] == FileTableSlot::Used {
                stats.open_files += 1;
                stats.total_bytes_read += self.files[i].bytes_read;
                stats.total_bytes_written += self.files[i].bytes_written;
            }
        }
        
        stats
    }
}

impl Default for FileTable {
    fn default() -> Self {
        Self::new()
    }
}

/// File table statistics.
#[derive(Clone, Copy, Debug, Default)]
pub struct FileTableStats {
    /// Number of open files
    pub open_files: u32,
    /// Total bytes read
    pub total_bytes_read: u64,
    /// Total bytes written
    pub total_bytes_written: u64,
}

// ============================================================================
// File Operations
// ============================================================================

/// Create file parameters.
#[derive(Clone, Copy)]
pub struct CreateParams {
    /// Parent inode
    pub parent_ino: u64,
    /// File name
    pub name: [u8; 256],
    /// Name length
    pub name_len: u8,
    /// Mode (permissions + type)
    pub mode: u16,
    /// Owner UID
    pub uid: u32,
    /// Owner GID
    pub gid: u32,
    /// Device number (for device files)
    pub rdev: u64,
}

impl CreateParams {
    /// New file create params
    pub fn file(parent_ino: u64, name: &[u8], mode: u16, uid: u32, gid: u32) -> Self {
        let mut params = Self {
            parent_ino,
            name: [0; 256],
            name_len: 0,
            mode: InodeType::Regular.to_mode() | (mode & 0o777),
            uid,
            gid,
            rdev: 0,
        };
        let len = core::cmp::min(name.len(), 255);
        params.name[..len].copy_from_slice(&name[..len]);
        params.name_len = len as u8;
        params
    }
    
    /// New directory create params
    pub fn dir(parent_ino: u64, name: &[u8], mode: u16, uid: u32, gid: u32) -> Self {
        let mut params = Self::file(parent_ino, name, mode, uid, gid);
        params.mode = InodeType::Directory.to_mode() | (mode & 0o777);
        params
    }
    
    /// Get name
    pub fn name(&self) -> &[u8] {
        &self.name[..self.name_len as usize]
    }
}

impl Default for CreateParams {
    fn default() -> Self {
        Self {
            parent_ino: 0,
            name: [0; 256],
            name_len: 0,
            mode: 0o644,
            uid: 0,
            gid: 0,
            rdev: 0,
        }
    }
}

/// Truncate parameters.
#[derive(Clone, Copy)]
pub struct TruncateParams {
    /// Inode
    pub ino: u64,
    /// New size
    pub size: u64,
}

impl TruncateParams {
    /// Create new truncate params
    pub fn new(ino: u64, size: u64) -> Self {
        Self { ino, size }
    }
}

/// File creation result.
#[derive(Clone, Copy)]
pub struct CreateResult {
    /// Created inode
    pub ino: u64,
    /// Mode
    pub mode: u16,
    /// Generation
    pub generation: u32,
}

// ============================================================================
// File Allocation
// ============================================================================

/// Fallocate mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum FallocateMode {
    /// Allocate space (default)
    Allocate = 0,
    /// Keep size unchanged
    KeepSize = 1,
    /// Punch hole
    PunchHole = 2,
    /// Zero range
    ZeroRange = 4,
    /// Collapse range
    CollapseRange = 8,
    /// Insert range
    InsertRange = 16,
    /// Unshare range
    UnshareRange = 32,
}

/// Fallocate parameters.
#[derive(Clone, Copy)]
pub struct FallocateParams {
    /// Inode
    pub ino: u64,
    /// Mode
    pub mode: FallocateMode,
    /// Offset
    pub offset: u64,
    /// Length
    pub len: u64,
}

impl FallocateParams {
    /// Allocate space
    pub fn allocate(ino: u64, offset: u64, len: u64) -> Self {
        Self {
            ino,
            mode: FallocateMode::Allocate,
            offset,
            len,
        }
    }
    
    /// Punch hole
    pub fn punch_hole(ino: u64, offset: u64, len: u64) -> Self {
        Self {
            ino,
            mode: FallocateMode::PunchHole,
            offset,
            len,
        }
    }
    
    /// Zero range
    pub fn zero_range(ino: u64, offset: u64, len: u64) -> Self {
        Self {
            ino,
            mode: FallocateMode::ZeroRange,
            offset,
            len,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_file_open_mode() {
        let flags = OpenFlags::O_RDONLY;
        let mode = FileOpenMode::from_flags(flags);
        assert!(mode.can_read());
        assert!(!mode.can_write());
        
        let flags = OpenFlags(OpenFlags::O_RDWR);
        let mode = FileOpenMode::from_flags(flags);
        assert!(mode.can_read());
        assert!(mode.can_write());
    }
    
    #[test]
    fn test_open_file() {
        let mut file = OpenFile::new(1, 100, OpenFlags(OpenFlags::O_RDWR));
        
        assert!(file.is_open());
        assert!(file.can_read());
        assert!(file.can_write());
        
        file.record_read(1024);
        assert_eq!(file.bytes_read, 1024);
        assert_eq!(file.pos, 1024);
        
        file.record_write(512);
        assert_eq!(file.bytes_written, 512);
        assert_eq!(file.pos, 1536);
    }
    
    #[test]
    fn test_file_table() {
        let mut table = FileTable::new();
        
        let idx = table.alloc(1, 100, OpenFlags(OpenFlags::O_RDONLY)).unwrap();
        assert_eq!(table.count(), 1);
        
        let file = table.get(idx).unwrap();
        assert_eq!(file.ino, 100);
        
        table.free(idx).unwrap();
        assert_eq!(table.count(), 0);
    }
    
    #[test]
    fn test_create_params() {
        let params = CreateParams::file(2, b"test.txt", 0o644, 1000, 1000);
        assert_eq!(params.name(), b"test.txt");
        assert_eq!(params.mode & 0o777, 0o644);
    }
}
