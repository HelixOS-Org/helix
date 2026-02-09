// SPDX-License-Identifier: GPL-2.0
//! App msgget — System V message queue creation

extern crate alloc;

/// Msgget result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsggetResult { Success, PermissionDenied, Exists, NoSpace, Error }

/// Msgget record
#[derive(Debug, Clone)]
pub struct MsggetRecord {
    pub result: MsggetResult,
    pub key: u32,
    pub msqid: i32,
    pub flags: u32,
}

impl MsggetRecord {
    pub fn new(key: u32) -> Self { Self { result: MsggetResult::Success, key, msqid: -1, flags: 0 } }
}

/// Msgget app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MsggetAppStats { pub total_ops: u64, pub created: u64, pub errors: u64 }

/// Main app msgget
#[derive(Debug)]
pub struct AppMsgget { pub stats: MsggetAppStats }

impl AppMsgget {
    pub fn new() -> Self { Self { stats: MsggetAppStats { total_ops: 0, created: 0, errors: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &MsggetRecord) {
        self.stats.total_ops += 1;
        if rec.result == MsggetResult::Success { self.stats.created += 1; }
        if rec.result != MsggetResult::Success { self.stats.errors += 1; }
    }
}
