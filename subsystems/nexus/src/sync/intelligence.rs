//! Synchronization Intelligence
//!
//! Central coordinator for all sync analysis components.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    AcquireMode, ContentionAnalyzer, DeadlockDetector, DeadlockInfo, LockId, LockInfo,
    LockOrderOptimizer, RwLockOptimizer, SpinlockAnalyzer, ThreadId, WaitTimePredictor,
};

/// Central synchronization intelligence coordinator
pub struct SyncIntelligence {
    /// Registered locks
    locks: BTreeMap<LockId, LockInfo>,
    /// Contention analyzer
    contention: ContentionAnalyzer,
    /// Deadlock detector
    deadlock: DeadlockDetector,
    /// Wait time predictor
    wait_time: WaitTimePredictor,
    /// Lock order optimizer
    order: LockOrderOptimizer,
    /// Spinlock analyzer
    spinlock: SpinlockAnalyzer,
    /// RwLock optimizer
    rwlock: RwLockOptimizer,
    /// Total lock operations
    total_ops: AtomicU64,
}

impl SyncIntelligence {
    /// Create new sync intelligence
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            contention: ContentionAnalyzer::default(),
            deadlock: DeadlockDetector::default(),
            wait_time: WaitTimePredictor::default(),
            order: LockOrderOptimizer::default(),
            spinlock: SpinlockAnalyzer::default(),
            rwlock: RwLockOptimizer::default(),
            total_ops: AtomicU64::new(0),
        }
    }

    /// Register lock
    pub fn register(&mut self, info: LockInfo) {
        self.locks.insert(info.id, info);
    }

    /// Record lock acquired
    pub fn lock_acquired(
        &mut self,
        lock_id: LockId,
        thread: ThreadId,
        wait_ns: u64,
        held_locks: &[LockId],
    ) {
        self.total_ops.fetch_add(1, Ordering::Relaxed);

        // Update lock info
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            let holder = lock.holder;
            lock.acquire(thread, AcquireMode::Exclusive);

            // Contention analysis
            self.contention.record(lock_id, thread, holder, wait_ns);

            // Wait time prediction
            self.wait_time
                .record(lock_id, wait_ns, lock.waiters.len() as u32);
        }

        // Deadlock detection
        self.deadlock.lock_acquired(thread, lock_id);

        // Lock order
        self.order.record_order(lock_id, held_locks);
    }

    /// Record lock released
    pub fn lock_released(&mut self, lock_id: LockId, thread: ThreadId) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.release(thread);
        }

        self.deadlock.lock_released(thread, lock_id);
    }

    /// Check for potential deadlock
    pub fn check_deadlock(&mut self, thread: ThreadId, lock_id: LockId) -> Option<DeadlockInfo> {
        self.deadlock.waiting_for(thread, lock_id)
    }

    /// Get lock info
    pub fn get_lock(&self, lock_id: LockId) -> Option<&LockInfo> {
        self.locks.get(&lock_id)
    }

    /// Get contention analyzer
    pub fn contention(&self) -> &ContentionAnalyzer {
        &self.contention
    }

    /// Get deadlock detector
    pub fn deadlock(&self) -> &DeadlockDetector {
        &self.deadlock
    }

    /// Get wait time predictor
    pub fn wait_time(&self) -> &WaitTimePredictor {
        &self.wait_time
    }

    /// Get lock order optimizer
    pub fn order(&self) -> &LockOrderOptimizer {
        &self.order
    }

    /// Get mutable order optimizer
    pub fn order_mut(&mut self) -> &mut LockOrderOptimizer {
        &mut self.order
    }

    /// Get spinlock analyzer
    pub fn spinlock(&self) -> &SpinlockAnalyzer {
        &self.spinlock
    }

    /// Get mutable spinlock analyzer
    pub fn spinlock_mut(&mut self) -> &mut SpinlockAnalyzer {
        &mut self.spinlock
    }

    /// Get rwlock optimizer
    pub fn rwlock(&self) -> &RwLockOptimizer {
        &self.rwlock
    }

    /// Get mutable rwlock optimizer
    pub fn rwlock_mut(&mut self) -> &mut RwLockOptimizer {
        &mut self.rwlock
    }

    /// Get total operations
    pub fn total_ops(&self) -> u64 {
        self.total_ops.load(Ordering::Relaxed)
    }

    /// Predict wait time
    pub fn predict_wait(&self, lock_id: LockId) -> f64 {
        let waiters = self
            .locks
            .get(&lock_id)
            .map(|l| l.waiters.len())
            .unwrap_or(0);
        self.wait_time.predict(lock_id, waiters as u32)
    }
}

impl Default for SyncIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
