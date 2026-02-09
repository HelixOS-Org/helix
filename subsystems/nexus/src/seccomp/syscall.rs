//! Syscall Definitions
//!
//! Syscall numbers, categories, and risk levels.

/// Syscall number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyscallNum(pub u32);

impl SyscallNum {
    /// Create new syscall number
    #[inline(always)]
    pub const fn new(num: u32) -> Self {
        Self(num)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }

    // Common x86_64 syscalls
    pub const READ: Self = Self(0);
    pub const WRITE: Self = Self(1);
    pub const OPEN: Self = Self(2);
    pub const CLOSE: Self = Self(3);
    pub const STAT: Self = Self(4);
    pub const FSTAT: Self = Self(5);
    pub const LSTAT: Self = Self(6);
    pub const POLL: Self = Self(7);
    pub const LSEEK: Self = Self(8);
    pub const MMAP: Self = Self(9);
    pub const MPROTECT: Self = Self(10);
    pub const MUNMAP: Self = Self(11);
    pub const BRK: Self = Self(12);
    pub const IOCTL: Self = Self(16);
    pub const ACCESS: Self = Self(21);
    pub const PIPE: Self = Self(22);
    pub const DUP: Self = Self(32);
    pub const DUP2: Self = Self(33);
    pub const SOCKET: Self = Self(41);
    pub const CONNECT: Self = Self(42);
    pub const ACCEPT: Self = Self(43);
    pub const SENDTO: Self = Self(44);
    pub const RECVFROM: Self = Self(45);
    pub const BIND: Self = Self(49);
    pub const LISTEN: Self = Self(50);
    pub const CLONE: Self = Self(56);
    pub const FORK: Self = Self(57);
    pub const VFORK: Self = Self(58);
    pub const EXECVE: Self = Self(59);
    pub const EXIT: Self = Self(60);
    pub const WAIT4: Self = Self(61);
    pub const KILL: Self = Self(62);
    pub const FCNTL: Self = Self(72);
    pub const FLOCK: Self = Self(73);
    pub const GETUID: Self = Self(102);
    pub const GETGID: Self = Self(104);
    pub const SETUID: Self = Self(105);
    pub const SETGID: Self = Self(106);
    pub const GETEUID: Self = Self(107);
    pub const GETEGID: Self = Self(108);
    pub const PTRACE: Self = Self(101);
    pub const GETPID: Self = Self(39);
    pub const GETPPID: Self = Self(110);
    pub const SETSID: Self = Self(112);
    pub const SETREUID: Self = Self(113);
    pub const SETREGID: Self = Self(114);
    pub const PRCTL: Self = Self(157);
    pub const MOUNT: Self = Self(165);
    pub const UMOUNT2: Self = Self(166);
    pub const INIT_MODULE: Self = Self(175);
    pub const DELETE_MODULE: Self = Self(176);
    pub const REBOOT: Self = Self(169);
    pub const SETHOSTNAME: Self = Self(170);
    pub const SETDOMAINNAME: Self = Self(171);
    pub const KEXEC_LOAD: Self = Self(246);
}

/// Syscall category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyscallCategory {
    /// File operations
    File,
    /// Process operations
    Process,
    /// Memory operations
    Memory,
    /// Network operations
    Network,
    /// IPC operations
    Ipc,
    /// Signal operations
    Signal,
    /// System operations
    System,
    /// Namespace operations
    Namespace,
    /// Security operations
    Security,
    /// Device operations
    Device,
    /// Time operations
    Time,
    /// Debugging
    Debug,
    /// Unknown
    Unknown,
}

impl SyscallCategory {
    /// Get category name
    pub fn name(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Process => "process",
            Self::Memory => "memory",
            Self::Network => "network",
            Self::Ipc => "ipc",
            Self::Signal => "signal",
            Self::System => "system",
            Self::Namespace => "namespace",
            Self::Security => "security",
            Self::Device => "device",
            Self::Time => "time",
            Self::Debug => "debug",
            Self::Unknown => "unknown",
        }
    }
}

/// Syscall risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// Safe (getpid, read from fd, etc.)
    Safe,
    /// Low risk (normal file operations)
    Low,
    /// Medium risk (network, IPC)
    Medium,
    /// High risk (privilege changes, ptrace)
    High,
    /// Critical (kernel module, mount, reboot)
    Critical,
}

impl RiskLevel {
    /// Get level name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Safe => "safe",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// Get numeric score
    #[inline]
    pub fn score(&self) -> u8 {
        match self {
            Self::Safe => 0,
            Self::Low => 2,
            Self::Medium => 5,
            Self::High => 8,
            Self::Critical => 10,
        }
    }
}

/// Syscall information
#[derive(Debug, Clone)]
pub struct SyscallInfo {
    /// Syscall number
    pub num: SyscallNum,
    /// Name
    pub name: &'static str,
    /// Category
    pub category: SyscallCategory,
    /// Risk level
    pub risk: RiskLevel,
    /// Description
    pub description: &'static str,
    /// Argument count
    pub arg_count: u8,
}

impl SyscallInfo {
    /// Create new syscall info
    #[inline]
    pub const fn new(
        num: SyscallNum,
        name: &'static str,
        category: SyscallCategory,
        risk: RiskLevel,
        description: &'static str,
        arg_count: u8,
    ) -> Self {
        Self {
            num,
            name,
            category,
            risk,
            description,
            arg_count,
        }
    }
}
