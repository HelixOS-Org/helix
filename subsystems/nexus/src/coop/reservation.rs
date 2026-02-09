//! # Cooperative Resource Reservation
//!
//! Advance resource reservation for cooperative scheduling:
//! - Guaranteed resource slots for critical tasks
//! - Reservation admission control
//! - Overcommit management with priorities
//! - Reservation expiry and renewal
//! - Multi-resource composite reservations

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// RESERVATION TYPES
// ============================================================================

/// Reservable resource
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservableResource {
    /// CPU cores/time
    Cpu,
    /// Memory pages
    Memory,
    /// I/O bandwidth (bytes/s)
    IoBandwidth,
    /// Network bandwidth
    NetworkBandwidth,
    /// GPU time
    GpuTime,
    /// File descriptors
    FileDescriptors,
    /// IPC channels
    IpcChannels,
}

/// Reservation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReservationState {
    /// Pending admission
    Pending,
    /// Admitted (guaranteed)
    Admitted,
    /// Active (currently using)
    Active,
    /// Suspended (temporarily yielded)
    Suspended,
    /// Expired
    Expired,
    /// Cancelled
    Cancelled,
    /// Denied (admission control rejected)
    Denied,
}

/// Reservation priority (for overcommit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReservationPriority {
    /// Best effort
    BestEffort,
    /// Standard
    Standard,
    /// Elevated
    Elevated,
    /// Guaranteed
    Guaranteed,
    /// Critical
    Critical,
}

// ============================================================================
// RESERVATION
// ============================================================================

/// Single resource request
#[derive(Debug, Clone)]
pub struct ResourceRequest {
    /// Resource type
    pub resource: ReservableResource,
    /// Minimum amount needed
    pub minimum: u64,
    /// Desired amount
    pub desired: u64,
    /// Maximum useful amount
    pub maximum: u64,
}

/// Granted allocation
#[derive(Debug, Clone)]
pub struct GrantedAllocation {
    /// Resource type
    pub resource: ReservableResource,
    /// Granted amount
    pub granted: u64,
    /// Actually used
    pub used: u64,
}

impl GrantedAllocation {
    /// Utilization ratio
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.granted == 0 {
            return 0.0;
        }
        self.used as f64 / self.granted as f64
    }
}

/// Reservation
#[derive(Debug)]
pub struct Reservation {
    /// Reservation id
    pub id: u64,
    /// Owner process
    pub owner: u64,
    /// State
    pub state: ReservationState,
    /// Priority
    pub priority: ReservationPriority,
    /// Requests
    pub requests: Vec<ResourceRequest>,
    /// Grants
    pub grants: Vec<GrantedAllocation>,
    /// Creation time
    pub created_at: u64,
    /// Start time (when admitted)
    pub start_at: u64,
    /// Expiry time
    pub expires_at: u64,
    /// Renewal count
    pub renewals: u32,
    /// Max renewals
    pub max_renewals: u32,
}

impl Reservation {
    pub fn new(
        id: u64,
        owner: u64,
        priority: ReservationPriority,
        requests: Vec<ResourceRequest>,
        now: u64,
        duration_ns: u64,
    ) -> Self {
        Self {
            id,
            owner,
            state: ReservationState::Pending,
            priority,
            requests,
            grants: Vec::new(),
            created_at: now,
            start_at: 0,
            expires_at: now + duration_ns,
            renewals: 0,
            max_renewals: 3,
        }
    }

    /// Admit reservation with grants
    #[inline]
    pub fn admit(&mut self, grants: Vec<GrantedAllocation>, now: u64) {
        self.state = ReservationState::Admitted;
        self.grants = grants;
        self.start_at = now;
    }

    /// Activate (start using)
    #[inline]
    pub fn activate(&mut self) {
        if self.state == ReservationState::Admitted {
            self.state = ReservationState::Active;
        }
    }

    /// Suspend
    #[inline]
    pub fn suspend(&mut self) {
        if self.state == ReservationState::Active {
            self.state = ReservationState::Suspended;
        }
    }

    /// Resume
    #[inline]
    pub fn resume(&mut self) {
        if self.state == ReservationState::Suspended {
            self.state = ReservationState::Active;
        }
    }

    /// Renew
    #[inline]
    pub fn renew(&mut self, additional_ns: u64) -> bool {
        if self.renewals >= self.max_renewals {
            return false;
        }
        self.renewals += 1;
        self.expires_at += additional_ns;
        true
    }

    /// Cancel
    #[inline(always)]
    pub fn cancel(&mut self) {
        self.state = ReservationState::Cancelled;
    }

    /// Check expiry
    #[inline]
    pub fn check_expiry(&mut self, now: u64) -> bool {
        if now >= self.expires_at
            && self.state != ReservationState::Expired
            && self.state != ReservationState::Cancelled
        {
            self.state = ReservationState::Expired;
            true
        } else {
            false
        }
    }

    /// Average utilization across grants
    #[inline]
    pub fn avg_utilization(&self) -> f64 {
        if self.grants.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.grants.iter().map(|g| g.utilization()).sum();
        sum / self.grants.len() as f64
    }

    /// Is all minimum satisfied?
    pub fn minimums_met(&self) -> bool {
        for req in &self.requests {
            let granted = self
                .grants
                .iter()
                .filter(|g| g.resource == req.resource)
                .map(|g| g.granted)
                .sum::<u64>();
            if granted < req.minimum {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// ADMISSION CONTROL
// ============================================================================

/// Resource capacity
#[derive(Debug, Clone)]
pub struct ResourceCapacity {
    /// Resource type
    pub resource: ReservableResource,
    /// Total capacity
    pub total: u64,
    /// Reserved (committed)
    pub reserved: u64,
    /// Overcommit ratio (e.g. 1.5 = 150%)
    pub overcommit_ratio: f64,
}

impl ResourceCapacity {
    pub fn new(resource: ReservableResource, total: u64) -> Self {
        Self {
            resource,
            total,
            reserved: 0,
            overcommit_ratio: 1.0,
        }
    }

    /// Effective capacity (with overcommit)
    #[inline(always)]
    pub fn effective_capacity(&self) -> u64 {
        (self.total as f64 * self.overcommit_ratio) as u64
    }

    /// Available
    #[inline(always)]
    pub fn available(&self) -> u64 {
        self.effective_capacity().saturating_sub(self.reserved)
    }

    /// Can admit amount?
    #[inline(always)]
    pub fn can_admit(&self, amount: u64) -> bool {
        amount <= self.available()
    }

    /// Reserve
    #[inline(always)]
    pub fn reserve(&mut self, amount: u64) {
        self.reserved += amount;
    }

    /// Release
    #[inline(always)]
    pub fn release(&mut self, amount: u64) {
        self.reserved = self.reserved.saturating_sub(amount);
    }

    /// Utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        self.reserved as f64 / self.total as f64
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Reservation stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopReservationStats {
    /// Total reservations
    pub total_reservations: u64,
    /// Active reservations
    pub active_count: usize,
    /// Admitted reservations
    pub admitted_count: usize,
    /// Denied reservations
    pub denied_count: u64,
    /// Expired reservations
    pub expired_count: u64,
}

/// Cooperative reservation manager
pub struct CoopReservationManager {
    /// Reservations
    reservations: BTreeMap<u64, Reservation>,
    /// Resource capacities
    capacities: BTreeMap<u8, ResourceCapacity>,
    /// Next reservation id
    next_id: u64,
    /// Stats
    stats: CoopReservationStats,
}

impl CoopReservationManager {
    pub fn new() -> Self {
        Self {
            reservations: BTreeMap::new(),
            capacities: BTreeMap::new(),
            next_id: 1,
            stats: CoopReservationStats::default(),
        }
    }

    /// Set capacity
    #[inline(always)]
    pub fn set_capacity(&mut self, capacity: ResourceCapacity) {
        self.capacities.insert(capacity.resource as u8, capacity);
    }

    /// Submit reservation
    pub fn submit(
        &mut self,
        owner: u64,
        priority: ReservationPriority,
        requests: Vec<ResourceRequest>,
        now: u64,
        duration_ns: u64,
    ) -> (u64, ReservationState) {
        let id = self.next_id;
        self.next_id += 1;
        self.stats.total_reservations += 1;

        let mut reservation =
            Reservation::new(id, owner, priority, requests.clone(), now, duration_ns);

        // Admission control
        let mut grants = Vec::new();
        let mut can_admit = true;
        for req in &requests {
            let key = req.resource as u8;
            if let Some(cap) = self.capacities.get(&key) {
                if cap.can_admit(req.minimum) {
                    let grant_amount = if cap.can_admit(req.desired) {
                        req.desired
                    } else {
                        req.minimum
                    };
                    grants.push(GrantedAllocation {
                        resource: req.resource,
                        granted: grant_amount,
                        used: 0,
                    });
                } else {
                    can_admit = false;
                    break;
                }
            }
        }

        if can_admit {
            // Commit reservations
            for grant in &grants {
                let key = grant.resource as u8;
                if let Some(cap) = self.capacities.get_mut(&key) {
                    cap.reserve(grant.granted);
                }
            }
            reservation.admit(grants, now);
        } else {
            reservation.state = ReservationState::Denied;
            self.stats.denied_count += 1;
        }

        let state = reservation.state;
        self.reservations.insert(id, reservation);
        self.update_stats();
        (id, state)
    }

    /// Check expiry on all reservations
    pub fn check_expiry(&mut self, now: u64) {
        let ids: Vec<u64> = self.reservations.keys().copied().collect();
        for id in ids {
            if let Some(res) = self.reservations.get_mut(&id) {
                if res.check_expiry(now) {
                    // Release capacity
                    for grant in &res.grants {
                        let key = grant.resource as u8;
                        if let Some(cap) = self.capacities.get_mut(&key) {
                            cap.release(grant.granted);
                        }
                    }
                    self.stats.expired_count += 1;
                }
            }
        }
        self.update_stats();
    }

    /// Cancel reservation
    pub fn cancel(&mut self, id: u64) -> bool {
        if let Some(res) = self.reservations.get_mut(&id) {
            for grant in &res.grants {
                let key = grant.resource as u8;
                if let Some(cap) = self.capacities.get_mut(&key) {
                    cap.release(grant.granted);
                }
            }
            res.cancel();
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Get reservation
    #[inline(always)]
    pub fn reservation(&self, id: u64) -> Option<&Reservation> {
        self.reservations.get(&id)
    }

    fn update_stats(&mut self) {
        self.stats.active_count = self
            .reservations
            .values()
            .filter(|r| r.state == ReservationState::Active)
            .count();
        self.stats.admitted_count = self
            .reservations
            .values()
            .filter(|r| {
                r.state == ReservationState::Admitted || r.state == ReservationState::Active
            })
            .count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopReservationStats {
        &self.stats
    }
}
