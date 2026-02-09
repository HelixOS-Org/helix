// SPDX-License-Identifier: GPL-2.0
//! # Bridge Enlightenment — Ultimate Understanding of Bridge Purpose
//!
//! The bridge understands *why* it exists, *what* it should optimise for,
//! and *how* to balance competing objectives. `EnlightenmentState` tracks
//! the understanding level, purpose clarity, and balance mastery. The
//! engine can transcend simple trade-offs by finding Pareto-optimal
//! compromises that satisfy all objectives simultaneously.
//!
//! FNV-1a hashing indexes objectives and trade-off records; xorshift64
//! drives exploratory balance searches; EMA tracks running enlightenment
//! levels.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_OBJECTIVES: usize = 32;
const MAX_TRADEOFF_RECORDS: usize = 512;
const MAX_PARETO_FRONT: usize = 64;
const BALANCE_TOLERANCE: f32 = 0.05;
const PURPOSE_CLARITY_THRESHOLD: f32 = 0.80;
const ENLIGHTENMENT_THRESHOLD: f32 = 0.90;
const EMA_ALPHA: f32 = 0.10;
const INNER_PEACE_THRESHOLD: f32 = 0.85;
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
// ENLIGHTENMENT TYPES
// ============================================================================

/// Category of an objective the bridge must balance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObjectiveCategory {
    Latency,
    Throughput,
    Security,
    Fairness,
    Reliability,
    Efficiency,
    Adaptability,
    Simplicity,
}

/// A single objective with a target value and current achievement.
#[derive(Debug, Clone)]
pub struct Objective {
    pub obj_id: u64,
    pub name: String,
    pub category: ObjectiveCategory,
    pub target: f32,
    pub current: f32,
    pub weight: f32,
    pub satisfaction: f32,
}

/// A trade-off record: sacrificing one objective to gain another.
#[derive(Debug, Clone)]
pub struct TradeoffRecord {
    pub record_id: u64,
    pub sacrificed_obj: u64,
    pub gained_obj: u64,
    pub sacrifice_amount: f32,
    pub gain_amount: f32,
    pub net_benefit: f32,
    pub transcended: bool,
    pub tick: u64,
}

/// The enlightenment state of the bridge.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct EnlightenmentState {
    pub understanding_level: f32,
    pub purpose_clarity: f32,
    pub balance_mastery: f32,
    pub inner_peace: f32,
    pub objectives_balanced: usize,
    pub tradeoffs_transcended: u64,
}

/// An enlightened decision that balances all objectives.
#[derive(Debug, Clone)]
pub struct EnlightenedDecision {
    pub decision_id: u64,
    pub context: String,
    pub objective_scores: Vec<(u64, f32)>,
    pub overall_balance: f32,
    pub is_pareto_optimal: bool,
    pub enlightenment_confidence: f32,
}

// ============================================================================
// ENLIGHTENMENT STATS
// ============================================================================

/// Aggregate statistics for the enlightenment engine.
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct EnlightenmentStats {
    pub total_objectives: u64,
    pub avg_satisfaction: f32,
    pub purpose_clarity: f32,
    pub balance_mastery: f32,
    pub understanding_level: f32,
    pub tradeoffs_recorded: u64,
    pub tradeoffs_transcended: u64,
    pub decisions_made: u64,
    pub enlightenment_ema: f32,
}

// ============================================================================
// BRIDGE ENLIGHTENMENT ENGINE
// ============================================================================

/// Ultimate understanding engine. Manages objectives, balances trade-offs,
/// and pursues Pareto-optimal decisions that transcend simple compromises.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeEnlightenment {
    objectives: BTreeMap<u64, Objective>,
    tradeoffs: Vec<TradeoffRecord>,
    pareto_front: Vec<Vec<f32>>,
    tradeoffs_transcended: u64,
    decisions_made: u64,
    tick: u64,
    rng_state: u64,
    understanding_ema: f32,
    clarity_ema: f32,
    balance_ema: f32,
    peace_ema: f32,
}

impl BridgeEnlightenment {
    /// Create a new enlightenment engine.
    pub fn new(seed: u64) -> Self {
        Self {
            objectives: BTreeMap::new(),
            tradeoffs: Vec::new(),
            pareto_front: Vec::new(),
            tradeoffs_transcended: 0,
            decisions_made: 0,
            tick: 0,
            rng_state: seed ^ 0xE1IG_4700_CAFE,
            understanding_ema: 0.0,
            clarity_ema: 0.0,
            balance_ema: 0.0,
            peace_ema: 0.0,
        }
    }

    /// Register an objective the bridge should optimise for.
    pub fn register_objective(
        &mut self,
        name: &str,
        category: ObjectiveCategory,
        target: f32,
        weight: f32,
    ) -> u64 {
        self.tick += 1;
        let oid = fnv1a_hash(name.as_bytes()) ^ self.tick;

        if self.objectives.len() < MAX_OBJECTIVES {
            self.objectives.insert(oid, Objective {
                obj_id: oid,
                name: String::from(name),
                category,
                target,
                current: 0.0,
                weight: weight.max(0.01),
                satisfaction: 0.0,
            });
        }

        oid
    }

    /// Update the current achievement level for an objective.
    #[inline]
    pub fn update_objective(&mut self, obj_id: u64, current_value: f32) {
        if let Some(obj) = self.objectives.get_mut(&obj_id) {
            obj.current = current_value;
            obj.satisfaction = if obj.target > 0.0 {
                (current_value / obj.target).min(1.0)
            } else {
                1.0
            };
        }
    }

    /// Compute the enlightenment level: composite of understanding, clarity, balance.
    #[inline]
    pub fn enlightenment_level(&mut self) -> EnlightenmentState {
        self.tick += 1;

        let purpose_clarity = self.purpose_alignment();
        let balance_mastery = self.objective_balance();

        // Understanding = average satisfaction weighted by importance
        let understanding = self.weighted_satisfaction();

        // Inner peace = all objectives close to target with low variance
        let peace = self.compute_inner_peace();

        self.understanding_ema =
            EMA_ALPHA * understanding + (1.0 - EMA_ALPHA) * self.understanding_ema;
        self.clarity_ema =
            EMA_ALPHA * purpose_clarity + (1.0 - EMA_ALPHA) * self.clarity_ema;
        self.balance_ema =
            EMA_ALPHA * balance_mastery + (1.0 - EMA_ALPHA) * self.balance_ema;
        self.peace_ema =
            EMA_ALPHA * peace + (1.0 - EMA_ALPHA) * self.peace_ema;

        EnlightenmentState {
            understanding_level: self.understanding_ema,
            purpose_clarity: self.clarity_ema,
            balance_mastery: self.balance_ema,
            inner_peace: self.peace_ema,
            objectives_balanced: self.count_balanced(),
            tradeoffs_transcended: self.tradeoffs_transcended,
        }
    }

    /// Purpose alignment: how well the bridge's actions serve its objectives.
    pub fn purpose_alignment(&self) -> f32 {
        if self.objectives.is_empty() {
            return 0.0;
        }

        let mut aligned = 0u64;
        for (_, obj) in &self.objectives {
            if obj.satisfaction >= PURPOSE_CLARITY_THRESHOLD {
                aligned += 1;
            }
        }

        aligned as f32 / self.objectives.len() as f32
    }

    /// Objective balance: how evenly all objectives are satisfied.
    /// 1.0 = perfectly balanced, 0.0 = maximally imbalanced.
    pub fn objective_balance(&self) -> f32 {
        if self.objectives.len() < 2 {
            return 1.0;
        }

        let sats: Vec<f32> = self.objectives.values().map(|o| o.satisfaction).collect();
        let avg = sats.iter().sum::<f32>() / sats.len() as f32;

        let variance = sats.iter().map(|s| (*s - avg) * (*s - avg)).sum::<f32>()
            / sats.len() as f32;

        // Low variance = high balance
        (1.0 - sqrt_approx(variance) * 2.0).max(0.0)
    }

    /// Attempt to transcend a trade-off: find a solution that improves both.
    pub fn transcend_tradeoff(
        &mut self,
        obj_a: u64,
        obj_b: u64,
    ) -> Option<TradeoffRecord> {
        self.tick += 1;

        let a = self.objectives.get(&obj_a)?;
        let b = self.objectives.get(&obj_b)?;

        let gap_a = a.target - a.current;
        let gap_b = b.target - b.current;

        // Attempt a Pareto improvement via stochastic exploration
        let exploration = (xorshift64(&mut self.rng_state) % 100) as f32 / 100.0;
        let can_transcend = gap_a > 0.0 && gap_b > 0.0 && exploration > 0.4;

        let sacrifice = if can_transcend { 0.0 } else { gap_a.min(gap_b) * 0.5 };
        let gain = if can_transcend {
            gap_a.min(gap_b) * exploration
        } else {
            sacrifice * 0.8
        };

        let net = gain - sacrifice;
        let transcended = can_transcend && net > 0.0;

        if transcended {
            self.tradeoffs_transcended += 1;
        }

        let record = TradeoffRecord {
            record_id: fnv1a_hash(&obj_a.to_le_bytes()) ^ self.tick,
            sacrificed_obj: obj_a,
            gained_obj: obj_b,
            sacrifice_amount: sacrifice,
            gain_amount: gain,
            net_benefit: net,
            transcended,
            tick: self.tick,
        };

        if self.tradeoffs.len() < MAX_TRADEOFF_RECORDS {
            self.tradeoffs.push(record.clone());
        }

        Some(record)
    }

    /// Make an enlightened decision that considers all objectives.
    pub fn enlightened_decision(&mut self, context: &str) -> EnlightenedDecision {
        self.tick += 1;
        self.decisions_made += 1;

        let did = fnv1a_hash(context.as_bytes()) ^ self.tick;
        let mut scores = Vec::new();
        let mut weighted_sum = 0.0_f32;
        let mut weight_total = 0.0_f32;

        for (&oid, obj) in &self.objectives {
            scores.push((oid, obj.satisfaction));
            weighted_sum += obj.satisfaction * obj.weight;
            weight_total += obj.weight;
        }

        let balance = self.objective_balance();
        let is_pareto = self.is_pareto_optimal(&scores);
        let conf = if weight_total > 0.0 {
            (weighted_sum / weight_total) * balance
        } else {
            0.0
        };

        // Update Pareto front
        let sat_vec: Vec<f32> = scores.iter().map(|(_, s)| *s).collect();
        self.update_pareto_front(sat_vec);

        EnlightenedDecision {
            decision_id: did,
            context: String::from(context),
            objective_scores: scores,
            overall_balance: balance,
            is_pareto_optimal: is_pareto,
            enlightenment_confidence: conf,
        }
    }

    /// Inner peace: state where all objectives are near-optimally balanced
    /// with minimal conflict.
    #[inline(always)]
    pub fn inner_peace(&self) -> f32 {
        self.peace_ema
    }

    /// Number of objectives currently balanced (within tolerance of target).
    #[inline]
    pub fn count_balanced(&self) -> usize {
        self.objectives
            .values()
            .filter(|o| abs_f32(o.satisfaction - 1.0) < BALANCE_TOLERANCE)
            .count()
    }

    /// Get an objective by ID.
    #[inline(always)]
    pub fn get_objective(&self, obj_id: u64) -> Option<&Objective> {
        self.objectives.get(&obj_id)
    }

    /// List all objective IDs.
    #[inline(always)]
    pub fn objective_ids(&self) -> Vec<u64> {
        self.objectives.keys().copied().collect()
    }

    /// Aggregate statistics.
    pub fn stats(&self) -> EnlightenmentStats {
        let avg_sat = if self.objectives.is_empty() {
            0.0
        } else {
            let sum: f32 = self.objectives.values().map(|o| o.satisfaction).sum();
            sum / self.objectives.len() as f32
        };

        EnlightenmentStats {
            total_objectives: self.objectives.len() as u64,
            avg_satisfaction: avg_sat,
            purpose_clarity: self.clarity_ema,
            balance_mastery: self.balance_ema,
            understanding_level: self.understanding_ema,
            tradeoffs_recorded: self.tradeoffs.len() as u64,
            tradeoffs_transcended: self.tradeoffs_transcended,
            decisions_made: self.decisions_made,
            enlightenment_ema: (self.understanding_ema + self.clarity_ema + self.balance_ema) / 3.0,
        }
    }

    /// Current tick.
    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    // --- private helpers ---

    fn weighted_satisfaction(&self) -> f32 {
        if self.objectives.is_empty() {
            return 0.0;
        }
        let mut ws = 0.0_f32;
        let mut wt = 0.0_f32;
        for (_, obj) in &self.objectives {
            ws += obj.satisfaction * obj.weight;
            wt += obj.weight;
        }
        if wt > 0.0 { ws / wt } else { 0.0 }
    }

    fn compute_inner_peace(&self) -> f32 {
        if self.objectives.is_empty() {
            return 1.0;
        }
        let balance = self.objective_balance();
        let avg_sat = self.weighted_satisfaction();
        let conflict_free = self.tradeoffs.iter().filter(|t| t.transcended).count() as f32
            / (self.tradeoffs.len().max(1) as f32);

        0.40 * avg_sat + 0.35 * balance + 0.25 * conflict_free
    }

    fn is_pareto_optimal(&self, scores: &[(u64, f32)]) -> bool {
        let current: Vec<f32> = scores.iter().map(|(_, s)| *s).collect();
        for front_point in &self.pareto_front {
            let mut all_worse = true;
            let mut any_strictly_worse = false;
            for i in 0..current.len().min(front_point.len()) {
                if current[i] > front_point[i] {
                    all_worse = false;
                    break;
                }
                if current[i] < front_point[i] {
                    any_strictly_worse = true;
                }
            }
            if all_worse && any_strictly_worse {
                return false; // dominated
            }
        }
        true
    }

    fn update_pareto_front(&mut self, point: Vec<f32>) {
        // Remove dominated points
        self.pareto_front.retain(|existing| {
            let mut all_worse = true;
            let mut any_strictly_worse = false;
            for i in 0..existing.len().min(point.len()) {
                if existing[i] > point[i] {
                    all_worse = false;
                    break;
                }
                if existing[i] < point[i] {
                    any_strictly_worse = true;
                }
            }
            !(all_worse && any_strictly_worse)
        });

        if self.pareto_front.len() < MAX_PARETO_FRONT {
            self.pareto_front.push(point);
        }
    }
}

// ============================================================================
// FREE FUNCTIONS
// ============================================================================

fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut guess = x;
    for _ in 0..8 {
        guess = 0.5 * (guess + x / guess);
    }
    guess
}
