//! Seccomp Core Types
//!
//! Fundamental types for seccomp filtering.

/// Filter ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FilterId(pub u64);

impl FilterId {
    /// Create new filter ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Profile ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProfileId(pub u64);

impl ProfileId {
    /// Create new profile ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Process ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pid(pub u32);

impl Pid {
    /// Create new PID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// Seccomp mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompMode {
    /// Disabled
    Disabled,
    /// Strict mode (only read/write/exit/_exit/sigreturn)
    Strict,
    /// Filter mode (BPF-based)
    Filter,
}

impl SeccompMode {
    /// Get mode name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Strict => "strict",
            Self::Filter => "filter",
        }
    }
}

/// Filter action
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FilterAction {
    /// Kill process
    Kill,
    /// Kill thread
    KillThread,
    /// Trap (send SIGSYS)
    Trap,
    /// Return errno
    Errno(u16),
    /// Trace (notify tracer)
    Trace(u16),
    /// Log (log and allow)
    Log,
    /// Allow
    Allow,
    /// Notify (user notification)
    Notify,
}

impl FilterAction {
    /// Get action name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Kill => "kill_process",
            Self::KillThread => "kill_thread",
            Self::Trap => "trap",
            Self::Errno(_) => "errno",
            Self::Trace(_) => "trace",
            Self::Log => "log",
            Self::Allow => "allow",
            Self::Notify => "notify",
        }
    }

    /// Get action severity (for security scoring)
    pub fn severity(&self) -> u8 {
        match self {
            Self::Kill => 10,
            Self::KillThread => 9,
            Self::Trap => 7,
            Self::Errno(_) => 5,
            Self::Trace(_) => 4,
            Self::Log => 2,
            Self::Allow => 0,
            Self::Notify => 3,
        }
    }

    /// Is blocking action
    #[inline(always)]
    pub fn is_blocking(&self) -> bool {
        !matches!(self, Self::Allow | Self::Log)
    }
}

/// Architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Architecture {
    /// x86 (32-bit)
    X86,
    /// x86_64
    X86_64,
    /// ARM (32-bit)
    Arm,
    /// ARM64 (AArch64)
    Arm64,
    /// RISC-V 64-bit
    Riscv64,
    /// MIPS
    Mips,
    /// Unknown
    Unknown(u32),
}

impl Architecture {
    /// Get audit arch constant
    #[inline]
    pub fn audit_arch(&self) -> u32 {
        match self {
            Self::X86 => 0x40000003,
            Self::X86_64 => 0xC000003E,
            Self::Arm => 0x40000028,
            Self::Arm64 => 0xC00000B7,
            Self::Riscv64 => 0xC00000F3,
            Self::Mips => 0x00000008,
            Self::Unknown(v) => *v,
        }
    }
}
