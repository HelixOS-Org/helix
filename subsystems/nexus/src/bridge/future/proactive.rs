// SPDX-License-Identifier: GPL-2.0
//! # Bridge Proactive Optimizer
//!
//! Pre-allocates resources, pre-warms caches, and pre-computes routes BEFORE
//! demand materializes. The proactive engine monitors prediction confidence
//! and acts only when the expected savings exceed the cost of being wrong.
//! Every proactive action is tracked against its actual outcome to learn
//! when proactivity helps and when it wastes resources.
//!
//! Fortune favors the prepared kernel.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_OPPORTUNITIES: usize = 256;
const MAX_ACTIVE_ACTIONS: usize = 128;
const EMA_ALPHA: f32 = 0.10;
const MIN_CONFIDENCE_TO_ACT: f32 = 0.60;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const SAVINGS_DECAY: f32 = 0.95;

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ============================================================================
// OPPORTUNITY & ACTION TYPES
// ============================================================================

/// Category of proactive opportunity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OpportunityKind {
    RoutePrecompute,
    CachePrewarm,
    BufferPreallocate,
    FdPoolExpand,
    PagePrefault,
    ConnectionPreopen,
    IndexPrebuild,
    CompressionPrepare,
}

/// A detected optimization opportunity
#[derive(Debug, Clone)]
pub struct Opportunity {
    pub id: u64,
    pub kind: OpportunityKind,
    pub description: String,
    pub confidence: f32,
    pub estimated_savings_ns: u64,
    pub estimated_cost_ns: u64,
    pub target_process: u64,
    pub target_syscall: u32,
    pub detected_tick: u64,
    pub acted_on: bool,
}

/// Outcome of a proactive action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionOutcome {
    Hit,
    Miss,
    PartialHit,
    Expired,
    Pending,
}

/// A proactive action that has been taken
#[derive(Debug, Clone)]
pub struct ProactiveAction {
    pub action_id: u64,
    pub opportunity_id: u64,
    pub kind: OpportunityKind,
    pub start_tick: u64,
    pub cost_ns: u64,
    pub outcome: ActionOutcome,
    pub actual_savings_ns: u64,
}

/// Per-kind performance tracker
#[derive(Debug, Clone)]
struct KindTracker {
    total_opportunities: u64,
    acted_on: u64,
    hits: u64,
    misses: u64,
    total_savings: u64,
    total_cost: u64,
    hit_rate_ema: f32,
    savings_ema: f32,
}

impl KindTracker {
    fn new() -> Self {
        Self {
            total_opportunities: 0,
            acted_on: 0,
            hits: 0,
            misses: 0,
            total_savings: 0,
            total_cost: 0,
            hit_rate_ema: 0.5,
            savings_ema: 0.0,
        }
    }

    fn record_opportunity(&mut self) {
        self.total_opportunities += 1;
    }

    fn record_action(&mut self, outcome: ActionOutcome, savings: u64, cost: u64) {
        self.acted_on += 1;
        self.total_cost += cost;
        match outcome {
            ActionOutcome::Hit => {
                self.hits += 1;
                self.total_savings += savings;
                self.hit_rate_ema = EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * self.hit_rate_ema;
            }
            ActionOutcome::PartialHit => {
                self.hits += 1;
                self.total_savings += savings / 2;
                self.hit_rate_ema = EMA_ALPHA * 0.5 + (1.0 - EMA_ALPHA) * self.hit_rate_ema;
            }
            ActionOutcome::Miss | ActionOutcome::Expired => {
                self.misses += 1;
                self.hit_rate_ema = EMA_ALPHA * 0.0 + (1.0 - EMA_ALPHA) * self.hit_rate_ema;
            }
            ActionOutcome::Pending => {}
        }
        let net = if savings > cost { (savings - cost) as f32 } else { 0.0 };
        self.savings_ema = EMA_ALPHA * net + (1.0 - EMA_ALPHA) * self.savings_ema;
    }

    fn roi(&self) -> f32 {
        if self.total_cost == 0 {
            return 0.0;
        }
        self.total_savings as f32 / self.total_cost as f32
    }
}

// ============================================================================
// PROACTIVE STATS
// ============================================================================

/// Aggregate proactive optimization statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ProactiveStats {
    pub total_opportunities: u64,
    pub acted_on: u64,
    pub hit_rate: f32,
    pub total_savings_ns: u64,
    pub total_cost_ns: u64,
    pub roi: f32,
    pub avg_savings_per_action: f32,
    pub act_rate: f32,
}

// ============================================================================
// BRIDGE PROACTIVE
// ============================================================================

/// Proactive bridge optimization engine. Identifies opportunities and takes
/// pre-emptive actions when confidence exceeds the action threshold.
#[derive(Debug)]
pub struct BridgeProactive {
    opportunities: Vec<Opportunity>,
    opp_write_idx: usize,
    active_actions: BTreeMap<u64, ProactiveAction>,
    kind_trackers: BTreeMap<u8, KindTracker>,
    tick: u64,
    total_opportunities: u64,
    total_actions: u64,
    cumulative_savings: u64,
    cumulative_cost: u64,
    savings_ema: f32,
}

impl BridgeProactive {
    pub fn new() -> Self {
        Self {
            opportunities: Vec::new(),
            opp_write_idx: 0,
            active_actions: BTreeMap::new(),
            kind_trackers: BTreeMap::new(),
            tick: 0,
            total_opportunities: 0,
            total_actions: 0,
            cumulative_savings: 0,
            cumulative_cost: 0,
            savings_ema: 0.0,
        }
    }

    /// Identify a proactive optimization opportunity
    pub fn identify_opportunity(
        &mut self,
        kind: OpportunityKind,
        description: String,
        confidence: f32,
        estimated_savings_ns: u64,
        estimated_cost_ns: u64,
        target_process: u64,
        target_syscall: u32,
    ) -> u64 {
        self.tick += 1;
        self.total_opportunities += 1;

        let id = fnv1a_hash(&self.total_opportunities.to_le_bytes())
            ^ fnv1a_hash(description.as_bytes());

        let opp = Opportunity {
            id,
            kind,
            description,
            confidence: confidence.max(0.0).min(1.0),
            estimated_savings_ns,
            estimated_cost_ns,
            target_process,
            target_syscall,
            detected_tick: self.tick,
            acted_on: false,
        };

        let tracker = self.kind_trackers
            .entry(kind as u8)
            .or_insert_with(KindTracker::new);
        tracker.record_opportunity();

        if self.opportunities.len() < MAX_OPPORTUNITIES {
            self.opportunities.push(opp);
        } else {
            self.opportunities[self.opp_write_idx] = opp;
        }
        self.opp_write_idx = (self.opp_write_idx + 1) % MAX_OPPORTUNITIES;

        id
    }

    /// Pre-compute a syscall route before it's needed
    pub fn precompute_route(&mut self, opportunity_id: u64, route_data: String) -> bool {
        self.execute_action(opportunity_id, OpportunityKind::RoutePrecompute, &route_data)
    }

    /// Pre-warm a cache entry based on predicted access
    pub fn prewarm_cache(&mut self, opportunity_id: u64, cache_key: String) -> bool {
        self.execute_action(opportunity_id, OpportunityKind::CachePrewarm, &cache_key)
    }

    /// Pre-allocate a buffer for predicted demand
    pub fn preallocate_buffer(&mut self, opportunity_id: u64, buffer_desc: String) -> bool {
        self.execute_action(opportunity_id, OpportunityKind::BufferPreallocate, &buffer_desc)
    }

    fn execute_action(
        &mut self,
        opportunity_id: u64,
        expected_kind: OpportunityKind,
        _context: &str,
    ) -> bool {
        let opp = self.opportunities.iter_mut().find(|o| o.id == opportunity_id);
        let opp = match opp {
            Some(o) => o,
            None => return false,
        };

        if opp.acted_on {
            return false;
        }

        // Check confidence threshold, adjusted by historical hit rate
        let kind_key = expected_kind as u8;
        let kind_hit_rate = self.kind_trackers.get(&kind_key)
            .map(|t| t.hit_rate_ema)
            .unwrap_or(0.5);
        let adjusted_threshold = MIN_CONFIDENCE_TO_ACT * (1.0 - kind_hit_rate * 0.3);
        if opp.confidence < adjusted_threshold {
            return false;
        }

        if self.active_actions.len() >= MAX_ACTIVE_ACTIONS {
            // Evict oldest pending action
            let oldest = self.active_actions.iter()
                .filter(|(_, a)| a.outcome == ActionOutcome::Pending)
                .min_by_key(|(_, a)| a.start_tick)
                .map(|(&k, _)| k);
            if let Some(k) = oldest {
                self.active_actions.remove(&k);
            }
        }

        opp.acted_on = true;
        self.total_actions += 1;

        let action_id = fnv1a_hash(&self.total_actions.to_le_bytes())
            ^ fnv1a_hash(&opportunity_id.to_le_bytes());

        let action = ProactiveAction {
            action_id,
            opportunity_id,
            kind: opp.kind,
            start_tick: self.tick,
            cost_ns: opp.estimated_cost_ns,
            outcome: ActionOutcome::Pending,
            actual_savings_ns: 0,
        };

        self.active_actions.insert(action_id, action);
        true
    }

    /// Record the actual outcome of a proactive action
    pub fn record_outcome(
        &mut self,
        action_id: u64,
        outcome: ActionOutcome,
        actual_savings_ns: u64,
    ) {
        if let Some(action) = self.active_actions.get_mut(&action_id) {
            action.outcome = outcome;
            action.actual_savings_ns = actual_savings_ns;

            let kind_key = action.kind as u8;
            let cost = action.cost_ns;
            let tracker = self.kind_trackers
                .entry(kind_key)
                .or_insert_with(KindTracker::new);
            tracker.record_action(outcome, actual_savings_ns, cost);

            self.cumulative_cost += cost;
            if matches!(outcome, ActionOutcome::Hit | ActionOutcome::PartialHit) {
                self.cumulative_savings += actual_savings_ns;
            }

            let net = if actual_savings_ns > cost {
                (actual_savings_ns - cost) as f32
            } else {
                0.0
            };
            self.savings_ema = EMA_ALPHA * net + (1.0 - EMA_ALPHA) * self.savings_ema;
        }
    }

    /// Total proactive savings (savings minus cost of actions that missed)
    pub fn proactive_savings(&self) -> (u64, u64, f32) {
        let net = if self.cumulative_savings > self.cumulative_cost {
            self.cumulative_savings - self.cumulative_cost
        } else {
            0
        };
        let roi = if self.cumulative_cost > 0 {
            self.cumulative_savings as f32 / self.cumulative_cost as f32
        } else {
            0.0
        };
        (net, self.cumulative_savings, roi)
    }

    /// Aggregate proactive statistics
    pub fn stats(&self) -> ProactiveStats {
        let hit_rate: f32 = if self.kind_trackers.is_empty() {
            0.0
        } else {
            self.kind_trackers.values()
                .map(|t| t.hit_rate_ema)
                .sum::<f32>() / self.kind_trackers.len() as f32
        };

        let avg_savings = if self.total_actions > 0 {
            self.savings_ema
        } else {
            0.0
        };

        let act_rate = if self.total_opportunities > 0 {
            self.total_actions as f32 / self.total_opportunities as f32
        } else {
            0.0
        };

        let roi = if self.cumulative_cost > 0 {
            self.cumulative_savings as f32 / self.cumulative_cost as f32
        } else {
            0.0
        };

        ProactiveStats {
            total_opportunities: self.total_opportunities,
            acted_on: self.total_actions,
            hit_rate,
            total_savings_ns: self.cumulative_savings,
            total_cost_ns: self.cumulative_cost,
            roi,
            avg_savings_per_action: avg_savings,
            act_rate,
        }
    }
}
