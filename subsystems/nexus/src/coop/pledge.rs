//! # Cooperative Pledge System
//!
//! Formal pledge mechanism for cooperative resource management:
//! - Pledge creation and validation
//! - Fulfillment tracking
//! - Penalty for broken pledges
//! - Pledge negotiation
//! - Historical reliability scoring

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// PLEDGE TYPES
// ============================================================================

/// Resource type for pledges
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PledgeResource {
    /// CPU time (ns)
    CpuTime,
    /// Memory (bytes)
    Memory,
    /// I/O bandwidth (bytes/s)
    IoBandwidth,
    /// Network bandwidth (bytes/s)
    NetBandwidth,
    /// GPU compute time
    GpuTime,
    /// File descriptors
    FileDescriptors,
}

/// Pledge state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PledgeState {
    /// Proposed, awaiting acceptance
    Proposed,
    /// Accepted by both parties
    Accepted,
    /// Active (being fulfilled)
    Active,
    /// Fulfilled successfully
    Fulfilled,
    /// Broken (not fulfilled)
    Broken,
    /// Expired
    Expired,
    /// Cancelled by mutual agreement
    Cancelled,
}

/// Pledge direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PledgeDirection {
    /// Pledger will provide resource
    Provide,
    /// Pledger will limit usage
    Limit,
    /// Pledger will release resource by deadline
    Release,
}

// ============================================================================
// PLEDGE
// ============================================================================

/// A cooperative pledge between processes
#[derive(Debug, Clone)]
pub struct CoopPledge {
    /// Pledge ID
    pub id: u64,
    /// Pledger (process offering)
    pub pledger: u64,
    /// Beneficiary (process receiving)
    pub beneficiary: u64,
    /// Resource type
    pub resource: PledgeResource,
    /// Direction
    pub direction: PledgeDirection,
    /// Amount
    pub amount: u64,
    /// Duration (ns)
    pub duration_ns: u64,
    /// State
    pub state: PledgeState,
    /// Created at
    pub created_at: u64,
    /// Deadline
    pub deadline: u64,
    /// Fulfilled amount so far
    pub fulfilled_amount: u64,
    /// Penalty for breaking (priority reduction)
    pub penalty: u32,
}

impl CoopPledge {
    pub fn new(
        id: u64,
        pledger: u64,
        beneficiary: u64,
        resource: PledgeResource,
        direction: PledgeDirection,
        amount: u64,
        duration_ns: u64,
        now: u64,
    ) -> Self {
        Self {
            id,
            pledger,
            beneficiary,
            resource,
            direction,
            amount,
            duration_ns,
            state: PledgeState::Proposed,
            created_at: now,
            deadline: now + duration_ns,
            fulfilled_amount: 0,
            penalty: 10,
        }
    }

    /// Accept the pledge
    pub fn accept(&mut self) {
        if self.state == PledgeState::Proposed {
            self.state = PledgeState::Accepted;
        }
    }

    /// Activate
    pub fn activate(&mut self) {
        if self.state == PledgeState::Accepted {
            self.state = PledgeState::Active;
        }
    }

    /// Record partial fulfillment
    pub fn record_fulfillment(&mut self, amount: u64) {
        self.fulfilled_amount += amount;
        if self.fulfilled_amount >= self.amount {
            self.state = PledgeState::Fulfilled;
        }
    }

    /// Check expiry
    pub fn check_expiry(&mut self, now: u64) {
        if now >= self.deadline && self.state == PledgeState::Active {
            if self.fulfilled_amount >= self.amount {
                self.state = PledgeState::Fulfilled;
            } else {
                self.state = PledgeState::Broken;
            }
        }
    }

    /// Fulfillment ratio
    pub fn fulfillment_ratio(&self) -> f64 {
        if self.amount == 0 {
            return 1.0;
        }
        self.fulfilled_amount as f64 / self.amount as f64
    }

    /// Is complete (fulfilled or broken or expired)
    pub fn is_complete(&self) -> bool {
        matches!(
            self.state,
            PledgeState::Fulfilled | PledgeState::Broken | PledgeState::Expired | PledgeState::Cancelled
        )
    }
}

// ============================================================================
// PLEDGE RELIABILITY
// ============================================================================

/// Per-process pledge reliability
#[derive(Debug, Clone)]
pub struct PledgeReliability {
    /// Process ID
    pub pid: u64,
    /// Total pledges made
    pub total_pledges: u64,
    /// Fulfilled pledges
    pub fulfilled: u64,
    /// Broken pledges
    pub broken: u64,
    /// Average fulfillment ratio
    pub avg_fulfillment: f64,
    /// Reliability score (0.0-1.0)
    pub reliability_score: f64,
    /// Accumulated penalty
    pub penalty_points: u32,
}

impl PledgeReliability {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            total_pledges: 0,
            fulfilled: 0,
            broken: 0,
            avg_fulfillment: 1.0,
            reliability_score: 1.0,
            penalty_points: 0,
        }
    }

    /// Record pledge outcome
    pub fn record_outcome(&mut self, fulfilled: bool, ratio: f64, penalty: u32) {
        self.total_pledges += 1;
        if fulfilled {
            self.fulfilled += 1;
        } else {
            self.broken += 1;
            self.penalty_points += penalty;
        }

        // Exponential moving average
        let alpha = 0.1;
        self.avg_fulfillment = self.avg_fulfillment * (1.0 - alpha) + ratio * alpha;
        self.recalculate_score();
    }

    fn recalculate_score(&mut self) {
        if self.total_pledges == 0 {
            self.reliability_score = 1.0;
            return;
        }
        let fulfill_rate = self.fulfilled as f64 / self.total_pledges as f64;
        let penalty_factor = 1.0 / (1.0 + self.penalty_points as f64 / 100.0);
        self.reliability_score = fulfill_rate * 0.6 + self.avg_fulfillment * 0.3 + penalty_factor * 0.1;
        if self.reliability_score > 1.0 {
            self.reliability_score = 1.0;
        }
    }
}

// ============================================================================
// PLEDGE MANAGER
// ============================================================================

/// Pledge manager stats
#[derive(Debug, Clone, Default)]
pub struct CoopPledgeStats {
    /// Active pledges
    pub active_pledges: usize,
    /// Total pledges
    pub total_pledges: u64,
    /// Fulfilled
    pub fulfilled: u64,
    /// Broken
    pub broken: u64,
    /// Average reliability
    pub avg_reliability: f64,
}

/// Cooperative pledge manager
pub struct CoopPledgeManager {
    /// All pledges
    pledges: BTreeMap<u64, CoopPledge>,
    /// Per-process reliability
    reliability: BTreeMap<u64, PledgeReliability>,
    /// Active pledges per process
    active_by_pid: BTreeMap<u64, Vec<u64>>,
    /// Next pledge ID
    next_id: u64,
    /// Stats
    stats: CoopPledgeStats,
}

impl CoopPledgeManager {
    pub fn new() -> Self {
        Self {
            pledges: BTreeMap::new(),
            reliability: BTreeMap::new(),
            active_by_pid: BTreeMap::new(),
            next_id: 1,
            stats: CoopPledgeStats::default(),
        }
    }

    /// Create pledge
    pub fn create_pledge(
        &mut self,
        pledger: u64,
        beneficiary: u64,
        resource: PledgeResource,
        direction: PledgeDirection,
        amount: u64,
        duration_ns: u64,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let pledge = CoopPledge::new(id, pledger, beneficiary, resource, direction, amount, duration_ns, now);
        self.pledges.insert(id, pledge);
        self.stats.total_pledges += 1;
        id
    }

    /// Accept pledge
    pub fn accept_pledge(&mut self, pledge_id: u64) {
        if let Some(pledge) = self.pledges.get_mut(&pledge_id) {
            pledge.accept();
        }
    }

    /// Activate pledge
    pub fn activate_pledge(&mut self, pledge_id: u64) {
        if let Some(pledge) = self.pledges.get_mut(&pledge_id) {
            pledge.activate();
            self.active_by_pid
                .entry(pledge.pledger)
                .or_insert_with(Vec::new)
                .push(pledge_id);
        }
        self.stats.active_pledges = self
            .pledges
            .values()
            .filter(|p| p.state == PledgeState::Active)
            .count();
    }

    /// Record fulfillment
    pub fn record_fulfillment(&mut self, pledge_id: u64, amount: u64) {
        if let Some(pledge) = self.pledges.get_mut(&pledge_id) {
            pledge.record_fulfillment(amount);
            if pledge.state == PledgeState::Fulfilled {
                let rel = self
                    .reliability
                    .entry(pledge.pledger)
                    .or_insert_with(|| PledgeReliability::new(pledge.pledger));
                rel.record_outcome(true, pledge.fulfillment_ratio(), 0);
                self.stats.fulfilled += 1;
            }
        }
    }

    /// Check expirations
    pub fn check_expirations(&mut self, now: u64) {
        let mut broken = Vec::new();
        for pledge in self.pledges.values_mut() {
            let old_state = pledge.state;
            pledge.check_expiry(now);
            if pledge.state == PledgeState::Broken && old_state != PledgeState::Broken {
                broken.push((pledge.pledger, pledge.fulfillment_ratio(), pledge.penalty));
            }
        }
        for (pid, ratio, penalty) in broken {
            let rel = self
                .reliability
                .entry(pid)
                .or_insert_with(|| PledgeReliability::new(pid));
            rel.record_outcome(false, ratio, penalty);
            self.stats.broken += 1;
        }
        self.stats.active_pledges = self
            .pledges
            .values()
            .filter(|p| p.state == PledgeState::Active)
            .count();
    }

    /// Get reliability
    pub fn reliability(&self, pid: u64) -> Option<&PledgeReliability> {
        self.reliability.get(&pid)
    }

    /// Get pledge
    pub fn pledge(&self, id: u64) -> Option<&CoopPledge> {
        self.pledges.get(&id)
    }

    /// Stats
    pub fn stats(&self) -> &CoopPledgeStats {
        &self.stats
    }
}
