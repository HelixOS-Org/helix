// SPDX-License-Identifier: GPL-2.0
//! Apps mremap_app — memory remapping management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Mremap flags
#[derive(Debug, Clone, Copy)]
pub struct MremapFlags(pub u32);

impl MremapFlags {
    pub const MAYMOVE: u32 = 1;
    pub const FIXED: u32 = 2;
    pub const DONTUNMAP: u32 = 4;
    pub fn new() -> Self { Self(0) }
    pub fn has(&self, f: u32) -> bool { self.0 & f != 0 }
}

/// Remap operation
#[derive(Debug, Clone)]
pub struct RemapOp {
    pub pid: u64,
    pub old_addr: u64,
    pub old_size: u64,
    pub new_addr: u64,
    pub new_size: u64,
    pub flags: MremapFlags,
    pub moved: bool,
    pub timestamp: u64,
    pub duration_ns: u64,
}

impl RemapOp {
    pub fn grew(&self) -> bool { self.new_size > self.old_size }
    pub fn shrank(&self) -> bool { self.new_size < self.old_size }
    pub fn delta(&self) -> i64 { self.new_size as i64 - self.old_size as i64 }
}

/// Process remap stats
#[derive(Debug)]
pub struct ProcessRemapInfo {
    pub pid: u64,
    pub remap_count: u64,
    pub move_count: u64,
    pub grow_count: u64,
    pub shrink_count: u64,
    pub total_grown_bytes: u64,
    pub total_shrunk_bytes: u64,
}

impl ProcessRemapInfo {
    pub fn new(pid: u64) -> Self { Self { pid, remap_count: 0, move_count: 0, grow_count: 0, shrink_count: 0, total_grown_bytes: 0, total_shrunk_bytes: 0 } }

    pub fn record(&mut self, op: &RemapOp) {
        self.remap_count += 1;
        if op.moved { self.move_count += 1; }
        if op.grew() { self.grow_count += 1; self.total_grown_bytes += (op.new_size - op.old_size); }
        if op.shrank() { self.shrink_count += 1; self.total_shrunk_bytes += (op.old_size - op.new_size); }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MremapAppStats {
    pub tracked_processes: u32,
    pub total_remaps: u64,
    pub total_moves: u64,
    pub total_grows: u64,
    pub total_shrinks: u64,
}

/// Main mremap app
pub struct AppMremap {
    processes: BTreeMap<u64, ProcessRemapInfo>,
    history: Vec<RemapOp>,
    max_history: usize,
}

impl AppMremap {
    pub fn new() -> Self { Self { processes: BTreeMap::new(), history: Vec::new(), max_history: 4096 } }
    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessRemapInfo::new(pid)); }

    pub fn remap(&mut self, op: RemapOp) {
        if let Some(p) = self.processes.get_mut(&op.pid) { p.record(&op); }
        if self.history.len() >= self.max_history { self.history.drain(..self.max_history / 2); }
        self.history.push(op);
    }

    pub fn stats(&self) -> MremapAppStats {
        let remaps: u64 = self.processes.values().map(|p| p.remap_count).sum();
        let moves: u64 = self.processes.values().map(|p| p.move_count).sum();
        let grows: u64 = self.processes.values().map(|p| p.grow_count).sum();
        let shrinks: u64 = self.processes.values().map(|p| p.shrink_count).sum();
        MremapAppStats { tracked_processes: self.processes.len() as u32, total_remaps: remaps, total_moves: moves, total_grows: grows, total_shrinks: shrinks }
    }
}
