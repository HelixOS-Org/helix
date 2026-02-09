// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Sequence Lock (seqlock for lock-free readers)

extern crate alloc;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

/// Seqlock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqlockState {
    Idle,
    Writing,
    Inconsistent,
}

/// A sequence lock instance
#[derive(Debug)]
pub struct SeqlockInstance {
    pub id: u64,
    pub sequence: AtomicU64,
    pub read_retries: u64,
    pub write_count: u64,
    pub read_count: u64,
    pub max_retries: u64,
}

impl SeqlockInstance {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            sequence: AtomicU64::new(0),
            read_retries: 0,
            write_count: 0,
            read_count: 0,
            max_retries: 0,
        }
    }

    #[inline(always)]
    pub fn write_begin(&self) -> u64 {
        let seq = self.sequence.fetch_add(1, Ordering::Release);
        seq + 1
    }

    #[inline(always)]
    pub fn write_end(&self) {
        self.sequence.fetch_add(1, Ordering::Release);
    }

    #[inline]
    pub fn read_begin(&self) -> u64 {
        loop {
            let seq = self.sequence.load(Ordering::Acquire);
            if seq & 1 == 0 {
                return seq;
            }
            core::hint::spin_loop();
        }
    }

    #[inline(always)]
    pub fn read_retry(&self, start_seq: u64) -> bool {
        let current = self.sequence.load(Ordering::Acquire);
        current != start_seq
    }

    #[inline(always)]
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::Acquire)
    }
}

/// Statistics for seqlock coop
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct SeqlockStats {
    pub locks_created: u64,
    pub total_writes: u64,
    pub total_reads: u64,
    pub total_retries: u64,
    pub max_retries_single: u64,
}

/// Main seqlock coop manager
#[derive(Debug)]
pub struct CoopSeqlock {
    locks: BTreeMap<u64, SeqlockInstance>,
    next_id: u64,
    stats: SeqlockStats,
}

impl CoopSeqlock {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            next_id: 1,
            stats: SeqlockStats {
                locks_created: 0,
                total_writes: 0,
                total_reads: 0,
                total_retries: 0,
                max_retries_single: 0,
            },
        }
    }

    #[inline]
    pub fn create(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.locks.insert(id, SeqlockInstance::new(id));
        self.stats.locks_created += 1;
        id
    }

    #[inline]
    pub fn write_begin(&mut self, lock_id: u64) -> Option<u64> {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            let seq = lock.write_begin();
            lock.write_count += 1;
            self.stats.total_writes += 1;
            Some(seq)
        } else {
            None
        }
    }

    #[inline]
    pub fn write_end(&mut self, lock_id: u64) {
        if let Some(lock) = self.locks.get(&lock_id) {
            lock.write_end();
        }
    }

    #[inline]
    pub fn read_begin(&mut self, lock_id: u64) -> Option<u64> {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            let seq = lock.read_begin();
            lock.read_count += 1;
            self.stats.total_reads += 1;
            Some(seq)
        } else {
            None
        }
    }

    #[inline]
    pub fn read_retry(&mut self, lock_id: u64, start: u64) -> bool {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            if lock.read_retry(start) {
                lock.read_retries += 1;
                self.stats.total_retries += 1;
                return true;
            }
        }
        false
    }

    #[inline(always)]
    pub fn stats(&self) -> &SeqlockStats {
        &self.stats
    }
}
