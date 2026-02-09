// SPDX-License-Identifier: GPL-2.0
//! App seccomp — seccomp filter application syscall interface

extern crate alloc;
use alloc::vec::Vec;

/// Seccomp app operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAppOp {
    SetModeStrict,
    SetModeFilter,
    GetAction,
    GetNotifSizes,
}

/// Seccomp app filter action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAppAction {
    Allow,
    Kill,
    KillProcess,
    Trap,
    Errno(u16),
    Trace(u16),
    Log,
    UserNotif,
}

/// Seccomp app result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAppResult {
    Success,
    InvalidArg,
    PermissionDenied,
    Nosys,
    Error,
}

/// Seccomp app record
#[derive(Debug, Clone)]
pub struct SeccompAppRecord {
    pub op: SeccompAppOp,
    pub result: SeccompAppResult,
    pub filter_count: u32,
    pub pid: u32,
}

impl SeccompAppRecord {
    pub fn new(op: SeccompAppOp) -> Self {
        Self {
            op,
            result: SeccompAppResult::Success,
            filter_count: 0,
            pid: 0,
        }
    }
}

/// Seccomp app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeccompAppStats {
    pub total_ops: u64,
    pub filters_set: u64,
    pub strict_mode: u64,
    pub errors: u64,
}

/// Main app seccomp
#[derive(Debug)]
pub struct AppSeccomp {
    pub stats: SeccompAppStats,
}

impl AppSeccomp {
    pub fn new() -> Self {
        Self {
            stats: SeccompAppStats {
                total_ops: 0,
                filters_set: 0,
                strict_mode: 0,
                errors: 0,
            },
        }
    }

    #[inline]
    pub fn record(&mut self, rec: &SeccompAppRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            SeccompAppOp::SetModeFilter => self.stats.filters_set += 1,
            SeccompAppOp::SetModeStrict => self.stats.strict_mode += 1,
            _ => {},
        }
        if rec.result != SeccompAppResult::Success {
            self.stats.errors += 1;
        }
    }
}
