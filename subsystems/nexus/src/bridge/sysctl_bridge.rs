// SPDX-License-Identifier: GPL-2.0
//! Bridge sysctl_bridge — sysctl parameter management bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Sysctl value type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysctlValueType {
    Integer,
    Long,
    String,
    Binary,
    Ulong,
    Bool,
}

/// Sysctl permission
#[derive(Debug, Clone, Copy)]
pub struct SysctlPerm {
    pub bits: u16,
}

impl SysctlPerm {
    pub const READ: u16 = 0o444;
    pub const WRITE: u16 = 0o200;
    pub const RW: u16 = 0o644;
    pub const ROOT_RW: u16 = 0o600;

    pub fn new(bits: u16) -> Self { Self { bits } }
    pub fn is_readable(&self) -> bool { self.bits & 0o444 != 0 }
    pub fn is_writable(&self) -> bool { self.bits & 0o222 != 0 }
}

/// Sysctl namespace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SysctlNs {
    Kernel,
    Vm,
    Net,
    Fs,
    Debug,
    Dev,
    User,
}

/// Sysctl parameter entry
#[derive(Debug, Clone)]
pub struct SysctlParam {
    pub id: u64,
    pub path: String,
    pub ns: SysctlNs,
    pub value_type: SysctlValueType,
    pub int_val: i64,
    pub str_val: String,
    pub min_val: i64,
    pub max_val: i64,
    pub default_val: i64,
    pub perm: SysctlPerm,
    pub read_count: u64,
    pub write_count: u64,
    pub last_modified: u64,
}

impl SysctlParam {
    pub fn new_int(id: u64, path: String, ns: SysctlNs, value: i64) -> Self {
        Self {
            id, path, ns, value_type: SysctlValueType::Integer,
            int_val: value, str_val: String::new(),
            min_val: i64::MIN, max_val: i64::MAX, default_val: value,
            perm: SysctlPerm::new(SysctlPerm::RW),
            read_count: 0, write_count: 0, last_modified: 0,
        }
    }

    pub fn new_string(id: u64, path: String, ns: SysctlNs, value: String) -> Self {
        Self {
            id, path, ns, value_type: SysctlValueType::String,
            int_val: 0, str_val: value,
            min_val: 0, max_val: 0, default_val: 0,
            perm: SysctlPerm::new(SysctlPerm::RW),
            read_count: 0, write_count: 0, last_modified: 0,
        }
    }

    pub fn read(&mut self) -> i64 {
        self.read_count += 1;
        self.int_val
    }

    pub fn write(&mut self, value: i64, now: u64) -> bool {
        if value < self.min_val || value > self.max_val { return false; }
        self.int_val = value;
        self.write_count += 1;
        self.last_modified = now;
        true
    }

    pub fn is_default(&self) -> bool { self.int_val == self.default_val }

    pub fn set_range(&mut self, min: i64, max: i64) {
        self.min_val = min;
        self.max_val = max;
    }

    pub fn fnv_hash(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in self.path.as_bytes() {
            hash ^= *b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }
}

/// Sysctl table (directory node)
#[derive(Debug, Clone)]
pub struct SysctlTable {
    pub ns: SysctlNs,
    pub param_ids: Vec<u64>,
    pub child_tables: Vec<SysctlNs>,
}

/// Sysctl change event
#[derive(Debug, Clone)]
pub struct SysctlChangeEvent {
    pub param_id: u64,
    pub old_val: i64,
    pub new_val: i64,
    pub pid: u64,
    pub timestamp: u64,
}

/// Bridge stats
#[derive(Debug, Clone)]
pub struct SysctlBridgeStats {
    pub total_params: u32,
    pub total_reads: u64,
    pub total_writes: u64,
    pub modified_params: u32,
    pub params_by_ns: BTreeMap<u8, u32>,
}

/// Main sysctl bridge
pub struct BridgeSysctl {
    params: BTreeMap<u64, SysctlParam>,
    changes: Vec<SysctlChangeEvent>,
    path_index: BTreeMap<u64, u64>, // hash → param id
    next_id: u64,
    max_changes: usize,
}

impl BridgeSysctl {
    pub fn new() -> Self {
        Self {
            params: BTreeMap::new(), changes: Vec::new(),
            path_index: BTreeMap::new(), next_id: 1, max_changes: 4096,
        }
    }

    pub fn register_int(&mut self, path: String, ns: SysctlNs, value: i64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let param = SysctlParam::new_int(id, path, ns, value);
        let hash = param.fnv_hash();
        self.path_index.insert(hash, id);
        self.params.insert(id, param);
        id
    }

    pub fn register_string(&mut self, path: String, ns: SysctlNs, value: String) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let param = SysctlParam::new_string(id, path, ns, value);
        let hash = param.fnv_hash();
        self.path_index.insert(hash, id);
        self.params.insert(id, param);
        id
    }

    pub fn read(&mut self, id: u64) -> Option<i64> {
        self.params.get_mut(&id).map(|p| p.read())
    }

    pub fn write(&mut self, id: u64, value: i64, pid: u64, now: u64) -> bool {
        if let Some(param) = self.params.get_mut(&id) {
            let old = param.int_val;
            if param.write(value, now) {
                if self.changes.len() >= self.max_changes { self.changes.drain(..self.max_changes / 4); }
                self.changes.push(SysctlChangeEvent { param_id: id, old_val: old, new_val: value, pid, timestamp: now });
                return true;
            }
        }
        false
    }

    pub fn stats(&self) -> SysctlBridgeStats {
        let modified = self.params.values().filter(|p| !p.is_default()).count() as u32;
        let total_reads: u64 = self.params.values().map(|p| p.read_count).sum();
        let total_writes: u64 = self.params.values().map(|p| p.write_count).sum();
        let mut by_ns = BTreeMap::new();
        for p in self.params.values() { *by_ns.entry(p.ns as u8).or_insert(0u32) += 1; }
        SysctlBridgeStats {
            total_params: self.params.len() as u32,
            total_reads, total_writes,
            modified_params: modified, params_by_ns: by_ns,
        }
    }
}
