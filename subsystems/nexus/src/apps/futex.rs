//! # Application Futex/Synchronization Profiling
//!
//! Futex and synchronization primitive analysis:
//! - Lock contention profiling
//! - Wait chain analysis
//! - Mutex/rwlock/condvar tracking
//! - Priority inversion detection
//! - Deadlock cycle detection

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SYNC PRIMITIVE TYPES
// ============================================================================

/// Synchronization primitive type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyncPrimitiveType {
    /// Futex
    Futex,
    /// Mutex
    Mutex,
    /// Read-write lock
    RwLock,
    /// Condition variable
    CondVar,
    /// Semaphore
    Semaphore,
    /// Spinlock
    Spinlock,
    /// Barrier
    Barrier,
}

/// Lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    /// Free
    Free,
    /// Held by single owner
    Held,
    /// Read-locked (multiple readers)
    ReadLocked,
    /// Write-locked
    WriteLocked,
    /// Destroyed
    Destroyed,
}

// ============================================================================
// LOCK DESCRIPTOR
// ============================================================================

/// Lock instance descriptor
#[derive(Debug, Clone)]
pub struct LockDescriptor {
    /// Lock address
    pub address: u64,
    /// Primitive type
    pub prim_type: SyncPrimitiveType,
    /// State
    pub state: LockState,
    /// Current owner (thread ID)
    pub owner: Option<u64>,
    /// Reader count (for RwLock)
    pub reader_count: u32,
    /// Waiters
    pub waiters: Vec<u64>,
    /// Acquisition count
    pub acquisition_count: u64,
    /// Total hold time (ns)
    pub total_hold_ns: u64,
    /// Total wait time across all waiters (ns)
    pub total_wait_ns: u64,
    /// Max hold time (ns)
    pub max_hold_ns: u64,
    /// Max wait time (ns)
    pub max_wait_ns: u64,
    /// Contention count
    pub contention_count: u64,
}

impl LockDescriptor {
    pub fn new(address: u64, prim_type: SyncPrimitiveType) -> Self {
        Self {
            address,
            prim_type,
            state: LockState::Free,
            owner: None,
            reader_count: 0,
            waiters: Vec::new(),
            acquisition_count: 0,
            total_hold_ns: 0,
            total_wait_ns: 0,
            max_hold_ns: 0,
            max_wait_ns: 0,
            contention_count: 0,
        }
    }

    /// Acquire lock
    pub fn acquire(&mut self, thread: u64, wait_ns: u64) {
        self.owner = Some(thread);
        self.state = LockState::Held;
        self.acquisition_count += 1;
        self.total_wait_ns += wait_ns;
        if wait_ns > self.max_wait_ns {
            self.max_wait_ns = wait_ns;
        }
        if wait_ns > 0 {
            self.contention_count += 1;
        }
        self.waiters.retain(|&t| t != thread);
    }

    /// Release lock
    #[inline]
    pub fn release(&mut self, hold_ns: u64) {
        self.owner = None;
        self.state = LockState::Free;
        self.total_hold_ns += hold_ns;
        if hold_ns > self.max_hold_ns {
            self.max_hold_ns = hold_ns;
        }
    }

    /// Add waiter
    #[inline]
    pub fn add_waiter(&mut self, thread: u64) {
        if !self.waiters.contains(&thread) {
            self.waiters.push(thread);
        }
    }

    /// Average hold time
    #[inline]
    pub fn avg_hold_ns(&self) -> f64 {
        if self.acquisition_count == 0 {
            return 0.0;
        }
        self.total_hold_ns as f64 / self.acquisition_count as f64
    }

    /// Average wait time
    #[inline]
    pub fn avg_wait_ns(&self) -> f64 {
        if self.contention_count == 0 {
            return 0.0;
        }
        self.total_wait_ns as f64 / self.contention_count as f64
    }

    /// Contention rate
    #[inline]
    pub fn contention_rate(&self) -> f64 {
        if self.acquisition_count == 0 {
            return 0.0;
        }
        self.contention_count as f64 / self.acquisition_count as f64
    }
}

// ============================================================================
// WAIT CHAIN
// ============================================================================

/// Wait chain entry
#[derive(Debug, Clone)]
pub struct WaitChainEntry {
    /// Waiting thread
    pub thread: u64,
    /// Waiting on lock address
    pub lock_address: u64,
    /// Lock owner
    pub owner: Option<u64>,
    /// Wait time so far (ns)
    pub wait_ns: u64,
}

/// Wait chain (potentially circular = deadlock)
#[derive(Debug, Clone)]
pub struct WaitChain {
    /// Chain entries
    pub entries: Vec<WaitChainEntry>,
    /// Is cyclic (deadlock)
    pub is_deadlock: bool,
    /// Chain length
    pub length: usize,
}

impl WaitChain {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            is_deadlock: false,
            length: 0,
        }
    }

    /// Add entry
    #[inline(always)]
    pub fn push(&mut self, entry: WaitChainEntry) {
        self.entries.push(entry);
        self.length = self.entries.len();
    }

    /// Check if this chain forms a deadlock
    #[inline]
    pub fn check_cycle(&mut self) -> bool {
        if self.entries.len() < 2 {
            self.is_deadlock = false;
            return false;
        }
        let first = self.entries[0].thread;
        let last_owner = self.entries.last().and_then(|e| e.owner);
        self.is_deadlock = last_owner == Some(first);
        self.is_deadlock
    }
}

// ============================================================================
// PRIORITY INVERSION
// ============================================================================

/// Priority inversion event
#[derive(Debug, Clone)]
pub struct PriorityInversion {
    /// High priority thread (blocked)
    pub high_prio_thread: u64,
    /// High priority level
    pub high_prio: u32,
    /// Low priority thread (holding lock)
    pub low_prio_thread: u64,
    /// Low priority level
    pub low_prio: u32,
    /// Lock address
    pub lock_address: u64,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Medium priority threads that preempted the low-priority holder
    pub interfering_threads: Vec<u64>,
}

// ============================================================================
// PROCESS SYNC PROFILE
// ============================================================================

/// Per-process synchronization profile
#[derive(Debug, Clone)]
pub struct ProcessSyncProfile {
    /// Process ID
    pub pid: u64,
    /// Locks owned or tracked
    pub locks: BTreeMap<u64, LockDescriptor>,
    /// Thread to lock held mapping
    pub thread_locks: BTreeMap<u64, Vec<u64>>,
    /// Thread to lock waiting mapping
    pub thread_waiting: LinearMap<u64, 64>,
    /// Priority inversions detected
    pub inversions: Vec<PriorityInversion>,
    /// Total lock time (ns)
    pub total_lock_time_ns: u64,
    /// Total wait time (ns)
    pub total_wait_time_ns: u64,
}

impl ProcessSyncProfile {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            locks: BTreeMap::new(),
            thread_locks: BTreeMap::new(),
            thread_waiting: LinearMap::new(),
            inversions: Vec::new(),
            total_lock_time_ns: 0,
            total_wait_time_ns: 0,
        }
    }

    /// Register lock
    #[inline]
    pub fn register_lock(&mut self, address: u64, prim_type: SyncPrimitiveType) {
        self.locks
            .entry(address)
            .or_insert_with(|| LockDescriptor::new(address, prim_type));
    }

    /// Acquire lock
    #[inline]
    pub fn acquire(&mut self, thread: u64, address: u64, wait_ns: u64) {
        if let Some(lock) = self.locks.get_mut(&address) {
            lock.acquire(thread, wait_ns);
        }
        self.thread_locks
            .entry(thread)
            .or_insert_with(Vec::new)
            .push(address);
        self.thread_waiting.remove(thread);
        self.total_wait_time_ns += wait_ns;
    }

    /// Release lock
    #[inline]
    pub fn release(&mut self, thread: u64, address: u64, hold_ns: u64) {
        if let Some(lock) = self.locks.get_mut(&address) {
            lock.release(hold_ns);
        }
        if let Some(held) = self.thread_locks.get_mut(&thread) {
            held.retain(|&a| a != address);
        }
        self.total_lock_time_ns += hold_ns;
    }

    /// Thread starts waiting
    #[inline]
    pub fn start_wait(&mut self, thread: u64, address: u64) {
        self.thread_waiting.insert(thread, address);
        if let Some(lock) = self.locks.get_mut(&address) {
            lock.add_waiter(thread);
        }
    }

    /// Build wait chains
    pub fn build_wait_chains(&self) -> Vec<WaitChain> {
        let mut chains = Vec::new();

        for (&thread, &lock_addr) in &self.thread_waiting {
            let mut chain = WaitChain::new();
            let mut visited = Vec::new();
            let mut current_thread = thread;
            let mut current_lock = lock_addr;

            loop {
                if visited.contains(&current_thread) {
                    chain.check_cycle();
                    break;
                }
                visited.push(current_thread);

                let owner = self.locks.get(&current_lock).and_then(|l| l.owner);

                chain.push(WaitChainEntry {
                    thread: current_thread,
                    lock_address: current_lock,
                    owner,
                    wait_ns: 0,
                });

                match owner {
                    Some(o) => {
                        if let Some(&next_lock) = self.thread_waiting.get(o) {
                            current_thread = o;
                            current_lock = next_lock;
                        } else {
                            break;
                        }
                    },
                    None => break,
                }
            }

            if chain.length >= 2 {
                chains.push(chain);
            }
        }

        chains
    }

    /// Most contended locks
    #[inline]
    pub fn most_contended(&self, limit: usize) -> Vec<&LockDescriptor> {
        let mut locks: Vec<_> = self.locks.values().collect();
        locks.sort_by(|a, b| b.contention_count.cmp(&a.contention_count));
        locks.truncate(limit);
        locks
    }
}

// ============================================================================
// FUTEX ANALYZER
// ============================================================================

/// Futex analyzer stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppFutexStats {
    /// Tracked processes
    pub process_count: usize,
    /// Total locks tracked
    pub total_locks: usize,
    /// Total contentions
    pub total_contentions: u64,
    /// Deadlocks detected
    pub deadlocks_detected: u64,
    /// Priority inversions
    pub inversions_detected: u64,
}

/// Application futex/sync analyzer
pub struct AppFutexAnalyzer {
    /// Per-process profiles
    profiles: BTreeMap<u64, ProcessSyncProfile>,
    /// Stats
    stats: AppFutexStats,
}

impl AppFutexAnalyzer {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            stats: AppFutexStats::default(),
        }
    }

    /// Register process
    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.profiles.insert(pid, ProcessSyncProfile::new(pid));
        self.stats.process_count = self.profiles.len();
    }

    /// Register lock
    #[inline]
    pub fn register_lock(&mut self, pid: u64, address: u64, prim_type: SyncPrimitiveType) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.register_lock(address, prim_type);
            self.stats.total_locks = self.profiles.values().map(|p| p.locks.len()).sum();
        }
    }

    /// Record acquire
    #[inline]
    pub fn record_acquire(&mut self, pid: u64, thread: u64, address: u64, wait_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.acquire(thread, address, wait_ns);
            if wait_ns > 0 {
                self.stats.total_contentions += 1;
            }
        }
    }

    /// Record release
    #[inline]
    pub fn record_release(&mut self, pid: u64, thread: u64, address: u64, hold_ns: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.release(thread, address, hold_ns);
        }
    }

    /// Record wait start
    #[inline]
    pub fn record_wait(&mut self, pid: u64, thread: u64, address: u64) {
        if let Some(profile) = self.profiles.get_mut(&pid) {
            profile.start_wait(thread, address);
        }
    }

    /// Detect deadlocks for a process
    #[inline]
    pub fn detect_deadlocks(&mut self, pid: u64) -> Vec<WaitChain> {
        let chains = match self.profiles.get(&pid) {
            Some(profile) => profile.build_wait_chains(),
            None => return Vec::new(),
        };
        let deadlocks: Vec<_> = chains.into_iter().filter(|c| c.is_deadlock).collect();
        self.stats.deadlocks_detected += deadlocks.len() as u64;
        deadlocks
    }

    /// Get profile
    #[inline(always)]
    pub fn profile(&self, pid: u64) -> Option<&ProcessSyncProfile> {
        self.profiles.get(&pid)
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppFutexStats {
        &self.stats
    }
}

// ============================================================================
// Merged from futex_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexOp {
    /// Wait
    Wait,
    /// Wake
    Wake,
    /// Requeue
    Requeue,
    /// Wait bitset
    WaitBitset,
    /// Wake bitset
    WakeBitset,
    /// Lock PI
    LockPi,
    /// Unlock PI
    UnlockPi,
    /// Trylock PI
    TrylockPi,
}

/// Wait result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitResult {
    /// Woken normally
    Woken,
    /// Timed out
    Timeout,
    /// Interrupted by signal
    Signal,
    /// Requeued
    Requeued,
    /// Value mismatch (spurious)
    Spurious,
}

/// Contention level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentionLevel {
    /// No contention
    None,
    /// Light (< 5 waiters)
    Light,
    /// Moderate (5-20 waiters)
    Moderate,
    /// Heavy (> 20 waiters)
    Heavy,
}

// ============================================================================
// FUTEX ADDRESS
// ============================================================================

/// Tracked futex address
#[derive(Debug, Clone)]
pub struct FutexAddress {
    /// Virtual address
    pub addr: u64,
    /// Hash bucket
    pub bucket: u32,
    /// Wait count
    pub wait_count: u64,
    /// Wake count
    pub wake_count: u64,
    /// Timeout count
    pub timeout_count: u64,
    /// Current waiters
    pub current_waiters: u32,
    /// Max waiters observed
    pub max_waiters: u32,
    /// Total wait time (ns)
    pub total_wait_ns: u64,
    /// Is PI futex
    pub is_pi: bool,
    /// Requeue count
    pub requeue_count: u64,
}

impl FutexAddress {
    pub fn new(addr: u64) -> Self {
        Self {
            addr,
            bucket: (addr as u32) & 0xFF,
            wait_count: 0,
            wake_count: 0,
            timeout_count: 0,
            current_waiters: 0,
            max_waiters: 0,
            total_wait_ns: 0,
            is_pi: false,
            requeue_count: 0,
        }
    }

    /// Record wait start
    #[inline]
    pub fn record_wait(&mut self) {
        self.wait_count += 1;
        self.current_waiters += 1;
        if self.current_waiters > self.max_waiters {
            self.max_waiters = self.current_waiters;
        }
    }

    /// Record wake
    pub fn record_wake(&mut self, result: WaitResult, wait_ns: u64) {
        self.wake_count += 1;
        if self.current_waiters > 0 {
            self.current_waiters -= 1;
        }
        self.total_wait_ns += wait_ns;
        match result {
            WaitResult::Timeout => self.timeout_count += 1,
            WaitResult::Requeued => self.requeue_count += 1,
            _ => {},
        }
    }

    /// Average wait time (ns)
    #[inline]
    pub fn avg_wait_ns(&self) -> u64 {
        if self.wake_count == 0 {
            return 0;
        }
        self.total_wait_ns / self.wake_count
    }

    /// Contention level
    #[inline]
    pub fn contention(&self) -> ContentionLevel {
        match self.current_waiters {
            0 => ContentionLevel::None,
            1..=4 => ContentionLevel::Light,
            5..=20 => ContentionLevel::Moderate,
            _ => ContentionLevel::Heavy,
        }
    }

    /// Timeout rate
    #[inline]
    pub fn timeout_rate(&self) -> f64 {
        if self.wait_count == 0 {
            return 0.0;
        }
        self.timeout_count as f64 / self.wait_count as f64
    }
}

// ============================================================================
// WAIT CHAIN
// ============================================================================

/// Wait chain detector
#[derive(Debug)]
pub struct WaitChainDetector {
    /// Active waits: tid -> chain entry
    active_waits: BTreeMap<u64, WaitChainEntry>,
    /// Deadlocks detected
    pub deadlocks_detected: u64,
    /// PI inversions detected
    pub pi_inversions: u64,
}

impl WaitChainDetector {
    pub fn new() -> Self {
        Self {
            active_waits: BTreeMap::new(),
            deadlocks_detected: 0,
            pi_inversions: 0,
        }
    }

    /// Record wait
    #[inline(always)]
    pub fn record_wait(&mut self, entry: WaitChainEntry) {
        self.active_waits.insert(entry.waiter_tid, entry);
    }

    /// Record wake
    #[inline(always)]
    pub fn record_wake(&mut self, tid: u64) {
        self.active_waits.remove(&tid);
    }

    /// Detect cycle (potential deadlock)
    pub fn detect_cycle(&self, start_tid: u64) -> Option<Vec<u64>> {
        let mut visited = Vec::new();
        let mut current = start_tid;
        loop {
            if visited.contains(&current) {
                return Some(visited);
            }
            visited.push(current);
            // Find what 'current' is waiting on
            let entry = match self.active_waits.get(&current) {
                Some(e) => e,
                None => return None,
            };
            // Find if owner is also waiting
            let owner_tid = entry.owner_pid; // simplified: using pid as tid
            if !self.active_waits.contains_key(&owner_tid) {
                return None;
            }
            current = owner_tid;
        }
    }

    /// Check for priority inversion
    #[inline(always)]
    pub fn check_pi(&self, waiter_priority: u8, owner_priority: u8) -> bool {
        waiter_priority > owner_priority
    }

    /// Active wait count
    #[inline(always)]
    pub fn active_count(&self) -> usize {
        self.active_waits.len()
    }
}

// ============================================================================
// HASH TABLE PROFILING
// ============================================================================

/// Futex hash bucket stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BucketStats {
    /// Bucket index
    pub index: u32,
    /// Current entries
    pub entries: u32,
    /// Total lookups
    pub lookups: u64,
    /// Collisions
    pub collisions: u64,
}

/// Hash table profiler
#[derive(Debug)]
pub struct FutexHashProfiler {
    /// Bucket stats
    buckets: BTreeMap<u32, BucketStats>,
    /// Total buckets
    total_buckets: u32,
}

impl FutexHashProfiler {
    pub fn new(total_buckets: u32) -> Self {
        Self {
            buckets: BTreeMap::new(),
            total_buckets,
        }
    }

    /// Record bucket access
    #[inline]
    pub fn record_access(&mut self, bucket: u32, had_collision: bool) {
        let stats = self.buckets.entry(bucket).or_insert_with(|| BucketStats {
            index: bucket,
            ..Default::default()
        });
        stats.lookups += 1;
        if had_collision {
            stats.collisions += 1;
        }
    }

    /// Collision rate
    #[inline]
    pub fn collision_rate(&self) -> f64 {
        let total_lookups: u64 = self.buckets.values().map(|b| b.lookups).sum();
        let total_collisions: u64 = self.buckets.values().map(|b| b.collisions).sum();
        if total_lookups == 0 {
            return 0.0;
        }
        total_collisions as f64 / total_lookups as f64
    }

    /// Hot buckets (above average load)
    pub fn hot_buckets(&self) -> Vec<u32> {
        let avg = if self.buckets.is_empty() {
            0
        } else {
            self.buckets.values().map(|b| b.lookups).sum::<u64>() / self.buckets.len() as u64
        };
        self.buckets
            .values()
            .filter(|b| b.lookups > avg * 2)
            .map(|b| b.index)
            .collect()
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Futex profiler stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AppFutexV2Stats {
    /// Tracked futex addresses
    pub tracked_addresses: usize,
    /// Currently contended
    pub contended_count: usize,
    /// Heavy contention count
    pub heavy_contention: usize,
    /// Active wait chains
    pub active_chains: usize,
    /// Total deadlocks detected
    pub deadlocks: u64,
}

/// App futex profiler v2
pub struct AppFutexV2Profiler {
    /// Tracked addresses
    addresses: BTreeMap<u64, FutexAddress>,
    /// Wait chain detector
    wait_chains: WaitChainDetector,
    /// Hash profiler
    hash_profiler: FutexHashProfiler,
    /// Stats
    stats: AppFutexV2Stats,
}

impl AppFutexV2Profiler {
    pub fn new() -> Self {
        Self {
            addresses: BTreeMap::new(),
            wait_chains: WaitChainDetector::new(),
            hash_profiler: FutexHashProfiler::new(256),
            stats: AppFutexV2Stats::default(),
        }
    }

    /// Record futex wait
    pub fn record_wait(&mut self, addr: u64, pid: u64, tid: u64, now: u64) {
        let futex = self
            .addresses
            .entry(addr)
            .or_insert_with(|| FutexAddress::new(addr));
        futex.record_wait();
        self.hash_profiler
            .record_access(futex.bucket, futex.current_waiters > 1);
        self.wait_chains.record_wait(WaitChainEntry {
            waiter_pid: pid,
            waiter_tid: tid,
            blocked_on: addr,
            owner_pid: 0,
            wait_start_ns: now,
        });
        self.update_stats();
    }

    /// Record futex wake
    #[inline]
    pub fn record_wake(&mut self, addr: u64, tid: u64, result: WaitResult, wait_ns: u64) {
        if let Some(futex) = self.addresses.get_mut(&addr) {
            futex.record_wake(result, wait_ns);
        }
        self.wait_chains.record_wake(tid);
        self.update_stats();
    }

    /// Get contention level
    #[inline]
    pub fn contention(&self, addr: u64) -> ContentionLevel {
        self.addresses
            .get(&addr)
            .map(|f| f.contention())
            .unwrap_or(ContentionLevel::None)
    }

    /// Check deadlocks for TID
    #[inline]
    pub fn check_deadlock(&mut self, tid: u64) -> Option<Vec<u64>> {
        let cycle = self.wait_chains.detect_cycle(tid);
        if cycle.is_some() {
            self.wait_chains.deadlocks_detected += 1;
            self.stats.deadlocks = self.wait_chains.deadlocks_detected;
        }
        cycle
    }

    fn update_stats(&mut self) {
        self.stats.tracked_addresses = self.addresses.len();
        self.stats.contended_count = self
            .addresses
            .values()
            .filter(|a| a.current_waiters > 0)
            .count();
        self.stats.heavy_contention = self
            .addresses
            .values()
            .filter(|a| a.contention() == ContentionLevel::Heavy)
            .count();
        self.stats.active_chains = self.wait_chains.active_count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &AppFutexV2Stats {
        &self.stats
    }
}

// ============================================================================
// Merged from futex_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Futex2Op {
    Wait,
    Wake,
    Requeue,
    CmpRequeue,
    WakeOp,
    LockPi,
    UnlockPi,
    TrylockPi,
    WaitBitset,
    WakeBitset,
    WaitV,
}

/// Futex2 size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Futex2Size {
    U8,
    U16,
    U32,
    U64,
}

/// Futex2 waiter
#[derive(Debug, Clone)]
pub struct Futex2Waiter {
    pub tid: u64,
    pub uaddr: u64,
    pub val: u64,
    pub bitset: u32,
    pub size: Futex2Size,
    pub priority: i32,
    pub enqueued_at: u64,
    pub is_pi: bool,
    pub timeout_ns: Option<u64>,
}

impl Futex2Waiter {
    pub fn new(tid: u64, uaddr: u64, val: u64, size: Futex2Size, now: u64) -> Self {
        Self {
            tid, uaddr, val, bitset: u32::MAX, size, priority: 0,
            enqueued_at: now, is_pi: false, timeout_ns: None,
        }
    }

    #[inline(always)]
    pub fn wait_time(&self, now: u64) -> u64 { now.saturating_sub(self.enqueued_at) }
    #[inline(always)]
    pub fn matches_bitset(&self, mask: u32) -> bool { self.bitset & mask != 0 }
}

/// Futex2 hash bucket
#[derive(Debug)]
pub struct Futex2Bucket {
    pub key: u64,
    pub waiters: Vec<Futex2Waiter>,
    pub pi_owner: Option<u64>,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_contended: u64,
}

impl Futex2Bucket {
    pub fn new(key: u64) -> Self {
        Self { key, waiters: Vec::new(), pi_owner: None, total_waits: 0, total_wakes: 0, total_contended: 0 }
    }

    #[inline]
    pub fn enqueue(&mut self, waiter: Futex2Waiter) {
        self.total_waits += 1;
        if !self.waiters.is_empty() { self.total_contended += 1; }
        self.waiters.push(waiter);
    }

    #[inline]
    pub fn wake_one(&mut self, bitset: u32) -> Option<u64> {
        let pos = self.waiters.iter().position(|w| w.matches_bitset(bitset));
        if let Some(idx) = pos {
            let w = self.waiters.remove(idx);
            self.total_wakes += 1;
            Some(w.tid)
        } else { None }
    }

    #[inline]
    pub fn wake_n(&mut self, n: u32, bitset: u32) -> Vec<u64> {
        let mut woken = Vec::new();
        for _ in 0..n {
            if let Some(tid) = self.wake_one(bitset) { woken.push(tid); }
            else { break; }
        }
        woken
    }

    #[inline]
    pub fn requeue_to(&mut self, dst: &mut Futex2Bucket, n: u32) -> u32 {
        let take = (n as usize).min(self.waiters.len());
        let moved: Vec<Futex2Waiter> = self.waiters.drain(..take).collect();
        let count = moved.len() as u32;
        for w in moved { dst.waiters.push(w); }
        count
    }

    #[inline(always)]
    pub fn contention_rate(&self) -> f64 {
        if self.total_waits == 0 { return 0.0; }
        self.total_contended as f64 / self.total_waits as f64
    }
}

/// WaitV descriptor (multiple futex wait)
#[derive(Debug, Clone)]
pub struct Futex2WaitV {
    pub entries: Vec<(u64, u64, Futex2Size, u32)>, // uaddr, val, size, flags
}

impl Futex2WaitV {
    pub fn new() -> Self { Self { entries: Vec::new() } }
    #[inline(always)]
    pub fn add(&mut self, uaddr: u64, val: u64, size: Futex2Size, flags: u32) {
        self.entries.push((uaddr, val, size, flags));
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct Futex3Stats {
    pub total_buckets: u32,
    pub total_waiters: u32,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_contended: u64,
    pub pi_buckets: u32,
    pub avg_contention: f64,
}

/// Main futex v3 manager
pub struct AppFutexV3 {
    buckets: BTreeMap<u64, Futex2Bucket>,
}

impl AppFutexV3 {
    pub fn new() -> Self { Self { buckets: BTreeMap::new() } }

    fn hash_key(uaddr: u64) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let bytes = uaddr.to_le_bytes();
        for b in &bytes { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        h
    }

    #[inline]
    pub fn wait(&mut self, uaddr: u64, tid: u64, val: u64, size: Futex2Size, now: u64) {
        let key = Self::hash_key(uaddr);
        let bucket = self.buckets.entry(key).or_insert_with(|| Futex2Bucket::new(key));
        bucket.enqueue(Futex2Waiter::new(tid, uaddr, val, size, now));
    }

    #[inline(always)]
    pub fn wake(&mut self, uaddr: u64, n: u32, bitset: u32) -> Vec<u64> {
        let key = Self::hash_key(uaddr);
        self.buckets.get_mut(&key).map(|b| b.wake_n(n, bitset)).unwrap_or_default()
    }

    pub fn stats(&self) -> Futex3Stats {
        let waiters: u32 = self.buckets.values().map(|b| b.waiters.len() as u32).sum();
        let waits: u64 = self.buckets.values().map(|b| b.total_waits).sum();
        let wakes: u64 = self.buckets.values().map(|b| b.total_wakes).sum();
        let contended: u64 = self.buckets.values().map(|b| b.total_contended).sum();
        let pi = self.buckets.values().filter(|b| b.pi_owner.is_some()).count() as u32;
        let rates: Vec<f64> = self.buckets.values().filter(|b| b.total_waits > 0).map(|b| b.contention_rate()).collect();
        let avg = if rates.is_empty() { 0.0 } else { rates.iter().sum::<f64>() / rates.len() as f64 };
        Futex3Stats {
            total_buckets: self.buckets.len() as u32, total_waiters: waiters,
            total_waits: waits, total_wakes: wakes, total_contended: contended,
            pi_buckets: pi, avg_contention: avg,
        }
    }
}
