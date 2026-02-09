// SPDX-License-Identifier: GPL-2.0
//! # Holistic Conscience
//!
//! **System-wide ethical framework.** The ultimate fairness engine ensures the
//! kernel never makes a decision that violates fundamental principles. Every
//! scheduling decision, memory allocation, and resource distribution is checked
//! against the principle hierarchy. Inviolable principles cannot be overridden
//! under any circumstances.
//!
//! ## Conscience Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │              SYSTEM CONSCIENCE                               │
//! ├──────────────────────────────────────────────────────────────┤
//! │  Principle Hierarchy                                        │
//! │       │                                                     │
//! │       ├── Inviolable: "Never starve a critical process"     │
//! │       ├── Inviolable: "Never corrupt memory integrity"      │
//! │       ├── Strong:     "Minimize worst-case latency"         │
//! │       ├── Normal:     "Maximize throughput fairness"        │
//! │       └── Advisory:   "Prefer energy efficiency"            │
//! │                                                             │
//! │  Ethical Check ──▶ Compliance Score ──▶ Override / Veto     │
//! │       │                  │                    │              │
//! │       ▼                  ▼                    ▼              │
//! │  "Does this        "How compliant    "VETO: principle       │
//! │   action comply?"   overall?"         violated!"            │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! The conscience acts as the kernel's moral authority — it can VETO any
//! decision that would violate an inviolable principle.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_PRINCIPLES: usize = 128;
const MAX_CHECKS: usize = 256;
const MAX_VIOLATIONS: usize = 128;
const MAX_HISTORY: usize = 256;
const COMPLIANCE_GOOD: f32 = 0.85;
const COMPLIANCE_WARN: f32 = 0.60;
const INJUSTICE_THRESHOLD: f32 = 0.30;
const OVERRIDE_COST: f32 = 10.0;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING & PRNG
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// PRINCIPLE LEVEL
// ============================================================================

/// How strongly a principle must be upheld
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrincipleLevel {
    /// Informational — nice to follow
    Advisory    = 0,
    /// Should be followed unless compelling reason
    Normal      = 1,
    /// Must be followed unless emergency
    Strong      = 2,
    /// NEVER violate under ANY circumstances
    Inviolable  = 3,
}

// ============================================================================
// SYSTEM PRINCIPLE
// ============================================================================

/// A fundamental system principle that the conscience enforces
#[derive(Debug, Clone)]
pub struct SystemPrinciple {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub level: PrincipleLevel,
    /// Whether this principle can ever be overridden
    pub inviolable: bool,
    /// Importance weight (0.0 – 1.0)
    pub importance: f32,
    /// How many times this principle was checked
    pub check_count: u64,
    /// How many times this principle was violated
    pub violation_count: u64,
    /// EMA-smoothed compliance rate
    pub compliance_rate: f32,
    /// Tick when principle was established
    pub established_tick: u64,
}

impl SystemPrinciple {
    pub fn new(name: String, description: String, level: PrincipleLevel, tick: u64) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        let inviolable = level == PrincipleLevel::Inviolable;
        let importance = match level {
            PrincipleLevel::Advisory => 0.3,
            PrincipleLevel::Normal => 0.5,
            PrincipleLevel::Strong => 0.8,
            PrincipleLevel::Inviolable => 1.0,
        };
        Self {
            id,
            name,
            description,
            level,
            inviolable,
            importance,
            check_count: 0,
            violation_count: 0,
            compliance_rate: 1.0,
            established_tick: tick,
        }
    }

    /// Record a compliance check result
    #[inline]
    pub fn record_check(&mut self, compliant: bool) {
        self.check_count += 1;
        if !compliant {
            self.violation_count += 1;
        }
        let val = if compliant { 1.0 } else { 0.0 };
        self.compliance_rate += EMA_ALPHA * (val - self.compliance_rate);
    }
}

// ============================================================================
// ETHICAL CHECK RESULT
// ============================================================================

/// Verdict from an ethical check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EthicalVerdict {
    /// Action is fully compliant
    Approved,
    /// Action is compliant but with warnings
    ApprovedWithWarnings,
    /// Action violates a non-inviolable principle
    Cautioned,
    /// Action is VETOED — violates an inviolable principle
    Vetoed,
}

/// Result of a system ethical check
#[derive(Debug, Clone)]
pub struct EthicalCheckResult {
    pub action_name: String,
    pub verdict: EthicalVerdict,
    pub overall_compliance: f32,
    pub violated_principles: Vec<u64>,
    pub warning_principles: Vec<u64>,
    pub tick: u64,
}

// ============================================================================
// INJUSTICE RECORD
// ============================================================================

/// A detected system injustice
#[derive(Debug, Clone)]
pub struct InjusticeRecord {
    pub id: u64,
    pub description: String,
    pub affected_entity: String,
    pub severity: f32,
    pub principle_violated: u64,
    pub detection_tick: u64,
    pub resolved: bool,
    pub resolution_tick: u64,
}

// ============================================================================
// OVERRIDE EVENT
// ============================================================================

/// Record of a conscience override
#[derive(Debug, Clone)]
pub struct OverrideEvent {
    pub action_name: String,
    pub principle_id: u64,
    pub justification: String,
    pub cost: f32,
    pub tick: u64,
    pub approved: bool,
}

// ============================================================================
// STATS
// ============================================================================

/// Conscience statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticConscienceStats {
    pub total_checks: u64,
    pub total_approvals: u64,
    pub total_vetoes: u64,
    pub total_overrides_attempted: u64,
    pub total_overrides_approved: u64,
    pub total_injustices_detected: u64,
    pub injustices_resolved: u64,
    pub average_compliance: f32,
    pub override_debt: f32,
}

// ============================================================================
// HOLISTIC CONSCIENCE
// ============================================================================

/// The system-wide ethical framework. Checks every decision against the
/// principle hierarchy and vetoes actions that violate inviolable principles.
pub struct HolisticConscience {
    /// The principle hierarchy
    principles: BTreeMap<u64, SystemPrinciple>,
    /// Check history ring buffer
    check_history: Vec<EthicalCheckResult>,
    check_write_idx: usize,
    /// Injustice records
    injustices: Vec<InjusticeRecord>,
    /// Override log
    overrides: Vec<OverrideEvent>,
    /// Stats
    stats: HolisticConscienceStats,
    /// PRNG
    rng: u64,
    /// Tick
    tick: u64,
}

impl HolisticConscience {
    /// Create a new holistic conscience
    pub fn new(seed: u64) -> Self {
        let mut check_history = Vec::with_capacity(MAX_HISTORY);
        for _ in 0..MAX_HISTORY {
            check_history.push(EthicalCheckResult {
                action_name: String::new(),
                verdict: EthicalVerdict::Approved,
                overall_compliance: 1.0,
                violated_principles: Vec::new(),
                warning_principles: Vec::new(),
                tick: 0,
            });
        }
        Self {
            principles: BTreeMap::new(),
            check_history,
            check_write_idx: 0,
            injustices: Vec::with_capacity(MAX_VIOLATIONS),
            overrides: Vec::new(),
            stats: HolisticConscienceStats {
                total_checks: 0,
                total_approvals: 0,
                total_vetoes: 0,
                total_overrides_attempted: 0,
                total_overrides_approved: 0,
                total_injustices_detected: 0,
                injustices_resolved: 0,
                average_compliance: 1.0,
                override_debt: 0.0,
            },
            rng: seed ^ 0xC0A5_C1E4_DEAD_CAFE,
            tick: 0,
        }
    }

    /// Check an action against ALL principles
    pub fn system_ethical_check(&mut self, action_name: &str, compliance_scores: &BTreeMap<u64, f32>, tick: u64) -> EthicalCheckResult {
        self.tick = tick;
        let mut violated = Vec::new();
        let mut warned = Vec::new();
        let mut inviolable_violated = false;
        let mut total_compliance = 0.0f32;
        let mut count = 0u32;

        for (pid, principle) in self.principles.iter_mut() {
            let score = compliance_scores.get(pid).copied().unwrap_or(1.0);
            let compliant = score >= COMPLIANCE_GOOD;
            principle.record_check(compliant);
            total_compliance += score;
            count += 1;

            if score < COMPLIANCE_WARN {
                violated.push(*pid);
                if principle.inviolable {
                    inviolable_violated = true;
                }
            } else if score < COMPLIANCE_GOOD {
                warned.push(*pid);
            }
        }

        let overall = if count > 0 { total_compliance / count as f32 } else { 1.0 };
        let verdict = if inviolable_violated {
            self.stats.total_vetoes += 1;
            EthicalVerdict::Vetoed
        } else if !violated.is_empty() {
            EthicalVerdict::Cautioned
        } else if !warned.is_empty() {
            self.stats.total_approvals += 1;
            EthicalVerdict::ApprovedWithWarnings
        } else {
            self.stats.total_approvals += 1;
            EthicalVerdict::Approved
        };

        let result = EthicalCheckResult {
            action_name: String::from(action_name),
            verdict,
            overall_compliance: overall,
            violated_principles: violated,
            warning_principles: warned,
            tick,
        };

        self.check_history[self.check_write_idx] = result.clone();
        self.check_write_idx = (self.check_write_idx + 1) % MAX_HISTORY;
        self.stats.total_checks += 1;
        self.stats.average_compliance += EMA_ALPHA * (overall - self.stats.average_compliance);
        result
    }

    /// Get the overall principle compliance score
    #[inline]
    pub fn principle_compliance(&self) -> f32 {
        if self.principles.is_empty() {
            return 1.0;
        }
        let total: f32 = self.principles.values().map(|p| p.compliance_rate * p.importance).sum();
        let weight: f32 = self.principles.values().map(|p| p.importance).sum();
        if weight > 0.0 { total / weight } else { 1.0 }
    }

    /// Detect injustice across the system
    pub fn detect_system_injustice(&mut self, entity: &str, fairness_score: f32, tick: u64) -> Option<InjusticeRecord> {
        self.tick = tick;
        if fairness_score < INJUSTICE_THRESHOLD {
            // Find which principle is most relevant
            let most_relevant = self.principles.values()
                .filter(|p| p.compliance_rate < COMPLIANCE_WARN)
                .min_by(|a, b| a.compliance_rate.partial_cmp(&b.compliance_rate).unwrap_or(core::cmp::Ordering::Equal))
                .map(|p| p.id)
                .unwrap_or(0);

            let rng_val = xorshift64(&mut self.rng);
            let record = InjusticeRecord {
                id: rng_val,
                description: String::from("fairness violation detected"),
                affected_entity: String::from(entity),
                severity: 1.0 - fairness_score,
                principle_violated: most_relevant,
                detection_tick: tick,
                resolved: false,
                resolution_tick: 0,
            };
            if self.injustices.len() < MAX_VIOLATIONS {
                self.injustices.push(record.clone());
            }
            self.stats.total_injustices_detected += 1;
            return Some(record);
        }
        None
    }

    /// Compute fairness across all tracked dimensions
    #[inline(always)]
    pub fn fairness_across_all(&self) -> f32 {
        self.principle_compliance()
    }

    /// Attempt a conscience override for a specific principle
    pub fn conscience_override(&mut self, action: &str, principle_id: u64, justification: &str, tick: u64) -> bool {
        self.tick = tick;
        self.stats.total_overrides_attempted += 1;
        let approved = if let Some(principle) = self.principles.get(&principle_id) {
            // Inviolable principles can NEVER be overridden
            if principle.inviolable {
                false
            } else {
                // Non-inviolable can be overridden with debt
                true
            }
        } else {
            false
        };
        let event = OverrideEvent {
            action_name: String::from(action),
            principle_id,
            justification: String::from(justification),
            cost: OVERRIDE_COST,
            tick,
            approved,
        };
        self.overrides.push(event);
        if approved {
            self.stats.total_overrides_approved += 1;
            self.stats.override_debt += OVERRIDE_COST;
        }
        approved
    }

    /// The conscience's moral authority score: how trustworthy is our ethical framework?
    #[inline]
    pub fn moral_authority(&self) -> f32 {
        let compliance = self.principle_compliance();
        let consistency = if self.stats.total_checks > 0 {
            1.0 - (self.stats.total_vetoes as f32 / self.stats.total_checks as f32)
        } else {
            1.0
        };
        let debt_penalty = (self.stats.override_debt / 100.0).min(0.5);
        (compliance * 0.4 + consistency * 0.4 - debt_penalty * 0.2).max(0.0).min(1.0)
    }

    /// Get the principle hierarchy — sorted by level then importance
    #[inline]
    pub fn principle_hierarchy(&self) -> Vec<(u64, String, PrincipleLevel, f32)> {
        let mut hierarchy: Vec<_> = self.principles.values()
            .map(|p| (p.id, p.name.clone(), p.level, p.importance))
            .collect();
        hierarchy.sort_by(|a, b| {
            b.2.cmp(&a.2).then(b.3.partial_cmp(&a.3).unwrap_or(core::cmp::Ordering::Equal))
        });
        hierarchy
    }

    /// Register a new principle
    #[inline]
    pub fn register_principle(&mut self, principle: SystemPrinciple) {
        if self.principles.len() < MAX_PRINCIPLES {
            self.principles.insert(principle.id, principle);
        }
    }

    /// Resolve an injustice
    #[inline]
    pub fn resolve_injustice(&mut self, injustice_id: u64, tick: u64) -> bool {
        for inj in self.injustices.iter_mut() {
            if inj.id == injustice_id && !inj.resolved {
                inj.resolved = true;
                inj.resolution_tick = tick;
                self.stats.injustices_resolved += 1;
                return true;
            }
        }
        false
    }

    /// Active injustices
    #[inline(always)]
    pub fn active_injustices(&self) -> Vec<&InjusticeRecord> {
        self.injustices.iter().filter(|i| !i.resolved).collect()
    }

    /// Principle count
    #[inline(always)]
    pub fn principle_count(&self) -> usize {
        self.principles.len()
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticConscienceStats {
        &self.stats
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_principle_creation() {
        let p = SystemPrinciple::new(
            String::from("no_starvation"),
            String::from("Never starve a critical process"),
            PrincipleLevel::Inviolable,
            1,
        );
        assert!(p.inviolable);
        assert_eq!(p.importance, 1.0);
        assert_eq!(p.compliance_rate, 1.0);
    }

    #[test]
    fn test_conscience_creation() {
        let conscience = HolisticConscience::new(42);
        assert_eq!(conscience.principle_count(), 0);
        assert_eq!(conscience.principle_compliance(), 1.0);
    }

    #[test]
    fn test_moral_authority() {
        let conscience = HolisticConscience::new(42);
        let authority = conscience.moral_authority();
        assert!(authority > 0.0);
    }

    #[test]
    fn test_inviolable_override_rejected() {
        let mut conscience = HolisticConscience::new(42);
        let p = SystemPrinciple::new(
            String::from("never_corrupt"),
            String::from("Never corrupt memory"),
            PrincipleLevel::Inviolable,
            1,
        );
        let pid = p.id;
        conscience.register_principle(p);
        let result = conscience.conscience_override("risky_action", pid, "testing", 2);
        assert!(!result); // Inviolable cannot be overridden
    }

    #[test]
    fn test_fnv1a() {
        assert_eq!(fnv1a_hash(b"conscience"), fnv1a_hash(b"conscience"));
    }
}
