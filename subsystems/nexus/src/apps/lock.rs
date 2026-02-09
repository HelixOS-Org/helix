//! # Application Lock Analyzer
//!
//! Lock contention and synchronization analysis:
//! - Lock usage tracking
//! - Contention hotspot detection
//! - Deadlock detection
//! - Lock ordering validation
//! - Priority inversion detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// LOCK TYPES
// ============================================================================

/// Lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LockType {
    /// Mutex
    Mutex,
    /// Read-write lock (read held)
    RwLockRead,
    /// Read-write lock (write held)
    RwLockWrite,
    /// Spinlock
    Spinlock,
    /// Semaphore
    Semaphore,
    /// Futex
    Futex,
    /// File lock
    FileLock,
}

/// Lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    /// Free
    Free,
    /// Held by one thread
    Held,
    /// Held with waiters
    HeldContended,
}

/// Lock event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockEventType {
    /// Acquire attempt
    TryAcquire,
    /// Acquired
    Acquired,
    /// Released
    Released,
    /// Contention (had to wait)
    Contended,
    /// Timeout
    Timeout,
}

// ============================================================================
// LOCK INSTANCE
// ============================================================================

/// A tracked lock instance
#[derive(Debug)]
pub struct LockInstance {
    /// Lock address/id
    pub id: u64,
    /// Lock type
    pub lock_type: LockType,
    /// Current state
    pub state: LockState,
    /// Current holder (tid)
    pub holder: Option<u64>,
    /// Waiter count
    pub waiter_count: u32,
    /// Total acquisitions
    pub total_acquisitions: u64,
    /// Total contentions
    pub total_contentions: u64,
    /// Total hold time (ns)
    pub total_hold_time_ns: u64,
    /// Total wait time (ns)
    pub total_wait_time_ns: u64,
    /// Max hold time (ns)
    pub max_hold_time_ns: u64,
    /// Last acquired at
    last_acquired: u64,
}

impl LockInstance {
    pub fn new(id: u64, lock_type: LockType) -> Self {
        Self {
            id,
            lock_type,
            state: LockState::Free,
            holder: None,
            waiter_count: 0,
            total_acquisitions: 0,
            total_contentions: 0,
            total_hold_time_ns: 0,
            total_wait_time_ns: 0,
            max_hold_time_ns: 0,
            last_acquired: 0,
        }
    }

    /// Acquire
    #[inline]
    pub fn acquire(&mut self, tid: u64, now: u64, waited_ns: u64) {
        self.state = LockState::Held;
        self.holder = Some(tid);
        self.total_acquisitions += 1;
        self.last_acquired = now;
        if waited_ns > 0 {
            self.total_contentions += 1;
            self.total_wait_time_ns += waited_ns;
        }
    }

    /// Release
    pub fn release(&mut self, now: u64) {
        let hold_time = now.saturating_sub(self.last_acquired);
        self.total_hold_time_ns += hold_time;
        if hold_time > self.max_hold_time_ns {
            self.max_hold_time_ns = hold_time;
        }
        self.holder = None;
        if self.waiter_count > 0 {
            self.state = LockState::HeldContended;
        } else {
            self.state = LockState::Free;
        }
    }

    /// Contention rate
    #[inline]
    pub fn contention_rate(&self) -> f64 {
        if self.total_acquisitions == 0 {
            return 0.0;
        }
        self.total_contentions as f64 / self.total_acquisitions as f64
    }

    /// Average hold time (ns)
    #[inline]
    pub fn avg_hold_time_ns(&self) -> f64 {
        if self.total_acquisitions == 0 {
            return 0.0;
        }
        self.total_hold_time_ns as f64 / self.total_acquisitions as f64
    }

    /// Average wait time (ns)
    #[inline]
    pub fn avg_wait_time_ns(&self) -> f64 {
        if self.total_contentions == 0 {
            return 0.0;
        }
        self.total_wait_time_ns as f64 / self.total_contentions as f64
    }

    /// Is hotspot? (high contention)
    #[inline(always)]
    pub fn is_hotspot(&self) -> bool {
        self.contention_rate() > 0.3 && self.total_acquisitions > 100
    }
}

// ============================================================================
// LOCK ORDER TRACKER
// ============================================================================

/// Lock ordering pair
#[derive(Debug, Clone)]
pub struct LockOrderPair {
    /// First lock id
    pub first: u64,
    /// Second lock id
    pub second: u64,
    /// Occurrence count
    pub count: u64,
}

/// Lock order validator
#[derive(Debug)]
pub struct LockOrderValidator {
    /// Observed orderings: (first, second) hash -> count
    orderings: BTreeMap<u64, LockOrderPair>,
    /// Per-thread held locks (tid -> ordered list of lock ids)
    held: BTreeMap<u64, Vec<u64>>,
    /// Violations detected
    pub violations: u64,
}

impl LockOrderValidator {
    pub fn new() -> Self {
        Self {
            orderings: BTreeMap::new(),
            held: BTreeMap::new(),
            violations: 0,
        }
    }

    fn pair_key(a: u64, b: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= a;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= b;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Record lock acquisition
    pub fn on_acquire(&mut self, tid: u64, lock_id: u64) {
        let held = self.held.entry(tid).or_insert_with(Vec::new);

        // Record ordering pairs with all currently held locks
        for &existing in held.iter() {
            let key = Self::pair_key(existing, lock_id);
            let reverse_key = Self::pair_key(lock_id, existing);

            if self.orderings.contains_key(&reverse_key) {
                // Potential lock order violation!
                self.violations += 1;
            }

            let entry = self.orderings.entry(key).or_insert_with(|| LockOrderPair {
                first: existing,
                second: lock_id,
                count: 0,
            });
            entry.count += 1;
        }

        held.push(lock_id);
    }

    /// Record lock release
    #[inline]
    pub fn on_release(&mut self, tid: u64, lock_id: u64) {
        if let Some(held) = self.held.get_mut(&tid) {
            held.retain(|&l| l != lock_id);
        }
    }
}

// ============================================================================
// DEADLOCK DETECTOR
// ============================================================================

/// Wait-for edge
#[derive(Debug, Clone)]
pub struct WaitForEdge {
    /// Waiting thread
    pub waiter: u64,
    /// Holder thread
    pub holder: u64,
    /// Lock id
    pub lock_id: u64,
}

/// Simple deadlock detector
#[derive(Debug)]
pub struct DeadlockDetector {
    /// Wait-for edges
    edges: Vec<WaitForEdge>,
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self {
            edges: Vec::new(),
        }
    }

    /// Add wait-for edge
    #[inline(always)]
    pub fn add_wait(&mut self, waiter: u64, holder: u64, lock_id: u64) {
        self.edges.push(WaitForEdge { waiter, holder, lock_id });
    }

    /// Remove waits for thread
    #[inline(always)]
    pub fn remove_thread(&mut self, tid: u64) {
        self.edges.retain(|e| e.waiter != tid && e.holder != tid);
    }

    /// Detect 2-thread cycle deadlocks
    pub fn detect_cycles(&self) -> Vec<(u64, u64)> {
        let mut cycles = Vec::new();
        for i in 0..self.edges.len() {
            for j in (i + 1)..self.edges.len() {
                if self.edges[i].waiter == self.edges[j].holder
                    && self.edges[i].holder == self.edges[j].waiter
                {
                    cycles.push((self.edges[i].waiter, self.edges[j].waiter));
                }
            }
        }
        cycles
    }
}

// ============================================================================
// LOCK ANALYZER ENGINE
// ============================================================================

/// Lock stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppLockStats {
    /// Tracked locks
    pub tracked_locks: usize,
    /// Contention hotspots
    pub hotspot_count: usize,
    /// Order violations
    pub order_violations: u64,
    /// Deadlocks detected
    pub deadlocks_detected: u64,
}

/// App lock analyzer
pub struct AppLockAnalyzer {
    /// Lock instances
    locks: BTreeMap<u64, LockInstance>,
    /// Order validator
    order_validator: LockOrderValidator,
    /// Deadlock detector
    deadlock_detector: DeadlockDetector,
    /// Stats
    stats: AppLockStats,
}

impl AppLockAnalyzer {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            order_validator: LockOrderValidator::new(),
            deadlock_detector: DeadlockDetector::new(),
            stats: AppLockStats::default(),
        }
    }

    /// Register lock
    #[inline(always)]
    pub fn register(&mut self, id: u64, lock_type: LockType) {
        self.locks.insert(id, LockInstance::new(id, lock_type));
        self.update_stats();
    }

    /// Record acquisition
    #[inline]
    pub fn on_acquire(&mut self, lock_id: u64, tid: u64, now: u64, waited_ns: u64) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.acquire(tid, now, waited_ns);
        }
        self.order_validator.on_acquire(tid, lock_id);
        self.deadlock_detector.remove_thread(tid);
        self.update_stats();
    }

    /// Record release
    #[inline]
    pub fn on_release(&mut self, lock_id: u64, tid: u64, now: u64) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.release(now);
        }
        self.order_validator.on_release(tid, lock_id);
        self.update_stats();
    }

    /// Record contention (waiting)
    #[inline]
    pub fn on_contend(&mut self, lock_id: u64, waiter_tid: u64) {
        if let Some(lock) = self.locks.get(&lock_id) {
            if let Some(holder) = lock.holder {
                self.deadlock_detector.add_wait(waiter_tid, holder, lock_id);
            }
        }
    }

    /// Check for deadlocks
    #[inline]
    pub fn check_deadlocks(&mut self) -> Vec<(u64, u64)> {
        let cycles = self.deadlock_detector.detect_cycles();
        self.stats.deadlocks_detected += cycles.len() as u64;
        cycles
    }

    /// Hotspot locks
    #[inline(always)]
    pub fn hotspots(&self) -> Vec<&LockInstance> {
        self.locks.values().filter(|l| l.is_hotspot()).collect()
    }

    fn update_stats(&mut self) {
        self.stats.tracked_locks = self.locks.len();
        self.stats.hotspot_count = self.locks.values().filter(|l| l.is_hotspot()).count();
        self.stats.order_violations = self.order_validator.violations;
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppLockStats {
        &self.stats
    }
}
