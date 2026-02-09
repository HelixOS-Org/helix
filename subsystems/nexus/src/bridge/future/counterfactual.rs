// SPDX-License-Identifier: GPL-2.0
//! # Bridge Counterfactual Engine
//!
//! "What if?" analysis for bridge decisions. After every routing decision, this
//! engine evaluates what would have happened if the bridge chose differently.
//! Maintains a library of decision points with both the chosen action and the
//! road not taken. Regret analysis drives online learning — the bridge literally
//! learns from its mistakes by imagining its alternatives.
//!
//! Hindsight is 20/20; this module makes foresight 19/20.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_DECISIONS: usize = 512;
const MAX_ALTERNATIVES: usize = 8;
const MAX_HISTORY: usize = 2048;
const EMA_ALPHA: f32 = 0.10;
const REGRET_DECAY: f32 = 0.998;
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

fn rand_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 1_000_000) as f32 / 1_000_000.0
}

// ============================================================================
// DECISION ACTION
// ============================================================================

/// An action that the bridge could take at a decision point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BridgeAction {
    /// Route syscall to fast-path handler
    FastPath(u32),
    /// Route syscall to standard handler
    StandardPath(u32),
    /// Queue the syscall for batching
    BatchQueue(u32),
    /// Preemptively allocate resources
    PreAllocate(u32),
    /// Defer processing to next tick
    Defer,
    /// Reject / rate-limit
    RateLimit,
}

impl BridgeAction {
    fn to_bytes(&self) -> [u8; 8] {
        let val: u64 = match self {
            BridgeAction::FastPath(n) => 0x1000_0000 | *n as u64,
            BridgeAction::StandardPath(n) => 0x2000_0000 | *n as u64,
            BridgeAction::BatchQueue(n) => 0x3000_0000 | *n as u64,
            BridgeAction::PreAllocate(n) => 0x4000_0000 | *n as u64,
            BridgeAction::Defer => 0x5000_0000,
            BridgeAction::RateLimit => 0x6000_0000,
        };
        val.to_le_bytes()
    }
}

// ============================================================================
// COUNTERFACTUAL SCENARIO
// ============================================================================

/// A counterfactual scenario: what would have happened with a different choice.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CounterfactualScenario {
    /// Unique identifier for this decision point
    pub decision_point: u64,
    /// The action that was actually chosen
    pub chosen_action: BridgeAction,
    /// The alternative action being evaluated
    pub alternative_action: BridgeAction,
    /// Observed outcome of the chosen action (higher = better)
    pub chosen_outcome: f32,
    /// Estimated outcome of the alternative action
    pub estimated_alternative_outcome: f32,
    /// Difference: alternative - chosen (positive = we missed out)
    pub estimated_difference: f32,
    /// Confidence in the alternative estimate
    pub confidence: f32,
    /// Tick at which the decision was made
    pub tick: u64,
}

// ============================================================================
// DECISION RECORD
// ============================================================================

/// Full record of a decision with its context and all evaluated alternatives.
#[derive(Debug, Clone)]
struct DecisionRecord {
    decision_point: u64,
    context_hash: u64,
    tick: u64,
    chosen_action: BridgeAction,
    chosen_outcome: f32,
    outcome_observed: bool,
    alternatives: Vec<AlternativeRecord>,
}

/// Record of one alternative action.
#[derive(Debug, Clone)]
struct AlternativeRecord {
    action: BridgeAction,
    estimated_outcome: f32,
    sample_count: u32,
    outcome_ema: f32,
}

// ============================================================================
// REGRET ENTRY
// ============================================================================

/// Cumulative regret for a particular decision context.
#[derive(Debug, Clone)]
struct RegretEntry {
    context_hash: u64,
    cumulative_regret: f32,
    decision_count: u64,
    best_action_history: LinearMap<u32, 64>, // action_hash -> times it was best
    avg_regret_ema: f32,
}

// ============================================================================
// COUNTERFACTUAL STATS
// ============================================================================

/// Statistics for the counterfactual engine.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CounterfactualStats {
    pub total_decisions: u64,
    pub total_what_ifs: u64,
    pub total_regret_queries: u64,
    pub avg_regret: f32,
    pub avg_decision_quality: f32,
    pub hindsight_optimal_rate: f32,
    pub positive_regret_fraction: f32,
    pub outcomes_observed: u64,
}

impl CounterfactualStats {
    fn new() -> Self {
        Self {
            total_decisions: 0,
            total_what_ifs: 0,
            total_regret_queries: 0,
            avg_regret: 0.0,
            avg_decision_quality: 0.5,
            hindsight_optimal_rate: 0.0,
            positive_regret_fraction: 0.0,
            outcomes_observed: 0,
        }
    }
}

// ============================================================================
// BRIDGE COUNTERFACTUAL
// ============================================================================

/// Counterfactual analysis engine for bridge decisions.
///
/// After each decision, evaluates what would have happened if a different action
/// was taken. Tracks cumulative regret, decision quality, and hindsight-optimal
/// rates to drive online learning.
#[repr(align(64))]
pub struct BridgeCounterfactual {
    /// Active decision records waiting for outcome observation
    pending: BTreeMap<u64, DecisionRecord>,
    /// Completed decisions with observed outcomes
    completed: VecDeque<DecisionRecord>,
    /// Regret tracking per context
    regret_table: BTreeMap<u64, RegretEntry>,
    /// Action → outcome model: (context_hash, action_hash) → EMA outcome
    action_model: BTreeMap<(u64, u64), (f32, u32)>, // (ema_outcome, count)
    /// Running statistics
    stats: CounterfactualStats,
    /// PRNG state
    rng: u64,
    /// Tick counter
    tick: u64,
}

impl BridgeCounterfactual {
    /// Create a new counterfactual engine.
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            completed: VecDeque::new(),
            regret_table: BTreeMap::new(),
            action_model: BTreeMap::new(),
            stats: CounterfactualStats::new(),
            rng: 0xC0DE_FACE_1234_5678,
            tick: 0,
        }
    }

    /// Record a decision: the bridge chose `action` among `alternatives` at `context`.
    pub fn record_decision(
        &mut self,
        context_hash: u64,
        chosen_action: BridgeAction,
        alternatives: &[BridgeAction],
        tick: u64,
    ) -> u64 {
        self.tick = tick;
        self.stats.total_decisions += 1;

        let dp_data = [
            context_hash.to_le_bytes(),
            tick.to_le_bytes(),
        ]
        .concat();
        let decision_point = fnv1a_hash(&dp_data);

        let alt_records: Vec<AlternativeRecord> = alternatives
            .iter()
            .filter(|a| **a != chosen_action)
            .take(MAX_ALTERNATIVES)
            .map(|a| {
                let action_hash = fnv1a_hash(&a.to_bytes());
                let (ema, _) = self
                    .action_model
                    .get(&(context_hash, action_hash))
                    .copied()
                    .unwrap_or((0.5, 0));
                AlternativeRecord {
                    action: *a,
                    estimated_outcome: ema,
                    sample_count: 0,
                    outcome_ema: ema,
                }
            })
            .collect();

        let record = DecisionRecord {
            decision_point,
            context_hash,
            tick,
            chosen_action,
            chosen_outcome: 0.0,
            outcome_observed: false,
            alternatives: alt_records,
        };

        self.pending.insert(decision_point, record);

        // Evict oldest pending if too many
        while self.pending.len() > MAX_DECISIONS {
            if let Some(&oldest_key) = self.pending.keys().next() {
                self.pending.remove(&oldest_key);
            }
        }

        decision_point
    }

    /// Observe the outcome of a previously recorded decision.
    pub fn observe_outcome(&mut self, decision_point: u64, outcome: f32) {
        self.stats.outcomes_observed += 1;

        if let Some(mut record) = self.pending.remove(&decision_point) {
            record.chosen_outcome = outcome;
            record.outcome_observed = true;

            // Update action model for chosen action
            let chosen_hash = fnv1a_hash(&record.chosen_action.to_bytes());
            let entry = self
                .action_model
                .entry((record.context_hash, chosen_hash))
                .or_insert((0.5, 0));
            entry.0 = entry.0 * (1.0 - EMA_ALPHA) + outcome * EMA_ALPHA;
            entry.1 += 1;

            // Compute regret against each alternative
            let mut max_alternative = f32::NEG_INFINITY;
            for alt in &record.alternatives {
                if alt.estimated_outcome > max_alternative {
                    max_alternative = alt.estimated_outcome;
                }
            }

            let regret = if max_alternative > f32::NEG_INFINITY {
                (max_alternative - outcome).max(0.0)
            } else {
                0.0
            };

            // Update regret table
            let regret_entry = self
                .regret_table
                .entry(record.context_hash)
                .or_insert_with(|| RegretEntry {
                    context_hash: record.context_hash,
                    cumulative_regret: 0.0,
                    decision_count: 0,
                    best_action_history: LinearMap::new(),
                    avg_regret_ema: 0.0,
                });
            regret_entry.cumulative_regret += regret;
            regret_entry.decision_count += 1;
            regret_entry.avg_regret_ema =
                regret_entry.avg_regret_ema * (1.0 - EMA_ALPHA) + regret * EMA_ALPHA;

            // Track which action was best in hindsight
            if regret < 0.001 {
                *regret_entry
                    .best_action_history
                    .entry(chosen_hash)
                    .or_insert(0) += 1;
            }

            // Update global stats
            self.stats.avg_regret =
                self.stats.avg_regret * (1.0 - EMA_ALPHA) + regret * EMA_ALPHA;
            let quality = if regret < 0.01 { 1.0 } else { outcome / (outcome + regret).max(0.001) };
            self.stats.avg_decision_quality = self.stats.avg_decision_quality * (1.0 - EMA_ALPHA)
                + quality * EMA_ALPHA;
            if regret < 0.01 {
                self.stats.hindsight_optimal_rate = self.stats.hindsight_optimal_rate
                    * (1.0 - EMA_ALPHA)
                    + EMA_ALPHA;
            } else {
                self.stats.hindsight_optimal_rate *= 1.0 - EMA_ALPHA;
            }
            if regret > 0.01 {
                self.stats.positive_regret_fraction = self.stats.positive_regret_fraction
                    * (1.0 - EMA_ALPHA)
                    + EMA_ALPHA;
            } else {
                self.stats.positive_regret_fraction *= 1.0 - EMA_ALPHA;
            }

            self.completed.push_back(record);
            if self.completed.len() > MAX_HISTORY {
                self.completed.pop_front();
            }
        }
    }

    /// Perform a "what if" analysis: what would have happened with `alt_action`?
    pub fn what_if(
        &mut self,
        decision_point: u64,
        alt_action: BridgeAction,
    ) -> Option<CounterfactualScenario> {
        self.stats.total_what_ifs += 1;

        let record = self.completed.iter().find(|r| r.decision_point == decision_point)?;
        if !record.outcome_observed {
            return None;
        }

        let alt_hash = fnv1a_hash(&alt_action.to_bytes());
        let (est_outcome, count) = self
            .action_model
            .get(&(record.context_hash, alt_hash))
            .copied()
            .unwrap_or((0.5, 0));

        let confidence = 1.0 - 1.0 / (1.0 + count as f32 * 0.2);
        let difference = est_outcome - record.chosen_outcome;

        Some(CounterfactualScenario {
            decision_point,
            chosen_action: record.chosen_action,
            alternative_action: alt_action,
            chosen_outcome: record.chosen_outcome,
            estimated_alternative_outcome: est_outcome,
            estimated_difference: difference,
            confidence,
            tick: record.tick,
        })
    }

    /// Compute regret analysis for a specific decision context.
    #[inline]
    pub fn regret_analysis(&mut self, context_hash: u64) -> Option<(f32, f32, u64)> {
        self.stats.total_regret_queries += 1;
        let entry = self.regret_table.get(&context_hash)?;
        let avg_regret = if entry.decision_count > 0 {
            entry.cumulative_regret / entry.decision_count as f32
        } else {
            0.0
        };
        Some((entry.cumulative_regret, avg_regret, entry.decision_count))
    }

    /// Find the best alternative action for a given context, based on historical outcomes.
    pub fn best_alternative(&self, context_hash: u64) -> Option<(u64, f32)> {
        let mut best_hash = 0u64;
        let mut best_outcome = f32::NEG_INFINITY;
        for (&(ctx, act_hash), &(ema, _count)) in &self.action_model {
            if ctx == context_hash && ema > best_outcome {
                best_outcome = ema;
                best_hash = act_hash;
            }
        }
        if best_outcome > f32::NEG_INFINITY {
            Some((best_hash, best_outcome))
        } else {
            None
        }
    }

    /// Compute the counterfactual value: expected gain from switching to the best alternative.
    #[inline]
    pub fn counterfactual_value(&self, context_hash: u64) -> f32 {
        let best = self.best_alternative(context_hash);
        let current_avg = self.current_average_outcome(context_hash);
        match best {
            Some((_, best_val)) => (best_val - current_avg).max(0.0),
            None => 0.0,
        }
    }

    fn current_average_outcome(&self, context_hash: u64) -> f32 {
        let mut sum = 0.0f32;
        let mut count = 0u32;
        for record in &self.completed {
            if record.context_hash == context_hash && record.outcome_observed {
                sum += record.chosen_outcome;
                count += 1;
            }
        }
        if count > 0 { sum / count as f32 } else { 0.5 }
    }

    /// Evaluate overall decision quality: fraction of decisions that were optimal.
    #[inline(always)]
    pub fn decision_quality(&self) -> f32 {
        self.stats.avg_decision_quality
    }

    /// Compute a hindsight score: how much better could we have done overall?
    pub fn hindsight_score(&self) -> f32 {
        if self.completed.is_empty() {
            return 0.0;
        }
        let mut chosen_sum = 0.0f32;
        let mut best_sum = 0.0f32;
        let mut count = 0u32;

        for record in &self.completed {
            if !record.outcome_observed {
                continue;
            }
            chosen_sum += record.chosen_outcome;
            let mut best_alt = record.chosen_outcome;
            for alt in &record.alternatives {
                if alt.estimated_outcome > best_alt {
                    best_alt = alt.estimated_outcome;
                }
            }
            best_sum += best_alt;
            count += 1;
        }

        if count == 0 || best_sum < 0.001 {
            return 1.0;
        }
        (chosen_sum / best_sum).min(1.0)
    }

    /// Decay regret entries over time.
    #[inline]
    pub fn decay_regret(&mut self) {
        for entry in self.regret_table.values_mut() {
            entry.cumulative_regret *= REGRET_DECAY;
            entry.avg_regret_ema *= REGRET_DECAY;
        }
    }

    /// Get statistics.
    #[inline(always)]
    pub fn stats(&self) -> &CounterfactualStats {
        &self.stats
    }

    /// Get the number of completed decisions in history.
    #[inline(always)]
    pub fn completed_count(&self) -> usize {
        self.completed.len()
    }

    /// Get the number of pending (unresolved) decisions.
    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}
