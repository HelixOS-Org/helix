// SPDX-License-Identifier: GPL-2.0
//! App shmget — System V shared memory segment creation

extern crate alloc;

/// Shmget result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmgetResult { Success, PermissionDenied, Exists, NoMemory, Error }

/// Shmget record
#[derive(Debug, Clone)]
pub struct ShmgetRecord {
    pub result: ShmgetResult,
    pub key: u32,
    pub shmid: i32,
    pub size: u64,
    pub flags: u32,
}

impl ShmgetRecord {
    pub fn new(key: u32, size: u64) -> Self { Self { result: ShmgetResult::Success, key, shmid: -1, size, flags: 0 } }
}

/// Shmget app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ShmgetAppStats { pub total_ops: u64, pub created: u64, pub total_bytes: u64, pub errors: u64 }

/// Main app shmget
#[derive(Debug)]
pub struct AppShmget { pub stats: ShmgetAppStats }

impl AppShmget {
    pub fn new() -> Self { Self { stats: ShmgetAppStats { total_ops: 0, created: 0, total_bytes: 0, errors: 0 } } }
    #[inline]
    pub fn record(&mut self, rec: &ShmgetRecord) {
        self.stats.total_ops += 1;
        if rec.result == ShmgetResult::Success { self.stats.created += 1; self.stats.total_bytes += rec.size; }
        if rec.result != ShmgetResult::Success { self.stats.errors += 1; }
    }
}
