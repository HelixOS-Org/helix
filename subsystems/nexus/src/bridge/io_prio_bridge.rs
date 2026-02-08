// SPDX-License-Identifier: GPL-2.0
//! Bridge io_prio_bridge — I/O priority management bridge.

extern crate alloc;

use alloc::collections::BTreeMap;

/// I/O scheduling class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoSchedClass {
    RealTime,
    BestEffort,
    Idle,
    None,
}

/// I/O priority (class + level 0-7)
#[derive(Debug, Clone, Copy)]
pub struct IoPriority {
    pub class: IoSchedClass,
    pub level: u8,
}

impl IoPriority {
    pub fn new(class: IoSchedClass, level: u8) -> Self { Self { class, level: level.min(7) } }
    pub fn default_prio() -> Self { Self { class: IoSchedClass::BestEffort, level: 4 } }
    pub fn encode(&self) -> u16 { ((self.class as u16) << 13) | (self.level as u16) }
    pub fn decode(val: u16) -> Self {
        let class = match val >> 13 { 0 => IoSchedClass::None, 1 => IoSchedClass::RealTime, 2 => IoSchedClass::BestEffort, _ => IoSchedClass::Idle };
        Self { class, level: (val & 7) as u8 }
    }
}

/// Per-process I/O priority
#[derive(Debug)]
pub struct ProcessIoPrio {
    pub pid: u64,
    pub prio: IoPriority,
    pub io_bytes_read: u64,
    pub io_bytes_written: u64,
    pub io_ops: u64,
    pub throttled_ns: u64,
}

impl ProcessIoPrio {
    pub fn new(pid: u64) -> Self { Self { pid, prio: IoPriority::default_prio(), io_bytes_read: 0, io_bytes_written: 0, io_ops: 0, throttled_ns: 0 } }
    pub fn total_bytes(&self) -> u64 { self.io_bytes_read + self.io_bytes_written }
}

/// I/O priority change event
#[derive(Debug, Clone)]
pub struct IoPrioEvent {
    pub pid: u64,
    pub old_class: IoSchedClass,
    pub new_class: IoSchedClass,
    pub old_level: u8,
    pub new_level: u8,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct IoPrioBridgeStats {
    pub tracked_processes: u32,
    pub rt_processes: u32,
    pub idle_processes: u32,
    pub total_io_bytes: u64,
    pub total_io_ops: u64,
    pub prio_changes: u64,
}

/// Main I/O priority bridge
pub struct BridgeIoPrio {
    processes: BTreeMap<u64, ProcessIoPrio>,
    prio_changes: u64,
}

impl BridgeIoPrio {
    pub fn new() -> Self { Self { processes: BTreeMap::new(), prio_changes: 0 } }

    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessIoPrio::new(pid)); }
    pub fn unregister(&mut self, pid: u64) { self.processes.remove(&pid); }

    pub fn set_priority(&mut self, pid: u64, class: IoSchedClass, level: u8) {
        if let Some(p) = self.processes.get_mut(&pid) { p.prio = IoPriority::new(class, level); self.prio_changes += 1; }
    }

    pub fn record_io(&mut self, pid: u64, read: u64, written: u64) {
        if let Some(p) = self.processes.get_mut(&pid) { p.io_bytes_read += read; p.io_bytes_written += written; p.io_ops += 1; }
    }

    pub fn stats(&self) -> IoPrioBridgeStats {
        let rt = self.processes.values().filter(|p| p.prio.class == IoSchedClass::RealTime).count() as u32;
        let idle = self.processes.values().filter(|p| p.prio.class == IoSchedClass::Idle).count() as u32;
        let bytes: u64 = self.processes.values().map(|p| p.total_bytes()).sum();
        let ops: u64 = self.processes.values().map(|p| p.io_ops).sum();
        IoPrioBridgeStats { tracked_processes: self.processes.len() as u32, rt_processes: rt, idle_processes: idle, total_io_bytes: bytes, total_io_ops: ops, prio_changes: self.prio_changes }
    }
}
