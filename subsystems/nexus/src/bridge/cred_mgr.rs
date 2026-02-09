//! # Bridge Credential Manager
//!
//! Process credential management for syscall authorization:
//! - UID/GID/supplementary groups tracking
//! - Effective/real/saved/fs credential sets
//! - Credential change auditing (setuid, setgid)
//! - Capability-aware credential checks
//! - Credential inheritance on fork/exec
//! - User namespace credential translation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Credential set for a process
#[derive(Debug, Clone)]
pub struct CredentialSet {
    pub real_uid: u32,
    pub effective_uid: u32,
    pub saved_uid: u32,
    pub fs_uid: u32,
    pub real_gid: u32,
    pub effective_gid: u32,
    pub saved_gid: u32,
    pub fs_gid: u32,
    pub supplementary_groups: Vec<u32>,
}

impl CredentialSet {
    #[inline]
    pub fn root() -> Self {
        Self {
            real_uid: 0, effective_uid: 0, saved_uid: 0, fs_uid: 0,
            real_gid: 0, effective_gid: 0, saved_gid: 0, fs_gid: 0,
            supplementary_groups: Vec::new(),
        }
    }

    #[inline]
    pub fn user(uid: u32, gid: u32) -> Self {
        Self {
            real_uid: uid, effective_uid: uid, saved_uid: uid, fs_uid: uid,
            real_gid: gid, effective_gid: gid, saved_gid: gid, fs_gid: gid,
            supplementary_groups: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn is_root(&self) -> bool { self.effective_uid == 0 }

    #[inline(always)]
    pub fn in_group(&self, gid: u32) -> bool {
        self.effective_gid == gid || self.supplementary_groups.contains(&gid)
    }

    pub fn setuid(&mut self, uid: u32) -> bool {
        if self.effective_uid == 0 {
            self.real_uid = uid;
            self.effective_uid = uid;
            self.saved_uid = uid;
            self.fs_uid = uid;
            true
        } else if uid == self.real_uid || uid == self.saved_uid {
            self.effective_uid = uid;
            self.fs_uid = uid;
            true
        } else { false }
    }

    pub fn setgid(&mut self, gid: u32) -> bool {
        if self.effective_uid == 0 {
            self.real_gid = gid;
            self.effective_gid = gid;
            self.saved_gid = gid;
            self.fs_gid = gid;
            true
        } else if gid == self.real_gid || gid == self.saved_gid {
            self.effective_gid = gid;
            self.fs_gid = gid;
            true
        } else { false }
    }

    #[inline]
    pub fn seteuid(&mut self, euid: u32) -> bool {
        if self.effective_uid == 0 || euid == self.real_uid || euid == self.saved_uid {
            self.effective_uid = euid;
            self.fs_uid = euid;
            true
        } else { false }
    }

    #[inline]
    pub fn setreuid(&mut self, ruid: u32, euid: u32) -> bool {
        if self.effective_uid == 0 || ruid == self.real_uid || euid == self.real_uid || euid == self.saved_uid {
            if ruid != u32::MAX { self.real_uid = ruid; }
            if euid != u32::MAX { self.effective_uid = euid; self.fs_uid = euid; }
            self.saved_uid = self.effective_uid;
            true
        } else { false }
    }

    pub fn setresuid(&mut self, ruid: u32, euid: u32, suid: u32) -> bool {
        if self.effective_uid == 0 {
            if ruid != u32::MAX { self.real_uid = ruid; }
            if euid != u32::MAX { self.effective_uid = euid; self.fs_uid = euid; }
            if suid != u32::MAX { self.saved_uid = suid; }
            true
        } else {
            let ok = (ruid == u32::MAX || ruid == self.real_uid || ruid == self.effective_uid || ruid == self.saved_uid)
                && (euid == u32::MAX || euid == self.real_uid || euid == self.effective_uid || euid == self.saved_uid)
                && (suid == u32::MAX || suid == self.real_uid || suid == self.effective_uid || suid == self.saved_uid);
            if ok {
                if ruid != u32::MAX { self.real_uid = ruid; }
                if euid != u32::MAX { self.effective_uid = euid; self.fs_uid = euid; }
                if suid != u32::MAX { self.saved_uid = suid; }
            }
            ok
        }
    }

    #[inline]
    pub fn set_groups(&mut self, groups: Vec<u32>) -> bool {
        if self.effective_uid != 0 { return false; }
        self.supplementary_groups = groups;
        true
    }

    #[inline(always)]
    pub fn fork_creds(&self) -> Self { self.clone() }
}

/// Credential change event
#[derive(Debug, Clone)]
pub struct CredChangeEvent {
    pub process_id: u64,
    pub change_type: CredChangeType,
    pub old_uid: u32,
    pub new_uid: u32,
    pub old_gid: u32,
    pub new_gid: u32,
    pub timestamp_ns: u64,
    pub success: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredChangeType {
    Setuid,
    Setgid,
    Seteuid,
    Setreuid,
    Setresuid,
    Setregid,
    Setresgid,
    SetGroups,
    ExecSetuid,
}

/// Per-process credential tracking
#[derive(Debug, Clone)]
pub struct ProcessCreds {
    pub process_id: u64,
    pub creds: CredentialSet,
    pub namespace_id: u64,
    pub no_new_privs: bool,
    pub dumpable: bool,
    pub credential_changes: u64,
}

impl ProcessCreds {
    pub fn new(pid: u64, creds: CredentialSet) -> Self {
        Self {
            process_id: pid,
            creds,
            namespace_id: 0,
            no_new_privs: false,
            dumpable: true,
            credential_changes: 0,
        }
    }
}

/// Bridge credential manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BridgeCredMgrStats {
    pub total_processes: usize,
    pub root_processes: usize,
    pub total_cred_changes: u64,
    pub failed_changes: u64,
    pub total_events: usize,
}

/// Bridge Credential Manager
#[repr(align(64))]
pub struct BridgeCredMgr {
    processes: BTreeMap<u64, ProcessCreds>,
    events: VecDeque<CredChangeEvent>,
    max_events: usize,
    stats: BridgeCredMgrStats,
}

impl BridgeCredMgr {
    pub fn new(max_events: usize) -> Self {
        Self {
            processes: BTreeMap::new(),
            events: VecDeque::new(),
            max_events,
            stats: BridgeCredMgrStats::default(),
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64, creds: CredentialSet) {
        self.processes.insert(pid, ProcessCreds::new(pid, creds));
    }

    #[inline]
    pub fn fork_process(&mut self, parent_pid: u64, child_pid: u64) {
        if let Some(parent) = self.processes.get(&parent_pid) {
            let child_creds = parent.creds.fork_creds();
            let mut child = ProcessCreds::new(child_pid, child_creds);
            child.namespace_id = parent.namespace_id;
            child.no_new_privs = parent.no_new_privs;
            self.processes.insert(child_pid, child);
        }
    }

    #[inline]
    pub fn setuid(&mut self, pid: u64, uid: u32, ts: u64) -> bool {
        if let Some(proc_creds) = self.processes.get_mut(&pid) {
            let old_uid = proc_creds.creds.effective_uid;
            let old_gid = proc_creds.creds.effective_gid;
            let success = proc_creds.creds.setuid(uid);
            proc_creds.credential_changes += 1;
            self.emit_event(pid, CredChangeType::Setuid, old_uid, uid, old_gid, old_gid, ts, success);
            success
        } else { false }
    }

    #[inline]
    pub fn setgid(&mut self, pid: u64, gid: u32, ts: u64) -> bool {
        if let Some(proc_creds) = self.processes.get_mut(&pid) {
            let old_uid = proc_creds.creds.effective_uid;
            let old_gid = proc_creds.creds.effective_gid;
            let success = proc_creds.creds.setgid(gid);
            proc_creds.credential_changes += 1;
            self.emit_event(pid, CredChangeType::Setgid, old_uid, old_uid, old_gid, gid, ts, success);
            success
        } else { false }
    }

    pub fn check_access(&self, pid: u64, required_uid: Option<u32>, required_gid: Option<u32>) -> bool {
        if let Some(proc_creds) = self.processes.get(&pid) {
            if proc_creds.creds.is_root() { return true; }
            if let Some(uid) = required_uid {
                if proc_creds.creds.effective_uid != uid { return false; }
            }
            if let Some(gid) = required_gid {
                if !proc_creds.creds.in_group(gid) { return false; }
            }
            true
        } else { false }
    }

    fn emit_event(&mut self, pid: u64, ctype: CredChangeType, old_uid: u32, new_uid: u32, old_gid: u32, new_gid: u32, ts: u64, success: bool) {
        self.events.push_back(CredChangeEvent {
            process_id: pid, change_type: ctype,
            old_uid, new_uid, old_gid, new_gid,
            timestamp_ns: ts, success,
        });
        while self.events.len() > self.max_events { self.events.pop_front(); }
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u64) { self.processes.remove(&pid); }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_processes = self.processes.len();
        self.stats.root_processes = self.processes.values().filter(|p| p.creds.is_root()).count();
        self.stats.total_cred_changes = self.processes.values().map(|p| p.credential_changes).sum();
        self.stats.failed_changes = self.events.iter().filter(|e| !e.success).count() as u64;
        self.stats.total_events = self.events.len();
    }

    #[inline(always)]
    pub fn process_creds(&self, pid: u64) -> Option<&ProcessCreds> { self.processes.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &BridgeCredMgrStats { &self.stats }
}
