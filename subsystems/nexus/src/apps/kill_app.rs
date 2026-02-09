// SPDX-License-Identifier: GPL-2.0
//! App kill — kill/tgkill/tkill signal sending interface

extern crate alloc;

/// Kill variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillVariant { Kill, Tgkill, Tkill }

/// Kill result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillResult { Success, PermissionDenied, NoProcess, InvalidSignal, Error }

/// Kill record
#[derive(Debug, Clone)]
pub struct KillRecord {
    pub variant: KillVariant,
    pub result: KillResult,
    pub signal_nr: u32,
    pub pid: i32,
    pub tid: i32,
}

impl KillRecord {
    pub fn new(variant: KillVariant, signal_nr: u32, pid: i32) -> Self {
        Self { variant, result: KillResult::Success, signal_nr, pid, tid: 0 }
    }
}

/// Kill app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KillAppStats { pub total_ops: u64, pub success: u64, pub denied: u64, pub no_process: u64 }

/// Main app kill
#[derive(Debug)]
pub struct AppKill { pub stats: KillAppStats }

impl AppKill {
    pub fn new() -> Self { Self { stats: KillAppStats { total_ops: 0, success: 0, denied: 0, no_process: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &KillRecord) {
        self.stats.total_ops += 1;
        match rec.result {
            KillResult::Success => self.stats.success += 1,
            KillResult::PermissionDenied => self.stats.denied += 1,
            KillResult::NoProcess => self.stats.no_process += 1,
            _ => {}
        }
    }
}
