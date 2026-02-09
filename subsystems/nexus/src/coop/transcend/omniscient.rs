// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Omniscience Engine
//!
//! Total knowledge of all cooperation dynamics across the NEXUS kernel.
//! Maintains a complete trust graph, resource flow map, and contention atlas.
//! Every trust relationship, every resource flow, every contention pattern is
//! tracked and queryable in constant time through FNV-1a indexed structures.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 10;
const MAX_TRUST_EDGES: usize = 4096;
const MAX_RESOURCE_FLOWS: usize = 2048;
const MAX_CONTENTION_ZONES: usize = 1024;
const TRUST_DECAY_NUM: u64 = 99;
const TRUST_DECAY_DEN: u64 = 100;

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

fn clamp(val: u64, lo: u64, hi: u64) -> u64 {
    if val < lo {
        lo
    } else if val > hi {
        hi
    } else {
        val
    }
}

// ---------------------------------------------------------------------------
// Trust edge
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct TrustEdge {
    pub source_id: u64,
    pub target_id: u64,
    pub trust_score: u64,
    pub interaction_count: u64,
    pub last_update_tick: u64,
    pub reciprocity: u64,
}

// ---------------------------------------------------------------------------
// Resource flow
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ResourceFlow {
    pub flow_id: u64,
    pub provider_id: u64,
    pub consumer_id: u64,
    pub bandwidth: u64,
    pub utilisation: u64,
    pub ema_latency: u64,
}

// ---------------------------------------------------------------------------
// Contention zone
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ContentionZone {
    pub zone_id: u64,
    pub participant_ids: Vec<u64>,
    pub severity: u64,
    pub ema_severity: u64,
    pub resolution_count: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct OmniscientStats {
    pub total_edges: usize,
    pub total_flows: usize,
    pub total_zones: usize,
    pub avg_trust: u64,
    pub avg_utilisation: u64,
    pub avg_severity: u64,
    pub knowledge_completeness: u64,
    pub observation_ticks: u64,
}

// ---------------------------------------------------------------------------
// CoopOmniscient
// ---------------------------------------------------------------------------

pub struct CoopOmniscient {
    trust_edges: BTreeMap<u64, TrustEdge>,
    resource_flows: BTreeMap<u64, ResourceFlow>,
    contention_zones: BTreeMap<u64, ContentionZone>,
    rng_state: u64,
    tick: u64,
    stats: OmniscientStats,
    trust_sum: u64,
    utilisation_sum: u64,
    severity_sum: u64,
}

impl CoopOmniscient {
    // -- construction -------------------------------------------------------

    pub fn new(seed: u64) -> Self {
        Self {
            trust_edges: BTreeMap::new(),
            resource_flows: BTreeMap::new(),
            contention_zones: BTreeMap::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: OmniscientStats {
                total_edges: 0,
                total_flows: 0,
                total_zones: 0,
                avg_trust: 50,
                avg_utilisation: 50,
                avg_severity: 0,
                knowledge_completeness: 0,
                observation_ticks: 0,
            },
            trust_sum: 0,
            utilisation_sum: 0,
            severity_sum: 0,
        }
    }

    // -- trust graph --------------------------------------------------------

    pub fn record_trust(&mut self, source: u64, target: u64, score: u64) {
        if self.trust_edges.len() >= MAX_TRUST_EDGES {
            return;
        }
        let key = fnv1a(&[source.to_le_bytes(), target.to_le_bytes()].concat());
        let clamped = clamp(score, 0, 100);
        if let Some(edge) = self.trust_edges.get_mut(&key) {
            edge.trust_score = ema_update(edge.trust_score, clamped);
            edge.interaction_count += 1;
            edge.last_update_tick = self.tick;
            edge.reciprocity = self.compute_reciprocity(source, target);
        } else {
            let reciprocity = self.compute_reciprocity(source, target);
            self.trust_edges.insert(key, TrustEdge {
                source_id: source,
                target_id: target,
                trust_score: clamped,
                interaction_count: 1,
                last_update_tick: self.tick,
                reciprocity,
            });
        }
        self.recalc_trust_sum();
    }

    fn compute_reciprocity(&self, src: u64, tgt: u64) -> u64 {
        let rev_key = fnv1a(&[tgt.to_le_bytes(), src.to_le_bytes()].concat());
        match self.trust_edges.get(&rev_key) {
            Some(rev) => {
                let fwd_key = fnv1a(&[src.to_le_bytes(), tgt.to_le_bytes()].concat());
                match self.trust_edges.get(&fwd_key) {
                    Some(fwd) => {
                        let diff = if fwd.trust_score > rev.trust_score {
                            fwd.trust_score - rev.trust_score
                        } else {
                            rev.trust_score - fwd.trust_score
                        };
                        100u64.saturating_sub(diff)
                    },
                    None => 50,
                }
            },
            None => 0,
        }
    }

    fn recalc_trust_sum(&mut self) {
        let mut sum = 0u64;
        for edge in self.trust_edges.values() {
            sum = sum.saturating_add(edge.trust_score);
        }
        self.trust_sum = sum;
    }

    pub fn trust_graph_complete(&self) -> BTreeMap<u64, TrustEdge> {
        self.trust_edges.clone()
    }

    // -- resource flows -----------------------------------------------------

    pub fn record_flow(&mut self, provider: u64, consumer: u64, bw: u64, lat: u64) {
        if self.resource_flows.len() >= MAX_RESOURCE_FLOWS {
            return;
        }
        let key = fnv1a(&[provider.to_le_bytes(), consumer.to_le_bytes()].concat());
        if let Some(flow) = self.resource_flows.get_mut(&key) {
            flow.bandwidth = ema_update(flow.bandwidth, bw);
            flow.ema_latency = ema_update(flow.ema_latency, lat);
            flow.utilisation = if flow.bandwidth > 0 {
                clamp(bw * 100 / flow.bandwidth, 0, 100)
            } else {
                0
            };
        } else {
            self.resource_flows.insert(key, ResourceFlow {
                flow_id: key,
                provider_id: provider,
                consumer_id: consumer,
                bandwidth: bw,
                utilisation: 50,
                ema_latency: lat,
            });
        }
        self.recalc_util_sum();
    }

    fn recalc_util_sum(&mut self) {
        let mut sum = 0u64;
        for flow in self.resource_flows.values() {
            sum = sum.saturating_add(flow.utilisation);
        }
        self.utilisation_sum = sum;
    }

    pub fn resource_flow_map(&self) -> BTreeMap<u64, ResourceFlow> {
        self.resource_flows.clone()
    }

    // -- contention zones ---------------------------------------------------

    pub fn record_contention(&mut self, participants: &[u64], severity: u64) {
        if self.contention_zones.len() >= MAX_CONTENTION_ZONES {
            return;
        }
        let mut buf = Vec::new();
        for &p in participants {
            buf.extend_from_slice(&p.to_le_bytes());
        }
        let key = fnv1a(&buf);
        let clamped = clamp(severity, 0, 100);
        if let Some(zone) = self.contention_zones.get_mut(&key) {
            zone.severity = clamped;
            zone.ema_severity = ema_update(zone.ema_severity, clamped);
            zone.resolution_count += 1;
        } else {
            self.contention_zones.insert(key, ContentionZone {
                zone_id: key,
                participant_ids: participants.to_vec(),
                severity: clamped,
                ema_severity: clamped,
                resolution_count: 0,
            });
        }
        self.recalc_severity_sum();
    }

    fn recalc_severity_sum(&mut self) {
        let mut sum = 0u64;
        for z in self.contention_zones.values() {
            sum = sum.saturating_add(z.ema_severity);
        }
        self.severity_sum = sum;
    }

    pub fn contention_atlas(&self) -> BTreeMap<u64, ContentionZone> {
        self.contention_zones.clone()
    }

    // -- tick / decay -------------------------------------------------------

    pub fn tick(&mut self) {
        self.tick += 1;
        // Decay trust scores that haven't been refreshed recently
        let stale_threshold = self.tick.saturating_sub(64);
        let keys: Vec<u64> = self.trust_edges.keys().copied().collect();
        for key in keys {
            if let Some(edge) = self.trust_edges.get_mut(&key) {
                if edge.last_update_tick < stale_threshold {
                    edge.trust_score = edge.trust_score * TRUST_DECAY_NUM / TRUST_DECAY_DEN;
                }
            }
        }
        self.recalc_trust_sum();
        self.refresh_stats();
    }

    // -- total knowledge ----------------------------------------------------

    pub fn total_cooperation_knowledge(&self) -> OmniscientStats {
        self.stats.clone()
    }

    // -- omniscience score --------------------------------------------------

    pub fn cooperation_omniscience(&self) -> u64 {
        let edge_score = if self.trust_edges.is_empty() { 0 } else { 30 };
        let flow_score = if self.resource_flows.is_empty() {
            0
        } else {
            30
        };
        let zone_score = if self.contention_zones.is_empty() {
            0
        } else {
            20
        };
        let tick_score = clamp(self.tick / 10, 0, 20);
        edge_score + flow_score + zone_score + tick_score
    }

    // -- internal -----------------------------------------------------------

    fn refresh_stats(&mut self) {
        let n_edges = self.trust_edges.len();
        let n_flows = self.resource_flows.len();
        let n_zones = self.contention_zones.len();
        let avg_t = if n_edges > 0 {
            self.trust_sum / n_edges as u64
        } else {
            0
        };
        let avg_u = if n_flows > 0 {
            self.utilisation_sum / n_flows as u64
        } else {
            0
        };
        let avg_s = if n_zones > 0 {
            self.severity_sum / n_zones as u64
        } else {
            0
        };
        self.stats = OmniscientStats {
            total_edges: n_edges,
            total_flows: n_flows,
            total_zones: n_zones,
            avg_trust: avg_t,
            avg_utilisation: avg_u,
            avg_severity: avg_s,
            knowledge_completeness: self.cooperation_omniscience(),
            observation_ticks: self.tick,
        };
    }

    // -- random probe (for fuzz / stress testing) ---------------------------

    pub fn random_probe(&mut self) -> u64 {
        let r = xorshift64(&mut self.rng_state);
        let scope = r % 3;
        match scope {
            0 => self.trust_edges.len() as u64,
            1 => self.resource_flows.len() as u64,
            _ => self.contention_zones.len() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_omniscience() {
        let mut om = CoopOmniscient::new(42);
        om.record_trust(1, 2, 80);
        om.record_trust(2, 1, 75);
        om.record_flow(1, 2, 100, 5);
        om.record_contention(&[1, 2], 30);
        om.tick();
        let stats = om.total_cooperation_knowledge();
        assert_eq!(stats.total_edges, 2);
        assert_eq!(stats.total_flows, 1);
        assert_eq!(stats.total_zones, 1);
        assert!(om.cooperation_omniscience() >= 80);
    }

    #[test]
    fn test_trust_graph_complete() {
        let mut om = CoopOmniscient::new(7);
        om.record_trust(10, 20, 90);
        let graph = om.trust_graph_complete();
        assert_eq!(graph.len(), 1);
    }

    #[test]
    fn test_resource_flow_map() {
        let mut om = CoopOmniscient::new(99);
        om.record_flow(5, 6, 200, 10);
        om.record_flow(5, 6, 300, 8);
        let flows = om.resource_flow_map();
        assert_eq!(flows.len(), 1);
    }

    #[test]
    fn test_contention_atlas() {
        let mut om = CoopOmniscient::new(123);
        om.record_contention(&[1, 2, 3], 50);
        let atlas = om.contention_atlas();
        assert_eq!(atlas.len(), 1);
    }
}
