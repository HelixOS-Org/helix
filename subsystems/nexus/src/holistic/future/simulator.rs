// SPDX-License-Identifier: GPL-2.0
//! # Holistic Simulator
//!
//! Full system simulation engine for the NEXUS prediction framework.
//! Simulates the **entire OS state** evolution — CPU scheduling, memory
//! allocation, I/O flow, network traffic, and process lifecycle — all at once.
//!
//! This is NOT a per-subsystem model. It captures cross-domain feedback loops:
//! memory pressure → OOM kills → scheduler churn → I/O spikes → network
//! retransmits — the cascading effects that single-domain simulators miss.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_SCENARIOS: usize = 128;
const MAX_STEPS_PER_SIM: usize = 1024;
const MAX_BOTTLENECK_ENTRIES: usize = 64;
const MAX_DIVERGENCE_RECORDS: usize = 128;
const EMA_ALPHA: f32 = 0.10;
const FIDELITY_THRESHOLD: f32 = 0.80;
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
// RESOURCE DOMAIN
// ============================================================================

/// System resource domain being simulated
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceDomain {
    Cpu,
    Memory,
    IoBlock,
    IoNetwork,
    ProcessLifecycle,
    ThermalPower,
}

/// Scenario classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioKind {
    Baseline,
    HighLoad,
    MemoryPressure,
    IoBurst,
    NetworkStorm,
    CascadeFailure,
    GracefulDegradation,
}

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Snapshot of the entire simulated system state at one step
#[derive(Debug, Clone)]
pub struct SimulationStep {
    pub step: u64,
    pub cpu_util_pct: f32,
    pub mem_used_pct: f32,
    pub io_queue_depth: u32,
    pub net_throughput_pct: f32,
    pub active_processes: u32,
    pub thermal_headroom: f32,
    pub stability_score: f32,
}

/// A complete simulation scenario result
#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub id: u64,
    pub kind: ScenarioKind,
    pub steps: Vec<SimulationStep>,
    pub final_stability: f32,
    pub bottleneck_domain: ResourceDomain,
    pub peak_resource_usage: f32,
    pub cascade_triggered: bool,
    pub tick: u64,
}

/// Divergence between two scenarios at a given step
#[derive(Debug, Clone)]
pub struct DivergenceRecord {
    pub scenario_a: u64,
    pub scenario_b: u64,
    pub step: u64,
    pub domain: ResourceDomain,
    pub magnitude: f32,
    pub description: String,
}

/// Predicted resource bottleneck
#[derive(Debug, Clone)]
pub struct BottleneckPrediction {
    pub id: u64,
    pub domain: ResourceDomain,
    pub estimated_step: u64,
    pub severity: f32,
    pub cascading_to: Vec<ResourceDomain>,
    pub mitigation_benefit: f32,
}

/// The optimal path through the simulation space
#[derive(Debug, Clone)]
pub struct OptimalPath {
    pub steps: Vec<SimulationStep>,
    pub total_cost: f32,
    pub total_stability: f32,
    pub interventions: Vec<String>,
}

/// Fidelity report comparing simulation to actual outcomes
#[derive(Debug, Clone)]
pub struct FidelityReport {
    pub scenario_id: u64,
    pub cpu_error: f32,
    pub mem_error: f32,
    pub io_error: f32,
    pub net_error: f32,
    pub overall_fidelity: f32,
    pub sample_count: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate simulation statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct SimulatorStats {
    pub total_simulations: u64,
    pub total_scenarios: u64,
    pub avg_fidelity: f32,
    pub avg_stability: f32,
    pub bottleneck_count: u64,
    pub cascade_count: u64,
    pub optimal_cost_ema: f32,
    pub divergence_events: u64,
}

// ============================================================================
// HOLISTIC SIMULATOR
// ============================================================================

/// Full system simulation engine. Models the entire OS state evolution,
/// captures cross-domain feedback, and identifies bottlenecks and cascades.
#[derive(Debug)]
pub struct HolisticSimulator {
    scenarios: BTreeMap<u64, ScenarioResult>,
    divergences: BTreeMap<u64, DivergenceRecord>,
    bottlenecks: BTreeMap<u64, BottleneckPrediction>,
    fidelity_history: BTreeMap<u64, FidelityReport>,
    total_simulations: u64,
    total_scenarios: u64,
    cascade_count: u64,
    tick: u64,
    rng_state: u64,
    fidelity_ema: f32,
    stability_ema: f32,
    cost_ema: f32,
}

impl HolisticSimulator {
    pub fn new() -> Self {
        Self {
            scenarios: BTreeMap::new(),
            divergences: BTreeMap::new(),
            bottlenecks: BTreeMap::new(),
            fidelity_history: BTreeMap::new(),
            total_simulations: 0,
            total_scenarios: 0,
            cascade_count: 0,
            tick: 0,
            rng_state: 0x51BA_1A70_AE06_1BE0,
            fidelity_ema: 0.5,
            stability_ema: 0.7,
            cost_ema: 0.0,
        }
    }

    /// Simulate the full system for a given number of steps
    pub fn simulate_full_system(
        &mut self,
        kind: ScenarioKind,
        num_steps: u64,
        initial_cpu: f32,
        initial_mem: f32,
    ) -> ScenarioResult {
        self.tick += 1;
        self.total_simulations += 1;
        self.total_scenarios += 1;

        let mut steps = Vec::new();
        let mut cpu = initial_cpu;
        let mut mem = initial_mem;
        let mut io_q = 4_u32;
        let mut net = 50.0_f32;
        let mut procs = 100_u32;
        let mut thermal = 0.8_f32;
        let mut cascade = false;

        let stress_factor = match kind {
            ScenarioKind::Baseline => 0.0_f32,
            ScenarioKind::HighLoad => 0.4,
            ScenarioKind::MemoryPressure => 0.3,
            ScenarioKind::IoBurst => 0.5,
            ScenarioKind::NetworkStorm => 0.45,
            ScenarioKind::CascadeFailure => 0.6,
            ScenarioKind::GracefulDegradation => 0.2,
        };

        let capped_steps = (num_steps as usize).min(MAX_STEPS_PER_SIM);
        for s in 0..capped_steps as u64 {
            let noise = (xorshift64(&mut self.rng_state) % 100) as f32 / 1000.0 - 0.05;

            cpu = (cpu + stress_factor * 2.0 + noise * 10.0).clamp(0.0, 100.0);
            mem = (mem + stress_factor * 0.5 * 0.01 + noise * 0.02).clamp(0.0, 1.0);

            if mem > 0.85 {
                procs = procs.saturating_sub(1);
                cpu = (cpu + 5.0).clamp(0.0, 100.0);
                io_q = io_q.saturating_add(2);
            }

            if cpu > 90.0 {
                thermal = (thermal - 0.02).clamp(0.0, 1.0);
                if thermal < 0.2 {
                    cpu = (cpu - 10.0).clamp(0.0, 100.0);
                }
            }

            if io_q > 32 {
                net = (net - 5.0).clamp(0.0, 100.0);
                cascade = true;
            }

            let stability = (thermal * 0.3
                + (1.0 - cpu / 100.0) * 0.3
                + (1.0 - mem) * 0.2
                + (1.0 - io_q as f32 / 64.0) * 0.2)
                .clamp(0.0, 1.0);

            steps.push(SimulationStep {
                step: s,
                cpu_util_pct: cpu,
                mem_used_pct: mem,
                io_queue_depth: io_q,
                net_throughput_pct: net,
                active_processes: procs,
                thermal_headroom: thermal,
                stability_score: stability,
            });
        }

        if cascade {
            self.cascade_count += 1;
        }

        let final_stability = steps.last().map(|s| s.stability_score).unwrap_or(0.5);
        let peak_usage = steps
            .iter()
            .map(|s| s.cpu_util_pct.max(s.mem_used_pct * 100.0))
            .fold(0.0_f32, f32::max);

        let bottleneck = if mem > 0.85 {
            ResourceDomain::Memory
        } else if cpu > 90.0 {
            ResourceDomain::Cpu
        } else if io_q > 32 {
            ResourceDomain::IoBlock
        } else {
            ResourceDomain::Cpu
        };

        let id = fnv1a_hash(format!("{:?}-{}", kind, self.tick).as_bytes())
            ^ xorshift64(&mut self.rng_state);

        self.stability_ema = EMA_ALPHA * final_stability + (1.0 - EMA_ALPHA) * self.stability_ema;

        let result = ScenarioResult {
            id,
            kind,
            steps,
            final_stability,
            bottleneck_domain: bottleneck,
            peak_resource_usage: peak_usage,
            cascade_triggered: cascade,
            tick: self.tick,
        };

        self.scenarios.insert(id, result.clone());
        if self.scenarios.len() > MAX_SCENARIOS {
            if let Some((&oldest, _)) = self.scenarios.iter().next() {
                self.scenarios.remove(&oldest);
            }
        }

        result
    }

    /// Run multiple scenarios in parallel and return all results
    pub fn parallel_scenarios(
        &mut self,
        kinds: &[ScenarioKind],
        steps: u64,
        cpu: f32,
        mem: f32,
    ) -> Vec<ScenarioResult> {
        let mut results = Vec::new();
        for &kind in kinds {
            results.push(self.simulate_full_system(kind, steps, cpu, mem));
        }
        results
    }

    /// Analyze divergence between two scenarios
    pub fn divergence_analysis(&mut self, id_a: u64, id_b: u64) -> Vec<DivergenceRecord> {
        let mut records = Vec::new();

        let steps_a = self.scenarios.get(&id_a).map(|s| s.steps.clone());
        let steps_b = self.scenarios.get(&id_b).map(|s| s.steps.clone());

        if let (Some(sa), Some(sb)) = (steps_a, steps_b) {
            let len = sa.len().min(sb.len());
            for i in 0..len {
                let cpu_diff = (sa[i].cpu_util_pct - sb[i].cpu_util_pct).abs();
                let mem_diff = (sa[i].mem_used_pct - sb[i].mem_used_pct).abs();

                if cpu_diff > 10.0 {
                    let rec_id = fnv1a_hash(format!("div-cpu-{}", i).as_bytes());
                    records.push(DivergenceRecord {
                        scenario_a: id_a,
                        scenario_b: id_b,
                        step: i as u64,
                        domain: ResourceDomain::Cpu,
                        magnitude: cpu_diff,
                        description: String::from("CPU utilization divergence"),
                    });
                    self.divergences.insert(rec_id, records.last().unwrap().clone());
                }

                if mem_diff > 0.1 {
                    let rec_id = fnv1a_hash(format!("div-mem-{}", i).as_bytes());
                    records.push(DivergenceRecord {
                        scenario_a: id_a,
                        scenario_b: id_b,
                        step: i as u64,
                        domain: ResourceDomain::Memory,
                        magnitude: mem_diff,
                        description: String::from("Memory pressure divergence"),
                    });
                    self.divergences.insert(rec_id, records.last().unwrap().clone());
                }
            }
        }

        while self.divergences.len() > MAX_DIVERGENCE_RECORDS {
            if let Some((&oldest, _)) = self.divergences.iter().next() {
                self.divergences.remove(&oldest);
            }
        }

        records
    }

    /// Compute the optimal path across all simulated scenarios
    pub fn optimal_path(&self) -> OptimalPath {
        let mut best_scenario: Option<&ScenarioResult> = None;
        let mut best_stability = -1.0_f32;

        for scenario in self.scenarios.values() {
            if scenario.final_stability > best_stability {
                best_stability = scenario.final_stability;
                best_scenario = Some(scenario);
            }
        }

        match best_scenario {
            Some(sc) => {
                let mut interventions = Vec::new();
                if sc.cascade_triggered {
                    interventions.push(String::from("throttle_io_before_cascade"));
                }
                if sc.peak_resource_usage > 90.0 {
                    interventions.push(String::from("preemptive_load_shed"));
                }
                OptimalPath {
                    steps: sc.steps.clone(),
                    total_cost: 1.0 - sc.final_stability,
                    total_stability: sc.final_stability,
                    interventions,
                }
            }
            None => OptimalPath {
                steps: Vec::new(),
                total_cost: 1.0,
                total_stability: 0.0,
                interventions: Vec::new(),
            },
        }
    }

    /// Measure simulation fidelity against actual observations
    pub fn simulation_fidelity(
        &mut self,
        scenario_id: u64,
        actual_cpu: f32,
        actual_mem: f32,
        actual_io: f32,
        actual_net: f32,
    ) -> FidelityReport {
        let (pred_cpu, pred_mem, pred_io, pred_net) = self
            .scenarios
            .get(&scenario_id)
            .and_then(|s| s.steps.last())
            .map(|step| {
                (
                    step.cpu_util_pct,
                    step.mem_used_pct,
                    step.io_queue_depth as f32,
                    step.net_throughput_pct,
                )
            })
            .unwrap_or((50.0, 0.5, 4.0, 50.0));

        let cpu_err = (pred_cpu - actual_cpu).abs() / 100.0;
        let mem_err = (pred_mem - actual_mem).abs();
        let io_err = (pred_io - actual_io).abs() / 64.0;
        let net_err = (pred_net - actual_net).abs() / 100.0;
        let fidelity = 1.0 - (cpu_err + mem_err + io_err + net_err) / 4.0;

        self.fidelity_ema = EMA_ALPHA * fidelity.clamp(0.0, 1.0)
            + (1.0 - EMA_ALPHA) * self.fidelity_ema;

        let report = FidelityReport {
            scenario_id,
            cpu_error: cpu_err,
            mem_error: mem_err,
            io_error: io_err,
            net_error: net_err,
            overall_fidelity: fidelity.clamp(0.0, 1.0),
            sample_count: self.total_simulations,
        };

        self.fidelity_history.insert(scenario_id, report.clone());
        report
    }

    /// Predict resource bottlenecks across all recent scenarios
    pub fn resource_bottleneck_prediction(&mut self) -> Vec<BottleneckPrediction> {
        let mut predictions = Vec::new();
        let mut domain_severity: BTreeMap<u8, (f32, u32)> = BTreeMap::new();

        for scenario in self.scenarios.values() {
            let domain_key = scenario.bottleneck_domain as u8;
            let entry = domain_severity.entry(domain_key).or_insert((0.0, 0));
            entry.0 += 1.0 - scenario.final_stability;
            entry.1 += 1;
        }

        for (&domain_key, &(total_sev, count)) in &domain_severity {
            let avg_sev = total_sev / count.max(1) as f32;
            let domain = match domain_key {
                0 => ResourceDomain::Cpu,
                1 => ResourceDomain::Memory,
                2 => ResourceDomain::IoBlock,
                3 => ResourceDomain::IoNetwork,
                4 => ResourceDomain::ProcessLifecycle,
                _ => ResourceDomain::ThermalPower,
            };

            let mut cascading = Vec::new();
            if domain == ResourceDomain::Memory {
                cascading.push(ResourceDomain::Cpu);
                cascading.push(ResourceDomain::IoBlock);
            } else if domain == ResourceDomain::Cpu {
                cascading.push(ResourceDomain::ThermalPower);
            }

            let id = fnv1a_hash(format!("bneck-{}", domain_key).as_bytes());
            predictions.push(BottleneckPrediction {
                id,
                domain,
                estimated_step: (self.tick + 10).min(u64::MAX),
                severity: avg_sev.clamp(0.0, 1.0),
                cascading_to: cascading,
                mitigation_benefit: (avg_sev * 0.6).clamp(0.0, 1.0),
            });

            self.bottlenecks.insert(id, predictions.last().unwrap().clone());
        }

        while self.bottlenecks.len() > MAX_BOTTLENECK_ENTRIES {
            if let Some((&oldest, _)) = self.bottlenecks.iter().next() {
                self.bottlenecks.remove(&oldest);
            }
        }

        predictions
    }

    /// Gather aggregate statistics
    pub fn stats(&self) -> SimulatorStats {
        SimulatorStats {
            total_simulations: self.total_simulations,
            total_scenarios: self.total_scenarios,
            avg_fidelity: self.fidelity_ema,
            avg_stability: self.stability_ema,
            bottleneck_count: self.bottlenecks.len() as u64,
            cascade_count: self.cascade_count,
            optimal_cost_ema: self.cost_ema,
            divergence_events: self.divergences.len() as u64,
        }
    }
}
