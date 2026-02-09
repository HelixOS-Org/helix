//! Kobject Core Types
//!
//! Fundamental types for kernel object management.

use alloc::string::String;
use alloc::vec::Vec;

/// Kernel object identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KobjectId(pub u64);

impl KobjectId {
    /// Create a new kobject ID
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

/// Kset identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KsetId(pub u64);

impl KsetId {
    /// Create a new kset ID
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

/// Ktype identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct KtypeId(pub u64);

impl KtypeId {
    /// Create a new ktype ID
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

/// Kobject state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KobjectState {
    /// Object initialized but not added
    Initialized,
    /// Object added to hierarchy
    Added,
    /// Object registered with sysfs
    Registered,
    /// Object being destroyed
    Destroying,
    /// Object destroyed
    Destroyed,
}

/// Uevent action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UeventAction {
    /// Device added
    Add,
    /// Device removed
    Remove,
    /// Device changed
    Change,
    /// Device moved
    Move,
    /// Device online
    Online,
    /// Device offline
    Offline,
    /// Device bound to driver
    Bind,
    /// Device unbound from driver
    Unbind,
}

impl UeventAction {
    /// Get action string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Remove => "remove",
            Self::Change => "change",
            Self::Move => "move",
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Bind => "bind",
            Self::Unbind => "unbind",
        }
    }
}

/// Kobject information
#[derive(Debug, Clone)]
pub struct KobjectInfo {
    /// Kobject ID
    pub id: KobjectId,
    /// Object name
    pub name: String,
    /// Full sysfs path
    pub path: String,
    /// Parent kobject
    pub parent: Option<KobjectId>,
    /// Kset membership
    pub kset: Option<KsetId>,
    /// Ktype
    pub ktype: Option<KtypeId>,
    /// Current state
    pub state: KobjectState,
    /// Reference count
    pub refcount: u32,
    /// Creation timestamp
    pub created_at: u64,
    /// Last access timestamp
    pub last_access: u64,
    /// Uevent suppressed
    pub uevent_suppressed: bool,
}

impl KobjectInfo {
    /// Create new kobject info
    pub fn new(id: KobjectId, name: String, timestamp: u64) -> Self {
        Self {
            id,
            name: name.clone(),
            path: alloc::format!("/{}", name),
            parent: None,
            kset: None,
            ktype: None,
            state: KobjectState::Initialized,
            refcount: 1,
            created_at: timestamp,
            last_access: timestamp,
            uevent_suppressed: false,
        }
    }

    /// Check if object is alive
    #[inline]
    pub fn is_alive(&self) -> bool {
        !matches!(
            self.state,
            KobjectState::Destroying | KobjectState::Destroyed
        )
    }

    /// Check if registered in sysfs
    #[inline(always)]
    pub fn is_registered(&self) -> bool {
        matches!(self.state, KobjectState::Registered)
    }
}

/// Kset information
#[derive(Debug, Clone)]
pub struct KsetInfo {
    /// Kset ID
    pub id: KsetId,
    /// Kset name
    pub name: String,
    /// Underlying kobject
    pub kobject: KobjectId,
    /// Child kobjects
    pub children: Vec<KobjectId>,
    /// Uevent handler
    pub has_uevent_ops: bool,
    /// Filter uevents
    pub filter_uevents: bool,
}

impl KsetInfo {
    /// Create new kset info
    pub fn new(id: KsetId, name: String, kobject: KobjectId) -> Self {
        Self {
            id,
            name,
            kobject,
            children: Vec::new(),
            has_uevent_ops: false,
            filter_uevents: false,
        }
    }

    /// Child count
    #[inline(always)]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }
}

/// Ktype information
#[derive(Debug, Clone)]
pub struct KtypeInfo {
    /// Ktype ID
    pub id: KtypeId,
    /// Type name
    pub name: String,
    /// Has release function
    pub has_release: bool,
    /// Has sysfs_ops
    pub has_sysfs_ops: bool,
    /// Default attributes
    pub default_attrs: Vec<String>,
    /// Child ktype (for namespace)
    pub child_ns_type: Option<KtypeId>,
}

impl KtypeInfo {
    /// Create new ktype info
    pub fn new(id: KtypeId, name: String) -> Self {
        Self {
            id,
            name,
            has_release: false,
            has_sysfs_ops: false,
            default_attrs: Vec::new(),
            child_ns_type: None,
        }
    }
}
