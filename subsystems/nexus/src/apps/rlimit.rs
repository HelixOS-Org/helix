//! # Application Resource Limit Tracking
//!
//! Track and enforce resource limits per application:
//! - RLIMIT emulation
//! - Soft/hard limit management
//! - Limit violation detection
//! - Usage vs limit analysis
//! - Dynamic limit adjustment

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// RESOURCE LIMIT TYPES
// ============================================================================

/// Resource type for limits
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RlimitResource {
    /// CPU time (seconds)
    CpuTime,
    /// File size (bytes)
    FileSize,
    /// Data segment (bytes)
    DataSize,
    /// Stack size (bytes)
    StackSize,
    /// Core file size (bytes)
    CoreSize,
    /// Resident set (bytes)
    Rss,
    /// Number of processes
    NumProcs,
    /// Open files
    OpenFiles,
    /// Locked memory (bytes)
    LockedMemory,
    /// Address space (bytes)
    AddressSpace,
    /// File locks
    FileLocks,
    /// Pending signals
    PendingSignals,
    /// Message queue bytes
    MsgQueue,
    /// Nice priority ceiling
    Nice,
    /// Real-time priority ceiling
    RtPriority,
}

impl RlimitResource {
    /// Default soft limit
    pub fn default_soft(&self) -> u64 {
        match self {
            Self::CpuTime => u64::MAX,
            Self::FileSize => u64::MAX,
            Self::DataSize => u64::MAX,
            Self::StackSize => 8 * 1024 * 1024,    // 8MB
            Self::CoreSize => 0,                     // no core dumps
            Self::Rss => u64::MAX,
            Self::NumProcs => 4096,
            Self::OpenFiles => 1024,
            Self::LockedMemory => 64 * 1024,        // 64KB
            Self::AddressSpace => u64::MAX,
            Self::FileLocks => u64::MAX,
            Self::PendingSignals => 4096,
            Self::MsgQueue => 819200,
            Self::Nice => 0,
            Self::RtPriority => 0,
        }
    }

    /// Default hard limit
    pub fn default_hard(&self) -> u64 {
        match self {
            Self::CpuTime => u64::MAX,
            Self::FileSize => u64::MAX,
            Self::DataSize => u64::MAX,
            Self::StackSize => u64::MAX,
            Self::CoreSize => u64::MAX,
            Self::Rss => u64::MAX,
            Self::NumProcs => 65536,
            Self::OpenFiles => 65536,
            Self::LockedMemory => 64 * 1024,
            Self::AddressSpace => u64::MAX,
            Self::FileLocks => u64::MAX,
            Self::PendingSignals => 65536,
            Self::MsgQueue => 819200,
            Self::Nice => 0,
            Self::RtPriority => 0,
        }
    }
}

/// Resource limit pair
#[derive(Debug, Clone, Copy)]
pub struct Rlimit {
    /// Soft limit
    pub soft: u64,
    /// Hard limit
    pub hard: u64,
}

impl Rlimit {
    pub fn new(soft: u64, hard: u64) -> Self {
        Self { soft, hard }
    }

    pub fn unlimited() -> Self {
        Self {
            soft: u64::MAX,
            hard: u64::MAX,
        }
    }

    /// Is unlimited?
    pub fn is_unlimited(&self) -> bool {
        self.soft == u64::MAX && self.hard == u64::MAX
    }

    /// Set soft (cannot exceed hard)
    pub fn set_soft(&mut self, value: u64) -> bool {
        if value <= self.hard {
            self.soft = value;
            true
        } else {
            false
        }
    }

    /// Set hard (privileged operation, cannot increase)
    pub fn set_hard(&mut self, value: u64, privileged: bool) -> bool {
        if privileged || value <= self.hard {
            self.hard = value;
            // Adjust soft if it exceeds new hard
            if self.soft > self.hard {
                self.soft = self.hard;
            }
            true
        } else {
            false
        }
    }

    /// Check if value exceeds soft limit
    pub fn soft_exceeded(&self, value: u64) -> bool {
        value > self.soft && self.soft != u64::MAX
    }

    /// Check if value exceeds hard limit
    pub fn hard_exceeded(&self, value: u64) -> bool {
        value > self.hard && self.hard != u64::MAX
    }

    /// Utilization relative to soft limit
    pub fn utilization(&self, value: u64) -> f64 {
        if self.soft == 0 || self.soft == u64::MAX {
            return 0.0;
        }
        value as f64 / self.soft as f64
    }
}

// ============================================================================
// LIMIT VIOLATION
// ============================================================================

/// Violation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    /// Soft limit exceeded
    SoftExceeded,
    /// Hard limit exceeded (blocked)
    HardExceeded,
    /// Approaching soft limit (>80%)
    Warning,
}

/// Limit violation event
#[derive(Debug, Clone)]
pub struct LimitViolation {
    /// Process id
    pub pid: u64,
    /// Resource
    pub resource: RlimitResource,
    /// Violation type
    pub violation: ViolationType,
    /// Current value
    pub current_value: u64,
    /// Limit
    pub limit: u64,
    /// Timestamp
    pub timestamp: u64,
}

// ============================================================================
// PROCESS LIMIT PROFILE
// ============================================================================

/// Process limit profile
#[derive(Debug)]
pub struct ProcessLimitProfile {
    /// Process id
    pub pid: u64,
    /// Limits
    limits: BTreeMap<u8, Rlimit>,
    /// Current usage
    usage: BTreeMap<u8, u64>,
    /// Violation count per resource
    violations: BTreeMap<u8, u64>,
    /// Peak usage
    peak_usage: BTreeMap<u8, u64>,
}

impl ProcessLimitProfile {
    pub fn new(pid: u64) -> Self {
        let mut profile = Self {
            pid,
            limits: BTreeMap::new(),
            usage: BTreeMap::new(),
            violations: BTreeMap::new(),
            peak_usage: BTreeMap::new(),
        };
        // Initialize defaults
        let resources = [
            RlimitResource::CpuTime,
            RlimitResource::FileSize,
            RlimitResource::DataSize,
            RlimitResource::StackSize,
            RlimitResource::CoreSize,
            RlimitResource::Rss,
            RlimitResource::NumProcs,
            RlimitResource::OpenFiles,
            RlimitResource::LockedMemory,
            RlimitResource::AddressSpace,
            RlimitResource::FileLocks,
            RlimitResource::PendingSignals,
            RlimitResource::MsgQueue,
            RlimitResource::Nice,
            RlimitResource::RtPriority,
        ];
        for res in resources.iter() {
            profile.limits.insert(
                *res as u8,
                Rlimit::new(res.default_soft(), res.default_hard()),
            );
        }
        profile
    }

    /// Get limit
    pub fn get_limit(&self, resource: RlimitResource) -> Rlimit {
        self.limits
            .get(&(resource as u8))
            .copied()
            .unwrap_or(Rlimit::unlimited())
    }

    /// Set limit
    pub fn set_limit(
        &mut self,
        resource: RlimitResource,
        soft: u64,
        hard: u64,
        privileged: bool,
    ) -> bool {
        let key = resource as u8;
        let limit = self
            .limits
            .entry(key)
            .or_insert_with(|| Rlimit::unlimited());
        if privileged {
            limit.hard = hard;
            limit.soft = soft.min(hard);
            true
        } else {
            if hard > limit.hard {
                return false;
            }
            limit.hard = hard;
            limit.soft = soft.min(hard);
            true
        }
    }

    /// Update usage and check violations
    pub fn update_usage(
        &mut self,
        resource: RlimitResource,
        value: u64,
        now: u64,
    ) -> Option<LimitViolation> {
        let key = resource as u8;
        self.usage.insert(key, value);

        // Track peak
        let peak = self.peak_usage.entry(key).or_insert(0);
        if value > *peak {
            *peak = value;
        }

        let limit = self.get_limit(resource);

        // Check hard limit
        if limit.hard_exceeded(value) {
            *self.violations.entry(key).or_insert(0) += 1;
            return Some(LimitViolation {
                pid: self.pid,
                resource,
                violation: ViolationType::HardExceeded,
                current_value: value,
                limit: limit.hard,
                timestamp: now,
            });
        }

        // Check soft limit
        if limit.soft_exceeded(value) {
            *self.violations.entry(key).or_insert(0) += 1;
            return Some(LimitViolation {
                pid: self.pid,
                resource,
                violation: ViolationType::SoftExceeded,
                current_value: value,
                limit: limit.soft,
                timestamp: now,
            });
        }

        // Check warning (>80%)
        let util = limit.utilization(value);
        if util > 0.8 {
            return Some(LimitViolation {
                pid: self.pid,
                resource,
                violation: ViolationType::Warning,
                current_value: value,
                limit: limit.soft,
                timestamp: now,
            });
        }

        None
    }

    /// Current usage
    pub fn current_usage(&self, resource: RlimitResource) -> u64 {
        self.usage.get(&(resource as u8)).copied().unwrap_or(0)
    }

    /// Peak usage
    pub fn peak(&self, resource: RlimitResource) -> u64 {
        self.peak_usage.get(&(resource as u8)).copied().unwrap_or(0)
    }

    /// Utilization
    pub fn utilization(&self, resource: RlimitResource) -> f64 {
        let usage = self.current_usage(resource);
        let limit = self.get_limit(resource);
        limit.utilization(usage)
    }

    /// Resources near limit (>70%)
    pub fn near_limit(&self) -> Vec<(RlimitResource, f64)> {
        let resources = [
            RlimitResource::CpuTime,
            RlimitResource::FileSize,
            RlimitResource::DataSize,
            RlimitResource::StackSize,
            RlimitResource::Rss,
            RlimitResource::NumProcs,
            RlimitResource::OpenFiles,
            RlimitResource::LockedMemory,
            RlimitResource::AddressSpace,
        ];
        let mut result = Vec::new();
        for &res in resources.iter() {
            let util = self.utilization(res);
            if util > 0.7 {
                result.push((res, util));
            }
        }
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        result
    }
}

// ============================================================================
// RLIMIT MANAGER
// ============================================================================

/// Rlimit stats
#[derive(Debug, Clone, Default)]
pub struct AppRlimitStats {
    /// Tracked processes
    pub processes: usize,
    /// Total violations
    pub total_violations: u64,
    /// Hard limit hits
    pub hard_violations: u64,
}

/// Application resource limit manager
pub struct AppRlimitManager {
    /// Profiles
    profiles: BTreeMap<u64, ProcessLimitProfile>,
    /// Violation log
    violations: Vec<LimitViolation>,
    /// Stats
    stats: AppRlimitStats,
    /// Max violations kept
    max_violations: usize,
}

impl AppRlimitManager {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            violations: Vec::new(),
            stats: AppRlimitStats::default(),
            max_violations: 4096,
        }
    }

    /// Register process
    pub fn register(&mut self, pid: u64) {
        self.profiles
            .entry(pid)
            .or_insert_with(|| ProcessLimitProfile::new(pid));
        self.stats.processes = self.profiles.len();
    }

    /// Set limit
    pub fn set_limit(
        &mut self,
        pid: u64,
        resource: RlimitResource,
        soft: u64,
        hard: u64,
        privileged: bool,
    ) -> bool {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.set_limit(resource, soft, hard, privileged)
        } else {
            false
        }
    }

    /// Update usage
    pub fn update_usage(
        &mut self,
        pid: u64,
        resource: RlimitResource,
        value: u64,
        now: u64,
    ) -> Option<ViolationType> {
        let violation = if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.update_usage(resource, value, now)
        } else {
            None
        };

        if let Some(ref v) = violation {
            self.stats.total_violations += 1;
            if v.violation == ViolationType::HardExceeded {
                self.stats.hard_violations += 1;
            }
            self.violations.push(v.clone());
            if self.violations.len() > self.max_violations {
                self.violations.remove(0);
            }
            Some(v.violation)
        } else {
            None
        }
    }

    /// Get limit
    pub fn get_limit(&self, pid: u64, resource: RlimitResource) -> Option<Rlimit> {
        self.profiles.get(&pid).map(|p| p.get_limit(resource))
    }

    /// Get profile
    pub fn profile(&self, pid: u64) -> Option<&ProcessLimitProfile> {
        self.profiles.get(&pid)
    }

    /// Recent violations
    pub fn recent_violations(&self, limit: usize) -> Vec<&LimitViolation> {
        self.violations.iter().rev().take(limit).collect()
    }

    /// Stats
    pub fn stats(&self) -> &AppRlimitStats {
        &self.stats
    }
}
