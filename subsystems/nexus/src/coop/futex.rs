// SPDX-License-Identifier: GPL-2.0
//! Coop futex — futex-based synchronization primitives.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Futex operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopFutexOp {
    Wait,
    Wake,
    Fd,
    Requeue,
    CmpRequeue,
    WakeOp,
    LockPi,
    UnlockPi,
    TrylockPi,
    WaitBitset,
    WakeBitset,
}

/// Futex waiter
#[derive(Debug)]
pub struct CoopFutexWaiter {
    pub tid: u64,
    pub addr: u64,
    pub expected_val: u32,
    pub bitset: u32,
    pub queued_at: u64,
    pub timeout_ns: u64,
}

/// Futex hash bucket
#[derive(Debug)]
pub struct CoopFutexBucket {
    pub waiters: Vec<CoopFutexWaiter>,
    pub wake_count: u64,
    pub wait_count: u64,
}

impl CoopFutexBucket {
    pub fn new() -> Self { Self { waiters: Vec::new(), wake_count: 0, wait_count: 0 } }

    pub fn wait(&mut self, waiter: CoopFutexWaiter) {
        self.wait_count += 1;
        self.waiters.push(waiter);
    }

    pub fn wake(&mut self, count: u32, bitset: u32) -> u32 {
        let mut woken = 0u32;
        let mut remaining = Vec::new();
        for w in self.waiters.drain(..) {
            if woken < count && (w.bitset & bitset) != 0 {
                woken += 1;
            } else {
                remaining.push(w);
            }
        }
        self.waiters = remaining;
        self.wake_count += woken as u64;
        woken
    }
}

fn hash_addr(addr: u64) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    let bytes = addr.to_le_bytes();
    for &b in &bytes { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
    h
}

/// Stats
#[derive(Debug, Clone)]
pub struct CoopFutexStats {
    pub total_buckets: u32,
    pub total_waiters: u32,
    pub total_waits: u64,
    pub total_wakes: u64,
}

/// Main coop futex manager
pub struct CoopFutex {
    buckets: BTreeMap<u64, CoopFutexBucket>,
    bucket_count: u32,
}

impl CoopFutex {
    pub fn new(n_buckets: u32) -> Self {
        let mut buckets = BTreeMap::new();
        for i in 0..n_buckets { buckets.insert(i as u64, CoopFutexBucket::new()); }
        Self { buckets, bucket_count: n_buckets }
    }

    fn bucket_id(&self, addr: u64) -> u64 {
        hash_addr(addr) % self.bucket_count as u64
    }

    pub fn wait(&mut self, waiter: CoopFutexWaiter) {
        let bid = self.bucket_id(waiter.addr);
        if let Some(b) = self.buckets.get_mut(&bid) { b.wait(waiter); }
    }

    pub fn wake(&mut self, addr: u64, count: u32, bitset: u32) -> u32 {
        let bid = self.bucket_id(addr);
        if let Some(b) = self.buckets.get_mut(&bid) { b.wake(count, bitset) }
        else { 0 }
    }

    pub fn stats(&self) -> CoopFutexStats {
        let waiters: u32 = self.buckets.values().map(|b| b.waiters.len() as u32).sum();
        let waits: u64 = self.buckets.values().map(|b| b.wait_count).sum();
        let wakes: u64 = self.buckets.values().map(|b| b.wake_count).sum();
        CoopFutexStats { total_buckets: self.bucket_count, total_waiters: waiters, total_waits: waits, total_wakes: wakes }
    }
}

// ============================================================================
// Merged from futex_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopFutexOp {
    Wait,
    Wake,
    WakeAll,
    Requeue,
    CompareAndWait,
}

/// Futex coop waiter
#[derive(Debug, Clone)]
pub struct CoopFutexWaiter {
    pub tid: u64,
    pub expected_val: u32,
    pub enqueue_tick: u64,
    pub woken: bool,
}

/// A cooperative futex
#[derive(Debug, Clone)]
pub struct CoopFutexV2Instance {
    pub addr: u64,
    pub value: u32,
    pub waiters: Vec<CoopFutexWaiter>,
    pub wake_ops: u64,
    pub wait_ops: u64,
    pub contentions: u64,
}

impl CoopFutexV2Instance {
    pub fn new(addr: u64, value: u32) -> Self {
        Self {
            addr, value,
            waiters: Vec::new(),
            wake_ops: 0, wait_ops: 0, contentions: 0,
        }
    }

    pub fn wait(&mut self, tid: u64, expected: u32, tick: u64) -> bool {
        if self.value != expected {
            return false;
        }
        self.waiters.push(CoopFutexWaiter {
            tid, expected_val: expected,
            enqueue_tick: tick, woken: false,
        });
        self.wait_ops += 1;
        self.contentions += 1;
        true
    }

    pub fn wake(&mut self, count: usize) -> u64 {
        let mut woken = 0u64;
        for w in self.waiters.iter_mut() {
            if !w.woken {
                w.woken = true;
                woken += 1;
                if woken >= count as u64 { break; }
            }
        }
        self.wake_ops += 1;
        self.waiters.retain(|w| !w.woken);
        woken
    }

    pub fn wake_all(&mut self) -> u64 {
        let count = self.waiters.len() as u64;
        self.waiters.clear();
        self.wake_ops += 1;
        count
    }
}

/// Statistics for coop futex V2
#[derive(Debug, Clone)]
pub struct CoopFutexV2Stats {
    pub futexes_created: u64,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_contentions: u64,
    pub spurious_wakeups: u64,
}

/// Main coop futex V2 manager
#[derive(Debug)]
pub struct CoopFutexV2 {
    futexes: BTreeMap<u64, CoopFutexV2Instance>,
    stats: CoopFutexV2Stats,
}

impl CoopFutexV2 {
    pub fn new() -> Self {
        Self {
            futexes: BTreeMap::new(),
            stats: CoopFutexV2Stats {
                futexes_created: 0, total_waits: 0,
                total_wakes: 0, total_contentions: 0,
                spurious_wakeups: 0,
            },
        }
    }

    pub fn get_or_create(&mut self, addr: u64, value: u32) -> &mut CoopFutexV2Instance {
        if !self.futexes.contains_key(&addr) {
            self.futexes.insert(addr, CoopFutexV2Instance::new(addr, value));
            self.stats.futexes_created += 1;
        }
        self.futexes.get_mut(&addr).unwrap()
    }

    pub fn wait(&mut self, addr: u64, tid: u64, expected: u32, tick: u64) -> bool {
        let futex = self.get_or_create(addr, expected);
        let ok = futex.wait(tid, expected, tick);
        if ok { self.stats.total_waits += 1; }
        ok
    }

    pub fn wake(&mut self, addr: u64, count: usize) -> u64 {
        if let Some(futex) = self.futexes.get_mut(&addr) {
            let woken = futex.wake(count);
            self.stats.total_wakes += woken;
            woken
        } else { 0 }
    }

    pub fn stats(&self) -> &CoopFutexV2Stats {
        &self.stats
    }
}

// ============================================================================
// Merged from futex_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexV3Op {
    Wait,
    Wake,
    WaitBitset,
    WakeBitset,
    WakeOp,
    Requeue,
    CmpRequeue,
    LockPi,
    UnlockPi,
    TrylockPi,
    WaitRequeuePi,
}

/// Futex wait result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexV3Result {
    Success,
    WouldBlock,
    Timeout,
    Interrupted,
    OwnerDied,
    NotRecoverable,
}

/// A futex V3 waiter.
#[derive(Debug, Clone)]
pub struct FutexV3Waiter {
    pub waiter_id: u64,
    pub pid: u64,
    pub priority: u32,
    pub bitset: u32,
    pub deadline_ns: Option<u64>,
    pub is_pi: bool,
    pub numa_node: u32,
}

impl FutexV3Waiter {
    pub fn new(waiter_id: u64, pid: u64) -> Self {
        Self {
            waiter_id,
            pid,
            priority: 0,
            bitset: 0xFFFFFFFF,
            deadline_ns: None,
            is_pi: false,
            numa_node: 0,
        }
    }
}

/// A futex V3 instance (identified by address).
#[derive(Debug, Clone)]
pub struct FutexV3Instance {
    pub address: u64,
    pub waiters: Vec<FutexV3Waiter>,
    pub owner_pid: Option<u64>,
    pub is_pi: bool,
    pub is_robust: bool,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_requeues: u64,
    pub owner_died_count: u64,
    pub pi_boosts: u64,
}

impl FutexV3Instance {
    pub fn new(address: u64) -> Self {
        Self {
            address,
            waiters: Vec::new(),
            owner_pid: None,
            is_pi: false,
            is_robust: false,
            total_waits: 0,
            total_wakes: 0,
            total_requeues: 0,
            owner_died_count: 0,
            pi_boosts: 0,
        }
    }

    pub fn enqueue_waiter(&mut self, waiter: FutexV3Waiter) {
        self.total_waits += 1;
        if waiter.is_pi {
            self.is_pi = true;
        }
        self.waiters.push(waiter);
    }

    pub fn wake(&mut self, count: usize, bitset: u32) -> Vec<FutexV3Waiter> {
        let mut woken = Vec::new();
        let mut remaining = Vec::new();
        for w in self.waiters.drain(..) {
            if woken.len() < count && (w.bitset & bitset) != 0 {
                woken.push(w);
            } else {
                remaining.push(w);
            }
        }
        self.total_wakes += woken.len() as u64;
        self.waiters = remaining;
        woken
    }

    pub fn requeue_to(&mut self, target: &mut FutexV3Instance, count: usize) -> u64 {
        let mut moved = 0u64;
        while !self.waiters.is_empty() && moved < count as u64 {
            let w = self.waiters.remove(0);
            target.waiters.push(w);
            moved += 1;
        }
        self.total_requeues += moved;
        moved
    }

    pub fn handle_owner_death(&mut self) -> Vec<FutexV3Waiter> {
        self.owner_died_count += 1;
        self.owner_pid = None;
        // Wake all PI waiters
        if self.is_pi {
            let all = core::mem::take(&mut self.waiters);
            self.total_wakes += all.len() as u64;
            return all;
        }
        Vec::new()
    }

    pub fn waiter_count(&self) -> usize {
        self.waiters.len()
    }
}

/// Statistics for futex V3.
#[derive(Debug, Clone)]
pub struct FutexV3Stats {
    pub total_futexes: u64,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_requeues: u64,
    pub pi_operations: u64,
    pub robust_recoveries: u64,
}

/// Main coop futex V3 manager.
pub struct CoopFutexV3 {
    pub futexes: BTreeMap<u64, FutexV3Instance>,
    pub robust_lists: BTreeMap<u64, Vec<u64>>, // pid → [futex addresses]
    pub next_waiter_id: u64,
    pub stats: FutexV3Stats,
}

impl CoopFutexV3 {
    pub fn new() -> Self {
        Self {
            futexes: BTreeMap::new(),
            robust_lists: BTreeMap::new(),
            next_waiter_id: 1,
            stats: FutexV3Stats {
                total_futexes: 0,
                total_waits: 0,
                total_wakes: 0,
                total_requeues: 0,
                pi_operations: 0,
                robust_recoveries: 0,
            },
        }
    }

    pub fn get_or_create(&mut self, address: u64) -> &mut FutexV3Instance {
        if !self.futexes.contains_key(&address) {
            let inst = FutexV3Instance::new(address);
            self.futexes.insert(address, inst);
            self.stats.total_futexes += 1;
        }
        self.futexes.get_mut(&address).unwrap()
    }

    pub fn register_robust_list(&mut self, pid: u64, addresses: Vec<u64>) {
        self.robust_lists.insert(pid, addresses);
    }

    pub fn futex_count(&self) -> usize {
        self.futexes.len()
    }
}
