// SPDX-License-Identifier: GPL-2.0
//! # Holistic Causal Forecast — System-Wide Causal Prediction
//!
//! The ultimate causal model: every event's cause and effect across **all**
//! subsystems. While sub-level causal predictors model within-subsystem
//! causality, this module models the *cross-subsystem* causal fabric — how a
//! memory-pressure event causes scheduler thrashing which causes I/O stalls
//! which causes application latency spikes.
//!
//! ## Capabilities
//!
//! - SystemCausalGraph with cross-subsystem directed edges
//! - Root cause analysis that traces any symptom back to its origin
//! - Causal cascade simulation: "if X happens, what follows?"
//! - Intervention planning: "what should we change to prevent Y?"
//! - Causal completeness metric: how much of the system is modelled
//! - Cross-subsystem causality strength quantification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_CAUSAL_NODES: usize = 1024;
const MAX_CAUSAL_EDGES: usize = 4096;
const MAX_CASCADE_DEPTH: usize = 32;
const MAX_INTERVENTION_PLANS: usize = 128;
const MAX_ROOT_CAUSE_CHAIN: usize = 64;
const CAUSALITY_DECAY: f32 = 0.92;
const MIN_CAUSAL_STRENGTH: f32 = 0.01;
const EMA_ALPHA: f32 = 0.11;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// HELPER FUNCTIONS
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

fn ema_update(current: f32, sample: f32) -> f32 {
    EMA_ALPHA * sample + (1.0 - EMA_ALPHA) * current
}

// ============================================================================
// SUBSYSTEM IDENTIFIERS
// ============================================================================

/// Subsystem identifier for causal graph nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubsystemDomain {
    Scheduler,
    Memory,
    IoSubsystem,
    Network,
    FileSystem,
    Ipc,
    Thermal,
    Power,
    Security,
    Driver,
    Userspace,
    Boot,
}

/// Causal event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CausalEventKind {
    ResourceExhaustion,
    LatencySpike,
    ThroughputDrop,
    ErrorBurst,
    ConfigChange,
    WorkloadShift,
    ThermalThrottle,
    MemoryPressure,
    ContextSwitchStorm,
    DeadlockRisk,
    CacheThrashing,
    InterruptFlood,
}

// ============================================================================
// CAUSAL GRAPH STRUCTURES
// ============================================================================

/// A node in the system causal graph representing an observable event
#[derive(Debug, Clone)]
pub struct CausalNode {
    pub node_id: u64,
    pub domain: SubsystemDomain,
    pub event_kind: CausalEventKind,
    pub description: String,
    pub severity: f32,
    pub observed_count: u64,
    pub last_seen_us: u64,
    pub ema_frequency: f32,
}

/// A directed causal edge between two nodes
#[derive(Debug, Clone)]
pub struct CausalEdge {
    pub from_node: u64,
    pub to_node: u64,
    pub causal_strength: f32,
    pub latency_us: u64,
    pub confidence: f32,
    pub observation_count: u64,
    pub cross_subsystem: bool,
}

/// Root cause analysis result
#[derive(Debug, Clone)]
pub struct RootCauseChain {
    pub symptom_node: u64,
    pub root_node: u64,
    pub chain: Vec<u64>,
    pub total_strength: f32,
    pub total_latency_us: u64,
    pub confidence: f32,
    pub crosses_subsystems: usize,
}

/// Causal cascade prediction from a trigger event
#[derive(Debug, Clone)]
pub struct CausalCascade {
    pub trigger_node: u64,
    pub affected_nodes: Vec<u64>,
    pub cascade_depth: usize,
    pub total_impact: f32,
    pub subsystems_affected: Vec<SubsystemDomain>,
    pub estimated_duration_us: u64,
}

/// An intervention plan to prevent or mitigate a causal cascade
#[derive(Debug, Clone)]
pub struct InterventionPlan {
    pub plan_id: u64,
    pub target_edge: (u64, u64),
    pub intervention_type: InterventionType,
    pub expected_reduction: f32,
    pub cost: f32,
    pub priority: f32,
    pub description: String,
}

/// Type of intervention
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterventionType {
    BreakCausalLink,
    ReduceStrength,
    AddBuffer,
    Reroute,
    Throttle,
    Isolate,
}

/// Causal completeness report
#[derive(Debug, Clone)]
pub struct CausalCompleteness {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub domains_covered: usize,
    pub domains_total: usize,
    pub edge_coverage: f32,
    pub orphan_nodes: usize,
    pub completeness_score: f32,
}

/// Cross-subsystem causality summary
#[derive(Debug, Clone)]
pub struct CrossSubsystemCausality {
    pub from_domain: SubsystemDomain,
    pub to_domain: SubsystemDomain,
    pub edge_count: usize,
    pub avg_strength: f32,
    pub max_strength: f32,
    pub avg_latency_us: u64,
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Runtime statistics for the causal forecast engine
#[derive(Debug, Clone)]
pub struct CausalForecastStats {
    pub predictions_made: u64,
    pub root_cause_analyses: u64,
    pub cascades_simulated: u64,
    pub interventions_planned: u64,
    pub completeness_audits: u64,
    pub cross_subsystem_queries: u64,
    pub avg_chain_length: f32,
    pub avg_cascade_depth: f32,
    pub avg_causal_strength: f32,
    pub graph_density: f32,
}

impl CausalForecastStats {
    fn new() -> Self {
        Self {
            predictions_made: 0,
            root_cause_analyses: 0,
            cascades_simulated: 0,
            interventions_planned: 0,
            completeness_audits: 0,
            cross_subsystem_queries: 0,
            avg_chain_length: 0.0,
            avg_cascade_depth: 0.0,
            avg_causal_strength: 0.0,
            graph_density: 0.0,
        }
    }
}

// ============================================================================
// HOLISTIC CAUSAL FORECAST ENGINE
// ============================================================================

/// System-wide causal prediction engine
pub struct HolisticCausalForecast {
    nodes: BTreeMap<u64, CausalNode>,
    edges: Vec<CausalEdge>,
    adjacency: BTreeMap<u64, Vec<u64>>,
    reverse_adjacency: BTreeMap<u64, Vec<u64>>,
    intervention_log: Vec<InterventionPlan>,
    rng_state: u64,
    next_node_id: u64,
    next_plan_id: u64,
    stats: CausalForecastStats,
    generation: u64,
}

impl HolisticCausalForecast {
    /// Create a new holistic causal forecast engine
    pub fn new(seed: u64) -> Self {
        Self {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            adjacency: BTreeMap::new(),
            reverse_adjacency: BTreeMap::new(),
            intervention_log: Vec::new(),
            rng_state: seed ^ 0xCAFE_BABE_1234_5678,
            next_node_id: 1,
            next_plan_id: 1,
            stats: CausalForecastStats::new(),
            generation: 0,
        }
    }

    /// Register a causal event node in the system graph
    pub fn register_event(
        &mut self,
        domain: SubsystemDomain,
        kind: CausalEventKind,
        severity: f32,
        timestamp_us: u64,
    ) -> u64 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        let node = CausalNode {
            node_id: id,
            domain,
            event_kind: kind,
            description: String::new(),
            severity: severity.clamp(0.0, 1.0),
            observed_count: 1,
            last_seen_us: timestamp_us,
            ema_frequency: 0.0,
        };
        self.nodes.insert(id, node);
        id
    }

    /// Add a causal edge between two events
    pub fn add_causal_link(
        &mut self,
        from: u64,
        to: u64,
        strength: f32,
        latency_us: u64,
    ) {
        let cross = self.nodes.get(&from).map(|n| n.domain)
            != self.nodes.get(&to).map(|n| n.domain);
        if self.edges.len() < MAX_CAUSAL_EDGES {
            self.edges.push(CausalEdge {
                from_node: from,
                to_node: to,
                causal_strength: strength.clamp(0.0, 1.0),
                latency_us,
                confidence: 0.5,
                observation_count: 1,
                cross_subsystem: cross,
            });
            self.adjacency.entry(from).or_insert_with(Vec::new).push(to);
            self.reverse_adjacency.entry(to).or_insert_with(Vec::new).push(from);
        }
    }

    /// Predict causal effects from an event across the entire system
    pub fn system_causal_predict(&mut self, trigger_id: u64) -> CausalCascade {
        self.stats.predictions_made += 1;
        self.generation += 1;
        self.simulate_cascade(trigger_id)
    }

    /// Trace the root cause of a symptom event back through the causal graph
    pub fn root_cause_analysis(&mut self, symptom_id: u64) -> RootCauseChain {
        self.stats.root_cause_analyses += 1;
        let mut chain: Vec<u64> = Vec::new();
        let mut current = symptom_id;
        let mut total_strength = 1.0_f32;
        let mut total_latency = 0_u64;
        let mut visited: BTreeMap<u64, bool> = BTreeMap::new();
        let mut cross_count = 0_usize;

        chain.push(current);
        visited.insert(current, true);

        while chain.len() < MAX_ROOT_CAUSE_CHAIN {
            let parents = self.reverse_adjacency.get(&current).cloned().unwrap_or_default();
            let strongest = parents
                .iter()
                .filter(|p| !visited.contains_key(p))
                .max_by(|a, b| {
                    let sa = self.edge_strength(**a, current);
                    let sb = self.edge_strength(**b, current);
                    sa.partial_cmp(&sb).unwrap_or(core::cmp::Ordering::Equal)
                })
                .copied();

            match strongest {
                Some(parent_id) => {
                    let edge_str = self.edge_strength(parent_id, current);
                    let edge_lat = self.edge_latency(parent_id, current);
                    total_strength *= edge_str;
                    total_latency += edge_lat;
                    if self.is_cross_subsystem(parent_id, current) {
                        cross_count += 1;
                    }
                    visited.insert(parent_id, true);
                    chain.push(parent_id);
                    current = parent_id;
                }
                None => break,
            }
        }

        let root = *chain.last().unwrap_or(&symptom_id);
        self.stats.avg_chain_length =
            ema_update(self.stats.avg_chain_length, chain.len() as f32);

        RootCauseChain {
            symptom_node: symptom_id,
            root_node: root,
            chain,
            total_strength,
            total_latency_us: total_latency,
            confidence: total_strength * CAUSALITY_DECAY,
            crosses_subsystems: cross_count,
        }
    }

    /// Simulate a full causal cascade from a trigger event
    pub fn causal_cascade(&mut self, trigger_id: u64, max_depth: usize) -> CausalCascade {
        self.stats.cascades_simulated += 1;
        let depth = if max_depth > MAX_CASCADE_DEPTH { MAX_CASCADE_DEPTH } else { max_depth };
        let mut affected: Vec<u64> = Vec::new();
        let mut frontier: Vec<u64> = Vec::new();
        let mut visited: BTreeMap<u64, bool> = BTreeMap::new();
        let mut total_impact = 0.0_f32;
        let mut subsystems: BTreeMap<u8, SubsystemDomain> = BTreeMap::new();
        let mut total_latency = 0_u64;

        frontier.push(trigger_id);
        visited.insert(trigger_id, true);

        for _d in 0..depth {
            let mut next_frontier: Vec<u64> = Vec::new();
            for &node_id in &frontier {
                let children = self.adjacency.get(&node_id).cloned().unwrap_or_default();
                for &child in &children {
                    if visited.contains_key(&child) {
                        continue;
                    }
                    let strength = self.edge_strength(node_id, child);
                    if strength < MIN_CAUSAL_STRENGTH {
                        continue;
                    }
                    visited.insert(child, true);
                    affected.push(child);
                    total_impact += strength
                        * self.nodes.get(&child).map(|n| n.severity).unwrap_or(0.0);
                    total_latency += self.edge_latency(node_id, child);
                    if let Some(n) = self.nodes.get(&child) {
                        subsystems.insert(n.domain as u8, n.domain);
                    }
                    next_frontier.push(child);
                }
            }
            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
        }

        self.stats.avg_cascade_depth =
            ema_update(self.stats.avg_cascade_depth, affected.len() as f32);

        CausalCascade {
            trigger_node: trigger_id,
            affected_nodes: affected,
            cascade_depth: depth,
            total_impact,
            subsystems_affected: subsystems.values().copied().collect(),
            estimated_duration_us: total_latency,
        }
    }

    /// Plan interventions to prevent or mitigate cascades
    pub fn intervention_planning(&mut self, cascade: &CausalCascade) -> Vec<InterventionPlan> {
        self.stats.interventions_planned += 1;
        let mut plans: Vec<InterventionPlan> = Vec::new();

        for window in cascade.affected_nodes.windows(2) {
            let from = window[0];
            let to = window[1];
            let strength = self.edge_strength(from, to);
            if strength < 0.3 {
                continue;
            }
            let intervention = if strength > 0.8 {
                InterventionType::Isolate
            } else if strength > 0.6 {
                InterventionType::BreakCausalLink
            } else if strength > 0.4 {
                InterventionType::Throttle
            } else {
                InterventionType::ReduceStrength
            };
            let plan_id = self.next_plan_id;
            self.next_plan_id += 1;
            let plan = InterventionPlan {
                plan_id,
                target_edge: (from, to),
                intervention_type: intervention,
                expected_reduction: strength * 0.6,
                cost: (1.0 - strength) * 0.5,
                priority: strength * cascade.total_impact,
                description: String::new(),
            };
            if plans.len() < MAX_INTERVENTION_PLANS {
                plans.push(plan);
            }
        }
        plans.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(core::cmp::Ordering::Equal));
        for p in &plans {
            if self.intervention_log.len() < MAX_INTERVENTION_PLANS {
                self.intervention_log.push(p.clone());
            }
        }
        plans
    }

    /// Audit the completeness of the causal graph
    pub fn causal_completeness(&self) -> CausalCompleteness {
        let total_nodes = self.nodes.len();
        let total_edges = self.edges.len();
        let mut domain_set: BTreeMap<u8, bool> = BTreeMap::new();
        for n in self.nodes.values() {
            domain_set.insert(n.domain as u8, true);
        }
        let domains_covered = domain_set.len();
        let domains_total = 12;
        let orphans = self
            .nodes
            .keys()
            .filter(|id| {
                !self.adjacency.contains_key(id) && !self.reverse_adjacency.contains_key(id)
            })
            .count();
        let max_edges = if total_nodes > 1 { total_nodes * (total_nodes - 1) } else { 1 };
        let edge_coverage = total_edges as f32 / max_edges as f32;
        let completeness = (domains_covered as f32 / domains_total as f32) * 0.4
            + edge_coverage.min(1.0) * 0.3
            + (1.0 - orphans as f32 / total_nodes.max(1) as f32) * 0.3;

        CausalCompleteness {
            total_nodes,
            total_edges,
            domains_covered,
            domains_total,
            edge_coverage,
            orphan_nodes: orphans,
            completeness_score: completeness.clamp(0.0, 1.0),
        }
    }

    /// Query cross-subsystem causality between two domains
    pub fn cross_subsystem_causality(
        &mut self,
        from: SubsystemDomain,
        to: SubsystemDomain,
    ) -> CrossSubsystemCausality {
        self.stats.cross_subsystem_queries += 1;
        let relevant: Vec<&CausalEdge> = self
            .edges
            .iter()
            .filter(|e| {
                let from_domain = self.nodes.get(&e.from_node).map(|n| n.domain);
                let to_domain = self.nodes.get(&e.to_node).map(|n| n.domain);
                from_domain == Some(from) && to_domain == Some(to)
            })
            .collect();

        let count = relevant.len();
        let avg_str = if count > 0 {
            relevant.iter().map(|e| e.causal_strength).sum::<f32>() / count as f32
        } else {
            0.0
        };
        let max_str = relevant
            .iter()
            .map(|e| e.causal_strength)
            .fold(0.0_f32, f32::max);
        let avg_lat = if count > 0 {
            relevant.iter().map(|e| e.latency_us).sum::<u64>() / count as u64
        } else {
            0
        };

        self.stats.avg_causal_strength = ema_update(self.stats.avg_causal_strength, avg_str);

        CrossSubsystemCausality {
            from_domain: from,
            to_domain: to,
            edge_count: count,
            avg_strength: avg_str,
            max_strength: max_str,
            avg_latency_us: avg_lat,
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> &CausalForecastStats {
        &self.stats
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    fn simulate_cascade(&mut self, trigger_id: u64) -> CausalCascade {
        self.causal_cascade(trigger_id, MAX_CASCADE_DEPTH)
    }

    fn edge_strength(&self, from: u64, to: u64) -> f32 {
        self.edges
            .iter()
            .find(|e| e.from_node == from && e.to_node == to)
            .map(|e| e.causal_strength)
            .unwrap_or(0.0)
    }

    fn edge_latency(&self, from: u64, to: u64) -> u64 {
        self.edges
            .iter()
            .find(|e| e.from_node == from && e.to_node == to)
            .map(|e| e.latency_us)
            .unwrap_or(0)
    }

    fn is_cross_subsystem(&self, a: u64, b: u64) -> bool {
        let da = self.nodes.get(&a).map(|n| n.domain);
        let db = self.nodes.get(&b).map(|n| n.domain);
        da != db
    }
}
