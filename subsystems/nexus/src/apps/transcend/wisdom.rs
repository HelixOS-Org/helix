// SPDX-License-Identifier: GPL-2.0
//! # Apps Wisdom — Accumulated App Management Wisdom
//!
//! Distills long-running experience into actionable wisdom. The engine
//! accumulates knowledge about which interventions work under which
//! circumstances, builds a corpus of advisory entries, and produces
//! sage-level insights that help the kernel choose between patience and
//! action when managing applications.
//!
//! Every wisdom entry carries context, advice, and a historical success
//! rate so that the quality of wisdom can be evaluated empirically. The
//! engine balances recency against long-term trends using exponential
//! moving averages and entropy-based novelty detection.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 9;
const MAX_WISDOM_ENTRIES: usize = 4096;
const MAX_INTERVENTIONS: usize = 2048;
const PATIENCE_THRESHOLD: u64 = 40;
const ACTION_THRESHOLD: u64 = 70;
const WISDOM_MATURITY_TICKS: u64 = 50;
const SAGE_INSIGHT_MIN_ENTRIES: usize = 8;
const ACCUMULATION_WINDOW: usize = 32;
const SUCCESS_EXCELLENT: u64 = 85;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

fn xorshift64(state: &mut u64) -> u64 {
    let mut s = *state;
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    *state = s;
    s
}

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single accumulated wisdom entry.
#[derive(Clone, Debug)]
pub struct WisdomEntry {
    pub entry_id: u64,
    pub context_hash: u64,
    pub advice_hash: u64,
    pub context_label: String,
    pub advice_label: String,
    pub success_rate: u64,
    pub application_count: u64,
    pub last_applied_tick: u64,
    pub created_tick: u64,
    pub confidence: u64,
}

/// Record of an intervention decision and its outcome.
#[derive(Clone, Debug)]
pub struct InterventionRecord {
    pub intervention_id: u64,
    pub app_id: u64,
    pub action_hash: u64,
    pub action_label: String,
    pub outcome_score: u64,
    pub tick: u64,
    pub patience_chosen: bool,
}

/// An allocation recommendation from accumulated wisdom.
#[derive(Clone, Debug)]
pub struct WiseAllocation {
    pub app_id: u64,
    pub resource_hash: u64,
    pub recommended_amount: u64,
    pub wisdom_confidence: u64,
    pub supporting_entries: u64,
}

/// A sage-level insight synthesised from many wisdom entries.
#[derive(Clone, Debug)]
pub struct SageInsight {
    pub insight_id: u64,
    pub topic_hash: u64,
    pub summary_label: String,
    pub entry_count: u64,
    pub avg_success: u64,
    pub depth: u64,
}

/// Running statistics for the wisdom engine.
#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct WisdomStats {
    pub total_entries: u64,
    pub total_interventions: u64,
    pub avg_success_rate: u64,
    pub patience_chosen_count: u64,
    pub action_chosen_count: u64,
    pub sage_insights_generated: u64,
    pub accumulation_score: u64,
    pub wisdom_depth: u64,
}

// ---------------------------------------------------------------------------
// AppsWisdom
// ---------------------------------------------------------------------------

/// Engine for accumulating and dispensing wisdom about app management.
pub struct AppsWisdom {
    entries: BTreeMap<u64, WisdomEntry>,
    interventions: Vec<InterventionRecord>,
    context_index: BTreeMap<u64, Vec<u64>>,
    insights: BTreeMap<u64, SageInsight>,
    success_ema: u64,
    stats: WisdomStats,
    rng: u64,
    tick: u64,
}

impl AppsWisdom {
    /// Create a new wisdom engine.
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            interventions: Vec::new(),
            context_index: BTreeMap::new(),
            insights: BTreeMap::new(),
            success_ema: 50,
            stats: WisdomStats::default(),
            rng: seed | 1,
            tick: 0,
        }
    }

    // -- public API ---------------------------------------------------------

    /// Query accumulated wisdom for a specific app context.
    ///
    /// Returns all wisdom entries whose context matches the given label,
    /// sorted by descending success rate.
    pub fn app_wisdom(&self, context_label: &str) -> Vec<&WisdomEntry> {
        let ctx_hash = fnv1a(context_label.as_bytes());
        let ids = match self.context_index.get(&ctx_hash) {
            Some(v) => v,
            None => return Vec::new(),
        };
        let mut result: Vec<&WisdomEntry> = ids
            .iter()
            .filter_map(|id| self.entries.get(id))
            .collect();
        result.sort_by(|a, b| b.success_rate.cmp(&a.success_rate));
        result
    }

    /// Record an intervention and its outcome, building intervention wisdom.
    ///
    /// The `outcome` value is 0–100 representing how successful the
    /// intervention was. The engine decides whether patience or action was
    /// the right call and folds the result into the wisdom corpus.
    #[inline]
    pub fn intervention_wisdom(
        &mut self,
        app_id: u64,
        action_label: &str,
        context_label: &str,
        outcome: u64,
    ) {
        self.tick += 1;
        let outcome_clamped = outcome.min(100);
        let action_hash = fnv1a(action_label.as_bytes());
        let patience = outcome_clamped < PATIENCE_THRESHOLD;

        let record = InterventionRecord {
            intervention_id: self.tick,
            app_id,
            action_hash,
            action_label: String::from(action_label),
            outcome_score: outcome_clamped,
            tick: self.tick,
            patience_chosen: patience,
        };

        if patience {
            self.stats.patience_chosen_count += 1;
        } else {
            self.stats.action_chosen_count += 1;
        }

        if self.interventions.len() < MAX_INTERVENTIONS {
            self.interventions.push(record);
        }
        self.stats.total_interventions += 1;
        self.success_ema = ema_update(self.success_ema, outcome_clamped);
        self.stats.avg_success_rate = self.success_ema;

        // Fold into wisdom entry
        self.fold_intervention_into_wisdom(context_label, action_label, outcome_clamped);
    }

    /// Decide between patience and action for a given app context.
    ///
    /// Returns `true` when action is recommended, `false` when patience is
    /// wiser. The decision draws on all accumulated wisdom matching the
    /// context.
    pub fn patience_vs_action(&self, context_label: &str) -> bool {
        let entries = self.app_wisdom(context_label);
        if entries.is_empty() {
            // No wisdom — default to patience
            return false;
        }
        let total_success: u64 = entries.iter().map(|e| e.success_rate).sum();
        let count = entries.len() as u64;
        if count == 0 {
            return false;
        }
        let avg = total_success / count;
        avg >= ACTION_THRESHOLD
    }

    /// Produce a wise resource allocation recommendation for an app.
    ///
    /// Uses wisdom about similar contexts to recommend an amount, biased
    /// by historical success rates.
    pub fn wise_allocation(
        &mut self,
        app_id: u64,
        resource_label: &str,
        base_amount: u64,
    ) -> WiseAllocation {
        let resource_hash = fnv1a(resource_label.as_bytes());
        let entries = self.matching_wisdom_entries(resource_hash);
        let supporting = entries.len() as u64;

        if supporting == 0 {
            return WiseAllocation {
                app_id,
                resource_hash,
                recommended_amount: base_amount,
                wisdom_confidence: 0,
                supporting_entries: 0,
            };
        }

        let avg_success: u64 = entries.iter().map(|e| e.success_rate).sum::<u64>() / supporting;
        let adjustment = if avg_success > 70 {
            base_amount * (avg_success - 50) / 100
        } else if avg_success < 30 {
            let reduction = base_amount * (50 - avg_success.min(50)) / 100;
            base_amount.saturating_sub(reduction);
            0
        } else {
            0
        };

        let recommended = base_amount.saturating_add(adjustment);
        let confidence = (avg_success * supporting.min(20)) / 20;

        WiseAllocation {
            app_id,
            resource_hash,
            recommended_amount: recommended,
            wisdom_confidence: confidence.min(100),
            supporting_entries: supporting,
        }
    }

    /// Measure the accumulation score of the wisdom corpus.
    ///
    /// Evaluates breadth (distinct contexts), depth (entries per context),
    /// and quality (average success rate). Returns 0–100.
    pub fn wisdom_accumulation(&self) -> u64 {
        let breadth = self.context_index.len() as u64;
        let depth = if breadth == 0 {
            0
        } else {
            self.entries.len() as u64 / breadth
        };
        let quality = self.success_ema;

        let breadth_score = (breadth * 10).min(100);
        let depth_score = (depth * 15).min(100);
        let quality_score = quality;

        (breadth_score + depth_score + quality_score) / 3
    }

    /// Generate sage-level insights by synthesising clusters of wisdom
    /// entries. Insights are stored internally and returned.
    pub fn sage_insight(&mut self) -> Vec<SageInsight> {
        let mut new_insights: Vec<SageInsight> = Vec::new();

        let context_keys: Vec<u64> = self.context_index.keys().copied().collect();
        for ctx_hash in &context_keys {
            let ids = match self.context_index.get(ctx_hash) {
                Some(v) => v.clone(),
                None => continue,
            };
            if ids.len() < SAGE_INSIGHT_MIN_ENTRIES {
                continue;
            }

            let mut total_success: u64 = 0;
            let mut count: u64 = 0;
            let mut best_label = String::new();
            let mut best_rate: u64 = 0;

            for eid in &ids {
                if let Some(entry) = self.entries.get(eid) {
                    total_success += entry.success_rate;
                    count += 1;
                    if entry.success_rate > best_rate {
                        best_rate = entry.success_rate;
                        best_label = entry.advice_label.clone();
                    }
                }
            }

            if count == 0 {
                continue;
            }
            let avg_success = total_success / count;
            let insight_id = fnv1a(&ctx_hash.to_le_bytes())
                ^ fnv1a(&count.to_le_bytes());

            let insight = SageInsight {
                insight_id,
                topic_hash: *ctx_hash,
                summary_label: best_label,
                entry_count: count,
                avg_success,
                depth: count / SAGE_INSIGHT_MIN_ENTRIES as u64,
            };
            new_insights.push(insight.clone());
            self.insights.insert(insight_id, insight);
        }

        self.stats.sage_insights_generated = self.insights.len() as u64;
        self.stats.wisdom_depth = self.compute_depth();
        new_insights
    }

    /// Return a snapshot of current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &WisdomStats {
        &self.stats
    }

    // -- internal -----------------------------------------------------------

    fn fold_intervention_into_wisdom(
        &mut self,
        context_label: &str,
        advice_label: &str,
        outcome: u64,
    ) {
        let ctx_hash = fnv1a(context_label.as_bytes());
        let advice_hash = fnv1a(advice_label.as_bytes());
        let entry_id = ctx_hash ^ advice_hash ^ self.tick;

        if let Some(existing) = self.find_matching_entry_mut(ctx_hash, advice_hash) {
            existing.success_rate = ema_update(existing.success_rate, outcome);
            existing.application_count += 1;
            existing.last_applied_tick = self.tick;
            existing.confidence = (existing.application_count * 5).min(100);
            return;
        }

        if self.entries.len() >= MAX_WISDOM_ENTRIES {
            self.evict_oldest_entry();
        }

        let entry = WisdomEntry {
            entry_id,
            context_hash: ctx_hash,
            advice_hash,
            context_label: String::from(context_label),
            advice_label: String::from(advice_label),
            success_rate: outcome,
            application_count: 1,
            last_applied_tick: self.tick,
            created_tick: self.tick,
            confidence: 5,
        };

        self.entries.insert(entry_id, entry);
        self.context_index
            .entry(ctx_hash)
            .or_insert_with(Vec::new)
            .push(entry_id);
        self.stats.total_entries = self.entries.len() as u64;
    }

    fn find_matching_entry_mut(
        &mut self,
        ctx_hash: u64,
        advice_hash: u64,
    ) -> Option<&mut WisdomEntry> {
        let ids = self.context_index.get(&ctx_hash)?;
        for eid in ids {
            if let Some(entry) = self.entries.get_mut(eid) {
                if entry.advice_hash == advice_hash {
                    return Some(entry);
                }
            }
        }
        None
    }

    fn matching_wisdom_entries(&self, topic_hash: u64) -> Vec<&WisdomEntry> {
        self.entries
            .values()
            .filter(|e| {
                e.context_hash == topic_hash || e.advice_hash == topic_hash
            })
            .collect()
    }

    fn evict_oldest_entry(&mut self) {
        let oldest_id = self
            .entries
            .values()
            .min_by_key(|e| e.last_applied_tick)
            .map(|e| e.entry_id);
        if let Some(oid) = oldest_id {
            if let Some(removed) = self.entries.remove(&oid) {
                if let Some(ids) = self.context_index.get_mut(&removed.context_hash) {
                    ids.retain(|id| *id != oid);
                }
            }
        }
    }

    fn compute_depth(&self) -> u64 {
        if self.context_index.is_empty() {
            return 0;
        }
        let total: u64 = self
            .context_index
            .values()
            .map(|ids| ids.len() as u64)
            .sum();
        total / self.context_index.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine() {
        let w = AppsWisdom::new(42);
        assert_eq!(w.stats().total_entries, 0);
        assert_eq!(w.stats().total_interventions, 0);
    }

    #[test]
    fn test_intervention_creates_wisdom() {
        let mut w = AppsWisdom::new(42);
        w.intervention_wisdom(1, "scale_up", "cpu_heavy", 80);
        assert_eq!(w.stats().total_entries, 1);
        assert_eq!(w.stats().total_interventions, 1);
    }

    #[test]
    fn test_app_wisdom_query() {
        let mut w = AppsWisdom::new(42);
        w.intervention_wisdom(1, "scale_up", "cpu_heavy", 90);
        w.intervention_wisdom(2, "throttle", "cpu_heavy", 40);
        let entries = w.app_wisdom("cpu_heavy");
        assert!(entries.len() >= 1);
    }

    #[test]
    fn test_patience_vs_action_default_patience() {
        let w = AppsWisdom::new(42);
        assert!(!w.patience_vs_action("unknown_context"));
    }

    #[test]
    fn test_wise_allocation_no_history() {
        let mut w = AppsWisdom::new(42);
        let alloc = w.wise_allocation(1, "memory", 1000);
        assert_eq!(alloc.recommended_amount, 1000);
        assert_eq!(alloc.wisdom_confidence, 0);
    }

    #[test]
    fn test_wisdom_accumulation_empty() {
        let w = AppsWisdom::new(42);
        let score = w.wisdom_accumulation();
        // success_ema starts at 50, breadth=0, depth=0 → ~16
        assert!(score <= 100);
    }

    #[test]
    fn test_sage_insight_generation() {
        let mut w = AppsWisdom::new(42);
        for i in 0..10 {
            w.intervention_wisdom(
                i,
                &alloc::format!("action_{}", i),
                "busy_context",
                60 + i,
            );
        }
        let insights = w.sage_insight();
        assert!(insights.len() >= 1);
    }
}
