// SPDX-License-Identifier: GPL-2.0
//! App semget — System V semaphore set creation

extern crate alloc;

/// Semget result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemgetResult { Success, PermissionDenied, Exists, NoSpace, Error }

/// Semget record
#[derive(Debug, Clone)]
pub struct SemgetRecord {
    pub result: SemgetResult,
    pub key: u32,
    pub semid: i32,
    pub nsems: u32,
    pub flags: u32,
}

impl SemgetRecord {
    pub fn new(key: u32, nsems: u32) -> Self { Self { result: SemgetResult::Success, key, semid: -1, nsems, flags: 0 } }
}

/// Semget app stats
#[derive(Debug, Clone)]
pub struct SemgetAppStats { pub total_ops: u64, pub created: u64, pub errors: u64 }

/// Main app semget
#[derive(Debug)]
pub struct AppSemget { pub stats: SemgetAppStats }

impl AppSemget {
    pub fn new() -> Self { Self { stats: SemgetAppStats { total_ops: 0, created: 0, errors: 0 } } }
    pub fn record(&mut self, rec: &SemgetRecord) {
        self.stats.total_ops += 1;
        if rec.result == SemgetResult::Success { self.stats.created += 1; }
        if rec.result != SemgetResult::Success { self.stats.errors += 1; }
    }
}
