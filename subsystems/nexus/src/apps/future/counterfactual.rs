// SPDX-License-Identifier: GPL-2.0
//! # Apps Counterfactual Analysis
//!
//! "What if we had allocated differently?" — this module evaluates alternative
//! resource allocation decisions in hindsight and prospectively. Given an
//! observed outcome it computes regret, identifies better alternatives, and
//! derives the hindsight-optimal decision to guide future allocation policy.
//!
//! Every allocation decision is logged with its observed result so the engine
//! can compare the actual outcome against counterfactual scenarios and learn
//! from missed opportunities.
//!
//! This is the apps engine reasoning about roads not taken.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_DECISIONS: usize = 2048;
const MAX_ALTERNATIVES: usize = 16;
const MAX_APPS: usize = 256;
const EMA_ALPHA: f64 = 0.12;
const REGRET_DECAY: f64 = 0.02;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const XORSHIFT_SEED: u64 = 0xfeedface_deadc0de;

// ============================================================================
// UTILITY FUNCTIONS
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

#[inline]
fn ema_update(current: f64, sample: f64, alpha: f64) -> f64 {
    alpha * sample + (1.0 - alpha) * current
}

// ============================================================================
// ALLOCATION TYPE
// ============================================================================

/// The type of resource allocation decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AllocationType {
    MemoryPages,
    CpuSlices,
    IoCredits,
    ThreadSlots,
    CacheLines,
    BandwidthQuota,
}

impl AllocationType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::MemoryPages => "mem_pages",
            Self::CpuSlices => "cpu_slices",
            Self::IoCredits => "io_credits",
            Self::ThreadSlots => "thread_slots",
            Self::CacheLines => "cache_lines",
            Self::BandwidthQuota => "bw_quota",
        }
    }

    fn unit_cost(&self) -> f64 {
        match self {
            Self::MemoryPages => 4.0,
            Self::CpuSlices => 10.0,
            Self::IoCredits => 2.0,
            Self::ThreadSlots => 8.0,
            Self::CacheLines => 1.0,
            Self::BandwidthQuota => 5.0,
        }
    }
}

// ============================================================================
// ALLOCATION DECISION
// ============================================================================

/// A recorded allocation decision with its outcome.
#[derive(Debug, Clone)]
pub struct AllocationDecision {
    pub decision_id: u64,
    pub app_id: u64,
    pub alloc_type: AllocationType,
    pub amount_allocated: u64,
    pub tick: u64,
    pub observed_utility: f64,
    pub observed_waste: f64,
    pub sla_met: bool,
    pub latency_impact_us: u64,
}

impl AllocationDecision {
    fn efficiency(&self) -> f64 {
        if self.amount_allocated == 0 {
            return 0.0;
        }
        self.observed_utility / (self.amount_allocated as f64 * self.alloc_type.unit_cost())
    }

    fn regret_vs(&self, alternative_utility: f64) -> f64 {
        let actual = self.observed_utility;
        if alternative_utility > actual {
            alternative_utility - actual
        } else {
            0.0
        }
    }
}

// ============================================================================
// COUNTERFACTUAL ALTERNATIVE
// ============================================================================

/// A counterfactual alternative: what would have happened with a different amount.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CounterfactualAlternative {
    pub alternative_amount: u64,
    pub estimated_utility: f64,
    pub estimated_waste: f64,
    pub estimated_sla_met: bool,
    pub estimated_latency_us: u64,
    pub regret: f64,
}

// ============================================================================
// PER-APP COUNTERFACTUAL STATE
// ============================================================================

#[derive(Debug, Clone)]
struct AppCounterfactualState {
    app_id: u64,
    decisions: VecDeque<AllocationDecision>,
    cumulative_regret: f64,
    ema_efficiency: f64,
    ema_waste: f64,
    total_sla_violations: u64,
    total_decisions: u64,
    utility_model: BTreeMap<u64, f64>,
}

impl AppCounterfactualState {
    fn new(app_id: u64) -> Self {
        Self {
            app_id,
            decisions: VecDeque::new(),
            cumulative_regret: 0.0,
            ema_efficiency: 0.5,
            ema_waste: 0.0,
            total_sla_violations: 0,
            total_decisions: 0,
            utility_model: BTreeMap::new(),
        }
    }

    fn record_decision(&mut self, decision: AllocationDecision) {
        let eff = decision.efficiency();
        self.ema_efficiency = ema_update(self.ema_efficiency, eff, EMA_ALPHA);
        self.ema_waste = ema_update(self.ema_waste, decision.observed_waste, EMA_ALPHA);
        if !decision.sla_met {
            self.total_sla_violations += 1;
        }
        self.total_decisions += 1;

        // Update utility model: map amount -> observed utility
        let key = decision.amount_allocated;
        let entry = self.utility_model.entry(key).or_insert(0.0);
        *entry = ema_update(*entry, decision.observed_utility, EMA_ALPHA);

        if self.decisions.len() >= MAX_DECISIONS {
            self.decisions.pop_front();
        }
        self.decisions.push_back(decision);
    }

    fn estimate_utility(&self, alloc_type: AllocationType, amount: u64) -> f64 {
        // Interpolate from observed utility model
        if self.utility_model.is_empty() {
            return amount as f64 * alloc_type.unit_cost() * 0.5;
        }

        let mut lower: Option<(u64, f64)> = None;
        let mut upper: Option<(u64, f64)> = None;

        for (&k, &v) in &self.utility_model {
            if k <= amount {
                lower = Some((k, v));
            }
            if k >= amount && upper.is_none() {
                upper = Some((k, v));
            }
        }

        match (lower, upper) {
            (Some((lk, lv)), Some((uk, uv))) => {
                if lk == uk {
                    return lv;
                }
                let frac = (amount - lk) as f64 / (uk - lk) as f64;
                lv + frac * (uv - lv)
            }
            (Some((_, lv)), None) => lv,
            (None, Some((_, uv))) => uv,
            (None, None) => amount as f64 * 0.5,
        }
    }
}

// ============================================================================
// COUNTERFACTUAL STATS
// ============================================================================

/// Engine-level statistics for counterfactual analysis.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CounterfactualStats {
    pub total_analyses: u64,
    pub total_regret_computations: u64,
    pub average_regret: f64,
    pub total_better_alternatives_found: u64,
    pub total_sla_counterfactuals: u64,
    pub hindsight_optimal_queries: u64,
    pub average_efficiency: f64,
    pub improvement_suggestions: u64,
}

impl CounterfactualStats {
    fn new() -> Self {
        Self {
            total_analyses: 0,
            total_regret_computations: 0,
            average_regret: 0.0,
            total_better_alternatives_found: 0,
            total_sla_counterfactuals: 0,
            hindsight_optimal_queries: 0,
            average_efficiency: 0.5,
            improvement_suggestions: 0,
        }
    }
}

// ============================================================================
// APPS COUNTERFACTUAL ENGINE
// ============================================================================

/// Counterfactual analysis engine for application resource allocation.
///
/// Logs allocation decisions and their outcomes, then evaluates alternative
/// scenarios to compute regret and identify policy improvements.
#[repr(align(64))]
pub struct AppsCounterfactual {
    app_states: BTreeMap<u64, AppCounterfactualState>,
    stats: CounterfactualStats,
    rng_state: u64,
    tick: u64,
    ema_regret: f64,
    ema_efficiency_global: f64,
}

impl AppsCounterfactual {
    /// Create a new counterfactual analysis engine.
    pub fn new() -> Self {
        Self {
            app_states: BTreeMap::new(),
            stats: CounterfactualStats::new(),
            rng_state: XORSHIFT_SEED,
            tick: 0,
            ema_regret: 0.0,
            ema_efficiency_global: 0.5,
        }
    }

    /// Record an allocation decision and its observed outcome.
    pub fn record_decision(
        &mut self,
        app_id: u64,
        alloc_type: AllocationType,
        amount: u64,
        utility: f64,
        waste: f64,
        sla_met: bool,
        latency_us: u64,
    ) {
        self.tick += 1;
        if self.app_states.len() >= MAX_APPS && !self.app_states.contains_key(&app_id) {
            return;
        }
        let state = self.app_states.entry(app_id).or_insert_with(|| AppCounterfactualState::new(app_id));

        let decision_id = fnv1a_hash(&[
            &app_id.to_le_bytes()[..],
            &self.tick.to_le_bytes()[..],
        ].concat());

        let decision = AllocationDecision {
            decision_id,
            app_id,
            alloc_type,
            amount_allocated: amount,
            tick: self.tick,
            observed_utility: utility,
            observed_waste: waste,
            sla_met,
            latency_impact_us: latency_us,
        };

        state.record_decision(decision);
        self.ema_efficiency_global = ema_update(self.ema_efficiency_global, state.ema_efficiency, EMA_ALPHA);
        self.stats.average_efficiency = self.ema_efficiency_global;
    }

    /// Evaluate "what if" for a different allocation amount.
    ///
    /// Given the last decision for an app and alloc type, estimate what would
    /// have happened with a different amount.
    pub fn what_if_allocation(
        &mut self,
        app_id: u64,
        alloc_type: AllocationType,
        alternative_amount: u64,
    ) -> Option<CounterfactualAlternative> {
        self.stats.total_analyses += 1;
        let state = self.app_states.get(&app_id)?;

        let last_decision = state
            .decisions
            .iter()
            .rev()
            .find(|d| d.alloc_type as u8 == alloc_type as u8)?;

        let est_utility = state.estimate_utility(alloc_type, alternative_amount);
        let ratio = if last_decision.amount_allocated > 0 {
            alternative_amount as f64 / last_decision.amount_allocated as f64
        } else {
            1.0
        };
        let est_waste = last_decision.observed_waste * ratio;
        let est_sla = est_utility >= last_decision.observed_utility * 0.9;
        let est_latency = (last_decision.latency_impact_us as f64 / ratio.max(0.1)) as u64;
        let regret = last_decision.regret_vs(est_utility);

        Some(CounterfactualAlternative {
            alternative_amount,
            estimated_utility: est_utility,
            estimated_waste: est_waste,
            estimated_sla_met: est_sla,
            estimated_latency_us: est_latency,
            regret,
        })
    }

    /// Compute the regret for the last N decisions of an app.
    pub fn regret_computation(&mut self, app_id: u64, last_n: usize) -> f64 {
        self.stats.total_regret_computations += 1;
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return 0.0,
        };

        let start = if state.decisions.len() > last_n {
            state.decisions.len() - last_n
        } else {
            0
        };

        let mut total_regret = 0.0;
        for decision in &state.decisions[start..] {
            // Compare against 2x and 0.5x alternatives
            let alt_high = state.estimate_utility(decision.alloc_type, decision.amount_allocated * 2);
            let alt_low = state.estimate_utility(decision.alloc_type, decision.amount_allocated / 2);
            let best_alt = if alt_high > alt_low { alt_high } else { alt_low };
            total_regret += decision.regret_vs(best_alt);
        }

        self.ema_regret = ema_update(self.ema_regret, total_regret, EMA_ALPHA);
        self.stats.average_regret = self.ema_regret;
        total_regret
    }

    /// Find a better allocation alternative for the last decision.
    pub fn better_alternative(
        &mut self,
        app_id: u64,
        alloc_type: AllocationType,
    ) -> Option<CounterfactualAlternative> {
        let state = self.app_states.get(&app_id)?;
        let last = state
            .decisions
            .iter()
            .rev()
            .find(|d| d.alloc_type as u8 == alloc_type as u8)?;

        let mut best: Option<CounterfactualAlternative> = None;
        let base = last.amount_allocated;

        // Search multipliers: 0.25, 0.5, 0.75, 1.25, 1.5, 2.0, 3.0
        let multipliers: [f64; 7] = [0.25, 0.5, 0.75, 1.25, 1.5, 2.0, 3.0];
        for &mult in &multipliers {
            let alt_amount = (base as f64 * mult) as u64;
            if alt_amount == 0 || alt_amount == base {
                continue;
            }
            let est_util = state.estimate_utility(alloc_type, alt_amount);
            let regret = last.regret_vs(est_util);
            if regret > 0.0 {
                let is_better = match &best {
                    Some(b) => est_util > b.estimated_utility,
                    None => true,
                };
                if is_better {
                    let ratio = alt_amount as f64 / base.max(1) as f64;
                    best = Some(CounterfactualAlternative {
                        alternative_amount: alt_amount,
                        estimated_utility: est_util,
                        estimated_waste: last.observed_waste * ratio,
                        estimated_sla_met: est_util >= last.observed_utility * 0.9,
                        estimated_latency_us: (last.latency_impact_us as f64 / ratio.max(0.1)) as u64,
                        regret,
                    });
                }
            }
        }

        if best.is_some() {
            self.stats.total_better_alternatives_found += 1;
        }
        best
    }

    /// Evaluate a counterfactual SLA scenario: would a different allocation
    /// have met SLA when the actual one did not?
    pub fn counterfactual_sla(&mut self, app_id: u64) -> Vec<(u64, bool, f64)> {
        self.stats.total_sla_counterfactuals += 1;
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut results = Vec::new();
        for decision in state.decisions.iter().rev().take(10) {
            if decision.sla_met {
                continue;
            }
            // Would 2x have met SLA?
            let alt_amount = decision.amount_allocated * 2;
            let est_util = state.estimate_utility(decision.alloc_type, alt_amount);
            let would_meet = est_util >= decision.observed_utility * 1.1;
            let extra_cost = alt_amount as f64 * decision.alloc_type.unit_cost()
                - decision.amount_allocated as f64 * decision.alloc_type.unit_cost();
            results.push((decision.decision_id, would_meet, extra_cost));
        }

        results
    }

    /// Suggest improvements to allocation policy based on cumulative regret patterns.
    pub fn decision_improvement(&mut self, app_id: u64) -> Vec<(AllocationType, f64)> {
        self.stats.improvement_suggestions += 1;
        let state = match self.app_states.get(&app_id) {
            Some(s) => s,
            None => return Vec::new(),
        };

        let mut type_regret: BTreeMap<u8, (f64, u64)> = BTreeMap::new();
        for decision in &state.decisions {
            let key = decision.alloc_type as u8;
            let alt_util = state.estimate_utility(decision.alloc_type, decision.amount_allocated * 2);
            let regret = decision.regret_vs(alt_util);
            let entry = type_regret.entry(key).or_insert((0.0, 0));
            entry.0 += regret;
            entry.1 += 1;
        }

        let mut improvements = Vec::new();
        for (&key, &(total_regret, count)) in &type_regret {
            if count > 0 && total_regret > 0.0 {
                let avg_regret = total_regret / count as f64;
                let alloc_type = match key {
                    0 => AllocationType::MemoryPages,
                    1 => AllocationType::CpuSlices,
                    2 => AllocationType::IoCredits,
                    3 => AllocationType::ThreadSlots,
                    4 => AllocationType::CacheLines,
                    _ => AllocationType::BandwidthQuota,
                };
                improvements.push((alloc_type, avg_regret));
            }
        }

        // Sort by regret descending
        for i in 1..improvements.len() {
            let mut j = i;
            while j > 0 && improvements[j].1 > improvements[j - 1].1 {
                improvements.swap(j, j - 1);
                j -= 1;
            }
        }

        improvements
    }

    /// Compute the hindsight-optimal allocation for the last decision.
    pub fn hindsight_optimal(
        &mut self,
        app_id: u64,
        alloc_type: AllocationType,
    ) -> Option<(u64, f64)> {
        self.stats.hindsight_optimal_queries += 1;
        let state = self.app_states.get(&app_id)?;

        let last = state
            .decisions
            .iter()
            .rev()
            .find(|d| d.alloc_type as u8 == alloc_type as u8)?;

        let base = last.amount_allocated;
        let mut best_amount = base;
        let mut best_utility = last.observed_utility;

        // Search a range of amounts
        for step in 1..=20u64 {
            let candidate = base * step / 10;
            if candidate == 0 {
                continue;
            }
            let est_util = state.estimate_utility(alloc_type, candidate);
            if est_util > best_utility {
                best_utility = est_util;
                best_amount = candidate;
            }
        }

        Some((best_amount, best_utility))
    }

    /// Return a snapshot of engine statistics.
    #[inline(always)]
    pub fn stats(&self) -> &CounterfactualStats {
        &self.stats
    }

    /// Total tracked decisions across all apps.
    #[inline(always)]
    pub fn total_decisions(&self) -> usize {
        self.app_states.values().map(|s| s.decisions.len()).sum()
    }

    /// Number of tracked apps.
    #[inline(always)]
    pub fn tracked_apps(&self) -> usize {
        self.app_states.len()
    }

    /// Global EMA-smoothed efficiency.
    #[inline(always)]
    pub fn global_efficiency(&self) -> f64 {
        self.ema_efficiency_global
    }
}
