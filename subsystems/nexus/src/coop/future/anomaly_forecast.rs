// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Anomaly Forecast
//!
//! Predict cooperation anomalies before they manifest. Detects deadlocks,
//! livelocks, priority inversions, and starvation conditions by analyzing
//! cooperation patterns, resource holding graphs, and scheduling signals.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// FNV-1a hash for deterministic key hashing in no_std.
fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Xorshift64 PRNG for lightweight stochastic perturbation.
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Exponential moving average update.
fn ema_update(current: u64, new_sample: u64, alpha_num: u64, alpha_den: u64) -> u64 {
    let weighted_old = current.saturating_mul(alpha_den.saturating_sub(alpha_num));
    let weighted_new = new_sample.saturating_mul(alpha_num);
    weighted_old.saturating_add(weighted_new) / alpha_den.max(1)
}

/// Type of cooperation anomaly.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnomalyType {
    Deadlock,
    Livelock,
    PriorityInversion,
    Starvation,
    CascadeFailure,
    FairnessCollapse,
}

/// A predicted deadlock event.
#[derive(Clone, Debug)]
pub struct DeadlockForecast {
    pub forecast_id: u64,
    pub involved_processes: Vec<u64>,
    pub involved_resources: Vec<u64>,
    pub cycle_length: u32,
    pub probability: u64,
    pub ticks_until_likely: u64,
    pub severity: u64,
    pub recommended_preemption: u64,
}

/// A livelock precursor detection.
#[derive(Clone, Debug)]
pub struct LivelockPrecursor {
    pub precursor_id: u64,
    pub oscillating_processes: Vec<u64>,
    pub oscillation_frequency: u64,
    pub energy_waste: u64,
    pub probability: u64,
    pub detection_confidence: u64,
}

/// A priority inversion warning.
#[derive(Clone, Debug)]
pub struct InversionWarning {
    pub warning_id: u64,
    pub high_priority_process: u64,
    pub blocking_process: u64,
    pub resource_id: u64,
    pub inversion_depth: u32,
    pub estimated_delay: u64,
    pub severity: u64,
}

/// Starvation prediction.
#[derive(Clone, Debug)]
pub struct StarvationPrediction {
    pub process_id: u64,
    pub resource_id: u64,
    pub wait_ticks: u64,
    pub projected_wait: u64,
    pub starvation_probability: u64,
    pub fairness_deficit: u64,
}

/// An early intervention recommendation.
#[derive(Clone, Debug)]
pub struct EarlyIntervention {
    pub intervention_id: u64,
    pub anomaly_type: AnomalyType,
    pub target_process: u64,
    pub target_resource: u64,
    pub action_hash: u64,
    pub urgency: u64,
    pub estimated_benefit: u64,
}

/// An anomaly pattern in the library.
#[derive(Clone, Debug)]
pub struct AnomalyPattern {
    pub pattern_id: u64,
    pub anomaly_type: AnomalyType,
    pub signature_hash: u64,
    pub occurrence_count: u64,
    pub avg_severity: u64,
    pub avg_duration: u64,
    pub known_mitigations: Vec<u64>,
}

/// Rolling statistics for the anomaly forecast engine.
#[derive(Clone, Debug)]
pub struct AnomalyForecastStats {
    pub deadlocks_predicted: u64,
    pub livelocks_detected: u64,
    pub inversions_warned: u64,
    pub starvations_predicted: u64,
    pub interventions_issued: u64,
    pub patterns_catalogued: u64,
    pub avg_severity: u64,
    pub prediction_accuracy_ema: u64,
}

impl AnomalyForecastStats {
    pub fn new() -> Self {
        Self {
            deadlocks_predicted: 0,
            livelocks_detected: 0,
            inversions_warned: 0,
            starvations_predicted: 0,
            interventions_issued: 0,
            patterns_catalogued: 0,
            avg_severity: 0,
            prediction_accuracy_ema: 500,
        }
    }
}

/// Internal resource holding state for deadlock detection.
#[derive(Clone, Debug)]
struct ResourceHolding {
    resource_id: u64,
    holder: u64,
    waiters: Vec<u64>,
    hold_duration: u64,
    contention_ema: u64,
}

/// Internal process wait state for livelock detection.
#[derive(Clone, Debug)]
struct ProcessWaitState {
    process_id: u64,
    state_changes: Vec<u64>,
    ema_change_interval: u64,
    resources_attempted: Vec<u64>,
    wait_cycles: u64,
}

/// Internal priority record for inversion detection.
#[derive(Clone, Debug)]
struct PriorityRecord {
    process_id: u64,
    priority: u64,
    held_resources: Vec<u64>,
    waiting_for: Vec<u64>,
}

/// Internal starvation tracker.
#[derive(Clone, Debug)]
struct StarvationTracker {
    process_id: u64,
    resource_id: u64,
    wait_start: u64,
    denial_count: u64,
    ema_wait: u64,
}

/// Cooperation anomaly forecast engine.
pub struct CoopAnomalyForecast {
    holdings: BTreeMap<u64, ResourceHolding>,
    wait_states: BTreeMap<u64, ProcessWaitState>,
    priorities: BTreeMap<u64, PriorityRecord>,
    starvation_trackers: BTreeMap<u64, StarvationTracker>,
    pattern_library: BTreeMap<u64, AnomalyPattern>,
    intervention_history: Vec<EarlyIntervention>,
    stats: AnomalyForecastStats,
    rng_state: u64,
    current_tick: u64,
    deadlock_threshold: u64,
    livelock_frequency_threshold: u64,
    starvation_tick_threshold: u64,
}

impl CoopAnomalyForecast {
    /// Create a new anomaly forecast engine.
    pub fn new(seed: u64) -> Self {
        Self {
            holdings: BTreeMap::new(),
            wait_states: BTreeMap::new(),
            priorities: BTreeMap::new(),
            starvation_trackers: BTreeMap::new(),
            pattern_library: BTreeMap::new(),
            intervention_history: Vec::new(),
            stats: AnomalyForecastStats::new(),
            rng_state: seed ^ 0xA707_A1Y0_F0CA_5700,
            current_tick: 0,
            deadlock_threshold: 3,
            livelock_frequency_threshold: 50,
            starvation_tick_threshold: 200,
        }
    }

    /// Record resource acquisition.
    pub fn record_acquire(&mut self, process_id: u64, resource_id: u64) {
        let holding = self.holdings.entry(resource_id).or_insert_with(|| ResourceHolding {
            resource_id,
            holder: 0,
            waiters: Vec::new(),
            hold_duration: 0,
            contention_ema: 0,
        });
        holding.holder = process_id;
        holding.hold_duration = 0;

        let priority = self.priorities.entry(process_id).or_insert_with(|| PriorityRecord {
            process_id,
            priority: 500,
            held_resources: Vec::new(),
            waiting_for: Vec::new(),
        });
        if !priority.held_resources.contains(&resource_id) {
            priority.held_resources.push(resource_id);
        }
        priority.waiting_for.retain(|&r| r != resource_id);
    }

    /// Record a wait request for a resource.
    pub fn record_wait(&mut self, process_id: u64, resource_id: u64) {
        if let Some(holding) = self.holdings.get_mut(&resource_id) {
            if !holding.waiters.contains(&process_id) {
                holding.waiters.push(process_id);
            }
            holding.contention_ema = ema_update(
                holding.contention_ema,
                holding.waiters.len() as u64 * 100,
                200,
                1000,
            );
        }

        let ws = self.wait_states.entry(process_id).or_insert_with(|| ProcessWaitState {
            process_id,
            state_changes: Vec::new(),
            ema_change_interval: 100,
            resources_attempted: Vec::new(),
            wait_cycles: 0,
        });
        ws.state_changes.push(self.current_tick);
        ws.wait_cycles = ws.wait_cycles.saturating_add(1);
        if !ws.resources_attempted.contains(&resource_id) {
            ws.resources_attempted.push(resource_id);
        }
        if ws.state_changes.len() > 64 {
            ws.state_changes.remove(0);
        }

        let priority = self.priorities.entry(process_id).or_insert_with(|| PriorityRecord {
            process_id,
            priority: 500,
            held_resources: Vec::new(),
            waiting_for: Vec::new(),
        });
        if !priority.waiting_for.contains(&resource_id) {
            priority.waiting_for.push(resource_id);
        }

        let tracker_key = fnv1a_hash(&[
            process_id.to_le_bytes().as_slice(),
            resource_id.to_le_bytes().as_slice(),
        ].concat());
        self.starvation_trackers.entry(tracker_key).or_insert_with(|| StarvationTracker {
            process_id,
            resource_id,
            wait_start: self.current_tick,
            denial_count: 0,
            ema_wait: 0,
        }).denial_count += 1;
    }

    /// Record resource release.
    pub fn record_release(&mut self, process_id: u64, resource_id: u64) {
        if let Some(holding) = self.holdings.get_mut(&resource_id) {
            if holding.holder == process_id {
                holding.holder = 0;
                holding.hold_duration = 0;
            }
            holding.waiters.retain(|&w| w != process_id);
        }

        if let Some(priority) = self.priorities.get_mut(&process_id) {
            priority.held_resources.retain(|&r| r != resource_id);
        }
    }

    /// Set priority for a process.
    pub fn set_priority(&mut self, process_id: u64, priority: u64) {
        let rec = self.priorities.entry(process_id).or_insert_with(|| PriorityRecord {
            process_id,
            priority,
            held_resources: Vec::new(),
            waiting_for: Vec::new(),
        });
        rec.priority = priority;
    }

    /// Forecast potential deadlocks.
    pub fn forecast_deadlock(&mut self) -> Vec<DeadlockForecast> {
        let mut forecasts: Vec<DeadlockForecast> = Vec::new();

        let process_ids: Vec<u64> = self.priorities.keys().copied().collect();

        for &pid in &process_ids {
            let cycle = self.detect_wait_cycle(pid);
            if cycle.len() >= 2 {
                let resources: Vec<u64> = cycle.iter()
                    .filter_map(|&p| {
                        self.priorities.get(&p)
                            .and_then(|pr| pr.waiting_for.first().copied())
                    })
                    .collect();

                let severity = (cycle.len() as u64).saturating_mul(200).min(1000);

                let avg_contention: u64 = resources.iter()
                    .filter_map(|r| self.holdings.get(r).map(|h| h.contention_ema))
                    .sum::<u64>()
                    / resources.len().max(1) as u64;

                let probability = avg_contention.saturating_mul(severity) / 1000;

                let noise = xorshift64(&mut self.rng_state) % 20;
                let ticks_until = 100u64.saturating_sub(probability / 10).saturating_add(noise);

                let forecast_id = fnv1a_hash(
                    &cycle.iter().flat_map(|p| p.to_le_bytes()).collect::<Vec<u8>>(),
                );

                let preempt = cycle.last().copied().unwrap_or(0);

                forecasts.push(DeadlockForecast {
                    forecast_id,
                    involved_processes: cycle,
                    involved_resources: resources,
                    cycle_length: forecasts.len() as u32 + 2,
                    probability: probability.min(1000),
                    ticks_until_likely: ticks_until,
                    severity,
                    recommended_preemption: preempt,
                });

                self.catalog_pattern(AnomalyType::Deadlock, forecast_id, severity);
            }
        }

        self.stats.deadlocks_predicted = self.stats.deadlocks_predicted
            .saturating_add(forecasts.len() as u64);
        forecasts
    }

    /// Detect livelock precursors.
    pub fn livelock_precursor(&mut self) -> Vec<LivelockPrecursor> {
        let mut precursors: Vec<LivelockPrecursor> = Vec::new();

        for (_, ws) in &self.wait_states {
            if ws.state_changes.len() < 4 {
                continue;
            }

            let n = ws.state_changes.len();
            let mut intervals: Vec<u64> = Vec::new();
            for i in 1..n {
                intervals.push(ws.state_changes[i].saturating_sub(ws.state_changes[i - 1]));
            }

            let avg_interval = intervals.iter().sum::<u64>() / intervals.len().max(1) as u64;

            let variance: u64 = intervals.iter()
                .map(|&i| {
                    let d = if i > avg_interval { i - avg_interval } else { avg_interval - i };
                    d.saturating_mul(d)
                })
                .sum::<u64>()
                / intervals.len().max(1) as u64;

            let is_periodic = variance < self.livelock_frequency_threshold
                && avg_interval < 50
                && ws.wait_cycles > 5;

            if is_periodic {
                let frequency = if avg_interval > 0 {
                    1000 / avg_interval
                } else {
                    1000
                };

                let energy_waste = frequency.saturating_mul(ws.wait_cycles);
                let probability = (ws.wait_cycles.min(20).saturating_mul(50)).min(1000);

                let pid = fnv1a_hash(&ws.process_id.to_le_bytes());

                precursors.push(LivelockPrecursor {
                    precursor_id: pid,
                    oscillating_processes: alloc::vec![ws.process_id],
                    oscillation_frequency: frequency,
                    energy_waste,
                    probability,
                    detection_confidence: 700,
                });

                self.catalog_pattern(AnomalyType::Livelock, pid, probability);
            }
        }

        self.stats.livelocks_detected = self.stats.livelocks_detected
            .saturating_add(precursors.len() as u64);
        precursors
    }

    /// Issue priority inversion warnings.
    pub fn inversion_warning(&mut self) -> Vec<InversionWarning> {
        let mut warnings: Vec<InversionWarning> = Vec::new();

        let prio_list: Vec<(u64, u64, Vec<u64>)> = self.priorities.values()
            .map(|p| (p.process_id, p.priority, p.waiting_for.clone()))
            .collect();

        for (high_pid, high_prio, waiting_for) in &prio_list {
            for &resource_id in waiting_for {
                if let Some(holding) = self.holdings.get(&resource_id) {
                    let blocker = holding.holder;
                    if blocker == 0 || blocker == *high_pid {
                        continue;
                    }

                    let blocker_prio = self.priorities.get(&blocker)
                        .map(|p| p.priority)
                        .unwrap_or(500);

                    if *high_prio > blocker_prio {
                        let depth = self.compute_inversion_depth(blocker);
                        let delay = holding.hold_duration.saturating_mul(depth as u64 + 1);
                        let severity = (*high_prio - blocker_prio)
                            .saturating_mul(delay) / 1000;

                        let wid = fnv1a_hash(&[
                            high_pid.to_le_bytes().as_slice(),
                            blocker.to_le_bytes().as_slice(),
                            resource_id.to_le_bytes().as_slice(),
                        ].concat());

                        warnings.push(InversionWarning {
                            warning_id: wid,
                            high_priority_process: *high_pid,
                            blocking_process: blocker,
                            resource_id,
                            inversion_depth: depth,
                            estimated_delay: delay,
                            severity: severity.min(1000),
                        });

                        self.catalog_pattern(AnomalyType::PriorityInversion, wid, severity.min(1000));
                    }
                }
            }
        }

        self.stats.inversions_warned = self.stats.inversions_warned
            .saturating_add(warnings.len() as u64);
        warnings
    }

    /// Predict starvation conditions.
    pub fn starvation_prediction(&mut self) -> Vec<StarvationPrediction> {
        let mut predictions: Vec<StarvationPrediction> = Vec::new();

        let trackers: Vec<(u64, u64, u64, u64, u64)> = self.starvation_trackers.values()
            .map(|t| (t.process_id, t.resource_id, t.wait_start, t.denial_count, t.ema_wait))
            .collect();

        for (pid, rid, start, denials, ema_wait) in trackers {
            let wait_ticks = self.current_tick.saturating_sub(start);
            let projected = ema_wait.saturating_add(wait_ticks / 2);

            let probability = if wait_ticks > self.starvation_tick_threshold {
                (wait_ticks.saturating_sub(self.starvation_tick_threshold))
                    .saturating_mul(10)
                    .min(900)
            } else {
                denials.saturating_mul(50).min(500)
            };

            let fairness_deficit = denials.saturating_mul(100)
                .saturating_add(wait_ticks / 10);

            if probability > 200 {
                predictions.push(StarvationPrediction {
                    process_id: pid,
                    resource_id: rid,
                    wait_ticks,
                    projected_wait: projected,
                    starvation_probability: probability.min(1000),
                    fairness_deficit: fairness_deficit.min(1000),
                });

                let pat_id = fnv1a_hash(&[
                    pid.to_le_bytes().as_slice(),
                    rid.to_le_bytes().as_slice(),
                ].concat());
                self.catalog_pattern(AnomalyType::Starvation, pat_id, probability.min(1000));
            }
        }

        self.stats.starvations_predicted = self.stats.starvations_predicted
            .saturating_add(predictions.len() as u64);
        predictions
    }

    /// Issue early intervention recommendations.
    pub fn early_intervention(&mut self) -> Vec<EarlyIntervention> {
        let mut interventions: Vec<EarlyIntervention> = Vec::new();

        let deadlocks = self.forecast_deadlock();
        for dl in &deadlocks {
            if dl.probability > 500 {
                let iid = fnv1a_hash(&dl.forecast_id.to_le_bytes());
                interventions.push(EarlyIntervention {
                    intervention_id: iid,
                    anomaly_type: AnomalyType::Deadlock,
                    target_process: dl.recommended_preemption,
                    target_resource: dl.involved_resources.first().copied().unwrap_or(0),
                    action_hash: fnv1a_hash(b"preempt_oldest"),
                    urgency: dl.severity,
                    estimated_benefit: dl.probability,
                });
            }
        }

        let starvations = self.starvation_prediction();
        for st in &starvations {
            if st.starvation_probability > 600 {
                let iid = fnv1a_hash(&[
                    st.process_id.to_le_bytes().as_slice(),
                    b"boost",
                ].concat());
                interventions.push(EarlyIntervention {
                    intervention_id: iid,
                    anomaly_type: AnomalyType::Starvation,
                    target_process: st.process_id,
                    target_resource: st.resource_id,
                    action_hash: fnv1a_hash(b"priority_boost"),
                    urgency: st.starvation_probability,
                    estimated_benefit: st.fairness_deficit,
                });
            }
        }

        self.stats.interventions_issued = self.stats.interventions_issued
            .saturating_add(interventions.len() as u64);

        for i in &interventions {
            self.intervention_history.push(i.clone());
        }
        if self.intervention_history.len() > 256 {
            self.intervention_history.drain(0..self.intervention_history.len() - 256);
        }

        interventions
    }

    /// Return the anomaly pattern library.
    pub fn anomaly_library(&self) -> Vec<&AnomalyPattern> {
        self.pattern_library.values().collect()
    }

    /// Advance the internal tick and update durations.
    pub fn tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
        for holding in self.holdings.values_mut() {
            if holding.holder != 0 {
                holding.hold_duration = holding.hold_duration.saturating_add(1);
            }
        }
        for tracker in self.starvation_trackers.values_mut() {
            let wait = self.current_tick.saturating_sub(tracker.wait_start);
            tracker.ema_wait = ema_update(tracker.ema_wait, wait, 100, 1000);
        }
    }

    /// Retrieve current statistics.
    pub fn stats(&self) -> &AnomalyForecastStats {
        &self.stats
    }

    // ── Private helpers ──────────────────────────────────────────────

    fn detect_wait_cycle(&self, start: u64) -> Vec<u64> {
        let mut path: Vec<u64> = Vec::new();
        let mut visited: BTreeMap<u64, bool> = BTreeMap::new();
        let mut current = start;

        for _ in 0..32 {
            if visited.contains_key(&current) {
                if current == start && path.len() >= 2 {
                    return path;
                }
                return Vec::new();
            }
            visited.insert(current, true);
            path.push(current);

            let next = self.priorities.get(&current)
                .and_then(|p| p.waiting_for.first())
                .and_then(|&r| self.holdings.get(&r))
                .map(|h| h.holder)
                .unwrap_or(0);

            if next == 0 {
                return Vec::new();
            }
            current = next;
        }

        Vec::new()
    }

    fn compute_inversion_depth(&self, blocker: u64) -> u32 {
        let mut depth: u32 = 0;
        let mut current = blocker;
        let mut visited: BTreeMap<u64, bool> = BTreeMap::new();

        for _ in 0..16 {
            if visited.contains_key(&current) {
                break;
            }
            visited.insert(current, true);

            let next = self.priorities.get(&current)
                .and_then(|p| p.waiting_for.first())
                .and_then(|&r| self.holdings.get(&r))
                .map(|h| h.holder)
                .unwrap_or(0);

            if next == 0 {
                break;
            }
            depth += 1;
            current = next;
        }

        depth
    }

    fn catalog_pattern(&mut self, anomaly_type: AnomalyType, instance_id: u64, severity: u64) {
        let sig = fnv1a_hash(&[
            (anomaly_type.clone() as u8 as u64).to_le_bytes().as_slice(),
            instance_id.to_le_bytes().as_slice(),
        ].concat());

        let pattern = self.pattern_library.entry(sig).or_insert_with(|| AnomalyPattern {
            pattern_id: sig,
            anomaly_type: anomaly_type.clone(),
            signature_hash: sig,
            occurrence_count: 0,
            avg_severity: severity,
            avg_duration: 0,
            known_mitigations: Vec::new(),
        });
        pattern.occurrence_count = pattern.occurrence_count.saturating_add(1);
        pattern.avg_severity = ema_update(pattern.avg_severity, severity, 200, 1000);

        self.stats.patterns_catalogued = self.pattern_library.len() as u64;
        self.stats.avg_severity = ema_update(self.stats.avg_severity, severity, 150, 1000);
    }
}
