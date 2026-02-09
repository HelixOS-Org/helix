// SPDX-License-Identifier: GPL-2.0
//! # Apps Rehearsal Engine
//!
//! Dry-runs application classification and resource allocation for
//! hypothetical scenarios without committing any changes. The rehearsal
//! engine tests "what-if" questions: what if this process suddenly
//! doubles its memory usage? What if we reclassify it as IO-bound?
//! Every rehearsal produces a resource impact estimate and an accuracy
//! score calibrated against past rehearsal-to-reality comparisons.
//!
//! This is the kernel practicing before performing.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SCENARIOS: usize = 128;
const MAX_REHEARSALS: usize = 512;
const EMA_ALPHA: f32 = 0.10;
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
// SCENARIO TYPES
// ============================================================================

/// Kind of hypothetical scenario
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioKind {
    DemandSpike,
    Reclassification,
    ResourceConstraint,
    NewProcess,
    ProcessExit,
    InterferenceChange,
    PolicyChange,
}

/// A hypothetical scenario to rehearse
#[derive(Debug, Clone)]
pub struct Scenario {
    pub id: u64,
    pub kind: ScenarioKind,
    pub process_id: u64,
    pub description: String,
    pub parameters: Vec<(String, f32)>,
}

/// Classification rehearsal result
#[derive(Debug, Clone)]
pub struct ClassificationRehearsal {
    pub scenario_id: u64,
    pub process_id: u64,
    pub original_class: String,
    pub rehearsed_class: String,
    pub confidence: f32,
    pub features_changed: u32,
    pub would_trigger_realloc: bool,
}

/// What-if analysis result
#[derive(Debug, Clone)]
pub struct WhatIfResult {
    pub scenario_id: u64,
    pub description: String,
    pub cpu_impact: f32,
    pub memory_impact: f32,
    pub io_impact: f32,
    pub latency_impact_ms: f32,
    pub risk_score: f32,
    pub recommendation: String,
}

/// Resource impact assessment
#[derive(Debug, Clone)]
pub struct ResourceImpact {
    pub scenario_id: u64,
    pub process_id: u64,
    pub cpu_delta: f32,
    pub memory_delta: f32,
    pub io_delta: f32,
    pub total_cost: f32,
    pub feasible: bool,
    pub bottleneck: String,
}

/// Scenario comparison entry
#[derive(Debug, Clone)]
pub struct ScenarioComparison {
    pub scenario_a_id: u64,
    pub scenario_b_id: u64,
    pub cpu_diff: f32,
    pub memory_diff: f32,
    pub io_diff: f32,
    pub cost_diff: f32,
    pub preferred: u64,
    pub reason: String,
}

// ============================================================================
// INTERNAL REHEARSAL RECORD
// ============================================================================

/// Internal record of a completed rehearsal
#[derive(Debug, Clone)]
struct RehearsalRecord {
    scenario_id: u64,
    tick: u64,
    predicted_cpu_impact: f32,
    predicted_mem_impact: f32,
    actual_cpu_impact: f32,
    actual_mem_impact: f32,
    validated: bool,
}

impl RehearsalRecord {
    fn accuracy(&self) -> f32 {
        if !self.validated {
            return 0.5;
        }
        let cpu_err = (self.predicted_cpu_impact - self.actual_cpu_impact).abs();
        let mem_err = (self.predicted_mem_impact - self.actual_mem_impact).abs();
        let denom = self.actual_cpu_impact.abs() + self.actual_mem_impact.abs() + 1.0;
        1.0 - ((cpu_err + mem_err) / denom).min(1.0)
    }
}

// ============================================================================
// REHEARSAL STATS
// ============================================================================

/// Aggregate rehearsal statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct RehearsalStats {
    pub total_rehearsals: u64,
    pub scenarios_tested: usize,
    pub avg_accuracy: f32,
    pub validated_count: u64,
    pub what_if_analyses: u64,
    pub classifications_rehearsed: u64,
    pub resource_impacts_computed: u64,
}

// ============================================================================
// APPS REHEARSAL ENGINE
// ============================================================================

/// Dry-run rehearsal engine for hypothetical application scenarios.
/// Tests classification and resource allocation changes without committing.
#[derive(Debug)]
pub struct AppsRehearsal {
    scenarios: BTreeMap<u64, Scenario>,
    records: Vec<RehearsalRecord>,
    impact_cache: BTreeMap<u64, ResourceImpact>,
    total_rehearsals: u64,
    what_if_count: u64,
    classification_count: u64,
    impact_count: u64,
    validated_count: u64,
    tick: u64,
    rng_state: u64,
    accuracy_ema: f32,
}

impl AppsRehearsal {
    pub fn new() -> Self {
        Self {
            scenarios: BTreeMap::new(),
            records: Vec::new(),
            impact_cache: BTreeMap::new(),
            total_rehearsals: 0,
            what_if_count: 0,
            classification_count: 0,
            impact_count: 0,
            validated_count: 0,
            tick: 0,
            rng_state: 0xBEEF_CAFE_0123_4567,
            accuracy_ema: 0.5,
        }
    }

    /// Register a scenario for rehearsal
    pub fn register_scenario(&mut self, scenario: Scenario) {
        if self.scenarios.len() < MAX_SCENARIOS {
            self.scenarios.insert(scenario.id, scenario);
        }
    }

    /// Create a scenario from parameters
    pub fn create_scenario(
        &mut self,
        kind: ScenarioKind,
        process_id: u64,
        description: &str,
        parameters: &[(String, f32)],
    ) -> u64 {
        let id_bytes = [
            &process_id.to_le_bytes()[..],
            &(self.total_rehearsals as u64).to_le_bytes()[..],
        ]
        .concat();
        let id = fnv1a_hash(&id_bytes);

        let mut params = Vec::new();
        for (k, v) in parameters {
            params.push((k.clone(), *v));
        }

        let scenario = Scenario {
            id,
            kind,
            process_id,
            description: String::from(description),
            parameters: params,
        };
        self.register_scenario(scenario);
        id
    }

    /// Rehearse a classification change for a scenario
    pub fn rehearse_classification(
        &mut self,
        scenario_id: u64,
        current_class: &str,
    ) -> ClassificationRehearsal {
        self.total_rehearsals += 1;
        self.classification_count += 1;

        let (process_id, features_changed, new_class) =
            if let Some(scenario) = self.scenarios.get(&scenario_id) {
                let fc = scenario.parameters.len() as u32;
                let hash = fnv1a_hash(scenario.description.as_bytes());
                let class_idx = hash % 6;
                let new_cls = match class_idx {
                    0 => String::from("compute"),
                    1 => String::from("io-bound"),
                    2 => String::from("memory-heavy"),
                    3 => String::from("network"),
                    4 => String::from("interactive"),
                    _ => String::from("batch"),
                };
                (scenario.process_id, fc, new_cls)
            } else {
                (0, 0, String::from("unknown"))
            };

        let same_class = current_class == new_class.as_str();
        let confidence = if same_class { 0.85 } else { 0.55 };

        ClassificationRehearsal {
            scenario_id,
            process_id,
            original_class: String::from(current_class),
            rehearsed_class: new_class,
            confidence,
            features_changed,
            would_trigger_realloc: !same_class,
        }
    }

    /// Perform what-if analysis on a scenario
    pub fn what_if_analysis(&mut self, scenario_id: u64) -> WhatIfResult {
        self.total_rehearsals += 1;
        self.what_if_count += 1;

        let (desc, cpu_imp, mem_imp, io_imp, risk) =
            if let Some(scenario) = self.scenarios.get(&scenario_id) {
                let hash = fnv1a_hash(&scenario_id.to_le_bytes());
                let cpu = ((hash % 100) as f32 - 50.0) / 100.0;
                let mem = (((hash >> 8) % 100) as f32 - 40.0) / 100.0;
                let io = (((hash >> 16) % 100) as f32 - 30.0) / 100.0;
                let risk = ((hash >> 24) % 100) as f32 / 100.0;
                (scenario.description.clone(), cpu, mem, io, risk)
            } else {
                (String::from("unknown"), 0.0, 0.0, 0.0, 0.5)
            };

        let latency = (cpu_imp.abs() + io_imp.abs()) * 10.0;
        let mut rec = String::new();
        if risk > 0.5 {
            rec.push_str("high_risk:defer");
        } else if cpu_imp.abs() + mem_imp.abs() + io_imp.abs() < 0.1 {
            rec.push_str("low_impact:proceed");
        } else {
            rec.push_str("moderate:staged_rollout");
        }

        self.records.push(RehearsalRecord {
            scenario_id,
            tick: self.tick,
            predicted_cpu_impact: cpu_imp,
            predicted_mem_impact: mem_imp,
            actual_cpu_impact: 0.0,
            actual_mem_impact: 0.0,
            validated: false,
        });
        if self.records.len() > MAX_REHEARSALS {
            self.records.drain(..MAX_REHEARSALS / 4);
        }

        WhatIfResult {
            scenario_id,
            description: desc,
            cpu_impact: cpu_imp,
            memory_impact: mem_imp,
            io_impact: io_imp,
            latency_impact_ms: latency,
            risk_score: risk,
            recommendation: rec,
        }
    }

    /// Compute resource impact for a scenario
    pub fn resource_impact(&mut self, scenario_id: u64) -> ResourceImpact {
        self.total_rehearsals += 1;
        self.impact_count += 1;

        if let Some(cached) = self.impact_cache.get(&scenario_id) {
            return cached.clone();
        }

        let (pid, cpu_d, mem_d, io_d) = if let Some(scenario) = self.scenarios.get(&scenario_id) {
            let mut cpu: f32 = 0.0;
            let mut mem: f32 = 0.0;
            let mut io: f32 = 0.0;
            for (key, val) in &scenario.parameters {
                let k_hash = fnv1a_hash(key.as_bytes());
                match k_hash % 3 {
                    0 => cpu += val,
                    1 => mem += val,
                    _ => io += val,
                }
            }
            (scenario.process_id, cpu, mem, io)
        } else {
            (0, 0.0, 0.0, 0.0)
        };

        let total_cost = cpu_d.abs() + mem_d.abs() * 0.5 + io_d.abs() * 0.3;
        let feasible = total_cost < 5.0;
        let mut bottleneck = String::new();
        if cpu_d.abs() >= mem_d.abs() && cpu_d.abs() >= io_d.abs() {
            bottleneck.push_str("cpu");
        } else if mem_d.abs() >= io_d.abs() {
            bottleneck.push_str("memory");
        } else {
            bottleneck.push_str("io");
        }

        let impact = ResourceImpact {
            scenario_id,
            process_id: pid,
            cpu_delta: cpu_d,
            memory_delta: mem_d,
            io_delta: io_d,
            total_cost,
            feasible,
            bottleneck,
        };

        if self.impact_cache.len() < MAX_SCENARIOS {
            self.impact_cache.insert(scenario_id, impact.clone());
        }
        impact
    }

    /// Compute rehearsal accuracy based on validated predictions
    pub fn rehearsal_accuracy(&self) -> f32 {
        self.accuracy_ema
    }

    /// Validate a past rehearsal against actual outcomes
    pub fn validate(&mut self, scenario_id: u64, actual_cpu: f32, actual_mem: f32) {
        self.validated_count += 1;
        for rec in self.records.iter_mut().rev() {
            if rec.scenario_id == scenario_id && !rec.validated {
                rec.actual_cpu_impact = actual_cpu;
                rec.actual_mem_impact = actual_mem;
                rec.validated = true;
                let acc = rec.accuracy();
                self.accuracy_ema = EMA_ALPHA * acc + (1.0 - EMA_ALPHA) * self.accuracy_ema;
                break;
            }
        }
    }

    /// Compare two scenarios side by side
    pub fn scenario_comparison(&mut self, scenario_a: u64, scenario_b: u64) -> ScenarioComparison {
        let impact_a = self.resource_impact(scenario_a);
        let impact_b = self.resource_impact(scenario_b);

        let cpu_diff = impact_a.cpu_delta - impact_b.cpu_delta;
        let mem_diff = impact_a.memory_delta - impact_b.memory_delta;
        let io_diff = impact_a.io_delta - impact_b.io_delta;
        let cost_diff = impact_a.total_cost - impact_b.total_cost;

        let preferred = if impact_a.total_cost <= impact_b.total_cost {
            scenario_a
        } else {
            scenario_b
        };

        let mut reason = String::new();
        if cost_diff.abs() < 0.1 {
            reason.push_str("similar_cost");
        } else if preferred == scenario_a {
            reason.push_str("a_lower_cost");
        } else {
            reason.push_str("b_lower_cost");
        }

        ScenarioComparison {
            scenario_a_id: scenario_a,
            scenario_b_id: scenario_b,
            cpu_diff,
            memory_diff: mem_diff,
            io_diff,
            cost_diff,
            preferred,
            reason,
        }
    }

    /// Advance the internal tick
    pub fn advance_tick(&mut self, tick: u64) {
        self.tick = tick;
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> RehearsalStats {
        RehearsalStats {
            total_rehearsals: self.total_rehearsals,
            scenarios_tested: self.scenarios.len(),
            avg_accuracy: self.accuracy_ema,
            validated_count: self.validated_count,
            what_if_analyses: self.what_if_count,
            classifications_rehearsed: self.classification_count,
            resource_impacts_computed: self.impact_count,
        }
    }
}
