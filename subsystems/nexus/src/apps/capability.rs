//! # Application Capability Tracking
//!
//! Track and manage application capabilities:
//! - Capability discovery
//! - Permission analysis
//! - Privilege escalation detection
//! - Capability delegation
//! - Least-privilege recommendations

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CAPABILITY TYPES
// ============================================================================

/// Capability category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapabilityCategory {
    /// File system access
    FileSystem,
    /// Network access
    Network,
    /// Process management
    ProcessMgmt,
    /// Memory management
    MemoryMgmt,
    /// Device access
    Device,
    /// System administration
    SysAdmin,
    /// Security
    Security,
    /// IPC
    Ipc,
}

/// Specific capability
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppCapability {
    /// Read files
    FileRead,
    /// Write files
    FileWrite,
    /// Execute files
    FileExec,
    /// Create files
    FileCreate,
    /// Network listen
    NetListen,
    /// Network connect
    NetConnect,
    /// Raw sockets
    NetRaw,
    /// Fork processes
    Fork,
    /// Signal other processes
    Signal,
    /// Map memory
    Mmap,
    /// Lock memory
    Mlock,
    /// Access devices
    DeviceAccess,
    /// Mount filesystems
    Mount,
    /// Change ownership
    Chown,
    /// Set capabilities
    SetCap,
    /// Trace processes
    Ptrace,
    /// Module load
    ModuleLoad,
    /// Reboot
    Reboot,
}

impl AppCapability {
    /// Category
    pub fn category(&self) -> CapabilityCategory {
        match self {
            Self::FileRead | Self::FileWrite | Self::FileExec | Self::FileCreate => {
                CapabilityCategory::FileSystem
            }
            Self::NetListen | Self::NetConnect | Self::NetRaw => CapabilityCategory::Network,
            Self::Fork | Self::Signal => CapabilityCategory::ProcessMgmt,
            Self::Mmap | Self::Mlock => CapabilityCategory::MemoryMgmt,
            Self::DeviceAccess => CapabilityCategory::Device,
            Self::Mount | Self::Chown | Self::ModuleLoad | Self::Reboot => {
                CapabilityCategory::SysAdmin
            }
            Self::SetCap | Self::Ptrace => CapabilityCategory::Security,
        }
    }

    /// Risk level (0-10)
    pub fn risk_level(&self) -> u8 {
        match self {
            Self::FileRead | Self::Mmap => 1,
            Self::FileWrite | Self::FileCreate | Self::Fork => 3,
            Self::NetConnect | Self::Signal | Self::Mlock => 4,
            Self::NetListen | Self::FileExec => 5,
            Self::DeviceAccess | Self::NetRaw => 7,
            Self::Ptrace | Self::Chown | Self::SetCap => 8,
            Self::Mount | Self::ModuleLoad => 9,
            Self::Reboot => 10,
        }
    }
}

// ============================================================================
// CAPABILITY SET
// ============================================================================

/// Capability set as bitmask
#[derive(Debug, Clone)]
pub struct AppCapabilitySet {
    /// Bitmask of capabilities
    bits: u64,
}

impl AppCapabilitySet {
    #[inline(always)]
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    #[inline(always)]
    pub fn full() -> Self {
        Self { bits: u64::MAX }
    }

    /// Grant capability
    #[inline(always)]
    pub fn grant(&mut self, cap: AppCapability) {
        self.bits |= 1u64 << (cap as u32);
    }

    /// Revoke capability
    #[inline(always)]
    pub fn revoke(&mut self, cap: AppCapability) {
        self.bits &= !(1u64 << (cap as u32));
    }

    /// Has capability?
    #[inline(always)]
    pub fn has(&self, cap: AppCapability) -> bool {
        (self.bits & (1u64 << (cap as u32))) != 0
    }

    /// Count capabilities
    #[inline]
    pub fn count(&self) -> u32 {
        let mut n = self.bits;
        let mut count = 0u32;
        while n != 0 {
            count += 1;
            n &= n - 1;
        }
        count
    }

    /// Max risk level
    pub fn max_risk(&self) -> u8 {
        let caps = [
            AppCapability::FileRead,
            AppCapability::FileWrite,
            AppCapability::FileExec,
            AppCapability::FileCreate,
            AppCapability::NetListen,
            AppCapability::NetConnect,
            AppCapability::NetRaw,
            AppCapability::Fork,
            AppCapability::Signal,
            AppCapability::Mmap,
            AppCapability::Mlock,
            AppCapability::DeviceAccess,
            AppCapability::Mount,
            AppCapability::Chown,
            AppCapability::SetCap,
            AppCapability::Ptrace,
            AppCapability::ModuleLoad,
            AppCapability::Reboot,
        ];

        caps.iter()
            .filter(|&&c| self.has(c))
            .map(|c| c.risk_level())
            .max()
            .unwrap_or(0)
    }

    /// Intersection
    #[inline]
    pub fn intersect(&self, other: &AppCapabilitySet) -> AppCapabilitySet {
        AppCapabilitySet {
            bits: self.bits & other.bits,
        }
    }

    /// Union
    #[inline]
    pub fn union(&self, other: &AppCapabilitySet) -> AppCapabilitySet {
        AppCapabilitySet {
            bits: self.bits | other.bits,
        }
    }
}

// ============================================================================
// CAPABILITY USAGE
// ============================================================================

/// Capability usage record
#[derive(Debug, Clone)]
pub struct CapUsageRecord {
    /// Capability
    pub capability: AppCapability,
    /// Times used
    pub use_count: u64,
    /// Last used
    pub last_used: u64,
    /// Denied count
    pub denied_count: u64,
}

/// Process capability profile
#[derive(Debug, Clone)]
pub struct ProcessCapProfile {
    /// Process id
    pub pid: u64,
    /// Granted capabilities
    pub granted: AppCapabilitySet,
    /// Actually used capabilities
    pub used: AppCapabilitySet,
    /// Usage records
    pub usage: BTreeMap<u8, CapUsageRecord>,
    /// Privilege escalation attempts
    pub escalation_attempts: u64,
}

impl ProcessCapProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            granted: AppCapabilitySet::empty(),
            used: AppCapabilitySet::empty(),
            usage: BTreeMap::new(),
            escalation_attempts: 0,
        }
    }

    /// Record usage
    #[inline]
    pub fn record_use(&mut self, cap: AppCapability, now: u64) {
        self.used.grant(cap);
        let record = self.usage.entry(cap as u8).or_insert_with(|| CapUsageRecord {
            capability: cap,
            use_count: 0,
            last_used: 0,
            denied_count: 0,
        });
        record.use_count += 1;
        record.last_used = now;
    }

    /// Record denial
    pub fn record_denial(&mut self, cap: AppCapability, now: u64) {
        let record = self.usage.entry(cap as u8).or_insert_with(|| CapUsageRecord {
            capability: cap,
            use_count: 0,
            last_used: 0,
            denied_count: 0,
        });
        record.denied_count += 1;
        record.last_used = now;

        if !self.granted.has(cap) {
            self.escalation_attempts += 1;
        }
    }

    /// Unused granted capabilities
    pub fn unused_capabilities(&self) -> Vec<AppCapability> {
        let caps = [
            AppCapability::FileRead,
            AppCapability::FileWrite,
            AppCapability::FileExec,
            AppCapability::FileCreate,
            AppCapability::NetListen,
            AppCapability::NetConnect,
            AppCapability::NetRaw,
            AppCapability::Fork,
            AppCapability::Signal,
            AppCapability::Mmap,
            AppCapability::Mlock,
            AppCapability::DeviceAccess,
            AppCapability::Mount,
            AppCapability::Chown,
            AppCapability::SetCap,
            AppCapability::Ptrace,
            AppCapability::ModuleLoad,
            AppCapability::Reboot,
        ];

        caps.iter()
            .filter(|&&c| self.granted.has(c) && !self.used.has(c))
            .copied()
            .collect()
    }

    /// Least-privilege recommendation
    #[inline(always)]
    pub fn least_privilege(&self) -> AppCapabilitySet {
        self.used.clone()
    }
}

// ============================================================================
// CAPABILITY MANAGER
// ============================================================================

/// Capability stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppCapabilityStats {
    /// Processes tracked
    pub processes: usize,
    /// Escalation attempts
    pub escalation_attempts: u64,
    /// Over-privileged processes
    pub over_privileged: usize,
}

/// Application capability manager
pub struct AppCapabilityManager {
    /// Profiles
    profiles: BTreeMap<u64, ProcessCapProfile>,
    /// Stats
    stats: AppCapabilityStats,
}

impl AppCapabilityManager {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppCapabilityStats::default(),
        }
    }

    /// Grant capabilities
    #[inline]
    pub fn grant(&mut self, pid: u64, caps: AppCapabilitySet) {
        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(|| ProcessCapProfile::new(pid));
        profile.granted = profile.granted.union(&caps);
        self.stats.processes = self.profiles.len();
    }

    /// Record use
    pub fn record_use(&mut self, pid: u64, cap: AppCapability, now: u64) {
        let profile = self
            .profiles
            .entry(pid)
            .or_insert_with(|| ProcessCapProfile::new(pid));
        if profile.granted.has(cap) {
            profile.record_use(cap, now);
        } else {
            profile.record_denial(cap, now);
            self.stats.escalation_attempts += 1;
        }
    }

    /// Check permission
    #[inline]
    pub fn check(&self, pid: u64, cap: AppCapability) -> bool {
        self.profiles
            .get(&pid)
            .map(|p| p.granted.has(cap))
            .unwrap_or(false)
    }

    /// Over-privileged processes
    #[inline]
    pub fn over_privileged(&self) -> Vec<(u64, Vec<AppCapability>)> {
        let mut result = Vec::new();
        for profile in self.profiles.values() {
            let unused = profile.unused_capabilities();
            if !unused.is_empty() {
                result.push((profile.pid, unused));
            }
        }
        self.stats.over_privileged;
        result
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppCapabilityStats {
        &self.stats
    }
}
