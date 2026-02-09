//! Capability Core Types
//!
//! Fundamental types for Linux capabilities management.

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

/// User ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uid(pub u32);

impl Uid {
    /// Create new UID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get raw value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Root user
    pub const ROOT: Self = Self(0);
}

/// Linux capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum Capability {
    /// Allow modification of files with setuid/setgid bits
    Chown = 0,
    /// Allow bypass of permission checks
    DacOverride = 1,
    /// Allow bypass of permission checks for read
    DacReadSearch = 2,
    /// Allow bypass of permission checks on filesystem operations
    Fowner = 3,
    /// Allow bypass of file permission on setuid/setgid
    Fsetid = 4,
    /// Allow kill signals to other processes
    Kill = 5,
    /// Allow setgid
    Setgid = 6,
    /// Allow setuid
    Setuid = 7,
    /// Allow setting capabilities
    Setpcap = 8,
    /// Immutable file changes
    LinuxImmutable = 9,
    /// Bind to privileged ports
    NetBindService = 10,
    /// Allow broadcasting
    NetBroadcast = 11,
    /// Network admin operations
    NetAdmin = 12,
    /// Allow raw sockets
    NetRaw = 13,
    /// Allow locking of memory
    IpcLock = 14,
    /// Allow IPC ownership bypass
    IpcOwner = 15,
    /// Allow loading kernel modules
    SysModule = 16,
    /// Allow raw I/O access
    SysRawio = 17,
    /// Allow chroot
    SysChroot = 18,
    /// Allow ptrace
    SysPtrace = 19,
    /// Allow system admin operations
    SysAdmin = 21,
    /// Allow boot/kexec
    SysBoot = 22,
    /// Allow system reboot
    SysNice = 23,
    /// Allow setting resource limits
    SysResource = 24,
    /// Allow setting system time
    SysTime = 25,
    /// Allow tty config
    SysTtyConfig = 26,
    /// Allow mknod
    Mknod = 27,
    /// Allow leases
    Lease = 28,
    /// Allow audit write
    AuditWrite = 29,
    /// Allow audit control
    AuditControl = 30,
    /// Allow setting file capabilities
    Setfcap = 31,
    /// Allow MAC override
    MacOverride = 32,
    /// Allow MAC administration
    MacAdmin = 33,
    /// Allow syslog
    Syslog = 34,
    /// Allow wake alarm
    WakeAlarm = 35,
    /// Allow block suspend
    BlockSuspend = 36,
    /// Allow audit read
    AuditRead = 37,
    /// Allow perfmon
    Perfmon = 38,
    /// Allow BPF operations
    Bpf = 39,
    /// Allow checkpoint/restore
    CheckpointRestore = 40,
}

impl Capability {
    /// Get capability name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Chown => "CAP_CHOWN",
            Self::DacOverride => "CAP_DAC_OVERRIDE",
            Self::DacReadSearch => "CAP_DAC_READ_SEARCH",
            Self::Fowner => "CAP_FOWNER",
            Self::Fsetid => "CAP_FSETID",
            Self::Kill => "CAP_KILL",
            Self::Setgid => "CAP_SETGID",
            Self::Setuid => "CAP_SETUID",
            Self::Setpcap => "CAP_SETPCAP",
            Self::IpcLock => "CAP_IPC_LOCK",
            Self::IpcOwner => "CAP_IPC_OWNER",
            Self::SysModule => "CAP_SYS_MODULE",
            Self::SysRawio => "CAP_SYS_RAWIO",
            Self::SysChroot => "CAP_SYS_CHROOT",
            Self::SysPtrace => "CAP_SYS_PTRACE",
            Self::SysBoot => "CAP_SYS_BOOT",
            Self::SysNice => "CAP_SYS_NICE",
            Self::SysResource => "CAP_SYS_RESOURCE",
            Self::SysTime => "CAP_SYS_TIME",
            Self::SysTtyConfig => "CAP_SYS_TTY_CONFIG",
            Self::SysAdmin => "CAP_SYS_ADMIN",
            Self::Mknod => "CAP_MKNOD",
            Self::Lease => "CAP_LEASE",
            Self::AuditWrite => "CAP_AUDIT_WRITE",
            Self::AuditControl => "CAP_AUDIT_CONTROL",
            Self::Setfcap => "CAP_SETFCAP",
            Self::MacAdmin => "CAP_MAC_ADMIN",
            Self::MacOverride => "CAP_MAC_OVERRIDE",
            Self::Syslog => "CAP_SYSLOG",
            Self::WakeAlarm => "CAP_WAKE_ALARM",
            Self::BlockSuspend => "CAP_BLOCK_SUSPEND",
            Self::AuditRead => "CAP_AUDIT_READ",
            Self::Perfmon => "CAP_PERFMON",
            Self::Bpf => "CAP_BPF",
            Self::CheckpointRestore => "CAP_CHECKPOINT_RESTORE",
            Self::NetAdmin => "CAP_NET_ADMIN",
            Self::NetBindService => "CAP_NET_BIND_SERVICE",
            Self::NetBroadcast => "CAP_NET_BROADCAST",
            Self::NetRaw => "CAP_NET_RAW",
            Self::LinuxImmutable => "CAP_LINUX_IMMUTABLE",
        }
    }

    /// Get capability number
    #[inline(always)]
    pub fn number(&self) -> u32 {
        *self as u32
    }

    /// Get risk level
    pub fn risk_level(&self) -> RiskLevel {
        match self {
            Self::SysAdmin | Self::SysModule | Self::SysBoot | Self::SysRawio | Self::MacAdmin => {
                RiskLevel::Critical
            }
            Self::SysPtrace
            | Self::Setuid
            | Self::Setgid
            | Self::Setpcap
            | Self::Setfcap
            | Self::MacOverride
            | Self::DacOverride
            | Self::NetAdmin
            | Self::NetRaw => RiskLevel::High,
            Self::Chown
            | Self::Fowner
            | Self::Kill
            | Self::SysChroot
            | Self::SysResource
            | Self::SysTime
            | Self::IpcOwner
            | Self::AuditControl
            | Self::Bpf => RiskLevel::Medium,
            Self::NetBindService
            | Self::NetBroadcast
            | Self::SysNice
            | Self::Mknod
            | Self::Lease
            | Self::AuditWrite
            | Self::Syslog
            | Self::WakeAlarm
            | Self::Perfmon => RiskLevel::Low,
            _ => RiskLevel::Minimal,
        }
    }

    /// Get category
    pub fn category(&self) -> CapabilityCategory {
        match self {
            Self::Chown
            | Self::DacOverride
            | Self::DacReadSearch
            | Self::Fowner
            | Self::Fsetid
            | Self::LinuxImmutable => CapabilityCategory::Filesystem,

            Self::Kill
            | Self::Setgid
            | Self::Setuid
            | Self::Setpcap
            | Self::SysPtrace
            | Self::SysNice => CapabilityCategory::Process,

            Self::NetAdmin | Self::NetBindService | Self::NetBroadcast | Self::NetRaw => {
                CapabilityCategory::Network
            }

            Self::SysAdmin
            | Self::SysModule
            | Self::SysBoot
            | Self::SysRawio
            | Self::SysChroot
            | Self::SysResource
            | Self::SysTime
            | Self::SysTtyConfig => CapabilityCategory::System,

            Self::IpcLock | Self::IpcOwner => CapabilityCategory::Ipc,

            Self::AuditWrite | Self::AuditControl | Self::AuditRead => CapabilityCategory::Audit,

            Self::MacAdmin | Self::MacOverride | Self::Setfcap => CapabilityCategory::Security,

            _ => CapabilityCategory::Other,
        }
    }

    /// Is network related
    #[inline(always)]
    pub fn is_network(&self) -> bool {
        matches!(self.category(), CapabilityCategory::Network)
    }

    /// Is privileged
    #[inline(always)]
    pub fn is_privileged(&self) -> bool {
        matches!(self.risk_level(), RiskLevel::Critical | RiskLevel::High)
    }

    /// All capabilities
    pub fn all() -> &'static [Capability] {
        &[
            Self::Chown,
            Self::DacOverride,
            Self::DacReadSearch,
            Self::Fowner,
            Self::Fsetid,
            Self::Kill,
            Self::Setgid,
            Self::Setuid,
            Self::Setpcap,
            Self::LinuxImmutable,
            Self::NetBindService,
            Self::NetBroadcast,
            Self::NetAdmin,
            Self::NetRaw,
            Self::IpcLock,
            Self::IpcOwner,
            Self::SysModule,
            Self::SysRawio,
            Self::SysChroot,
            Self::SysPtrace,
            Self::SysAdmin,
            Self::SysBoot,
            Self::SysNice,
            Self::SysResource,
            Self::SysTime,
            Self::SysTtyConfig,
            Self::Mknod,
            Self::Lease,
            Self::AuditWrite,
            Self::AuditControl,
            Self::Setfcap,
            Self::MacOverride,
            Self::MacAdmin,
            Self::Syslog,
            Self::WakeAlarm,
            Self::BlockSuspend,
            Self::AuditRead,
            Self::Perfmon,
            Self::Bpf,
            Self::CheckpointRestore,
        ]
    }
}

/// Risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    /// Minimal risk
    Minimal,
    /// Low risk
    Low,
    /// Medium risk
    Medium,
    /// High risk
    High,
    /// Critical risk
    Critical,
}

impl RiskLevel {
    /// Get level name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
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
            Self::Minimal => 1,
            Self::Low => 3,
            Self::Medium => 5,
            Self::High => 8,
            Self::Critical => 10,
        }
    }
}

/// Capability category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapabilityCategory {
    /// Filesystem operations
    Filesystem,
    /// Process operations
    Process,
    /// Network operations
    Network,
    /// System operations
    System,
    /// IPC operations
    Ipc,
    /// Audit operations
    Audit,
    /// Security operations
    Security,
    /// Other
    Other,
}

impl CapabilityCategory {
    /// Get category name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Filesystem => "filesystem",
            Self::Process => "process",
            Self::Network => "network",
            Self::System => "system",
            Self::Ipc => "ipc",
            Self::Audit => "audit",
            Self::Security => "security",
            Self::Other => "other",
        }
    }
}
