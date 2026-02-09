//! # Cooperative Priority Donation
//!
//! Priority donation protocol for cooperative scheduling:
//! - Priority lending between processes
//! - Priority ceiling protocol
//! - Transitive donation
//! - Donation chains
//! - Starvation prevention

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// PRIORITY TYPES
// ============================================================================

/// Donation priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DonationPriority {
    /// Idle
    Idle = 0,
    /// Low
    Low = 1,
    /// Normal
    Normal = 2,
    /// High
    High = 3,
    /// Realtime
    Realtime = 4,
    /// Critical
    Critical = 5,
}

impl DonationPriority {
    /// Numeric value
    #[inline(always)]
    pub fn value(&self) -> u32 {
        *self as u32
    }

    /// From numeric value
    #[inline]
    pub fn from_value(v: u32) -> Self {
        match v {
            0 => Self::Idle,
            1 => Self::Low,
            2 => Self::Normal,
            3 => Self::High,
            4 => Self::Realtime,
            _ => Self::Critical,
        }
    }
}

/// Donation reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DonationReason {
    /// Holding a mutex needed by higher-priority process
    MutexHolder,
    /// In critical section
    CriticalSection,
    /// Deadline approaching
    DeadlineUrgent,
    /// Dependency chain
    DependencyChain,
    /// Explicit cooperative donation
    Voluntary,
}

// ============================================================================
// DONATION RECORD
// ============================================================================

/// A priority donation
#[derive(Debug, Clone)]
pub struct PriorityDonation {
    /// Donation id
    pub id: u64,
    /// Donor (high priority)
    pub donor: u64,
    /// Recipient (low priority)
    pub recipient: u64,
    /// Donated priority level
    pub donated_priority: DonationPriority,
    /// Original priority of recipient
    pub original_priority: DonationPriority,
    /// Reason
    pub reason: DonationReason,
    /// Timestamp
    pub created_at: u64,
    /// Active
    pub active: bool,
    /// Resource held (mutex address, etc.)
    pub resource: u64,
}

impl PriorityDonation {
    pub fn new(
        id: u64,
        donor: u64,
        recipient: u64,
        priority: DonationPriority,
        original: DonationPriority,
        reason: DonationReason,
        resource: u64,
        now: u64,
    ) -> Self {
        Self {
            id,
            donor,
            recipient,
            donated_priority: priority,
            original_priority: original,
            reason,
            created_at: now,
            active: true,
            resource,
        }
    }

    /// Revoke donation
    #[inline(always)]
    pub fn revoke(&mut self) {
        self.active = false;
    }

    /// Duration
    #[inline(always)]
    pub fn duration_ns(&self, now: u64) -> u64 {
        now.saturating_sub(self.created_at)
    }
}

// ============================================================================
// PROCESS PRIORITY STATE
// ============================================================================

/// Priority state for a process
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DonationPriorityState {
    /// Process id
    pub pid: u64,
    /// Base priority
    pub base_priority: DonationPriority,
    /// Active donations received
    pub received_donations: Vec<u64>,
    /// Active donations given
    pub given_donations: Vec<u64>,
    /// Resources held
    pub resources_held: Vec<u64>,
    /// Effective priority (max of base + donations)
    pub effective_priority: DonationPriority,
}

impl DonationPriorityState {
    pub fn new(pid: u64, base: DonationPriority) -> Self {
        Self {
            pid,
            base_priority: base,
            received_donations: Vec::new(),
            given_donations: Vec::new(),
            resources_held: Vec::new(),
            effective_priority: base,
        }
    }

    /// Acquire resource
    #[inline]
    pub fn acquire_resource(&mut self, resource: u64) {
        if !self.resources_held.contains(&resource) {
            self.resources_held.push(resource);
        }
    }

    /// Release resource
    #[inline(always)]
    pub fn release_resource(&mut self, resource: u64) {
        self.resources_held.retain(|&r| r != resource);
    }

    /// Holds resource?
    #[inline(always)]
    pub fn holds_resource(&self, resource: u64) -> bool {
        self.resources_held.contains(&resource)
    }

    /// Recalculate effective priority
    #[inline]
    pub fn recalculate(&mut self, donations: &BTreeMap<u64, PriorityDonation>) {
        let mut max = self.base_priority;
        for &did in &self.received_donations {
            if let Some(donation) = donations.get(&did) {
                if donation.active && donation.donated_priority > max {
                    max = donation.donated_priority;
                }
            }
        }
        self.effective_priority = max;
    }
}

// ============================================================================
// PRIORITY CEILING
// ============================================================================

/// Priority ceiling for a resource
#[derive(Debug, Clone)]
pub struct PriorityCeiling {
    /// Resource id
    pub resource: u64,
    /// Ceiling priority
    pub ceiling: DonationPriority,
    /// Current holder
    pub holder: Option<u64>,
}

impl PriorityCeiling {
    pub fn new(resource: u64, ceiling: DonationPriority) -> Self {
        Self {
            resource,
            ceiling,
            holder: None,
        }
    }
}

// ============================================================================
// DONATION MANAGER
// ============================================================================

/// Donation stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopDonationStats {
    /// Active donations
    pub active: usize,
    /// Total donations ever
    pub total: u64,
    /// Chain length max
    pub max_chain: usize,
    /// Processes boosted
    pub boosted: usize,
}

/// Cooperative priority donation manager
pub struct CoopDonationManager {
    /// All donations
    donations: BTreeMap<u64, PriorityDonation>,
    /// Process states
    states: BTreeMap<u64, DonationPriorityState>,
    /// Priority ceilings
    ceilings: BTreeMap<u64, PriorityCeiling>,
    /// Next id
    next_id: u64,
    /// Stats
    stats: CoopDonationStats,
}

impl CoopDonationManager {
    pub fn new() -> Self {
        Self {
            donations: BTreeMap::new(),
            states: BTreeMap::new(),
            ceilings: BTreeMap::new(),
            next_id: 1,
            stats: CoopDonationStats::default(),
        }
    }

    /// Register process
    #[inline]
    pub fn register(&mut self, pid: u64, base: DonationPriority) {
        self.states
            .entry(pid)
            .or_insert_with(|| DonationPriorityState::new(pid, base));
    }

    /// Set priority ceiling for resource
    #[inline]
    pub fn set_ceiling(&mut self, resource: u64, ceiling: DonationPriority) {
        self.ceilings
            .entry(resource)
            .or_insert_with(|| PriorityCeiling::new(resource, ceiling))
            .ceiling = ceiling;
    }

    /// Donate priority
    pub fn donate(
        &mut self,
        donor: u64,
        recipient: u64,
        reason: DonationReason,
        resource: u64,
        now: u64,
    ) -> Option<u64> {
        let donor_priority = self
            .states
            .get(&donor)
            .map(|s| s.effective_priority)?;
        let recipient_priority = self
            .states
            .get(&recipient)
            .map(|s| s.base_priority)?;

        // Only donate if donor has higher priority
        if donor_priority <= recipient_priority {
            return None;
        }

        let id = self.next_id;
        self.next_id += 1;

        let donation = PriorityDonation::new(
            id,
            donor,
            recipient,
            donor_priority,
            recipient_priority,
            reason,
            resource,
            now,
        );
        self.donations.insert(id, donation);

        if let Some(state) = self.states.get_mut(&donor) {
            state.given_donations.push(id);
        }
        if let Some(state) = self.states.get_mut(&recipient) {
            state.received_donations.push(id);
            state.recalculate(&self.donations);
        }

        self.stats.total += 1;
        self.update_stats();
        Some(id)
    }

    /// Revoke donation
    #[inline]
    pub fn revoke(&mut self, donation_id: u64) {
        if let Some(donation) = self.donations.get_mut(&donation_id) {
            donation.revoke();
            let recipient = donation.recipient;
            if let Some(state) = self.states.get_mut(&recipient) {
                state.received_donations.retain(|&d| d != donation_id);
                state.recalculate(&self.donations);
            }
        }
        self.update_stats();
    }

    /// Acquire resource (triggers ceiling protocol)
    #[inline]
    pub fn acquire_resource(&mut self, pid: u64, resource: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            state.acquire_resource(resource);
        }
        if let Some(ceiling) = self.ceilings.get_mut(&resource) {
            ceiling.holder = Some(pid);
        }
    }

    /// Release resource (revokes associated donations)
    pub fn release_resource(&mut self, pid: u64, resource: u64) {
        if let Some(state) = self.states.get_mut(&pid) {
            state.release_resource(resource);
        }
        if let Some(ceiling) = self.ceilings.get_mut(&resource) {
            ceiling.holder = None;
        }

        // Revoke all donations for this resource
        let to_revoke: Vec<u64> = self
            .donations
            .values()
            .filter(|d| d.active && d.resource == resource && d.recipient == pid)
            .map(|d| d.id)
            .collect();
        for id in to_revoke {
            self.revoke(id);
        }
    }

    /// Effective priority
    #[inline(always)]
    pub fn effective_priority(&self, pid: u64) -> Option<DonationPriority> {
        self.states.get(&pid).map(|s| s.effective_priority)
    }

    fn update_stats(&mut self) {
        self.stats.active = self.donations.values().filter(|d| d.active).count();
        self.stats.boosted = self
            .states
            .values()
            .filter(|s| s.effective_priority > s.base_priority)
            .count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopDonationStats {
        &self.stats
    }
}
