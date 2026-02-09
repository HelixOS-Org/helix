//! LSM Core Types
//!
//! Fundamental types for Linux Security Module management.

use alloc::string::String;

/// Hook ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HookId(pub u64);

impl HookId {
    /// Create new hook ID
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

/// Policy ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PolicyId(pub u64);

impl PolicyId {
    /// Create new policy ID
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

/// LSM type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LsmType {
    /// SELinux
    Selinux,
    /// AppArmor
    Apparmor,
    /// Smack
    Smack,
    /// TOMOYO
    Tomoyo,
    /// Yama
    Yama,
    /// LoadPin
    Loadpin,
    /// SafeSetID
    Safesetid,
    /// Lockdown
    Lockdown,
    /// BPF LSM
    Bpf,
    /// Landlock
    Landlock,
    /// Custom
    Custom(u32),
}

impl LsmType {
    /// Get LSM name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Selinux => "selinux",
            Self::Apparmor => "apparmor",
            Self::Smack => "smack",
            Self::Tomoyo => "tomoyo",
            Self::Yama => "yama",
            Self::Loadpin => "loadpin",
            Self::Safesetid => "safesetid",
            Self::Lockdown => "lockdown",
            Self::Bpf => "bpf",
            Self::Landlock => "landlock",
            Self::Custom(_) => "custom",
        }
    }

    /// Is major LSM
    #[inline]
    pub fn is_major(&self) -> bool {
        matches!(
            self,
            Self::Selinux | Self::Apparmor | Self::Smack | Self::Tomoyo
        )
    }

    /// Is stacking supported
    #[inline(always)]
    pub fn supports_stacking(&self) -> bool {
        !self.is_major() || matches!(self, Self::Bpf | Self::Landlock)
    }
}

/// LSM state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmState {
    /// Disabled
    Disabled,
    /// Enabled (permissive)
    Permissive,
    /// Enabled (enforcing)
    Enforcing,
}

impl LsmState {
    /// Get state name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Permissive => "permissive",
            Self::Enforcing => "enforcing",
        }
    }
}

/// Security context
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(align(64))]
pub struct SecurityContext {
    /// User
    pub user: String,
    /// Role
    pub role: String,
    /// Type
    pub type_: String,
    /// Level (MLS/MCS)
    pub level: Option<String>,
}

impl SecurityContext {
    /// Create new context
    pub fn new(user: String, role: String, type_: String) -> Self {
        Self {
            user,
            role,
            type_,
            level: None,
        }
    }

    /// Parse from string (SELinux format)
    pub fn parse(s: &str) -> Option<Self> {
        let parts: alloc::vec::Vec<&str> = s.split(':').collect();
        if parts.len() < 3 {
            return None;
        }

        Some(Self {
            user: String::from(parts[0]),
            role: String::from(parts[1]),
            type_: String::from(parts[2]),
            level: if parts.len() > 3 {
                Some(parts[3..].join(":"))
            } else {
                None
            },
        })
    }

    /// Format to string
    #[inline]
    pub fn to_string(&self) -> String {
        if let Some(ref level) = self.level {
            alloc::format!("{}:{}:{}:{}", self.user, self.role, self.type_, level)
        } else {
            alloc::format!("{}:{}:{}", self.user, self.role, self.type_)
        }
    }

    /// Is unconfined
    #[inline(always)]
    pub fn is_unconfined(&self) -> bool {
        self.type_.contains("unconfined") || self.role.contains("unconfined")
    }
}

/// AppArmor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppArmorMode {
    /// Enforce mode
    Enforce,
    /// Complain mode
    Complain,
    /// Kill mode
    Kill,
    /// Unconfined
    Unconfined,
}

impl AppArmorMode {
    /// Get mode name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Enforce => "enforce",
            Self::Complain => "complain",
            Self::Kill => "kill",
            Self::Unconfined => "unconfined",
        }
    }
}

/// AppArmor profile
#[derive(Debug, Clone)]
pub struct AppArmorProfile {
    /// Profile name
    pub name: String,
    /// Mode
    pub mode: AppArmorMode,
    /// Parent profile
    pub parent: Option<String>,
    /// Is enforcing
    pub enforcing: bool,
}

impl AppArmorProfile {
    /// Create new profile
    pub fn new(name: String, mode: AppArmorMode) -> Self {
        Self {
            name,
            mode,
            parent: None,
            enforcing: matches!(mode, AppArmorMode::Enforce),
        }
    }

    /// Is unconfined
    #[inline(always)]
    pub fn is_unconfined(&self) -> bool {
        self.name == "unconfined" || matches!(self.mode, AppArmorMode::Unconfined)
    }
}
