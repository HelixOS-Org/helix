// SPDX-License-Identifier: GPL-2.0
//! Coop seq_lock — sequence lock for read-heavy data.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Seqlock instance
#[derive(Debug)]
pub struct SeqLock {
    pub id: u64,
    pub sequence: u64,
    pub write_in_progress: bool,
    pub read_count: u64,
    pub write_count: u64,
    pub retry_count: u64,
    pub writer_tid: Option<u64>,
}

impl SeqLock {
    pub fn new(id: u64) -> Self {
        Self { id, sequence: 0, write_in_progress: false, read_count: 0, write_count: 0, retry_count: 0, writer_tid: None }
    }

    pub fn read_begin(&self) -> u64 { self.sequence }

    pub fn read_retry(&mut self, start_seq: u64) -> bool {
        let retry = start_seq & 1 != 0 || self.sequence != start_seq;
        if retry { self.retry_count += 1; }
        else { self.read_count += 1; }
        retry
    }

    pub fn write_begin(&mut self, tid: u64) {
        self.sequence += 1;
        self.write_in_progress = true;
        self.writer_tid = Some(tid);
    }

    pub fn write_end(&mut self) {
        self.sequence += 1;
        self.write_in_progress = false;
        self.writer_tid = None;
        self.write_count += 1;
    }

    pub fn contention_ratio(&self) -> f64 {
        let total = self.read_count + self.retry_count;
        if total == 0 { 0.0 } else { self.retry_count as f64 / total as f64 }
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct SeqLockStats {
    pub total_locks: u32,
    pub total_reads: u64,
    pub total_writes: u64,
    pub total_retries: u64,
    pub avg_contention: f64,
}

/// Main seqlock manager
pub struct CoopSeqLock {
    locks: BTreeMap<u64, SeqLock>,
    next_id: u64,
}

impl CoopSeqLock {
    pub fn new() -> Self { Self { locks: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.locks.insert(id, SeqLock::new(id));
        id
    }

    pub fn stats(&self) -> SeqLockStats {
        let reads: u64 = self.locks.values().map(|l| l.read_count).sum();
        let writes: u64 = self.locks.values().map(|l| l.write_count).sum();
        let retries: u64 = self.locks.values().map(|l| l.retry_count).sum();
        let contns: Vec<f64> = self.locks.values().map(|l| l.contention_ratio()).collect();
        let avg = if contns.is_empty() { 0.0 } else { contns.iter().sum::<f64>() / contns.len() as f64 };
        SeqLockStats { total_locks: self.locks.len() as u32, total_reads: reads, total_writes: writes, total_retries: retries, avg_contention: avg }
    }
}
