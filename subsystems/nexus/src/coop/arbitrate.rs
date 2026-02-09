//! # Cooperative Arbitration
//!
//! Conflict resolution between cooperating processes:
//! - Dispute detection and classification
//! - Arbitration protocols
//! - Fair resource splitting
//! - Escalation policies
//! - Resolution history

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// DISPUTE TYPES
// ============================================================================

/// Dispute category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisputeCategory {
    /// Resource contention
    ResourceContention,
    /// Priority conflict
    PriorityConflict,
    /// Broken agreement
    BrokenAgreement,
    /// Fairness violation
    FairnessViolation,
    /// Deadline miss
    DeadlineMiss,
    /// Access conflict
    AccessConflict,
}

/// Dispute severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisputeSeverity {
    /// Low (advisory)
    Low,
    /// Medium (resource degradation)
    Medium,
    /// High (service impact)
    High,
    /// Critical (system stability)
    Critical,
}

/// Dispute state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeState {
    /// Filed
    Filed,
    /// Under review
    UnderReview,
    /// In arbitration
    InArbitration,
    /// Resolved
    Resolved,
    /// Escalated
    Escalated,
    /// Dismissed
    Dismissed,
}

// ============================================================================
// DISPUTE
// ============================================================================

/// A dispute between processes
#[derive(Debug, Clone)]
pub struct Dispute {
    /// Dispute ID
    pub id: u64,
    /// Complainant
    pub complainant: u64,
    /// Respondent
    pub respondent: u64,
    /// Category
    pub category: DisputeCategory,
    /// Severity
    pub severity: DisputeSeverity,
    /// State
    pub state: DisputeState,
    /// Filed at
    pub filed_at: u64,
    /// Resolved at
    pub resolved_at: Option<u64>,
    /// Evidence (resource amounts, timestamps)
    pub evidence: Vec<DisputeEvidence>,
    /// Resolution
    pub resolution: Option<Resolution>,
}

/// Evidence for a dispute
#[derive(Debug, Clone)]
pub struct DisputeEvidence {
    /// Evidence type
    pub evidence_type: EvidenceType,
    /// Numeric value
    pub value: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Source PID
    pub source: u64,
}

/// Evidence type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    /// Resource usage measurement
    ResourceUsage,
    /// Agreement reference
    AgreementRef,
    /// Priority level
    PriorityLevel,
    /// Deadline timestamp
    Deadline,
    /// Witness attestation
    WitnessAttestation,
}

// ============================================================================
// RESOLUTION
// ============================================================================

/// Dispute resolution
#[derive(Debug, Clone)]
pub struct Resolution {
    /// Resolution type
    pub resolution_type: ResolutionType,
    /// Favors complainant (true) or respondent (false)
    pub favors_complainant: bool,
    /// Resource adjustment
    pub adjustments: Vec<ResourceAdjustment>,
    /// Penalty applied
    pub penalty_applied: bool,
    /// Penalty target
    pub penalty_target: Option<u64>,
}

/// Resolution type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionType {
    /// Equal split
    EqualSplit,
    /// Proportional split
    ProportionalSplit,
    /// Winner takes all
    WinnerTakesAll,
    /// Compromise
    Compromise,
    /// Escalated to higher authority
    Escalated,
    /// Dismissed (no action)
    Dismissed,
}

/// Resource adjustment in resolution
#[derive(Debug, Clone)]
pub struct ResourceAdjustment {
    /// Target PID
    pub pid: u64,
    /// Resource type code
    pub resource_type: u8,
    /// New allocation
    pub new_allocation: u64,
    /// Previous allocation
    pub prev_allocation: u64,
}

// ============================================================================
// ARBITRATION POLICY
// ============================================================================

/// Arbitration policy
#[derive(Debug, Clone)]
pub struct ArbitrationPolicy {
    /// Default resolution strategy
    pub default_strategy: ResolutionType,
    /// Auto-resolve threshold (severity below this auto-resolves)
    pub auto_resolve_severity: DisputeSeverity,
    /// Max resolution time (ns)
    pub max_resolution_ns: u64,
    /// Escalation threshold (unresolved after this time)
    pub escalation_ns: u64,
    /// Allow penalties
    pub penalties_enabled: bool,
}

impl ArbitrationPolicy {
    #[inline]
    pub fn default_policy() -> Self {
        Self {
            default_strategy: ResolutionType::ProportionalSplit,
            auto_resolve_severity: DisputeSeverity::Low,
            max_resolution_ns: 5_000_000_000,     // 5 seconds
            escalation_ns: 10_000_000_000,         // 10 seconds
            penalties_enabled: true,
        }
    }
}

// ============================================================================
// ARBITRATION MANAGER
// ============================================================================

/// Arbitration stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopArbitrationStats {
    /// Active disputes
    pub active_disputes: usize,
    /// Total disputes
    pub total_disputes: u64,
    /// Resolved disputes
    pub resolved: u64,
    /// Escalated
    pub escalated: u64,
    /// Average resolution time (ns)
    pub avg_resolution_ns: f64,
    /// Penalties applied
    pub penalties_applied: u64,
}

/// Cooperative arbitration manager
pub struct CoopArbitrationManager {
    /// Disputes
    disputes: BTreeMap<u64, Dispute>,
    /// Policy
    policy: ArbitrationPolicy,
    /// Resolution history per process pair
    history: BTreeMap<(u64, u64), Vec<u64>>,
    /// Next dispute ID
    next_id: u64,
    /// Total resolution time
    total_resolution_ns: u64,
    /// Stats
    stats: CoopArbitrationStats,
}

impl CoopArbitrationManager {
    pub fn new() -> Self {
        Self {
            disputes: BTreeMap::new(),
            policy: ArbitrationPolicy::default_policy(),
            history: BTreeMap::new(),
            next_id: 1,
            total_resolution_ns: 0,
            stats: CoopArbitrationStats::default(),
        }
    }

    /// Set policy
    #[inline(always)]
    pub fn set_policy(&mut self, policy: ArbitrationPolicy) {
        self.policy = policy;
    }

    /// File dispute
    pub fn file_dispute(
        &mut self,
        complainant: u64,
        respondent: u64,
        category: DisputeCategory,
        severity: DisputeSeverity,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let state = if severity <= self.policy.auto_resolve_severity {
            DisputeState::InArbitration
        } else {
            DisputeState::Filed
        };

        self.disputes.insert(
            id,
            Dispute {
                id,
                complainant,
                respondent,
                category,
                severity,
                state,
                filed_at: now,
                resolved_at: None,
                evidence: Vec::new(),
                resolution: None,
            },
        );

        self.stats.total_disputes += 1;
        self.update_active_count();
        id
    }

    /// Add evidence
    #[inline]
    pub fn add_evidence(&mut self, dispute_id: u64, evidence: DisputeEvidence) {
        if let Some(dispute) = self.disputes.get_mut(&dispute_id) {
            dispute.evidence.push(evidence);
        }
    }

    /// Begin arbitration
    #[inline]
    pub fn begin_arbitration(&mut self, dispute_id: u64) {
        if let Some(dispute) = self.disputes.get_mut(&dispute_id) {
            if dispute.state == DisputeState::Filed {
                dispute.state = DisputeState::UnderReview;
            }
        }
    }

    /// Resolve dispute
    pub fn resolve(
        &mut self,
        dispute_id: u64,
        resolution: Resolution,
        now: u64,
    ) {
        if let Some(dispute) = self.disputes.get_mut(&dispute_id) {
            let duration = now.saturating_sub(dispute.filed_at);
            dispute.state = DisputeState::Resolved;
            dispute.resolved_at = Some(now);

            if resolution.penalty_applied {
                self.stats.penalties_applied += 1;
            }
            dispute.resolution = Some(resolution);

            // Record in history
            let pair = (
                dispute.complainant.min(dispute.respondent),
                dispute.complainant.max(dispute.respondent),
            );
            self.history
                .entry(pair)
                .or_insert_with(Vec::new)
                .push(dispute_id);

            self.total_resolution_ns += duration;
            self.stats.resolved += 1;
            if self.stats.resolved > 0 {
                self.stats.avg_resolution_ns =
                    self.total_resolution_ns as f64 / self.stats.resolved as f64;
            }
        }
        self.update_active_count();
    }

    /// Auto-resolve low-severity disputes
    pub fn auto_resolve(&mut self, now: u64) {
        let auto_ids: Vec<u64> = self
            .disputes
            .iter()
            .filter(|(_, d)| {
                d.state == DisputeState::InArbitration
                    && d.severity <= self.policy.auto_resolve_severity
            })
            .map(|(&id, _)| id)
            .collect();

        for id in auto_ids {
            let resolution = Resolution {
                resolution_type: self.policy.default_strategy,
                favors_complainant: false,
                adjustments: Vec::new(),
                penalty_applied: false,
                penalty_target: None,
            };
            self.resolve(id, resolution, now);
        }
    }

    /// Escalate overdue disputes
    pub fn escalate_overdue(&mut self, now: u64) {
        for dispute in self.disputes.values_mut() {
            if !matches!(
                dispute.state,
                DisputeState::Resolved | DisputeState::Escalated | DisputeState::Dismissed
            ) {
                let age = now.saturating_sub(dispute.filed_at);
                if age > self.policy.escalation_ns {
                    dispute.state = DisputeState::Escalated;
                    self.stats.escalated += 1;
                }
            }
        }
    }

    fn update_active_count(&mut self) {
        self.stats.active_disputes = self
            .disputes
            .values()
            .filter(|d| {
                !matches!(
                    d.state,
                    DisputeState::Resolved | DisputeState::Dismissed | DisputeState::Escalated
                )
            })
            .count();
    }

    /// Dispute history between two processes
    #[inline(always)]
    pub fn dispute_history(&self, pid1: u64, pid2: u64) -> usize {
        let pair = (pid1.min(pid2), pid1.max(pid2));
        self.history.get(&pair).map(|h| h.len()).unwrap_or(0)
    }

    /// Get dispute
    #[inline(always)]
    pub fn dispute(&self, id: u64) -> Option<&Dispute> {
        self.disputes.get(&id)
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopArbitrationStats {
        &self.stats
    }
}
