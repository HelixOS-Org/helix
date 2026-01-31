//! Audit Core Types
//!
//! Fundamental types for audit management.

/// Audit event identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuditEventId(pub u64);

impl AuditEventId {
    /// Create a new audit event ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Audit rule identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuditRuleId(pub u64);

impl AuditRuleId {
    /// Create a new audit rule ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// User identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Uid(pub u32);

impl Uid {
    /// Create a new UID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw value
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Root user
    pub const ROOT: Self = Self(0);

    /// Invalid/unset UID
    pub const INVALID: Self = Self(u32::MAX);
}

/// Group identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Gid(pub u32);

impl Gid {
    /// Create a new GID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw value
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Root group
    pub const ROOT: Self = Self(0);
}

/// Process identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pid(pub u32);

impl Pid {
    /// Create a new PID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw value
    pub const fn raw(&self) -> u32 {
        self.0
    }
}

/// Audit message type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuditMessageType {
    /// Syscall event
    Syscall,
    /// Path information
    Path,
    /// IPC operations
    Ipc,
    /// Socket operations
    Socketcall,
    /// Configuration change
    Config,
    /// Kernel crypto events
    Crypto,
    /// Kernel anomaly
    Anomaly,
    /// Integrity event
    Integrity,
    /// Login/logout
    UserLogin,
    /// User authentication
    UserAuth,
    /// User account changes
    UserAcct,
    /// User start
    UserStart,
    /// User end
    UserEnd,
    /// User error
    UserErr,
    /// Credential acquisition
    CredAcq,
    /// Credential refresh
    CredRefr,
    /// Credential disposal
    CredDisp,
    /// Role change
    RoleChange,
    /// Label override
    LabelOverride,
    /// SELinux/AppArmor
    Mac,
    /// Network event
    Netfilter,
    /// Kernel module
    Kernel,
    /// AVC (Access Vector Cache)
    Avc,
    /// Seccomp action
    Seccomp,
    /// TTY input
    Tty,
    /// EOE (End of Event)
    Eoe,
    /// Unknown
    Unknown(u32),
}

impl AuditMessageType {
    /// Get message type number
    pub fn number(&self) -> u32 {
        match self {
            Self::Syscall => 1300,
            Self::Path => 1302,
            Self::Ipc => 1303,
            Self::Socketcall => 1304,
            Self::Config => 1305,
            Self::Crypto => 1400,
            Self::Anomaly => 1700,
            Self::Integrity => 1800,
            Self::UserLogin => 1100,
            Self::UserAuth => 1101,
            Self::UserAcct => 1102,
            Self::UserStart => 1103,
            Self::UserEnd => 1104,
            Self::UserErr => 1105,
            Self::CredAcq => 1106,
            Self::CredRefr => 1107,
            Self::CredDisp => 1108,
            Self::RoleChange => 1109,
            Self::LabelOverride => 1110,
            Self::Mac => 1400,
            Self::Netfilter => 2500,
            Self::Kernel => 1300,
            Self::Avc => 1400,
            Self::Seccomp => 1326,
            Self::Tty => 1319,
            Self::Eoe => 1320,
            Self::Unknown(n) => *n,
        }
    }

    /// Get message type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Syscall => "SYSCALL",
            Self::Path => "PATH",
            Self::Ipc => "IPC",
            Self::Socketcall => "SOCKETCALL",
            Self::Config => "CONFIG_CHANGE",
            Self::Crypto => "CRYPTO",
            Self::Anomaly => "ANOMALY",
            Self::Integrity => "INTEGRITY",
            Self::UserLogin => "USER_LOGIN",
            Self::UserAuth => "USER_AUTH",
            Self::UserAcct => "USER_ACCT",
            Self::UserStart => "USER_START",
            Self::UserEnd => "USER_END",
            Self::UserErr => "USER_ERR",
            Self::CredAcq => "CRED_ACQ",
            Self::CredRefr => "CRED_REFR",
            Self::CredDisp => "CRED_DISP",
            Self::RoleChange => "ROLE_CHANGE",
            Self::LabelOverride => "LABEL_OVERRIDE",
            Self::Mac => "MAC",
            Self::Netfilter => "NETFILTER",
            Self::Kernel => "KERNEL",
            Self::Avc => "AVC",
            Self::Seccomp => "SECCOMP",
            Self::Tty => "TTY",
            Self::Eoe => "EOE",
            Self::Unknown(_) => "UNKNOWN",
        }
    }

    /// Is security related
    pub fn is_security(&self) -> bool {
        matches!(
            self,
            Self::UserAuth
                | Self::Mac
                | Self::Avc
                | Self::Seccomp
                | Self::Integrity
                | Self::CredAcq
        )
    }
}

/// Audit event result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditResult {
    /// Success
    Success,
    /// Failure
    Failure,
    /// Unknown
    Unknown,
}

impl AuditResult {
    /// Get result name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Unknown => "unknown",
        }
    }
}
