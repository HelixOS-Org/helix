// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Timeline Projector
//!
//! Projects cooperation timelines including contract expirations, trust decay
//! curves, resource lease schedules, and negotiation cycles. Provides a
//! temporal view of all cooperative commitments and obligations.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic key generation.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for jitter and perturbation.
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

/// A projected contract event on the timeline.
#[derive(Clone, Debug)]
pub struct ContractEvent {
    pub contract_id: u64,
    pub event_type: ContractEventType,
    pub tick: u64,
    pub parties: Vec<u64>,
    pub impact_score: u64,
}

/// Types of contract lifecycle events.
#[derive(Clone, Debug, PartialEq)]
pub enum ContractEventType {
    Activation,
    Renewal,
    Expiration,
    Renegotiation,
    Termination,
}

/// Trust decay data point on the projected curve.
#[derive(Clone, Debug)]
pub struct TrustDecayPoint {
    pub partner_id: u64,
    pub tick: u64,
    pub projected_trust: u64,
    pub decay_rate: u64,
    pub critical: bool,
}

/// Resource lease timeline entry.
#[derive(Clone, Debug)]
pub struct LeaseEntry {
    pub lease_id: u64,
    pub resource_hash: u64,
    pub holder_id: u64,
    pub start_tick: u64,
    pub end_tick: u64,
    pub renewal_probability: u64,
}

/// Negotiation schedule entry.
#[derive(Clone, Debug)]
pub struct NegotiationEntry {
    pub negotiation_id: u64,
    pub participants: Vec<u64>,
    pub scheduled_tick: u64,
    pub estimated_duration: u64,
    pub priority: u64,
    pub topic_hash: u64,
}

/// Risk assessment for a timeline window.
#[derive(Clone, Debug)]
pub struct TimelineRisk {
    pub window_start: u64,
    pub window_end: u64,
    pub expiration_count: u64,
    pub trust_critical_count: u64,
    pub lease_gaps: u64,
    pub overall_risk: u64,
}

/// Rolling statistics for the timeline projector.
#[derive(Clone, Debug)]
pub struct TimelineStats {
    pub contracts_tracked: u64,
    pub leases_tracked: u64,
    pub trust_curves_projected: u64,
    pub negotiations_scheduled: u64,
    pub risk_assessments: u64,
    pub avg_risk_score: u64,
    pub timeline_errors: u64,
}

impl TimelineStats {
    pub fn new() -> Self {
        Self {
            contracts_tracked: 0,
            leases_tracked: 0,
            trust_curves_projected: 0,
            negotiations_scheduled: 0,
            risk_assessments: 0,
            avg_risk_score: 200,
            timeline_errors: 0,
        }
    }
}

/// Internal contract record.
#[derive(Clone, Debug)]
struct ContractRecord {
    contract_id: u64,
    parties: Vec<u64>,
    start_tick: u64,
    end_tick: u64,
    renewal_chance: u64,
    renegotiation_interval: u64,
    value: u64,
}

/// Internal trust decay model.
#[derive(Clone, Debug)]
struct TrustDecayModel {
    partner_id: u64,
    current_trust: u64,
    half_life_ticks: u64,
    floor_trust: u64,
    last_interaction_tick: u64,
}

/// Internal lease record.
#[derive(Clone, Debug)]
struct LeaseRecord {
    lease_id: u64,
    resource_hash: u64,
    holder_id: u64,
    start_tick: u64,
    duration_ticks: u64,
    renewal_prob: u64,
}

/// Cooperation timeline projection engine.
pub struct CoopTimeline {
    contracts: BTreeMap<u64, ContractRecord>,
    trust_models: BTreeMap<u64, TrustDecayModel>,
    leases: BTreeMap<u64, LeaseRecord>,
    negotiations: BTreeMap<u64, NegotiationEntry>,
    risk_history: Vec<u64>,
    stats: TimelineStats,
    rng_state: u64,
    current_tick: u64,
    max_entries: usize,
}

impl CoopTimeline {
    /// Create a new timeline projector with the given seed.
    pub fn new(seed: u64) -> Self {
        Self {
            contracts: BTreeMap::new(),
            trust_models: BTreeMap::new(),
            leases: BTreeMap::new(),
            negotiations: BTreeMap::new(),
            risk_history: Vec::new(),
            stats: TimelineStats::new(),
            rng_state: seed | 1,
            current_tick: 0,
            max_entries: 256,
        }
    }

    /// Advance the internal clock.
    pub fn tick(&mut self, now: u64) {
        self.current_tick = now;
    }

    /// Register a contract for timeline tracking.
    pub fn register_contract(
        &mut self,
        parties: Vec<u64>,
        start_tick: u64,
        duration: u64,
        renewal_chance: u64,
        renegotiation_interval: u64,
        value: u64,
    ) -> u64 {
        self.stats.contracts_tracked = self.stats.contracts_tracked.saturating_add(1);
        let contract_id = fnv1a_hash(
            &[
                self.stats.contracts_tracked.to_le_bytes(),
                start_tick.to_le_bytes(),
            ]
            .concat(),
        );

        if self.contracts.len() < self.max_entries {
            self.contracts.insert(contract_id, ContractRecord {
                contract_id,
                parties,
                start_tick,
                end_tick: start_tick.saturating_add(duration),
                renewal_chance: renewal_chance.min(1000),
                renegotiation_interval,
                value,
            });
        }
        contract_id
    }

    /// Register a trust decay model for a partner.
    pub fn register_trust_model(
        &mut self,
        partner_id: u64,
        current_trust: u64,
        half_life: u64,
        floor: u64,
    ) {
        let key = fnv1a_hash(&partner_id.to_le_bytes());
        self.trust_models.insert(key, TrustDecayModel {
            partner_id,
            current_trust: current_trust.min(1000),
            half_life_ticks: half_life.max(1),
            floor_trust: floor.min(current_trust),
            last_interaction_tick: self.current_tick,
        });
    }

    /// Register a resource lease.
    pub fn register_lease(
        &mut self,
        resource_name: &str,
        holder_id: u64,
        start_tick: u64,
        duration: u64,
        renewal_prob: u64,
    ) -> u64 {
        self.stats.leases_tracked = self.stats.leases_tracked.saturating_add(1);
        let resource_hash = fnv1a_hash(resource_name.as_bytes());
        let lease_id = fnv1a_hash(
            &[
                resource_hash.to_le_bytes(),
                holder_id.to_le_bytes(),
                start_tick.to_le_bytes(),
            ]
            .concat(),
        );

        if self.leases.len() < self.max_entries {
            self.leases.insert(lease_id, LeaseRecord {
                lease_id,
                resource_hash,
                holder_id,
                start_tick,
                duration_ticks: duration,
                renewal_prob: renewal_prob.min(1000),
            });
        }
        lease_id
    }

    /// Project contract events within a future window.
    pub fn project_contracts(&self, window_start: u64, window_end: u64) -> Vec<ContractEvent> {
        let mut events = Vec::new();
        for record in self.contracts.values() {
            if record.start_tick >= window_start && record.start_tick <= window_end {
                events.push(ContractEvent {
                    contract_id: record.contract_id,
                    event_type: ContractEventType::Activation,
                    tick: record.start_tick,
                    parties: record.parties.clone(),
                    impact_score: record.value,
                });
            }
            if record.end_tick >= window_start && record.end_tick <= window_end {
                let ev_type = if record.renewal_chance > 500 {
                    ContractEventType::Renewal
                } else {
                    ContractEventType::Expiration
                };
                events.push(ContractEvent {
                    contract_id: record.contract_id,
                    event_type: ev_type,
                    tick: record.end_tick,
                    parties: record.parties.clone(),
                    impact_score: record.value,
                });
            }
            if record.renegotiation_interval > 0 {
                let mut re_tick = record
                    .start_tick
                    .saturating_add(record.renegotiation_interval);
                while re_tick < record.end_tick && re_tick >= window_start && re_tick <= window_end
                {
                    events.push(ContractEvent {
                        contract_id: record.contract_id,
                        event_type: ContractEventType::Renegotiation,
                        tick: re_tick,
                        parties: record.parties.clone(),
                        impact_score: record.value / 4,
                    });
                    re_tick = re_tick.saturating_add(record.renegotiation_interval);
                }
            }
        }
        events.sort_by_key(|e| e.tick);
        events
    }

    /// Project trust decay curve for a partner over a horizon.
    pub fn trust_decay_curve(
        &mut self,
        partner_id: u64,
        horizon_ticks: u64,
        sample_points: u64,
    ) -> Vec<TrustDecayPoint> {
        self.stats.trust_curves_projected = self.stats.trust_curves_projected.saturating_add(1);
        let key = fnv1a_hash(&partner_id.to_le_bytes());

        let model = match self.trust_models.get(&key) {
            Some(m) => m.clone(),
            None => return Vec::new(),
        };

        let mut points = Vec::new();
        let step = if sample_points > 0 {
            horizon_ticks / sample_points
        } else {
            horizon_ticks
        };

        for i in 0..=sample_points.min(64) {
            let tick_offset = i.saturating_mul(step);
            let elapsed = tick_offset.saturating_add(
                self.current_tick
                    .saturating_sub(model.last_interaction_tick),
            );
            let half_lives = elapsed / model.half_life_ticks.max(1);
            let decay_factor = 1u64 << half_lives.min(10);
            let decayed_range = model.current_trust.saturating_sub(model.floor_trust);
            let decayed = decayed_range / decay_factor.max(1);
            let projected = model.floor_trust.saturating_add(decayed);
            let critical = projected < model.current_trust / 3;
            let rate = if elapsed > 0 {
                model
                    .current_trust
                    .saturating_sub(projected)
                    .saturating_mul(1000)
                    / elapsed
            } else {
                0
            };

            points.push(TrustDecayPoint {
                partner_id,
                tick: self.current_tick.saturating_add(tick_offset),
                projected_trust: projected,
                decay_rate: rate,
                critical,
            });
        }
        points
    }

    /// Project the lease timeline within a window, detecting gaps.
    pub fn lease_timeline(&self, window_start: u64, window_end: u64) -> Vec<LeaseEntry> {
        let mut entries = Vec::new();
        for record in self.leases.values() {
            let lease_end = record.start_tick.saturating_add(record.duration_ticks);
            let overlaps = record.start_tick <= window_end && lease_end >= window_start;
            if overlaps {
                entries.push(LeaseEntry {
                    lease_id: record.lease_id,
                    resource_hash: record.resource_hash,
                    holder_id: record.holder_id,
                    start_tick: record.start_tick,
                    end_tick: lease_end,
                    renewal_probability: record.renewal_prob,
                });
            }
        }
        entries.sort_by_key(|e| e.start_tick);
        entries
    }

    /// Schedule a negotiation at a future tick.
    pub fn negotiation_schedule(
        &mut self,
        participants: Vec<u64>,
        scheduled_tick: u64,
        estimated_duration: u64,
        priority: u64,
        topic: &str,
    ) -> u64 {
        self.stats.negotiations_scheduled = self.stats.negotiations_scheduled.saturating_add(1);
        let topic_hash = fnv1a_hash(topic.as_bytes());
        let neg_id = fnv1a_hash(
            &[
                topic_hash.to_le_bytes(),
                scheduled_tick.to_le_bytes(),
                self.stats.negotiations_scheduled.to_le_bytes(),
            ]
            .concat(),
        );

        if self.negotiations.len() < self.max_entries {
            self.negotiations.insert(neg_id, NegotiationEntry {
                negotiation_id: neg_id,
                participants,
                scheduled_tick,
                estimated_duration,
                priority: priority.min(1000),
                topic_hash,
            });
        }
        neg_id
    }

    /// Assess timeline risk for a given window.
    pub fn timeline_risk(&mut self, window_start: u64, window_end: u64) -> TimelineRisk {
        self.stats.risk_assessments = self.stats.risk_assessments.saturating_add(1);

        let mut expiration_count: u64 = 0;
        for record in self.contracts.values() {
            if record.end_tick >= window_start && record.end_tick <= window_end {
                expiration_count = expiration_count.saturating_add(1);
            }
        }

        let mut trust_critical: u64 = 0;
        let keys: Vec<u64> = self.trust_models.keys().copied().collect();
        for key in keys {
            if let Some(model) = self.trust_models.get(&key) {
                let elapsed = window_end.saturating_sub(model.last_interaction_tick);
                let half_lives = elapsed / model.half_life_ticks.max(1);
                let decay_factor = 1u64 << half_lives.min(10);
                let decayed_range = model.current_trust.saturating_sub(model.floor_trust);
                let projected = model
                    .floor_trust
                    .saturating_add(decayed_range / decay_factor.max(1));
                if projected < model.current_trust / 3 {
                    trust_critical = trust_critical.saturating_add(1);
                }
            }
        }

        let mut lease_gaps: u64 = 0;
        let lease_entries = self.lease_timeline(window_start, window_end);
        for i in 1..lease_entries.len() {
            if lease_entries[i].start_tick > lease_entries[i - 1].end_tick {
                lease_gaps = lease_gaps.saturating_add(1);
            }
        }

        let overall_risk = expiration_count
            .saturating_mul(200)
            .saturating_add(trust_critical.saturating_mul(300))
            .saturating_add(lease_gaps.saturating_mul(150))
            .min(1000);

        if self.risk_history.len() >= 64 {
            self.risk_history.remove(0);
        }
        self.risk_history.push(overall_risk);
        self.stats.avg_risk_score = ema_update(self.stats.avg_risk_score, overall_risk, 3, 10);

        TimelineRisk {
            window_start,
            window_end,
            expiration_count,
            trust_critical_count: trust_critical,
            lease_gaps,
            overall_risk,
        }
    }

    /// Get the current statistics snapshot.
    pub fn stats(&self) -> &TimelineStats {
        &self.stats
    }
}
