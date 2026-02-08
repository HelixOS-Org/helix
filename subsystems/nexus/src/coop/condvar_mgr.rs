// SPDX-License-Identifier: GPL-2.0
//! Coop condvar_mgr — condition variable manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Condvar wait result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondWaitResult {
    Signaled,
    Broadcast,
    TimedOut,
    Spurious,
}

/// Condvar waiter
#[derive(Debug)]
pub struct CondWaiter {
    pub tid: u64,
    pub wait_start: u64,
    pub result: Option<CondWaitResult>,
}

impl CondWaiter {
    pub fn new(tid: u64, now: u64) -> Self { Self { tid, wait_start: now, result: None } }
    pub fn wake(&mut self, result: CondWaitResult) { self.result = Some(result); }
    pub fn wait_time(&self, now: u64) -> u64 { now.saturating_sub(self.wait_start) }
}

/// Condition variable
#[derive(Debug)]
pub struct CondVar {
    pub id: u64,
    pub mutex_id: u64,
    pub waiters: Vec<CondWaiter>,
    pub total_signals: u64,
    pub total_broadcasts: u64,
    pub total_waits: u64,
    pub total_timeouts: u64,
    pub total_spurious: u64,
}

impl CondVar {
    pub fn new(id: u64, mutex_id: u64) -> Self {
        Self { id, mutex_id, waiters: Vec::new(), total_signals: 0, total_broadcasts: 0, total_waits: 0, total_timeouts: 0, total_spurious: 0 }
    }

    pub fn wait(&mut self, tid: u64, now: u64) {
        self.total_waits += 1;
        self.waiters.push(CondWaiter::new(tid, now));
    }

    pub fn signal(&mut self) -> Option<u64> {
        self.total_signals += 1;
        if let Some(w) = self.waiters.iter_mut().find(|w| w.result.is_none()) {
            w.wake(CondWaitResult::Signaled);
            Some(w.tid)
        } else { None }
    }

    pub fn broadcast(&mut self) -> u32 {
        self.total_broadcasts += 1;
        let mut woken = 0u32;
        for w in &mut self.waiters {
            if w.result.is_none() { w.wake(CondWaitResult::Broadcast); woken += 1; }
        }
        woken
    }

    pub fn pending_count(&self) -> u32 { self.waiters.iter().filter(|w| w.result.is_none()).count() as u32 }
}

/// Stats
#[derive(Debug, Clone)]
pub struct CondvarMgrStats {
    pub total_condvars: u32,
    pub total_signals: u64,
    pub total_broadcasts: u64,
    pub total_waits: u64,
    pub total_timeouts: u64,
    pub pending_waiters: u32,
}

/// Main condvar manager
pub struct CoopCondvarMgr {
    condvars: BTreeMap<u64, CondVar>,
    next_id: u64,
}

impl CoopCondvarMgr {
    pub fn new() -> Self { Self { condvars: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, mutex_id: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.condvars.insert(id, CondVar::new(id, mutex_id));
        id
    }

    pub fn wait(&mut self, cv: u64, tid: u64, now: u64) {
        if let Some(c) = self.condvars.get_mut(&cv) { c.wait(tid, now); }
    }

    pub fn signal(&mut self, cv: u64) -> Option<u64> {
        self.condvars.get_mut(&cv)?.signal()
    }

    pub fn broadcast(&mut self, cv: u64) -> u32 {
        self.condvars.get_mut(&cv).map(|c| c.broadcast()).unwrap_or(0)
    }

    pub fn stats(&self) -> CondvarMgrStats {
        let sigs: u64 = self.condvars.values().map(|c| c.total_signals).sum();
        let bcast: u64 = self.condvars.values().map(|c| c.total_broadcasts).sum();
        let waits: u64 = self.condvars.values().map(|c| c.total_waits).sum();
        let timeouts: u64 = self.condvars.values().map(|c| c.total_timeouts).sum();
        let pending: u32 = self.condvars.values().map(|c| c.pending_count()).sum();
        CondvarMgrStats { total_condvars: self.condvars.len() as u32, total_signals: sigs, total_broadcasts: bcast, total_waits: waits, total_timeouts: timeouts, pending_waiters: pending }
    }
}

// ============================================================================
// Merged from condvar_v2_mgr
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondvarV2Result {
    Signaled,
    Broadcast,
    TimedOut,
    Spurious,
    Interrupted,
}

/// A condvar waiter
#[derive(Debug, Clone)]
pub struct CondvarV2Waiter {
    pub thread_id: u64,
    pub mutex_id: u64,
    pub wait_start_tick: u64,
    pub timeout_ns: u64,
    pub result: Option<CondvarV2Result>,
}

/// A condition variable instance
#[derive(Debug, Clone)]
pub struct CondvarV2Instance {
    pub id: u64,
    pub waiters: Vec<CondvarV2Waiter>,
    pub signal_count: u64,
    pub broadcast_count: u64,
    pub total_waits: u64,
    pub spurious_wakeups: u64,
}

impl CondvarV2Instance {
    pub fn new(id: u64) -> Self {
        Self {
            id, waiters: Vec::new(),
            signal_count: 0, broadcast_count: 0,
            total_waits: 0, spurious_wakeups: 0,
        }
    }

    pub fn wait(&mut self, tid: u64, mutex_id: u64, tick: u64, timeout: u64) {
        self.waiters.push(CondvarV2Waiter {
            thread_id: tid, mutex_id,
            wait_start_tick: tick, timeout_ns: timeout,
            result: None,
        });
        self.total_waits += 1;
    }

    pub fn signal(&mut self) -> Option<u64> {
        self.signal_count += 1;
        for w in self.waiters.iter_mut() {
            if w.result.is_none() {
                w.result = Some(CondvarV2Result::Signaled);
                return Some(w.thread_id);
            }
        }
        None
    }

    pub fn broadcast(&mut self) -> u64 {
        self.broadcast_count += 1;
        let mut woken = 0u64;
        for w in self.waiters.iter_mut() {
            if w.result.is_none() {
                w.result = Some(CondvarV2Result::Broadcast);
                woken += 1;
            }
        }
        woken
    }

    pub fn drain_completed(&mut self) {
        self.waiters.retain(|w| w.result.is_none());
    }
}

/// Statistics for condvar V2
#[derive(Debug, Clone)]
pub struct CondvarV2Stats {
    pub condvars_created: u64,
    pub total_waits: u64,
    pub total_signals: u64,
    pub total_broadcasts: u64,
    pub timeouts: u64,
    pub spurious_wakeups: u64,
}

/// Main condvar V2 coop manager
#[derive(Debug)]
pub struct CoopCondvarV2 {
    condvars: BTreeMap<u64, CondvarV2Instance>,
    next_id: u64,
    stats: CondvarV2Stats,
}

impl CoopCondvarV2 {
    pub fn new() -> Self {
        Self {
            condvars: BTreeMap::new(),
            next_id: 1,
            stats: CondvarV2Stats {
                condvars_created: 0, total_waits: 0,
                total_signals: 0, total_broadcasts: 0,
                timeouts: 0, spurious_wakeups: 0,
            },
        }
    }

    pub fn create(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.condvars.insert(id, CondvarV2Instance::new(id));
        self.stats.condvars_created += 1;
        id
    }

    pub fn wait(&mut self, cv_id: u64, tid: u64, mutex_id: u64, tick: u64, timeout: u64) -> bool {
        if let Some(cv) = self.condvars.get_mut(&cv_id) {
            cv.wait(tid, mutex_id, tick, timeout);
            self.stats.total_waits += 1;
            true
        } else { false }
    }

    pub fn signal(&mut self, cv_id: u64) -> Option<u64> {
        if let Some(cv) = self.condvars.get_mut(&cv_id) {
            self.stats.total_signals += 1;
            cv.signal()
        } else { None }
    }

    pub fn broadcast(&mut self, cv_id: u64) -> u64 {
        if let Some(cv) = self.condvars.get_mut(&cv_id) {
            self.stats.total_broadcasts += 1;
            cv.broadcast()
        } else { 0 }
    }

    pub fn stats(&self) -> &CondvarV2Stats {
        &self.stats
    }
}

// ============================================================================
// Merged from condvar_v3_mgr
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondvarV3WaitResult {
    Signaled,
    Broadcast,
    Timeout,
    Spurious,
    Interrupted,
}

/// Wait-morph target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CondvarV3MorphTarget {
    Mutex,
    RwLockRead,
    RwLockWrite,
    Semaphore,
}

/// A waiter on the condvar.
#[derive(Debug, Clone)]
pub struct CondvarV3Waiter {
    pub waiter_id: u64,
    pub pid: u64,
    pub deadline_ns: Option<u64>,
    pub morph_target: Option<CondvarV3MorphTarget>,
    pub spurious_count: u32,
    pub enqueue_time: u64,
}

impl CondvarV3Waiter {
    pub fn new(waiter_id: u64, pid: u64) -> Self {
        Self {
            waiter_id,
            pid,
            deadline_ns: None,
            morph_target: None,
            spurious_count: 0,
            enqueue_time: 0,
        }
    }
}

/// A condvar V3 instance.
#[derive(Debug, Clone)]
pub struct CondvarV3Instance {
    pub cv_id: u64,
    pub waiters: Vec<CondvarV3Waiter>,
    pub associated_lock: Option<u64>,
    pub signal_count: u64,
    pub broadcast_count: u64,
    pub total_waits: u64,
    pub spurious_wakeups: u64,
    pub timeout_count: u64,
    pub morph_count: u64,
}

impl CondvarV3Instance {
    pub fn new(cv_id: u64) -> Self {
        Self {
            cv_id,
            waiters: Vec::new(),
            associated_lock: None,
            signal_count: 0,
            broadcast_count: 0,
            total_waits: 0,
            spurious_wakeups: 0,
            timeout_count: 0,
            morph_count: 0,
        }
    }

    pub fn enqueue_waiter(&mut self, waiter: CondvarV3Waiter) {
        self.total_waits += 1;
        self.waiters.push(waiter);
    }

    pub fn signal_one(&mut self) -> Option<CondvarV3Waiter> {
        self.signal_count += 1;
        if self.waiters.is_empty() {
            return None;
        }
        Some(self.waiters.remove(0))
    }

    pub fn broadcast_all(&mut self) -> Vec<CondvarV3Waiter> {
        self.broadcast_count += 1;
        let mut all = Vec::new();
        core::mem::swap(&mut all, &mut self.waiters);
        all
    }

    pub fn expire_timeouts(&mut self, current_ns: u64) -> Vec<CondvarV3Waiter> {
        let mut expired = Vec::new();
        self.waiters.retain(|w| {
            if let Some(deadline) = w.deadline_ns {
                if current_ns >= deadline {
                    expired.push(w.clone());
                    return false;
                }
            }
            true
        });
        self.timeout_count += expired.len() as u64;
        expired
    }

    pub fn waiter_count(&self) -> usize {
        self.waiters.len()
    }
}

/// Statistics for condvar V3.
#[derive(Debug, Clone)]
pub struct CondvarV3Stats {
    pub total_condvars: u64,
    pub total_signals: u64,
    pub total_broadcasts: u64,
    pub total_waits: u64,
    pub total_spurious: u64,
    pub total_timeouts: u64,
    pub total_morphs: u64,
}

/// Main coop condvar V3 manager.
pub struct CoopCondvarV3 {
    pub condvars: BTreeMap<u64, CondvarV3Instance>,
    pub next_cv_id: u64,
    pub stats: CondvarV3Stats,
}

impl CoopCondvarV3 {
    pub fn new() -> Self {
        Self {
            condvars: BTreeMap::new(),
            next_cv_id: 1,
            stats: CondvarV3Stats {
                total_condvars: 0,
                total_signals: 0,
                total_broadcasts: 0,
                total_waits: 0,
                total_spurious: 0,
                total_timeouts: 0,
                total_morphs: 0,
            },
        }
    }

    pub fn create_condvar(&mut self) -> u64 {
        let id = self.next_cv_id;
        self.next_cv_id += 1;
        let cv = CondvarV3Instance::new(id);
        self.condvars.insert(id, cv);
        self.stats.total_condvars += 1;
        id
    }

    pub fn signal(&mut self, cv_id: u64) -> Option<CondvarV3Waiter> {
        if let Some(cv) = self.condvars.get_mut(&cv_id) {
            self.stats.total_signals += 1;
            cv.signal_one()
        } else {
            None
        }
    }

    pub fn condvar_count(&self) -> usize {
        self.condvars.len()
    }
}
