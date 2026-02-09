//! # Holistic Futex Tracker
//!
//! Fast userspace mutex tracking and optimization:
//! - Futex hash bucket management
//! - Contention monitoring per futex address
//! - Priority inheritance chain tracking
//! - Waiter queue statistics
//! - Timeout and requeue operations
//! - Deadlock detection for futex chains

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Futex operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexOp {
    Wait,
    Wake,
    Requeue,
    CmpRequeue,
    WaitBitset,
    WakeBitset,
    LockPi,
    UnlockPi,
    TrylockPi,
    WaitRequeuePi,
}

/// Futex waiter state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaiterState {
    Waiting,
    Woken,
    TimedOut,
    Requeued,
    Interrupted,
}

/// Futex waiter
#[derive(Debug, Clone)]
pub struct FutexWaiter {
    pub task_id: u64,
    pub state: WaiterState,
    pub enqueue_ts: u64,
    pub wake_ts: u64,
    pub bitset: u32,
    pub priority: i32,
    pub pi_boosted: bool,
    pub timeout_ns: u64,
}

impl FutexWaiter {
    pub fn new(task_id: u64, ts: u64, bitset: u32) -> Self {
        Self {
            task_id, state: WaiterState::Waiting, enqueue_ts: ts,
            wake_ts: 0, bitset, priority: 0, pi_boosted: false,
            timeout_ns: u64::MAX,
        }
    }

    #[inline(always)]
    pub fn wake(&mut self, ts: u64) {
        self.state = WaiterState::Woken;
        self.wake_ts = ts;
    }

    #[inline(always)]
    pub fn timeout(&mut self, ts: u64) {
        self.state = WaiterState::TimedOut;
        self.wake_ts = ts;
    }

    #[inline(always)]
    pub fn wait_time_ns(&self) -> u64 {
        if self.wake_ts > self.enqueue_ts { self.wake_ts - self.enqueue_ts } else { 0 }
    }
}

/// Futex hash bucket
#[derive(Debug, Clone)]
pub struct FutexBucket {
    pub addr: u64,
    pub hash: u64,
    pub waiters: Vec<FutexWaiter>,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_requeues: u64,
    pub total_timeouts: u64,
    pub contention_count: u64,
    pub max_waiters_seen: u32,
    pub owner_task: Option<u64>,
    pub pi_enabled: bool,
}

impl FutexBucket {
    pub fn new(addr: u64) -> Self {
        let hash = Self::hash_addr(addr);
        Self {
            addr, hash, waiters: Vec::new(),
            total_waits: 0, total_wakes: 0, total_requeues: 0,
            total_timeouts: 0, contention_count: 0, max_waiters_seen: 0,
            owner_task: None, pi_enabled: false,
        }
    }

    fn hash_addr(addr: u64) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let bytes = addr.to_le_bytes();
        for &b in &bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    #[inline]
    pub fn add_waiter(&mut self, waiter: FutexWaiter) {
        self.total_waits += 1;
        self.waiters.push(waiter);
        let cur = self.waiters.len() as u32;
        if cur > self.max_waiters_seen { self.max_waiters_seen = cur; }
        if cur > 1 { self.contention_count += 1; }
    }

    #[inline]
    pub fn wake_one(&mut self, ts: u64, bitset: u32) -> Option<u64> {
        for w in &mut self.waiters {
            if w.state == WaiterState::Waiting && (w.bitset & bitset) != 0 {
                w.wake(ts);
                self.total_wakes += 1;
                return Some(w.task_id);
            }
        }
        None
    }

    pub fn wake_n(&mut self, n: u32, ts: u64, bitset: u32) -> u32 {
        let mut woken = 0u32;
        for w in &mut self.waiters {
            if woken >= n { break; }
            if w.state == WaiterState::Waiting && (w.bitset & bitset) != 0 {
                w.wake(ts);
                woken += 1;
            }
        }
        self.total_wakes += woken as u64;
        woken
    }

    #[inline]
    pub fn process_timeouts(&mut self, ts: u64) -> u32 {
        let mut timed_out = 0u32;
        for w in &mut self.waiters {
            if w.state == WaiterState::Waiting && ts >= w.enqueue_ts + w.timeout_ns {
                w.timeout(ts);
                timed_out += 1;
            }
        }
        self.total_timeouts += timed_out as u64;
        timed_out
    }

    #[inline(always)]
    pub fn cleanup_completed(&mut self) {
        self.waiters.retain(|w| w.state == WaiterState::Waiting);
    }

    #[inline(always)]
    pub fn active_waiters(&self) -> usize {
        self.waiters.iter().filter(|w| w.state == WaiterState::Waiting).count()
    }

    #[inline]
    pub fn avg_wait_time_ns(&self) -> u64 {
        let completed: Vec<&FutexWaiter> = self.waiters.iter().filter(|w| w.state != WaiterState::Waiting).collect();
        if completed.is_empty() { return 0; }
        let total: u64 = completed.iter().map(|w| w.wait_time_ns()).sum();
        total / completed.len() as u64
    }
}

/// Priority inheritance chain
#[derive(Debug, Clone)]
pub struct PiChain {
    pub chain: Vec<(u64, u64)>, // (task_id, futex_addr) pairs
    pub boosted_priority: i32,
    pub depth: u32,
    pub has_cycle: bool,
}

impl PiChain {
    pub fn new() -> Self {
        Self { chain: Vec::new(), boosted_priority: 0, depth: 0, has_cycle: false }
    }

    #[inline]
    pub fn add_link(&mut self, task: u64, futex_addr: u64) {
        // Check for cycle
        if self.chain.iter().any(|(t, _)| *t == task) {
            self.has_cycle = true;
            return;
        }
        self.chain.push((task, futex_addr));
        self.depth = self.chain.len() as u32;
    }
}

/// Requeue operation
#[derive(Debug, Clone)]
pub struct RequeueOp {
    pub src_addr: u64,
    pub dst_addr: u64,
    pub nr_wake: u32,
    pub nr_requeue: u32,
    pub actual_woken: u32,
    pub actual_requeued: u32,
    pub timestamp: u64,
}

/// Futex tracker stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FutexTrackerStats {
    pub total_futexes: usize,
    pub total_active_waiters: u64,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_timeouts: u64,
    pub total_requeues: u64,
    pub avg_contention: f64,
    pub max_contention: u32,
    pub avg_wait_time_ns: u64,
    pub pi_chains: usize,
    pub deadlocks_detected: usize,
}

/// Holistic futex tracker
pub struct HolisticFutexTracker {
    buckets: BTreeMap<u64, FutexBucket>,
    pi_chains: Vec<PiChain>,
    requeue_history: VecDeque<RequeueOp>,
    max_history: usize,
    stats: FutexTrackerStats,
}

impl HolisticFutexTracker {
    pub fn new() -> Self {
        Self {
            buckets: BTreeMap::new(), pi_chains: Vec::new(),
            requeue_history: VecDeque::new(), max_history: 256,
            stats: FutexTrackerStats::default(),
        }
    }

    #[inline]
    pub fn wait(&mut self, addr: u64, task_id: u64, ts: u64, bitset: u32, timeout_ns: Option<u64>) {
        let bucket = self.buckets.entry(addr).or_insert_with(|| FutexBucket::new(addr));
        let mut waiter = FutexWaiter::new(task_id, ts, bitset);
        if let Some(t) = timeout_ns { waiter.timeout_ns = t; }
        bucket.add_waiter(waiter);
    }

    #[inline]
    pub fn wake(&mut self, addr: u64, nr_wake: u32, ts: u64) -> u32 {
        if let Some(bucket) = self.buckets.get_mut(&addr) {
            bucket.wake_n(nr_wake, ts, u32::MAX)
        } else { 0 }
    }

    #[inline]
    pub fn wake_bitset(&mut self, addr: u64, nr_wake: u32, ts: u64, bitset: u32) -> u32 {
        if let Some(bucket) = self.buckets.get_mut(&addr) {
            bucket.wake_n(nr_wake, ts, bitset)
        } else { 0 }
    }

    pub fn requeue(&mut self, src: u64, dst: u64, nr_wake: u32, nr_requeue: u32, ts: u64) -> (u32, u32) {
        let woken = self.wake(src, nr_wake, ts);
        let mut requeued = 0u32;

        if let Some(src_bucket) = self.buckets.get_mut(&src) {
            let mut to_move: Vec<FutexWaiter> = Vec::new();
            let mut count = 0u32;
            for w in &mut src_bucket.waiters {
                if count >= nr_requeue { break; }
                if w.state == WaiterState::Waiting {
                    w.state = WaiterState::Requeued;
                    let mut new_w = FutexWaiter::new(w.task_id, w.enqueue_ts, w.bitset);
                    new_w.priority = w.priority;
                    to_move.push(new_w);
                    count += 1;
                }
            }
            requeued = to_move.len() as u32;
            src_bucket.total_requeues += requeued as u64;

            let dst_bucket = self.buckets.entry(dst).or_insert_with(|| FutexBucket::new(dst));
            for w in to_move { dst_bucket.add_waiter(w); }
        }

        self.requeue_history.push_back(RequeueOp {
            src_addr: src, dst_addr: dst, nr_wake, nr_requeue,
            actual_woken: woken, actual_requeued: requeued, timestamp: ts,
        });
        if self.requeue_history.len() > self.max_history { self.requeue_history.pop_front(); }

        (woken, requeued)
    }

    #[inline]
    pub fn process_timeouts(&mut self, ts: u64) -> u32 {
        let mut total = 0u32;
        let addrs: Vec<u64> = self.buckets.keys().copied().collect();
        for addr in addrs {
            if let Some(b) = self.buckets.get_mut(&addr) {
                total += b.process_timeouts(ts);
            }
        }
        total
    }

    pub fn detect_pi_deadlock(&mut self, start_task: u64) -> bool {
        let mut chain = PiChain::new();
        let mut current_task = start_task;
        let mut visited = Vec::new();

        for _ in 0..32 { // Max chain depth
            if visited.contains(&current_task) {
                chain.has_cycle = true;
                break;
            }
            visited.push(current_task);

            // Find futex where this task is waiting
            let mut found = false;
            for (addr, bucket) in &self.buckets {
                if bucket.pi_enabled {
                    if bucket.waiters.iter().any(|w| w.task_id == current_task && w.state == WaiterState::Waiting) {
                        chain.add_link(current_task, *addr);
                        if let Some(owner) = bucket.owner_task {
                            current_task = owner;
                            found = true;
                        }
                        break;
                    }
                }
            }
            if !found { break; }
        }

        let is_deadlock = chain.has_cycle;
        self.pi_chains.push(chain);
        is_deadlock
    }

    #[inline]
    pub fn cleanup(&mut self) {
        let addrs: Vec<u64> = self.buckets.keys().copied().collect();
        for addr in addrs {
            if let Some(b) = self.buckets.get_mut(&addr) { b.cleanup_completed(); }
        }
        self.buckets.retain(|_, b| !b.waiters.is_empty() || b.total_waits > 0);
    }

    pub fn recompute(&mut self) {
        self.stats.total_futexes = self.buckets.len();
        self.stats.total_active_waiters = self.buckets.values().map(|b| b.active_waiters() as u64).sum();
        self.stats.total_waits = self.buckets.values().map(|b| b.total_waits).sum();
        self.stats.total_wakes = self.buckets.values().map(|b| b.total_wakes).sum();
        self.stats.total_timeouts = self.buckets.values().map(|b| b.total_timeouts).sum();
        self.stats.total_requeues = self.buckets.values().map(|b| b.total_requeues).sum();
        let conts: Vec<f64> = self.buckets.values().map(|b| b.contention_count as f64).collect();
        self.stats.avg_contention = if conts.is_empty() { 0.0 } else { conts.iter().sum::<f64>() / conts.len() as f64 };
        self.stats.max_contention = self.buckets.values().map(|b| b.max_waiters_seen).max().unwrap_or(0);
        let waits: Vec<u64> = self.buckets.values().map(|b| b.avg_wait_time_ns()).collect();
        self.stats.avg_wait_time_ns = if waits.is_empty() { 0 } else { waits.iter().sum::<u64>() / waits.len() as u64 };
        self.stats.pi_chains = self.pi_chains.len();
        self.stats.deadlocks_detected = self.pi_chains.iter().filter(|c| c.has_cycle).count();
    }

    #[inline(always)]
    pub fn bucket(&self, addr: u64) -> Option<&FutexBucket> { self.buckets.get(&addr) }
    #[inline(always)]
    pub fn stats(&self) -> &FutexTrackerStats { &self.stats }
}
