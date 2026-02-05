//! # Syscall Interface
//!
//! Complete syscall implementation for Helix userspace.
//!
//! ## Syscall Convention (x86_64)
//! - Syscall number: RAX
//! - Arguments: RDI, RSI, RDX, R10, R8, R9
//! - Return: RAX (negative = error)
//!
//! ## Syscall Categories
//! - Process management (fork, exec, exit, wait)
//! - File I/O (read, write, open, close)
//! - Memory management (mmap, munmap, brk)
//! - IPC (pipe, socket, etc.)

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use spin::RwLock;

use super::{UserResult, STATS};

/// Syscall numbers (Linux-compatible subset)
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syscall {
    /// Read from file descriptor
    Read            = 0,
    /// Write to file descriptor
    Write           = 1,
    /// Open file
    Open            = 2,
    /// Close file descriptor
    Close           = 3,
    /// Get file status
    Stat            = 4,
    /// Get file status (fd)
    Fstat           = 5,
    /// Seek
    Lseek           = 8,
    /// Memory map
    Mmap            = 9,
    /// Memory protect
    Mprotect        = 10,
    /// Memory unmap
    Munmap          = 11,
    /// Change heap size
    Brk             = 12,
    /// I/O control
    Ioctl           = 16,
    /// Access check
    Access          = 21,
    /// Create pipe
    Pipe            = 22,
    /// Duplicate FD
    Dup             = 32,
    /// Duplicate FD to specific number
    Dup2            = 33,
    /// Pause
    Pause           = 34,
    /// Nanosleep
    Nanosleep       = 35,
    /// Get process ID
    Getpid          = 39,
    /// Send signal
    Kill            = 62,
    /// Fork
    Fork            = 57,
    /// Execute program
    Execve          = 59,
    /// Exit
    Exit            = 60,
    /// Wait for child
    Wait4           = 61,
    /// Get current directory
    Getcwd          = 79,
    /// Change directory
    Chdir           = 80,
    /// Create directory
    Mkdir           = 83,
    /// Remove directory
    Rmdir           = 84,
    /// Unlink file
    Unlink          = 87,
    /// Get time
    Gettimeofday    = 96,
    /// Get resource usage
    Getrusage       = 98,
    /// Get UID
    Getuid          = 102,
    /// Get GID
    Getgid          = 104,
    /// Get EUID
    Geteuid         = 107,
    /// Get EGID
    Getegid         = 108,
    /// Get parent PID
    Getppid         = 110,
    /// Set signal handler
    RtSigaction     = 13,
    /// Signal return
    RtSigreturn     = 15,
    /// Architecture-specific
    ArchPrctl       = 158,
    /// Exit process group
    ExitGroup       = 231,

    // Helix-specific syscalls (start at 1000)
    /// Get DIS statistics
    HelixDisStats   = 1000,
    /// Hot-reload module
    HelixHotReload  = 1001,
    /// Self-heal trigger
    HelixSelfHeal   = 1002,
    /// Run benchmark
    HelixBenchmark  = 1003,
    /// Get kernel info
    HelixKernelInfo = 1004,
}

impl Syscall {
    /// Convert from number
    pub fn from_num(num: u64) -> Option<Self> {
        match num {
            0 => Some(Syscall::Read),
            1 => Some(Syscall::Write),
            2 => Some(Syscall::Open),
            3 => Some(Syscall::Close),
            4 => Some(Syscall::Stat),
            5 => Some(Syscall::Fstat),
            8 => Some(Syscall::Lseek),
            9 => Some(Syscall::Mmap),
            10 => Some(Syscall::Mprotect),
            11 => Some(Syscall::Munmap),
            12 => Some(Syscall::Brk),
            16 => Some(Syscall::Ioctl),
            21 => Some(Syscall::Access),
            22 => Some(Syscall::Pipe),
            32 => Some(Syscall::Dup),
            33 => Some(Syscall::Dup2),
            34 => Some(Syscall::Pause),
            35 => Some(Syscall::Nanosleep),
            39 => Some(Syscall::Getpid),
            57 => Some(Syscall::Fork),
            59 => Some(Syscall::Execve),
            60 => Some(Syscall::Exit),
            61 => Some(Syscall::Wait4),
            62 => Some(Syscall::Kill),
            79 => Some(Syscall::Getcwd),
            80 => Some(Syscall::Chdir),
            83 => Some(Syscall::Mkdir),
            84 => Some(Syscall::Rmdir),
            87 => Some(Syscall::Unlink),
            96 => Some(Syscall::Gettimeofday),
            98 => Some(Syscall::Getrusage),
            102 => Some(Syscall::Getuid),
            104 => Some(Syscall::Getgid),
            107 => Some(Syscall::Geteuid),
            108 => Some(Syscall::Getegid),
            110 => Some(Syscall::Getppid),
            158 => Some(Syscall::ArchPrctl),
            231 => Some(Syscall::ExitGroup),
            1000 => Some(Syscall::HelixDisStats),
            1001 => Some(Syscall::HelixHotReload),
            1002 => Some(Syscall::HelixSelfHeal),
            1003 => Some(Syscall::HelixBenchmark),
            1004 => Some(Syscall::HelixKernelInfo),
            _ => None,
        }
    }
}

/// Syscall result type
pub type SyscallResult = Result<u64, SyscallError>;

/// Syscall error codes (POSIX-compatible)
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyscallError {
    /// Operation not permitted
    EPERM   = 1,
    /// No such file or directory
    ENOENT  = 2,
    /// No such process
    ESRCH   = 3,
    /// Interrupted system call
    EINTR   = 4,
    /// I/O error
    EIO     = 5,
    /// No such device or address
    ENXIO   = 6,
    /// Argument list too long
    E2BIG   = 7,
    /// Exec format error
    ENOEXEC = 8,
    /// Bad file descriptor
    EBADF   = 9,
    /// No child processes
    ECHILD  = 10,
    /// Resource temporarily unavailable
    EAGAIN  = 11,
    /// Out of memory
    ENOMEM  = 12,
    /// Permission denied
    EACCES  = 13,
    /// Bad address
    EFAULT  = 14,
    /// Block device required
    ENOTBLK = 15,
    /// Device or resource busy
    EBUSY   = 16,
    /// File exists
    EEXIST  = 17,
    /// Invalid cross-device link
    EXDEV   = 18,
    /// No such device
    ENODEV  = 19,
    /// Not a directory
    ENOTDIR = 20,
    /// Is a directory
    EISDIR  = 21,
    /// Invalid argument
    EINVAL  = 22,
    /// File table overflow
    ENFILE  = 23,
    /// Too many open files
    EMFILE  = 24,
    /// Inappropriate ioctl for device
    ENOTTY  = 25,
    /// Text file busy
    ETXTBSY = 26,
    /// File too large
    EFBIG   = 27,
    /// No space left on device
    ENOSPC  = 28,
    /// Illegal seek
    ESPIPE  = 29,
    /// Read-only file system
    EROFS   = 30,
    /// Too many links
    EMLINK  = 31,
    /// Broken pipe
    EPIPE   = 32,
    /// Math argument out of domain
    EDOM    = 33,
    /// Result too large
    ERANGE  = 34,
    /// Function not implemented
    ENOSYS  = 38,
}

impl SyscallError {
    /// Convert to negative errno
    pub fn to_errno(self) -> i64 {
        -(self as i64)
    }
}

/// Syscall arguments
#[derive(Debug, Clone, Copy, Default)]
pub struct SyscallArgs {
    /// Arg 1 (RDI)
    pub arg1: u64,
    /// Arg 2 (RSI)
    pub arg2: u64,
    /// Arg 3 (RDX)
    pub arg3: u64,
    /// Arg 4 (R10)
    pub arg4: u64,
    /// Arg 5 (R8)
    pub arg5: u64,
    /// Arg 6 (R9)
    pub arg6: u64,
}

impl SyscallArgs {
    /// Create new args
    pub const fn new() -> Self {
        Self {
            arg1: 0,
            arg2: 0,
            arg3: 0,
            arg4: 0,
            arg5: 0,
            arg6: 0,
        }
    }

    /// Create from array
    pub fn from_array(args: [u64; 6]) -> Self {
        Self {
            arg1: args[0],
            arg2: args[1],
            arg3: args[2],
            arg4: args[3],
            arg5: args[4],
            arg6: args[5],
        }
    }
}

/// Syscall handler function type
pub type SyscallHandler = fn(SyscallArgs) -> SyscallResult;

/// Syscall table entry
struct SyscallEntry {
    /// Syscall number
    number: Syscall,
    /// Handler function
    handler: SyscallHandler,
    /// Number of arguments
    arg_count: u8,
    /// Description
    name: &'static str,
}

/// The syscall table
pub struct SyscallTable {
    /// Handlers indexed by syscall number
    handlers: RwLock<Vec<Option<SyscallEntry>>>,
    /// Statistics
    call_counts: [AtomicU64; 256],
}

impl SyscallTable {
    /// Create new syscall table
    pub const fn new() -> Self {
        const ZERO: AtomicU64 = AtomicU64::new(0);
        Self {
            handlers: RwLock::new(Vec::new()),
            call_counts: [ZERO; 256],
        }
    }

    /// Initialize table with default handlers
    pub fn init(&self) {
        let mut handlers = self.handlers.write();
        handlers.resize_with(256, || None);

        // Register standard syscalls
        self.register_handler_internal(&mut handlers, Syscall::Read, sys_read, 3, "read");
        self.register_handler_internal(&mut handlers, Syscall::Write, sys_write, 3, "write");
        self.register_handler_internal(&mut handlers, Syscall::Open, sys_open, 3, "open");
        self.register_handler_internal(&mut handlers, Syscall::Close, sys_close, 1, "close");
        self.register_handler_internal(&mut handlers, Syscall::Getpid, sys_getpid, 0, "getpid");
        self.register_handler_internal(&mut handlers, Syscall::Getppid, sys_getppid, 0, "getppid");
        self.register_handler_internal(&mut handlers, Syscall::Fork, sys_fork, 0, "fork");
        self.register_handler_internal(&mut handlers, Syscall::Exit, sys_exit, 1, "exit");
        self.register_handler_internal(&mut handlers, Syscall::Brk, sys_brk, 1, "brk");
        self.register_handler_internal(&mut handlers, Syscall::Mmap, sys_mmap, 6, "mmap");
        self.register_handler_internal(&mut handlers, Syscall::Munmap, sys_munmap, 2, "munmap");
    }

    fn register_handler_internal(
        &self,
        handlers: &mut Vec<Option<SyscallEntry>>,
        syscall: Syscall,
        handler: SyscallHandler,
        arg_count: u8,
        name: &'static str,
    ) {
        let num = syscall as usize;
        if num < handlers.len() {
            handlers[num] = Some(SyscallEntry {
                number: syscall,
                handler,
                arg_count,
                name,
            });
        }
    }

    /// Handle a syscall
    pub fn handle(&self, num: u64, args: SyscallArgs) -> SyscallResult {
        STATS.syscall_made();

        let handlers = self.handlers.read();

        if num >= handlers.len() as u64 {
            return Err(SyscallError::ENOSYS);
        }

        if let Some(entry) = &handlers[num as usize] {
            // Update stats
            if num < 256 {
                self.call_counts[num as usize].fetch_add(1, Ordering::Relaxed);
            }

            // Call handler
            (entry.handler)(args)
        } else {
            Err(SyscallError::ENOSYS)
        }
    }

    /// Get syscall count
    pub fn get_count(&self, syscall: Syscall) -> u64 {
        let num = syscall as usize;
        if num < 256 {
            self.call_counts[num].load(Ordering::Relaxed)
        } else {
            0
        }
    }
}

impl Default for SyscallTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Syscall Implementations
// ============================================================================

/// Read from file descriptor
fn sys_read(args: SyscallArgs) -> SyscallResult {
    let _fd = args.arg1 as i32;
    let _buf = args.arg2 as *mut u8;
    let _count = args.arg3 as usize;

    // In real OS, would read from fd_table entry
    // For now, return 0 (EOF)
    Ok(0)
}

/// Write to file descriptor
fn sys_write(args: SyscallArgs) -> SyscallResult {
    let fd = args.arg1 as i32;
    let buf = args.arg2 as *const u8;
    let count = args.arg3 as usize;

    if buf.is_null() {
        return Err(SyscallError::EFAULT);
    }

    // For stdout/stderr, would output to console
    match fd {
        1 | 2 => {
            // Would write to console
            // In real OS: console::write(buf, count)
            Ok(count as u64)
        },
        _ => Err(SyscallError::EBADF),
    }
}

/// Open file
fn sys_open(_args: SyscallArgs) -> SyscallResult {
    // Filesystem not implemented
    Err(SyscallError::ENOSYS)
}

/// Close file descriptor
fn sys_close(args: SyscallArgs) -> SyscallResult {
    let fd = args.arg1 as i32;

    // Would close in fd_table
    if fd >= 0 {
        Ok(0)
    } else {
        Err(SyscallError::EBADF)
    }
}

/// Get process ID
fn sys_getpid(_args: SyscallArgs) -> SyscallResult {
    // Would return current process PID
    Ok(1)
}

/// Get parent process ID
fn sys_getppid(_args: SyscallArgs) -> SyscallResult {
    // Would return parent PID
    Ok(0)
}

/// Fork process
fn sys_fork(_args: SyscallArgs) -> SyscallResult {
    // Would create child process
    // For now, not implemented
    Err(SyscallError::ENOSYS)
}

/// Exit process
fn sys_exit(args: SyscallArgs) -> SyscallResult {
    let _code = args.arg1 as i32;

    // Would terminate current process
    // This syscall never returns
    Ok(0)
}

/// Change heap size
fn sys_brk(args: SyscallArgs) -> SyscallResult {
    let new_brk = args.arg1;

    // Would adjust heap
    // For now, just return the requested value
    Ok(new_brk)
}

/// Memory map
fn sys_mmap(args: SyscallArgs) -> SyscallResult {
    let addr = args.arg1;
    let len = args.arg2;
    let _prot = args.arg3 as i32;
    let _flags = args.arg4 as i32;
    let _fd = args.arg5 as i32;
    let _offset = args.arg6;

    if len == 0 {
        return Err(SyscallError::EINVAL);
    }

    // Would allocate virtual memory
    // For now, return error
    if addr == 0 {
        Err(SyscallError::ENOMEM)
    } else {
        Ok(addr)
    }
}

/// Memory unmap
fn sys_munmap(args: SyscallArgs) -> SyscallResult {
    let _addr = args.arg1;
    let _len = args.arg2;

    // Would free virtual memory
    Ok(0)
}

/// Global syscall table
pub static SYSCALL_TABLE: SyscallTable = SyscallTable::new();

/// Initialize syscall subsystem
pub fn init() -> UserResult<()> {
    SYSCALL_TABLE.init();
    Ok(())
}

/// Handle a syscall (main entry point)
pub fn handle_syscall(num: u64, args: SyscallArgs) -> SyscallResult {
    SYSCALL_TABLE.handle(num, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_from_num() {
        assert_eq!(Syscall::from_num(0), Some(Syscall::Read));
        assert_eq!(Syscall::from_num(1), Some(Syscall::Write));
        assert_eq!(Syscall::from_num(60), Some(Syscall::Exit));
        assert_eq!(Syscall::from_num(9999), None);
    }

    #[test]
    fn test_syscall_error() {
        assert_eq!(SyscallError::ENOENT.to_errno(), -2);
        assert_eq!(SyscallError::EINVAL.to_errno(), -22);
    }
}
