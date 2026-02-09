//! # Apps Credential Tracker
//!
//! Application-level credential and permission tracking:
//! - Per-app UID/GID tracking with change history
//! - Capability bounding set monitoring
//! - Securebits management
//! - LSM (Linux Security Module) context tracking
//! - Privilege escalation detection
//! - No-new-privs enforcement

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Security context label
#[derive(Debug, Clone)]
pub struct SecurityLabel {
    pub label_hash: u64,
    pub label_type: SecurityLabelType,
    pub level: u32,
    pub category: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLabelType {
    Unconfined,
    Selinux,
    AppArmor,
    Smack,
    Tomoyo,
}

/// Capability set (64 bits)
#[derive(Debug, Clone, Copy, Default)]
pub struct CapBitmask {
    pub bits: u64,
}

impl CapBitmask {
    #[inline(always)]
    pub fn empty() -> Self { Self { bits: 0 } }
    #[inline(always)]
    pub fn full() -> Self { Self { bits: u64::MAX } }

    #[inline(always)]
    pub fn set(&mut self, cap: u32) { if cap < 64 { self.bits |= 1u64 << cap; } }
    #[inline(always)]
    pub fn clear(&mut self, cap: u32) { if cap < 64 { self.bits &= !(1u64 << cap); } }
    #[inline(always)]
    pub fn has(&self, cap: u32) -> bool { if cap < 64 { (self.bits & (1u64 << cap)) != 0 } else { false } }
    #[inline(always)]
    pub fn count(&self) -> u32 { self.bits.count_ones() }

    #[inline(always)]
    pub fn intersect(&self, other: &CapBitmask) -> CapBitmask { CapBitmask { bits: self.bits & other.bits } }
    #[inline(always)]
    pub fn union(&self, other: &CapBitmask) -> CapBitmask { CapBitmask { bits: self.bits | other.bits } }
    #[inline(always)]
    pub fn subtract(&self, other: &CapBitmask) -> CapBitmask { CapBitmask { bits: self.bits & !other.bits } }
}

/// Securebits flags
#[derive(Debug, Clone, Copy, Default)]
pub struct Securebits {
    pub noroot: bool,
    pub noroot_locked: bool,
    pub no_setuid_fixup: bool,
    pub no_setuid_fixup_locked: bool,
    pub keep_caps: bool,
    pub keep_caps_locked: bool,
    pub no_cap_ambient_raise: bool,
    pub no_cap_ambient_raise_locked: bool,
}

/// Per-app credential state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppCredState {
    pub process_id: u64,
    pub uid: u32,
    pub gid: u32,
    pub euid: u32,
    pub egid: u32,
    pub cap_effective: CapBitmask,
    pub cap_permitted: CapBitmask,
    pub cap_inheritable: CapBitmask,
    pub cap_bounding: CapBitmask,
    pub cap_ambient: CapBitmask,
    pub securebits: Securebits,
    pub no_new_privs: bool,
    pub security_label: Option<SecurityLabel>,
    pub escalation_count: u32,
    pub change_history: VecDeque<CredentialChange>,
    pub max_history: usize,
}

impl AppCredState {
    pub fn new(pid: u64, uid: u32, gid: u32, max_hist: usize) -> Self {
        Self {
            process_id: pid, uid, gid, euid: uid, egid: gid,
            cap_effective: CapBitmask::empty(),
            cap_permitted: CapBitmask::empty(),
            cap_inheritable: CapBitmask::empty(),
            cap_bounding: CapBitmask::full(),
            cap_ambient: CapBitmask::empty(),
            securebits: Securebits::default(),
            no_new_privs: false,
            security_label: None,
            escalation_count: 0,
            change_history: VecDeque::new(),
            max_history: max_hist,
        }
    }

    #[inline(always)]
    pub fn is_privileged(&self) -> bool { self.euid == 0 || self.cap_effective.count() > 0 }

    #[inline]
    pub fn record_change(&mut self, change: CredentialChange) {
        // Detect privilege escalation
        if change.gained_privilege { self.escalation_count += 1; }
        self.change_history.push_back(change);
        while self.change_history.len() > self.max_history { self.change_history.pop_front(); }
    }

    #[inline]
    pub fn drop_privileges(&mut self) {
        self.cap_effective = CapBitmask::empty();
        self.cap_permitted = CapBitmask::empty();
        self.cap_ambient = CapBitmask::empty();
    }
}

/// Credential change record
#[derive(Debug, Clone)]
pub struct CredentialChange {
    pub timestamp_ns: u64,
    pub change_type: AppCredChangeType,
    pub old_value: u64,
    pub new_value: u64,
    pub gained_privilege: bool,
    pub syscall_nr: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCredChangeType {
    UidChange,
    GidChange,
    CapChange,
    SecurebitsChange,
    LabelChange,
    NoNewPrivs,
    ExecSetuid,
    ExecSetgid,
}

/// Privilege escalation alert
#[derive(Debug, Clone)]
pub struct EscalationAlert {
    pub process_id: u64,
    pub timestamp_ns: u64,
    pub escalation_type: EscalationType,
    pub details_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationType {
    SetuidRoot,
    CapabilityGain,
    SecurityLabelWeaken,
    NoNewPrivsViolation,
    AmbientCapRaise,
}

/// Apps credential tracker stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppsCredTrackerStats {
    pub total_processes: usize,
    pub privileged_count: usize,
    pub total_changes: usize,
    pub total_escalations: u64,
    pub no_new_privs_count: usize,
}

/// Apps Credential Tracker
pub struct AppsCredTracker {
    states: BTreeMap<u64, AppCredState>,
    alerts: VecDeque<EscalationAlert>,
    max_alerts: usize,
    stats: AppsCredTrackerStats,
}

impl AppsCredTracker {
    pub fn new(max_alerts: usize) -> Self {
        Self {
            states: BTreeMap::new(),
            alerts: VecDeque::new(),
            max_alerts,
            stats: AppsCredTrackerStats::default(),
        }
    }

    #[inline(always)]
    pub fn register(&mut self, pid: u64, uid: u32, gid: u32, max_hist: usize) {
        self.states.entry(pid).or_insert_with(|| AppCredState::new(pid, uid, gid, max_hist));
    }

    pub fn update_uid(&mut self, pid: u64, new_uid: u32, ts: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            let old = state.euid;
            let gained = old != 0 && new_uid == 0;
            state.euid = new_uid;
            state.record_change(CredentialChange {
                timestamp_ns: ts,
                change_type: AppCredChangeType::UidChange,
                old_value: old as u64, new_value: new_uid as u64,
                gained_privilege: gained, syscall_nr: 0,
            });
            if gained {
                self.alerts.push_back(EscalationAlert {
                    process_id: pid, timestamp_ns: ts,
                    escalation_type: EscalationType::SetuidRoot,
                    details_hash: 0,
                });
                while self.alerts.len() > self.max_alerts { self.alerts.pop_front(); }
            }
        }
    }

    pub fn update_caps(&mut self, pid: u64, effective: CapBitmask, permitted: CapBitmask, ts: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            let old_count = state.cap_effective.count();
            state.cap_effective = effective;
            state.cap_permitted = permitted;
            let new_count = effective.count();
            let gained = new_count > old_count;
            state.record_change(CredentialChange {
                timestamp_ns: ts,
                change_type: AppCredChangeType::CapChange,
                old_value: old_count as u64, new_value: new_count as u64,
                gained_privilege: gained, syscall_nr: 0,
            });
        }
    }

    #[inline]
    pub fn set_no_new_privs(&mut self, pid: u64, ts: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            state.no_new_privs = true;
            state.record_change(CredentialChange {
                timestamp_ns: ts,
                change_type: AppCredChangeType::NoNewPrivs,
                old_value: 0, new_value: 1,
                gained_privilege: false, syscall_nr: 0,
            });
        }
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) { self.states.remove(&pid); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.states.len();
        self.stats.privileged_count = self.states.values().filter(|s| s.is_privileged()).count();
        self.stats.total_changes = self.states.values().map(|s| s.change_history.len()).sum();
        self.stats.total_escalations = self.states.values().map(|s| s.escalation_count as u64).sum();
        self.stats.no_new_privs_count = self.states.values().filter(|s| s.no_new_privs).count();
    }

    #[inline(always)]
    pub fn app_creds(&self, pid: u64) -> Option<&AppCredState> { self.states.get(&pid) }
    #[inline(always)]
    pub fn alerts(&self) -> &[EscalationAlert] { &self.alerts }
    #[inline(always)]
    pub fn stats(&self) -> &AppsCredTrackerStats { &self.stats }
}
