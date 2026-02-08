// SPDX-License-Identifier: GPL-2.0
//! Bridge pkey_bridge — memory protection keys bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Protection key access rights
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkeyAccess {
    ReadWrite,
    ReadOnly,
    NoAccess,
    WriteOnly,
}

/// Protection key
#[derive(Debug, Clone)]
pub struct Pkey {
    pub key: u32,
    pub access: PkeyAccess,
    pub allocated: bool,
    pub owner_pid: u64,
}

/// PKRU register state (32-bit: 2 bits per key, 16 keys)
#[derive(Debug, Clone, Copy)]
pub struct PkruState(pub u32);

impl PkruState {
    pub fn new() -> Self { Self(0) }

    pub fn set_key(&mut self, key: u32, access: PkeyAccess) {
        if key >= 16 { return; }
        let shift = key * 2;
        let mask = !(3u32 << shift);
        let bits = match access { PkeyAccess::ReadWrite => 0, PkeyAccess::NoAccess => 3, PkeyAccess::WriteOnly => 1, PkeyAccess::ReadOnly => 2 };
        self.0 = (self.0 & mask) | (bits << shift);
    }

    pub fn get_key(&self, key: u32) -> PkeyAccess {
        if key >= 16 { return PkeyAccess::NoAccess; }
        match (self.0 >> (key * 2)) & 3 { 0 => PkeyAccess::ReadWrite, 1 => PkeyAccess::WriteOnly, 2 => PkeyAccess::ReadOnly, _ => PkeyAccess::NoAccess }
    }
}

/// Per-process pkey state
#[derive(Debug)]
pub struct ProcessPkeys {
    pub pid: u64,
    pub pkru: PkruState,
    pub allocated_keys: Vec<u32>,
    pub violation_count: u64,
}

impl ProcessPkeys {
    pub fn new(pid: u64) -> Self { Self { pid, pkru: PkruState::new(), allocated_keys: Vec::new(), violation_count: 0 } }

    pub fn alloc_key(&mut self) -> Option<u32> {
        for k in 1u32..16 {
            if !self.allocated_keys.contains(&k) { self.allocated_keys.push(k); return Some(k); }
        }
        None
    }

    pub fn free_key(&mut self, key: u32) {
        if let Some(pos) = self.allocated_keys.iter().position(|&k| k == key) {
            self.allocated_keys.remove(pos);
            self.pkru.set_key(key, PkeyAccess::ReadWrite);
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PkeyBridgeStats {
    pub tracked_processes: u32,
    pub total_allocated_keys: u32,
    pub total_violations: u64,
}

/// Main pkey bridge
pub struct BridgePkey {
    processes: BTreeMap<u64, ProcessPkeys>,
}

impl BridgePkey {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }
    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessPkeys::new(pid)); }
    pub fn unregister(&mut self, pid: u64) { self.processes.remove(&pid); }

    pub fn alloc_key(&mut self, pid: u64) -> Option<u32> { self.processes.get_mut(&pid)?.alloc_key() }
    pub fn free_key(&mut self, pid: u64, key: u32) { if let Some(p) = self.processes.get_mut(&pid) { p.free_key(key); } }

    pub fn set_access(&mut self, pid: u64, key: u32, access: PkeyAccess) {
        if let Some(p) = self.processes.get_mut(&pid) { p.pkru.set_key(key, access); }
    }

    pub fn stats(&self) -> PkeyBridgeStats {
        let keys: u32 = self.processes.values().map(|p| p.allocated_keys.len() as u32).sum();
        let violations: u64 = self.processes.values().map(|p| p.violation_count).sum();
        PkeyBridgeStats { tracked_processes: self.processes.len() as u32, total_allocated_keys: keys, total_violations: violations }
    }
}

// ============================================================================
// Merged from pkey_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkeyV2Access {
    ReadWrite,
    WriteDisable,
    AccessDisable,
}

/// Pkey entry
#[derive(Debug)]
pub struct PkeyV2Entry {
    pub key: u32,
    pub access: PkeyV2Access,
    pub pid: u64,
    pub assigned_pages: u64,
    pub violations: u64,
    pub allocated_at: u64,
}

impl PkeyV2Entry {
    pub fn new(key: u32, pid: u64, now: u64) -> Self {
        Self { key, access: PkeyV2Access::ReadWrite, pid, assigned_pages: 0, violations: 0, allocated_at: now }
    }
}

/// PKRU register state
#[derive(Debug)]
pub struct PkruState {
    pub pid: u64,
    pub pkru_value: u32,
    pub last_modified: u64,
}

impl PkruState {
    pub fn new(pid: u64) -> Self { Self { pid, pkru_value: 0, last_modified: 0 } }

    pub fn set_key_access(&mut self, key: u32, disable_write: bool, disable_access: bool) {
        let bits = ((disable_access as u32) << 1) | (disable_write as u32);
        let shift = key * 2;
        self.pkru_value = (self.pkru_value & !(3 << shift)) | (bits << shift);
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PkeyV2BridgeStats {
    pub total_keys: u32,
    pub active_processes: u32,
    pub total_violations: u64,
    pub total_assigned_pages: u64,
}

/// Main bridge pkey v2
pub struct BridgePkeyV2 {
    keys: BTreeMap<u32, PkeyV2Entry>,
    pkru_states: BTreeMap<u64, PkruState>,
}

impl BridgePkeyV2 {
    pub fn new() -> Self { Self { keys: BTreeMap::new(), pkru_states: BTreeMap::new() } }

    pub fn alloc(&mut self, key: u32, pid: u64, now: u64) {
        self.keys.insert(key, PkeyV2Entry::new(key, pid, now));
        self.pkru_states.entry(pid).or_insert_with(|| PkruState::new(pid));
    }

    pub fn free(&mut self, key: u32) { self.keys.remove(&key); }

    pub fn set_access(&mut self, pid: u64, key: u32, dw: bool, da: bool) {
        if let Some(ps) = self.pkru_states.get_mut(&pid) { ps.set_key_access(key, dw, da); }
        if let Some(ke) = self.keys.get_mut(&key) {
            ke.access = if da { PkeyV2Access::AccessDisable } else if dw { PkeyV2Access::WriteDisable } else { PkeyV2Access::ReadWrite };
        }
    }

    pub fn stats(&self) -> PkeyV2BridgeStats {
        let violations: u64 = self.keys.values().map(|k| k.violations).sum();
        let pages: u64 = self.keys.values().map(|k| k.assigned_pages).sum();
        PkeyV2BridgeStats { total_keys: self.keys.len() as u32, active_processes: self.pkru_states.len() as u32, total_violations: violations, total_assigned_pages: pages }
    }
}
