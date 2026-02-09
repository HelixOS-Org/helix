//! Core filesystem types.

// ============================================================================
// TYPE ALIASES
// ============================================================================

/// Inode number
pub type Inode = u64;

/// Block number
pub type BlockNum = u64;

/// File descriptor
pub type FileDescriptor = u32;

// ============================================================================
// FILE TYPE
// ============================================================================

/// File type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// Regular file
    Regular,
    /// Directory
    Directory,
    /// Symbolic link
    Symlink,
    /// Block device
    BlockDevice,
    /// Character device
    CharDevice,
    /// Named pipe (FIFO)
    Pipe,
    /// Unix socket
    Socket,
    /// Unknown type
    Unknown,
}

impl FileType {
    /// Is this a regular file?
    #[inline(always)]
    pub fn is_file(&self) -> bool {
        matches!(self, Self::Regular)
    }

    /// Is this a directory?
    #[inline(always)]
    pub fn is_dir(&self) -> bool {
        matches!(self, Self::Directory)
    }

    /// Can this be cached?
    #[inline(always)]
    pub fn is_cacheable(&self) -> bool {
        matches!(self, Self::Regular | Self::Directory | Self::Symlink)
    }
}

// ============================================================================
// ACCESS MODE
// ============================================================================

/// Access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// Read-only
    Read,
    /// Write-only
    Write,
    /// Read-write
    ReadWrite,
    /// Append
    Append,
    /// Execute
    Execute,
}

impl AccessMode {
    /// Is this a read operation?
    #[inline(always)]
    pub fn is_read(&self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite)
    }

    /// Is this a write operation?
    #[inline(always)]
    pub fn is_write(&self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite | Self::Append)
    }
}

// ============================================================================
// I/O PATTERN TYPE
// ============================================================================

/// I/O pattern type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoPatternType {
    /// Sequential reads
    SequentialRead,
    /// Sequential writes
    SequentialWrite,
    /// Random reads
    RandomRead,
    /// Random writes
    RandomWrite,
    /// Mixed I/O
    Mixed,
    /// Metadata heavy
    MetadataHeavy,
}
