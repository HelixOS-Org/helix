//! # Cognitive Synchronization
//!
//! Synchronization primitives for cognitive domains.
//! Provides barriers, locks, and coordination mechanisms.

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// BARRIER
// ============================================================================

/// A synchronization barrier
pub struct CognitiveBarrier {
    /// Barrier ID
    id: u64,
    /// Name
    name: String,
    /// Expected participants
    expected: usize,
    /// Arrived participants
    arrived: Vec<DomainId>,
    /// Generation (for reuse)
    generation: AtomicU64,
    /// Is open
    open: AtomicBool,
    /// Creation time
    created: Timestamp,
    /// Last sync time
    last_sync: Timestamp,
}

impl CognitiveBarrier {
    /// Create a new barrier
    pub fn new(id: u64, name: &str, expected: usize) -> Self {
        let now = Timestamp::now();
        Self {
            id,
            name: name.into(),
            expected,
            arrived: Vec::with_capacity(expected),
            generation: AtomicU64::new(0),
            open: AtomicBool::new(false),
            created: now,
            last_sync: now,
        }
    }

    /// Arrive at barrier
    pub fn arrive(&mut self, domain: DomainId) -> BarrierResult {
        // Already arrived in this generation?
        if self.arrived.contains(&domain) {
            return BarrierResult::AlreadyArrived;
        }

        self.arrived.push(domain);

        if self.arrived.len() >= self.expected {
            // All arrived - open barrier
            self.open.store(true, Ordering::Release);
            self.last_sync = Timestamp::now();
            BarrierResult::AllArrived
        } else {
            BarrierResult::Waiting(self.expected - self.arrived.len())
        }
    }

    /// Check if barrier is open
    #[inline(always)]
    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }

    /// Get arrived count
    #[inline(always)]
    pub fn arrived_count(&self) -> usize {
        self.arrived.len()
    }

    /// Get expected count
    #[inline(always)]
    pub fn expected(&self) -> usize {
        self.expected
    }

    /// Get who is missing
    #[inline]
    pub fn missing(&self, all: &[DomainId]) -> Vec<DomainId> {
        all.iter()
            .filter(|d| !self.arrived.contains(d))
            .copied()
            .collect()
    }

    /// Reset for next generation
    #[inline]
    pub fn reset(&mut self) {
        self.arrived.clear();
        self.open.store(false, Ordering::Release);
        self.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current generation
    #[inline(always)]
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }

    /// Get barrier ID
    #[inline(always)]
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get barrier name
    #[inline(always)]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Barrier arrival result
#[derive(Debug, Clone, Copy)]
pub enum BarrierResult {
    /// Waiting for others
    Waiting(usize),
    /// All arrived
    AllArrived,
    /// Already arrived this generation
    AlreadyArrived,
}

// ============================================================================
// SEMAPHORE
// ============================================================================

/// A counting semaphore
pub struct CognitiveSemaphore {
    /// Semaphore ID
    id: u64,
    /// Name
    name: String,
    /// Maximum count
    max_count: u32,
    /// Current count
    count: AtomicU64,
    /// Waiters
    waiters: VecDeque<DomainId>,
    /// Holders
    holders: Vec<(DomainId, u32)>,
}

impl CognitiveSemaphore {
    /// Create a new semaphore
    pub fn new(id: u64, name: &str, count: u32) -> Self {
        Self {
            id,
            name: name.into(),
            max_count: count,
            count: AtomicU64::new(count as u64),
            waiters: VecDeque::new(),
            holders: Vec::new(),
        }
    }

    /// Try to acquire
    pub fn try_acquire(&mut self, domain: DomainId, count: u32) -> bool {
        let current = self.count.load(Ordering::Acquire);
        if current >= count as u64 {
            self.count.fetch_sub(count as u64, Ordering::Release);

            // Track holder
            if let Some((_, held)) = self.holders.iter_mut().find(|(d, _)| *d == domain) {
                *held += count;
            } else {
                self.holders.push((domain, count));
            }

            true
        } else {
            false
        }
    }

    /// Acquire (blocking simulation - adds to waiters)
    pub fn acquire(&mut self, domain: DomainId, count: u32) -> bool {
        if self.try_acquire(domain, count) {
            return true;
        }

        // Add to waiters
        if !self.waiters.contains(&domain) {
            self.waiters.push_back(domain);
        }

        false
    }

    /// Release
    pub fn release(&mut self, domain: DomainId, count: u32) -> bool {
        // Check holder
        if let Some(pos) = self.holders.iter().position(|(d, _)| *d == domain) {
            let (_, held) = &mut self.holders[pos];
            if *held >= count {
                *held -= count;
                if *held == 0 {
                    self.holders.remove(pos);
                }

                let new_count =
                    self.count.fetch_add(count as u64, Ordering::Release) + count as u64;

                // Wake waiters if possible
                if new_count > 0 && !self.waiters.is_empty() {
                    // Signal first waiter
                    self.waiters.pop_front();
                }

                return true;
            }
        }

        false
    }

    /// Get available count
    #[inline(always)]
    pub fn available(&self) -> u32 {
        self.count.load(Ordering::Acquire) as u32
    }

    /// Get waiter count
    #[inline(always)]
    pub fn waiter_count(&self) -> usize {
        self.waiters.len()
    }

    /// Get holders
    #[inline(always)]
    pub fn holders(&self) -> &[(DomainId, u32)] {
        &self.holders
    }

    /// Get ID
    #[inline(always)]
    pub fn id(&self) -> u64 {
        self.id
    }
}

// ============================================================================
// READ-WRITE LOCK
// ============================================================================

/// A read-write lock
pub struct CognitiveRwLock {
    /// Lock ID
    id: u64,
    /// Name
    name: String,
    /// Readers
    readers: Vec<DomainId>,
    /// Writer
    writer: Option<DomainId>,
    /// Read waiters
    read_waiters: VecDeque<DomainId>,
    /// Write waiters
    write_waiters: VecDeque<DomainId>,
    /// Prefer writers
    prefer_writers: bool,
}

impl CognitiveRwLock {
    /// Create a new lock
    pub fn new(id: u64, name: &str, prefer_writers: bool) -> Self {
        Self {
            id,
            name: name.into(),
            readers: Vec::new(),
            writer: None,
            read_waiters: VecDeque::new(),
            write_waiters: VecDeque::new(),
            prefer_writers,
        }
    }

    /// Try to acquire read lock
    pub fn try_read(&mut self, domain: DomainId) -> bool {
        // Can't read if there's a writer
        if self.writer.is_some() {
            return false;
        }

        // If preferring writers, can't read if writers waiting
        if self.prefer_writers && !self.write_waiters.is_empty() {
            return false;
        }

        // Already a reader?
        if !self.readers.contains(&domain) {
            self.readers.push(domain);
        }

        true
    }

    /// Try to acquire write lock
    #[inline]
    pub fn try_write(&mut self, domain: DomainId) -> bool {
        // Can't write if there's a writer or readers
        if self.writer.is_some() || !self.readers.is_empty() {
            return false;
        }

        self.writer = Some(domain);
        true
    }

    /// Acquire read (adds to waiters if can't acquire)
    #[inline]
    pub fn read(&mut self, domain: DomainId) -> bool {
        if self.try_read(domain) {
            return true;
        }

        if !self.read_waiters.contains(&domain) {
            self.read_waiters.push(domain);
        }

        false
    }

    /// Acquire write (adds to waiters if can't acquire)
    #[inline]
    pub fn write(&mut self, domain: DomainId) -> bool {
        if self.try_write(domain) {
            return true;
        }

        if !self.write_waiters.contains(&domain) {
            self.write_waiters.push_back(domain);
        }

        false
    }

    /// Release read lock
    pub fn release_read(&mut self, domain: DomainId) -> bool {
        if let Some(pos) = self.readers.iter().position(|&d| d == domain) {
            self.readers.remove(pos);

            // Wake a writer if no more readers
            if self.readers.is_empty() && !self.write_waiters.is_empty() {
                let waiter = self.write_waiters.pop_front().unwrap();
                self.writer = Some(waiter);
            }

            return true;
        }

        false
    }

    /// Release write lock
    pub fn release_write(&mut self, domain: DomainId) -> bool {
        if self.writer == Some(domain) {
            self.writer = None;

            // Wake writers first (if prefer_writers) or readers
            if self.prefer_writers && !self.write_waiters.is_empty() {
                let waiter = self.write_waiters.pop_front().unwrap();
                self.writer = Some(waiter);
            } else if !self.read_waiters.is_empty() {
                // Wake all readers
                self.readers.append(&mut self.read_waiters);
            } else if !self.write_waiters.is_empty() {
                let waiter = self.write_waiters.pop_front().unwrap();
                self.writer = Some(waiter);
            }

            return true;
        }

        false
    }

    /// Check if locked for reading
    #[inline(always)]
    pub fn is_read_locked(&self) -> bool {
        !self.readers.is_empty()
    }

    /// Check if locked for writing
    #[inline(always)]
    pub fn is_write_locked(&self) -> bool {
        self.writer.is_some()
    }

    /// Get reader count
    #[inline(always)]
    pub fn reader_count(&self) -> usize {
        self.readers.len()
    }

    /// Get ID
    #[inline(always)]
    pub fn id(&self) -> u64 {
        self.id
    }
}

// ============================================================================
// CONDITION VARIABLE
// ============================================================================

/// A condition variable
pub struct CognitiveCondVar {
    /// CondVar ID
    id: u64,
    /// Name
    name: String,
    /// Waiters
    waiters: VecDeque<DomainId>,
    /// Signal count
    signals: u64,
}

impl CognitiveCondVar {
    /// Create a new condition variable
    pub fn new(id: u64, name: &str) -> Self {
        Self {
            id,
            name: name.into(),
            waiters: VecDeque::new(),
            signals: 0,
        }
    }

    /// Wait on condition
    #[inline]
    pub fn wait(&mut self, domain: DomainId) {
        if !self.waiters.contains(&domain) {
            self.waiters.push_back(domain);
        }
    }

    /// Signal one waiter
    #[inline]
    pub fn signal(&mut self) -> Option<DomainId> {
        self.signals += 1;
        if !self.waiters.is_empty() {
            self.waiters.pop_front()
        } else {
            None
        }
    }

    /// Signal all waiters
    #[inline(always)]
    pub fn broadcast(&mut self) -> Vec<DomainId> {
        self.signals += 1;
        core::mem::take(&mut self.waiters)
    }

    /// Get waiter count
    #[inline(always)]
    pub fn waiter_count(&self) -> usize {
        self.waiters.len()
    }

    /// Get signal count
    #[inline(always)]
    pub fn signal_count(&self) -> u64 {
        self.signals
    }

    /// Get ID
    #[inline(always)]
    pub fn id(&self) -> u64 {
        self.id
    }
}

// ============================================================================
// SYNC MANAGER
// ============================================================================

/// Manages synchronization primitives
pub struct SyncManager {
    /// Barriers
    barriers: BTreeMap<u64, CognitiveBarrier>,
    /// Semaphores
    semaphores: BTreeMap<u64, CognitiveSemaphore>,
    /// RwLocks
    rwlocks: BTreeMap<u64, CognitiveRwLock>,
    /// CondVars
    condvars: BTreeMap<u64, CognitiveCondVar>,
    /// Next ID
    next_id: AtomicU64,
    /// Statistics
    stats: SyncStats,
}

/// Sync statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct SyncStats {
    /// Total barriers created
    pub barriers_created: u64,
    /// Total barrier syncs
    pub barrier_syncs: u64,
    /// Total semaphore acquires
    pub semaphore_acquires: u64,
    /// Total lock acquires
    pub lock_acquires: u64,
    /// Total signals
    pub signals: u64,
}

impl SyncManager {
    /// Create a new sync manager
    pub fn new() -> Self {
        Self {
            barriers: BTreeMap::new(),
            semaphores: BTreeMap::new(),
            rwlocks: BTreeMap::new(),
            condvars: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            stats: SyncStats::default(),
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    // === Barriers ===

    /// Create a barrier
    #[inline]
    pub fn create_barrier(&mut self, name: &str, expected: usize) -> u64 {
        let id = self.next_id();
        let barrier = CognitiveBarrier::new(id, name, expected);
        self.barriers.insert(id, barrier);
        self.stats.barriers_created += 1;
        id
    }

    /// Arrive at barrier
    #[inline]
    pub fn barrier_arrive(&mut self, barrier_id: u64, domain: DomainId) -> Option<BarrierResult> {
        let barrier = self.barriers.get_mut(&barrier_id)?;
        let result = barrier.arrive(domain);
        if matches!(result, BarrierResult::AllArrived) {
            self.stats.barrier_syncs += 1;
        }
        Some(result)
    }

    /// Reset barrier
    #[inline]
    pub fn barrier_reset(&mut self, barrier_id: u64) {
        if let Some(barrier) = self.barriers.get_mut(&barrier_id) {
            barrier.reset();
        }
    }

    /// Get barrier
    #[inline(always)]
    pub fn get_barrier(&self, barrier_id: u64) -> Option<&CognitiveBarrier> {
        self.barriers.get(&barrier_id)
    }

    // === Semaphores ===

    /// Create a semaphore
    #[inline]
    pub fn create_semaphore(&mut self, name: &str, count: u32) -> u64 {
        let id = self.next_id();
        let sem = CognitiveSemaphore::new(id, name, count);
        self.semaphores.insert(id, sem);
        id
    }

    /// Try acquire semaphore
    #[inline]
    pub fn semaphore_try_acquire(&mut self, sem_id: u64, domain: DomainId, count: u32) -> bool {
        if let Some(sem) = self.semaphores.get_mut(&sem_id) {
            if sem.try_acquire(domain, count) {
                self.stats.semaphore_acquires += 1;
                return true;
            }
        }
        false
    }

    /// Release semaphore
    #[inline]
    pub fn semaphore_release(&mut self, sem_id: u64, domain: DomainId, count: u32) -> bool {
        self.semaphores
            .get_mut(&sem_id)
            .map(|s| s.release(domain, count))
            .unwrap_or(false)
    }

    /// Get semaphore
    #[inline(always)]
    pub fn get_semaphore(&self, sem_id: u64) -> Option<&CognitiveSemaphore> {
        self.semaphores.get(&sem_id)
    }

    // === RwLocks ===

    /// Create a rwlock
    #[inline]
    pub fn create_rwlock(&mut self, name: &str, prefer_writers: bool) -> u64 {
        let id = self.next_id();
        let lock = CognitiveRwLock::new(id, name, prefer_writers);
        self.rwlocks.insert(id, lock);
        id
    }

    /// Try read lock
    #[inline]
    pub fn rwlock_try_read(&mut self, lock_id: u64, domain: DomainId) -> bool {
        if let Some(lock) = self.rwlocks.get_mut(&lock_id) {
            if lock.try_read(domain) {
                self.stats.lock_acquires += 1;
                return true;
            }
        }
        false
    }

    /// Try write lock
    #[inline]
    pub fn rwlock_try_write(&mut self, lock_id: u64, domain: DomainId) -> bool {
        if let Some(lock) = self.rwlocks.get_mut(&lock_id) {
            if lock.try_write(domain) {
                self.stats.lock_acquires += 1;
                return true;
            }
        }
        false
    }

    /// Release read lock
    #[inline]
    pub fn rwlock_release_read(&mut self, lock_id: u64, domain: DomainId) -> bool {
        self.rwlocks
            .get_mut(&lock_id)
            .map(|l| l.release_read(domain))
            .unwrap_or(false)
    }

    /// Release write lock
    #[inline]
    pub fn rwlock_release_write(&mut self, lock_id: u64, domain: DomainId) -> bool {
        self.rwlocks
            .get_mut(&lock_id)
            .map(|l| l.release_write(domain))
            .unwrap_or(false)
    }

    /// Get rwlock
    #[inline(always)]
    pub fn get_rwlock(&self, lock_id: u64) -> Option<&CognitiveRwLock> {
        self.rwlocks.get(&lock_id)
    }

    // === CondVars ===

    /// Create a condvar
    #[inline]
    pub fn create_condvar(&mut self, name: &str) -> u64 {
        let id = self.next_id();
        let cv = CognitiveCondVar::new(id, name);
        self.condvars.insert(id, cv);
        id
    }

    /// Wait on condvar
    #[inline]
    pub fn condvar_wait(&mut self, cv_id: u64, domain: DomainId) {
        if let Some(cv) = self.condvars.get_mut(&cv_id) {
            cv.wait(domain);
        }
    }

    /// Signal condvar
    #[inline]
    pub fn condvar_signal(&mut self, cv_id: u64) -> Option<DomainId> {
        if let Some(cv) = self.condvars.get_mut(&cv_id) {
            self.stats.signals += 1;
            return cv.signal();
        }
        None
    }

    /// Broadcast condvar
    #[inline]
    pub fn condvar_broadcast(&mut self, cv_id: u64) -> Vec<DomainId> {
        if let Some(cv) = self.condvars.get_mut(&cv_id) {
            self.stats.signals += 1;
            return cv.broadcast();
        }
        Vec::new()
    }

    /// Get condvar
    #[inline(always)]
    pub fn get_condvar(&self, cv_id: u64) -> Option<&CognitiveCondVar> {
        self.condvars.get(&cv_id)
    }

    // === Cleanup ===

    /// Delete barrier
    #[inline(always)]
    pub fn delete_barrier(&mut self, id: u64) -> bool {
        self.barriers.remove(&id).is_some()
    }

    /// Delete semaphore
    #[inline(always)]
    pub fn delete_semaphore(&mut self, id: u64) -> bool {
        self.semaphores.remove(&id).is_some()
    }

    /// Delete rwlock
    #[inline(always)]
    pub fn delete_rwlock(&mut self, id: u64) -> bool {
        self.rwlocks.remove(&id).is_some()
    }

    /// Delete condvar
    #[inline(always)]
    pub fn delete_condvar(&mut self, id: u64) -> bool {
        self.condvars.remove(&id).is_some()
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &SyncStats {
        &self.stats
    }
}

impl Default for SyncManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barrier() {
        let mut barrier = CognitiveBarrier::new(1, "test", 3);

        let d1 = DomainId::new(1);
        let d2 = DomainId::new(2);
        let d3 = DomainId::new(3);

        assert!(matches!(barrier.arrive(d1), BarrierResult::Waiting(2)));
        assert!(matches!(barrier.arrive(d2), BarrierResult::Waiting(1)));
        assert!(matches!(barrier.arrive(d3), BarrierResult::AllArrived));
        assert!(barrier.is_open());

        // Reset
        barrier.reset();
        assert!(!barrier.is_open());
        assert_eq!(barrier.generation(), 1);
    }

    #[test]
    fn test_semaphore() {
        let mut sem = CognitiveSemaphore::new(1, "test", 2);

        let d1 = DomainId::new(1);
        let d2 = DomainId::new(2);
        let d3 = DomainId::new(3);

        assert!(sem.try_acquire(d1, 1));
        assert!(sem.try_acquire(d2, 1));
        assert!(!sem.try_acquire(d3, 1)); // No capacity

        assert!(sem.release(d1, 1));
        assert!(sem.try_acquire(d3, 1)); // Now has capacity
    }

    #[test]
    fn test_rwlock() {
        let mut lock = CognitiveRwLock::new(1, "test", false);

        let d1 = DomainId::new(1);
        let d2 = DomainId::new(2);
        let d3 = DomainId::new(3);

        // Multiple readers
        assert!(lock.try_read(d1));
        assert!(lock.try_read(d2));
        assert_eq!(lock.reader_count(), 2);

        // Writer blocked
        assert!(!lock.try_write(d3));

        // Release readers
        assert!(lock.release_read(d1));
        assert!(lock.release_read(d2));

        // Writer can now acquire
        assert!(lock.try_write(d3));
        assert!(lock.is_write_locked());
    }

    #[test]
    fn test_condvar() {
        let mut cv = CognitiveCondVar::new(1, "test");

        let d1 = DomainId::new(1);
        let d2 = DomainId::new(2);

        cv.wait(d1);
        cv.wait(d2);
        assert_eq!(cv.waiter_count(), 2);

        let woken = cv.signal();
        assert_eq!(woken, Some(d1));
        assert_eq!(cv.waiter_count(), 1);

        let woken = cv.broadcast();
        assert_eq!(woken, vec![d2]);
        assert_eq!(cv.waiter_count(), 0);
    }
}
