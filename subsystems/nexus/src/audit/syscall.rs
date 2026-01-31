//! Syscall Information
//!
//! Syscall number definitions and information structures.

/// Syscall number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyscallNum(pub u32);

impl SyscallNum {
    /// Create new syscall number
    pub const fn new(num: u32) -> Self {
        Self(num)
    }

    /// Get raw value
    pub const fn raw(&self) -> u32 {
        self.0
    }

    // Common syscall numbers (x86_64)
    pub const READ: Self = Self(0);
    pub const WRITE: Self = Self(1);
    pub const OPEN: Self = Self(2);
    pub const CLOSE: Self = Self(3);
    pub const STAT: Self = Self(4);
    pub const FSTAT: Self = Self(5);
    pub const MMAP: Self = Self(9);
    pub const MPROTECT: Self = Self(10);
    pub const EXECVE: Self = Self(59);
    pub const EXIT: Self = Self(60);
    pub const KILL: Self = Self(62);
    pub const SETUID: Self = Self(105);
    pub const SETGID: Self = Self(106);
    pub const SOCKET: Self = Self(41);
    pub const CONNECT: Self = Self(42);
    pub const ACCEPT: Self = Self(43);
    pub const BIND: Self = Self(49);
    pub const LISTEN: Self = Self(50);
    pub const CLONE: Self = Self(56);
    pub const FORK: Self = Self(57);
    pub const PTRACE: Self = Self(101);
    pub const INIT_MODULE: Self = Self(175);
    pub const DELETE_MODULE: Self = Self(176);
}

/// Syscall information
#[derive(Debug, Clone)]
pub struct SyscallInfo {
    /// Syscall number
    pub syscall: SyscallNum,
    /// Argument 0
    pub a0: u64,
    /// Argument 1
    pub a1: u64,
    /// Argument 2
    pub a2: u64,
    /// Argument 3
    pub a3: u64,
    /// Argument 4
    pub a4: u64,
    /// Exit value
    pub exit: i64,
    /// Items (related records)
    pub items: u32,
}

impl SyscallInfo {
    /// Create new syscall info
    pub fn new(syscall: SyscallNum) -> Self {
        Self {
            syscall,
            a0: 0,
            a1: 0,
            a2: 0,
            a3: 0,
            a4: 0,
            exit: 0,
            items: 0,
        }
    }

    /// Was syscall successful
    pub fn is_success(&self) -> bool {
        self.exit >= 0
    }

    /// Set arguments
    pub fn with_args(mut self, a0: u64, a1: u64, a2: u64, a3: u64, a4: u64) -> Self {
        self.a0 = a0;
        self.a1 = a1;
        self.a2 = a2;
        self.a3 = a3;
        self.a4 = a4;
        self
    }

    /// Set exit value
    pub fn with_exit(mut self, exit: i64) -> Self {
        self.exit = exit;
        self
    }
}
