//! # Security Subsystem
//!
//! Security framework initialization including capabilities, permissions,
//! access control, and security policies.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// =============================================================================
// CAPABILITIES
// =============================================================================

/// Capability ID
pub type CapabilityId = u32;

/// Standard Linux-like capabilities
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    // Process/user capabilities
    Chown             = 0,
    DacOverride       = 1,
    DacReadSearch     = 2,
    Fowner            = 3,
    Fsetid            = 4,
    Kill              = 5,
    Setgid            = 6,
    Setuid            = 7,
    Setpcap           = 8,

    // System capabilities
    LinuxImmutable    = 9,
    NetBindService    = 10,
    NetBroadcast      = 11,
    NetAdmin          = 12,
    NetRaw            = 13,
    IpcLock           = 14,
    IpcOwner          = 15,
    SysModule         = 16,
    SysRawio          = 17,
    SysChroot         = 18,
    SysPtrace         = 19,
    SysPacct          = 20,
    SysAdmin          = 21,
    SysBoot           = 22,
    SysNice           = 23,
    SysResource       = 24,
    SysTime           = 25,
    SysTtyConfig      = 26,

    // Extended capabilities
    Mknod             = 27,
    Lease             = 28,
    AuditWrite        = 29,
    AuditControl      = 30,
    Setfcap           = 31,
    MacOverride       = 32,
    MacAdmin          = 33,
    Syslog            = 34,
    WakeAlarm         = 35,
    BlockSuspend      = 36,
    AuditRead         = 37,
    Perfmon           = 38,
    Bpf               = 39,
    CheckpointRestore = 40,
}

impl Capability {
    /// Maximum capability ID
    pub const MAX: u32 = 41;

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
            Self::LinuxImmutable => "CAP_LINUX_IMMUTABLE",
            Self::NetBindService => "CAP_NET_BIND_SERVICE",
            Self::NetBroadcast => "CAP_NET_BROADCAST",
            Self::NetAdmin => "CAP_NET_ADMIN",
            Self::NetRaw => "CAP_NET_RAW",
            Self::IpcLock => "CAP_IPC_LOCK",
            Self::IpcOwner => "CAP_IPC_OWNER",
            Self::SysModule => "CAP_SYS_MODULE",
            Self::SysRawio => "CAP_SYS_RAWIO",
            Self::SysChroot => "CAP_SYS_CHROOT",
            Self::SysPtrace => "CAP_SYS_PTRACE",
            Self::SysPacct => "CAP_SYS_PACCT",
            Self::SysAdmin => "CAP_SYS_ADMIN",
            Self::SysBoot => "CAP_SYS_BOOT",
            Self::SysNice => "CAP_SYS_NICE",
            Self::SysResource => "CAP_SYS_RESOURCE",
            Self::SysTime => "CAP_SYS_TIME",
            Self::SysTtyConfig => "CAP_SYS_TTY_CONFIG",
            Self::Mknod => "CAP_MKNOD",
            Self::Lease => "CAP_LEASE",
            Self::AuditWrite => "CAP_AUDIT_WRITE",
            Self::AuditControl => "CAP_AUDIT_CONTROL",
            Self::Setfcap => "CAP_SETFCAP",
            Self::MacOverride => "CAP_MAC_OVERRIDE",
            Self::MacAdmin => "CAP_MAC_ADMIN",
            Self::Syslog => "CAP_SYSLOG",
            Self::WakeAlarm => "CAP_WAKE_ALARM",
            Self::BlockSuspend => "CAP_BLOCK_SUSPEND",
            Self::AuditRead => "CAP_AUDIT_READ",
            Self::Perfmon => "CAP_PERFMON",
            Self::Bpf => "CAP_BPF",
            Self::CheckpointRestore => "CAP_CHECKPOINT_RESTORE",
        }
    }
}

/// Capability set (bitmask for up to 64 capabilities)
#[derive(Debug, Clone, Copy, Default)]
pub struct CapabilitySet {
    bits: u64,
}

impl CapabilitySet {
    /// Empty set
    pub const EMPTY: Self = Self { bits: 0 };

    /// All capabilities
    pub const ALL: Self = Self { bits: u64::MAX };

    /// Create from bits
    pub const fn from_bits(bits: u64) -> Self {
        Self { bits }
    }

    /// Add capability
    pub fn add(&mut self, cap: Capability) {
        self.bits |= 1 << (cap as u32);
    }

    /// Remove capability
    pub fn remove(&mut self, cap: Capability) {
        self.bits &= !(1 << (cap as u32));
    }

    /// Check if has capability
    pub fn has(&self, cap: Capability) -> bool {
        (self.bits & (1 << (cap as u32))) != 0
    }

    /// Union with another set
    pub fn union(&self, other: &Self) -> Self {
        Self {
            bits: self.bits | other.bits,
        }
    }

    /// Intersection with another set
    pub fn intersection(&self, other: &Self) -> Self {
        Self {
            bits: self.bits & other.bits,
        }
    }

    /// Is empty?
    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// Count capabilities
    pub fn count(&self) -> u32 {
        self.bits.count_ones()
    }
}

// =============================================================================
// CREDENTIALS
// =============================================================================

/// User ID
pub type Uid = u32;

/// Group ID
pub type Gid = u32;

/// Process credentials
#[derive(Debug, Clone)]
pub struct Credentials {
    // User IDs
    pub uid: Uid,   // Real UID
    pub euid: Uid,  // Effective UID
    pub suid: Uid,  // Saved UID
    pub fsuid: Uid, // Filesystem UID

    // Group IDs
    pub gid: Gid,   // Real GID
    pub egid: Gid,  // Effective GID
    pub sgid: Gid,  // Saved GID
    pub fsgid: Gid, // Filesystem GID

    // Supplementary groups
    pub groups: Vec<Gid>,

    // Capabilities
    pub cap_inheritable: CapabilitySet,
    pub cap_permitted: CapabilitySet,
    pub cap_effective: CapabilitySet,
    pub cap_bounding: CapabilitySet,
    pub cap_ambient: CapabilitySet,

    // Security context
    pub secctx: Option<String>,
}

impl Credentials {
    /// Root credentials
    pub fn root() -> Self {
        Self {
            uid: 0,
            euid: 0,
            suid: 0,
            fsuid: 0,
            gid: 0,
            egid: 0,
            sgid: 0,
            fsgid: 0,
            groups: Vec::new(),
            cap_inheritable: CapabilitySet::EMPTY,
            cap_permitted: CapabilitySet::ALL,
            cap_effective: CapabilitySet::ALL,
            cap_bounding: CapabilitySet::ALL,
            cap_ambient: CapabilitySet::EMPTY,
            secctx: None,
        }
    }

    /// Unprivileged user credentials
    pub fn user(uid: Uid, gid: Gid) -> Self {
        Self {
            uid,
            euid: uid,
            suid: uid,
            fsuid: uid,
            gid,
            egid: gid,
            sgid: gid,
            fsgid: gid,
            groups: Vec::new(),
            cap_inheritable: CapabilitySet::EMPTY,
            cap_permitted: CapabilitySet::EMPTY,
            cap_effective: CapabilitySet::EMPTY,
            cap_bounding: CapabilitySet::ALL,
            cap_ambient: CapabilitySet::EMPTY,
            secctx: None,
        }
    }

    /// Is root?
    pub fn is_root(&self) -> bool {
        self.euid == 0
    }

    /// Has capability?
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.is_root() || self.cap_effective.has(cap)
    }

    /// Can access as owner?
    pub fn is_owner(&self, uid: Uid) -> bool {
        self.euid == uid || self.is_root()
    }
}

impl Default for Credentials {
    fn default() -> Self {
        Self::root()
    }
}

// =============================================================================
// ACCESS CONTROL
// =============================================================================

/// Access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessMode {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl AccessMode {
    pub const READ: Self = Self {
        read: true,
        write: false,
        execute: false,
    };
    pub const WRITE: Self = Self {
        read: false,
        write: true,
        execute: false,
    };
    pub const EXECUTE: Self = Self {
        read: false,
        write: false,
        execute: true,
    };
    pub const RW: Self = Self {
        read: true,
        write: true,
        execute: false,
    };
    pub const RX: Self = Self {
        read: true,
        write: false,
        execute: true,
    };
    pub const RWX: Self = Self {
        read: true,
        write: true,
        execute: true,
    };
}

impl Default for AccessMode {
    fn default() -> Self {
        Self::READ
    }
}

/// Access control entry
#[derive(Debug, Clone)]
pub struct AccessControlEntry {
    pub principal_type: PrincipalType,
    pub principal_id: u32,
    pub allow: AccessMode,
    pub deny: AccessMode,
}

/// Principal type for ACL
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalType {
    User,
    Group,
    Everyone,
    Owner,
    OwnerGroup,
}

/// Access control list
#[derive(Debug, Clone, Default)]
pub struct AccessControlList {
    pub entries: Vec<AccessControlEntry>,
}

impl AccessControlList {
    /// Create new empty ACL
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add entry
    pub fn add(&mut self, entry: AccessControlEntry) {
        self.entries.push(entry);
    }

    /// Check access
    pub fn check(&self, creds: &Credentials, mode: AccessMode) -> bool {
        // Default to checking owner/group/other
        let mut allowed = AccessMode {
            read: false,
            write: false,
            execute: false,
        };
        let mut denied = AccessMode {
            read: false,
            write: false,
            execute: false,
        };

        for entry in &self.entries {
            let matches = match entry.principal_type {
                PrincipalType::User => creds.euid == entry.principal_id,
                PrincipalType::Group => {
                    creds.egid == entry.principal_id || creds.groups.contains(&entry.principal_id)
                },
                PrincipalType::Everyone => true,
                PrincipalType::Owner => false, // Context-dependent
                PrincipalType::OwnerGroup => false,
            };

            if matches {
                if entry.allow.read {
                    allowed.read = true;
                }
                if entry.allow.write {
                    allowed.write = true;
                }
                if entry.allow.execute {
                    allowed.execute = true;
                }

                if entry.deny.read {
                    denied.read = true;
                }
                if entry.deny.write {
                    denied.write = true;
                }
                if entry.deny.execute {
                    denied.execute = true;
                }
            }
        }

        // Deny takes precedence
        (!mode.read || (allowed.read && !denied.read))
            && (!mode.write || (allowed.write && !denied.write))
            && (!mode.execute || (allowed.execute && !denied.execute))
    }
}

// =============================================================================
// SECURITY POLICIES
// =============================================================================

/// Security policy type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityPolicy {
    /// Discretionary Access Control (traditional Unix)
    Dac,
    /// Mandatory Access Control (SELinux-like)
    Mac,
    /// Role-Based Access Control
    Rbac,
    /// Attribute-Based Access Control
    Abac,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::Dac
    }
}

/// Security audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    pub timestamp: u64,
    pub event_type: AuditEventType,
    pub subject: String,
    pub object: String,
    pub action: String,
    pub result: bool,
    pub details: String,
}

/// Audit event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventType {
    Login,
    Logout,
    FileAccess,
    ProcessExec,
    NetworkConnection,
    PrivilegeEscalation,
    PolicyViolation,
    SystemCall,
    ModuleLoad,
    ConfigChange,
}

// =============================================================================
// SECURITY SUBSYSTEM
// =============================================================================

/// Security Subsystem
///
/// Manages security policies, credentials, and access control.
pub struct SecuritySubsystem {
    info: SubsystemInfo,

    // Active policy
    policy: SecurityPolicy,

    // Credential cache (task_id -> credentials)
    credentials: BTreeMap<u64, Credentials>,

    // User database (uid -> name)
    users: BTreeMap<Uid, String>,

    // Group database (gid -> name)
    groups: BTreeMap<Gid, String>,

    // Audit log
    audit_log: Vec<AuditEvent>,
    audit_enabled: AtomicBool,
    max_audit_entries: usize,

    // Security counters
    access_checks: AtomicU64,
    access_denials: AtomicU64,
    capability_checks: AtomicU64,
}

static SECURITY_DEPS: [Dependency; 1] = [Dependency::required("scheduler")];

impl SecuritySubsystem {
    /// Create new security subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("security", InitPhase::Late)
                .with_priority(600)
                .with_description("Security framework")
                .with_dependencies(&SECURITY_DEPS)
                .provides(PhaseCapabilities::SECURITY),
            policy: SecurityPolicy::Dac,
            credentials: BTreeMap::new(),
            users: BTreeMap::new(),
            groups: BTreeMap::new(),
            audit_log: Vec::new(),
            audit_enabled: AtomicBool::new(true),
            max_audit_entries: 10000,
            access_checks: AtomicU64::new(0),
            access_denials: AtomicU64::new(0),
            capability_checks: AtomicU64::new(0),
        }
    }

    /// Get active security policy
    pub fn policy(&self) -> SecurityPolicy {
        self.policy
    }

    /// Set security policy
    pub fn set_policy(&mut self, policy: SecurityPolicy) {
        self.policy = policy;
    }

    /// Register credentials for task
    pub fn register_credentials(&mut self, task_id: u64, creds: Credentials) {
        self.credentials.insert(task_id, creds);
    }

    /// Get credentials for task
    pub fn get_credentials(&self, task_id: u64) -> Option<&Credentials> {
        self.credentials.get(&task_id)
    }

    /// Remove credentials for task
    pub fn remove_credentials(&mut self, task_id: u64) {
        self.credentials.remove(&task_id);
    }

    /// Add user
    pub fn add_user(&mut self, uid: Uid, name: &str) {
        self.users.insert(uid, String::from(name));
    }

    /// Add group
    pub fn add_group(&mut self, gid: Gid, name: &str) {
        self.groups.insert(gid, String::from(name));
    }

    /// Lookup user name
    pub fn user_name(&self, uid: Uid) -> Option<&str> {
        self.users.get(&uid).map(|s| s.as_str())
    }

    /// Lookup group name
    pub fn group_name(&self, gid: Gid) -> Option<&str> {
        self.groups.get(&gid).map(|s| s.as_str())
    }

    /// Check capability for task
    pub fn check_capability(&self, task_id: u64, cap: Capability) -> bool {
        self.capability_checks.fetch_add(1, Ordering::Relaxed);

        if let Some(creds) = self.get_credentials(task_id) {
            creds.has_capability(cap)
        } else {
            false
        }
    }

    /// Check file access
    pub fn check_access(
        &self,
        task_id: u64,
        owner_uid: Uid,
        owner_gid: Gid,
        mode: u32,
        access: AccessMode,
    ) -> bool {
        self.access_checks.fetch_add(1, Ordering::Relaxed);

        let creds = match self.get_credentials(task_id) {
            Some(c) => c,
            None => {
                self.access_denials.fetch_add(1, Ordering::Relaxed);
                return false;
            },
        };

        // Root can do anything (with DAC)
        if creds.is_root() && self.policy == SecurityPolicy::Dac {
            return true;
        }

        let (r, w, x) = if creds.euid == owner_uid {
            // Owner permissions
            ((mode >> 6) & 7, (mode >> 6) & 7, (mode >> 6) & 7)
        } else if creds.egid == owner_gid || creds.groups.contains(&owner_gid) {
            // Group permissions
            ((mode >> 3) & 7, (mode >> 3) & 7, (mode >> 3) & 7)
        } else {
            // Other permissions
            (mode & 7, mode & 7, mode & 7)
        };

        let allowed = (!access.read || (r & 4) != 0)
            && (!access.write || (w & 2) != 0)
            && (!access.execute || (x & 1) != 0);

        if !allowed {
            self.access_denials.fetch_add(1, Ordering::Relaxed);
        }

        allowed
    }

    /// Log audit event
    pub fn audit(&mut self, event: AuditEvent) {
        if !self.audit_enabled.load(Ordering::Relaxed) {
            return;
        }

        // Rotate if full
        if self.audit_log.len() >= self.max_audit_entries {
            self.audit_log.remove(0);
        }

        self.audit_log.push(event);
    }

    /// Get audit log
    pub fn audit_log(&self) -> &[AuditEvent] {
        &self.audit_log
    }

    /// Get statistics
    pub fn stats(&self) -> SecurityStats {
        SecurityStats {
            access_checks: self.access_checks.load(Ordering::Relaxed),
            access_denials: self.access_denials.load(Ordering::Relaxed),
            capability_checks: self.capability_checks.load(Ordering::Relaxed),
            audit_events: self.audit_log.len(),
            active_credentials: self.credentials.len(),
        }
    }

    /// Initialize default users/groups
    fn init_defaults(&mut self) {
        // System users
        self.add_user(0, "root");
        self.add_user(1, "daemon");
        self.add_user(2, "bin");
        self.add_user(65534, "nobody");

        // System groups
        self.add_group(0, "root");
        self.add_group(1, "daemon");
        self.add_group(2, "bin");
        self.add_group(65534, "nogroup");
        self.add_group(100, "users");
        self.add_group(27, "sudo");
        self.add_group(4, "adm");
    }
}

/// Security statistics
#[derive(Debug, Clone)]
pub struct SecurityStats {
    pub access_checks: u64,
    pub access_denials: u64,
    pub capability_checks: u64,
    pub audit_events: usize,
    pub active_credentials: usize,
}

impl Default for SecuritySubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for SecuritySubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing security subsystem");

        // Initialize default users and groups
        self.init_defaults();
        ctx.debug(alloc::format!(
            "Loaded {} users, {} groups",
            self.users.len(),
            self.groups.len()
        ));

        // Register kernel task credentials (task 0)
        self.register_credentials(0, Credentials::root());

        ctx.info(alloc::format!(
            "Security policy: {:?}, audit: {}",
            self.policy,
            if self.audit_enabled.load(Ordering::Relaxed) {
                "enabled"
            } else {
                "disabled"
            }
        ));

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        let stats = self.stats();
        ctx.info(alloc::format!(
            "Security shutdown: {} checks, {} denials, {} audit events",
            stats.access_checks,
            stats.access_denials,
            stats.audit_events
        ));

        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_subsystem() {
        let sub = SecuritySubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Late);
        assert!(sub.info().provides.contains(PhaseCapabilities::SECURITY));
    }

    #[test]
    fn test_capability_set() {
        let mut caps = CapabilitySet::EMPTY;
        assert!(!caps.has(Capability::SysAdmin));

        caps.add(Capability::SysAdmin);
        assert!(caps.has(Capability::SysAdmin));

        caps.remove(Capability::SysAdmin);
        assert!(!caps.has(Capability::SysAdmin));
    }

    #[test]
    fn test_credentials() {
        let root = Credentials::root();
        assert!(root.is_root());
        assert!(root.has_capability(Capability::SysAdmin));

        let user = Credentials::user(1000, 1000);
        assert!(!user.is_root());
        assert!(!user.has_capability(Capability::SysAdmin));
    }

    #[test]
    fn test_access_check() {
        let mut sec = SecuritySubsystem::new();

        // Register user credentials
        sec.register_credentials(1, Credentials::user(1000, 1000));

        // Owner can read/write/exec own file
        assert!(sec.check_access(1, 1000, 1000, 0o755, AccessMode::READ));
        assert!(sec.check_access(1, 1000, 1000, 0o755, AccessMode::EXECUTE));

        // Owner can't write read-only file
        assert!(!sec.check_access(1, 1000, 1000, 0o555, AccessMode::WRITE));

        // Other user can't access private file
        assert!(!sec.check_access(1, 0, 0, 0o700, AccessMode::READ));
    }
}
