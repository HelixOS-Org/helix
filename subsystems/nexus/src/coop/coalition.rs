//! # Cooperative Coalition Formation
//!
//! Mechanisms for processes to form cooperative groups:
//! - Coalition formation and dissolution
//! - Resource pooling
//! - Joint optimization
//! - Coalition stability analysis
//! - Benefit distribution

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// COALITION TYPES
// ============================================================================

/// Coalition purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoalitionPurpose {
    /// Pool CPU resources
    CpuPooling,
    /// Pool memory
    MemoryPooling,
    /// Shared cache optimization
    CacheSharing,
    /// Coordinated I/O
    IoCoordination,
    /// Network optimization
    NetworkOptimization,
    /// Power optimization
    PowerManagement,
    /// Joint scheduling
    JointScheduling,
}

/// Coalition state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoalitionState {
    /// Forming (recruiting members)
    Forming,
    /// Active
    Active,
    /// Degraded (some members left)
    Degraded,
    /// Dissolving
    Dissolving,
    /// Dissolved
    Dissolved,
}

/// Member role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberRole {
    /// Leader (coordinator)
    Leader,
    /// Regular member
    Member,
    /// Observer (can see but not contribute)
    Observer,
}

// ============================================================================
// COALITION MEMBER
// ============================================================================

/// Coalition member
#[derive(Debug, Clone)]
pub struct CoalitionMember {
    /// Process id
    pub pid: u64,
    /// Role
    pub role: MemberRole,
    /// Resources contributed
    pub contribution: u64,
    /// Resources received
    pub benefit: u64,
    /// Joined at
    pub joined_at: u64,
    /// Active
    pub active: bool,
    /// Cooperation score
    pub cooperation_score: f64,
}

impl CoalitionMember {
    pub fn new(pid: u64, role: MemberRole, now: u64) -> Self {
        Self {
            pid,
            role,
            contribution: 0,
            benefit: 0,
            joined_at: now,
            active: true,
            cooperation_score: 1.0,
        }
    }

    /// Record contribution
    pub fn contribute(&mut self, amount: u64) {
        self.contribution += amount;
    }

    /// Record benefit
    pub fn receive_benefit(&mut self, amount: u64) {
        self.benefit += amount;
    }

    /// Net value (benefit - contribution)
    pub fn net_value(&self) -> i64 {
        self.benefit as i64 - self.contribution as i64
    }

    /// Fair share ratio
    pub fn fairness_ratio(&self) -> f64 {
        if self.contribution == 0 {
            return if self.benefit > 0 { f64::MAX } else { 1.0 };
        }
        self.benefit as f64 / self.contribution as f64
    }

    /// Deactivate
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

// ============================================================================
// COALITION
// ============================================================================

/// A cooperative coalition
#[derive(Debug)]
pub struct Coalition {
    /// Coalition id
    pub id: u64,
    /// Purpose
    pub purpose: CoalitionPurpose,
    /// State
    pub state: CoalitionState,
    /// Members
    pub members: BTreeMap<u64, CoalitionMember>,
    /// Minimum members
    pub min_members: usize,
    /// Maximum members
    pub max_members: usize,
    /// Created at
    pub created_at: u64,
    /// Total pooled resources
    pub pooled_resources: u64,
    /// Efficiency gain (>1.0 means coalition is beneficial)
    pub efficiency: f64,
}

impl Coalition {
    pub fn new(id: u64, purpose: CoalitionPurpose, min: usize, max: usize, now: u64) -> Self {
        Self {
            id,
            purpose,
            state: CoalitionState::Forming,
            members: BTreeMap::new(),
            min_members: min,
            max_members: max,
            created_at: now,
            pooled_resources: 0,
            efficiency: 1.0,
        }
    }

    /// Add member
    pub fn add_member(&mut self, pid: u64, role: MemberRole, now: u64) -> bool {
        if self.members.len() >= self.max_members {
            return false;
        }
        if self.state == CoalitionState::Dissolving || self.state == CoalitionState::Dissolved {
            return false;
        }
        let member = CoalitionMember::new(pid, role, now);
        self.members.insert(pid, member);

        // Check if we have enough members to activate
        if self.state == CoalitionState::Forming
            && self.active_count() >= self.min_members
        {
            self.state = CoalitionState::Active;
        }
        true
    }

    /// Remove member
    pub fn remove_member(&mut self, pid: u64) {
        if let Some(member) = self.members.get_mut(&pid) {
            member.deactivate();
        }
        // Check if we've lost quorum
        if self.state == CoalitionState::Active
            && self.active_count() < self.min_members
        {
            self.state = CoalitionState::Degraded;
        }
    }

    /// Contribute resources
    pub fn contribute(&mut self, pid: u64, amount: u64) -> bool {
        if let Some(member) = self.members.get_mut(&pid) {
            if !member.active {
                return false;
            }
            member.contribute(amount);
            self.pooled_resources += amount;
            true
        } else {
            false
        }
    }

    /// Distribute benefit
    pub fn distribute(&mut self, pid: u64, amount: u64) -> bool {
        if let Some(member) = self.members.get_mut(&pid) {
            if !member.active {
                return false;
            }
            member.receive_benefit(amount);
            true
        } else {
            false
        }
    }

    /// Fair distribution of pooled resources
    pub fn fair_distribute(&mut self) {
        let active = self.active_count();
        if active == 0 {
            return;
        }
        let total_contribution: u64 = self
            .members
            .values()
            .filter(|m| m.active)
            .map(|m| m.contribution)
            .sum();
        if total_contribution == 0 {
            return;
        }

        // Distribute proportionally to contribution
        let total_pool = self.pooled_resources;
        for member in self.members.values_mut() {
            if member.active && member.contribution > 0 {
                let share =
                    (total_pool as f64 * member.contribution as f64 / total_contribution as f64)
                        as u64;
                member.receive_benefit(share);
            }
        }
    }

    /// Active member count
    pub fn active_count(&self) -> usize {
        self.members.values().filter(|m| m.active).count()
    }

    /// Leader
    pub fn leader(&self) -> Option<u64> {
        self.members
            .values()
            .find(|m| m.active && m.role == MemberRole::Leader)
            .map(|m| m.pid)
    }

    /// Fairness score (0-1, 1 = perfectly fair)
    pub fn fairness_score(&self) -> f64 {
        let ratios: Vec<f64> = self
            .members
            .values()
            .filter(|m| m.active && m.contribution > 0)
            .map(|m| m.fairness_ratio())
            .collect();
        if ratios.is_empty() {
            return 1.0;
        }
        // Jain's fairness index
        let sum: f64 = ratios.iter().sum();
        let sum_sq: f64 = ratios.iter().map(|r| r * r).sum();
        let n = ratios.len() as f64;
        if sum_sq * n == 0.0 {
            return 1.0;
        }
        (sum * sum) / (n * sum_sq)
    }

    /// Dissolve coalition
    pub fn dissolve(&mut self) {
        self.state = CoalitionState::Dissolving;
        for member in self.members.values_mut() {
            member.deactivate();
        }
        self.state = CoalitionState::Dissolved;
    }
}

// ============================================================================
// COALITION MANAGER
// ============================================================================

/// Coalition stats
#[derive(Debug, Clone, Default)]
pub struct CoopCoalitionStats {
    /// Active coalitions
    pub active: usize,
    /// Total members
    pub total_members: usize,
    /// Average efficiency
    pub avg_efficiency: f64,
}

/// Cooperative coalition manager
pub struct CoopCoalitionManager {
    /// Coalitions
    coalitions: BTreeMap<u64, Coalition>,
    /// Process -> coalition mapping
    membership: BTreeMap<u64, Vec<u64>>,
    /// Next id
    next_id: u64,
    /// Stats
    stats: CoopCoalitionStats,
}

impl CoopCoalitionManager {
    pub fn new() -> Self {
        Self {
            coalitions: BTreeMap::new(),
            membership: BTreeMap::new(),
            next_id: 1,
            stats: CoopCoalitionStats::default(),
        }
    }

    /// Create coalition
    pub fn create(
        &mut self,
        purpose: CoalitionPurpose,
        min_members: usize,
        max_members: usize,
        leader: u64,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut coalition = Coalition::new(id, purpose, min_members, max_members, now);
        coalition.add_member(leader, MemberRole::Leader, now);
        self.coalitions.insert(id, coalition);
        self.membership
            .entry(leader)
            .or_insert_with(Vec::new)
            .push(id);
        self.update_stats();
        id
    }

    /// Join coalition
    pub fn join(&mut self, coalition_id: u64, pid: u64, now: u64) -> bool {
        let ok = if let Some(c) = self.coalitions.get_mut(&coalition_id) {
            c.add_member(pid, MemberRole::Member, now)
        } else {
            false
        };
        if ok {
            self.membership
                .entry(pid)
                .or_insert_with(Vec::new)
                .push(coalition_id);
        }
        self.update_stats();
        ok
    }

    /// Leave coalition
    pub fn leave(&mut self, coalition_id: u64, pid: u64) {
        if let Some(c) = self.coalitions.get_mut(&coalition_id) {
            c.remove_member(pid);
        }
        if let Some(memberships) = self.membership.get_mut(&pid) {
            memberships.retain(|&id| id != coalition_id);
        }
        self.update_stats();
    }

    /// Dissolve
    pub fn dissolve(&mut self, coalition_id: u64) {
        if let Some(c) = self.coalitions.get_mut(&coalition_id) {
            c.dissolve();
        }
        self.update_stats();
    }

    /// Get coalition
    pub fn coalition(&self, id: u64) -> Option<&Coalition> {
        self.coalitions.get(&id)
    }

    /// Coalitions for process
    pub fn coalitions_for(&self, pid: u64) -> Vec<u64> {
        self.membership.get(&pid).cloned().unwrap_or_default()
    }

    fn update_stats(&mut self) {
        let active: Vec<_> = self
            .coalitions
            .values()
            .filter(|c| c.state == CoalitionState::Active)
            .collect();
        self.stats.active = active.len();
        self.stats.total_members = active.iter().map(|c| c.active_count()).sum();
        if !active.is_empty() {
            self.stats.avg_efficiency =
                active.iter().map(|c| c.efficiency).sum::<f64>() / active.len() as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &CoopCoalitionStats {
        &self.stats
    }
}
