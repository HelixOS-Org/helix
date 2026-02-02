//! File and Directory Operations
//!
//! Implements high-level file system operations with proper
//! error handling and atomic guarantees.

use super::vfs::{FileHandle, PathBuf};
use super::{DirEntry, FileStat, OpenFlags, SeekWhence};
use crate::core::error::{HfsError, HfsResult};

// ============================================================================
// Constants
// ============================================================================

/// Maximum read/write size in one call
pub const MAX_IO_SIZE: usize = 16 * 1024 * 1024; // 16 MB

/// Maximum readdir entries per call
pub const MAX_READDIR_ENTRIES: usize = 4096;

/// Rename flags
pub mod rename_flags {
    /// No rename flags
    pub const RENAME_NOREPLACE: u32 = 1 << 0;
    /// Exchange source and destination
    pub const RENAME_EXCHANGE: u32 = 1 << 1;
    /// Don't follow symlinks in final component
    pub const RENAME_WHITEOUT: u32 = 1 << 2;
}

/// Fallocate modes
pub mod falloc_mode {
    /// Default allocation
    pub const FALLOC_FL_DEFAULT: u32 = 0;
    /// Keep size
    pub const FALLOC_FL_KEEP_SIZE: u32 = 1 << 0;
    /// Punch hole
    pub const FALLOC_FL_PUNCH_HOLE: u32 = 1 << 1;
    /// Zero range
    pub const FALLOC_FL_ZERO_RANGE: u32 = 1 << 2;
    /// Collapse range
    pub const FALLOC_FL_COLLAPSE_RANGE: u32 = 1 << 3;
    /// Insert range
    pub const FALLOC_FL_INSERT_RANGE: u32 = 1 << 4;
}

// ============================================================================
// Read Operation
// ============================================================================

/// Read operation context.
#[derive(Clone, Copy, Debug)]
pub struct ReadOp {
    /// Inode number
    pub ino: u64,
    /// Offset in file
    pub offset: u64,
    /// Maximum bytes to read
    pub len: usize,
    /// Direct I/O (bypass cache)
    pub direct: bool,
}

impl ReadOp {
    /// Create new read operation
    pub fn new(ino: u64, offset: u64, len: usize) -> Self {
        Self {
            ino,
            offset,
            len,
            direct: false,
        }
    }

    /// Set direct I/O
    pub fn direct(mut self) -> Self {
        self.direct = true;
        self
    }

    /// Validate operation
    pub fn validate(&self) -> HfsResult<()> {
        if self.len > MAX_IO_SIZE {
            return Err(HfsError::InvalidArgument);
        }

        if self.offset.checked_add(self.len as u64).is_none() {
            return Err(HfsError::Overflow);
        }

        Ok(())
    }
}

/// Read operation result.
#[derive(Clone, Copy, Debug)]
pub struct ReadResult {
    /// Bytes read
    pub bytes_read: usize,
    /// End of file reached
    pub eof: bool,
}

// ============================================================================
// Write Operation
// ============================================================================

/// Write operation context.
#[derive(Clone, Copy, Debug)]
pub struct WriteOp {
    /// Inode number
    pub ino: u64,
    /// Offset in file
    pub offset: u64,
    /// Bytes to write
    pub len: usize,
    /// Direct I/O (bypass cache)
    pub direct: bool,
    /// Append mode
    pub append: bool,
    /// Sync after write
    pub sync: bool,
}

impl WriteOp {
    /// Create new write operation
    pub fn new(ino: u64, offset: u64, len: usize) -> Self {
        Self {
            ino,
            offset,
            len,
            direct: false,
            append: false,
            sync: false,
        }
    }

    /// Set direct I/O
    pub fn direct(mut self) -> Self {
        self.direct = true;
        self
    }

    /// Set append mode
    pub fn append(mut self) -> Self {
        self.append = true;
        self
    }

    /// Set sync mode
    pub fn sync(mut self) -> Self {
        self.sync = true;
        self
    }

    /// Validate operation
    pub fn validate(&self) -> HfsResult<()> {
        if self.len > MAX_IO_SIZE {
            return Err(HfsError::InvalidArgument);
        }

        if !self.append {
            if self.offset.checked_add(self.len as u64).is_none() {
                return Err(HfsError::Overflow);
            }
        }

        Ok(())
    }
}

/// Write operation result.
#[derive(Clone, Copy, Debug)]
pub struct WriteResult {
    /// Bytes written
    pub bytes_written: usize,
    /// New file size
    pub new_size: u64,
}

// ============================================================================
// Create Operation
// ============================================================================

/// File creation options.
#[derive(Clone, Copy, Debug)]
pub struct CreateOp {
    /// Parent directory inode
    pub parent: u64,
    /// File mode (permissions)
    pub mode: u32,
    /// Open flags
    pub flags: OpenFlags,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
}

impl CreateOp {
    /// Create new file creation operation
    pub fn new(parent: u64, mode: u32) -> Self {
        Self {
            parent,
            mode,
            flags: OpenFlags(OpenFlags::O_CREAT | OpenFlags::O_RDWR),
            uid: 0,
            gid: 0,
        }
    }

    /// Set flags
    pub fn flags(mut self, flags: OpenFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set owner
    pub fn owner(mut self, uid: u32, gid: u32) -> Self {
        self.uid = uid;
        self.gid = gid;
        self
    }

    /// Set exclusive creation
    pub fn exclusive(mut self) -> Self {
        self.flags = OpenFlags(self.flags.0 | OpenFlags::O_EXCL);
        self
    }
}

/// Create operation result.
#[derive(Clone, Copy, Debug)]
pub struct CreateResult {
    /// Created inode number
    pub ino: u64,
    /// File handle
    pub handle: FileHandle,
    /// File attributes
    pub attr: FileStat,
}

// ============================================================================
// Directory Operations
// ============================================================================

/// Mkdir operation.
#[derive(Clone, Copy, Debug)]
pub struct MkdirOp {
    /// Parent directory inode
    pub parent: u64,
    /// Directory mode
    pub mode: u32,
    /// User ID
    pub uid: u32,
    /// Group ID  
    pub gid: u32,
}

impl MkdirOp {
    /// Create new mkdir operation
    pub fn new(parent: u64, mode: u32) -> Self {
        Self {
            parent,
            mode,
            uid: 0,
            gid: 0,
        }
    }

    /// Set owner
    pub fn owner(mut self, uid: u32, gid: u32) -> Self {
        self.uid = uid;
        self.gid = gid;
        self
    }
}

/// Readdir operation result.
#[derive(Clone, Debug)]
pub struct ReaddirResult {
    /// Entries read
    pub entries: [DirEntry; 64],
    /// Number of entries
    pub count: usize,
    /// More entries available
    pub has_more: bool,
    /// Next offset
    pub next_offset: u64,
}

impl ReaddirResult {
    /// Create new result
    pub fn new() -> Self {
        Self {
            entries: [DirEntry::default(); 64],
            count: 0,
            has_more: false,
            next_offset: 0,
        }
    }
}

impl Default for ReaddirResult {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Link Operations
// ============================================================================

/// Link operation.
#[derive(Clone, Copy, Debug)]
pub struct LinkOp {
    /// Source inode
    pub source: u64,
    /// Destination parent
    pub parent: u64,
}

/// Symlink operation.
#[derive(Clone, Debug)]
pub struct SymlinkOp {
    /// Parent directory
    pub parent: u64,
    /// Target path
    pub target: PathBuf,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
}

impl SymlinkOp {
    /// Create new symlink operation
    pub fn new(parent: u64, target: &[u8]) -> Self {
        Self {
            parent,
            target: PathBuf::from_slice(target),
            uid: 0,
            gid: 0,
        }
    }
}

/// Rename operation.
#[derive(Clone, Copy, Debug)]
pub struct RenameOp {
    /// Source parent
    pub oldparent: u64,
    /// Destination parent
    pub newparent: u64,
    /// Rename flags
    pub flags: u32,
}

impl RenameOp {
    /// Create new rename operation
    pub fn new(oldparent: u64, newparent: u64) -> Self {
        Self {
            oldparent,
            newparent,
            flags: 0,
        }
    }

    /// No replace
    pub fn noreplace(mut self) -> Self {
        self.flags |= rename_flags::RENAME_NOREPLACE;
        self
    }

    /// Exchange
    pub fn exchange(mut self) -> Self {
        self.flags |= rename_flags::RENAME_EXCHANGE;
        self
    }
}

// ============================================================================
// Truncate Operation
// ============================================================================

/// Truncate operation.
#[derive(Clone, Copy, Debug)]
pub struct TruncateOp {
    /// Inode number
    pub ino: u64,
    /// New size
    pub size: u64,
}

impl TruncateOp {
    /// Create new truncate operation
    pub fn new(ino: u64, size: u64) -> Self {
        Self { ino, size }
    }
}

// ============================================================================
// Seek Operation
// ============================================================================

/// Seek operation.
#[derive(Clone, Copy, Debug)]
pub struct SeekOp {
    /// Current position
    pub current: u64,
    /// File size
    pub size: u64,
    /// Offset
    pub offset: i64,
    /// Whence
    pub whence: SeekWhence,
}

impl SeekOp {
    /// Create new seek operation
    pub fn new(current: u64, size: u64, offset: i64, whence: SeekWhence) -> Self {
        Self {
            current,
            size,
            offset,
            whence,
        }
    }

    /// Calculate new position
    pub fn calculate(&self) -> HfsResult<u64> {
        let new_pos = match self.whence {
            SeekWhence::Set => {
                if self.offset < 0 {
                    return Err(HfsError::InvalidArgument);
                }
                self.offset as u64
            },
            SeekWhence::Cur => {
                if self.offset < 0 {
                    let abs_offset = (-self.offset) as u64;
                    if abs_offset > self.current {
                        return Err(HfsError::InvalidArgument);
                    }
                    self.current - abs_offset
                } else {
                    self.current
                        .checked_add(self.offset as u64)
                        .ok_or(HfsError::Overflow)?
                }
            },
            SeekWhence::End => {
                if self.offset < 0 {
                    let abs_offset = (-self.offset) as u64;
                    if abs_offset > self.size {
                        return Err(HfsError::InvalidArgument);
                    }
                    self.size - abs_offset
                } else {
                    self.size
                        .checked_add(self.offset as u64)
                        .ok_or(HfsError::Overflow)?
                }
            },
            SeekWhence::Data | SeekWhence::Hole => {
                // These require extent information
                return Err(HfsError::NotSupported);
            },
        };

        Ok(new_pos)
    }
}

// ============================================================================
// Fallocate Operation
// ============================================================================

/// Fallocate operation.
#[derive(Clone, Copy, Debug)]
pub struct FallocateOp {
    /// Inode number
    pub ino: u64,
    /// Mode flags
    pub mode: u32,
    /// Start offset
    pub offset: u64,
    /// Length
    pub len: u64,
}

impl FallocateOp {
    /// Create new fallocate operation
    pub fn new(ino: u64, offset: u64, len: u64) -> Self {
        Self {
            ino,
            mode: falloc_mode::FALLOC_FL_DEFAULT,
            offset,
            len,
        }
    }

    /// Keep size
    pub fn keep_size(mut self) -> Self {
        self.mode |= falloc_mode::FALLOC_FL_KEEP_SIZE;
        self
    }

    /// Punch hole
    pub fn punch_hole(mut self) -> Self {
        self.mode |= falloc_mode::FALLOC_FL_PUNCH_HOLE | falloc_mode::FALLOC_FL_KEEP_SIZE;
        self
    }

    /// Zero range
    pub fn zero_range(mut self) -> Self {
        self.mode |= falloc_mode::FALLOC_FL_ZERO_RANGE;
        self
    }

    /// Validate operation
    pub fn validate(&self) -> HfsResult<()> {
        if self.len == 0 {
            return Err(HfsError::InvalidArgument);
        }

        if self.offset.checked_add(self.len).is_none() {
            return Err(HfsError::Overflow);
        }

        Ok(())
    }

    /// Is punch hole
    #[inline]
    pub fn is_punch_hole(&self) -> bool {
        self.mode & falloc_mode::FALLOC_FL_PUNCH_HOLE != 0
    }

    /// Is zero range
    #[inline]
    pub fn is_zero_range(&self) -> bool {
        self.mode & falloc_mode::FALLOC_FL_ZERO_RANGE != 0
    }

    /// Is keep size
    #[inline]
    pub fn is_keep_size(&self) -> bool {
        self.mode & falloc_mode::FALLOC_FL_KEEP_SIZE != 0
    }
}

// ============================================================================
// Copy File Range
// ============================================================================

/// Copy file range operation.
#[derive(Clone, Copy, Debug)]
pub struct CopyFileRangeOp {
    /// Source inode
    pub src_ino: u64,
    /// Source offset
    pub src_off: u64,
    /// Destination inode
    pub dst_ino: u64,
    /// Destination offset
    pub dst_off: u64,
    /// Length to copy
    pub len: u64,
    /// Flags
    pub flags: u32,
}

impl CopyFileRangeOp {
    /// Create new copy operation
    pub fn new(src_ino: u64, src_off: u64, dst_ino: u64, dst_off: u64, len: u64) -> Self {
        Self {
            src_ino,
            src_off,
            dst_ino,
            dst_off,
            len,
            flags: 0,
        }
    }

    /// Validate operation
    pub fn validate(&self) -> HfsResult<()> {
        if self.len == 0 {
            return Err(HfsError::InvalidArgument);
        }

        if self.src_off.checked_add(self.len).is_none() {
            return Err(HfsError::Overflow);
        }

        if self.dst_off.checked_add(self.len).is_none() {
            return Err(HfsError::Overflow);
        }

        Ok(())
    }
}

/// Copy file range result.
#[derive(Clone, Copy, Debug)]
pub struct CopyFileRangeResult {
    /// Bytes copied
    pub bytes_copied: u64,
    /// Used reflink (CoW)
    pub reflinked: bool,
}

// ============================================================================
// IO Vector
// ============================================================================

/// IO vector for scatter/gather I/O.
#[derive(Clone, Copy, Debug)]
pub struct IoVec {
    /// Buffer pointer (as usize for no_std)
    pub base: usize,
    /// Buffer length
    pub len: usize,
}

impl IoVec {
    /// Create new IO vector
    pub fn new(base: usize, len: usize) -> Self {
        Self { base, len }
    }

    /// Total length of vector array
    pub fn total_len(iov: &[IoVec]) -> usize {
        iov.iter().map(|v| v.len).sum()
    }
}

/// Readv operation.
#[derive(Clone, Debug)]
pub struct ReadvOp {
    /// Inode number
    pub ino: u64,
    /// Offset
    pub offset: u64,
    /// IO vectors
    pub iov: [IoVec; 16],
    /// Number of vectors
    pub iovcnt: usize,
}

impl ReadvOp {
    /// Create new readv operation
    pub fn new(ino: u64, offset: u64) -> Self {
        Self {
            ino,
            offset,
            iov: [IoVec::new(0, 0); 16],
            iovcnt: 0,
        }
    }

    /// Add vector
    pub fn add_vec(&mut self, base: usize, len: usize) -> bool {
        if self.iovcnt >= 16 {
            return false;
        }

        self.iov[self.iovcnt] = IoVec::new(base, len);
        self.iovcnt += 1;
        true
    }

    /// Total length
    pub fn total_len(&self) -> usize {
        IoVec::total_len(&self.iov[..self.iovcnt])
    }
}

/// Writev operation.
#[derive(Clone, Debug)]
pub struct WritevOp {
    /// Inode number
    pub ino: u64,
    /// Offset
    pub offset: u64,
    /// IO vectors
    pub iov: [IoVec; 16],
    /// Number of vectors
    pub iovcnt: usize,
    /// Sync after write
    pub sync: bool,
}

impl WritevOp {
    /// Create new writev operation
    pub fn new(ino: u64, offset: u64) -> Self {
        Self {
            ino,
            offset,
            iov: [IoVec::new(0, 0); 16],
            iovcnt: 0,
            sync: false,
        }
    }

    /// Add vector
    pub fn add_vec(&mut self, base: usize, len: usize) -> bool {
        if self.iovcnt >= 16 {
            return false;
        }

        self.iov[self.iovcnt] = IoVec::new(base, len);
        self.iovcnt += 1;
        true
    }

    /// Total length
    pub fn total_len(&self) -> usize {
        IoVec::total_len(&self.iov[..self.iovcnt])
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_op() {
        let op = ReadOp::new(100, 0, 4096).direct();

        assert_eq!(op.ino, 100);
        assert!(op.direct);
        assert!(op.validate().is_ok());
    }

    #[test]
    fn test_write_op() {
        let op = WriteOp::new(100, 0, 4096).sync().append();

        assert!(op.sync);
        assert!(op.append);
        assert!(op.validate().is_ok());
    }

    #[test]
    fn test_seek_op() {
        let op = SeekOp::new(100, 1000, 50, SeekWhence::Cur);
        assert_eq!(op.calculate().unwrap(), 150);

        let op = SeekOp::new(100, 1000, -50, SeekWhence::End);
        assert_eq!(op.calculate().unwrap(), 950);

        let op = SeekOp::new(100, 1000, 0, SeekWhence::Set);
        assert_eq!(op.calculate().unwrap(), 0);
    }

    #[test]
    fn test_fallocate_op() {
        let op = FallocateOp::new(100, 0, 4096).punch_hole();

        assert!(op.is_punch_hole());
        assert!(op.is_keep_size());
        assert!(op.validate().is_ok());
    }

    #[test]
    fn test_copy_file_range() {
        let op = CopyFileRangeOp::new(1, 0, 2, 0, 1024);

        assert_eq!(op.src_ino, 1);
        assert_eq!(op.dst_ino, 2);
        assert!(op.validate().is_ok());
    }

    #[test]
    fn test_io_vec() {
        let mut op = ReadvOp::new(100, 0);

        assert!(op.add_vec(0x1000, 4096));
        assert!(op.add_vec(0x2000, 4096));

        assert_eq!(op.total_len(), 8192);
    }
}
