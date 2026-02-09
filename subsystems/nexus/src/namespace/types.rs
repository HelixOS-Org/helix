//! Namespace Core Types
//!
//! Fundamental types for namespace management.

/// Namespace identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NamespaceId(pub u64);

impl NamespaceId {
    /// Create a new namespace ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Initial namespace ID
    pub const INIT: Self = Self(0);
}

/// Process identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(pub u64);

impl ProcessId {
    /// Create a new process ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// User identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(pub u32);

impl UserId {
    /// Create a new user ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Root user
    pub const ROOT: Self = Self(0);
}

/// Group identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(pub u32);

impl GroupId {
    /// Create a new group ID
    #[inline(always)]
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Root group
    pub const ROOT: Self = Self(0);
}

/// Namespace types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NamespaceType {
    /// Mount namespace
    Mnt,
    /// PID namespace
    Pid,
    /// Network namespace
    Net,
    /// IPC namespace
    Ipc,
    /// UTS namespace (hostname)
    Uts,
    /// User namespace
    User,
    /// Cgroup namespace
    Cgroup,
    /// Time namespace
    Time,
}

impl NamespaceType {
    /// Get namespace name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Mnt => "mnt",
            Self::Pid => "pid",
            Self::Net => "net",
            Self::Ipc => "ipc",
            Self::Uts => "uts",
            Self::User => "user",
            Self::Cgroup => "cgroup",
            Self::Time => "time",
        }
    }

    /// Get clone flag
    pub fn clone_flag(&self) -> u64 {
        match self {
            Self::Mnt => 0x00020000,    // CLONE_NEWNS
            Self::Pid => 0x20000000,    // CLONE_NEWPID
            Self::Net => 0x40000000,    // CLONE_NEWNET
            Self::Ipc => 0x08000000,    // CLONE_NEWIPC
            Self::Uts => 0x04000000,    // CLONE_NEWUTS
            Self::User => 0x10000000,   // CLONE_NEWUSER
            Self::Cgroup => 0x02000000, // CLONE_NEWCGROUP
            Self::Time => 0x00000080,   // CLONE_NEWTIME
        }
    }

    /// All namespace types
    pub fn all() -> &'static [NamespaceType] {
        &[
            Self::Mnt,
            Self::Pid,
            Self::Net,
            Self::Ipc,
            Self::Uts,
            Self::User,
            Self::Cgroup,
            Self::Time,
        ]
    }

    /// Check if namespace requires user namespace
    #[inline(always)]
    pub fn requires_user_ns(&self) -> bool {
        matches!(self, Self::User)
    }

    /// Check if namespace is hierarchical
    #[inline(always)]
    pub fn is_hierarchical(&self) -> bool {
        matches!(self, Self::Pid | Self::User | Self::Cgroup | Self::Time)
    }
}

/// Namespace state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceState {
    /// Active namespace
    Active,
    /// Namespace being destroyed
    Dying,
    /// Namespace destroyed
    Dead,
}
