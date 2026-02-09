// SPDX-License-Identifier: GPL-2.0
//! App sigwait — sigwaitinfo/sigtimedwait interface

extern crate alloc;

/// Sigwait variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigwaitVariant { Sigwaitinfo, Sigtimedwait }

/// Sigwait result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigwaitResult { Received, Timeout, Interrupted, Error }

/// Sigwait record
#[derive(Debug, Clone)]
pub struct SigwaitRecord {
    pub variant: SigwaitVariant,
    pub result: SigwaitResult,
    pub mask_bits: u64,
    pub signal_nr: u32,
    pub wait_ns: u64,
    pub pid: u32,
}

impl SigwaitRecord {
    pub fn new(variant: SigwaitVariant) -> Self {
        Self { variant, result: SigwaitResult::Received, mask_bits: 0, signal_nr: 0, wait_ns: 0, pid: 0 }
    }
}

/// Sigwait app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SigwaitAppStats { pub total_ops: u64, pub received: u64, pub timeouts: u64 }

/// Main app sigwait
#[derive(Debug)]
pub struct AppSigwait { pub stats: SigwaitAppStats }

impl AppSigwait {
    pub fn new() -> Self { Self { stats: SigwaitAppStats { total_ops: 0, received: 0, timeouts: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &SigwaitRecord) {
        self.stats.total_ops += 1;
        match rec.result {
            SigwaitResult::Received => self.stats.received += 1,
            SigwaitResult::Timeout => self.stats.timeouts += 1,
            _ => {}
        }
    }
}
