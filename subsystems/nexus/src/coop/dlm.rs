//! # Cooperative Distributed Lock Manager
//!
//! Distributed lock coordination across processes/nodes:
//! - Hierarchical lock domains
//! - Distributed deadlock detection
//! - Lock lease management with expiry
//! - Fairness-aware queueing
//! - Lock migration on process exit
//! - Split-brain recovery

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Distributed lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DlmLockType {
    Exclusive,
    Shared,
    IntentExclusive,
    IntentShared,
    Update,
}

/// Lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DlmLockState {
    Granted,
    Waiting,
    Converting,
    Expired,
    Revoked,
}

/// Lock compatibility matrix result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockCompat {
    Compatible,
    Incompatible,
    Convertible,
}

/// Lock request
#[derive(Debug, Clone)]
pub struct DlmLockRequest {
    pub request_id: u64,
    pub resource_id: u64,
    pub owner_id: u64,
    pub lock_type: DlmLockType,
    pub state: DlmLockState,
    pub timestamp: u64,
    pub lease_ns: u64,
    pub priority: i32,
}

impl DlmLockRequest {
    pub fn new(request_id: u64, resource_id: u64, owner_id: u64, lock_type: DlmLockType) -> Self {
        Self {
            request_id,
            resource_id,
            owner_id,
            lock_type,
            state: DlmLockState::Waiting,
            timestamp: 0,
            lease_ns: 30_000_000_000, // 30 seconds default
            priority: 0,
        }
    }

    #[inline]
    pub fn is_expired(&self, now: u64) -> bool {
        self.state == DlmLockState::Granted
            && self.lease_ns > 0
            && now > self.timestamp + self.lease_ns
    }
}

/// Distributed lock resource
#[derive(Debug, Clone)]
pub struct DlmResource {
    pub resource_id: u64,
    pub granted: Vec<DlmLockRequest>,
    pub waiters: Vec<DlmLockRequest>,
    pub total_grants: u64,
    pub total_waits: u64,
    pub total_deadlocks: u64,
    pub max_queue_depth: u32,
}

impl DlmResource {
    pub fn new(resource_id: u64) -> Self {
        Self {
            resource_id,
            granted: Vec::new(),
            waiters: Vec::new(),
            total_grants: 0,
            total_waits: 0,
            total_deadlocks: 0,
            max_queue_depth: 0,
        }
    }

    /// Check lock compatibility
    #[inline]
    pub fn check_compat(held: DlmLockType, requested: DlmLockType) -> LockCompat {
        match (held, requested) {
            (DlmLockType::Shared, DlmLockType::Shared) => LockCompat::Compatible,
            (DlmLockType::IntentShared, DlmLockType::IntentShared) => LockCompat::Compatible,
            (DlmLockType::IntentShared, DlmLockType::Shared) => LockCompat::Compatible,
            (DlmLockType::Shared, DlmLockType::IntentShared) => LockCompat::Compatible,
            (DlmLockType::Shared, DlmLockType::Update) => LockCompat::Compatible,
            _ => LockCompat::Incompatible,
        }
    }

    #[inline]
    pub fn can_grant(&self, req: &DlmLockRequest) -> bool {
        for held in &self.granted {
            if Self::check_compat(held.lock_type, req.lock_type) == LockCompat::Incompatible {
                return false;
            }
        }
        true
    }

    pub fn try_grant(&mut self, mut req: DlmLockRequest, now: u64) -> bool {
        if self.can_grant(&req) {
            req.state = DlmLockState::Granted;
            req.timestamp = now;
            self.granted.push(req);
            self.total_grants += 1;
            true
        } else {
            req.state = DlmLockState::Waiting;
            req.timestamp = now;
            self.waiters.push(req);
            self.total_waits += 1;
            let depth = self.waiters.len() as u32;
            if depth > self.max_queue_depth { self.max_queue_depth = depth; }
            false
        }
    }

    pub fn release(&mut self, owner_id: u64, now: u64) -> Vec<DlmLockRequest> {
        self.granted.retain(|l| l.owner_id != owner_id);
        // Try granting waiters
        let mut newly_granted = Vec::new();
        let mut remaining = Vec::new();
        for mut waiter in self.waiters.drain(..) {
            if self.can_grant(&waiter) {
                waiter.state = DlmLockState::Granted;
                waiter.timestamp = now;
                newly_granted.push(waiter.clone());
                self.granted.push(waiter);
                self.total_grants += 1;
            } else {
                remaining.push(waiter);
            }
        }
        self.waiters = remaining;
        newly_granted
    }

    /// Expire stale leases
    pub fn expire_leases(&mut self, now: u64) -> Vec<u64> {
        let mut expired_owners = Vec::new();
        let mut remaining = Vec::new();
        for lock in self.granted.drain(..) {
            if lock.is_expired(now) {
                expired_owners.push(lock.owner_id);
            } else {
                remaining.push(lock);
            }
        }
        self.granted = remaining;
        expired_owners
    }
}

/// Distributed deadlock detection edge
#[derive(Debug, Clone)]
pub struct WaitForEdge {
    pub waiter_id: u64,
    pub holder_id: u64,
    pub resource_id: u64,
}

/// Coop DLM stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopDlmStats {
    pub total_resources: usize,
    pub total_granted: usize,
    pub total_waiting: usize,
    pub total_grants: u64,
    pub total_deadlocks: u64,
    pub total_expirations: u64,
}

/// Cooperative Distributed Lock Manager
pub struct CoopDlm {
    resources: BTreeMap<u64, DlmResource>,
    next_request_id: u64,
    stats: CoopDlmStats,
}

impl CoopDlm {
    pub fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            next_request_id: 1,
            stats: CoopDlmStats::default(),
        }
    }

    #[inline]
    pub fn lock(&mut self, resource_id: u64, owner_id: u64, lock_type: DlmLockType, now: u64) -> (u64, bool) {
        let req_id = self.next_request_id;
        self.next_request_id += 1;
        let req = DlmLockRequest::new(req_id, resource_id, owner_id, lock_type);
        let resource = self.resources.entry(resource_id)
            .or_insert_with(|| DlmResource::new(resource_id));
        let granted = resource.try_grant(req, now);
        self.recompute();
        (req_id, granted)
    }

    #[inline]
    pub fn unlock(&mut self, resource_id: u64, owner_id: u64, now: u64) -> Vec<DlmLockRequest> {
        let result = if let Some(resource) = self.resources.get_mut(&resource_id) {
            resource.release(owner_id, now)
        } else { Vec::new() };
        self.recompute();
        result
    }

    pub fn expire_all(&mut self, now: u64) -> Vec<(u64, u64)> {
        let mut expired = Vec::new();
        let ids: Vec<u64> = self.resources.keys().copied().collect();
        for rid in ids {
            if let Some(res) = self.resources.get_mut(&rid) {
                for owner in res.expire_leases(now) {
                    expired.push((rid, owner));
                }
            }
        }
        self.stats.total_expirations += expired.len() as u64;
        if !expired.is_empty() { self.recompute(); }
        expired
    }

    /// Simple deadlock detection via wait-for graph cycle detection
    pub fn detect_deadlocks(&self) -> Vec<Vec<u64>> {
        let mut wait_for: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
        for res in self.resources.values() {
            for waiter in &res.waiters {
                for holder in &res.granted {
                    wait_for.entry(waiter.owner_id)
                        .or_insert_with(Vec::new)
                        .push(holder.owner_id);
                }
            }
        }
        // DFS cycle detection
        let mut cycles = Vec::new();
        let nodes: Vec<u64> = wait_for.keys().copied().collect();
        for start in &nodes {
            let mut stack = alloc::vec![(*start, 0usize)];
            let mut path = alloc::vec![*start];
            let mut visited = alloc::vec![*start];
            while let Some((node, idx)) = stack.last_mut() {
                if let Some(neighbors) = wait_for.get(node) {
                    if *idx < neighbors.len() {
                        let next = neighbors[*idx];
                        *idx += 1;
                        if next == *start && path.len() > 1 {
                            cycles.push(path.clone());
                            break;
                        }
                        if !visited.contains(&next) {
                            visited.push(next);
                            path.push(next);
                            stack.push((next, 0));
                        }
                    } else {
                        path.pop();
                        stack.pop();
                    }
                } else {
                    path.pop();
                    stack.pop();
                }
            }
        }
        cycles
    }

    fn recompute(&mut self) {
        self.stats.total_resources = self.resources.len();
        self.stats.total_granted = self.resources.values().map(|r| r.granted.len()).sum();
        self.stats.total_waiting = self.resources.values().map(|r| r.waiters.len()).sum();
        self.stats.total_grants = self.resources.values().map(|r| r.total_grants).sum();
        self.stats.total_deadlocks = self.resources.values().map(|r| r.total_deadlocks).sum();
    }

    #[inline(always)]
    pub fn resource(&self, id: u64) -> Option<&DlmResource> {
        self.resources.get(&id)
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopDlmStats {
        &self.stats
    }
}
