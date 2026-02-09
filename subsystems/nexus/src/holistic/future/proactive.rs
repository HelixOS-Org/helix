// SPDX-License-Identifier: GPL-2.0
//! # Holistic Proactive Optimizer
//!
//! System-wide proactive optimization engine. Coordinates anticipatory actions
//! across ALL subsystems — pre-balances load before bottlenecks form,
//! pre-reclaims memory before pressure spikes, pre-routes network traffic
//! before congestion hits.
//!
//! This module doesn't react; it **acts first**. By integrating predictions
//! from every subsystem, it executes coordinated, cross-domain pre-emptive
//! optimizations that individual subsystems cannot achieve alone.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ACTIONS: usize = 512;
const MAX_PREWARM_ENTRIES: usize = 128;
const MAX_CASCADE_RECORDS: usize = 64;
const MAX_SAVINGS_ENTRIES: usize = 256;
const EMA_ALPHA: f32 = 0.10;
const URGENCY_THRESHOLD: f32 = 0.60;
const CASCADE_RISK_THRESHOLD: f32 = 0.40;
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

// ============================================================================
// ACTION DOMAIN
// ============================================================================

/// Subsystem domain targeted by a proactive action
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionDomain {
    LoadBalance,
    MemoryReclaim,
    NetworkRoute,
    IoSchedule,
    ThermalThrottle,
    ProcessMigrate,
    CacheFlush,
    Defragment,
}

/// Urgency level of a proactive action
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgency {
    Low,
    Medium,
    High,
    Critical,
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// A coordinated proactive action across subsystems
#[derive(Debug, Clone)]
pub struct ProactiveAction {
    pub id: u64,
    pub domain: ActionDomain,
    pub urgency: Urgency,
    pub description: String,
    pub predicted_benefit: f32,
    pub execution_cost: f32,
    pub net_benefit: f32,
    pub tick: u64,
    pub executed: bool,
    pub outcome_score: f32,
}

/// A pre-warm entry — resource readied before demand arrives
#[derive(Debug, Clone)]
pub struct PrewarmEntry {
    pub id: u64,
    pub domain: ActionDomain,
    pub target: String,
    pub warmup_ticks: u64,
    pub predicted_demand_tick: u64,
    pub readiness: f32,
    pub waste_if_unused: f32,
}

/// Anticipatory balance record — load redistributed before overload
#[derive(Debug, Clone)]
pub struct BalanceRecord {
    pub id: u64,
    pub from_domain: ActionDomain,
    pub to_domain: ActionDomain,
    pub load_moved: f32,
    pub balance_improvement: f32,
    pub tick: u64,
}

/// Cascade prevention record — chain reaction stopped before it starts
#[derive(Debug, Clone)]
pub struct CascadePreventionRecord {
    pub id: u64,
    pub trigger_domain: ActionDomain,
    pub affected_domains: Vec<ActionDomain>,
    pub risk_before: f32,
    pub risk_after: f32,
    pub actions_taken: Vec<String>,
    pub tick: u64,
}

/// Defragmentation action record
#[derive(Debug, Clone)]
pub struct DefragRecord {
    pub id: u64,
    pub domain: ActionDomain,
    pub fragmentation_before: f32,
    pub fragmentation_after: f32,
    pub pages_moved: u64,
    pub contiguous_gained: u64,
}

/// Savings measurement from a proactive action
#[derive(Debug, Clone)]
pub struct SavingsEntry {
    pub action_id: u64,
    pub domain: ActionDomain,
    pub latency_saved_us: u64,
    pub memory_saved_kb: u64,
    pub cpu_cycles_saved: u64,
    pub io_ops_saved: u64,
    pub tick: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate proactive optimization statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ProactiveStats {
    pub total_actions: u64,
    pub total_prewarms: u64,
    pub total_balances: u64,
    pub cascades_prevented: u64,
    pub defrags_performed: u64,
    pub avg_net_benefit: f32,
    pub avg_outcome: f32,
    pub total_latency_saved_us: u64,
    pub total_memory_saved_kb: u64,
}

// ============================================================================
// HOLISTIC PROACTIVE OPTIMIZER
// ============================================================================

/// System-wide proactive optimization engine. Executes anticipatory,
/// cross-domain optimizations based on fused subsystem predictions.
#[derive(Debug)]
pub struct HolisticProactive {
    actions: BTreeMap<u64, ProactiveAction>,
    prewarm_queue: BTreeMap<u64, PrewarmEntry>,
    balance_history: BTreeMap<u64, BalanceRecord>,
    cascade_records: BTreeMap<u64, CascadePreventionRecord>,
    defrag_history: BTreeMap<u64, DefragRecord>,
    savings: BTreeMap<u64, SavingsEntry>,
    total_actions: u64,
    total_prewarms: u64,
    total_balances: u64,
    cascades_prevented: u64,
    defrags_done: u64,
    tick: u64,
    rng_state: u64,
    benefit_ema: f32,
    outcome_ema: f32,
}

impl HolisticProactive {
    pub fn new() -> Self {
        Self {
            actions: BTreeMap::new(),
            prewarm_queue: BTreeMap::new(),
            balance_history: BTreeMap::new(),
            cascade_records: BTreeMap::new(),
            defrag_history: BTreeMap::new(),
            savings: BTreeMap::new(),
            total_actions: 0,
            total_prewarms: 0,
            total_balances: 0,
            cascades_prevented: 0,
            defrags_done: 0,
            tick: 0,
            rng_state: 0xCA0A_C71F_E0C7_1B12,
            benefit_ema: 0.0,
            outcome_ema: 0.5,
        }
    }

    /// Execute a global preemptive action across subsystems
    pub fn global_preemptive_action(
        &mut self,
        domain: ActionDomain,
        description: String,
        predicted_benefit: f32,
        execution_cost: f32,
    ) -> ProactiveAction {
        self.tick += 1;
        self.total_actions += 1;

        let net = (predicted_benefit - execution_cost).clamp(-1.0, 1.0);
        let urgency = if net > 0.6 {
            Urgency::Critical
        } else if net > 0.3 {
            Urgency::High
        } else if net > 0.1 {
            Urgency::Medium
        } else {
            Urgency::Low
        };

        let id = fnv1a_hash(description.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let action = ProactiveAction {
            id,
            domain,
            urgency,
            description,
            predicted_benefit: predicted_benefit.clamp(0.0, 1.0),
            execution_cost: execution_cost.clamp(0.0, 1.0),
            net_benefit: net,
            tick: self.tick,
            executed: net > 0.0,
            outcome_score: 0.0,
        };

        self.benefit_ema = EMA_ALPHA * net + (1.0 - EMA_ALPHA) * self.benefit_ema;
        self.actions.insert(id, action.clone());

        if self.actions.len() > MAX_ACTIONS {
            if let Some((&oldest, _)) = self.actions.iter().next() {
                self.actions.remove(&oldest);
            }
        }

        action
    }

    /// Coordinate pre-warming of resources across subsystems
    pub fn coordinated_prewarm(
        &mut self,
        domain: ActionDomain,
        target: String,
        predicted_demand_tick: u64,
    ) -> PrewarmEntry {
        self.tick += 1;
        self.total_prewarms += 1;

        let warmup_needed = predicted_demand_tick.saturating_sub(self.tick);
        let readiness = if warmup_needed == 0 {
            1.0
        } else {
            (1.0 / warmup_needed as f32).clamp(0.0, 1.0)
        };

        let waste = match domain {
            ActionDomain::MemoryReclaim => 0.15,
            ActionDomain::CacheFlush => 0.10,
            ActionDomain::IoSchedule => 0.05,
            _ => 0.08,
        };

        let id = fnv1a_hash(target.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let entry = PrewarmEntry {
            id,
            domain,
            target,
            warmup_ticks: warmup_needed,
            predicted_demand_tick,
            readiness,
            waste_if_unused: waste,
        };

        self.prewarm_queue.insert(id, entry.clone());
        if self.prewarm_queue.len() > MAX_PREWARM_ENTRIES {
            if let Some((&oldest, _)) = self.prewarm_queue.iter().next() {
                self.prewarm_queue.remove(&oldest);
            }
        }

        entry
    }

    /// Anticipatory load balance — redistribute before overload
    pub fn anticipatory_balance(
        &mut self,
        from: ActionDomain,
        to: ActionDomain,
        load_fraction: f32,
    ) -> BalanceRecord {
        self.tick += 1;
        self.total_balances += 1;

        let improvement = (load_fraction * 0.8).clamp(0.0, 1.0);
        let id = fnv1a_hash(format!("bal-{:?}-{:?}-{}", from, to, self.tick).as_bytes())
            ^ xorshift64(&mut self.rng_state);

        let record = BalanceRecord {
            id,
            from_domain: from,
            to_domain: to,
            load_moved: load_fraction.clamp(0.0, 1.0),
            balance_improvement: improvement,
            tick: self.tick,
        };

        self.balance_history.insert(id, record.clone());
        record
    }

    /// Proactive defragmentation — compact memory/IO before fragmentation hurts
    pub fn proactive_defrag(
        &mut self,
        domain: ActionDomain,
        fragmentation_level: f32,
        pages_available: u64,
    ) -> DefragRecord {
        self.tick += 1;
        self.defrags_done += 1;

        let pages_to_move = ((fragmentation_level * pages_available as f32) as u64)
            .min(pages_available);
        let frag_after = (fragmentation_level * (1.0 - 0.7)).clamp(0.0, 1.0);
        let contiguous = pages_to_move * 3 / 4;

        let id = fnv1a_hash(format!("defrag-{:?}-{}", domain, self.tick).as_bytes())
            ^ xorshift64(&mut self.rng_state);

        let record = DefragRecord {
            id,
            domain,
            fragmentation_before: fragmentation_level.clamp(0.0, 1.0),
            fragmentation_after: frag_after,
            pages_moved: pages_to_move,
            contiguous_gained: contiguous,
        };

        self.defrag_history.insert(id, record.clone());
        record
    }

    /// Prevent a cascade failure before it triggers
    pub fn cascade_prevention(
        &mut self,
        trigger_domain: ActionDomain,
        affected: Vec<ActionDomain>,
        current_risk: f32,
    ) -> CascadePreventionRecord {
        self.tick += 1;

        let mut actions_taken = Vec::new();

        if current_risk > CASCADE_RISK_THRESHOLD {
            actions_taken.push(String::from("throttle_trigger_domain"));
            self.cascades_prevented += 1;
        }
        if current_risk > 0.6 {
            actions_taken.push(String::from("isolate_affected_domains"));
        }
        if current_risk > 0.8 {
            actions_taken.push(String::from("emergency_resource_reserve"));
        }

        let mitigation = actions_taken.len() as f32 * 0.2;
        let risk_after = (current_risk - mitigation).clamp(0.0, 1.0);

        let id = fnv1a_hash(
            format!("cascade-{:?}-{}", trigger_domain, self.tick).as_bytes(),
        ) ^ xorshift64(&mut self.rng_state);

        let record = CascadePreventionRecord {
            id,
            trigger_domain,
            affected_domains: affected,
            risk_before: current_risk.clamp(0.0, 1.0),
            risk_after,
            actions_taken,
            tick: self.tick,
        };

        self.cascade_records.insert(id, record.clone());
        if self.cascade_records.len() > MAX_CASCADE_RECORDS {
            if let Some((&oldest, _)) = self.cascade_records.iter().next() {
                self.cascade_records.remove(&oldest);
            }
        }

        record
    }

    /// Aggregate savings across all proactive actions
    pub fn savings_aggregation(&self) -> ProactiveStats {
        let mut total_lat = 0_u64;
        let mut total_mem = 0_u64;

        for s in self.savings.values() {
            total_lat += s.latency_saved_us;
            total_mem += s.memory_saved_kb;
        }

        ProactiveStats {
            total_actions: self.total_actions,
            total_prewarms: self.total_prewarms,
            total_balances: self.total_balances,
            cascades_prevented: self.cascades_prevented,
            defrags_performed: self.defrags_done,
            avg_net_benefit: self.benefit_ema,
            avg_outcome: self.outcome_ema,
            total_latency_saved_us: total_lat,
            total_memory_saved_kb: total_mem,
        }
    }

    /// Record the outcome of a previously-executed proactive action
    pub fn record_outcome(&mut self, action_id: u64, outcome: f32, savings: SavingsEntry) {
        if let Some(action) = self.actions.get_mut(&action_id) {
            action.outcome_score = outcome.clamp(0.0, 1.0);
            self.outcome_ema = EMA_ALPHA * outcome.clamp(0.0, 1.0)
                + (1.0 - EMA_ALPHA) * self.outcome_ema;
        }
        self.savings.insert(savings.action_id, savings);

        if self.savings.len() > MAX_SAVINGS_ENTRIES {
            if let Some((&oldest, _)) = self.savings.iter().next() {
                self.savings.remove(&oldest);
            }
        }
    }

    /// Gather aggregate statistics
    pub fn stats(&self) -> ProactiveStats {
        self.savings_aggregation()
    }
}
