// SPDX-License-Identifier: GPL-2.0
//! Holistic futex_mgr — futex wait/wake management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Futex operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FutexOp {
    Wait,
    Wake,
    Requeue,
    CmpRequeue,
    WaitBitset,
    WakeBitset,
    WakeOp,
    LockPi,
    UnlockPi,
}

/// Futex waiter
#[derive(Debug)]
pub struct FutexWaiter {
    pub tid: u64,
    pub uaddr: u64,
    pub expected_val: u32,
    pub bitset: u32,
    pub wait_start: u64,
    pub is_pi: bool,
}

/// Futex hash bucket
#[derive(Debug)]
pub struct FutexBucket {
    pub hash: u64,
    pub waiters: Vec<FutexWaiter>,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_requeues: u64,
}

impl FutexBucket {
    pub fn new(hash: u64) -> Self {
        Self { hash, waiters: Vec::new(), total_waits: 0, total_wakes: 0, total_requeues: 0 }
    }

    pub fn wait(&mut self, tid: u64, uaddr: u64, val: u32, bitset: u32, now: u64, is_pi: bool) {
        self.waiters.push(FutexWaiter { tid, uaddr, expected_val: val, bitset, wait_start: now, is_pi });
        self.total_waits += 1;
    }

    pub fn wake(&mut self, uaddr: u64, nr: u32, bitset: u32) -> u32 {
        let mut woken = 0u32;
        self.waiters.retain(|w| {
            if w.uaddr == uaddr && (bitset == 0 || w.bitset & bitset != 0) && woken < nr { woken += 1; false }
            else { true }
        });
        self.total_wakes += woken as u64;
        woken
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct FutexMgrStats {
    pub total_buckets: u32,
    pub total_waiters: u32,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_requeues: u64,
}

/// Main holistic futex manager
pub struct HolisticFutexMgr {
    buckets: BTreeMap<u64, FutexBucket>,
}

impl HolisticFutexMgr {
    pub fn new() -> Self { Self { buckets: BTreeMap::new() } }

    fn hash_uaddr(uaddr: u64) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        let bytes = uaddr.to_le_bytes();
        for &b in &bytes { h ^= b as u64; h = h.wrapping_mul(0x100000001b3); }
        h
    }

    pub fn wait(&mut self, tid: u64, uaddr: u64, val: u32, bitset: u32, now: u64) {
        let hash = Self::hash_uaddr(uaddr);
        let bucket = self.buckets.entry(hash).or_insert_with(|| FutexBucket::new(hash));
        bucket.wait(tid, uaddr, val, bitset, now, false);
    }

    pub fn wake(&mut self, uaddr: u64, nr: u32, bitset: u32) -> u32 {
        let hash = Self::hash_uaddr(uaddr);
        self.buckets.get_mut(&hash).map_or(0, |b| b.wake(uaddr, nr, bitset))
    }

    pub fn stats(&self) -> FutexMgrStats {
        let waiters: u32 = self.buckets.values().map(|b| b.waiters.len() as u32).sum();
        let waits: u64 = self.buckets.values().map(|b| b.total_waits).sum();
        let wakes: u64 = self.buckets.values().map(|b| b.total_wakes).sum();
        let requeues: u64 = self.buckets.values().map(|b| b.total_requeues).sum();
        FutexMgrStats { total_buckets: self.buckets.len() as u32, total_waiters: waiters, total_waits: waits, total_wakes: wakes, total_requeues: requeues }
    }
}
