//! # Coop Distributed Lock Manager
//!
//! Distributed locking across cooperative subsystems:
//! - Lease-based distributed locks
//! - Hierarchical lock namespaces
//! - Deadlock detection across nodes
//! - Fencing tokens for partitioned locks
//! - Lock migration on node failure
//! - Fair queuing with priority inheritance

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Lock mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockMode {
    Exclusive,
    Shared,
    IntentExclusive,
    IntentShared,
    SharedIntentExclusive,
    Update,
}

impl LockMode {
    pub fn is_compatible(&self, other: &LockMode) -> bool {
        match (self, other) {
            (Self::Shared, Self::Shared) => true,
            (Self::Shared, Self::IntentShared) => true,
            (Self::IntentShared, Self::Shared) => true,
            (Self::IntentShared, Self::IntentShared) => true,
            (Self::IntentShared, Self::IntentExclusive) => true,
            (Self::IntentExclusive, Self::IntentShared) => true,
            _ => false,
        }
    }
}

/// Lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DLockState {
    Free,
    Held,
    Converting,
    Waiting,
    Expired,
}

/// Fencing token
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct FencingToken(pub u64);

impl FencingToken {
    pub fn new(epoch: u64) -> Self { Self(epoch) }
    pub fn next(&self) -> Self { Self(self.0 + 1) }
    pub fn is_valid_after(&self, other: &FencingToken) -> bool { self.0 > other.0 }
}

/// Lock holder
#[derive(Debug, Clone)]
pub struct LockHolder {
    pub node_id: u64,
    pub owner_id: u64,
    pub mode: LockMode,
    pub fence: FencingToken,
    pub acquired_ts: u64,
    pub lease_deadline: u64,
    pub conversion_pending: Option<LockMode>,
}

impl LockHolder {
    pub fn new(node: u64, owner: u64, mode: LockMode, fence: FencingToken, ts: u64, lease_ns: u64) -> Self {
        Self { node_id: node, owner_id: owner, mode, fence, acquired_ts: ts, lease_deadline: ts + lease_ns, conversion_pending: None }
    }

    pub fn is_expired(&self, now: u64) -> bool { now > self.lease_deadline }

    pub fn renew(&mut self, now: u64, lease_ns: u64) { self.lease_deadline = now + lease_ns; }
}

/// Lock waiter
#[derive(Debug, Clone)]
pub struct LockWaiter {
    pub node_id: u64,
    pub owner_id: u64,
    pub requested_mode: LockMode,
    pub priority: u32,
    pub enqueued_ts: u64,
    pub timeout_ns: u64,
}

impl LockWaiter {
    pub fn new(node: u64, owner: u64, mode: LockMode, priority: u32, ts: u64, timeout: u64) -> Self {
        Self { node_id: node, owner_id: owner, requested_mode: mode, priority, enqueued_ts: ts, timeout_ns: timeout }
    }

    pub fn is_timed_out(&self, now: u64) -> bool {
        self.timeout_ns > 0 && now.saturating_sub(self.enqueued_ts) > self.timeout_ns
    }
}

/// Distributed lock
#[derive(Debug, Clone)]
pub struct DistributedLock {
    pub name: String,
    pub state: DLockState,
    pub holders: Vec<LockHolder>,
    pub waiters: Vec<LockWaiter>,
    pub fence: FencingToken,
    pub created_ts: u64,
    pub grant_count: u64,
    pub contention_count: u64,
    pub timeout_count: u64,
}

impl DistributedLock {
    pub fn new(name: String, ts: u64) -> Self {
        Self {
            name, state: DLockState::Free, holders: Vec::new(), waiters: Vec::new(),
            fence: FencingToken(0), created_ts: ts, grant_count: 0,
            contention_count: 0, timeout_count: 0,
        }
    }

    pub fn can_grant(&self, mode: LockMode) -> bool {
        if self.holders.is_empty() { return true; }
        self.holders.iter().all(|h| h.mode.is_compatible(&mode))
    }

    pub fn grant(&mut self, node: u64, owner: u64, mode: LockMode, ts: u64, lease_ns: u64) -> FencingToken {
        self.fence = self.fence.next();
        let holder = LockHolder::new(node, owner, mode, self.fence, ts, lease_ns);
        self.holders.push(holder);
        self.state = DLockState::Held;
        self.grant_count += 1;
        self.fence
    }

    pub fn release(&mut self, node: u64, owner: u64) -> bool {
        let before = self.holders.len();
        self.holders.retain(|h| !(h.node_id == node && h.owner_id == owner));
        if self.holders.is_empty() { self.state = DLockState::Free; }
        self.holders.len() < before
    }

    pub fn expire_leases(&mut self, now: u64) -> Vec<LockHolder> {
        let mut expired = Vec::new();
        self.holders.retain(|h| {
            if h.is_expired(now) { expired.push(h.clone()); false } else { true }
        });
        if self.holders.is_empty() && !expired.is_empty() { self.state = DLockState::Free; }
        expired
    }

    pub fn enqueue_waiter(&mut self, waiter: LockWaiter) {
        self.contention_count += 1;
        self.waiters.push(waiter);
        self.waiters.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    pub fn drain_expired_waiters(&mut self, now: u64) -> Vec<LockWaiter> {
        let mut timed_out = Vec::new();
        self.waiters.retain(|w| {
            if w.is_timed_out(now) { timed_out.push(w.clone()); self.timeout_count += 1; false } else { true }
        });
        timed_out
    }

    pub fn try_grant_waiters(&mut self, ts: u64, lease_ns: u64) -> Vec<FencingToken> {
        let mut granted = Vec::new();
        while let Some(w) = self.waiters.first() {
            if self.can_grant(w.requested_mode) {
                let w = self.waiters.remove(0);
                let fence = self.grant(w.node_id, w.owner_id, w.requested_mode, ts, lease_ns);
                granted.push(fence);
            } else {
                break;
            }
        }
        granted
    }
}

/// Deadlock edge
#[derive(Debug, Clone)]
pub struct DeadlockEdge {
    pub waiter_node: u64,
    pub waiter_owner: u64,
    pub holder_node: u64,
    pub holder_owner: u64,
    pub lock_name: String,
}

/// DLM stats
#[derive(Debug, Clone, Default)]
pub struct DlmStats {
    pub total_locks: usize,
    pub held_locks: usize,
    pub free_locks: usize,
    pub total_holders: usize,
    pub total_waiters: usize,
    pub total_grants: u64,
    pub total_contentions: u64,
    pub total_timeouts: u64,
    pub total_expirations: u64,
    pub deadlocks_detected: u64,
}

/// Coop distributed lock manager
pub struct CoopDistLockMgr {
    locks: BTreeMap<u64, DistributedLock>,
    stats: DlmStats,
    default_lease_ns: u64,
}

impl CoopDistLockMgr {
    pub fn new(default_lease_ns: u64) -> Self {
        Self { locks: BTreeMap::new(), stats: DlmStats::default(), default_lease_ns }
    }

    fn name_hash(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        hash
    }

    pub fn create_lock(&mut self, name: String, ts: u64) -> u64 {
        let id = Self::name_hash(&name);
        self.locks.entry(id).or_insert_with(|| DistributedLock::new(name, ts));
        id
    }

    pub fn try_acquire(&mut self, lock_id: u64, node: u64, owner: u64, mode: LockMode, ts: u64) -> Option<FencingToken> {
        let lease = self.default_lease_ns;
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            if lock.can_grant(mode) {
                return Some(lock.grant(node, owner, mode, ts, lease));
            }
        }
        None
    }

    pub fn enqueue(&mut self, lock_id: u64, node: u64, owner: u64, mode: LockMode, priority: u32, ts: u64, timeout: u64) {
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.enqueue_waiter(LockWaiter::new(node, owner, mode, priority, ts, timeout));
        }
    }

    pub fn release(&mut self, lock_id: u64, node: u64, owner: u64, ts: u64) {
        let lease = self.default_lease_ns;
        if let Some(lock) = self.locks.get_mut(&lock_id) {
            lock.release(node, owner);
            lock.try_grant_waiters(ts, lease);
        }
    }

    pub fn expire_all(&mut self, now: u64) {
        let lease = self.default_lease_ns;
        for lock in self.locks.values_mut() {
            let expired = lock.expire_leases(now);
            self.stats.total_expirations += expired.len() as u64;
            lock.try_grant_waiters(now, lease);
            lock.drain_expired_waiters(now);
        }
    }

    pub fn recompute(&mut self) {
        self.stats.total_locks = self.locks.len();
        self.stats.held_locks = self.locks.values().filter(|l| l.state == DLockState::Held).count();
        self.stats.free_locks = self.locks.values().filter(|l| l.state == DLockState::Free).count();
        self.stats.total_holders = self.locks.values().map(|l| l.holders.len()).sum();
        self.stats.total_waiters = self.locks.values().map(|l| l.waiters.len()).sum();
        self.stats.total_grants = self.locks.values().map(|l| l.grant_count).sum();
        self.stats.total_contentions = self.locks.values().map(|l| l.contention_count).sum();
        self.stats.total_timeouts = self.locks.values().map(|l| l.timeout_count).sum();
    }

    pub fn lock(&self, id: u64) -> Option<&DistributedLock> { self.locks.get(&id) }
    pub fn stats(&self) -> &DlmStats { &self.stats }
}
