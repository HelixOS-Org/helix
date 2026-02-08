// SPDX-License-Identifier: GPL-2.0
//! Bridge kcmp_bridge — kernel resource comparison for process dedup.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Kcmp resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcmpType {
    File,
    Vm,
    Files,
    Fs,
    Sighand,
    Io,
    Sysvsem,
    Epoll,
}

/// Comparison result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcmpResult {
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    Error,
}

/// Kcmp request
#[derive(Debug, Clone)]
pub struct KcmpRequest {
    pub pid1: u64,
    pub pid2: u64,
    pub resource_type: KcmpType,
    pub idx1: u64,
    pub idx2: u64,
    pub timestamp: u64,
}

impl KcmpRequest {
    pub fn new(pid1: u64, pid2: u64, rtype: KcmpType, idx1: u64, idx2: u64, now: u64) -> Self {
        Self { pid1, pid2, resource_type: rtype, idx1, idx2, timestamp: now }
    }
}

/// Kcmp response
#[derive(Debug, Clone)]
pub struct KcmpResponse {
    pub request: KcmpRequest,
    pub result: KcmpResult,
    pub kernel_ptr_order: u64,
    pub duration_ns: u64,
}

/// Resource identity for dedup tracking
#[derive(Debug, Clone)]
pub struct ResourceIdentity {
    pub resource_type: KcmpType,
    pub kernel_id: u64,
    pub ref_count: u32,
    pub sharing_pids: Vec<u64>,
}

impl ResourceIdentity {
    pub fn new(rtype: KcmpType, kid: u64) -> Self {
        Self { resource_type: rtype, kernel_id: kid, ref_count: 1, sharing_pids: Vec::new() }
    }

    pub fn add_sharer(&mut self, pid: u64) {
        if !self.sharing_pids.contains(&pid) {
            self.sharing_pids.push(pid);
            self.ref_count = self.sharing_pids.len() as u32;
        }
    }

    pub fn is_shared(&self) -> bool { self.ref_count > 1 }
}

/// Process resource map
#[derive(Debug, Clone)]
pub struct ProcessResources {
    pub pid: u64,
    pub files: BTreeMap<u64, u64>,
    pub vm_id: u64,
    pub fs_id: u64,
    pub sighand_id: u64,
    pub io_id: u64,
}

impl ProcessResources {
    pub fn new(pid: u64) -> Self {
        Self { pid, files: BTreeMap::new(), vm_id: pid, fs_id: pid, sighand_id: pid, io_id: pid }
    }

    pub fn compare(&self, other: &ProcessResources, rtype: KcmpType, idx1: u64, idx2: u64) -> KcmpResult {
        match rtype {
            KcmpType::File => {
                let a = self.files.get(&idx1);
                let b = other.files.get(&idx2);
                match (a, b) {
                    (Some(&a), Some(&b)) if a == b => KcmpResult::Equal,
                    (Some(&a), Some(&b)) if a < b => KcmpResult::LessThan,
                    (Some(_), Some(_)) => KcmpResult::GreaterThan,
                    _ => KcmpResult::Error,
                }
            }
            KcmpType::Vm => if self.vm_id == other.vm_id { KcmpResult::Equal } else { KcmpResult::NotEqual },
            KcmpType::Fs => if self.fs_id == other.fs_id { KcmpResult::Equal } else { KcmpResult::NotEqual },
            KcmpType::Sighand => if self.sighand_id == other.sighand_id { KcmpResult::Equal } else { KcmpResult::NotEqual },
            KcmpType::Io => if self.io_id == other.io_id { KcmpResult::Equal } else { KcmpResult::NotEqual },
            _ => KcmpResult::Error,
        }
    }
}

/// Bridge stats
#[derive(Debug, Clone)]
pub struct KcmpBridgeStats {
    pub total_comparisons: u64,
    pub equal_results: u64,
    pub processes_tracked: u32,
    pub shared_resources: u32,
    pub avg_duration_ns: u64,
}

/// Main kcmp bridge
pub struct BridgeKcmp {
    processes: BTreeMap<u64, ProcessResources>,
    history: Vec<KcmpResponse>,
    max_history: usize,
}

impl BridgeKcmp {
    pub fn new() -> Self {
        Self { processes: BTreeMap::new(), history: Vec::new(), max_history: 4096 }
    }

    pub fn register_process(&mut self, pid: u64) {
        self.processes.entry(pid).or_insert_with(|| ProcessResources::new(pid));
    }

    pub fn compare(&mut self, pid1: u64, pid2: u64, rtype: KcmpType, idx1: u64, idx2: u64, now: u64) -> KcmpResult {
        let req = KcmpRequest::new(pid1, pid2, rtype, idx1, idx2, now);
        let result = match (self.processes.get(&pid1), self.processes.get(&pid2)) {
            (Some(p1), Some(p2)) => p1.compare(p2, rtype, idx1, idx2),
            _ => KcmpResult::Error,
        };
        let resp = KcmpResponse { request: req, result, kernel_ptr_order: 0, duration_ns: 0 };
        if self.history.len() >= self.max_history { self.history.drain(..self.max_history / 4); }
        self.history.push(resp);
        result
    }

    pub fn stats(&self) -> KcmpBridgeStats {
        let equal = self.history.iter().filter(|r| r.result == KcmpResult::Equal).count() as u64;
        let avg_dur = if self.history.is_empty() { 0 } else {
            self.history.iter().map(|r| r.duration_ns).sum::<u64>() / self.history.len() as u64
        };
        KcmpBridgeStats {
            total_comparisons: self.history.len() as u64,
            equal_results: equal, processes_tracked: self.processes.len() as u32,
            shared_resources: 0, avg_duration_ns: avg_dur,
        }
    }
}

// ============================================================================
// Merged from kcmp_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcmpV2Type {
    File,
    Vm,
    Files,
    Fs,
    Sighand,
    Io,
    Sysvsem,
    Epoll,
}

/// Comparison result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcmpV2Result {
    Equal,
    NotEqual,
    Less,
    Greater,
}

/// Resource identity
#[derive(Debug, Clone)]
pub struct ResourceId {
    pub kind: KcmpV2Type,
    pub pid: u64,
    pub index: u64,
    pub kernel_ptr_hash: u64,
}

impl ResourceId {
    pub fn hash_key(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        h ^= self.pid; h = h.wrapping_mul(0x100000001b3);
        h ^= self.index; h = h.wrapping_mul(0x100000001b3);
        h ^= self.kind as u64; h = h.wrapping_mul(0x100000001b3);
        h
    }
}

/// Comparison record
#[derive(Debug, Clone)]
pub struct KcmpV2Record {
    pub id: u64,
    pub pid1: u64,
    pub pid2: u64,
    pub cmp_type: KcmpV2Type,
    pub idx1: u64,
    pub idx2: u64,
    pub result: KcmpV2Result,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct KcmpV2BridgeStats {
    pub comparisons: u64,
    pub equal_results: u64,
    pub tracked_resources: u32,
    pub by_type: [u64; 8],
}

/// Main kcmp v2 bridge
pub struct BridgeKcmpV2 {
    resources: BTreeMap<u64, ResourceId>,
    records: Vec<KcmpV2Record>,
    next_id: u64,
    max_records: usize,
    type_counts: [u64; 8],
    equal_count: u64,
}

impl BridgeKcmpV2 {
    pub fn new() -> Self { Self { resources: BTreeMap::new(), records: Vec::new(), next_id: 1, max_records: 4096, type_counts: [0; 8], equal_count: 0 } }

    pub fn register_resource(&mut self, res: ResourceId) {
        let key = res.hash_key();
        self.resources.insert(key, res);
    }

    pub fn compare(&mut self, pid1: u64, pid2: u64, cmp_type: KcmpV2Type, idx1: u64, idx2: u64, now: u64) -> KcmpV2Result {
        let r1 = ResourceId { kind: cmp_type, pid: pid1, index: idx1, kernel_ptr_hash: 0 };
        let r2 = ResourceId { kind: cmp_type, pid: pid2, index: idx2, kernel_ptr_hash: 0 };
        let h1 = r1.hash_key();
        let h2 = r2.hash_key();
        let result = if h1 == h2 { KcmpV2Result::Equal } else if h1 < h2 { KcmpV2Result::Less } else { KcmpV2Result::Greater };
        if result == KcmpV2Result::Equal { self.equal_count += 1; }
        self.type_counts[cmp_type as usize] += 1;
        let id = self.next_id; self.next_id += 1;
        if self.records.len() >= self.max_records { self.records.drain(..self.max_records / 2); }
        self.records.push(KcmpV2Record { id, pid1, pid2, cmp_type, idx1, idx2, result, timestamp: now });
        result
    }

    pub fn stats(&self) -> KcmpV2BridgeStats {
        KcmpV2BridgeStats { comparisons: self.records.len() as u64, equal_results: self.equal_count, tracked_resources: self.resources.len() as u32, by_type: self.type_counts }
    }
}
