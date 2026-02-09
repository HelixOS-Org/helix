//! # Holistic Cred Manager
//!
//! Process credential and capability management:
//! - UID/GID tracking per process
//! - Linux capabilities bitmask management
//! - Privilege escalation detection
//! - Capability bounding sets
//! - Security context transitions
//! - Credential inheritance tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Capability (Linux-style)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    CapChown = 0,
    CapDacOverride = 1,
    CapDacReadSearch = 2,
    CapFowner = 3,
    CapFsetid = 4,
    CapKill = 5,
    CapSetgid = 6,
    CapSetuid = 7,
    CapSetpcap = 8,
    CapLinuxImmutable = 9,
    CapNetBindService = 10,
    CapNetBroadcast = 11,
    CapNetAdmin = 12,
    CapNetRaw = 13,
    CapIpcLock = 14,
    CapIpcOwner = 15,
    CapSysModule = 16,
    CapSysRawio = 17,
    CapSysChroot = 18,
    CapSysPtrace = 19,
    CapSysPacct = 20,
    CapSysAdmin = 21,
    CapSysBoot = 22,
    CapSysNice = 23,
    CapSysResource = 24,
    CapSysTime = 25,
    CapSysTtyConfig = 26,
    CapMknod = 27,
    CapLease = 28,
    CapAuditWrite = 29,
    CapAuditControl = 30,
    CapSetfcap = 31,
    CapMacOverride = 32,
    CapMacAdmin = 33,
    CapSyslog = 34,
    CapWakeAlarm = 35,
    CapBlockSuspend = 36,
    CapAuditRead = 37,
    CapPerfmon = 38,
    CapBpf = 39,
    CapCheckpointRestore = 40,
}

/// Capability set bitmask (64-bit)
#[derive(Debug, Clone, Copy)]
pub struct CapSet {
    bits: u64,
}

impl CapSet {
    #[inline(always)]
    pub fn empty() -> Self { Self { bits: 0 } }
    #[inline(always)]
    pub fn full() -> Self { Self { bits: (1u64 << 41) - 1 } }

    #[inline(always)]
    pub fn set(&mut self, cap: Capability) { self.bits |= 1u64 << (cap as u32); }
    #[inline(always)]
    pub fn clear(&mut self, cap: Capability) { self.bits &= !(1u64 << (cap as u32)); }
    #[inline(always)]
    pub fn has(&self, cap: Capability) -> bool { (self.bits >> (cap as u32)) & 1 == 1 }
    #[inline(always)]
    pub fn count(&self) -> u32 { self.bits.count_ones() }
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.bits == 0 }

    #[inline(always)]
    pub fn intersect(&self, other: &Self) -> Self { Self { bits: self.bits & other.bits } }
    #[inline(always)]
    pub fn union(&self, other: &Self) -> Self { Self { bits: self.bits | other.bits } }
    #[inline(always)]
    pub fn difference(&self, other: &Self) -> Self { Self { bits: self.bits & !other.bits } }
    #[inline(always)]
    pub fn is_subset_of(&self, other: &Self) -> bool { self.bits & !other.bits == 0 }

    #[inline(always)]
    pub fn has_privileged(&self) -> bool {
        self.has(Capability::CapSysAdmin) || self.has(Capability::CapNetAdmin) || self.has(Capability::CapSysRawio)
    }
}

/// Process credentials
#[derive(Debug, Clone)]
pub struct ProcessCred {
    pub pid: u64,
    pub uid: u32,
    pub gid: u32,
    pub euid: u32,
    pub egid: u32,
    pub suid: u32,
    pub sgid: u32,
    pub fsuid: u32,
    pub fsgid: u32,
    pub supplementary_gids: Vec<u32>,
    pub cap_effective: CapSet,
    pub cap_permitted: CapSet,
    pub cap_inheritable: CapSet,
    pub cap_bounding: CapSet,
    pub cap_ambient: CapSet,
    pub securebits: u32,
    pub no_new_privs: bool,
}

impl ProcessCred {
    #[inline]
    pub fn root() -> Self {
        Self {
            pid: 0, uid: 0, gid: 0, euid: 0, egid: 0, suid: 0, sgid: 0,
            fsuid: 0, fsgid: 0, supplementary_gids: Vec::new(),
            cap_effective: CapSet::full(), cap_permitted: CapSet::full(),
            cap_inheritable: CapSet::empty(), cap_bounding: CapSet::full(),
            cap_ambient: CapSet::empty(), securebits: 0, no_new_privs: false,
        }
    }

    #[inline]
    pub fn unprivileged(uid: u32, gid: u32) -> Self {
        Self {
            pid: 0, uid, gid, euid: uid, egid: gid, suid: uid, sgid: gid,
            fsuid: uid, fsgid: gid, supplementary_gids: Vec::new(),
            cap_effective: CapSet::empty(), cap_permitted: CapSet::empty(),
            cap_inheritable: CapSet::empty(), cap_bounding: CapSet::full(),
            cap_ambient: CapSet::empty(), securebits: 0, no_new_privs: false,
        }
    }

    #[inline(always)]
    pub fn is_root(&self) -> bool { self.euid == 0 }
    #[inline(always)]
    pub fn is_privileged(&self) -> bool { self.is_root() || self.cap_effective.has_privileged() }

    #[inline(always)]
    pub fn can(&self, cap: Capability) -> bool { self.cap_effective.has(cap) }

    #[inline]
    pub fn setuid(&mut self, uid: u32) -> bool {
        if self.euid == 0 || self.can(Capability::CapSetuid) {
            self.uid = uid; self.euid = uid; self.suid = uid; self.fsuid = uid;
            if uid != 0 { self.cap_effective = CapSet::empty(); }
            true
        } else if uid == self.uid || uid == self.suid {
            self.euid = uid; self.fsuid = uid;
            true
        } else { false }
    }

    #[inline]
    pub fn setgid(&mut self, gid: u32) -> bool {
        if self.egid == 0 || self.can(Capability::CapSetgid) {
            self.gid = gid; self.egid = gid; self.sgid = gid; self.fsgid = gid;
            true
        } else if gid == self.gid || gid == self.sgid {
            self.egid = gid; self.fsgid = gid;
            true
        } else { false }
    }

    #[inline]
    pub fn inherit_to_child(&self) -> ProcessCred {
        let mut child = self.clone();
        // On exec, calculate new caps
        let new_permitted = self.cap_inheritable.intersect(&self.cap_bounding).union(&self.cap_ambient);
        let new_effective = if self.is_root() { new_permitted } else { self.cap_ambient };
        child.cap_permitted = new_permitted;
        child.cap_effective = new_effective;
        child
    }

    #[inline]
    pub fn drop_privileges(&mut self) {
        self.cap_effective = CapSet::empty();
        self.cap_permitted = CapSet::empty();
        self.cap_ambient = CapSet::empty();
    }
}

/// Security event
#[derive(Debug, Clone)]
pub struct CredEvent {
    pub pid: u64,
    pub event_type: CredEventType,
    pub timestamp: u64,
    pub old_uid: u32,
    pub new_uid: u32,
    pub caps_gained: CapSet,
    pub caps_lost: CapSet,
}

/// Credential event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredEventType {
    UidChange,
    GidChange,
    CapGrant,
    CapRevoke,
    PrivilegeEscalation,
    PrivilegeDrop,
    ExecTransition,
}

/// Cred manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CredManagerStats {
    pub processes_tracked: usize,
    pub root_processes: usize,
    pub privileged_processes: usize,
    pub no_new_privs_count: usize,
    pub total_uid_changes: u64,
    pub total_cap_grants: u64,
    pub escalation_attempts: u64,
    pub events_recorded: usize,
}

/// Holistic credential manager
pub struct HolisticCredManager {
    creds: BTreeMap<u64, ProcessCred>,
    events: VecDeque<CredEvent>,
    max_events: usize,
    stats: CredManagerStats,
}

impl HolisticCredManager {
    pub fn new() -> Self {
        Self { creds: BTreeMap::new(), events: VecDeque::new(), max_events: 2048, stats: CredManagerStats::default() }
    }

    #[inline]
    pub fn register(&mut self, pid: u64, cred: ProcessCred) {
        let mut c = cred;
        c.pid = pid;
        self.creds.insert(pid, c);
    }

    pub fn setuid(&mut self, pid: u64, uid: u32, ts: u64) -> bool {
        if let Some(c) = self.creds.get_mut(&pid) {
            let old_uid = c.euid;
            let was_root = c.is_root();
            if c.setuid(uid) {
                let etype = if !was_root && uid == 0 { CredEventType::PrivilegeEscalation }
                    else if was_root && uid != 0 { CredEventType::PrivilegeDrop }
                    else { CredEventType::UidChange };
                self.record_event(pid, etype, old_uid, uid, CapSet::empty(), CapSet::empty(), ts);
                true
            } else { false }
        } else { false }
    }

    pub fn grant_cap(&mut self, pid: u64, cap: Capability, ts: u64) -> bool {
        if let Some(c) = self.creds.get_mut(&pid) {
            if c.cap_bounding.has(cap) {
                c.cap_effective.set(cap);
                c.cap_permitted.set(cap);
                let mut gained = CapSet::empty();
                gained.set(cap);
                self.record_event(pid, CredEventType::CapGrant, c.euid, c.euid, gained, CapSet::empty(), ts);
                true
            } else { false }
        } else { false }
    }

    #[inline]
    pub fn revoke_cap(&mut self, pid: u64, cap: Capability, ts: u64) {
        if let Some(c) = self.creds.get_mut(&pid) {
            c.cap_effective.clear(cap);
            c.cap_permitted.clear(cap);
            let mut lost = CapSet::empty();
            lost.set(cap);
            self.record_event(pid, CredEventType::CapRevoke, c.euid, c.euid, CapSet::empty(), lost, ts);
        }
    }

    #[inline(always)]
    pub fn check_capability(&self, pid: u64, cap: Capability) -> bool {
        self.creds.get(&pid).map(|c| c.can(cap)).unwrap_or(false)
    }

    #[inline]
    pub fn fork_cred(&mut self, parent_pid: u64, child_pid: u64) {
        if let Some(parent) = self.creds.get(&parent_pid).cloned() {
            let mut child = parent;
            child.pid = child_pid;
            self.creds.insert(child_pid, child);
        }
    }

    #[inline]
    pub fn exec_transition(&mut self, pid: u64, ts: u64) {
        if let Some(c) = self.creds.get(&pid).cloned() {
            let new_cred = c.inherit_to_child();
            let gained = new_cred.cap_effective.difference(&c.cap_effective);
            let lost = c.cap_effective.difference(&new_cred.cap_effective);
            self.record_event(pid, CredEventType::ExecTransition, c.euid, new_cred.euid, gained, lost, ts);
            self.creds.insert(pid, new_cred);
        }
    }

    fn record_event(&mut self, pid: u64, etype: CredEventType, old_uid: u32, new_uid: u32, gained: CapSet, lost: CapSet, ts: u64) {
        self.events.push_back(CredEvent { pid, event_type: etype, timestamp: ts, old_uid, new_uid, caps_gained: gained, caps_lost: lost });
        if self.events.len() > self.max_events { self.events.pop_front(); }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.processes_tracked = self.creds.len();
        self.stats.root_processes = self.creds.values().filter(|c| c.is_root()).count();
        self.stats.privileged_processes = self.creds.values().filter(|c| c.is_privileged()).count();
        self.stats.no_new_privs_count = self.creds.values().filter(|c| c.no_new_privs).count();
        self.stats.total_uid_changes = self.events.iter().filter(|e| e.event_type == CredEventType::UidChange).count() as u64;
        self.stats.total_cap_grants = self.events.iter().filter(|e| e.event_type == CredEventType::CapGrant).count() as u64;
        self.stats.escalation_attempts = self.events.iter().filter(|e| e.event_type == CredEventType::PrivilegeEscalation).count() as u64;
        self.stats.events_recorded = self.events.len();
    }

    #[inline(always)]
    pub fn cred(&self, pid: u64) -> Option<&ProcessCred> { self.creds.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &CredManagerStats { &self.stats }
    #[inline(always)]
    pub fn events(&self) -> &[CredEvent] { &self.events }
}
