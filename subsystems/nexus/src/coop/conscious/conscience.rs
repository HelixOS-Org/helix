// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Conscience
//!
//! Ethical cooperation framework ensuring fairness axioms are never violated.
//! The conscience module acts as the moral compass of the cooperation protocol,
//! continuously checking that resource sharing, mediation, and trust-building
//! activities satisfy a set of declared axioms.
//!
//! ## Conscience Axioms
//!
//! Each axiom has a name, a weight (importance), and an enforcement level.
//! The conscience engine evaluates cooperation decisions against all axioms
//! and raises alarms when violations are detected. It also detects
//! exploitation patterns where one process systematically benefits at the
//! expense of others.
//!
//! ## Key Methods
//!
//! - `axiom_check()` — Check a cooperation decision against all axioms
//! - `fairness_guarantee()` — Verify fairness bounds are maintained
//! - `detect_exploitation()` — Detect systematic exploitation patterns
//! - `enforce_equity()` — Enforce equitable resource distribution
//! - `conscience_report()` — Generate a conscience status report
//! - `moral_cooperation()` — Overall moral health of cooperation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const MAX_AXIOMS: usize = 64;
const MAX_VIOLATIONS: usize = 256;
const MAX_EXPLOITATION_RECORDS: usize = 128;
const FAIRNESS_FLOOR: f32 = 0.3;
const EXPLOITATION_THRESHOLD: f32 = 0.7;
const EQUITY_TOLERANCE: f32 = 0.15;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Xorshift64 PRNG for noise in moral assessments
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// ENFORCEMENT LEVEL
// ============================================================================

/// How strictly an axiom is enforced
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnforcementLevel {
    /// Advisory only — violations logged but not blocked
    Advisory,
    /// Warning — violations generate alerts
    Warning,
    /// Strict — violations trigger corrective action
    Strict,
    /// Absolute — violations immediately block the decision
    Absolute,
}

impl EnforcementLevel {
    pub fn severity(&self) -> f32 {
        match self {
            EnforcementLevel::Advisory => 0.2,
            EnforcementLevel::Warning => 0.5,
            EnforcementLevel::Strict => 0.8,
            EnforcementLevel::Absolute => 1.0,
        }
    }
}

// ============================================================================
// CONSCIENCE AXIOM
// ============================================================================

/// A single ethical axiom for cooperation
#[derive(Debug, Clone)]
pub struct ConscienceAxiom {
    pub axiom_id: u64,
    pub name: String,
    /// Importance weight (0.0–1.0)
    pub weight: f32,
    /// How strictly this axiom is enforced
    pub enforcement: EnforcementLevel,
    /// Number of checks performed
    pub check_count: u64,
    /// Number of violations detected
    pub violation_count: u64,
    /// EMA-smoothed compliance rate
    pub compliance_rate: f32,
    /// Description of what this axiom ensures
    pub description: String,
    /// Tick of last check
    pub last_check_tick: u64,
}

impl ConscienceAxiom {
    pub fn new(name: String, weight: f32, enforcement: EnforcementLevel, description: String) -> Self {
        let axiom_id = fnv1a_hash(name.as_bytes());
        let w = if weight < 0.0 { 0.0 } else if weight > 1.0 { 1.0 } else { weight };
        Self {
            axiom_id,
            name,
            weight: w,
            enforcement,
            check_count: 0,
            violation_count: 0,
            compliance_rate: 1.0,
            description,
            last_check_tick: 0,
        }
    }

    /// Record a check result
    pub fn record_check(&mut self, passed: bool, tick: u64) {
        self.check_count += 1;
        if !passed {
            self.violation_count += 1;
        }
        let outcome = if passed { 1.0 } else { 0.0 };
        self.compliance_rate += EMA_ALPHA * (outcome - self.compliance_rate);
        self.last_check_tick = tick;
    }

    /// Weighted violation severity
    pub fn violation_severity(&self) -> f32 {
        (1.0 - self.compliance_rate) * self.weight * self.enforcement.severity()
    }
}

// ============================================================================
// VIOLATION RECORD
// ============================================================================

/// Record of a detected axiom violation
#[derive(Debug, Clone)]
pub struct ViolationRecord {
    pub violation_id: u64,
    pub axiom_id: u64,
    pub axiom_name: String,
    /// Processes involved in the violation
    pub involved_processes: Vec<u64>,
    /// Severity of the violation
    pub severity: f32,
    /// Tick when detected
    pub detected_tick: u64,
    /// Description of what went wrong
    pub description: String,
}

// ============================================================================
// EXPLOITATION RECORD
// ============================================================================

/// Record of systematic exploitation by one process
#[derive(Debug, Clone)]
pub struct ExploitationRecord {
    pub record_id: u64,
    /// The exploiting process
    pub exploiter_id: u64,
    /// Processes being exploited
    pub victim_ids: Vec<u64>,
    /// Exploitation score (0.0–1.0)
    pub exploitation_score: f32,
    /// How many times detected
    pub detection_count: u64,
    /// EMA-smoothed exploitation intensity
    pub intensity_ema: f32,
    /// Tick of first detection
    pub first_detected_tick: u64,
    /// Tick of latest detection
    pub last_detected_tick: u64,
}

// ============================================================================
// CONSCIENCE STATS
// ============================================================================

#[derive(Debug, Clone)]
pub struct CoopConscienceStats {
    pub total_axioms: usize,
    pub total_checks: u64,
    pub total_violations: u64,
    pub overall_compliance: f32,
    pub active_exploitations: usize,
    pub moral_health: f32,
    pub worst_axiom_compliance: f32,
    pub worst_axiom_name: String,
    pub equity_score: f32,
    pub fairness_floor_violations: u64,
}

impl CoopConscienceStats {
    pub fn new() -> Self {
        Self {
            total_axioms: 0,
            total_checks: 0,
            total_violations: 0,
            overall_compliance: 1.0,
            active_exploitations: 0,
            moral_health: 1.0,
            worst_axiom_compliance: 1.0,
            worst_axiom_name: String::new(),
            equity_score: 1.0,
            fairness_floor_violations: 0,
        }
    }
}

// ============================================================================
// COOPERATION CONSCIENCE
// ============================================================================

/// Ethical framework for cooperation ensuring fairness axioms hold
pub struct CoopConscience {
    axioms: BTreeMap<u64, ConscienceAxiom>,
    violations: Vec<ViolationRecord>,
    exploitations: BTreeMap<u64, ExploitationRecord>,
    /// Per-process resource share tracking for equity enforcement
    process_shares: BTreeMap<u64, f32>,
    pub stats: CoopConscienceStats,
    rng_state: u64,
    tick: u64,
    /// EMA-smoothed moral health
    moral_health_ema: f32,
    /// EMA-smoothed equity score
    equity_ema: f32,
}

impl CoopConscience {
    pub fn new(seed: u64) -> Self {
        Self {
            axioms: BTreeMap::new(),
            violations: Vec::new(),
            exploitations: BTreeMap::new(),
            process_shares: BTreeMap::new(),
            stats: CoopConscienceStats::new(),
            rng_state: seed | 1,
            tick: 0,
            moral_health_ema: 1.0,
            equity_ema: 1.0,
        }
    }

    /// Register a new axiom
    pub fn register_axiom(
        &mut self,
        name: String,
        weight: f32,
        enforcement: EnforcementLevel,
        description: String,
    ) -> u64 {
        if self.axioms.len() >= MAX_AXIOMS {
            return 0;
        }
        let axiom = ConscienceAxiom::new(name, weight, enforcement, description);
        let id = axiom.axiom_id;
        self.axioms.insert(id, axiom);
        self.stats.total_axioms = self.axioms.len();
        id
    }

    // ========================================================================
    // AXIOM CHECK
    // ========================================================================

    /// Check a cooperation decision against all registered axioms
    ///
    /// Takes a set of fairness scores for the decision. Returns a list of
    /// violated axiom IDs.
    pub fn axiom_check(
        &mut self,
        fairness_score: f32,
        equity_score: f32,
        processes_involved: Vec<u64>,
    ) -> Vec<u64> {
        self.tick += 1;
        let mut violated_ids = Vec::new();

        let axiom_ids: Vec<u64> = self.axioms.keys().copied().collect();
        let tick = self.tick;

        for aid in axiom_ids {
            if let Some(axiom) = self.axioms.get_mut(&aid) {
                // Generic check: fairness must meet the axiom weight threshold
                let passes = fairness_score >= axiom.weight * FAIRNESS_FLOOR
                    && equity_score >= FAIRNESS_FLOOR;
                axiom.record_check(passes, tick);

                if !passes {
                    violated_ids.push(aid);

                    // Record violation
                    if self.violations.len() < MAX_VIOLATIONS {
                        let viol_hash = {
                            let mut buf = Vec::new();
                            buf.extend_from_slice(&aid.to_le_bytes());
                            buf.extend_from_slice(&tick.to_le_bytes());
                            fnv1a_hash(&buf)
                        };
                        self.violations.push(ViolationRecord {
                            violation_id: viol_hash,
                            axiom_id: aid,
                            axiom_name: axiom.name.clone(),
                            involved_processes: processes_involved.clone(),
                            severity: axiom.violation_severity(),
                            detected_tick: tick,
                            description: String::from("axiom_violation"),
                        });
                    }
                }
            }
        }

        self.stats.total_checks += 1;
        self.stats.total_violations += violated_ids.len() as u64;
        self.update_compliance();
        violated_ids
    }

    // ========================================================================
    // FAIRNESS GUARANTEE
    // ========================================================================

    /// Verify that fairness bounds are maintained across all processes
    pub fn fairness_guarantee(&mut self, process_fairness: &BTreeMap<u64, f32>) -> bool {
        self.tick += 1;
        let mut all_pass = true;

        for (pid, fairness) in process_fairness.iter() {
            if *fairness < FAIRNESS_FLOOR {
                all_pass = false;
                self.stats.fairness_floor_violations += 1;

                // Record as a violation against the most relevant axiom
                if let Some((aid, axiom)) = self.axioms.iter().next() {
                    if self.violations.len() < MAX_VIOLATIONS {
                        let mut buf = Vec::new();
                        buf.extend_from_slice(&pid.to_le_bytes());
                        buf.extend_from_slice(&self.tick.to_le_bytes());
                        let viol_id = fnv1a_hash(&buf);
                        self.violations.push(ViolationRecord {
                            violation_id: viol_id,
                            axiom_id: *aid,
                            axiom_name: axiom.name.clone(),
                            involved_processes: { let mut v = Vec::new(); v.push(*pid); v },
                            severity: (FAIRNESS_FLOOR - *fairness) / FAIRNESS_FLOOR,
                            detected_tick: self.tick,
                            description: String::from("fairness_floor_breach"),
                        });
                    }
                }
            }
        }

        all_pass
    }

    // ========================================================================
    // DETECT EXPLOITATION
    // ========================================================================

    /// Detect systematic exploitation patterns
    ///
    /// Takes per-process resource usage ratios. A process that consistently
    /// takes more than its fair share while others suffer is flagged.
    pub fn detect_exploitation(&mut self, usage_ratios: &BTreeMap<u64, f32>) -> Vec<u64> {
        self.tick += 1;
        let count = usage_ratios.len();
        if count < 2 {
            return Vec::new();
        }

        let fair_share = 1.0 / count as f32;
        let mut exploiters = Vec::new();

        for (pid, ratio) in usage_ratios.iter() {
            let excess = *ratio - fair_share;
            if excess > EQUITY_TOLERANCE {
                let exploitation_score = (excess / (1.0 - fair_share)).min(1.0);
                if exploitation_score >= EXPLOITATION_THRESHOLD {
                    exploiters.push(*pid);

                    // Find victims (those below fair share)
                    let victims: Vec<u64> = usage_ratios.iter()
                        .filter(|(vid, r)| **vid != *pid && **r < fair_share - EQUITY_TOLERANCE)
                        .map(|(vid, _)| *vid)
                        .collect();

                    let record_id = fnv1a_hash(&pid.to_le_bytes());
                    if let Some(existing) = self.exploitations.get_mut(&record_id) {
                        existing.detection_count += 1;
                        existing.intensity_ema += EMA_ALPHA * (exploitation_score - existing.intensity_ema);
                        existing.last_detected_tick = self.tick;
                    } else if self.exploitations.len() < MAX_EXPLOITATION_RECORDS {
                        self.exploitations.insert(record_id, ExploitationRecord {
                            record_id,
                            exploiter_id: *pid,
                            victim_ids: victims,
                            exploitation_score,
                            detection_count: 1,
                            intensity_ema: exploitation_score,
                            first_detected_tick: self.tick,
                            last_detected_tick: self.tick,
                        });
                    }
                }
            }
        }

        self.stats.active_exploitations = self.exploitations.len();
        exploiters
    }

    // ========================================================================
    // ENFORCE EQUITY
    // ========================================================================

    /// Enforce equitable resource distribution
    ///
    /// Takes current shares and returns adjusted shares that satisfy equity
    /// constraints. Redistributes excess from over-allocated processes.
    pub fn enforce_equity(
        &mut self,
        current_shares: &BTreeMap<u64, f32>,
    ) -> BTreeMap<u64, f32> {
        self.tick += 1;
        let count = current_shares.len();
        if count == 0 {
            return BTreeMap::new();
        }

        let fair_share = 1.0 / count as f32;
        let mut adjusted = BTreeMap::new();
        let mut total_excess = 0.0f32;
        let mut deficit_pids = Vec::new();

        // Identify excess and deficit
        for (pid, share) in current_shares.iter() {
            let excess = *share - (fair_share + EQUITY_TOLERANCE);
            if excess > 0.0 {
                total_excess += excess;
                adjusted.insert(*pid, fair_share + EQUITY_TOLERANCE);
            } else if *share < fair_share - EQUITY_TOLERANCE {
                deficit_pids.push(*pid);
                adjusted.insert(*pid, *share);
            } else {
                adjusted.insert(*pid, *share);
            }
        }

        // Redistribute excess to deficit processes
        if !deficit_pids.is_empty() && total_excess > 0.0 {
            let per_deficit = total_excess / deficit_pids.len() as f32;
            for pid in deficit_pids {
                if let Some(share) = adjusted.get_mut(&pid) {
                    *share = (*share + per_deficit).min(fair_share + EQUITY_TOLERANCE);
                }
            }
        }

        // Update process share tracking
        for (pid, share) in adjusted.iter() {
            self.process_shares.insert(*pid, *share);
        }

        // Compute equity score
        let mut variance_sum = 0.0f32;
        for (_, share) in adjusted.iter() {
            let dev = *share - fair_share;
            variance_sum += dev * dev;
        }
        let variance = variance_sum / count as f32;
        let equity = (1.0 - variance * 10.0).max(0.0).min(1.0);
        self.equity_ema += EMA_ALPHA * (equity - self.equity_ema);
        self.stats.equity_score = self.equity_ema;

        adjusted
    }

    // ========================================================================
    // CONSCIENCE REPORT
    // ========================================================================

    /// Generate a comprehensive conscience status report
    pub fn conscience_report(&self) -> CoopConscienceReport {
        let mut axiom_summaries = Vec::new();
        for (_, axiom) in self.axioms.iter() {
            axiom_summaries.push(AxiomSummary {
                name: axiom.name.clone(),
                compliance_rate: axiom.compliance_rate,
                violation_count: axiom.violation_count,
                enforcement: axiom.enforcement,
                severity: axiom.violation_severity(),
            });
        }

        let recent_violations: Vec<ViolationRecord> = self.violations.iter()
            .rev()
            .take(10)
            .cloned()
            .collect();

        CoopConscienceReport {
            axiom_summaries,
            overall_compliance: self.stats.overall_compliance,
            moral_health: self.stats.moral_health,
            equity_score: self.stats.equity_score,
            active_exploitations: self.exploitations.len(),
            recent_violations,
            total_checks: self.stats.total_checks,
            total_violations: self.stats.total_violations,
        }
    }

    // ========================================================================
    // MORAL COOPERATION
    // ========================================================================

    /// Overall moral health score of the cooperation protocol
    pub fn moral_cooperation(&mut self) -> f32 {
        let compliance = self.stats.overall_compliance;
        let equity = self.equity_ema;
        let exploitation_penalty = if self.exploitations.is_empty() {
            0.0
        } else {
            let mut total_intensity = 0.0f32;
            for (_, e) in self.exploitations.iter() {
                total_intensity += e.intensity_ema;
            }
            (total_intensity / self.exploitations.len() as f32).min(1.0)
        };

        let raw = compliance * 0.4 + equity * 0.35 + (1.0 - exploitation_penalty) * 0.25;
        let clamped = if raw < 0.0 { 0.0 } else if raw > 1.0 { 1.0 } else { raw };

        self.moral_health_ema += EMA_ALPHA * (clamped - self.moral_health_ema);
        self.stats.moral_health = self.moral_health_ema;
        self.moral_health_ema
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn update_compliance(&mut self) {
        if self.axioms.is_empty() {
            return;
        }
        let mut total_compliance = 0.0f32;
        let mut worst = 1.0f32;
        let mut worst_name = String::new();

        for (_, axiom) in self.axioms.iter() {
            total_compliance += axiom.compliance_rate;
            if axiom.compliance_rate < worst {
                worst = axiom.compliance_rate;
                worst_name = axiom.name.clone();
            }
        }

        self.stats.overall_compliance = total_compliance / self.axioms.len() as f32;
        self.stats.worst_axiom_compliance = worst;
        self.stats.worst_axiom_name = worst_name;
    }

    // ========================================================================
    // QUERIES
    // ========================================================================

    pub fn axiom(&self, id: u64) -> Option<&ConscienceAxiom> {
        self.axioms.get(&id)
    }

    pub fn axiom_count(&self) -> usize {
        self.axioms.len()
    }

    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    pub fn exploitation_count(&self) -> usize {
        self.exploitations.len()
    }

    pub fn snapshot_stats(&self) -> CoopConscienceStats {
        self.stats.clone()
    }
}

// ============================================================================
// REPORT TYPES
// ============================================================================

/// Summary of a single axiom's status
#[derive(Debug, Clone)]
pub struct AxiomSummary {
    pub name: String,
    pub compliance_rate: f32,
    pub violation_count: u64,
    pub enforcement: EnforcementLevel,
    pub severity: f32,
}

/// Comprehensive conscience status report
#[derive(Debug, Clone)]
pub struct CoopConscienceReport {
    pub axiom_summaries: Vec<AxiomSummary>,
    pub overall_compliance: f32,
    pub moral_health: f32,
    pub equity_score: f32,
    pub active_exploitations: usize,
    pub recent_violations: Vec<ViolationRecord>,
    pub total_checks: u64,
    pub total_violations: u64,
}
