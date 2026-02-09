// SPDX-License-Identifier: GPL-2.0
//! # Proactive Cooperation Optimizer
//!
//! Pre-negotiates resources, pre-establishes trust, and pre-builds coalitions
//! for anticipated cooperation needs. Reduces negotiation latency by preparing
//! cooperative agreements before demand materializes.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic key derivation.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for stochastic decisions.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// EMA update for running averages.
fn ema_update(current: u64, sample: u64, alpha_num: u64, alpha_den: u64) -> u64 {
    let old_part = current.saturating_mul(alpha_den.saturating_sub(alpha_num));
    let new_part = sample.saturating_mul(alpha_num);
    old_part.saturating_add(new_part) / alpha_den.max(1)
}

/// Result of a pre-negotiation attempt.
#[derive(Clone, Debug)]
pub struct PreNegotiationResult {
    pub contract_id: u64,
    pub resource_id: u64,
    pub partner_id: u64,
    pub reserved_amount: u64,
    pub validity_ticks: u64,
    pub negotiation_cost: u64,
    pub success: bool,
}

/// Pre-built trust record.
#[derive(Clone, Debug)]
pub struct PreTrustRecord {
    pub partner_id: u64,
    pub established_trust: u64,
    pub handshake_cost: u64,
    pub expiry_tick: u64,
    pub renewable: bool,
}

/// Anticipated coalition definition.
#[derive(Clone, Debug)]
pub struct AnticipatedCoalition {
    pub coalition_id: u64,
    pub member_ids: Vec<u64>,
    pub purpose_hash: u64,
    pub estimated_benefit: u64,
    pub formation_cost: u64,
    pub readiness: u64,
}

/// Proactive contract for future use.
#[derive(Clone, Debug)]
pub struct ProactiveContract {
    pub contract_id: u64,
    pub parties: Vec<u64>,
    pub resource_hash: u64,
    pub terms_hash: u64,
    pub activation_tick: u64,
    pub duration_ticks: u64,
    pub penalty_rate: u64,
}

/// Rolling statistics for proactive optimization.
#[derive(Clone, Debug)]
#[repr(align(64))]
pub struct ProactiveStats {
    pub pre_negotiations: u64,
    pub successful_pre_negotiations: u64,
    pub trust_establishments: u64,
    pub coalitions_anticipated: u64,
    pub contracts_created: u64,
    pub avg_savings: u64,
    pub total_negotiation_cost_saved: u64,
    pub proactive_errors: u64,
}

impl ProactiveStats {
    pub fn new() -> Self {
        Self {
            pre_negotiations: 0,
            successful_pre_negotiations: 0,
            trust_establishments: 0,
            coalitions_anticipated: 0,
            contracts_created: 0,
            avg_savings: 0,
            total_negotiation_cost_saved: 0,
            proactive_errors: 0,
        }
    }
}

/// Internal pre-negotiation slot.
#[derive(Clone, Debug)]
struct NegotiationSlot {
    contract_id: u64,
    resource_id: u64,
    partner_id: u64,
    reserved: u64,
    created_tick: u64,
    expiry_tick: u64,
    activated: bool,
}

/// Internal trust bootstrap entry.
#[derive(Clone, Debug)]
struct TrustBootstrap {
    partner_id: u64,
    trust_level: u64,
    handshakes: u64,
    cost_accumulated: u64,
    expiry_tick: u64,
}

/// Internal coalition plan.
#[derive(Clone, Debug)]
struct CoalitionPlan {
    coalition_id: u64,
    members: Vec<u64>,
    purpose_hash: u64,
    readiness_score: u64,
    formation_steps: u64,
    benefit_estimate: u64,
}

/// Proactive cooperation optimization engine.
pub struct CoopProactive {
    negotiation_slots: BTreeMap<u64, NegotiationSlot>,
    trust_bootstraps: BTreeMap<u64, TrustBootstrap>,
    coalition_plans: BTreeMap<u64, CoalitionPlan>,
    contracts: BTreeMap<u64, ProactiveContract>,
    savings_history: VecDeque<u64>,
    stats: ProactiveStats,
    rng_state: u64,
    current_tick: u64,
    max_slots: usize,
}

impl CoopProactive {
    /// Create a new proactive optimizer with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            negotiation_slots: BTreeMap::new(),
            trust_bootstraps: BTreeMap::new(),
            coalition_plans: BTreeMap::new(),
            contracts: BTreeMap::new(),
            savings_history: VecDeque::new(),
            stats: ProactiveStats::new(),
            rng_state: seed | 1,
            current_tick: 0,
            max_slots: 128,
        }
    }

    /// Advance the internal tick counter and expire stale entries.
    #[inline]
    pub fn tick(&mut self, now: u64) {
        self.current_tick = now;
        self.expire_stale_slots();
        self.expire_stale_trusts();
    }

    /// Pre-negotiate a resource reservation with an anticipated partner.
    pub fn pre_negotiate(
        &mut self,
        resource_id: u64,
        partner_id: u64,
        amount: u64,
        validity_ticks: u64,
    ) -> PreNegotiationResult {
        self.stats.pre_negotiations = self.stats.pre_negotiations.saturating_add(1);

        let contract_id = fnv1a_hash(
            &[
                resource_id.to_le_bytes(),
                partner_id.to_le_bytes(),
                self.stats.pre_negotiations.to_le_bytes(),
            ]
            .concat(),
        );

        let trust_factor = self
            .trust_bootstraps
            .get(&fnv1a_hash(&partner_id.to_le_bytes()))
            .map(|t| t.trust_level)
            .unwrap_or(300);

        let noise = xorshift64(&mut self.rng_state) % 100;
        let success_threshold = trust_factor.saturating_mul(8) / 10 + noise;
        let success = success_threshold > 400;

        let negotiation_cost = amount / 20 + xorshift64(&mut self.rng_state) % 50;

        if success && self.negotiation_slots.len() < self.max_slots {
            self.negotiation_slots.insert(contract_id, NegotiationSlot {
                contract_id,
                resource_id,
                partner_id,
                reserved: amount,
                created_tick: self.current_tick,
                expiry_tick: self.current_tick.saturating_add(validity_ticks),
                activated: false,
            });
            self.stats.successful_pre_negotiations =
                self.stats.successful_pre_negotiations.saturating_add(1);
        }

        PreNegotiationResult {
            contract_id,
            resource_id,
            partner_id,
            reserved_amount: if success { amount } else { 0 },
            validity_ticks,
            negotiation_cost,
            success,
        }
    }

    /// Pre-establish trust with a cooperation partner.
    pub fn pre_build_trust(
        &mut self,
        partner_id: u64,
        target_trust: u64,
        expiry_ticks: u64,
    ) -> PreTrustRecord {
        self.stats.trust_establishments = self.stats.trust_establishments.saturating_add(1);

        let key = fnv1a_hash(&partner_id.to_le_bytes());
        let handshake_cost = target_trust / 5 + 10;

        let entry = self
            .trust_bootstraps
            .entry(key)
            .or_insert_with(|| TrustBootstrap {
                partner_id,
                trust_level: 0,
                handshakes: 0,
                cost_accumulated: 0,
                expiry_tick: self.current_tick.saturating_add(expiry_ticks),
            });

        let increment = target_trust.saturating_sub(entry.trust_level) / 3;
        entry.trust_level = entry.trust_level.saturating_add(increment).min(1000);
        entry.handshakes = entry.handshakes.saturating_add(1);
        entry.cost_accumulated = entry.cost_accumulated.saturating_add(handshake_cost);
        entry.expiry_tick = self.current_tick.saturating_add(expiry_ticks);

        PreTrustRecord {
            partner_id,
            established_trust: entry.trust_level,
            handshake_cost,
            expiry_tick: entry.expiry_tick,
            renewable: entry.handshakes < 10,
        }
    }

    /// Anticipate and plan a coalition for a future cooperation need.
    pub fn anticipate_coalition(
        &mut self,
        member_ids: Vec<u64>,
        purpose: &str,
    ) -> AnticipatedCoalition {
        self.stats.coalitions_anticipated = self.stats.coalitions_anticipated.saturating_add(1);

        let purpose_hash = fnv1a_hash(purpose.as_bytes());
        let coalition_id = fnv1a_hash(
            &[
                purpose_hash.to_le_bytes(),
                self.stats.coalitions_anticipated.to_le_bytes(),
            ]
            .concat(),
        );

        let mut readiness: u64 = 0;
        let mut benefit: u64 = 0;
        for &member in &member_ids {
            let key = fnv1a_hash(&member.to_le_bytes());
            if let Some(trust) = self.trust_bootstraps.get(&key) {
                readiness = readiness.saturating_add(trust.trust_level);
                benefit = benefit.saturating_add(trust.trust_level / 2);
            } else {
                readiness = readiness.saturating_add(200);
                benefit = benefit.saturating_add(100);
            }
        }
        let member_count = member_ids.len() as u64;
        readiness = if member_count > 0 {
            readiness / member_count
        } else {
            0
        };
        benefit = benefit.saturating_add(member_count.saturating_mul(50));
        let formation_cost = member_count.saturating_mul(30) + 20;

        let plan = CoalitionPlan {
            coalition_id,
            members: member_ids.clone(),
            purpose_hash,
            readiness_score: readiness,
            formation_steps: member_count.saturating_mul(2),
            benefit_estimate: benefit,
        };
        self.coalition_plans.insert(coalition_id, plan);

        AnticipatedCoalition {
            coalition_id,
            member_ids,
            purpose_hash,
            estimated_benefit: benefit,
            formation_cost,
            readiness,
        }
    }

    /// Create a proactive contract for future cooperation.
    pub fn proactive_contract(
        &mut self,
        parties: Vec<u64>,
        resource_name: &str,
        terms: &str,
        activation_delay: u64,
        duration: u64,
    ) -> ProactiveContract {
        self.stats.contracts_created = self.stats.contracts_created.saturating_add(1);

        let resource_hash = fnv1a_hash(resource_name.as_bytes());
        let terms_hash = fnv1a_hash(terms.as_bytes());
        let contract_id = fnv1a_hash(
            &[
                resource_hash.to_le_bytes(),
                terms_hash.to_le_bytes(),
                self.stats.contracts_created.to_le_bytes(),
            ]
            .concat(),
        );

        let penalty_rate = 50 + (xorshift64(&mut self.rng_state) % 100);
        let activation_tick = self.current_tick.saturating_add(activation_delay);

        let contract = ProactiveContract {
            contract_id,
            parties: parties.clone(),
            resource_hash,
            terms_hash,
            activation_tick,
            duration_ticks: duration,
            penalty_rate,
        };
        self.contracts.insert(contract_id, contract.clone());
        contract
    }

    /// Estimate the negotiation cost savings from proactive preparation.
    pub fn savings_estimate(&mut self) -> u64 {
        let active_slots = self
            .negotiation_slots
            .values()
            .filter(|s| !s.activated && s.expiry_tick > self.current_tick)
            .count() as u64;

        let trust_coverage = self
            .trust_bootstraps
            .values()
            .filter(|t| t.expiry_tick > self.current_tick)
            .count() as u64;

        let coalition_readiness: u64 = self
            .coalition_plans
            .values()
            .map(|c| c.readiness_score)
            .sum::<u64>()
            / self.coalition_plans.len().max(1) as u64;

        let slot_savings = active_slots.saturating_mul(80);
        let trust_savings = trust_coverage.saturating_mul(120);
        let coalition_savings =
            coalition_readiness.saturating_mul(self.coalition_plans.len() as u64) / 10;

        let total_savings = slot_savings
            .saturating_add(trust_savings)
            .saturating_add(coalition_savings);
        self.stats.total_negotiation_cost_saved = self
            .stats
            .total_negotiation_cost_saved
            .saturating_add(total_savings / 100);

        if self.savings_history.len() >= 64 {
            self.savings_history.pop_front();
        }
        self.savings_history.push_back(total_savings);
        self.stats.avg_savings = ema_update(self.stats.avg_savings, total_savings, 3, 10);

        total_savings
    }

    /// Activate a pre-negotiated slot, converting it to a live reservation.
    #[inline]
    pub fn activate_slot(&mut self, contract_id: u64) -> bool {
        if let Some(slot) = self.negotiation_slots.get_mut(&contract_id) {
            if !slot.activated && slot.expiry_tick > self.current_tick {
                slot.activated = true;
                return true;
            }
        }
        false
    }

    /// Get the current statistics snapshot.
    #[inline(always)]
    pub fn stats(&self) -> &ProactiveStats {
        &self.stats
    }

    /// Remove expired negotiation slots.
    fn expire_stale_slots(&mut self) {
        let current = self.current_tick;
        let expired: Vec<u64> = self
            .negotiation_slots
            .iter()
            .filter(|(_, s)| s.expiry_tick <= current)
            .map(|(&k, _)| k)
            .collect();
        for key in expired {
            self.negotiation_slots.remove(&key);
        }
    }

    /// Remove expired trust bootstraps.
    fn expire_stale_trusts(&mut self) {
        let current = self.current_tick;
        let expired: Vec<u64> = self
            .trust_bootstraps
            .iter()
            .filter(|(_, t)| t.expiry_tick <= current)
            .map(|(&k, _)| k)
            .collect();
        for key in expired {
            self.trust_bootstraps.remove(&key);
        }
    }
}
