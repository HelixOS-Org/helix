// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Precognition Engine
//!
//! Pre-cognitive cooperation sensing. Detects subtle shifts in the
//! cooperation climate before they fully manifest — trust erosion,
//! contention buildup, cooperation phase changes — enabling early
//! adaptive responses to emerging cooperation dynamics.

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

/// Cooperation climate phase.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CooperationPhase {
    Stable,
    Growing,
    Declining,
    Crisis,
    Recovery,
    Transition,
}

/// A detected shift in cooperation climate.
#[derive(Clone, Debug)]
pub struct CooperationShift {
    pub shift_id: u64,
    pub from_phase: CooperationPhase,
    pub to_phase: CooperationPhase,
    pub shift_magnitude: u64,
    pub confidence: u64,
    pub trigger_signals: Vec<u64>,
    pub estimated_duration: u64,
}

/// Trust erosion detection result.
#[derive(Clone, Debug)]
pub struct TrustErosionDetection {
    pub partner_id: u64,
    pub erosion_rate: u64,
    pub current_trust: u64,
    pub projected_trust: u64,
    pub erosion_cause_hash: u64,
    pub ticks_until_critical: u64,
    pub confidence: u64,
}

/// Contention buildup sensing.
#[derive(Clone, Debug)]
pub struct ContentionBuildup {
    pub resource_id: u64,
    pub buildup_rate: u64,
    pub current_pressure: u64,
    pub projected_peak: u64,
    pub ticks_until_peak: u64,
    pub contributing_processes: Vec<u64>,
    pub confidence: u64,
}

/// Cooperation phase change detection.
#[derive(Clone, Debug)]
pub struct PhaseChange {
    pub previous_phase: CooperationPhase,
    pub emerging_phase: CooperationPhase,
    pub transition_progress: u64,
    pub stability_score: u64,
    pub signals_detected: u32,
    pub confidence: u64,
}

/// Precognition signal measurement.
#[derive(Clone, Debug)]
pub struct PrecognitionSignal {
    pub signal_id: u64,
    pub signal_type_hash: u64,
    pub strength: u64,
    pub direction: i64,
    pub noise_ratio: u64,
    pub actionable: bool,
}

/// Early adaptation recommendation.
#[derive(Clone, Debug)]
pub struct EarlyAdaptation {
    pub adaptation_id: u64,
    pub trigger_signal: u64,
    pub recommended_action_hash: u64,
    pub urgency: u64,
    pub expected_benefit: u64,
    pub risk: u64,
}

/// Rolling statistics for the precognition engine.
#[derive(Clone, Debug)]
pub struct PrecognitionStats {
    pub shifts_detected: u64,
    pub erosions_detected: u64,
    pub buildups_sensed: u64,
    pub phase_changes: u64,
    pub signals_generated: u64,
    pub adaptations_recommended: u64,
    pub avg_signal_strength: u64,
    pub avg_lead_time: u64,
}

impl PrecognitionStats {
    pub fn new() -> Self {
        Self {
            shifts_detected: 0,
            erosions_detected: 0,
            buildups_sensed: 0,
            phase_changes: 0,
            signals_generated: 0,
            adaptations_recommended: 0,
            avg_signal_strength: 0,
            avg_lead_time: 0,
        }
    }
}

/// Internal trust trajectory tracker.
#[derive(Clone, Debug)]
struct TrustTracker {
    partner_id: u64,
    trust_history: Vec<u64>,
    ema_trust: u64,
    ema_delta: i64,
    consecutive_drops: u32,
    last_peak: u64,
}

/// Internal contention accumulation tracker.
#[derive(Clone, Debug)]
struct ContentionAccumulator {
    resource_id: u64,
    pressure_history: Vec<u64>,
    ema_pressure: u64,
    ema_acceleration: i64,
    processes_seen: BTreeMap<u64, u64>,
}

/// Internal climate signal aggregator.
#[derive(Clone, Debug)]
struct ClimateSignal {
    signal_hash: u64,
    values: Vec<u64>,
    ema_value: u64,
    ema_trend: i64,
    activated_at: u64,
}

/// Internal phase state tracker.
#[derive(Clone, Debug)]
struct PhaseState {
    current_phase: CooperationPhase,
    phase_start_tick: u64,
    phase_stability: u64,
    transition_signals: Vec<u64>,
}

/// Pre-cognitive cooperation sensing engine.
pub struct CoopPrecognition {
    trust_trackers: BTreeMap<u64, TrustTracker>,
    contention_accumulators: BTreeMap<u64, ContentionAccumulator>,
    climate_signals: BTreeMap<u64, ClimateSignal>,
    phase_state: PhaseState,
    signal_history: Vec<PrecognitionSignal>,
    stats: PrecognitionStats,
    rng_state: u64,
    current_tick: u64,
    max_history: usize,
    erosion_threshold: u64,
    buildup_threshold: u64,
}

impl CoopPrecognition {
    /// Create a new precognition engine.
    pub fn new(seed: u64) -> Self {
        Self {
            trust_trackers: BTreeMap::new(),
            contention_accumulators: BTreeMap::new(),
            climate_signals: BTreeMap::new(),
            phase_state: PhaseState {
                current_phase: CooperationPhase::Stable,
                phase_start_tick: 0,
                phase_stability: 500,
                transition_signals: Vec::new(),
            },
            signal_history: Vec::new(),
            stats: PrecognitionStats::new(),
            rng_state: seed ^ 0xF8EC_0671_C00F_0001,
            current_tick: 0,
            max_history: 256,
            erosion_threshold: 3,
            buildup_threshold: 600,
        }
    }

    /// Feed a trust observation for a partner.
    pub fn feed_trust(&mut self, partner_id: u64, trust_value: u64) {
        let tracker = self.trust_trackers.entry(partner_id).or_insert_with(|| TrustTracker {
            partner_id,
            trust_history: Vec::new(),
            ema_trust: trust_value,
            ema_delta: 0,
            consecutive_drops: 0,
            last_peak: trust_value,
        });

        let prev = tracker.ema_trust;
        tracker.trust_history.push(trust_value);
        tracker.ema_trust = ema_update(tracker.ema_trust, trust_value, 200, 1000);

        let delta = tracker.ema_trust as i64 - prev as i64;
        tracker.ema_delta = (tracker.ema_delta.saturating_mul(800) + delta.saturating_mul(200)) / 1000;

        if delta < 0 {
            tracker.consecutive_drops = tracker.consecutive_drops.saturating_add(1);
        } else {
            tracker.consecutive_drops = 0;
            if trust_value > tracker.last_peak {
                tracker.last_peak = trust_value;
            }
        }

        if tracker.trust_history.len() > self.max_history {
            tracker.trust_history.remove(0);
        }

        self.emit_trust_signal(partner_id, tracker.ema_delta, tracker.consecutive_drops);
    }

    /// Feed a contention observation for a resource.
    pub fn feed_contention(&mut self, resource_id: u64, pressure: u64, process_id: u64) {
        let acc = self.contention_accumulators.entry(resource_id).or_insert_with(|| ContentionAccumulator {
            resource_id,
            pressure_history: Vec::new(),
            ema_pressure: pressure,
            ema_acceleration: 0,
            processes_seen: BTreeMap::new(),
        });

        let prev = acc.ema_pressure;
        acc.pressure_history.push(pressure);
        acc.ema_pressure = ema_update(acc.ema_pressure, pressure, 250, 1000);

        let accel = acc.ema_pressure as i64 - prev as i64;
        acc.ema_acceleration = (acc.ema_acceleration.saturating_mul(700) + accel.saturating_mul(300)) / 1000;

        *acc.processes_seen.entry(process_id).or_insert(0) += 1;

        if acc.pressure_history.len() > self.max_history {
            acc.pressure_history.remove(0);
        }

        self.emit_contention_signal(resource_id, acc.ema_pressure, acc.ema_acceleration);
    }

    /// Sense a shift in the overall cooperation climate.
    pub fn sense_cooperation_shift(&mut self) -> Option<CooperationShift> {
        let trust_signals = self.aggregate_trust_signals();
        let contention_signals = self.aggregate_contention_signals();

        let combined_trend = trust_signals.0 as i64 - contention_signals.0 as i64;
        let magnitude = combined_trend.unsigned_abs();

        if magnitude < 100 {
            return None;
        }

        let from_phase = self.phase_state.current_phase.clone();
        let to_phase = self.determine_phase(combined_trend, magnitude);

        if from_phase == to_phase {
            return None;
        }

        let noise = xorshift64(&mut self.rng_state) % 50;
        let confidence = (magnitude.min(500).saturating_mul(2)).saturating_sub(noise).min(1000);

        let trigger_signals: Vec<u64> = self.climate_signals.keys().copied().take(5).collect();
        let duration = 50u64.saturating_add(magnitude / 5);

        let shift_id = fnv1a_hash(&[
            self.current_tick.to_le_bytes().as_slice(),
            magnitude.to_le_bytes().as_slice(),
        ].concat());

        self.phase_state.current_phase = to_phase.clone();
        self.phase_state.phase_start_tick = self.current_tick;

        self.stats.shifts_detected = self.stats.shifts_detected.saturating_add(1);

        Some(CooperationShift {
            shift_id,
            from_phase,
            to_phase,
            shift_magnitude: magnitude,
            confidence,
            trigger_signals,
            estimated_duration: duration,
        })
    }

    /// Detect trust erosion for a partner.
    pub fn trust_erosion_detection(&mut self, partner_id: u64) -> Option<TrustErosionDetection> {
        let tracker = self.trust_trackers.get(&partner_id)?;

        if tracker.consecutive_drops < self.erosion_threshold as u32 {
            return None;
        }

        let erosion_rate = tracker.ema_delta.unsigned_abs().min(1000);
        let projected = if tracker.ema_delta < 0 {
            tracker.ema_trust.saturating_sub(erosion_rate.saturating_mul(10))
        } else {
            tracker.ema_trust
        };

        let critical_level: u64 = 200;
        let ticks_until_critical = if tracker.ema_trust > critical_level && erosion_rate > 0 {
            (tracker.ema_trust - critical_level).saturating_mul(10) / erosion_rate.max(1)
        } else if tracker.ema_trust <= critical_level {
            0
        } else {
            1000
        };

        let cause_hash = fnv1a_hash(&[
            partner_id.to_le_bytes().as_slice(),
            tracker.consecutive_drops.to_le_bytes().as_slice(),
        ].concat());

        let confidence = (tracker.consecutive_drops as u64)
            .saturating_mul(200)
            .min(900);

        self.stats.erosions_detected = self.stats.erosions_detected.saturating_add(1);
        self.stats.avg_lead_time = ema_update(
            self.stats.avg_lead_time,
            ticks_until_critical,
            200,
            1000,
        );

        Some(TrustErosionDetection {
            partner_id,
            erosion_rate,
            current_trust: tracker.ema_trust,
            projected_trust: projected,
            erosion_cause_hash: cause_hash,
            ticks_until_critical,
            confidence,
        })
    }

    /// Sense contention buildup for a resource.
    pub fn contention_buildup(&mut self, resource_id: u64) -> Option<ContentionBuildup> {
        let acc = self.contention_accumulators.get(&resource_id)?;

        if acc.ema_pressure < self.buildup_threshold || acc.ema_acceleration <= 0 {
            return None;
        }

        let buildup_rate = acc.ema_acceleration as u64;
        let projected_peak = acc.ema_pressure
            .saturating_add(buildup_rate.saturating_mul(10))
            .min(1000);

        let ticks_until_peak = if buildup_rate > 0 {
            (1000u64.saturating_sub(acc.ema_pressure)) / buildup_rate.max(1)
        } else {
            100
        };

        let contributing: Vec<u64> = acc.processes_seen.keys().copied().collect();

        let confidence = if acc.pressure_history.len() > 10 {
            700
        } else {
            400
        };

        self.stats.buildups_sensed = self.stats.buildups_sensed.saturating_add(1);

        Some(ContentionBuildup {
            resource_id,
            buildup_rate,
            current_pressure: acc.ema_pressure,
            projected_peak,
            ticks_until_peak,
            contributing_processes: contributing,
            confidence,
        })
    }

    /// Detect a cooperation phase change.
    pub fn cooperation_phase_change(&mut self) -> Option<PhaseChange> {
        let trust_agg = self.aggregate_trust_signals();
        let contention_agg = self.aggregate_contention_signals();

        let combined = trust_agg.0 as i64 - contention_agg.0 as i64;
        let magnitude = combined.unsigned_abs();

        let emerging = self.determine_phase(combined, magnitude);

        if emerging == self.phase_state.current_phase {
            self.phase_state.phase_stability = ema_update(
                self.phase_state.phase_stability,
                800,
                200,
                1000,
            );
            return None;
        }

        let phase_age = self.current_tick.saturating_sub(self.phase_state.phase_start_tick);
        let transition_progress = (magnitude.saturating_mul(10)).min(1000);
        let stability = self.phase_state.phase_stability;
        let signals = self.phase_state.transition_signals.len() as u32;

        let confidence = transition_progress.saturating_mul(
            1000u64.saturating_sub(stability),
        ) / 1000;

        if confidence < 300 {
            return None;
        }

        self.stats.phase_changes = self.stats.phase_changes.saturating_add(1);

        Some(PhaseChange {
            previous_phase: self.phase_state.current_phase.clone(),
            emerging_phase: emerging,
            transition_progress,
            stability_score: stability,
            signals_detected: signals,
            confidence,
        })
    }

    /// Generate a precognition signal from current state.
    pub fn precognition_signal(&mut self) -> PrecognitionSignal {
        let trust_agg = self.aggregate_trust_signals();
        let contention_agg = self.aggregate_contention_signals();

        let direction = trust_agg.0 as i64 - contention_agg.0 as i64;
        let strength = direction.unsigned_abs().min(1000);
        let noise_ratio = if strength > 0 {
            let noise = xorshift64(&mut self.rng_state) % (strength.max(1));
            noise.saturating_mul(1000) / strength.max(1)
        } else {
            1000
        };

        let actionable = strength > 200 && noise_ratio < 500;

        let sig_id = fnv1a_hash(&[
            self.current_tick.to_le_bytes().as_slice(),
            strength.to_le_bytes().as_slice(),
        ].concat());

        let signal = PrecognitionSignal {
            signal_id: sig_id,
            signal_type_hash: fnv1a_hash(b"coop_precog"),
            strength,
            direction,
            noise_ratio,
            actionable,
        };

        self.signal_history.push(signal.clone());
        if self.signal_history.len() > self.max_history {
            self.signal_history.remove(0);
        }

        self.stats.signals_generated = self.stats.signals_generated.saturating_add(1);
        self.stats.avg_signal_strength = ema_update(
            self.stats.avg_signal_strength,
            strength,
            200,
            1000,
        );

        signal
    }

    /// Generate early adaptation recommendations.
    pub fn early_adaptation(&mut self) -> Vec<EarlyAdaptation> {
        let mut adaptations: Vec<EarlyAdaptation> = Vec::new();

        let erosion_partners: Vec<u64> = self.trust_trackers.keys().copied().collect();
        for pid in erosion_partners {
            if let Some(erosion) = self.trust_erosion_detection(pid) {
                if erosion.confidence > 500 {
                    let aid = fnv1a_hash(&[
                        pid.to_le_bytes().as_slice(),
                        b"trust_repair",
                    ].concat());
                    adaptations.push(EarlyAdaptation {
                        adaptation_id: aid,
                        trigger_signal: erosion.erosion_cause_hash,
                        recommended_action_hash: fnv1a_hash(b"increase_fairness"),
                        urgency: erosion.erosion_rate.min(1000),
                        expected_benefit: erosion.current_trust.saturating_sub(erosion.projected_trust),
                        risk: 200,
                    });
                }
            }
        }

        let buildup_resources: Vec<u64> = self.contention_accumulators.keys().copied().collect();
        for rid in buildup_resources {
            if let Some(buildup) = self.contention_buildup(rid) {
                if buildup.confidence > 500 {
                    let aid = fnv1a_hash(&[
                        rid.to_le_bytes().as_slice(),
                        b"load_balance",
                    ].concat());
                    adaptations.push(EarlyAdaptation {
                        adaptation_id: aid,
                        trigger_signal: rid,
                        recommended_action_hash: fnv1a_hash(b"redistribute_load"),
                        urgency: buildup.buildup_rate.min(1000),
                        expected_benefit: buildup.projected_peak.saturating_sub(buildup.current_pressure),
                        risk: 300,
                    });
                }
            }
        }

        self.stats.adaptations_recommended = self.stats.adaptations_recommended
            .saturating_add(adaptations.len() as u64);

        adaptations
    }

    /// Advance the internal tick.
    pub fn tick(&mut self) {
        self.current_tick = self.current_tick.wrapping_add(1);
    }

    /// Retrieve current statistics.
    pub fn stats(&self) -> &PrecognitionStats {
        &self.stats
    }

    // ── Private helpers ──────────────────────────────────────────────

    fn emit_trust_signal(&mut self, partner_id: u64, delta: i64, consecutive_drops: u32) {
        let sig_hash = fnv1a_hash(&[
            b"trust_",
            partner_id.to_le_bytes().as_slice(),
        ].concat());

        let signal = self.climate_signals.entry(sig_hash).or_insert_with(|| ClimateSignal {
            signal_hash: sig_hash,
            values: Vec::new(),
            ema_value: 500,
            ema_trend: 0,
            activated_at: self.current_tick,
        });

        let val = if delta > 0 {
            500u64.saturating_add(delta as u64)
        } else {
            500u64.saturating_sub(delta.unsigned_abs())
        };

        signal.values.push(val);
        signal.ema_value = ema_update(signal.ema_value, val, 200, 1000);
        signal.ema_trend = (signal.ema_trend.saturating_mul(800) + delta.saturating_mul(200)) / 1000;

        if signal.values.len() > 64 {
            signal.values.remove(0);
        }

        if consecutive_drops > 2 {
            self.phase_state.transition_signals.push(sig_hash);
        }
    }

    fn emit_contention_signal(&mut self, resource_id: u64, pressure: u64, acceleration: i64) {
        let sig_hash = fnv1a_hash(&[
            b"cont_",
            resource_id.to_le_bytes().as_slice(),
        ].concat());

        let signal = self.climate_signals.entry(sig_hash).or_insert_with(|| ClimateSignal {
            signal_hash: sig_hash,
            values: Vec::new(),
            ema_value: 500,
            ema_trend: 0,
            activated_at: self.current_tick,
        });

        signal.values.push(pressure);
        signal.ema_value = ema_update(signal.ema_value, pressure, 200, 1000);
        signal.ema_trend = (signal.ema_trend.saturating_mul(800)
            + acceleration.saturating_mul(200)) / 1000;

        if signal.values.len() > 64 {
            signal.values.remove(0);
        }
    }

    fn aggregate_trust_signals(&self) -> (u64, u64) {
        let trust_sigs: Vec<&ClimateSignal> = self.climate_signals.values()
            .filter(|s| {
                let key_bytes = s.signal_hash.to_le_bytes();
                key_bytes[0] % 2 == 0
            })
            .collect();

        if trust_sigs.is_empty() {
            return (500, 0);
        }

        let avg = trust_sigs.iter().map(|s| s.ema_value).sum::<u64>()
            / trust_sigs.len() as u64;
        let count = trust_sigs.len() as u64;
        (avg, count)
    }

    fn aggregate_contention_signals(&self) -> (u64, u64) {
        let cont_sigs: Vec<&ClimateSignal> = self.climate_signals.values()
            .filter(|s| {
                let key_bytes = s.signal_hash.to_le_bytes();
                key_bytes[0] % 2 == 1
            })
            .collect();

        if cont_sigs.is_empty() {
            return (500, 0);
        }

        let avg = cont_sigs.iter().map(|s| s.ema_value).sum::<u64>()
            / cont_sigs.len() as u64;
        let count = cont_sigs.len() as u64;
        (avg, count)
    }

    fn determine_phase(&self, combined_trend: i64, magnitude: u64) -> CooperationPhase {
        if magnitude < 100 {
            CooperationPhase::Stable
        } else if combined_trend > 0 && magnitude > 300 {
            CooperationPhase::Growing
        } else if combined_trend < 0 && magnitude > 500 {
            CooperationPhase::Crisis
        } else if combined_trend < 0 && magnitude > 200 {
            CooperationPhase::Declining
        } else if combined_trend > 0 && magnitude > 100 {
            CooperationPhase::Recovery
        } else {
            CooperationPhase::Transition
        }
    }
}
