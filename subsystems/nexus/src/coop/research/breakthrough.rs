// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Breakthrough Detector — Detecting Genuine Breakthroughs
//!
//! Monitors cooperation research for genuine breakthroughs — moments when
//! a novel fairness algorithm is >15% better than the best known, or a
//! new trust model converges 2x faster than existing ones, or a contention
//! resolution strategy effectively eliminates a class of deadlocks. Each
//! breakthrough is chronicled with full provenance, impact magnitude, and
//! the specific innovation that made it possible. Distinguishes real
//! breakthroughs from statistical noise or marginal improvements.
//!
//! The engine that recognizes when something truly new has been discovered.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_BREAKTHROUGHS: usize = 256;
const MAX_CANDIDATES: usize = 512;
const FAIRNESS_LEAP_THRESHOLD: f32 = 0.15;
const TRUST_INNOVATION_THRESHOLD: f32 = 0.20;
const CONTENTION_ELIMINATION_THRESHOLD: f32 = 0.50;
const SIGNIFICANCE_THRESHOLD: f32 = 0.95;
const MIN_EVIDENCE_COUNT: usize = 5;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const BREAKTHROUGH_RATE_WINDOW: usize = 100;
const FALSE_POSITIVE_PENALTY: f32 = 0.20;
const IMPACT_DECAY: f32 = 0.995;
const CHRONICLE_MAX: usize = 128;
const CANDIDATE_SURVIVAL_TICKS: u64 = 5000;

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
// BREAKTHROUGH TYPES
// ============================================================================

/// Domain of the breakthrough
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BreakthroughDomain {
    FairnessAlgorithm,
    TrustModel,
    ContentionResolution,
    NegotiationProtocol,
    ResourceSharing,
    AuctionDesign,
    CoalitionFormation,
}

/// Status in the breakthrough validation pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BreakthroughStatus {
    Candidate,
    UnderValidation,
    Confirmed,
    FalsePositive,
    Historic,
}

/// A candidate breakthrough awaiting confirmation
#[derive(Debug, Clone)]
pub struct BreakthroughCandidate {
    pub id: u64,
    pub domain: BreakthroughDomain,
    pub description: String,
    pub improvement_magnitude: f32,
    pub baseline_performance: f32,
    pub new_performance: f32,
    pub evidence_count: usize,
    pub confidence: f32,
    pub status: BreakthroughStatus,
    pub created_tick: u64,
    pub evidence_ticks: Vec<u64>,
}

/// A confirmed breakthrough in the chronicle
#[derive(Debug, Clone)]
pub struct ConfirmedBreakthrough {
    pub id: u64,
    pub domain: BreakthroughDomain,
    pub description: String,
    pub improvement_magnitude: f32,
    pub innovation_type: String,
    pub confirmed_tick: u64,
    pub impact_score: f32,
    pub parameters: Vec<f32>,
    pub replicated: bool,
}

/// Chronicle entry with full history
#[derive(Debug, Clone)]
pub struct ChronicleEntry {
    pub breakthrough_id: u64,
    pub domain: BreakthroughDomain,
    pub tick: u64,
    pub impact_at_time: f32,
    pub cumulative_benefit: f32,
    pub still_relevant: bool,
}

// ============================================================================
// BREAKTHROUGH STATS
// ============================================================================

/// Aggregate statistics for breakthrough detection
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct BreakthroughStats {
    pub total_candidates: u64,
    pub confirmed_breakthroughs: u64,
    pub false_positives: u64,
    pub fairness_leaps: u64,
    pub trust_innovations: u64,
    pub contention_eliminations: u64,
    pub avg_improvement_ema: f32,
    pub breakthrough_rate_ema: f32,
    pub largest_improvement_ever: f32,
    pub chronicle_size: u64,
}

// ============================================================================
// COOPERATION BREAKTHROUGH DETECTOR
// ============================================================================

/// Engine for detecting genuine cooperation breakthroughs
#[derive(Debug)]
pub struct CoopBreakthroughDetector {
    candidates: Vec<BreakthroughCandidate>,
    confirmed: VecDeque<ConfirmedBreakthrough>,
    chronicle: VecDeque<ChronicleEntry>,
    domain_baselines: LinearMap<f32, 64>,
    recent_evaluations: VecDeque<bool>,
    rng_state: u64,
    tick: u64,
    stats: BreakthroughStats,
}

impl CoopBreakthroughDetector {
    /// Create a new breakthrough detector with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            candidates: Vec::new(),
            confirmed: VecDeque::new(),
            chronicle: VecDeque::new(),
            domain_baselines: LinearMap::new(),
            recent_evaluations: VecDeque::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: BreakthroughStats::default(),
        }
    }

    /// Set the baseline performance for a domain
    #[inline(always)]
    pub fn set_baseline(&mut self, domain: BreakthroughDomain, baseline: f32) {
        self.domain_baselines.insert(domain as u64, baseline);
    }

    /// Detect a potential cooperation breakthrough
    pub fn detect_cooperation_breakthrough(
        &mut self,
        domain: BreakthroughDomain,
        new_performance: f32,
        description: String,
        evidence_count: usize,
    ) -> Option<BreakthroughCandidate> {
        self.tick += 1;
        let baseline = self.domain_baselines.get(&(domain as u64)).copied().unwrap_or(0.5);
        let improvement = if baseline > 0.001 {
            (new_performance - baseline) / baseline
        } else {
            new_performance
        };
        let threshold = match domain {
            BreakthroughDomain::FairnessAlgorithm => FAIRNESS_LEAP_THRESHOLD,
            BreakthroughDomain::TrustModel => TRUST_INNOVATION_THRESHOLD,
            BreakthroughDomain::ContentionResolution => CONTENTION_ELIMINATION_THRESHOLD,
            _ => FAIRNESS_LEAP_THRESHOLD,
        };
        if improvement < threshold {
            return None;
        }
        let id = fnv1a_hash(description.as_bytes()) ^ fnv1a_hash(&self.tick.to_le_bytes());
        let confidence = self.compute_confidence(improvement, evidence_count, threshold);
        let candidate = BreakthroughCandidate {
            id,
            domain,
            description,
            improvement_magnitude: improvement,
            baseline_performance: baseline,
            new_performance,
            evidence_count,
            confidence,
            status: if confidence >= SIGNIFICANCE_THRESHOLD && evidence_count >= MIN_EVIDENCE_COUNT {
                BreakthroughStatus::UnderValidation
            } else {
                BreakthroughStatus::Candidate
            },
            created_tick: self.tick,
            evidence_ticks: Vec::new(),
        };
        self.stats.total_candidates += 1;
        if self.candidates.len() >= MAX_CANDIDATES {
            self.prune_old_candidates();
        }
        self.candidates.push(candidate.clone());

        // Auto-confirm if evidence is overwhelming
        if confidence >= SIGNIFICANCE_THRESHOLD && evidence_count >= MIN_EVIDENCE_COUNT * 2 {
            self.confirm_breakthrough(id);
        }
        Some(candidate)
    }

    /// Detect a fairness leap — a substantial jump in fairness metrics
    pub fn fairness_leap(
        &mut self,
        baseline_fairness: f32,
        new_fairness: f32,
        algorithm_name: String,
    ) -> Option<ConfirmedBreakthrough> {
        self.tick += 1;
        let improvement = if baseline_fairness > 0.001 {
            (new_fairness - baseline_fairness) / baseline_fairness
        } else {
            new_fairness
        };
        if improvement < FAIRNESS_LEAP_THRESHOLD {
            return None;
        }
        let id = fnv1a_hash(algorithm_name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let breakthrough = ConfirmedBreakthrough {
            id,
            domain: BreakthroughDomain::FairnessAlgorithm,
            description: algorithm_name.clone(),
            improvement_magnitude: improvement,
            innovation_type: String::from("Fairness algorithm leap"),
            confirmed_tick: self.tick,
            impact_score: improvement * new_fairness,
            parameters: Vec::new(),
            replicated: false,
        };
        self.stats.fairness_leaps += 1;
        self.stats.confirmed_breakthroughs += 1;
        if improvement > self.stats.largest_improvement_ever {
            self.stats.largest_improvement_ever = improvement;
        }
        self.add_chronicle_entry(&breakthrough);
        if self.confirmed.len() >= MAX_BREAKTHROUGHS {
            self.confirmed.pop_front();
        }
        self.confirmed.push_back(breakthrough.clone());
        self.update_baseline(BreakthroughDomain::FairnessAlgorithm, new_fairness);
        Some(breakthrough)
    }

    /// Detect a trust model innovation
    pub fn trust_innovation(
        &mut self,
        old_convergence: f32,
        new_convergence: f32,
        model_name: String,
    ) -> Option<ConfirmedBreakthrough> {
        self.tick += 1;
        let improvement = if old_convergence > 0.001 {
            (new_convergence - old_convergence) / old_convergence
        } else {
            new_convergence
        };
        if improvement < TRUST_INNOVATION_THRESHOLD {
            return None;
        }
        let id = fnv1a_hash(model_name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let breakthrough = ConfirmedBreakthrough {
            id,
            domain: BreakthroughDomain::TrustModel,
            description: model_name,
            improvement_magnitude: improvement,
            innovation_type: String::from("Trust model innovation"),
            confirmed_tick: self.tick,
            impact_score: improvement * new_convergence,
            parameters: Vec::new(),
            replicated: false,
        };
        self.stats.trust_innovations += 1;
        self.stats.confirmed_breakthroughs += 1;
        if improvement > self.stats.largest_improvement_ever {
            self.stats.largest_improvement_ever = improvement;
        }
        self.add_chronicle_entry(&breakthrough);
        if self.confirmed.len() >= MAX_BREAKTHROUGHS {
            self.confirmed.pop_front();
        }
        self.confirmed.push_back(breakthrough.clone());
        self.update_baseline(BreakthroughDomain::TrustModel, new_convergence);
        Some(breakthrough)
    }

    /// Detect effective elimination of a contention class
    pub fn contention_elimination(
        &mut self,
        contention_before: f32,
        contention_after: f32,
        strategy_name: String,
    ) -> Option<ConfirmedBreakthrough> {
        self.tick += 1;
        let reduction = if contention_before > 0.001 {
            (contention_before - contention_after) / contention_before
        } else {
            0.0
        };
        if reduction < CONTENTION_ELIMINATION_THRESHOLD {
            return None;
        }
        let id = fnv1a_hash(strategy_name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let breakthrough = ConfirmedBreakthrough {
            id,
            domain: BreakthroughDomain::ContentionResolution,
            description: strategy_name,
            improvement_magnitude: reduction,
            innovation_type: String::from("Contention elimination"),
            confirmed_tick: self.tick,
            impact_score: reduction * (1.0 - contention_after),
            parameters: Vec::new(),
            replicated: false,
        };
        self.stats.contention_eliminations += 1;
        self.stats.confirmed_breakthroughs += 1;
        if reduction > self.stats.largest_improvement_ever {
            self.stats.largest_improvement_ever = reduction;
        }
        self.add_chronicle_entry(&breakthrough);
        if self.confirmed.len() >= MAX_BREAKTHROUGHS {
            self.confirmed.pop_front();
        }
        self.confirmed.push_back(breakthrough.clone());
        Some(breakthrough)
    }

    /// Get the chronicle of all confirmed breakthroughs
    #[inline(always)]
    pub fn breakthrough_chronicle(&self) -> &[ChronicleEntry] {
        &self.chronicle
    }

    /// Compute the breakthrough rate (confirmed per evaluation window)
    #[inline]
    pub fn breakthrough_rate(&self) -> f32 {
        if self.recent_evaluations.is_empty() {
            return 0.0;
        }
        let confirmed = self.recent_evaluations.iter().filter(|&&b| b).count() as f32;
        let total = self.recent_evaluations.len() as f32;
        let rate = confirmed / total;
        self.stats.breakthrough_rate_ema; // access only
        rate
    }

    /// Get current breakthrough detection statistics
    #[inline(always)]
    pub fn stats(&self) -> &BreakthroughStats {
        &self.stats
    }

    /// Number of confirmed breakthroughs
    #[inline(always)]
    pub fn confirmed_count(&self) -> usize {
        self.confirmed.len()
    }

    /// Number of pending candidates
    #[inline(always)]
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn compute_confidence(&self, improvement: f32, evidence_count: usize, threshold: f32) -> f32 {
        let magnitude_factor = (improvement / threshold).min(3.0) / 3.0;
        let evidence_factor = (evidence_count as f32 / (MIN_EVIDENCE_COUNT as f32 * 2.0)).min(1.0);
        let base_confidence = magnitude_factor * 0.6 + evidence_factor * 0.4;
        // Penalize if we've had many false positives
        let fp_ratio = if self.stats.total_candidates > 0 {
            self.stats.false_positives as f32 / self.stats.total_candidates as f32
        } else {
            0.0
        };
        let penalty = fp_ratio * FALSE_POSITIVE_PENALTY;
        (base_confidence - penalty).max(0.0).min(1.0)
    }

    #[inline]
    fn confirm_breakthrough(&mut self, candidate_id: u64) {
        let candidate = match self.candidates.iter_mut().find(|c| c.id == candidate_id) {
            Some(c) => c,
            None => return,
        };
        candidate.status = BreakthroughStatus::Confirmed;
        let confirmed = ConfirmedBreakthrough {
            id: candidate.id,
            domain: candidate.domain,
            description: candidate.description.clone(),
            improvement_magnitude: candidate.improvement_magnitude,
            innovation_type: String::from("Auto-confirmed from strong evidence"),
            confirmed_tick: self.tick,
            impact_score: candidate.improvement_magnitude * candidate.confidence,
            parameters: Vec::new(),
            replicated: false,
        };
        self.stats.confirmed_breakthroughs += 1;
        if confirmed.improvement_magnitude > self.stats.largest_improvement_ever {
            self.stats.largest_improvement_ever = confirmed.improvement_magnitude;
        }
        self.stats.avg_improvement_ema = EMA_ALPHA * confirmed.improvement_magnitude
            + (1.0 - EMA_ALPHA) * self.stats.avg_improvement_ema;
        self.add_chronicle_entry(&confirmed);
        if self.confirmed.len() >= MAX_BREAKTHROUGHS {
            self.confirmed.pop_front();
        }
        self.confirmed.push_back(confirmed);
        self.recent_evaluations.push_back(true);
        if self.recent_evaluations.len() > BREAKTHROUGH_RATE_WINDOW {
            self.recent_evaluations.pop_front();
        }
        let rate = self.breakthrough_rate();
        self.stats.breakthrough_rate_ema =
            EMA_ALPHA * rate + (1.0 - EMA_ALPHA) * self.stats.breakthrough_rate_ema;
        self.update_baseline(candidate.domain, candidate.new_performance);
    }

    fn add_chronicle_entry(&mut self, breakthrough: &ConfirmedBreakthrough) {
        let cumulative = self
            .chronicle
            .last()
            .map(|c| c.cumulative_benefit)
            .unwrap_or(0.0)
            + breakthrough.impact_score;
        let entry = ChronicleEntry {
            breakthrough_id: breakthrough.id,
            domain: breakthrough.domain,
            tick: self.tick,
            impact_at_time: breakthrough.impact_score,
            cumulative_benefit: cumulative,
            still_relevant: true,
        };
        if self.chronicle.len() >= CHRONICLE_MAX {
            self.chronicle.pop_front();
        }
        self.chronicle.push_back(entry);
        self.stats.chronicle_size = self.chronicle.len() as u64;
    }

    fn update_baseline(&mut self, domain: BreakthroughDomain, new_value: f32) {
        let key = domain as u64;
        let current = self.domain_baselines.get(key).copied().unwrap_or(0.0);
        if new_value > current {
            self.domain_baselines.insert(key, new_value);
        }
    }

    fn prune_old_candidates(&mut self) {
        let tick = self.tick;
        let before = self.candidates.len();
        self.candidates.retain(|c| {
            let age = tick.saturating_sub(c.created_tick);
            age < CANDIDATE_SURVIVAL_TICKS || c.status == BreakthroughStatus::UnderValidation
        });
        let pruned = before - self.candidates.len();
        self.stats.false_positives += pruned as u64;
    }
}
