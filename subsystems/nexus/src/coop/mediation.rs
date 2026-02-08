//! # Cooperative Mediation Protocol
//!
//! Mediation for resource conflicts between processes:
//! - Conflict detection
//! - Fair resolution strategies
//! - Compromise negotiation
//! - History-based mediation
//! - Multi-party dispute resolution

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CONFLICT TYPES
// ============================================================================

/// Conflict resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConflictResource {
    /// CPU time
    CpuTime,
    /// Memory bandwidth
    MemoryBandwidth,
    /// Cache space
    CacheSpace,
    /// I/O bandwidth
    IoBandwidth,
    /// Network bandwidth
    NetworkBandwidth,
    /// Lock contention
    LockContention,
    /// Priority conflict
    PriorityConflict,
}

/// Conflict severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConflictSeverity {
    /// Minor disagreement
    Minor,
    /// Moderate contention
    Moderate,
    /// Severe conflict
    Severe,
    /// Critical deadlock risk
    Critical,
}

/// Resolution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionStrategy {
    /// Split equally
    EqualSplit,
    /// Proportional to need
    Proportional,
    /// Priority-based
    PriorityBased,
    /// Round-robin time-sharing
    TimeSharing,
    /// Auction-based
    AuctionBased,
    /// Historical fairness
    HistoricalFairness,
}

/// Resolution state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionState {
    /// Pending
    Pending,
    /// In mediation
    Mediating,
    /// Resolved
    Resolved,
    /// Escalated (couldn't resolve)
    Escalated,
    /// Timed out
    TimedOut,
}

// ============================================================================
// CONFLICT
// ============================================================================

/// A resource conflict
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Conflict id
    pub id: u64,
    /// Parties involved
    pub parties: Vec<u64>,
    /// Resource
    pub resource: ConflictResource,
    /// Resource identifier
    pub resource_id: u64,
    /// Severity
    pub severity: ConflictSeverity,
    /// State
    pub state: ResolutionState,
    /// Strategy used
    pub strategy: Option<ResolutionStrategy>,
    /// Created at
    pub created_at: u64,
    /// Resolved at
    pub resolved_at: Option<u64>,
    /// Allocations (pid -> share)
    pub allocations: BTreeMap<u64, u64>,
    /// Satisfaction scores (pid -> 0-100)
    pub satisfaction: BTreeMap<u64, u32>,
}

impl Conflict {
    pub fn new(
        id: u64,
        parties: Vec<u64>,
        resource: ConflictResource,
        resource_id: u64,
        severity: ConflictSeverity,
        now: u64,
    ) -> Self {
        Self {
            id,
            parties,
            resource,
            resource_id,
            severity,
            state: ResolutionState::Pending,
            strategy: None,
            created_at: now,
            resolved_at: None,
            allocations: BTreeMap::new(),
            satisfaction: BTreeMap::new(),
        }
    }

    /// Begin mediation
    pub fn begin_mediation(&mut self, strategy: ResolutionStrategy) {
        self.state = ResolutionState::Mediating;
        self.strategy = Some(strategy);
    }

    /// Resolve with allocations
    pub fn resolve(&mut self, allocations: BTreeMap<u64, u64>, now: u64) {
        self.allocations = allocations;
        self.state = ResolutionState::Resolved;
        self.resolved_at = Some(now);
    }

    /// Escalate
    pub fn escalate(&mut self) {
        self.state = ResolutionState::Escalated;
    }

    /// Duration to resolve
    pub fn resolution_time_ns(&self) -> Option<u64> {
        self.resolved_at
            .map(|r| r.saturating_sub(self.created_at))
    }

    /// Average satisfaction
    pub fn avg_satisfaction(&self) -> f64 {
        if self.satisfaction.is_empty() {
            return 0.0;
        }
        let sum: u32 = self.satisfaction.values().sum();
        sum as f64 / self.satisfaction.len() as f64
    }
}

// ============================================================================
// MEDIATION POLICY
// ============================================================================

/// Mediation policy
#[derive(Debug, Clone)]
pub struct MediationPolicy {
    /// Preferred strategy per resource type
    pub strategies: BTreeMap<u8, ResolutionStrategy>,
    /// Timeout (ns)
    pub timeout_ns: u64,
    /// Max escalation attempts before forced resolution
    pub max_escalations: u32,
    /// Fairness weight (0-100)
    pub fairness_weight: u32,
}

impl MediationPolicy {
    pub fn default_policy() -> Self {
        let mut strategies = BTreeMap::new();
        strategies.insert(ConflictResource::CpuTime as u8, ResolutionStrategy::Proportional);
        strategies.insert(
            ConflictResource::MemoryBandwidth as u8,
            ResolutionStrategy::Proportional,
        );
        strategies.insert(
            ConflictResource::CacheSpace as u8,
            ResolutionStrategy::EqualSplit,
        );
        strategies.insert(
            ConflictResource::IoBandwidth as u8,
            ResolutionStrategy::Proportional,
        );
        strategies.insert(
            ConflictResource::LockContention as u8,
            ResolutionStrategy::PriorityBased,
        );
        Self {
            strategies,
            timeout_ns: 100_000_000, // 100ms
            max_escalations: 3,
            fairness_weight: 50,
        }
    }

    /// Get strategy for resource
    pub fn strategy_for(&self, resource: ConflictResource) -> ResolutionStrategy {
        self.strategies
            .get(&(resource as u8))
            .copied()
            .unwrap_or(ResolutionStrategy::EqualSplit)
    }
}

// ============================================================================
// FAIRNESS TRACKER
// ============================================================================

/// Historical fairness for a process
#[derive(Debug, Clone)]
pub struct FairnessRecord {
    /// Process id
    pub pid: u64,
    /// Total resources requested
    pub total_requested: u64,
    /// Total resources received
    pub total_received: u64,
    /// Conflicts won
    pub conflicts_won: u64,
    /// Conflicts lost
    pub conflicts_lost: u64,
}

impl FairnessRecord {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            total_requested: 0,
            total_received: 0,
            conflicts_won: 0,
            conflicts_lost: 0,
        }
    }

    /// Satisfaction ratio
    pub fn satisfaction_ratio(&self) -> f64 {
        if self.total_requested == 0 {
            return 1.0;
        }
        self.total_received as f64 / self.total_requested as f64
    }

    /// Win rate
    pub fn win_rate(&self) -> f64 {
        let total = self.conflicts_won + self.conflicts_lost;
        if total == 0 {
            return 0.5;
        }
        self.conflicts_won as f64 / total as f64
    }

    /// Fairness debt (positive = owed resources)
    pub fn fairness_debt(&self) -> i64 {
        self.total_requested as i64 - self.total_received as i64
    }
}

// ============================================================================
// MEDIATION MANAGER
// ============================================================================

/// Mediation stats
#[derive(Debug, Clone, Default)]
pub struct CoopMediationStats {
    /// Active conflicts
    pub active: usize,
    /// Resolved
    pub resolved: u64,
    /// Escalated
    pub escalated: u64,
    /// Average satisfaction
    pub avg_satisfaction: f64,
    /// Average resolution time ns
    pub avg_resolution_ns: u64,
}

/// Cooperative mediation manager
pub struct CoopMediationManager {
    /// Conflicts
    conflicts: BTreeMap<u64, Conflict>,
    /// Fairness records
    fairness: BTreeMap<u64, FairnessRecord>,
    /// Policy
    policy: MediationPolicy,
    /// Next id
    next_id: u64,
    /// Stats
    stats: CoopMediationStats,
    /// Resolution time sum
    resolution_time_sum: u64,
    resolution_count: u64,
}

impl CoopMediationManager {
    pub fn new() -> Self {
        Self {
            conflicts: BTreeMap::new(),
            fairness: BTreeMap::new(),
            policy: MediationPolicy::default_policy(),
            next_id: 1,
            stats: CoopMediationStats::default(),
            resolution_time_sum: 0,
            resolution_count: 0,
        }
    }

    /// Report conflict
    pub fn report_conflict(
        &mut self,
        parties: Vec<u64>,
        resource: ConflictResource,
        resource_id: u64,
        severity: ConflictSeverity,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let conflict = Conflict::new(id, parties, resource, resource_id, severity, now);
        self.conflicts.insert(id, conflict);
        self.update_stats();
        id
    }

    /// Mediate conflict
    pub fn mediate(&mut self, conflict_id: u64, total_resource: u64, now: u64) -> bool {
        let (parties, resource, strategy) = if let Some(conflict) = self.conflicts.get(&conflict_id) {
            let strategy = self.policy.strategy_for(conflict.resource);
            (conflict.parties.clone(), conflict.resource, strategy)
        } else {
            return false;
        };

        if let Some(conflict) = self.conflicts.get_mut(&conflict_id) {
            conflict.begin_mediation(strategy);
        }

        // Compute allocations based on strategy
        let allocations = match strategy {
            ResolutionStrategy::EqualSplit => {
                let share = if parties.is_empty() {
                    0
                } else {
                    total_resource / parties.len() as u64
                };
                let mut alloc = BTreeMap::new();
                for &pid in &parties {
                    alloc.insert(pid, share);
                }
                alloc
            }
            ResolutionStrategy::Proportional | ResolutionStrategy::HistoricalFairness => {
                // Use fairness debt to adjust proportions
                let mut weights: Vec<(u64, f64)> = Vec::new();
                for &pid in &parties {
                    let record = self
                        .fairness
                        .get(&pid)
                        .cloned()
                        .unwrap_or_else(|| FairnessRecord::new(pid));
                    let debt = record.fairness_debt();
                    let weight = 1.0 + (debt as f64 / 1000.0).max(0.0);
                    weights.push((pid, weight));
                }
                let total_weight: f64 = weights.iter().map(|(_, w)| w).sum();
                let mut alloc = BTreeMap::new();
                if total_weight > 0.0 {
                    for (pid, weight) in &weights {
                        let share = (total_resource as f64 * weight / total_weight) as u64;
                        alloc.insert(*pid, share);
                    }
                }
                alloc
            }
            _ => {
                // Default: equal split
                let share = if parties.is_empty() {
                    0
                } else {
                    total_resource / parties.len() as u64
                };
                let mut alloc = BTreeMap::new();
                for &pid in &parties {
                    alloc.insert(pid, share);
                }
                alloc
            }
        };

        // Apply allocations
        for (&pid, &amount) in &allocations {
            let record = self
                .fairness
                .entry(pid)
                .or_insert_with(|| FairnessRecord::new(pid));
            record.total_received += amount;
            record.total_requested += total_resource / parties.len().max(1) as u64;
        }

        if let Some(conflict) = self.conflicts.get_mut(&conflict_id) {
            conflict.resolve(allocations, now);
            if let Some(time) = conflict.resolution_time_ns() {
                self.resolution_time_sum += time;
                self.resolution_count += 1;
            }
            self.stats.resolved += 1;
        }

        self.update_stats();
        true
    }

    /// Get conflict
    pub fn conflict(&self, id: u64) -> Option<&Conflict> {
        self.conflicts.get(&id)
    }

    /// Fairness record
    pub fn fairness_of(&self, pid: u64) -> Option<&FairnessRecord> {
        self.fairness.get(&pid)
    }

    fn update_stats(&mut self) {
        self.stats.active = self
            .conflicts
            .values()
            .filter(|c| {
                c.state == ResolutionState::Pending || c.state == ResolutionState::Mediating
            })
            .count();
        if self.resolution_count > 0 {
            self.stats.avg_resolution_ns = self.resolution_time_sum / self.resolution_count;
        }
    }

    /// Stats
    pub fn stats(&self) -> &CoopMediationStats {
        &self.stats
    }
}
