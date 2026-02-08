// SPDX-License-Identifier: GPL-2.0
//! Bridge semaphore — System V semaphore bridge

extern crate alloc;

/// Semaphore operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreOp {
    Semget,
    Semop,
    Semtimedop,
    Semctl,
}

/// Semctl command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemctlCmd {
    Getval,
    Setval,
    Getall,
    Setall,
    IpcStat,
    IpcSet,
    IpcRmid,
    GetPid,
    GetNcnt,
    GetZcnt,
}

/// Semaphore result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemaphoreResult {
    Success,
    WouldBlock,
    Interrupted,
    PermissionDenied,
    InvalidId,
    Error,
}

/// Semaphore record
#[derive(Debug, Clone)]
pub struct SemaphoreRecord {
    pub op: SemaphoreOp,
    pub result: SemaphoreResult,
    pub semid: i32,
    pub nsems: u32,
    pub sem_num: u32,
    pub key: u32,
}

impl SemaphoreRecord {
    pub fn new(op: SemaphoreOp) -> Self {
        Self { op, result: SemaphoreResult::Success, semid: -1, nsems: 0, sem_num: 0, key: 0 }
    }
}

/// Semaphore bridge stats
#[derive(Debug, Clone)]
pub struct SemaphoreBridgeStats {
    pub total_ops: u64,
    pub semops: u64,
    pub sets_created: u64,
    pub blocks: u64,
    pub errors: u64,
}

/// Main bridge semaphore
#[derive(Debug)]
pub struct BridgeSemaphore {
    pub stats: SemaphoreBridgeStats,
}

impl BridgeSemaphore {
    pub fn new() -> Self {
        Self { stats: SemaphoreBridgeStats { total_ops: 0, semops: 0, sets_created: 0, blocks: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &SemaphoreRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            SemaphoreOp::Semop | SemaphoreOp::Semtimedop => self.stats.semops += 1,
            SemaphoreOp::Semget => self.stats.sets_created += 1,
            _ => {}
        }
        if rec.result == SemaphoreResult::WouldBlock { self.stats.blocks += 1; }
        if rec.result == SemaphoreResult::Error { self.stats.errors += 1; }
    }
}
