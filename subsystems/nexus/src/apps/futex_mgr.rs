//! # Apps Futex Manager
//!
//! Userspace futex operation management for applications:
//! - Futex wait/wake queue management per address
//! - Priority-inheritance futex support
//! - Robust list tracking for process exit cleanup
//! - Futex contention statistics
//! - PI-boosted wakeup chains
//! - Timeout management for timed waits

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Futex operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexOp {
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
}

/// Futex waiter state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexWaiterState {
    Waiting,
    Woken,
    TimedOut,
    Interrupted,
    Requeued,
}

/// Futex waiter entry
#[derive(Debug, Clone)]
pub struct FutexWaiter {
    pub thread_id: u64,
    pub process_id: u64,
    pub state: FutexWaiterState,
    pub expected_val: u32,
    pub bitset: u32,
    pub priority: u32,
    pub enqueue_ns: u64,
    pub timeout_ns: Option<u64>,
    pub wakeup_ns: u64,
}

impl FutexWaiter {
    pub fn new(tid: u64, pid: u64, expected: u32, bitset: u32, prio: u32, ts: u64) -> Self {
        Self {
            thread_id: tid,
            process_id: pid,
            state: FutexWaiterState::Waiting,
            expected_val: expected,
            bitset,
            priority: prio,
            enqueue_ns: ts,
            timeout_ns: None,
            wakeup_ns: 0,
        }
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout_ns = Some(timeout);
        self
    }

    pub fn wait_duration(&self) -> u64 {
        if self.wakeup_ns > self.enqueue_ns {
            self.wakeup_ns - self.enqueue_ns
        } else { 0 }
    }

    pub fn is_expired(&self, now: u64) -> bool {
        self.timeout_ns.map(|t| now >= t).unwrap_or(false)
    }
}

/// Futex queue (per-address)
#[derive(Debug, Clone)]
pub struct FutexQueue {
    pub address: u64,
    pub waiters: Vec<FutexWaiter>,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_timeouts: u64,
    pub total_requeues: u64,
    pub max_waiters: usize,
    pub pi_owner: Option<u64>,
}

impl FutexQueue {
    pub fn new(addr: u64) -> Self {
        Self {
            address: addr,
            waiters: Vec::new(),
            total_waits: 0,
            total_wakes: 0,
            total_timeouts: 0,
            total_requeues: 0,
            max_waiters: 0,
            pi_owner: None,
        }
    }

    pub fn enqueue(&mut self, waiter: FutexWaiter) {
        self.total_waits += 1;
        self.waiters.push(waiter);
        // Sort by priority (lower = higher priority)
        self.waiters.sort_by_key(|w| w.priority);
        if self.waiters.len() > self.max_waiters {
            self.max_waiters = self.waiters.len();
        }
    }

    pub fn wake(&mut self, count: usize, now: u64) -> Vec<u64> {
        let mut woken = Vec::new();
        let mut remaining = count;
        for waiter in self.waiters.iter_mut() {
            if remaining == 0 { break; }
            if waiter.state == FutexWaiterState::Waiting {
                waiter.state = FutexWaiterState::Woken;
                waiter.wakeup_ns = now;
                woken.push(waiter.thread_id);
                remaining -= 1;
                self.total_wakes += 1;
            }
        }
        self.waiters.retain(|w| w.state == FutexWaiterState::Waiting);
        woken
    }

    pub fn wake_bitset(&mut self, bitset: u32, count: usize, now: u64) -> Vec<u64> {
        let mut woken = Vec::new();
        let mut remaining = count;
        for waiter in self.waiters.iter_mut() {
            if remaining == 0 { break; }
            if waiter.state == FutexWaiterState::Waiting && (waiter.bitset & bitset) != 0 {
                waiter.state = FutexWaiterState::Woken;
                waiter.wakeup_ns = now;
                woken.push(waiter.thread_id);
                remaining -= 1;
                self.total_wakes += 1;
            }
        }
        self.waiters.retain(|w| w.state == FutexWaiterState::Waiting);
        woken
    }

    pub fn expire_timeouts(&mut self, now: u64) -> Vec<u64> {
        let mut expired = Vec::new();
        for waiter in self.waiters.iter_mut() {
            if waiter.state == FutexWaiterState::Waiting && waiter.is_expired(now) {
                waiter.state = FutexWaiterState::TimedOut;
                waiter.wakeup_ns = now;
                expired.push(waiter.thread_id);
                self.total_timeouts += 1;
            }
        }
        self.waiters.retain(|w| w.state == FutexWaiterState::Waiting);
        expired
    }

    pub fn waiter_count(&self) -> usize { self.waiters.len() }
    pub fn is_empty(&self) -> bool { self.waiters.is_empty() }
    pub fn contention_level(&self) -> f64 { self.waiters.len() as f64 }
}

/// Robust futex entry
#[derive(Debug, Clone)]
pub struct RobustEntry {
    pub address: u64,
    pub owner_tid: u64,
    pub pending: bool,
}

/// Per-process robust list
#[derive(Debug, Clone)]
pub struct ProcessRobustList {
    pub process_id: u64,
    pub entries: Vec<RobustEntry>,
    pub head_addr: u64,
}

/// Futex manager stats
#[derive(Debug, Clone, Default)]
pub struct AppsFutexMgrStats {
    pub total_queues: usize,
    pub total_waiting: usize,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_timeouts: u64,
    pub max_contention: usize,
}

/// Apps Futex Manager
pub struct AppsFutexMgr {
    queues: BTreeMap<u64, FutexQueue>,
    robust_lists: BTreeMap<u64, ProcessRobustList>,
    stats: AppsFutexMgrStats,
}

impl AppsFutexMgr {
    pub fn new() -> Self {
        Self {
            queues: BTreeMap::new(),
            robust_lists: BTreeMap::new(),
            stats: AppsFutexMgrStats::default(),
        }
    }

    pub fn futex_wait(&mut self, addr: u64, waiter: FutexWaiter) {
        let queue = self.queues.entry(addr).or_insert_with(|| FutexQueue::new(addr));
        queue.enqueue(waiter);
    }

    pub fn futex_wake(&mut self, addr: u64, count: usize, now: u64) -> Vec<u64> {
        if let Some(queue) = self.queues.get_mut(&addr) {
            queue.wake(count, now)
        } else { Vec::new() }
    }

    pub fn futex_wake_bitset(&mut self, addr: u64, bitset: u32, count: usize, now: u64) -> Vec<u64> {
        if let Some(queue) = self.queues.get_mut(&addr) {
            queue.wake_bitset(bitset, count, now)
        } else { Vec::new() }
    }

    pub fn futex_requeue(&mut self, from: u64, to: u64, wake_count: usize, requeue_count: usize, now: u64) -> (Vec<u64>, usize) {
        let woken = if let Some(q) = self.queues.get_mut(&from) {
            q.wake(wake_count, now)
        } else { return (Vec::new(), 0); };

        // Move remaining waiters to target queue
        let mut moved = 0;
        if let Some(from_q) = self.queues.get_mut(&from) {
            let mut to_move = Vec::new();
            let mut remaining = requeue_count;
            let mut i = 0;
            while i < from_q.waiters.len() && remaining > 0 {
                if from_q.waiters[i].state == FutexWaiterState::Waiting {
                    let mut w = from_q.waiters.remove(i);
                    w.state = FutexWaiterState::Requeued;
                    to_move.push(w);
                    remaining -= 1;
                    moved += 1;
                } else { i += 1; }
            }
            from_q.total_requeues += moved as u64;

            let to_q = self.queues.entry(to).or_insert_with(|| FutexQueue::new(to));
            for mut w in to_move {
                w.state = FutexWaiterState::Waiting;
                to_q.enqueue(w);
            }
        }
        (woken, moved)
    }

    pub fn tick_timeouts(&mut self, now: u64) -> Vec<u64> {
        let mut all_expired = Vec::new();
        for queue in self.queues.values_mut() {
            let expired = queue.expire_timeouts(now);
            all_expired.extend(expired);
        }
        all_expired
    }

    pub fn register_robust_list(&mut self, pid: u64, head: u64) {
        self.robust_lists.insert(pid, ProcessRobustList {
            process_id: pid, entries: Vec::new(), head_addr: head,
        });
    }

    pub fn cleanup_process(&mut self, pid: u64, now: u64) {
        // Wake all waiters for robust futexes owned by this process
        if let Some(robust) = self.robust_lists.remove(&pid) {
            for entry in &robust.entries {
                if let Some(queue) = self.queues.get_mut(&entry.address) {
                    queue.wake(1, now);
                }
            }
        }
        // Remove empty queues
        self.queues.retain(|_, q| !q.is_empty());
    }

    pub fn recompute(&mut self) {
        self.stats.total_queues = self.queues.len();
        self.stats.total_waiting = self.queues.values().map(|q| q.waiter_count()).sum();
        self.stats.total_waits = self.queues.values().map(|q| q.total_waits).sum();
        self.stats.total_wakes = self.queues.values().map(|q| q.total_wakes).sum();
        self.stats.total_timeouts = self.queues.values().map(|q| q.total_timeouts).sum();
        self.stats.max_contention = self.queues.values().map(|q| q.max_waiters).max().unwrap_or(0);
    }

    pub fn queue(&self, addr: u64) -> Option<&FutexQueue> { self.queues.get(&addr) }
    pub fn stats(&self) -> &AppsFutexMgrStats { &self.stats }
}
