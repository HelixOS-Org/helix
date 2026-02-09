// SPDX-License-Identifier: GPL-2.0
//! Coop futex_mgr — futex-based synchronization manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Futex operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexOp {
    Wait,
    Wake,
    WakeOp,
    Requeue,
    CmpRequeue,
    WaitBitset,
    WakeBitset,
    LockPi,
    UnlockPi,
    TrylockPi,
    WaitRequeuePi,
}

/// Futex wait state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexWaitState {
    Blocked,
    Woken,
    TimedOut,
    Cancelled,
    Requeued,
}

/// Futex waiter
#[derive(Debug, Clone)]
pub struct FutexWaiter {
    pub tid: u32,
    pub state: FutexWaitState,
    pub expected_val: u32,
    pub bitset: u32,
    pub enqueue_time: u64,
    pub wake_time: u64,
    pub is_pi: bool,
}

impl FutexWaiter {
    pub fn new(tid: u32, expected: u32, bitset: u32, now: u64) -> Self {
        Self {
            tid, state: FutexWaitState::Blocked, expected_val: expected,
            bitset, enqueue_time: now, wake_time: 0, is_pi: false,
        }
    }

    #[inline(always)]
    pub fn wake(&mut self, now: u64) {
        self.state = FutexWaitState::Woken;
        self.wake_time = now;
    }

    #[inline(always)]
    pub fn wait_time(&self) -> u64 {
        if self.wake_time > 0 { self.wake_time - self.enqueue_time } else { 0 }
    }
}

/// Futex hash bucket
#[derive(Debug)]
pub struct FutexBucket {
    pub address: u64,
    pub waiters: Vec<FutexWaiter>,
    pub wake_count: u64,
    pub wait_count: u64,
    pub requeue_count: u64,
    pub collision_count: u64,
}

impl FutexBucket {
    pub fn new(address: u64) -> Self {
        Self {
            address, waiters: Vec::new(), wake_count: 0,
            wait_count: 0, requeue_count: 0, collision_count: 0,
        }
    }

    #[inline(always)]
    pub fn add_waiter(&mut self, waiter: FutexWaiter) {
        self.wait_count += 1;
        self.waiters.push(waiter);
    }

    pub fn wake_n(&mut self, n: u32, bitset: u32, now: u64) -> u32 {
        let mut woken = 0u32;
        for w in &mut self.waiters {
            if woken >= n { break; }
            if w.state == FutexWaitState::Blocked && (w.bitset & bitset) != 0 {
                w.wake(now);
                woken += 1;
            }
        }
        self.wake_count += woken as u64;
        self.waiters.retain(|w| w.state == FutexWaitState::Blocked);
        woken
    }

    pub fn requeue_to(&mut self, target: &mut FutexBucket, n: u32, now: u64) -> u32 {
        let mut requeued = 0u32;
        let mut remaining = Vec::new();
        for w in self.waiters.drain(..) {
            if requeued < n && w.state == FutexWaitState::Blocked {
                let mut moved = w;
                moved.state = FutexWaitState::Requeued;
                target.waiters.push(FutexWaiter {
                    tid: moved.tid, state: FutexWaitState::Blocked,
                    expected_val: 0, bitset: moved.bitset,
                    enqueue_time: now, wake_time: 0, is_pi: moved.is_pi,
                });
                requeued += 1;
            } else {
                remaining.push(w);
            }
        }
        self.waiters = remaining;
        self.requeue_count += requeued as u64;
        requeued
    }

    #[inline]
    pub fn timeout_waiters(&mut self, deadline: u64) -> u32 {
        let mut count = 0u32;
        for w in &mut self.waiters {
            if w.state == FutexWaitState::Blocked && w.enqueue_time < deadline {
                w.state = FutexWaitState::TimedOut;
                count += 1;
            }
        }
        self.waiters.retain(|w| w.state == FutexWaitState::Blocked);
        count
    }

    #[inline(always)]
    pub fn waiter_count(&self) -> usize { self.waiters.len() }
}

/// Futex manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FutexMgrStats {
    pub total_buckets: u32,
    pub total_waiters: u64,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_requeues: u64,
    pub total_timeouts: u64,
    pub peak_waiters: u64,
}

/// Main futex manager
pub struct CoopFutexMgr {
    buckets: BTreeMap<u64, FutexBucket>,
    total_waits: u64,
    total_wakes: u64,
    total_requeues: u64,
    total_timeouts: u64,
    peak_waiters: u64,
    current_waiters: u64,
}

impl CoopFutexMgr {
    pub fn new() -> Self {
        Self {
            buckets: BTreeMap::new(), total_waits: 0, total_wakes: 0,
            total_requeues: 0, total_timeouts: 0, peak_waiters: 0, current_waiters: 0,
        }
    }

    fn hash_address(addr: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        let bytes = addr.to_le_bytes();
        for &b in &bytes {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    #[inline]
    pub fn wait(&mut self, addr: u64, expected: u32, bitset: u32, tid: u32, now: u64) {
        let key = Self::hash_address(addr);
        let bucket = self.buckets.entry(key).or_insert_with(|| FutexBucket::new(addr));
        bucket.add_waiter(FutexWaiter::new(tid, expected, bitset, now));
        self.total_waits += 1;
        self.current_waiters += 1;
        if self.current_waiters > self.peak_waiters { self.peak_waiters = self.current_waiters; }
    }

    #[inline]
    pub fn wake(&mut self, addr: u64, n: u32, bitset: u32, now: u64) -> u32 {
        let key = Self::hash_address(addr);
        if let Some(bucket) = self.buckets.get_mut(&key) {
            let woken = bucket.wake_n(n, bitset, now);
            self.total_wakes += woken as u64;
            self.current_waiters = self.current_waiters.saturating_sub(woken as u64);
            woken
        } else { 0 }
    }

    pub fn requeue(&mut self, from_addr: u64, to_addr: u64, n: u32, now: u64) -> u32 {
        let from_key = Self::hash_address(from_addr);
        let to_key = Self::hash_address(to_addr);
        if from_key == to_key { return 0; }

        // Need to handle borrow checker with temp storage
        let mut from_bucket = match self.buckets.remove(&from_key) {
            Some(b) => b,
            None => return 0,
        };
        let to_bucket = self.buckets.entry(to_key).or_insert_with(|| FutexBucket::new(to_addr));
        let requeued = from_bucket.requeue_to(to_bucket, n, now);
        self.total_requeues += requeued as u64;
        if !from_bucket.waiters.is_empty() {
            self.buckets.insert(from_key, from_bucket);
        }
        requeued
    }

    #[inline]
    pub fn tick_timeouts(&mut self, deadline: u64) -> u32 {
        let mut total = 0u32;
        for bucket in self.buckets.values_mut() {
            let t = bucket.timeout_waiters(deadline);
            total += t;
            self.current_waiters = self.current_waiters.saturating_sub(t as u64);
        }
        self.total_timeouts += total as u64;
        total
    }

    #[inline]
    pub fn hottest_buckets(&self, n: usize) -> Vec<(u64, usize)> {
        let mut v: Vec<_> = self.buckets.iter()
            .map(|(&k, b)| (k, b.waiter_count()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    #[inline]
    pub fn stats(&self) -> FutexMgrStats {
        FutexMgrStats {
            total_buckets: self.buckets.len() as u32,
            total_waiters: self.current_waiters,
            total_waits: self.total_waits,
            total_wakes: self.total_wakes,
            total_requeues: self.total_requeues,
            total_timeouts: self.total_timeouts,
            peak_waiters: self.peak_waiters,
        }
    }
}
