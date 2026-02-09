//! # Cooperative Lease Protocol
//!
//! Time-bounded resource leasing between processes:
//! - Lease creation and renewal
//! - Automatic expiration
//! - Lease transfer
//! - Conflict resolution
//! - Fair queuing for contested leases

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::vec::Vec;

use crate::fast::linear_map::LinearMap;

// ============================================================================
// LEASE TYPES
// ============================================================================

/// Leased resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeaseResource {
    /// CPU core affinity
    CpuCore,
    /// Memory region
    MemoryRegion,
    /// I/O device
    IoDevice,
    /// Network port
    NetworkPort,
    /// File lock
    FileLock,
    /// GPU context
    GpuContext,
    /// Cache partition
    CachePartition,
}

/// Lease state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseState {
    /// Active
    Active,
    /// Grace period (expired but not yet reclaimed)
    GracePeriod,
    /// Expired
    Expired,
    /// Revoked
    Revoked,
    /// Transferred
    Transferred,
}

/// Lease priority for conflict resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeasePriority {
    /// Best effort
    BestEffort,
    /// Normal
    Normal,
    /// High
    High,
    /// Exclusive
    Exclusive,
}

// ============================================================================
// LEASE
// ============================================================================

/// A resource lease
#[derive(Debug, Clone)]
pub struct Lease {
    /// Lease id
    pub id: u64,
    /// Lessee (holder)
    pub lessee: u64,
    /// Resource type
    pub resource: LeaseResource,
    /// Resource identifier
    pub resource_id: u64,
    /// State
    pub state: LeaseState,
    /// Priority
    pub priority: LeasePriority,
    /// Created at
    pub created_at: u64,
    /// Expires at
    pub expires_at: u64,
    /// Grace period duration (ns after expiry)
    pub grace_ns: u64,
    /// Renewals count
    pub renewals: u32,
    /// Max renewals (0 = unlimited)
    pub max_renewals: u32,
    /// Duration per lease term (ns)
    pub term_ns: u64,
}

impl Lease {
    pub fn new(
        id: u64,
        lessee: u64,
        resource: LeaseResource,
        resource_id: u64,
        term_ns: u64,
        now: u64,
    ) -> Self {
        Self {
            id,
            lessee,
            resource,
            resource_id,
            state: LeaseState::Active,
            priority: LeasePriority::Normal,
            created_at: now,
            expires_at: now + term_ns,
            grace_ns: term_ns / 10, // 10% grace
            renewals: 0,
            max_renewals: 0,
            term_ns,
        }
    }

    /// Renew lease
    pub fn renew(&mut self, now: u64) -> bool {
        if self.max_renewals > 0 && self.renewals >= self.max_renewals {
            return false;
        }
        if self.state == LeaseState::Revoked || self.state == LeaseState::Transferred {
            return false;
        }
        self.expires_at = now + self.term_ns;
        self.state = LeaseState::Active;
        self.renewals += 1;
        true
    }

    /// Check expiration
    pub fn check_expiry(&mut self, now: u64) -> bool {
        if self.state != LeaseState::Active && self.state != LeaseState::GracePeriod {
            return false;
        }
        if now >= self.expires_at + self.grace_ns {
            self.state = LeaseState::Expired;
            return true;
        }
        if now >= self.expires_at && self.state == LeaseState::Active {
            self.state = LeaseState::GracePeriod;
        }
        false
    }

    /// Revoke lease
    #[inline(always)]
    pub fn revoke(&mut self) {
        self.state = LeaseState::Revoked;
    }

    /// Transfer to new lessee
    #[inline]
    pub fn transfer(&mut self, new_lessee: u64, now: u64) {
        self.state = LeaseState::Transferred;
        self.lessee = new_lessee;
        self.state = LeaseState::Active;
        self.expires_at = now + self.term_ns;
    }

    /// Remaining time (ns)
    #[inline(always)]
    pub fn remaining_ns(&self, now: u64) -> u64 {
        self.expires_at.saturating_sub(now)
    }

    /// Is expired (including grace)?
    #[inline(always)]
    pub fn is_expired(&self) -> bool {
        self.state == LeaseState::Expired
    }

    /// Is active (including grace)?
    #[inline(always)]
    pub fn is_active(&self) -> bool {
        self.state == LeaseState::Active || self.state == LeaseState::GracePeriod
    }

    /// Total held duration
    #[inline(always)]
    pub fn held_ns(&self, now: u64) -> u64 {
        now.saturating_sub(self.created_at)
    }
}

// ============================================================================
// LEASE QUEUE
// ============================================================================

/// Queued lease request
#[derive(Debug, Clone)]
pub struct LeaseRequest {
    /// Requester pid
    pub pid: u64,
    /// Resource
    pub resource: LeaseResource,
    /// Resource id
    pub resource_id: u64,
    /// Priority
    pub priority: LeasePriority,
    /// Requested term
    pub term_ns: u64,
    /// Queued at
    pub queued_at: u64,
    /// Timeout
    pub timeout_ns: u64,
}

impl LeaseRequest {
    /// Is expired?
    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        now > self.queued_at + self.timeout_ns
    }

    /// Wait time
    #[inline(always)]
    pub fn wait_ns(&self, now: u64) -> u64 {
        now.saturating_sub(self.queued_at)
    }
}

// ============================================================================
// LEASE MANAGER
// ============================================================================

/// Lease stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopLeaseStats {
    /// Active leases
    pub active: usize,
    /// Queued requests
    pub queued: usize,
    /// Renewals
    pub total_renewals: u64,
    /// Expirations
    pub total_expirations: u64,
}

/// Cooperative lease manager
pub struct CoopLeaseManager {
    /// Active leases
    leases: BTreeMap<u64, Lease>,
    /// Request queue per resource
    queues: BTreeMap<u64, VecDeque<LeaseRequest>>,
    /// Resource -> active lease mapping
    resource_leases: LinearMap<u64, 64>,
    /// Next id
    next_id: u64,
    /// Stats
    stats: CoopLeaseStats,
}

impl CoopLeaseManager {
    pub fn new() -> Self {
        Self {
            leases: BTreeMap::new(),
            queues: BTreeMap::new(),
            resource_leases: LinearMap::new(),
            next_id: 1,
            stats: CoopLeaseStats::default(),
        }
    }

    fn resource_key(resource: LeaseResource, resource_id: u64) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= resource as u64;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= resource_id;
        hash = hash.wrapping_mul(0x100000001b3);
        hash
    }

    /// Acquire lease (or queue if unavailable)
    pub fn acquire(
        &mut self,
        pid: u64,
        resource: LeaseResource,
        resource_id: u64,
        term_ns: u64,
        now: u64,
    ) -> Result<u64, ()> {
        let key = Self::resource_key(resource, resource_id);

        // Check if resource is already leased
        if let Some(&lease_id) = self.resource_leases.get(key) {
            if let Some(lease) = self.leases.get(&lease_id) {
                if lease.is_active() {
                    // Queue request
                    let request = LeaseRequest {
                        pid,
                        resource,
                        resource_id,
                        priority: LeasePriority::Normal,
                        term_ns,
                        queued_at: now,
                        timeout_ns: term_ns * 2,
                    };
                    self.queues
                        .entry(key)
                        .or_insert_with(VecDeque::new)
                        .push_back(request);
                    self.stats.queued = self.queues.values().map(|q| q.len()).sum();
                    return Err(());
                }
            }
        }

        // Grant lease
        let id = self.next_id;
        self.next_id += 1;
        let lease = Lease::new(id, pid, resource, resource_id, term_ns, now);
        self.leases.insert(id, lease);
        self.resource_leases.insert(key, id);
        self.stats.active = self.leases.values().filter(|l| l.is_active()).count();
        Ok(id)
    }

    /// Renew lease
    #[inline]
    pub fn renew(&mut self, lease_id: u64, now: u64) -> bool {
        if let Some(lease) = self.leases.get_mut(&lease_id) {
            let ok = lease.renew(now);
            if ok {
                self.stats.total_renewals += 1;
            }
            ok
        } else {
            false
        }
    }

    /// Release lease
    pub fn release(&mut self, lease_id: u64, now: u64) -> Option<u64> {
        let key = if let Some(lease) = self.leases.get_mut(&lease_id) {
            lease.revoke();
            Some(Self::resource_key(lease.resource, lease.resource_id))
        } else {
            None
        };

        // Try to grant to queued request
        if let Some(key) = key {
            self.resource_leases.remove(key);
            return self.grant_queued(key, now);
        }
        None
    }

    /// Check all expirations and grant queued
    pub fn tick(&mut self, now: u64) -> Vec<u64> {
        let mut expired_keys = Vec::new();
        for lease in self.leases.values_mut() {
            if lease.check_expiry(now) {
                self.stats.total_expirations += 1;
                let key = Self::resource_key(lease.resource, lease.resource_id);
                expired_keys.push(key);
            }
        }

        // Grant queued for expired resources
        let mut granted = Vec::new();
        for key in expired_keys {
            self.resource_leases.remove(key);
            if let Some(new_lease) = self.grant_queued(key, now) {
                granted.push(new_lease);
            }
        }

        // Clean expired queue entries
        for queue in self.queues.values_mut() {
            queue.retain(|r| !r.is_expired(now));
        }

        self.stats.active = self.leases.values().filter(|l| l.is_active()).count();
        self.stats.queued = self.queues.values().map(|q| q.len()).sum();
        granted
    }

    fn grant_queued(&mut self, key: u64, now: u64) -> Option<u64> {
        if let Some(queue) = self.queues.get_mut(&key) {
            // Sort by priority (highest first), then by queued time
            queue.make_contiguous().sort_by(|a, b| {
                b.priority
                    .cmp(&a.priority)
                    .then(a.queued_at.cmp(&b.queued_at))
            });
            if let Some(request) = queue.front() {
                let id = self.next_id;
                self.next_id += 1;
                let lease = Lease::new(
                    id,
                    request.pid,
                    request.resource,
                    request.resource_id,
                    request.term_ns,
                    now,
                );
                self.leases.insert(id, lease);
                self.resource_leases.insert(key, id);
                queue.pop_front();
                return Some(id);
            }
        }
        None
    }

    /// Get lease
    #[inline(always)]
    pub fn lease(&self, id: u64) -> Option<&Lease> {
        self.leases.get(&id)
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopLeaseStats {
        &self.stats
    }
}
