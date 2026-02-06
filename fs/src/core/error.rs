//! Error types and result handling for HelixFS.
//!
//! This module provides a comprehensive error type that covers all possible
//! failure modes in the filesystem, from I/O errors to corruption detection.

use core::fmt;

/// Result type alias for HelixFS operations.
pub type HfsResult<T> = Result<T, HfsError>;

/// Comprehensive error type for HelixFS operations.
///
/// This enum covers all possible error conditions that can occur during
/// filesystem operations, including I/O errors, resource exhaustion,
/// permission issues, and data integrity failures.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum HfsError {
    // ========================================================================
    // General Errors (0-99)
    // ========================================================================
    /// Success (not really an error, but useful for FFI)
    Success              = 0,

    /// Generic I/O error
    Io                   = 1,

    /// Out of memory
    NoMemory             = 2,

    /// Invalid argument
    InvalidArgument      = 3,

    /// Operation not supported
    NotSupported         = 4,

    /// Operation would block (for non-blocking I/O)
    WouldBlock           = 5,

    /// Operation interrupted
    Interrupted          = 6,

    /// Try again later
    Again                = 7,

    /// Operation in progress
    InProgress           = 8,

    /// Operation already in progress
    Already              = 9,

    /// Operation timed out
    TimedOut             = 10,

    /// Deadlock would occur
    Deadlock             = 11,

    /// Resource busy
    Busy                 = 12,

    /// Buffer too small
    BufferTooSmall       = 13,

    /// End of file/iteration
    EndOfFile            = 14,

    /// Arithmetic overflow
    Overflow             = 15,

    /// Arithmetic underflow
    Underflow            = 16,

    /// Data too big
    TooBig               = 17,

    /// Out of memory (alias)
    OutOfMemory          = 18,

    /// Invalid alignment
    InvalidAlignment     = 19,

    /// Invalid data
    InvalidData          = 20,

    /// Resource is locked
    Locked               = 21,

    /// Not initialized
    NotInitialized       = 22,

    /// Integrity error
    IntegrityError       = 23,

    /// Invalid parameter (alias for InvalidArgument)
    InvalidParameter     = 24,

    /// Invalid arg (alias for InvalidArgument)
    InvalidArg           = 25,

    // ========================================================================
    // File/Directory Errors (100-199)
    // ========================================================================
    /// No such file or directory
    NotFound             = 100,

    /// File or directory already exists
    AlreadyExists        = 101,

    /// Not a directory
    NotDirectory         = 102,

    /// Is a directory (when file expected)
    IsDirectory          = 103,

    /// Directory not empty
    NotEmpty             = 104,

    /// Too many open files
    TooManyOpenFiles     = 105,

    /// Bad file descriptor
    BadFileDescriptor    = 106,

    /// File name too long
    NameTooLong          = 107,

    /// Too many symbolic link levels
    SymlinkLoop          = 108,

    /// Invalid path
    InvalidPath          = 109,

    /// Cross-device link
    CrossDevice          = 110,

    /// Stale file handle
    Stale                = 111,

    /// File too large
    FileTooLarge         = 112,

    /// Invalid file type
    InvalidFileType      = 113,

    /// Bad file handle
    BadHandle            = 114,

    /// Invalid handle
    InvalidHandle        = 115,

    /// Bad file descriptor (alias)
    BadFd                = 116,

    /// Cannot delete root
    CannotDeleteRoot     = 117,

    /// Directory is full
    DirectoryFull        = 118,

    // ========================================================================
    // Permission Errors (200-299)
    // ========================================================================
    /// Permission denied
    PermissionDenied     = 200,

    /// Operation not permitted
    NotPermitted         = 201,

    /// Read-only filesystem
    ReadOnly             = 202,

    /// Read-only filesystem (alias)
    ReadOnlyFilesystem   = 206,

    /// Immutable file
    Immutable            = 203,

    /// Append-only file
    AppendOnly           = 204,

    /// Access denied by ACL
    AclDenied            = 205,

    // ========================================================================
    // Space/Resource Errors (300-399)
    // ========================================================================
    /// No space left on device
    NoSpace              = 300,

    /// Disk quota exceeded
    QuotaExceeded        = 301,

    /// No inodes left
    NoInodes             = 302,

    /// Extent tree full
    ExtentTreeFull       = 303,

    /// Maximum file size exceeded
    MaxFileSizeExceeded  = 304,

    /// Too many snapshots
    TooManySnapshots     = 305,

    /// Too many hard links
    TooManyLinks         = 306,

    /// Invalid block number
    InvalidBlockNumber   = 307,

    /// B-tree is full
    BtreeFull            = 308,

    /// B-tree too deep
    BtreeTooDeep         = 309,

    /// Refcount overflow
    RefcountOverflow     = 310,

    /// Refcount underflow
    RefcountUnderflow    = 311,

    // ========================================================================
    // Data Integrity Errors (400-499)
    // ========================================================================
    /// Data corruption detected
    Corruption           = 400,

    /// Checksum mismatch
    ChecksumMismatch     = 401,

    /// Invalid magic number
    BadMagic             = 402,

    /// Incompatible filesystem version
    IncompatibleVersion  = 403,

    /// Merkle tree verification failed
    MerkleVerifyFailed   = 404,

    /// Journal corruption
    JournalCorruption    = 405,

    /// Superblock corruption
    SuperblockCorruption = 406,

    /// Extent tree corruption
    ExtentTreeCorruption = 407,

    /// Inode corruption
    InodeCorruption      = 408,

    /// Directory corruption
    DirectoryCorruption  = 409,

    /// Directory corruption (alias)
    DirCorruption        = 415,

    /// Generic corrupted data
    CorruptedData        = 410,

    /// B-tree corruption
    BtreeCorruption      = 411,

    /// B-tree key not found
    BtreeKeyNotFound     = 412,

    /// Extent corruption
    ExtentCorruption     = 413,

    /// Extent not found
    ExtentNotFound       = 414,

    // ========================================================================
    // Transaction/Journal Errors (500-599)
    // ========================================================================
    /// Transaction aborted
    TransactionAborted   = 500,

    /// Transaction already committed
    TransactionCommitted = 501,

    /// No active transaction
    NoTransaction        = 502,

    /// Transaction too large
    TransactionTooLarge  = 503,

    /// Journal full
    JournalFull          = 504,

    /// Journal replay required
    JournalReplayRequired = 505,

    /// Conflicting transaction
    TransactionConflict  = 506,

    /// Journal corrupted (alias)
    JournalCorrupted     = 507,

    /// Invalid journal size
    InvalidJournalSize   = 508,

    /// Invalid checkpoint
    InvalidCheckpoint    = 509,

    /// Invalid record type
    InvalidRecordType    = 510,

    /// Too many transactions
    TooManyTransactions  = 511,

    // ========================================================================
    // Snapshot Errors (600-699)
    // ========================================================================
    /// Snapshot not found
    SnapshotNotFound     = 600,

    /// Snapshot already exists
    SnapshotExists       = 601,

    /// Cannot delete active snapshot
    SnapshotInUse        = 602,

    /// Snapshot is read-only
    SnapshotReadOnly     = 603,

    /// Invalid snapshot ID
    InvalidSnapshot      = 604,

    /// Snapshot corrupted
    SnapshotCorrupted    = 605,

    /// Maximum snapshot depth exceeded
    SnapshotDepthExceeded = 606,

    /// Invalid snapshot ID (alias)
    InvalidSnapshotId    = 607,

    /// Snapshot has children (cannot delete)
    SnapshotHasChildren  = 608,

    /// Invalid version
    InvalidVersion       = 609,

    // ========================================================================
    // Crypto Errors (700-799)
    // ========================================================================
    /// Encryption key not found
    KeyNotFound          = 700,

    /// Decryption failed
    DecryptionFailed     = 701,

    /// Encryption failed
    EncryptionFailed     = 702,

    /// Key derivation failed
    KeyDerivationFailed  = 703,

    /// Invalid key
    InvalidKey           = 704,

    /// Authentication failed (AEAD tag mismatch)
    AuthenticationFailed = 705,

    /// Auth failed (alias)
    AuthFailed           = 707,

    /// Generic crypto error
    CryptoError          = 708,

    /// Crypto not initialized
    CryptoNotInitialized = 706,

    // ========================================================================
    // Compression Errors (800-899)
    // ========================================================================
    /// Decompression failed
    DecompressionFailed  = 800,

    /// Compression failed
    CompressionFailed    = 801,

    /// Invalid compressed data
    InvalidCompressedData = 802,

    /// Compression ratio too low (not worth compressing)
    CompressionNotWorth  = 803,

    // ========================================================================
    // Device Errors (900-999)
    // ========================================================================
    /// Device not ready
    DeviceNotReady       = 900,

    /// Device removed
    DeviceRemoved        = 901,

    /// Device I/O error
    DeviceIoError        = 902,

    /// I/O read error
    IoReadError          = 903,

    /// I/O write error
    IoWriteError         = 904,

    /// Device timeout
    DeviceTimeout        = 905,

    /// Device full
    DeviceFull           = 906,

    /// Invalid device
    InvalidDevice        = 907,

    // ========================================================================
    // Internal Errors (1000+)
    // ========================================================================
    /// Internal error (bug in filesystem code)
    Internal             = 1000,

    /// Assertion failed
    AssertionFailed      = 1001,

    /// Not implemented
    NotImplemented       = 1002,

    /// Lock poisoned
    LockPoisoned         = 1003,

    /// Invalid state
    InvalidState         = 1004,
}

impl HfsError {
    /// Convert error code to errno-compatible value
    pub const fn to_errno(self) -> i32 {
        match self {
            Self::Success => 0,
            Self::Io | Self::DeviceIoError => 5, // EIO
            Self::NoMemory => 12,                // ENOMEM
            Self::PermissionDenied | Self::AclDenied => 13, // EACCES
            Self::Busy => 16,                    // EBUSY
            Self::AlreadyExists => 17,           // EEXIST
            Self::CrossDevice => 18,             // EXDEV
            Self::NotDirectory => 20,            // ENOTDIR
            Self::IsDirectory => 21,             // EISDIR
            Self::InvalidArgument => 22,         // EINVAL
            Self::TooManyOpenFiles => 24,        // EMFILE
            Self::FileTooLarge | Self::MaxFileSizeExceeded => 27, // EFBIG
            Self::NoSpace | Self::DeviceFull => 28, // ENOSPC
            Self::ReadOnly => 30,                // EROFS
            Self::TooManyLinks => 31,            // EMLINK
            Self::SymlinkLoop => 40,             // ELOOP
            Self::NameTooLong => 36,             // ENAMETOOLONG
            Self::NotEmpty => 39,                // ENOTEMPTY
            Self::NotFound => 2,                 // ENOENT
            Self::NotPermitted | Self::Immutable | Self::AppendOnly => 1, // EPERM
            Self::Again | Self::WouldBlock => 11, // EAGAIN
            Self::TimedOut | Self::DeviceTimeout => 110, // ETIMEDOUT
            Self::Stale => 116,                  // ESTALE
            Self::QuotaExceeded => 122,          // EDQUOT
            Self::Deadlock => 35,                // EDEADLK
            Self::BadFileDescriptor => 9,        // EBADF
            Self::Interrupted => 4,              // EINTR
            Self::NotSupported | Self::NotImplemented => 95, // EOPNOTSUPP
            Self::NoInodes => 28,                // ENOSPC (no separate errno)
            _ => 5,                              // EIO for everything else
        }
    }

    /// Get human-readable error message
    pub const fn message(self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::Io => "I/O error",
            Self::NoMemory => "Out of memory",
            Self::InvalidArgument => "Invalid argument",
            Self::NotSupported => "Operation not supported",
            Self::WouldBlock => "Operation would block",
            Self::Interrupted => "Interrupted",
            Self::Again => "Try again",
            Self::InProgress => "Operation in progress",
            Self::Already => "Already in progress",
            Self::TimedOut => "Timed out",
            Self::Deadlock => "Deadlock would occur",
            Self::Busy => "Resource busy",
            Self::BufferTooSmall => "Buffer too small",
            Self::EndOfFile => "End of file",
            Self::NotFound => "No such file or directory",
            Self::AlreadyExists => "File exists",
            Self::NotDirectory => "Not a directory",
            Self::IsDirectory => "Is a directory",
            Self::NotEmpty => "Directory not empty",
            Self::TooManyOpenFiles => "Too many open files",
            Self::BadFileDescriptor => "Bad file descriptor",
            Self::NameTooLong => "File name too long",
            Self::SymlinkLoop => "Too many symbolic links",
            Self::InvalidPath => "Invalid path",
            Self::CrossDevice => "Cross-device link",
            Self::Stale => "Stale file handle",
            Self::FileTooLarge => "File too large",
            Self::InvalidFileType => "Invalid file type",
            Self::PermissionDenied => "Permission denied",
            Self::NotPermitted => "Operation not permitted",
            Self::ReadOnly => "Read-only filesystem",
            Self::Immutable => "File is immutable",
            Self::AppendOnly => "File is append-only",
            Self::AclDenied => "Access denied by ACL",
            Self::NoSpace => "No space left on device",
            Self::QuotaExceeded => "Disk quota exceeded",
            Self::NoInodes => "No inodes available",
            Self::ExtentTreeFull => "Extent tree full",
            Self::MaxFileSizeExceeded => "Maximum file size exceeded",
            Self::TooManySnapshots => "Too many snapshots",
            Self::TooManyLinks => "Too many hard links",
            Self::Corruption => "Data corruption detected",
            Self::ChecksumMismatch => "Checksum mismatch",
            Self::BadMagic => "Invalid magic number",
            Self::IncompatibleVersion => "Incompatible version",
            Self::MerkleVerifyFailed => "Merkle verification failed",
            Self::JournalCorruption => "Journal corruption",
            Self::SuperblockCorruption => "Superblock corruption",
            Self::ExtentTreeCorruption => "Extent tree corruption",
            Self::InodeCorruption => "Inode corruption",
            Self::DirectoryCorruption => "Directory corruption",
            Self::TransactionAborted => "Transaction aborted",
            Self::TransactionCommitted => "Transaction already committed",
            Self::NoTransaction => "No active transaction",
            Self::TransactionTooLarge => "Transaction too large",
            Self::JournalFull => "Journal full",
            Self::JournalReplayRequired => "Journal replay required",
            Self::TransactionConflict => "Transaction conflict",
            Self::SnapshotNotFound => "Snapshot not found",
            Self::SnapshotExists => "Snapshot already exists",
            Self::SnapshotInUse => "Snapshot in use",
            Self::SnapshotReadOnly => "Snapshot is read-only",
            Self::InvalidSnapshot => "Invalid snapshot",
            Self::KeyNotFound => "Encryption key not found",
            Self::DecryptionFailed => "Decryption failed",
            Self::EncryptionFailed => "Encryption failed",
            Self::KeyDerivationFailed => "Key derivation failed",
            Self::InvalidKey => "Invalid key",
            Self::AuthenticationFailed => "Authentication failed",
            Self::CryptoNotInitialized => "Crypto not initialized",
            Self::DecompressionFailed => "Decompression failed",
            Self::CompressionFailed => "Compression failed",
            Self::InvalidCompressedData => "Invalid compressed data",
            Self::CompressionNotWorth => "Compression not worthwhile",
            Self::DeviceNotReady => "Device not ready",
            Self::DeviceRemoved => "Device removed",
            Self::DeviceIoError => "Device I/O error",
            Self::DeviceTimeout => "Device timeout",
            Self::DeviceFull => "Device full",
            Self::InvalidDevice => "Invalid device",
            Self::Internal => "Internal error",
            Self::AssertionFailed => "Assertion failed",
            Self::NotImplemented => "Not implemented",
            Self::LockPoisoned => "Lock poisoned",
            Self::InvalidState => "Invalid state",
            // Aliases and additional error types
            Self::Overflow => "Arithmetic overflow",
            Self::Underflow => "Arithmetic underflow",
            Self::TooBig => "Value too big",
            Self::OutOfMemory => "Out of memory",
            Self::InvalidAlignment => "Invalid alignment",
            Self::InvalidData => "Invalid data",
            Self::Locked => "Resource locked",
            Self::NotInitialized => "Not initialized",
            Self::IntegrityError => "Integrity error",
            Self::InvalidParameter => "Invalid parameter",
            Self::InvalidArg => "Invalid argument",
            Self::BadHandle => "Bad handle",
            Self::InvalidHandle => "Invalid handle",
            Self::BadFd => "Bad file descriptor",
            Self::CannotDeleteRoot => "Cannot delete root",
            Self::DirectoryFull => "Directory full",
            Self::InvalidBlockNumber => "Invalid block number",
            Self::BtreeFull => "B-tree full",
            Self::BtreeTooDeep => "B-tree too deep",
            Self::RefcountOverflow => "Reference count overflow",
            Self::RefcountUnderflow => "Reference count underflow",
            Self::CorruptedData => "Corrupted data",
            Self::BtreeCorruption => "B-tree corruption",
            Self::BtreeKeyNotFound => "B-tree key not found",
            Self::ExtentCorruption => "Extent corruption",
            Self::ExtentNotFound => "Extent not found",
            Self::DirCorruption => "Directory corruption",
            Self::JournalCorrupted => "Journal corrupted",
            Self::InvalidJournalSize => "Invalid journal size",
            Self::InvalidCheckpoint => "Invalid checkpoint",
            Self::InvalidRecordType => "Invalid record type",
            Self::TooManyTransactions => "Too many transactions",
            Self::SnapshotCorrupted => "Snapshot corrupted",
            Self::SnapshotDepthExceeded => "Snapshot depth exceeded",
            Self::InvalidSnapshotId => "Invalid snapshot ID",
            Self::SnapshotHasChildren => "Snapshot has children",
            Self::InvalidVersion => "Invalid version",
            Self::AuthFailed => "Authentication failed",
            Self::CryptoError => "Crypto error",
            Self::IoReadError => "I/O read error",
            Self::IoWriteError => "I/O write error",
            Self::ReadOnlyFilesystem => "Read-only filesystem",
        }
    }

    /// Check if this error is recoverable
    pub const fn is_recoverable(self) -> bool {
        matches!(
            self,
            Self::WouldBlock | Self::Again | Self::Interrupted | Self::TimedOut | Self::Busy
        )
    }

    /// Check if this is a corruption error
    pub const fn is_corruption(self) -> bool {
        matches!(
            self,
            Self::Corruption
                | Self::ChecksumMismatch
                | Self::BadMagic
                | Self::MerkleVerifyFailed
                | Self::JournalCorruption
                | Self::SuperblockCorruption
                | Self::ExtentTreeCorruption
                | Self::InodeCorruption
                | Self::DirectoryCorruption
        )
    }

    /// Check if this is a space-related error
    pub const fn is_space_error(self) -> bool {
        matches!(
            self,
            Self::NoSpace
                | Self::NoInodes
                | Self::QuotaExceeded
                | Self::ExtentTreeFull
                | Self::MaxFileSizeExceeded
                | Self::JournalFull
        )
    }

    /// Check if this is a permission error
    pub const fn is_permission_error(self) -> bool {
        matches!(
            self,
            Self::PermissionDenied
                | Self::NotPermitted
                | Self::ReadOnly
                | Self::Immutable
                | Self::AppendOnly
                | Self::AclDenied
        )
    }
}

impl fmt::Display for HfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

// Note: std::error::Error is not available in no_std kernel context.
// Core::error::Error requires nightly feature.

// ============================================================================
// Error Context
// ============================================================================

/// Error with additional context information.
///
/// This type wraps an HfsError with optional context about where the error
/// occurred, useful for debugging and logging.
#[derive(Clone, Debug)]
pub struct HfsErrorContext {
    /// The underlying error
    pub error: HfsError,
    /// Operation that was being performed
    pub operation: &'static str,
    /// Optional inode number involved
    pub inode: Option<u64>,
    /// Optional block number involved
    pub block: Option<u64>,
}

impl HfsErrorContext {
    /// Create a new error context
    #[inline]
    pub const fn new(error: HfsError, operation: &'static str) -> Self {
        Self {
            error,
            operation,
            inode: None,
            block: None,
        }
    }

    /// Add inode context
    #[inline]
    pub const fn with_inode(mut self, ino: u64) -> Self {
        self.inode = Some(ino);
        self
    }

    /// Add block context
    #[inline]
    pub const fn with_block(mut self, block: u64) -> Self {
        self.block = Some(block);
        self
    }
}

impl fmt::Display for HfsErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.operation, self.error)?;
        if let Some(ino) = self.inode {
            write!(f, " (inode {})", ino)?;
        }
        if let Some(block) = self.block {
            write!(f, " (block {})", block)?;
        }
        Ok(())
    }
}

// ============================================================================
// Helper Macros
// ============================================================================

/// Return early if the result is an error.
#[macro_export]
macro_rules! hfs_try {
    ($expr:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => return Err(e),
        }
    };
}

/// Return early if the option is None.
#[macro_export]
macro_rules! hfs_some {
    ($expr:expr) => {
        match $expr {
            Some(v) => v,
            None => return Err($crate::core::error::HfsError::NotFound),
        }
    };
    ($expr:expr, $err:expr) => {
        match $expr {
            Some(v) => v,
            None => return Err($err),
        }
    };
}

/// Create an error result with context.
#[macro_export]
macro_rules! hfs_err {
    ($err:expr) => {
        Err($err)
    };
    ($err:expr, $op:expr) => {
        Err($crate::core::error::HfsErrorContext::new($err, $op))
    };
}

/// Ensure a condition is true, returning error if not.
#[macro_export]
macro_rules! hfs_ensure {
    ($cond:expr, $err:expr) => {
        if !($cond) {
            return Err($err);
        }
    };
}

/// Ensure a condition is true in debug builds.
#[macro_export]
macro_rules! hfs_debug_assert {
    ($cond:expr) => {
        #[cfg(debug_assertions)]
        if !($cond) {
            return Err($crate::core::error::HfsError::AssertionFailed);
        }
    };
}
