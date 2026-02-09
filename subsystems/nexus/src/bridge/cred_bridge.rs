// SPDX-License-Identifier: GPL-2.0
//! Bridge cred_bridge — process credential management bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Credential type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredType {
    Real,
    Effective,
    Saved,
    FileSystem,
}

/// Capability set
#[derive(Debug, Clone, Copy)]
pub struct CapSet {
    pub effective: u64,
    pub permitted: u64,
    pub inheritable: u64,
    pub bounding: u64,
    pub ambient: u64,
}

impl CapSet {
    #[inline(always)]
    pub fn empty() -> Self { Self { effective: 0, permitted: 0, inheritable: 0, bounding: 0, ambient: 0 } }
    #[inline(always)]
    pub fn full() -> Self { Self { effective: u64::MAX, permitted: u64::MAX, inheritable: 0, bounding: u64::MAX, ambient: 0 } }
    #[inline(always)]
    pub fn has_cap(&self, cap: u8) -> bool { self.effective & (1u64 << cap) != 0 }
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
    pub caps: CapSet,
    pub securebits: u32,
    pub supplementary_groups: Vec<u32>,
    pub no_new_privs: bool,
}

impl ProcessCred {
    #[inline(always)]
    pub fn root() -> Self {
        Self { pid: 0, uid: 0, gid: 0, euid: 0, egid: 0, suid: 0, sgid: 0, fsuid: 0, fsgid: 0, caps: CapSet::full(), securebits: 0, supplementary_groups: Vec::new(), no_new_privs: false }
    }
    #[inline(always)]
    pub fn is_privileged(&self) -> bool { self.euid == 0 || self.caps.effective != 0 }
    #[inline(always)]
    pub fn in_group(&self, gid: u32) -> bool { self.egid == gid || self.supplementary_groups.contains(&gid) }
}

/// Credential change event
#[derive(Debug, Clone)]
pub struct CredChangeEvent {
    pub pid: u64,
    pub cred_type: CredType,
    pub old_uid: u32,
    pub new_uid: u32,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CredBridgeStats {
    pub tracked_processes: u32,
    pub privileged_processes: u32,
    pub cred_changes: u64,
    pub setuid_calls: u64,
}

/// Main credential bridge
#[repr(align(64))]
pub struct BridgeCred {
    creds: BTreeMap<u64, ProcessCred>,
    events: Vec<CredChangeEvent>,
    max_events: usize,
}

impl BridgeCred {
    pub fn new() -> Self { Self { creds: BTreeMap::new(), events: Vec::new(), max_events: 4096 } }

    #[inline(always)]
    pub fn register(&mut self, cred: ProcessCred) { self.creds.insert(cred.pid, cred); }
    #[inline(always)]
    pub fn unregister(&mut self, pid: u64) { self.creds.remove(&pid); }

    #[inline]
    pub fn setuid(&mut self, pid: u64, new_uid: u32, now: u64) {
        if let Some(c) = self.creds.get_mut(&pid) {
            let old = c.uid;
            c.uid = new_uid; c.euid = new_uid; c.suid = new_uid;
            if self.events.len() >= self.max_events { self.events.drain(..self.max_events / 2); }
            self.events.push(CredChangeEvent { pid, cred_type: CredType::Real, old_uid: old, new_uid, timestamp: now });
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> CredBridgeStats {
        let priv_count = self.creds.values().filter(|c| c.is_privileged()).count() as u32;
        CredBridgeStats { tracked_processes: self.creds.len() as u32, privileged_processes: priv_count, cred_changes: self.events.len() as u64, setuid_calls: self.events.iter().filter(|e| e.cred_type == CredType::Real).count() as u64 }
    }
}

// ============================================================================
// Merged from cred_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredV2Type {
    Real,
    Effective,
    Saved,
    FileSystem,
}

/// Capability set v2
#[derive(Debug, Clone)]
pub struct CapabilitySetV2 {
    pub effective: u64,
    pub permitted: u64,
    pub inheritable: u64,
    pub bounding: u64,
    pub ambient: u64,
}

impl CapabilitySetV2 {
    pub fn new() -> Self { Self { effective: 0, permitted: 0, inheritable: 0, bounding: u64::MAX, ambient: 0 } }
    #[inline(always)]
    pub fn has_cap(&self, bit: u32) -> bool { (self.effective >> bit) & 1 == 1 }
}

/// Process credentials v2
#[derive(Debug)]
pub struct ProcessCredV2 {
    pub pid: u64,
    pub uid: u32,
    pub gid: u32,
    pub euid: u32,
    pub egid: u32,
    pub suid: u32,
    pub sgid: u32,
    pub fsuid: u32,
    pub fsgid: u32,
    pub caps: CapabilitySetV2,
    pub securebits: u32,
    pub groups: Vec<u32>,
    pub no_new_privs: bool,
}

impl ProcessCredV2 {
    pub fn new(pid: u64, uid: u32, gid: u32) -> Self {
        Self { pid, uid, gid, euid: uid, egid: gid, suid: uid, sgid: gid, fsuid: uid, fsgid: gid, caps: CapabilitySetV2::new(), securebits: 0, groups: Vec::new(), no_new_privs: false }
    }

    #[inline(always)]
    pub fn is_root(&self) -> bool { self.euid == 0 }
    #[inline(always)]
    pub fn in_group(&self, gid: u32) -> bool { self.egid == gid || self.groups.contains(&gid) }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CredV2BridgeStats {
    pub tracked_processes: u32,
    pub root_processes: u32,
    pub no_new_privs_count: u32,
}

/// Main bridge cred v2
#[repr(align(64))]
pub struct BridgeCredV2 {
    creds: BTreeMap<u64, ProcessCredV2>,
}

impl BridgeCredV2 {
    pub fn new() -> Self { Self { creds: BTreeMap::new() } }
    #[inline(always)]
    pub fn track(&mut self, pid: u64, uid: u32, gid: u32) { self.creds.insert(pid, ProcessCredV2::new(pid, uid, gid)); }
    #[inline(always)]
    pub fn setuid(&mut self, pid: u64, uid: u32) { if let Some(c) = self.creds.get_mut(&pid) { c.euid = uid; c.fsuid = uid; } }
    #[inline(always)]
    pub fn setgid(&mut self, pid: u64, gid: u32) { if let Some(c) = self.creds.get_mut(&pid) { c.egid = gid; c.fsgid = gid; } }
    #[inline(always)]
    pub fn untrack(&mut self, pid: u64) { self.creds.remove(&pid); }

    #[inline]
    pub fn stats(&self) -> CredV2BridgeStats {
        let root = self.creds.values().filter(|c| c.is_root()).count() as u32;
        let nnp = self.creds.values().filter(|c| c.no_new_privs).count() as u32;
        CredV2BridgeStats { tracked_processes: self.creds.len() as u32, root_processes: root, no_new_privs_count: nnp }
    }
}
