// SPDX-License-Identifier: GPL-2.0
//! Bridge ioprio_bridge — I/O priority bridge.

extern crate alloc;

use alloc::collections::BTreeMap;

/// I/O priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoprioClass {
    None,
    RealTime,
    BestEffort,
    Idle,
}

/// I/O priority who
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoprioWho {
    Process,
    ProcessGroup,
    User,
}

/// I/O priority entry
#[derive(Debug)]
pub struct IoprioEntry {
    pub who: IoprioWho,
    pub who_id: u64,
    pub class: IoprioClass,
    pub data: u8,
    pub set_at: u64,
    pub io_ops: u64,
    pub io_bytes: u64,
}

impl IoprioEntry {
    pub fn new(who: IoprioWho, id: u64, class: IoprioClass, data: u8, now: u64) -> Self {
        Self { who, who_id: id, class, data, set_at: now, io_ops: 0, io_bytes: 0 }
    }

    pub fn effective_priority(&self) -> u32 {
        let class_val = match self.class {
            IoprioClass::RealTime => 0,
            IoprioClass::BestEffort => 1,
            IoprioClass::Idle => 2,
            IoprioClass::None => 3,
        };
        (class_val << 8) | self.data as u32
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct IoprioBridgeStats {
    pub total_entries: u32,
    pub realtime_count: u32,
    pub best_effort_count: u32,
    pub idle_count: u32,
}

/// Main bridge ioprio
pub struct BridgeIoprio {
    entries: BTreeMap<u64, IoprioEntry>,
}

impl BridgeIoprio {
    pub fn new() -> Self { Self { entries: BTreeMap::new() } }

    pub fn set(&mut self, who: IoprioWho, id: u64, class: IoprioClass, data: u8, now: u64) {
        self.entries.insert(id, IoprioEntry::new(who, id, class, data, now));
    }

    pub fn get(&self, id: u64) -> Option<&IoprioEntry> { self.entries.get(&id) }

    pub fn record_io(&mut self, id: u64, bytes: u64) {
        if let Some(e) = self.entries.get_mut(&id) { e.io_ops += 1; e.io_bytes += bytes; }
    }

    pub fn remove(&mut self, id: u64) { self.entries.remove(&id); }

    pub fn stats(&self) -> IoprioBridgeStats {
        let rt = self.entries.values().filter(|e| e.class == IoprioClass::RealTime).count() as u32;
        let be = self.entries.values().filter(|e| e.class == IoprioClass::BestEffort).count() as u32;
        let idle = self.entries.values().filter(|e| e.class == IoprioClass::Idle).count() as u32;
        IoprioBridgeStats { total_entries: self.entries.len() as u32, realtime_count: rt, best_effort_count: be, idle_count: idle }
    }
}
