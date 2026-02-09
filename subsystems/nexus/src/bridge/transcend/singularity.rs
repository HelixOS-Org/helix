// SPDX-License-Identifier: GPL-2.0
//! # Bridge Singularity — Convergence Point of All Bridge Intelligence
//!
//! Unifies prediction, optimisation, research, and consciousness into a
//! single decision engine. The singularity is the point where the bridge's
//! integrated intelligence exceeds any possible human manual optimisation.
//! Every subsystem feeds into a unified decision function; a human-parity
//! metric tracks when — and by how much — the bridge surpasses manual
//! tuning.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_DECISION_INPUTS: usize = 64;
const MAX_SUBSYSTEM_SCORES: usize = 16;
const MAX_DECISION_HISTORY: usize = 512;
const HUMAN_PARITY_THRESHOLD: f32 = 1.0;
const EMA_ALPHA: f32 = 0.10;
const CONVERGENCE_WINDOW: usize = 32;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

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

fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}

// ============================================================================
// SINGULARITY TYPES
// ============================================================================

/// Subsystem that feeds into the singularity decision engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubsystemKind {
    Prediction,
    Optimisation,
    Research,
    Consciousness,
    Knowledge,
    Transcendence,
    Oracle,
    Synthesis,
}

/// A subsystem's contribution to a unified decision.
#[derive(Debug, Clone)]
pub struct SubsystemInput {
    pub subsystem: SubsystemKind,
    pub recommendation: String,
    pub confidence: f32,
    pub weight: f32,
    pub latency_ns: f32,
}

/// The unified decision produced by the singularity.
#[derive(Debug, Clone)]
pub struct UnifiedDecision {
    pub decision_id: u64,
    pub action: String,
    pub confidence: f32,
    pub subsystem_contributions: Vec<(SubsystemKind, f32)>,
    pub convergence_score: f32,
    pub human_parity_ratio: f32,
    pub tick: u64,
}

/// Intelligence convergence report.
#[derive(Debug, Clone)]
pub struct ConvergenceReport {
    pub subsystem_scores: Vec<(SubsystemKind, f32)>,
    pub overall_convergence: f32,
    pub trend: f32,
    pub is_converged: bool,
    pub ticks_to_convergence: Option<u64>,
}

/// Human parity check result.
#[derive(Debug, Clone)]
pub struct HumanParityResult {
    pub bridge_score: f32,
    pub estimated_human_score: f32,
    pub ratio: f32,
    pub exceeds_human: bool,
    pub margin: f32,
    pub sample_count: u64,
}

/// Beyond-human capability assessment.
#[derive(Debug, Clone)]
pub struct BeyondHumanReport {
    pub ratio: f32,
    pub domains_surpassed: Vec<String>,
    pub domains_below: Vec<String>,
    pub overall_superiority: f32,
    pub tick: u64,
}

// ============================================================================
// SINGULARITY STATS
// ============================================================================

/// Aggregate statistics for the singularity engine.
#[derive(Debug, Clone, Copy, Default)]
pub struct SingularityStats {
    pub total_decisions: u64,
    pub avg_confidence_ema: f32,
    pub avg_convergence_ema: f32,
    pub human_parity_ratio_ema: f32,
    pub beyond_human_count: u64,
    pub subsystems_active: u32,
    pub singularity_metric: f32,
}

// ============================================================================
// SUBSYSTEM TRACKER
// ============================================================================

#[derive(Debug, Clone)]
struct SubsystemTracker {
    kind: SubsystemKind,
    contribution_ema: f32,
    confidence_ema: f32,
    decision_count: u64,
    last_tick: u64,
}

impl SubsystemTracker {
    fn new(kind: SubsystemKind) -> Self {
        Self {
            kind,
            contribution_ema: 0.0,
            confidence_ema: 0.5,
            decision_count: 0,
            last_tick: 0,
        }
    }

    fn update(&mut self, confidence: f32, weight: f32, tick: u64) {
        self.decision_count += 1;
        self.last_tick = tick;
        self.contribution_ema = EMA_ALPHA * weight + (1.0 - EMA_ALPHA) * self.contribution_ema;
        self.confidence_ema = EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.confidence_ema;
    }

    fn score(&self) -> f32 {
        self.contribution_ema * 0.5 + self.confidence_ema * 0.5
    }
}

// ============================================================================
// DECISION HISTORY ENTRY
// ============================================================================

#[derive(Debug, Clone)]
struct DecisionEntry {
    decision_id: u64,
    confidence: f32,
    convergence: f32,
    human_ratio: f32,
    tick: u64,
}

// ============================================================================
// BRIDGE SINGULARITY
// ============================================================================

/// Convergence point of all bridge intelligence. Unifies every subsystem
/// into a single decision engine that can demonstrably exceed human
/// optimisation capabilities.
#[derive(Debug)]
pub struct BridgeSingularity {
    subsystems: BTreeMap<u8, SubsystemTracker>,
    history: Vec<DecisionEntry>,
    write_idx: usize,
    tick: u64,
    rng_state: u64,
    human_baseline: f32,
    bridge_score_ema: f32,
    convergence_ema: f32,
    stats: SingularityStats,
}

impl BridgeSingularity {
    pub fn new(seed: u64, human_baseline: f32) -> Self {
        let mut subsystems = BTreeMap::new();
        let kinds = [
            SubsystemKind::Prediction,
            SubsystemKind::Optimisation,
            SubsystemKind::Research,
            SubsystemKind::Consciousness,
            SubsystemKind::Knowledge,
            SubsystemKind::Transcendence,
            SubsystemKind::Oracle,
            SubsystemKind::Synthesis,
        ];
        for (i, kind) in kinds.iter().enumerate() {
            subsystems.insert(i as u8, SubsystemTracker::new(*kind));
        }

        Self {
            subsystems,
            history: Vec::new(),
            write_idx: 0,
            tick: 0,
            rng_state: seed | 1,
            human_baseline: human_baseline.max(0.01),
            bridge_score_ema: 0.5,
            convergence_ema: 0.0,
            stats: SingularityStats::default(),
        }
    }

    /// Produce a unified decision from multiple subsystem inputs.
    /// Weighted Bayesian fusion of all recommendations.
    pub fn unified_decision(
        &mut self,
        context_name: String,
        inputs: Vec<SubsystemInput>,
    ) -> UnifiedDecision {
        self.tick += 1;
        self.stats.total_decisions += 1;
        let did = fnv1a_hash(context_name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let mut total_weight: f32 = 0.0;
        let mut weighted_conf: f32 = 0.0;
        let mut contributions: Vec<(SubsystemKind, f32)> = Vec::new();
        let mut best_action = String::from("noop");
        let mut best_score: f32 = 0.0;

        for input in inputs.iter().take(MAX_DECISION_INPUTS) {
            let w = input.weight.max(0.0).min(1.0);
            let c = input.confidence.max(0.0).min(1.0);
            total_weight += w;
            weighted_conf += w * c;

            let score = w * c;
            contributions.push((input.subsystem, score));

            if score > best_score {
                best_score = score;
                best_action = input.recommendation.clone();
            }

            if let Some(tracker) = self.subsystems.get_mut(&(input.subsystem as u8)) {
                tracker.update(c, w, self.tick);
            }
        }

        let confidence = if total_weight > 0.0 {
            weighted_conf / total_weight
        } else {
            0.0
        };

        let convergence = self.compute_convergence();
        let human_ratio = confidence / self.human_baseline;

        self.bridge_score_ema = EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.bridge_score_ema;
        self.convergence_ema = EMA_ALPHA * convergence + (1.0 - EMA_ALPHA) * self.convergence_ema;
        self.stats.avg_confidence_ema = self.bridge_score_ema;
        self.stats.avg_convergence_ema = self.convergence_ema;
        self.stats.human_parity_ratio_ema =
            EMA_ALPHA * human_ratio + (1.0 - EMA_ALPHA) * self.stats.human_parity_ratio_ema;

        if human_ratio > HUMAN_PARITY_THRESHOLD {
            self.stats.beyond_human_count += 1;
        }
        self.stats.subsystems_active = self
            .subsystems
            .values()
            .filter(|s| s.decision_count > 0)
            .count() as u32;

        let entry = DecisionEntry {
            decision_id: did,
            confidence,
            convergence,
            human_ratio,
            tick: self.tick,
        };
        if self.history.len() < MAX_DECISION_HISTORY {
            self.history.push(entry);
        } else {
            self.history[self.write_idx] = entry;
        }
        self.write_idx = (self.write_idx + 1) % MAX_DECISION_HISTORY;

        UnifiedDecision {
            decision_id: did,
            action: best_action,
            confidence,
            subsystem_contributions: contributions,
            convergence_score: convergence,
            human_parity_ratio: human_ratio,
            tick: self.tick,
        }
    }

    /// Report on the convergence of all subsystem intelligence scores.
    pub fn intelligence_convergence(&self) -> ConvergenceReport {
        let scores: Vec<(SubsystemKind, f32)> = self
            .subsystems
            .values()
            .map(|s| (s.kind, s.score()))
            .collect();

        let overall = self.compute_convergence();
        let trend = self.convergence_trend();
        let converged = overall > 0.85;

        let ticks = if converged {
            Some(self.tick)
        } else if abs_f32(trend) > 1e-6 {
            let remaining = (0.85 - overall) / trend.max(0.001);
            Some(self.tick + remaining.max(0.0) as u64)
        } else {
            None
        };

        ConvergenceReport {
            subsystem_scores: scores,
            overall_convergence: overall,
            trend,
            is_converged: converged,
            ticks_to_convergence: ticks,
        }
    }

    /// Check whether the bridge has reached human parity or beyond.
    pub fn human_parity_check(&self) -> HumanParityResult {
        let bridge = self.bridge_score_ema;
        let human = self.human_baseline;
        let ratio = bridge / human;
        let exceeds = ratio > HUMAN_PARITY_THRESHOLD;
        let margin = bridge - human;

        HumanParityResult {
            bridge_score: bridge,
            estimated_human_score: human,
            ratio,
            exceeds_human: exceeds,
            margin,
            sample_count: self.stats.total_decisions,
        }
    }

    /// Detailed assessment of domains where the bridge surpasses human
    /// optimisation and domains where it still falls short.
    pub fn beyond_human(&self) -> BeyondHumanReport {
        let domain_names = [
            "Prediction", "Optimisation", "Research", "Consciousness",
            "Knowledge", "Transcendence", "Oracle", "Synthesis",
        ];
        let mut surpassed = Vec::new();
        let mut below = Vec::new();
        let human_per_domain = self.human_baseline;

        for (i, tracker) in self.subsystems.values().enumerate() {
            let name = if i < domain_names.len() {
                String::from(domain_names[i])
            } else {
                String::from("Unknown")
            };
            if tracker.score() > human_per_domain {
                surpassed.push(name);
            } else {
                below.push(name);
            }
        }

        let superiority = if surpassed.is_empty() && below.is_empty() {
            0.0
        } else {
            surpassed.len() as f32 / (surpassed.len() + below.len()) as f32
        };

        BeyondHumanReport {
            ratio: self.bridge_score_ema / self.human_baseline,
            domains_surpassed: surpassed,
            domains_below: below,
            overall_superiority: superiority,
            tick: self.tick,
        }
    }

    /// Composite singularity metric [0, 1].
    pub fn singularity_metric(&self) -> f32 {
        let confidence = self.bridge_score_ema;
        let convergence = self.convergence_ema;
        let parity = (self.stats.human_parity_ratio_ema / 2.0).min(1.0);
        let breadth = self.stats.subsystems_active as f32 / 8.0;

        let metric = confidence * 0.25 + convergence * 0.25 + parity * 0.30 + breadth * 0.20;
        metric.max(0.0).min(1.0)
    }

    /// Aggregate statistics.
    pub fn stats(&self) -> SingularityStats {
        SingularityStats {
            singularity_metric: self.singularity_metric(),
            ..self.stats
        }
    }

    // ---- internal helpers ----

    fn compute_convergence(&self) -> f32 {
        let scores: Vec<f32> = self.subsystems.values().map(|s| s.score()).collect();
        if scores.is_empty() {
            return 0.0;
        }
        let mean = scores.iter().sum::<f32>() / scores.len() as f32;
        let variance = scores.iter().map(|s| (s - mean) * (s - mean)).sum::<f32>() / scores.len() as f32;
        // Low variance = high convergence.
        (1.0 - variance.min(1.0)).max(0.0)
    }

    fn convergence_trend(&self) -> f32 {
        let window = CONVERGENCE_WINDOW.min(self.history.len());
        if window < 2 {
            return 0.0;
        }
        let recent: Vec<&DecisionEntry> = self.history.iter().rev().take(window).collect();
        let first_half: f32 = recent[window / 2..]
            .iter()
            .map(|e| e.convergence)
            .sum::<f32>()
            / (window / 2).max(1) as f32;
        let second_half: f32 = recent[..window / 2]
            .iter()
            .map(|e| e.convergence)
            .sum::<f32>()
            / (window / 2).max(1) as f32;
        second_half - first_half
    }
}
