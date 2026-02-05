//! # Syscall Framework
//!
//! Provides the infrastructure for handling system calls.

pub mod dispatcher;
pub mod gateway;
pub mod registry;
pub mod validation;

use crate::KernelError;

/// System call number type
pub type SyscallNumber = u64;

/// Maximum number of syscall arguments
pub const MAX_SYSCALL_ARGS: usize = 6;

/// Syscall arguments
#[derive(Debug, Clone, Copy, Default)]
pub struct SyscallArgs {
    /// Arguments (up to 6)
    pub args: [u64; MAX_SYSCALL_ARGS],
    /// Number of valid arguments
    pub count: usize,
}

impl SyscallArgs {
    /// Create new syscall arguments
    pub const fn new() -> Self {
        Self {
            args: [0; MAX_SYSCALL_ARGS],
            count: 0,
        }
    }

    /// Create from a slice
    pub fn from_slice(args: &[u64]) -> Self {
        let mut result = Self::new();
        let count = args.len().min(MAX_SYSCALL_ARGS);
        result.args[..count].copy_from_slice(&args[..count]);
        result.count = count;
        result
    }

    /// Get an argument by index
    pub fn get(&self, index: usize) -> Option<u64> {
        if index < self.count {
            Some(self.args[index])
        } else {
            None
        }
    }

    /// Get an argument or default
    pub fn get_or(&self, index: usize, default: u64) -> u64 {
        self.get(index).unwrap_or(default)
    }
}

/// Syscall return value
#[derive(Debug, Clone, Copy)]
pub enum SyscallReturn {
    /// Success with return value
    Success(u64),
    /// Error with error code
    Error(SyscallError),
}

impl SyscallReturn {
    /// Convert to a raw return value (for returning to userspace)
    pub fn to_raw(self) -> i64 {
        match self {
            SyscallReturn::Success(v) => v as i64,
            SyscallReturn::Error(e) => -(e as i64),
        }
    }
}

/// Syscall error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub enum SyscallError {
    /// Operation not permitted
    NotPermitted      = 1,
    /// No such file or directory
    NoEntry           = 2,
    /// No such process
    NoProcess         = 3,
    /// Interrupted system call
    Interrupted       = 4,
    /// I/O error
    IoError           = 5,
    /// Bad file descriptor
    BadFd             = 9,
    /// Try again
    TryAgain          = 11,
    /// Out of memory
    OutOfMemory       = 12,
    /// Permission denied
    PermissionDenied  = 13,
    /// Bad address
    BadAddress        = 14,
    /// Resource busy
    Busy              = 16,
    /// File exists
    Exists            = 17,
    /// Invalid argument
    InvalidArgument   = 22,
    /// Too many open files
    TooManyFiles      = 24,
    /// No space left on device
    NoSpace           = 28,
    /// Function not implemented
    NotImplemented    = 38,
    /// Operation would block (same as TryAgain in POSIX, using different value here)
    WouldBlock        = 35,
    /// Connection refused
    ConnectionRefused = 111,
    /// Timed out
    TimedOut          = 110,
}

impl From<KernelError> for SyscallError {
    fn from(e: KernelError) -> Self {
        match e {
            KernelError::NotPermitted => SyscallError::NotPermitted,
            KernelError::NotFound => SyscallError::NoEntry,
            KernelError::AlreadyExists => SyscallError::Exists,
            KernelError::InvalidArgument => SyscallError::InvalidArgument,
            KernelError::WouldBlock => SyscallError::WouldBlock,
            KernelError::Busy => SyscallError::Busy,
            KernelError::OutOfMemory => SyscallError::OutOfMemory,
            KernelError::Timeout => SyscallError::TimedOut,
            KernelError::Interrupted => SyscallError::Interrupted,
            KernelError::IoError => SyscallError::IoError,
            KernelError::NotImplemented => SyscallError::NotImplemented,
            _ => SyscallError::InvalidArgument,
        }
    }
}

/// Syscall handler trait
pub trait SyscallHandler: Send + Sync {
    /// Handle a syscall
    fn handle(&self, args: &SyscallArgs) -> SyscallReturn;

    /// Get the syscall name (for debugging)
    fn name(&self) -> &'static str;

    /// Get the expected argument count
    fn arg_count(&self) -> usize;

    /// Validate arguments before handling
    fn validate(&self, _args: &SyscallArgs) -> Result<(), SyscallError> {
        Ok(())
    }
}

/// Syscall context (information about the calling context)
#[derive(Debug, Clone)]
pub struct SyscallContext {
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// User ID
    pub uid: u64,
    /// Group ID
    pub gid: u64,
    /// Calling from user mode?
    pub from_user: bool,
}
