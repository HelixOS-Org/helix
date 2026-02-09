//! Access Decisions
//!
//! Access control decisions and permissions.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::SecurityContext;

/// Access permission
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Read
    Read,
    /// Write
    Write,
    /// Execute
    Execute,
    /// Append
    Append,
    /// Create
    Create,
    /// Delete/Unlink
    Delete,
    /// Link
    Link,
    /// Rename
    Rename,
    /// Setattr
    Setattr,
    /// Getattr
    Getattr,
    /// Open
    Open,
    /// Lock
    Lock,
    /// Ioctl
    Ioctl,
    /// Mmap
    Mmap,
    /// Signal
    Signal,
    /// Ptrace
    Ptrace,
    /// Connect
    Connect,
    /// Bind
    Bind,
    /// Listen
    Listen,
    /// Accept
    Accept,
    /// Send
    Send,
    /// Receive
    Receive,
    /// Transition
    Transition,
}

impl Permission {
    /// Get permission name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Execute => "execute",
            Self::Append => "append",
            Self::Create => "create",
            Self::Delete => "delete",
            Self::Link => "link",
            Self::Rename => "rename",
            Self::Setattr => "setattr",
            Self::Getattr => "getattr",
            Self::Open => "open",
            Self::Lock => "lock",
            Self::Ioctl => "ioctl",
            Self::Mmap => "mmap",
            Self::Signal => "signal",
            Self::Ptrace => "ptrace",
            Self::Connect => "connect",
            Self::Bind => "bind",
            Self::Listen => "listen",
            Self::Accept => "accept",
            Self::Send => "send",
            Self::Receive => "receive",
            Self::Transition => "transition",
        }
    }

    /// Is sensitive
    #[inline]
    pub fn is_sensitive(&self) -> bool {
        matches!(
            self,
            Self::Execute | Self::Ptrace | Self::Transition | Self::Write | Self::Setattr | Self::Delete
        )
    }
}

/// Object class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectClass {
    /// File
    File,
    /// Directory
    Dir,
    /// Link
    Lnk,
    /// Character device
    Chr,
    /// Block device
    Blk,
    /// Socket
    Sock,
    /// FIFO
    Fifo,
    /// Process
    Process,
    /// Capability
    Capability,
    /// Filesystem
    Filesystem,
    /// Security
    Security,
    /// System
    System,
    /// Kernel module
    Module,
    /// BPF
    Bpf,
    /// Unknown
    Unknown,
}

impl ObjectClass {
    /// Get class name
    pub fn name(&self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Dir => "dir",
            Self::Lnk => "lnk_file",
            Self::Chr => "chr_file",
            Self::Blk => "blk_file",
            Self::Sock => "sock_file",
            Self::Fifo => "fifo_file",
            Self::Process => "process",
            Self::Capability => "capability",
            Self::Filesystem => "filesystem",
            Self::Security => "security",
            Self::System => "system",
            Self::Module => "module",
            Self::Bpf => "bpf",
            Self::Unknown => "unknown",
        }
    }
}

/// Access decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDecision {
    /// Allowed
    Allowed,
    /// Denied
    Denied,
    /// Audited (allowed but logged)
    Audited,
    /// Unknown (no policy)
    Unknown,
}

impl AccessDecision {
    /// Get decision name
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
            Self::Audited => "audited",
            Self::Unknown => "unknown",
        }
    }

    /// Is allowed
    #[inline(always)]
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allowed | Self::Audited)
    }
}

/// Access vector cache entry
#[derive(Debug)]
pub struct AvcEntry {
    /// Source context
    pub source: SecurityContext,
    /// Target context
    pub target: SecurityContext,
    /// Object class
    pub class: ObjectClass,
    /// Allowed permissions
    pub allowed: Vec<Permission>,
    /// Audited permissions
    pub audited: Vec<Permission>,
    /// Hit count
    pub hits: AtomicU64,
    /// Last access time
    pub last_access: u64,
}

impl Clone for AvcEntry {
    fn clone(&self) -> Self {
        Self {
            source: self.source.clone(),
            target: self.target.clone(),
            class: self.class,
            allowed: self.allowed.clone(),
            audited: self.audited.clone(),
            hits: AtomicU64::new(self.hits.load(Ordering::Relaxed)),
            last_access: self.last_access,
        }
    }
}

impl AvcEntry {
    /// Create new entry
    pub fn new(source: SecurityContext, target: SecurityContext, class: ObjectClass) -> Self {
        Self {
            source,
            target,
            class,
            allowed: Vec::new(),
            audited: Vec::new(),
            hits: AtomicU64::new(0),
            last_access: 0,
        }
    }

    /// Check permission
    pub fn check(&self, perm: Permission) -> AccessDecision {
        self.hits.fetch_add(1, Ordering::Relaxed);

        if self.allowed.contains(&perm) {
            if self.audited.contains(&perm) {
                AccessDecision::Audited
            } else {
                AccessDecision::Allowed
            }
        } else {
            AccessDecision::Denied
        }
    }
}
