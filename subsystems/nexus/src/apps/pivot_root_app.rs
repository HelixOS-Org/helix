// SPDX-License-Identifier: GPL-2.0
//! Apps pivot_root_app — pivot_root syscall support.

extern crate alloc;

use alloc::vec::Vec;

/// Pivot root state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PivotState {
    Pending,
    Completed,
    Failed,
    RolledBack,
}

/// Pivot root operation
#[derive(Debug)]
pub struct PivotRootOp {
    pub id: u64,
    pub pid: u64,
    pub new_root_hash: u64,
    pub put_old_hash: u64,
    pub state: PivotState,
    pub timestamp: u64,
    pub duration_ns: u64,
}

impl PivotRootOp {
    pub fn new(id: u64, pid: u64, new_root: u64, put_old: u64, now: u64) -> Self {
        Self { id, pid, new_root_hash: new_root, put_old_hash: put_old, state: PivotState::Pending, timestamp: now, duration_ns: 0 }
    }

    pub fn complete(&mut self, dur: u64) { self.state = PivotState::Completed; self.duration_ns = dur; }
    pub fn fail(&mut self) { self.state = PivotState::Failed; }
}

/// Stats
#[derive(Debug, Clone)]
pub struct PivotRootAppStats {
    pub total_ops: u32,
    pub completed: u32,
    pub failed: u32,
    pub avg_duration_ns: u64,
}

/// Main pivot root app
pub struct AppPivotRoot {
    ops: Vec<PivotRootOp>,
    next_id: u64,
}

impl AppPivotRoot {
    pub fn new() -> Self { Self { ops: Vec::new(), next_id: 1 } }

    pub fn pivot_root(&mut self, pid: u64, new_root: u64, put_old: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.ops.push(PivotRootOp::new(id, pid, new_root, put_old, now));
        id
    }

    pub fn stats(&self) -> PivotRootAppStats {
        let completed = self.ops.iter().filter(|o| o.state == PivotState::Completed).count() as u32;
        let failed = self.ops.iter().filter(|o| o.state == PivotState::Failed).count() as u32;
        let durs: Vec<u64> = self.ops.iter().filter(|o| o.state == PivotState::Completed).map(|o| o.duration_ns).collect();
        let avg = if durs.is_empty() { 0 } else { durs.iter().sum::<u64>() / durs.len() as u64 };
        PivotRootAppStats { total_ops: self.ops.len() as u32, completed, failed, avg_duration_ns: avg }
    }
}
