// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — RwLock (readers-writer lock with fairness control)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// RwLock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwLockState {
    Free,
    ReadLocked,
    WriteLocked,
    Upgrading,
}

/// RwLock fairness policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwLockFairness {
    ReaderPreferred,
    WriterPreferred,
    Fair,
}

/// A waiter on an rwlock
#[derive(Debug, Clone)]
pub struct RwLockWaiter {
    pub thread_id: u64,
    pub wants_write: bool,
    pub enqueue_tick: u64,
}

/// An rwlock instance
#[derive(Debug)]
pub struct RwLockInstance {
    pub id: u64,
    pub state: RwLockState,
    pub fairness: RwLockFairness,
    pub reader_count: u32,
    pub writer_id: Option<u64>,
    pub waiters: Vec<RwLockWaiter>,
    pub read_acquisitions: u64,
    pub write_acquisitions: u64,
    pub contention_count: u64,
    pub max_readers: u32,
}

impl RwLockInstance {
    pub fn new(id: u64, fairness: RwLockFairness) -> Self {
        Self {
            id, state: RwLockState::Free, fairness,
            reader_count: 0, writer_id: None,
            waiters: Vec::new(),
            read_acquisitions: 0, write_acquisitions: 0,
            contention_count: 0, max_readers: 0,
        }
    }

    pub fn try_read(&mut self, tid: u64) -> bool {
        match self.state {
            RwLockState::Free | RwLockState::ReadLocked => {
                if self.fairness == RwLockFairness::WriterPreferred && self.waiters.iter().any(|w| w.wants_write) {
                    return false;
                }
                self.state = RwLockState::ReadLocked;
                self.reader_count += 1;
                self.read_acquisitions += 1;
                if self.reader_count > self.max_readers {
                    self.max_readers = self.reader_count;
                }
                true
            }
            _ => {
                self.contention_count += 1;
                false
            }
        }
    }

    pub fn try_write(&mut self, tid: u64) -> bool {
        if self.state == RwLockState::Free {
            self.state = RwLockState::WriteLocked;
            self.writer_id = Some(tid);
            self.write_acquisitions += 1;
            true
        } else {
            self.contention_count += 1;
            false
        }
    }

    pub fn read_unlock(&mut self) {
        if self.reader_count > 0 {
            self.reader_count -= 1;
            if self.reader_count == 0 {
                self.state = RwLockState::Free;
            }
        }
    }

    pub fn write_unlock(&mut self) {
        self.state = RwLockState::Free;
        self.writer_id = None;
    }
}

/// Statistics for RwLock coop
#[derive(Debug, Clone)]
pub struct RwLockStats {
    pub locks_created: u64,
    pub read_acquisitions: u64,
    pub write_acquisitions: u64,
    pub contentions: u64,
    pub deadlocks_detected: u64,
    pub max_concurrent_readers: u32,
}

/// Main RwLock coop manager
#[derive(Debug)]
pub struct CoopRwLock {
    locks: BTreeMap<u64, RwLockInstance>,
    next_id: u64,
    stats: RwLockStats,
}

impl CoopRwLock {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            next_id: 1,
            stats: RwLockStats {
                locks_created: 0, read_acquisitions: 0,
                write_acquisitions: 0, contentions: 0,
                deadlocks_detected: 0, max_concurrent_readers: 0,
            },
        }
    }

    pub fn create_lock(&mut self, fairness: RwLockFairness) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.locks.insert(id, RwLockInstance::new(id, fairness));
        self.stats.locks_created += 1;
        id
    }

    pub fn read_lock(&mut self, lock_id: u64, tid: u64) -> bool {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            if lock.try_read(tid) {
                self.stats.read_acquisitions += 1;
                if lock.reader_count as u32 > self.stats.max_concurrent_readers {
                    self.stats.max_concurrent_readers = lock.reader_count as u32;
                }
                return true;
            }
            self.stats.contentions += 1;
        }
        false
    }

    pub fn write_lock(&mut self, lock_id: u64, tid: u64) -> bool {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            if lock.try_write(tid) {
                self.stats.write_acquisitions += 1;
                return true;
            }
            self.stats.contentions += 1;
        }
        false
    }

    pub fn read_unlock(&mut self, lock_id: u64) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.read_unlock();
        }
    }

    pub fn write_unlock(&mut self, lock_id: u64) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.write_unlock();
        }
    }

    pub fn stats(&self) -> &RwLockStats {
        &self.stats
    }
}

// ============================================================================
// Merged from rwlock_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwLockV2State {
    Unlocked,
    ReadLocked,
    WriteLocked,
    WriterWaiting,
    Poisoned,
}

/// RwLock fairness policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RwLockV2Fairness {
    ReaderPrefer,
    WriterPrefer,
    PhaseFair,
    Fifo,
}

/// A reader-writer lock V2 instance.
#[derive(Debug)]
pub struct RwLockV2Instance {
    pub lock_id: u64,
    pub state: RwLockV2State,
    pub fairness: RwLockV2Fairness,
    pub reader_count: AtomicU64,
    pub writer_pid: Option<u64>,
    pub waiting_writers: u32,
    pub waiting_readers: u32,
    pub total_read_acquires: u64,
    pub total_write_acquires: u64,
    pub max_reader_count: u64,
    pub write_hold_ticks: u64,
    pub read_hold_ticks: u64,
}

impl RwLockV2Instance {
    pub fn new(lock_id: u64, fairness: RwLockV2Fairness) -> Self {
        Self {
            lock_id,
            state: RwLockV2State::Unlocked,
            fairness,
            reader_count: AtomicU64::new(0),
            writer_pid: None,
            waiting_writers: 0,
            waiting_readers: 0,
            total_read_acquires: 0,
            total_write_acquires: 0,
            max_reader_count: 0,
            write_hold_ticks: 0,
            read_hold_ticks: 0,
        }
    }

    pub fn try_read_lock(&mut self) -> bool {
        match self.state {
            RwLockV2State::Unlocked | RwLockV2State::ReadLocked => {
                if self.fairness == RwLockV2Fairness::WriterPrefer && self.waiting_writers > 0 {
                    self.waiting_readers += 1;
                    return false;
                }
                let count = self.reader_count.fetch_add(1, Ordering::AcqRel) + 1;
                self.state = RwLockV2State::ReadLocked;
                self.total_read_acquires += 1;
                if count > self.max_reader_count {
                    self.max_reader_count = count;
                }
                true
            }
            _ => {
                self.waiting_readers += 1;
                false
            }
        }
    }

    pub fn try_write_lock(&mut self, pid: u64) -> bool {
        if self.state == RwLockV2State::Unlocked {
            self.state = RwLockV2State::WriteLocked;
            self.writer_pid = Some(pid);
            self.total_write_acquires += 1;
            true
        } else {
            self.waiting_writers += 1;
            false
        }
    }

    pub fn read_unlock(&mut self) {
        let prev = self.reader_count.fetch_sub(1, Ordering::AcqRel);
        if prev <= 1 {
            self.state = RwLockV2State::Unlocked;
            self.reader_count.store(0, Ordering::Release);
        }
    }

    pub fn write_unlock(&mut self) {
        self.state = RwLockV2State::Unlocked;
        self.writer_pid = None;
    }

    pub fn downgrade(&mut self) -> bool {
        if self.state == RwLockV2State::WriteLocked {
            self.reader_count.store(1, Ordering::Release);
            self.state = RwLockV2State::ReadLocked;
            self.writer_pid = None;
            true
        } else {
            false
        }
    }

    pub fn contention_ratio(&self) -> f64 {
        let total = self.total_read_acquires + self.total_write_acquires;
        if total == 0 {
            return 0.0;
        }
        (self.waiting_writers as u64 + self.waiting_readers as u64) as f64 / total as f64
    }
}

/// Statistics for rwlock V2.
#[derive(Debug, Clone)]
pub struct RwLockV2Stats {
    pub total_locks: u64,
    pub total_read_acquires: u64,
    pub total_write_acquires: u64,
    pub total_downgrades: u64,
    pub total_contention: u64,
    pub poisoned_count: u64,
}

/// Main coop rwlock V2 manager.
pub struct CoopRwLockV2 {
    pub locks: BTreeMap<u64, RwLockV2Instance>,
    pub next_lock_id: u64,
    pub stats: RwLockV2Stats,
}

impl CoopRwLockV2 {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            next_lock_id: 1,
            stats: RwLockV2Stats {
                total_locks: 0,
                total_read_acquires: 0,
                total_write_acquires: 0,
                total_downgrades: 0,
                total_contention: 0,
                poisoned_count: 0,
            },
        }
    }

    pub fn create_lock(&mut self, fairness: RwLockV2Fairness) -> u64 {
        let id = self.next_lock_id;
        self.next_lock_id += 1;
        let lock = RwLockV2Instance::new(id, fairness);
        self.locks.insert(id, lock);
        self.stats.total_locks += 1;
        id
    }

    pub fn lock_count(&self) -> usize {
        self.locks.len()
    }
}
