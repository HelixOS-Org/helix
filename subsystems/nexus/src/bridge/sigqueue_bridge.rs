// SPDX-License-Identifier: GPL-2.0
//! Bridge sigqueue — real-time signal queue bridge

extern crate alloc;

/// Sigqueue result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SigqueueResult {
    Queued,
    QueueFull,
    PermissionDenied,
    NoProcess,
    Error,
}

/// Sigqueue record
#[derive(Debug, Clone)]
pub struct SigqueueRecord {
    pub signal_nr: u32,
    pub result: SigqueueResult,
    pub sender_pid: u32,
    pub target_pid: u32,
    pub value: u64,
    pub queue_depth: u32,
}

impl SigqueueRecord {
    pub fn new(signal_nr: u32) -> Self {
        Self { signal_nr, result: SigqueueResult::Queued, sender_pid: 0, target_pid: 0, value: 0, queue_depth: 0 }
    }
}

/// Sigqueue bridge stats
#[derive(Debug, Clone)]
pub struct SigqueueBridgeStats {
    pub total_ops: u64,
    pub queued: u64,
    pub queue_full: u64,
    pub denied: u64,
}

/// Main bridge sigqueue
#[derive(Debug)]
pub struct BridgeSigqueue {
    pub stats: SigqueueBridgeStats,
}

impl BridgeSigqueue {
    pub fn new() -> Self {
        Self { stats: SigqueueBridgeStats { total_ops: 0, queued: 0, queue_full: 0, denied: 0 } }
    }

    pub fn record(&mut self, rec: &SigqueueRecord) {
        self.stats.total_ops += 1;
        match rec.result {
            SigqueueResult::Queued => self.stats.queued += 1,
            SigqueueResult::QueueFull => self.stats.queue_full += 1,
            SigqueueResult::PermissionDenied => self.stats.denied += 1,
            _ => {}
        }
    }
}
