//! # App Futex Profiler v2
//!
//! Enhanced futex contention analysis:
//! - Wait chain detection
//! - Priority inversion tracking
//! - Futex hash table profiling
//! - Timeout analysis
//! - Requeue pattern detection

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// FUTEX TYPES
// ============================================================================

/// Futex operation
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
    pub fn avg_wait_ns(&self) -> u64 {
        if self.wake_count == 0 {
            return 0;
        }
        self.total_wait_ns / self.wake_count
    }

    /// Contention level
    pub fn contention(&self) -> ContentionLevel {
        match self.current_waiters {
            0 => ContentionLevel::None,
            1..=4 => ContentionLevel::Light,
            5..=20 => ContentionLevel::Moderate,
            _ => ContentionLevel::Heavy,
        }
    }

    /// Timeout rate
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

/// Wait chain entry
#[derive(Debug, Clone)]
pub struct WaitChainEntry {
    /// Waiter PID
    pub waiter_pid: u64,
    /// Waiter TID
    pub waiter_tid: u64,
    /// Blocked on futex address
    pub blocked_on: u64,
    /// Owner PID (of the lock)
    pub owner_pid: u64,
    /// Wait start
    pub wait_start_ns: u64,
}

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
    pub fn record_wait(&mut self, entry: WaitChainEntry) {
        self.active_waits.insert(entry.waiter_tid, entry);
    }

    /// Record wake
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
    pub fn check_pi(&self, waiter_priority: u8, owner_priority: u8) -> bool {
        waiter_priority > owner_priority
    }

    /// Active wait count
    pub fn active_count(&self) -> usize {
        self.active_waits.len()
    }
}

// ============================================================================
// HASH TABLE PROFILING
// ============================================================================

/// Futex hash bucket stats
#[derive(Debug, Clone, Default)]
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
    pub fn record_wake(&mut self, addr: u64, tid: u64, result: WaitResult, wait_ns: u64) {
        if let Some(futex) = self.addresses.get_mut(&addr) {
            futex.record_wake(result, wait_ns);
        }
        self.wait_chains.record_wake(tid);
        self.update_stats();
    }

    /// Get contention level
    pub fn contention(&self, addr: u64) -> ContentionLevel {
        self.addresses
            .get(&addr)
            .map(|f| f.contention())
            .unwrap_or(ContentionLevel::None)
    }

    /// Check deadlocks for TID
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
    pub fn stats(&self) -> &AppFutexV2Stats {
        &self.stats
    }
}
