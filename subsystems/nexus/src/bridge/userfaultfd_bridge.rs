// SPDX-License-Identifier: GPL-2.0
//! Bridge userfaultfd_bridge — userfaultfd page fault handling bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Userfaultfd feature flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UffdFeatures(pub u64);

impl UffdFeatures {
    pub const PAGEFAULT_FLAG_WP: u64 = 1 << 0;
    pub const EVENT_FORK: u64 = 1 << 1;
    pub const EVENT_REMAP: u64 = 1 << 2;
    pub const EVENT_REMOVE: u64 = 1 << 3;
    pub const EVENT_UNMAP: u64 = 1 << 4;
    pub const MISSING_HUGETLB: u64 = 1 << 5;
    pub const MISSING_SHMEM: u64 = 1 << 6;
    pub const SIGBUS: u64 = 1 << 7;
    pub const THREAD_ID: u64 = 1 << 8;
    pub const MINOR_HUGETLB: u64 = 1 << 9;
    pub const MINOR_SHMEM: u64 = 1 << 10;
    pub const EXACT_ADDRESS: u64 = 1 << 11;
    pub const WP_HUGETLB: u64 = 1 << 12;
    pub const WP_UNPOPULATED: u64 = 1 << 13;
    pub const POISON: u64 = 1 << 14;
    pub const WP_ASYNC: u64 = 1 << 15;

    pub fn new() -> Self { Self(0) }
    pub fn set(&mut self, f: u64) { self.0 |= f; }
    pub fn has(&self, f: u64) -> bool { self.0 & f != 0 }
}

/// Fault type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UffdFaultType {
    Missing,
    WriteProtect,
    MinorFault,
}

/// Fault event
#[derive(Debug, Clone)]
pub struct UffdEvent {
    pub id: u64,
    pub fault_type: UffdFaultType,
    pub address: u64,
    pub tid: u64,
    pub timestamp: u64,
    pub resolved: bool,
    pub resolution_ns: u64,
}

impl UffdEvent {
    pub fn new(id: u64, ftype: UffdFaultType, addr: u64, tid: u64, now: u64) -> Self {
        Self { id, fault_type: ftype, address: addr, tid, timestamp: now, resolved: false, resolution_ns: 0 }
    }

    pub fn resolve(&mut self, now: u64) {
        self.resolved = true;
        self.resolution_ns = now.saturating_sub(self.timestamp);
    }
}

/// Registration range
#[derive(Debug, Clone)]
pub struct UffdRange {
    pub start: u64,
    pub length: u64,
    pub mode: UffdRegMode,
}

/// Registration mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UffdRegMode {
    Missing,
    WriteProtect,
    Minor,
    MissingWp,
}

/// Userfaultfd instance
#[derive(Debug)]
pub struct UffdInstance {
    pub id: u64,
    pub features: UffdFeatures,
    pub ranges: Vec<UffdRange>,
    pub pending_events: Vec<UffdEvent>,
    pub total_faults: u64,
    pub total_resolved: u64,
    pub total_copy: u64,
    pub total_zeropage: u64,
    pub total_wp: u64,
    pub created_at: u64,
}

impl UffdInstance {
    pub fn new(id: u64, features: UffdFeatures, now: u64) -> Self {
        Self {
            id, features, ranges: Vec::new(), pending_events: Vec::new(),
            total_faults: 0, total_resolved: 0, total_copy: 0,
            total_zeropage: 0, total_wp: 0, created_at: now,
        }
    }

    pub fn register(&mut self, start: u64, length: u64, mode: UffdRegMode) {
        self.ranges.push(UffdRange { start, length, mode });
    }

    pub fn fault(&mut self, ftype: UffdFaultType, addr: u64, tid: u64, now: u64) -> u64 {
        self.total_faults += 1;
        let event = UffdEvent::new(self.total_faults, ftype, addr, tid, now);
        let eid = event.id;
        self.pending_events.push(event);
        eid
    }

    pub fn resolve_copy(&mut self, event_id: u64, now: u64) {
        if let Some(ev) = self.pending_events.iter_mut().find(|e| e.id == event_id) {
            ev.resolve(now);
            self.total_resolved += 1;
            self.total_copy += 1;
        }
    }

    pub fn resolve_zeropage(&mut self, event_id: u64, now: u64) {
        if let Some(ev) = self.pending_events.iter_mut().find(|e| e.id == event_id) {
            ev.resolve(now);
            self.total_resolved += 1;
            self.total_zeropage += 1;
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct UffdBridgeStats {
    pub total_instances: u32,
    pub total_ranges: u32,
    pub total_faults: u64,
    pub total_resolved: u64,
    pub pending_faults: u32,
    pub avg_resolution_ns: u64,
}

/// Main userfaultfd bridge
pub struct BridgeUserfaultfd {
    instances: BTreeMap<u64, UffdInstance>,
    next_id: u64,
}

impl BridgeUserfaultfd {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, features: UffdFeatures, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.instances.insert(id, UffdInstance::new(id, features, now));
        id
    }

    pub fn register(&mut self, id: u64, start: u64, length: u64, mode: UffdRegMode) {
        if let Some(inst) = self.instances.get_mut(&id) { inst.register(start, length, mode); }
    }

    pub fn fault(&mut self, id: u64, ftype: UffdFaultType, addr: u64, tid: u64, now: u64) -> Option<u64> {
        self.instances.get_mut(&id).map(|inst| inst.fault(ftype, addr, tid, now))
    }

    pub fn stats(&self) -> UffdBridgeStats {
        let ranges: u32 = self.instances.values().map(|i| i.ranges.len() as u32).sum();
        let faults: u64 = self.instances.values().map(|i| i.total_faults).sum();
        let resolved: u64 = self.instances.values().map(|i| i.total_resolved).sum();
        let pending: u32 = self.instances.values().map(|i| i.pending_events.iter().filter(|e| !e.resolved).count() as u32).sum();
        let res_times: Vec<u64> = self.instances.values().flat_map(|i| &i.pending_events).filter(|e| e.resolved).map(|e| e.resolution_ns).collect();
        let avg = if res_times.is_empty() { 0 } else { res_times.iter().sum::<u64>() / res_times.len() as u64 };
        UffdBridgeStats { total_instances: self.instances.len() as u32, total_ranges: ranges, total_faults: faults, total_resolved: resolved, pending_faults: pending, avg_resolution_ns: avg }
    }
}

// ============================================================================
// Merged from userfaultfd_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UffdV2Type {
    Missing,
    Wp,
    Minor,
}

/// Userfault feature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UffdV2Feature {
    Pagefault,
    EventFork,
    EventRemap,
    EventRemove,
    EventUnmap,
    MissingHugeTlb,
    MissingShmem,
    MinorHugeTlb,
    MinorShmem,
    ExactAddress,
    WriteProtect,
}

/// Userfault event
#[derive(Debug)]
pub struct UffdV2Event {
    pub id: u64,
    pub fault_type: UffdV2Type,
    pub address: u64,
    pub pid: u64,
    pub timestamp: u64,
    pub resolved: bool,
    pub resolution_ns: u64,
}

/// Userfaultfd v2 instance
#[derive(Debug)]
pub struct UffdV2Instance {
    pub fd: u64,
    pub pid: u64,
    pub features: u64,
    pub registered_ranges: Vec<(u64, u64)>,
    pub events: Vec<UffdV2Event>,
    pub total_faults: u64,
    pub total_resolved: u64,
}

impl UffdV2Instance {
    pub fn new(fd: u64, pid: u64, features: u64) -> Self {
        Self { fd, pid, features, registered_ranges: Vec::new(), events: Vec::new(), total_faults: 0, total_resolved: 0 }
    }

    pub fn register_range(&mut self, start: u64, len: u64) { self.registered_ranges.push((start, len)); }

    pub fn fault(&mut self, id: u64, ft: UffdV2Type, addr: u64, now: u64) {
        self.events.push(UffdV2Event { id, fault_type: ft, address: addr, pid: self.pid, timestamp: now, resolved: false, resolution_ns: 0 });
        self.total_faults += 1;
    }

    pub fn resolve(&mut self, id: u64, now: u64) {
        if let Some(ev) = self.events.iter_mut().find(|e| e.id == id && !e.resolved) {
            ev.resolved = true;
            ev.resolution_ns = now.saturating_sub(ev.timestamp);
            self.total_resolved += 1;
        }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct UffdV2BridgeStats {
    pub total_instances: u32,
    pub total_faults: u64,
    pub total_resolved: u64,
    pub pending_faults: u64,
    pub avg_resolution_ns: u64,
}

/// Main bridge userfaultfd v2
pub struct BridgeUserfaultfdV2 {
    instances: BTreeMap<u64, UffdV2Instance>,
}

impl BridgeUserfaultfdV2 {
    pub fn new() -> Self { Self { instances: BTreeMap::new() } }

    pub fn create(&mut self, fd: u64, pid: u64, features: u64) { self.instances.insert(fd, UffdV2Instance::new(fd, pid, features)); }

    pub fn close(&mut self, fd: u64) { self.instances.remove(&fd); }

    pub fn stats(&self) -> UffdV2BridgeStats {
        let faults: u64 = self.instances.values().map(|i| i.total_faults).sum();
        let resolved: u64 = self.instances.values().map(|i| i.total_resolved).sum();
        let pending = faults - resolved;
        let res_times: Vec<u64> = self.instances.values().flat_map(|i| i.events.iter()).filter(|e| e.resolved).map(|e| e.resolution_ns).collect();
        let avg = if res_times.is_empty() { 0 } else { res_times.iter().sum::<u64>() / res_times.len() as u64 };
        UffdV2BridgeStats { total_instances: self.instances.len() as u32, total_faults: faults, total_resolved: resolved, pending_faults: pending, avg_resolution_ns: avg }
    }
}
