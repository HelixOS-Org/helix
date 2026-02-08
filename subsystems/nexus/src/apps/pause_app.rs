// SPDX-License-Identifier: GPL-2.0
//! App pause — pause syscall waiting for signal

extern crate alloc;

/// Pause result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseResult { Interrupted, Error }

/// Pause record
#[derive(Debug, Clone)]
pub struct PauseRecord {
    pub result: PauseResult,
    pub signal_nr: u32,
    pub wait_ns: u64,
    pub pid: u32,
}

impl PauseRecord {
    pub fn new() -> Self { Self { result: PauseResult::Interrupted, signal_nr: 0, wait_ns: 0, pid: 0 } }
}

/// Pause app stats
#[derive(Debug, Clone)]
pub struct PauseAppStats { pub total_ops: u64, pub total_wait_ns: u64, pub avg_wait_ns: f64 }

/// Main app pause
#[derive(Debug)]
pub struct AppPause { pub stats: PauseAppStats }

impl AppPause {
    pub fn new() -> Self { Self { stats: PauseAppStats { total_ops: 0, total_wait_ns: 0, avg_wait_ns: 0.0 } } }
    pub fn record(&mut self, rec: &PauseRecord) {
        self.stats.total_ops += 1;
        self.stats.total_wait_ns += rec.wait_ns;
        self.stats.avg_wait_ns = self.stats.total_wait_ns as f64 / self.stats.total_ops as f64;
    }
}
