// SPDX-License-Identifier: GPL-2.0
//! Coop fair_lock — fair/ordered lock primitive with starvation prevention.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Lock fairness policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FairnessPolicy {
    /// Strict FIFO ordering
    Fifo,
    /// Priority-based with aging
    PriorityAging,
    /// Reader-writer with writer preference
    RwWriterPref,
    /// Reader-writer with reader preference
    RwReaderPref,
    /// Ticket-based
    Ticket,
    /// MCS-style queue-based
    Mcs,
}

/// Lock holder type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoldType {
    Exclusive,
    Shared,
}

/// Waiter state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaiterState {
    Waiting,
    Spinning,
    Granted,
    TimedOut,
    Cancelled,
}

/// A lock waiter entry
#[derive(Debug, Clone)]
pub struct LockWaiter {
    pub thread_id: u64,
    pub hold_type: HoldType,
    pub state: WaiterState,
    pub priority: u32,
    pub effective_priority: u32,
    pub ticket: u64,
    pub enqueue_ns: u64,
    pub age_bumps: u32,
}

impl LockWaiter {
    pub fn new(thread_id: u64, hold_type: HoldType, priority: u32, ticket: u64, now_ns: u64) -> Self {
        Self {
            thread_id,
            hold_type,
            state: WaiterState::Waiting,
            priority,
            effective_priority: priority,
            ticket,
            enqueue_ns: now_ns,
            age_bumps: 0,
        }
    }

    pub fn wait_ns(&self, now_ns: u64) -> u64 {
        now_ns.saturating_sub(self.enqueue_ns)
    }

    pub fn bump_priority(&mut self) {
        self.age_bumps += 1;
        self.effective_priority = self.priority.saturating_add(self.age_bumps);
    }
}

/// Active holder info
#[derive(Debug, Clone)]
pub struct LockHolder {
    pub thread_id: u64,
    pub hold_type: HoldType,
    pub acquire_ns: u64,
    pub hold_duration_ns: u64,
}

/// A fair lock instance
#[derive(Debug)]
pub struct FairLock {
    pub id: u64,
    pub policy: FairnessPolicy,
    pub holders: Vec<LockHolder>,
    waiters: Vec<LockWaiter>,
    next_ticket: u64,
    now_serving: u64,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_waits: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
    pub total_hold_ns: u64,
    pub max_hold_ns: u64,
    pub starvation_events: u64,
    starvation_threshold_ns: u64,
}

impl FairLock {
    pub fn new(id: u64, policy: FairnessPolicy) -> Self {
        Self {
            id,
            policy,
            holders: Vec::new(),
            waiters: Vec::new(),
            next_ticket: 0,
            now_serving: 0,
            total_acquires: 0,
            total_releases: 0,
            total_waits: 0,
            total_wait_ns: 0,
            max_wait_ns: 0,
            total_hold_ns: 0,
            max_hold_ns: 0,
            starvation_events: 0,
            starvation_threshold_ns: 100_000_000, // 100ms
        }
    }

    pub fn try_acquire(&mut self, thread_id: u64, hold_type: HoldType, priority: u32, now_ns: u64) -> bool {
        // Check if can be granted immediately
        match hold_type {
            HoldType::Exclusive => {
                if !self.holders.is_empty() || !self.waiters.is_empty() {
                    return false;
                }
            }
            HoldType::Shared => {
                if self.holders.iter().any(|h| h.hold_type == HoldType::Exclusive) {
                    return false;
                }
                // Writer preference: block if writers waiting
                if matches!(self.policy, FairnessPolicy::RwWriterPref) {
                    if self.waiters.iter().any(|w| w.hold_type == HoldType::Exclusive) {
                        return false;
                    }
                }
            }
        }
        self.holders.push(LockHolder {
            thread_id,
            hold_type,
            acquire_ns: now_ns,
            hold_duration_ns: 0,
        });
        self.total_acquires += 1;
        true
    }

    pub fn enqueue(&mut self, thread_id: u64, hold_type: HoldType, priority: u32, now_ns: u64) -> u64 {
        let ticket = self.next_ticket;
        self.next_ticket += 1;
        self.waiters.push(LockWaiter::new(thread_id, hold_type, priority, ticket, now_ns));
        self.total_waits += 1;
        ticket
    }

    pub fn grant_next(&mut self, now_ns: u64) -> Option<u64> {
        if self.waiters.is_empty() { return None; }

        let idx = match self.policy {
            FairnessPolicy::Fifo | FairnessPolicy::Ticket => 0,
            FairnessPolicy::PriorityAging => {
                self.waiters.iter().enumerate()
                    .max_by_key(|(_, w)| w.effective_priority)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            }
            _ => 0,
        };

        let waiter = &self.waiters[idx];
        // Check compatibility with current holders
        match waiter.hold_type {
            HoldType::Exclusive => {
                if !self.holders.is_empty() { return None; }
            }
            HoldType::Shared => {
                if self.holders.iter().any(|h| h.hold_type == HoldType::Exclusive) { return None; }
            }
        }

        let waiter = self.waiters.remove(idx);
        let wait_time = now_ns.saturating_sub(waiter.enqueue_ns);
        self.total_wait_ns += wait_time;
        if wait_time > self.max_wait_ns {
            self.max_wait_ns = wait_time;
        }
        if wait_time > self.starvation_threshold_ns {
            self.starvation_events += 1;
        }

        let tid = waiter.thread_id;
        self.holders.push(LockHolder {
            thread_id: tid,
            hold_type: waiter.hold_type,
            acquire_ns: now_ns,
            hold_duration_ns: 0,
        });
        self.total_acquires += 1;
        self.now_serving = waiter.ticket;
        Some(tid)
    }

    pub fn release(&mut self, thread_id: u64, now_ns: u64) -> bool {
        if let Some(pos) = self.holders.iter().position(|h| h.thread_id == thread_id) {
            let holder = self.holders.remove(pos);
            let hold_time = now_ns.saturating_sub(holder.acquire_ns);
            self.total_hold_ns += hold_time;
            if hold_time > self.max_hold_ns {
                self.max_hold_ns = hold_time;
            }
            self.total_releases += 1;
            true
        } else {
            false
        }
    }

    pub fn age_waiters(&mut self, now_ns: u64) {
        for w in &mut self.waiters {
            if w.wait_ns(now_ns) > self.starvation_threshold_ns / 2 {
                w.bump_priority();
            }
        }
    }

    pub fn waiter_count(&self) -> usize {
        self.waiters.len()
    }

    pub fn holder_count(&self) -> usize {
        self.holders.len()
    }

    pub fn is_held(&self) -> bool {
        !self.holders.is_empty()
    }

    pub fn contention_level(&self) -> f64 {
        if self.total_acquires == 0 { return 0.0; }
        self.total_waits as f64 / self.total_acquires as f64
    }

    pub fn avg_wait_ns(&self) -> f64 {
        if self.total_waits == 0 { return 0.0; }
        self.total_wait_ns as f64 / self.total_waits as f64
    }

    pub fn avg_hold_ns(&self) -> f64 {
        if self.total_releases == 0 { return 0.0; }
        self.total_hold_ns as f64 / self.total_releases as f64
    }
}

/// Fair lock stats
#[derive(Debug, Clone)]
pub struct FairLockStats {
    pub total_locks: u64,
    pub total_acquires: u64,
    pub total_releases: u64,
    pub total_starvations: u64,
    pub avg_contention: f64,
}

/// Main fair lock manager
pub struct CoopFairLock {
    locks: BTreeMap<u64, FairLock>,
    next_id: u64,
    stats: FairLockStats,
}

impl CoopFairLock {
    pub fn new() -> Self {
        Self {
            locks: BTreeMap::new(),
            next_id: 1,
            stats: FairLockStats {
                total_locks: 0,
                total_acquires: 0,
                total_releases: 0,
                total_starvations: 0,
                avg_contention: 0.0,
            },
        }
    }

    pub fn create_lock(&mut self, policy: FairnessPolicy) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.locks.insert(id, FairLock::new(id, policy));
        self.stats.total_locks += 1;
        id
    }

    pub fn try_acquire(&mut self, lock_id: u64, thread_id: u64, hold_type: HoldType, priority: u32, now_ns: u64) -> bool {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            let result = lock.try_acquire(thread_id, hold_type, priority, now_ns);
            if result {
                self.stats.total_acquires += 1;
            }
            result
        } else {
            false
        }
    }

    pub fn enqueue_waiter(&mut self, lock_id: u64, thread_id: u64, hold_type: HoldType, priority: u32, now_ns: u64) -> Option<u64> {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            Some(lock.enqueue(thread_id, hold_type, priority, now_ns))
        } else {
            None
        }
    }

    pub fn release(&mut self, lock_id: u64, thread_id: u64, now_ns: u64) -> bool {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            let released = lock.release(thread_id, now_ns);
            if released {
                self.stats.total_releases += 1;
                // Try to grant next waiter
                lock.grant_next(now_ns);
            }
            released
        } else {
            false
        }
    }

    pub fn age_all_waiters(&mut self, now_ns: u64) {
        for lock in self.locks.values_mut() {
            lock.age_waiters(now_ns);
        }
    }

    pub fn most_contended(&self, top: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<(u64, f64)> = self.locks.iter()
            .map(|(&id, l)| (id, l.contention_level()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(top);
        v
    }

    pub fn get_lock(&self, id: u64) -> Option<&FairLock> {
        self.locks.get(&id)
    }

    pub fn stats(&self) -> &FairLockStats {
        &self.stats
    }
}

// ============================================================================
// Merged from fair_lock_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McsNodeState {
    Waiting,
    Granted,
    Released,
}

/// MCS queue node
#[derive(Debug)]
pub struct McsNode {
    pub thread_id: u64,
    pub state: McsNodeState,
    pub next: Option<u64>,
    pub spin_count: u64,
    pub wait_start: u64,
    pub wait_end: u64,
}

impl McsNode {
    pub fn new(tid: u64, now: u64) -> Self {
        Self { thread_id: tid, state: McsNodeState::Waiting, next: None, spin_count: 0, wait_start: now, wait_end: 0 }
    }

    pub fn grant(&mut self, now: u64) { self.state = McsNodeState::Granted; self.wait_end = now; }
    pub fn wait_time(&self) -> u64 { if self.wait_end > 0 { self.wait_end - self.wait_start } else { 0 } }
}

/// Fair lock v2
#[derive(Debug)]
pub struct FairLockV2 {
    pub id: u64,
    pub owner: Option<u64>,
    pub tail: Option<u64>,
    pub nodes: BTreeMap<u64, McsNode>,
    pub acquisitions: u64,
    pub contentions: u64,
    pub total_wait_ns: u64,
    pub max_wait_ns: u64,
}

impl FairLockV2 {
    pub fn new(id: u64) -> Self {
        Self { id, owner: None, tail: None, nodes: BTreeMap::new(), acquisitions: 0, contentions: 0, total_wait_ns: 0, max_wait_ns: 0 }
    }

    pub fn lock(&mut self, tid: u64, now: u64) -> bool {
        if self.owner.is_none() {
            self.owner = Some(tid); self.acquisitions += 1; return true;
        }
        self.contentions += 1;
        let node = McsNode::new(tid, now);
        if let Some(tail_tid) = self.tail {
            if let Some(tail_node) = self.nodes.get_mut(&tail_tid) { tail_node.next = Some(tid); }
        }
        self.tail = Some(tid);
        self.nodes.insert(tid, node);
        false
    }

    pub fn unlock(&mut self, now: u64) {
        if let Some(_owner) = self.owner.take() {
            if let Some((&first_tid, _)) = self.nodes.iter().next() {
                if let Some(node) = self.nodes.get_mut(&first_tid) {
                    node.grant(now);
                    let wait = node.wait_time();
                    self.total_wait_ns += wait;
                    if wait > self.max_wait_ns { self.max_wait_ns = wait; }
                }
                self.owner = Some(first_tid);
                self.acquisitions += 1;
                self.nodes.remove(&first_tid);
            }
        }
    }

    pub fn avg_wait(&self) -> u64 { if self.acquisitions == 0 { 0 } else { self.total_wait_ns / self.acquisitions } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct FairLockV2Stats {
    pub total_locks: u32,
    pub total_acquisitions: u64,
    pub total_contentions: u64,
    pub avg_wait_ns: u64,
    pub max_wait_ns: u64,
}

/// Main fair lock v2 manager
pub struct CoopFairLockV2 {
    locks: BTreeMap<u64, FairLockV2>,
    next_id: u64,
}

impl CoopFairLockV2 {
    pub fn new() -> Self { Self { locks: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.locks.insert(id, FairLockV2::new(id));
        id
    }

    pub fn stats(&self) -> FairLockV2Stats {
        let acqs: u64 = self.locks.values().map(|l| l.acquisitions).sum();
        let conts: u64 = self.locks.values().map(|l| l.contentions).sum();
        let waits: Vec<u64> = self.locks.values().map(|l| l.avg_wait()).collect();
        let avg = if waits.is_empty() { 0 } else { waits.iter().sum::<u64>() / waits.len() as u64 };
        let max = self.locks.values().map(|l| l.max_wait_ns).max().unwrap_or(0);
        FairLockV2Stats { total_locks: self.locks.len() as u32, total_acquisitions: acqs, total_contentions: conts, avg_wait_ns: avg, max_wait_ns: max }
    }
}
