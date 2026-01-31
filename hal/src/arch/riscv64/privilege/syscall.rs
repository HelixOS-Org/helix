//! # RISC-V System Call Handling
//!
//! This module provides system call (ECALL) handling for RISC-V.
//!
//! ## System Call Convention
//!
//! RISC-V follows a convention similar to Linux:
//!
//! - **a7**: System call number
//! - **a0-a5**: Arguments (6 arguments max)
//! - **a0**: Return value (or negative error code)
//! - **a1**: Secondary return value (for some calls)
//!
//! ## ECALL Instruction
//!
//! The ECALL instruction causes an exception:
//! - From U-mode: cause = 8 (ECALL_FROM_U)
//! - From S-mode: cause = 9 (ECALL_FROM_S)
//! - From M-mode: cause = 11 (ECALL_FROM_M)

use super::traps::TrapFrame;

// ============================================================================
// Syscall Arguments
// ============================================================================

/// System call arguments
#[derive(Debug, Clone, Copy)]
pub struct SyscallArgs {
    /// System call number (from a7)
    pub syscall: u64,
    /// First argument (a0)
    pub arg0: u64,
    /// Second argument (a1)
    pub arg1: u64,
    /// Third argument (a2)
    pub arg2: u64,
    /// Fourth argument (a3)
    pub arg3: u64,
    /// Fifth argument (a4)
    pub arg4: u64,
    /// Sixth argument (a5)
    pub arg5: u64,
}

impl SyscallArgs {
    /// Extract syscall arguments from trap frame
    pub fn from_frame(frame: &TrapFrame) -> Self {
        Self {
            syscall: frame.regs.a7,
            arg0: frame.regs.a0,
            arg1: frame.regs.a1,
            arg2: frame.regs.a2,
            arg3: frame.regs.a3,
            arg4: frame.regs.a4,
            arg5: frame.regs.a5,
        }
    }

    /// Get arguments as array
    pub fn as_array(&self) -> [u64; 6] {
        [
            self.arg0,
            self.arg1,
            self.arg2,
            self.arg3,
            self.arg4,
            self.arg5,
        ]
    }

    /// Get argument by index
    pub fn get(&self, index: usize) -> Option<u64> {
        match index {
            0 => Some(self.arg0),
            1 => Some(self.arg1),
            2 => Some(self.arg2),
            3 => Some(self.arg3),
            4 => Some(self.arg4),
            5 => Some(self.arg5),
            _ => None,
        }
    }
}

// ============================================================================
// Syscall Return Value
// ============================================================================

/// System call return value
#[derive(Debug, Clone, Copy)]
pub struct SyscallReturn {
    /// Primary return value (a0)
    pub value: u64,
    /// Secondary return value (a1) - used for some calls
    pub value2: u64,
}

impl SyscallReturn {
    /// Create success return
    pub const fn success(value: u64) -> Self {
        Self { value, value2: 0 }
    }

    /// Create error return (negative errno)
    pub const fn error(errno: i64) -> Self {
        Self {
            value: errno as u64,
            value2: 0,
        }
    }

    /// Create return with two values
    pub const fn pair(value: u64, value2: u64) -> Self {
        Self { value, value2 }
    }

    /// Apply return value to trap frame
    pub fn apply_to_frame(&self, frame: &mut TrapFrame) {
        frame.regs.a0 = self.value;
        frame.regs.a1 = self.value2;
    }
}

// ============================================================================
// Syscall Handler Type
// ============================================================================

/// Type of syscall handler function
pub type SyscallHandlerFn = fn(&SyscallArgs, &mut TrapFrame) -> SyscallReturn;

// ============================================================================
// Syscall Table
// ============================================================================

/// Maximum number of syscalls
pub const MAX_SYSCALLS: usize = 512;

/// Syscall table entry
#[derive(Clone, Copy)]
pub struct SyscallEntry {
    /// Handler function
    pub handler: Option<SyscallHandlerFn>,
    /// Syscall name (for debugging)
    pub name: &'static str,
    /// Number of arguments
    pub nargs: u8,
}

impl SyscallEntry {
    /// Create an empty entry
    pub const fn empty() -> Self {
        Self {
            handler: None,
            name: "unknown",
            nargs: 0,
        }
    }

    /// Create an entry with a handler
    pub const fn new(handler: SyscallHandlerFn, name: &'static str, nargs: u8) -> Self {
        Self {
            handler: Some(handler),
            name,
            nargs,
        }
    }
}

/// Syscall table
pub struct SyscallTable {
    entries: [SyscallEntry; MAX_SYSCALLS],
}

impl SyscallTable {
    /// Create an empty syscall table
    pub const fn new() -> Self {
        Self {
            entries: [SyscallEntry::empty(); MAX_SYSCALLS],
        }
    }

    /// Register a syscall handler
    pub fn register(&mut self, num: usize, entry: SyscallEntry) {
        if num < MAX_SYSCALLS {
            self.entries[num] = entry;
        }
    }

    /// Get a syscall entry
    pub fn get(&self, num: usize) -> Option<&SyscallEntry> {
        if num < MAX_SYSCALLS {
            Some(&self.entries[num])
        } else {
            None
        }
    }

    /// Execute a syscall
    pub fn execute(&self, num: usize, args: &SyscallArgs, frame: &mut TrapFrame) -> SyscallReturn {
        if let Some(entry) = self.get(num) {
            if let Some(handler) = entry.handler {
                handler(args, frame)
            } else {
                SyscallReturn::error(-ENOSYS)
            }
        } else {
            SyscallReturn::error(-ENOSYS)
        }
    }
}

impl Default for SyscallTable {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Global Syscall Table
// ============================================================================

/// Global syscall table
static mut SYSCALL_TABLE: SyscallTable = SyscallTable::new();

/// Get reference to the global syscall table
pub fn get_syscall_table() -> &'static SyscallTable {
    unsafe { &SYSCALL_TABLE }
}

/// Get mutable reference to the global syscall table
///
/// # Safety
/// Must ensure exclusive access
pub unsafe fn get_syscall_table_mut() -> &'static mut SyscallTable {
    &mut SYSCALL_TABLE
}

/// Register a syscall
pub fn register_syscall(num: usize, handler: SyscallHandlerFn, name: &'static str, nargs: u8) {
    unsafe {
        get_syscall_table_mut().register(num, SyscallEntry::new(handler, name, nargs));
    }
}

// ============================================================================
// Main Syscall Handler
// ============================================================================

/// Main syscall handler
///
/// Called from the trap handler when an ECALL from U-mode is detected.
pub fn syscall_handler(frame: &mut TrapFrame) -> SyscallReturn {
    let args = SyscallArgs::from_frame(frame);
    let table = get_syscall_table();

    let result = table.execute(args.syscall as usize, &args, frame);
    result.apply_to_frame(frame);

    result
}

// ============================================================================
// Error Codes (Linux-compatible)
// ============================================================================

/// Operation not permitted
pub const EPERM: i64 = 1;
/// No such file or directory
pub const ENOENT: i64 = 2;
/// No such process
pub const ESRCH: i64 = 3;
/// Interrupted system call
pub const EINTR: i64 = 4;
/// I/O error
pub const EIO: i64 = 5;
/// No such device or address
pub const ENXIO: i64 = 6;
/// Argument list too long
pub const E2BIG: i64 = 7;
/// Exec format error
pub const ENOEXEC: i64 = 8;
/// Bad file number
pub const EBADF: i64 = 9;
/// No child processes
pub const ECHILD: i64 = 10;
/// Try again
pub const EAGAIN: i64 = 11;
/// Out of memory
pub const ENOMEM: i64 = 12;
/// Permission denied
pub const EACCES: i64 = 13;
/// Bad address
pub const EFAULT: i64 = 14;
/// Device or resource busy
pub const EBUSY: i64 = 16;
/// File exists
pub const EEXIST: i64 = 17;
/// Invalid argument
pub const EINVAL: i64 = 22;
/// No space left on device
pub const ENOSPC: i64 = 28;
/// Function not implemented
pub const ENOSYS: i64 = 38;

// ============================================================================
// Common Syscall Numbers (Linux RISC-V ABI)
// ============================================================================

pub mod syscall_nr {
    /// io_setup
    pub const IO_SETUP: usize = 0;
    /// io_destroy
    pub const IO_DESTROY: usize = 1;
    /// io_submit
    pub const IO_SUBMIT: usize = 2;
    /// io_cancel
    pub const IO_CANCEL: usize = 3;
    /// io_getevents
    pub const IO_GETEVENTS: usize = 4;
    /// setxattr
    pub const SETXATTR: usize = 5;
    /// getcwd
    pub const GETCWD: usize = 17;
    /// dup
    pub const DUP: usize = 23;
    /// dup3
    pub const DUP3: usize = 24;
    /// fcntl
    pub const FCNTL: usize = 25;
    /// ioctl
    pub const IOCTL: usize = 29;
    /// flock
    pub const FLOCK: usize = 32;
    /// mknodat
    pub const MKNODAT: usize = 33;
    /// mkdirat
    pub const MKDIRAT: usize = 34;
    /// unlinkat
    pub const UNLINKAT: usize = 35;
    /// symlinkat
    pub const SYMLINKAT: usize = 36;
    /// linkat
    pub const LINKAT: usize = 37;
    /// renameat
    pub const RENAMEAT: usize = 38;
    /// umount2
    pub const UMOUNT2: usize = 39;
    /// mount
    pub const MOUNT: usize = 40;
    /// statfs
    pub const STATFS: usize = 43;
    /// ftruncate
    pub const FTRUNCATE: usize = 46;
    /// faccessat
    pub const FACCESSAT: usize = 48;
    /// chdir
    pub const CHDIR: usize = 49;
    /// fchmod
    pub const FCHMOD: usize = 52;
    /// fchmodat
    pub const FCHMODAT: usize = 53;
    /// fchownat
    pub const FCHOWNAT: usize = 54;
    /// fchown
    pub const FCHOWN: usize = 55;
    /// openat
    pub const OPENAT: usize = 56;
    /// close
    pub const CLOSE: usize = 57;
    /// pipe2
    pub const PIPE2: usize = 59;
    /// lseek
    pub const LSEEK: usize = 62;
    /// read
    pub const READ: usize = 63;
    /// write
    pub const WRITE: usize = 64;
    /// readv
    pub const READV: usize = 65;
    /// writev
    pub const WRITEV: usize = 66;
    /// pread64
    pub const PREAD64: usize = 67;
    /// pwrite64
    pub const PWRITE64: usize = 68;
    /// ppoll
    pub const PPOLL: usize = 73;
    /// readlinkat
    pub const READLINKAT: usize = 78;
    /// fstatat
    pub const FSTATAT: usize = 79;
    /// fstat
    pub const FSTAT: usize = 80;
    /// fsync
    pub const FSYNC: usize = 82;
    /// exit
    pub const EXIT: usize = 93;
    /// exit_group
    pub const EXIT_GROUP: usize = 94;
    /// set_tid_address
    pub const SET_TID_ADDRESS: usize = 96;
    /// nanosleep
    pub const NANOSLEEP: usize = 101;
    /// clock_gettime
    pub const CLOCK_GETTIME: usize = 113;
    /// sched_yield
    pub const SCHED_YIELD: usize = 124;
    /// kill
    pub const KILL: usize = 129;
    /// tkill
    pub const TKILL: usize = 130;
    /// rt_sigaction
    pub const RT_SIGACTION: usize = 134;
    /// rt_sigprocmask
    pub const RT_SIGPROCMASK: usize = 135;
    /// rt_sigreturn
    pub const RT_SIGRETURN: usize = 139;
    /// setpriority
    pub const SETPRIORITY: usize = 140;
    /// getpriority
    pub const GETPRIORITY: usize = 141;
    /// times
    pub const TIMES: usize = 153;
    /// setpgid
    pub const SETPGID: usize = 154;
    /// getpgid
    pub const GETPGID: usize = 155;
    /// uname
    pub const UNAME: usize = 160;
    /// getrlimit
    pub const GETRLIMIT: usize = 163;
    /// setrlimit
    pub const SETRLIMIT: usize = 164;
    /// getrusage
    pub const GETRUSAGE: usize = 165;
    /// umask
    pub const UMASK: usize = 166;
    /// gettimeofday
    pub const GETTIMEOFDAY: usize = 169;
    /// getpid
    pub const GETPID: usize = 172;
    /// getppid
    pub const GETPPID: usize = 173;
    /// getuid
    pub const GETUID: usize = 174;
    /// geteuid
    pub const GETEUID: usize = 175;
    /// getgid
    pub const GETGID: usize = 176;
    /// getegid
    pub const GETEGID: usize = 177;
    /// gettid
    pub const GETTID: usize = 178;
    /// sysinfo
    pub const SYSINFO: usize = 179;
    /// mq_open
    pub const MQ_OPEN: usize = 180;
    /// socket
    pub const SOCKET: usize = 198;
    /// bind
    pub const BIND: usize = 200;
    /// listen
    pub const LISTEN: usize = 201;
    /// accept
    pub const ACCEPT: usize = 202;
    /// connect
    pub const CONNECT: usize = 203;
    /// sendto
    pub const SENDTO: usize = 206;
    /// recvfrom
    pub const RECVFROM: usize = 207;
    /// brk
    pub const BRK: usize = 214;
    /// mmap
    pub const MMAP: usize = 222;
    /// mprotect
    pub const MPROTECT: usize = 226;
    /// munmap
    pub const MUNMAP: usize = 215;
    /// madvise
    pub const MADVISE: usize = 233;
    /// wait4
    pub const WAIT4: usize = 260;
    /// clone
    pub const CLONE: usize = 220;
    /// execve
    pub const EXECVE: usize = 221;
}

// ============================================================================
// Fast Path Syscalls
// ============================================================================

/// Check if syscall can use fast path (no blocking, no scheduler involvement)
pub fn is_fast_path_syscall(num: usize) -> bool {
    matches!(
        num,
        syscall_nr::GETPID
            | syscall_nr::GETTID
            | syscall_nr::GETUID
            | syscall_nr::GETEUID
            | syscall_nr::GETGID
            | syscall_nr::GETEGID
            | syscall_nr::GETPPID
    )
}

// ============================================================================
// Syscall Auditing
// ============================================================================

/// Syscall audit record
#[derive(Debug, Clone)]
pub struct SyscallAudit {
    /// Syscall number
    pub syscall: u64,
    /// Arguments
    pub args: [u64; 6],
    /// Return value
    pub result: i64,
    /// Process ID
    pub pid: u32,
    /// Thread ID
    pub tid: u32,
}

/// Audit callback type
pub type AuditCallback = fn(&SyscallAudit);

/// Global audit callback
static mut AUDIT_CALLBACK: Option<AuditCallback> = None;

/// Set the audit callback
pub fn set_audit_callback(callback: AuditCallback) {
    unsafe {
        AUDIT_CALLBACK = Some(callback);
    }
}

/// Clear the audit callback
pub fn clear_audit_callback() {
    unsafe {
        AUDIT_CALLBACK = None;
    }
}

/// Audit a syscall (called after execution)
pub fn audit_syscall(syscall: u64, args: [u64; 6], result: i64, pid: u32, tid: u32) {
    if let Some(callback) = unsafe { AUDIT_CALLBACK } {
        callback(&SyscallAudit {
            syscall,
            args,
            result,
            pid,
            tid,
        });
    }
}
