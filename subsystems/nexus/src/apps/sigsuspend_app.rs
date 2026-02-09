// SPDX-License-Identifier: GPL-2.0
//! App sigsuspend — sigsuspend atomic mask+wait

extern crate alloc;

/// Sigsuspend result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigsuspendResult { Interrupted, Error }

/// Sigsuspend record
#[derive(Debug, Clone)]
pub struct SigsuspendRecord {
    pub result: SigsuspendResult,
    pub mask_bits: u64,
    pub signal_nr: u32,
    pub wait_ns: u64,
    pub pid: u32,
}

impl SigsuspendRecord {
    pub fn new(mask: u64) -> Self { Self { result: SigsuspendResult::Interrupted, mask_bits: mask, signal_nr: 0, wait_ns: 0, pid: 0 } }
}

/// Sigsuspend app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SigsuspendAppStats { pub total_ops: u64, pub total_wait_ns: u64 }

/// Main app sigsuspend
#[derive(Debug)]
pub struct AppSigsuspend { pub stats: SigsuspendAppStats }

impl AppSigsuspend {
    pub fn new() -> Self { Self { stats: SigsuspendAppStats { total_ops: 0, total_wait_ns: 0 } } }
    #[inline(always)]
    pub fn record(&mut self, rec: &SigsuspendRecord) {
        self.stats.total_ops += 1;
        self.stats.total_wait_ns += rec.wait_ns;
    }
}
