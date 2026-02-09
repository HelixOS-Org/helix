// SPDX-License-Identifier: GPL-2.0
//! Coop lease_mgr — cooperative resource lease management with expiry.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Lease type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseType {
    Exclusive,
    Shared,
    ReadOnly,
    WriteOnly,
    Timed,
}

/// Lease state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseState {
    Active,
    Expired,
    Revoked,
    Suspended,
    Renewing,
}

/// Lease holder identity
#[derive(Debug, Clone, Copy)]
pub struct LeaseHolder {
    pub pid: u32,
    pub tid: u32,
    pub priority: u32,
}

/// Resource lease
#[derive(Debug, Clone)]
pub struct ResourceLease {
    pub id: u64,
    pub resource_id: u64,
    pub lease_type: LeaseType,
    pub state: LeaseState,
    pub holder: LeaseHolder,
    pub granted_at: u64,
    pub expires_at: u64,
    pub duration_ns: u64,
    pub renewals: u32,
    pub max_renewals: u32,
    pub auto_renew: bool,
}

impl ResourceLease {
    pub fn new(id: u64, resource_id: u64, ltype: LeaseType, holder: LeaseHolder,
               duration: u64, now: u64) -> Self {
        Self {
            id, resource_id, lease_type: ltype, state: LeaseState::Active,
            holder, granted_at: now, expires_at: now + duration,
            duration_ns: duration, renewals: 0, max_renewals: 10,
            auto_renew: false,
        }
    }

    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }

    #[inline(always)]
    pub fn remaining_ns(&self, now: u64) -> u64 {
        self.expires_at.saturating_sub(now)
    }

    #[inline]
    pub fn renew(&mut self, now: u64) -> bool {
        if self.state != LeaseState::Active { return false; }
        if self.renewals >= self.max_renewals { return false; }
        self.renewals += 1;
        self.expires_at = now + self.duration_ns;
        true
    }

    #[inline(always)]
    pub fn revoke(&mut self) {
        self.state = LeaseState::Revoked;
    }

    #[inline(always)]
    pub fn suspend(&mut self) {
        self.state = LeaseState::Suspended;
    }

    #[inline(always)]
    pub fn held_time(&self, now: u64) -> u64 {
        now.saturating_sub(self.granted_at)
    }

    #[inline]
    pub fn utilization(&self, now: u64) -> f64 {
        let elapsed = self.held_time(now);
        let total = self.duration_ns * (self.renewals as u64 + 1);
        if total == 0 { return 0.0; }
        elapsed as f64 / total as f64
    }
}

/// Lease request (pending)
#[derive(Debug, Clone)]
pub struct LeaseRequest {
    pub requester: LeaseHolder,
    pub resource_id: u64,
    pub lease_type: LeaseType,
    pub duration_ns: u64,
    pub requested_at: u64,
    pub priority: u32,
}

/// Resource lease state
#[derive(Debug)]
#[repr(align(64))]
pub struct ResourceLeaseState {
    pub resource_id: u64,
    pub active_leases: Vec<ResourceLease>,
    pub pending_requests: Vec<LeaseRequest>,
    pub max_shared: u32,
    pub total_granted: u64,
    pub total_denied: u64,
    pub total_expired: u64,
}

impl ResourceLeaseState {
    pub fn new(resource_id: u64, max_shared: u32) -> Self {
        Self {
            resource_id, active_leases: Vec::new(),
            pending_requests: Vec::new(), max_shared,
            total_granted: 0, total_denied: 0, total_expired: 0,
        }
    }

    #[inline(always)]
    pub fn has_exclusive(&self) -> bool {
        self.active_leases.iter().any(|l| l.lease_type == LeaseType::Exclusive && l.state == LeaseState::Active)
    }

    #[inline]
    pub fn shared_count(&self) -> u32 {
        self.active_leases.iter()
            .filter(|l| l.lease_type == LeaseType::Shared && l.state == LeaseState::Active)
            .count() as u32
    }

    #[inline]
    pub fn can_grant(&self, ltype: LeaseType) -> bool {
        match ltype {
            LeaseType::Exclusive | LeaseType::WriteOnly => {
                self.active_leases.iter().all(|l| l.state != LeaseState::Active)
            }
            LeaseType::Shared | LeaseType::ReadOnly | LeaseType::Timed => {
                !self.has_exclusive() && self.shared_count() < self.max_shared
            }
        }
    }

    pub fn expire_leases(&mut self, now: u64) -> u32 {
        let mut expired = 0u32;
        for lease in &mut self.active_leases {
            if lease.state == LeaseState::Active && lease.is_expired(now) {
                if lease.auto_renew && lease.renewals < lease.max_renewals {
                    lease.renew(now);
                } else {
                    lease.state = LeaseState::Expired;
                    expired += 1;
                }
            }
        }
        self.active_leases.retain(|l| l.state == LeaseState::Active);
        self.total_expired += expired as u64;
        expired
    }
}

/// Lease manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LeaseMgrStats {
    pub tracked_resources: u32,
    pub active_leases: u64,
    pub pending_requests: u64,
    pub total_granted: u64,
    pub total_denied: u64,
    pub total_expired: u64,
    pub total_revoked: u64,
}

/// Main lease manager
pub struct CoopLeaseMgr {
    resources: BTreeMap<u64, ResourceLeaseState>,
    next_lease_id: u64,
    total_granted: u64,
    total_denied: u64,
    total_revoked: u64,
    default_max_shared: u32,
}

impl CoopLeaseMgr {
    pub fn new(max_shared: u32) -> Self {
        Self {
            resources: BTreeMap::new(), next_lease_id: 1,
            total_granted: 0, total_denied: 0, total_revoked: 0,
            default_max_shared: max_shared,
        }
    }

    #[inline]
    pub fn ensure_resource(&mut self, resource_id: u64) {
        if !self.resources.contains_key(&resource_id) {
            self.resources.insert(resource_id, ResourceLeaseState::new(resource_id, self.default_max_shared));
        }
    }

    pub fn request_lease(&mut self, holder: LeaseHolder, resource_id: u64, ltype: LeaseType,
                          duration: u64, now: u64) -> Option<u64> {
        self.ensure_resource(resource_id);
        let res = self.resources.get_mut(&resource_id)?;

        if res.can_grant(ltype) {
            let id = self.next_lease_id;
            self.next_lease_id += 1;
            let lease = ResourceLease::new(id, resource_id, ltype, holder, duration, now);
            res.active_leases.push(lease);
            res.total_granted += 1;
            self.total_granted += 1;
            Some(id)
        } else {
            res.total_denied += 1;
            self.total_denied += 1;
            res.pending_requests.push(LeaseRequest {
                requester: holder, resource_id, lease_type: ltype,
                duration_ns: duration, requested_at: now, priority: holder.priority,
            });
            None
        }
    }

    #[inline]
    pub fn release_lease(&mut self, resource_id: u64, lease_id: u64) -> bool {
        if let Some(res) = self.resources.get_mut(&resource_id) {
            let before = res.active_leases.len();
            res.active_leases.retain(|l| l.id != lease_id);
            res.active_leases.len() < before
        } else { false }
    }

    pub fn revoke_lease(&mut self, resource_id: u64, lease_id: u64) -> bool {
        if let Some(res) = self.resources.get_mut(&resource_id) {
            for lease in &mut res.active_leases {
                if lease.id == lease_id {
                    lease.revoke();
                    self.total_revoked += 1;
                    return true;
                }
            }
        }
        false
    }

    #[inline]
    pub fn tick(&mut self, now: u64) -> u32 {
        let mut total_expired = 0u32;
        for res in self.resources.values_mut() {
            total_expired += res.expire_leases(now);
        }
        total_expired
    }

    pub fn stats(&self) -> LeaseMgrStats {
        let active: u64 = self.resources.values().map(|r| r.active_leases.len() as u64).sum();
        let pending: u64 = self.resources.values().map(|r| r.pending_requests.len() as u64).sum();
        let total_expired: u64 = self.resources.values().map(|r| r.total_expired).sum();
        LeaseMgrStats {
            tracked_resources: self.resources.len() as u32,
            active_leases: active, pending_requests: pending,
            total_granted: self.total_granted,
            total_denied: self.total_denied,
            total_expired, total_revoked: self.total_revoked,
        }
    }
}

// ============================================================================
// Merged from lease_mgr_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseState {
    Active,
    Renewing,
    Expiring,
    Expired,
    Revoked,
    Transferring,
}

/// Lease type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseType {
    Exclusive,
    Shared,
    ReadOnly,
    TimeBounded,
    Delegated,
}

/// Lease record
#[derive(Debug, Clone)]
pub struct LeaseRecord {
    pub id: u64,
    pub resource_name: String,
    pub lease_type: LeaseType,
    pub state: LeaseState,
    pub holder_node: u64,
    pub holder_id: u64,
    pub epoch: u64,
    pub grant_ts: u64,
    pub expiry_ts: u64,
    pub last_renewal_ts: u64,
    pub renewal_count: u32,
    pub max_renewals: u32,
    pub ttl_ns: u64,
    pub delegated_from: Option<u64>,
    pub delegated_to: Vec<u64>,
    pub fence_token: u64,
}

impl LeaseRecord {
    pub fn new(id: u64, resource: String, ltype: LeaseType, node: u64, holder: u64, ts: u64, ttl: u64) -> Self {
        Self {
            id, resource_name: resource, lease_type: ltype, state: LeaseState::Active,
            holder_node: node, holder_id: holder, epoch: 1, grant_ts: ts,
            expiry_ts: ts + ttl, last_renewal_ts: ts, renewal_count: 0,
            max_renewals: u32::MAX, ttl_ns: ttl, delegated_from: None,
            delegated_to: Vec::new(), fence_token: id,
        }
    }

    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool { now >= self.expiry_ts }

    #[inline(always)]
    pub fn remaining_ns(&self, now: u64) -> u64 { self.expiry_ts.saturating_sub(now) }

    #[inline(always)]
    pub fn remaining_pct(&self, now: u64) -> f64 {
        if self.ttl_ns == 0 { return 0.0; }
        (self.remaining_ns(now) as f64 / self.ttl_ns as f64) * 100.0
    }

    #[inline]
    pub fn renew(&mut self, now: u64) -> bool {
        if self.state == LeaseState::Revoked || self.state == LeaseState::Expired { return false; }
        if self.renewal_count >= self.max_renewals { return false; }
        self.expiry_ts = now + self.ttl_ns;
        self.last_renewal_ts = now;
        self.renewal_count += 1;
        self.epoch += 1;
        self.state = LeaseState::Active;
        true
    }

    #[inline(always)]
    pub fn revoke(&mut self) {
        self.state = LeaseState::Revoked;
        self.epoch += 1;
    }

    #[inline(always)]
    pub fn delegate(&mut self, to_node: u64) {
        self.delegated_to.push(to_node);
    }

    #[inline(always)]
    pub fn should_renew(&self, now: u64) -> bool {
        self.state == LeaseState::Active && self.remaining_pct(now) < 30.0
    }
}

/// Lease request
#[derive(Debug, Clone)]
pub struct LeaseRequest {
    pub id: u64,
    pub resource_name: String,
    pub lease_type: LeaseType,
    pub requester_node: u64,
    pub requester_id: u64,
    pub requested_ttl_ns: u64,
    pub priority: u32,
    pub timestamp: u64,
}

/// Lease event
#[derive(Debug, Clone)]
pub struct LeaseEvent {
    pub lease_id: u64,
    pub event_type: LeaseEventType,
    pub node_id: u64,
    pub timestamp: u64,
    pub epoch: u64,
}

/// Lease event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseEventType {
    Granted,
    Renewed,
    Expired,
    Revoked,
    Transferred,
    Delegated,
    Fenced,
}

/// Lease manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct LeaseMgrStats {
    pub total_leases: usize,
    pub active_leases: usize,
    pub expired_leases: usize,
    pub revoked_leases: usize,
    pub total_grants: u64,
    pub total_renewals: u64,
    pub total_revocations: u64,
    pub total_expirations: u64,
    pub total_transfers: u64,
    pub pending_requests: usize,
}

/// Coop lease manager
pub struct CoopLeaseMgrV2 {
    leases: BTreeMap<u64, LeaseRecord>,
    pending: Vec<LeaseRequest>,
    events: Vec<LeaseEvent>,
    stats: LeaseMgrStats,
    next_id: u64,
    default_ttl_ns: u64,
}

impl CoopLeaseMgrV2 {
    pub fn new(default_ttl_ns: u64) -> Self {
        Self { leases: BTreeMap::new(), pending: Vec::new(), events: Vec::new(), stats: LeaseMgrStats::default(), next_id: 1, default_ttl_ns }
    }

    fn resource_hash(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        hash
    }

    #[inline]
    pub fn grant(&mut self, resource: String, ltype: LeaseType, node: u64, holder: u64, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        let lease = LeaseRecord::new(id, resource, ltype, node, holder, ts, self.default_ttl_ns);
        self.leases.insert(id, lease);
        self.events.push(LeaseEvent { lease_id: id, event_type: LeaseEventType::Granted, node_id: node, timestamp: ts, epoch: 1 });
        self.stats.total_grants += 1;
        id
    }

    #[inline]
    pub fn renew(&mut self, lease_id: u64, now: u64) -> bool {
        if let Some(lease) = self.leases.get_mut(&lease_id) {
            if lease.renew(now) {
                self.events.push(LeaseEvent { lease_id, event_type: LeaseEventType::Renewed, node_id: lease.holder_node, timestamp: now, epoch: lease.epoch });
                self.stats.total_renewals += 1;
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn revoke(&mut self, lease_id: u64, now: u64) {
        if let Some(lease) = self.leases.get_mut(&lease_id) {
            let node = lease.holder_node;
            let epoch = lease.epoch + 1;
            lease.revoke();
            self.events.push(LeaseEvent { lease_id, event_type: LeaseEventType::Revoked, node_id: node, timestamp: now, epoch });
            self.stats.total_revocations += 1;
        }
    }

    pub fn transfer(&mut self, lease_id: u64, new_node: u64, new_holder: u64, now: u64) -> bool {
        if let Some(lease) = self.leases.get_mut(&lease_id) {
            if lease.state != LeaseState::Active { return false; }
            lease.holder_node = new_node;
            lease.holder_id = new_holder;
            lease.epoch += 1;
            lease.expiry_ts = now + lease.ttl_ns;
            lease.state = LeaseState::Active;
            self.events.push(LeaseEvent { lease_id, event_type: LeaseEventType::Transferred, node_id: new_node, timestamp: now, epoch: lease.epoch });
            self.stats.total_transfers += 1;
            return true;
        }
        false
    }

    #[inline]
    pub fn expire_stale(&mut self, now: u64) {
        for lease in self.leases.values_mut() {
            if lease.state == LeaseState::Active && lease.is_expired(now) {
                lease.state = LeaseState::Expired;
                self.stats.total_expirations += 1;
            }
        }
    }

    #[inline(always)]
    pub fn find_renewable(&self, now: u64) -> Vec<u64> {
        self.leases.iter().filter(|(_, l)| l.should_renew(now)).map(|(&id, _)| id).collect()
    }

    #[inline(always)]
    pub fn request(&mut self, resource: String, ltype: LeaseType, node: u64, requester: u64, ttl: u64, priority: u32, ts: u64) {
        let id = self.next_id; self.next_id += 1;
        self.pending.push(LeaseRequest { id, resource_name: resource, lease_type: ltype, requester_node: node, requester_id: requester, requested_ttl_ns: ttl, priority, timestamp: ts });
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.total_leases = self.leases.len();
        self.stats.active_leases = self.leases.values().filter(|l| l.state == LeaseState::Active).count();
        self.stats.expired_leases = self.leases.values().filter(|l| l.state == LeaseState::Expired).count();
        self.stats.revoked_leases = self.leases.values().filter(|l| l.state == LeaseState::Revoked).count();
        self.stats.pending_requests = self.pending.len();
    }

    #[inline(always)]
    pub fn lease(&self, id: u64) -> Option<&LeaseRecord> { self.leases.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &LeaseMgrStats { &self.stats }
}
