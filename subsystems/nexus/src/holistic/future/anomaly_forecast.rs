// SPDX-License-Identifier: GPL-2.0
//! # Holistic Anomaly Forecast — System-Wide Anomaly Prediction
//!
//! Predicts **system-wide anomalies** before they occur: cascading failures,
//! resource exhaustion waves, performance cliffs, and emergent pathological
//! patterns that only become visible when analyzing the entire system as a
//! whole. Individual subsystem anomaly detectors catch local oddities; this
//! module catches the global ones — the ones that bring systems down.
//!
//! ## Capabilities
//!
//! - System-wide anomaly forecasting from multi-subsystem signals
//! - Cascade failure prediction: which failure chains are forming?
//! - Performance cliff detection: approaching non-linear degradation points
//! - Systemic risk assessment: overall system fragility score
//! - Early system warning: signal aggregation and threshold alerting
//! - Prevention strategy planning: what to do before the anomaly hits

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ANOMALY_SIGNALS: usize = 2048;
const MAX_CASCADE_CHAINS: usize = 128;
const MAX_CLIFF_DETECTORS: usize = 64;
const MAX_RISK_HISTORY: usize = 512;
const MAX_WARNING_LOG: usize = 256;
const MAX_PREVENTION_PLANS: usize = 64;
const CLIFF_THRESHOLD: f32 = 0.85;
const RISK_DECAY: f32 = 0.96;
const SIGNAL_HALF_LIFE_US: u64 = 30_000_000;
const EMA_ALPHA: f32 = 0.10;
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
// DOMAIN TYPES
// ============================================================================

/// Anomaly category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnomalyCategory {
    CascadeFailure,
    ResourceExhaustion,
    PerformanceCliff,
    LatencySpiral,
    DeadlockFormation,
    ThermalRunaway,
    MemoryLeak,
    ContextSwitchStorm,
    IoStarvation,
    NetworkPartition,
    SecurityBreach,
    CorruptionSpread,
}

/// Subsystem that originates the anomaly signal
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AnomalySource {
    Scheduler,
    Memory,
    Io,
    Network,
    Thermal,
    Power,
    FileSystem,
    Ipc,
    Security,
    Driver,
    Userspace,
    Ensemble,
}

/// Severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeverityLevel {
    Advisory,
    Warning,
    Critical,
    Emergency,
}

// ============================================================================
// ANOMALY FORECAST STRUCTURES
// ============================================================================

/// An anomaly signal from a subsystem
#[derive(Debug, Clone)]
pub struct AnomalySignal {
    pub signal_id: u64,
    pub source: AnomalySource,
    pub category: AnomalyCategory,
    pub severity: SeverityLevel,
    pub magnitude: f32,
    pub timestamp_us: u64,
    pub description: String,
    pub confidence: f32,
}

/// System-wide anomaly forecast result
#[derive(Debug, Clone)]
pub struct SystemAnomalyForecast {
    pub forecasted_anomalies: Vec<ForecastedAnomaly>,
    pub overall_risk: f32,
    pub highest_risk_category: AnomalyCategory,
    pub active_signals: usize,
    pub suppressed_signals: usize,
    pub forecast_horizon_us: u64,
    pub timestamp_us: u64,
}

/// A single forecasted anomaly
#[derive(Debug, Clone)]
pub struct ForecastedAnomaly {
    pub category: AnomalyCategory,
    pub probability: f32,
    pub estimated_onset_us: u64,
    pub estimated_severity: SeverityLevel,
    pub contributing_sources: Vec<AnomalySource>,
    pub confidence: f32,
    pub description: String,
}

/// Cascade failure prediction
#[derive(Debug, Clone)]
pub struct CascadePrediction {
    pub trigger_source: AnomalySource,
    pub chain: Vec<CascadeStep>,
    pub total_probability: f32,
    pub total_impact: f32,
    pub estimated_duration_us: u64,
    pub subsystems_at_risk: Vec<AnomalySource>,
}

/// A step in a cascade chain
#[derive(Debug, Clone)]
pub struct CascadeStep {
    pub source: AnomalySource,
    pub category: AnomalyCategory,
    pub step_probability: f32,
    pub step_delay_us: u64,
    pub impact: f32,
}

/// Performance cliff warning
#[derive(Debug, Clone)]
pub struct CliffWarning {
    pub source: AnomalySource,
    pub metric_name: String,
    pub current_value: f32,
    pub cliff_threshold: f32,
    pub distance_to_cliff: f32,
    pub rate_of_approach: f32,
    pub estimated_time_to_cliff_us: u64,
    pub severity: SeverityLevel,
}

/// Systemic risk assessment
#[derive(Debug, Clone)]
pub struct SystemicRisk {
    pub overall_risk_score: f32,
    pub risk_by_category: BTreeMap<u64, f32>,
    pub risk_by_source: BTreeMap<u64, f32>,
    pub fragility_index: f32,
    pub resilience_score: f32,
    pub risk_trend: f32,
    pub top_risk_factors: Vec<RiskFactor>,
}

/// A specific risk factor
#[derive(Debug, Clone)]
pub struct RiskFactor {
    pub source: AnomalySource,
    pub category: AnomalyCategory,
    pub risk_contribution: f32,
    pub description: String,
}

/// Early system warning
#[derive(Debug, Clone)]
pub struct EarlySystemWarning {
    pub warning_id: u64,
    pub category: AnomalyCategory,
    pub severity: SeverityLevel,
    pub lead_time_us: u64,
    pub confidence: f32,
    pub contributing_signals: Vec<u64>,
    pub recommended_action: String,
    pub timestamp_us: u64,
}

/// Prevention strategy
#[derive(Debug, Clone)]
pub struct PreventionStrategy {
    pub strategy_id: u64,
    pub target_anomaly: AnomalyCategory,
    pub actions: Vec<PreventionAction>,
    pub expected_risk_reduction: f32,
    pub cost: f32,
    pub priority: f32,
    pub feasibility: f32,
}

/// A single prevention action
#[derive(Debug, Clone)]
pub struct PreventionAction {
    pub target_source: AnomalySource,
    pub action_type: PreventionActionType,
    pub description: String,
    pub impact: f32,
}

/// Type of prevention action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreventionActionType {
    Throttle,
    Reroute,
    PreAllocate,
    Isolate,
    Shed,
    Reconfigure,
    Alert,
}

// ============================================================================
// STATISTICS
// ============================================================================

/// Runtime statistics for the anomaly forecast engine
#[derive(Debug, Clone)]
pub struct AnomalyForecastStats {
    pub forecasts_generated: u64,
    pub cascades_predicted: u64,
    pub cliff_warnings: u64,
    pub risk_assessments: u64,
    pub early_warnings: u64,
    pub prevention_plans: u64,
    pub avg_risk_score: f32,
    pub avg_lead_time_us: f32,
    pub avg_cascade_length: f32,
    pub avg_confidence: f32,
}

impl AnomalyForecastStats {
    fn new() -> Self {
        Self {
            forecasts_generated: 0,
            cascades_predicted: 0,
            cliff_warnings: 0,
            risk_assessments: 0,
            early_warnings: 0,
            prevention_plans: 0,
            avg_risk_score: 0.0,
            avg_lead_time_us: 0.0,
            avg_cascade_length: 0.0,
            avg_confidence: 0.0,
        }
    }
}

// ============================================================================
// HOLISTIC ANOMALY FORECAST ENGINE
// ============================================================================

/// System-wide anomaly prediction engine
pub struct HolisticAnomalyForecast {
    signals: Vec<AnomalySignal>,
    signal_index: BTreeMap<u64, usize>,
    cascade_history: Vec<CascadePrediction>,
    risk_history: Vec<SystemicRisk>,
    warning_log: Vec<EarlySystemWarning>,
    prevention_log: Vec<PreventionStrategy>,
    rng_state: u64,
    next_signal_id: u64,
    next_warning_id: u64,
    next_strategy_id: u64,
    cumulative_risk: f32,
    stats: AnomalyForecastStats,
    generation: u64,
}

impl HolisticAnomalyForecast {
    /// Create a new holistic anomaly forecast engine
    pub fn new(seed: u64) -> Self {
        Self {
            signals: Vec::new(),
            signal_index: BTreeMap::new(),
            cascade_history: Vec::new(),
            risk_history: Vec::new(),
            warning_log: Vec::new(),
            prevention_log: Vec::new(),
            rng_state: seed ^ 0xDEAD_C0DE_FACE_B00C,
            next_signal_id: 1,
            next_warning_id: 1,
            next_strategy_id: 1,
            cumulative_risk: 0.0,
            stats: AnomalyForecastStats::new(),
            generation: 0,
        }
    }

    /// Ingest an anomaly signal from a subsystem
    pub fn ingest_signal(
        &mut self,
        source: AnomalySource,
        category: AnomalyCategory,
        severity: SeverityLevel,
        magnitude: f32,
        timestamp_us: u64,
    ) -> u64 {
        let id = self.next_signal_id;
        self.next_signal_id += 1;
        let signal = AnomalySignal {
            signal_id: id,
            source,
            category,
            severity,
            magnitude: magnitude.clamp(0.0, 1.0),
            timestamp_us,
            description: String::new(),
            confidence: 0.5 + (xorshift64(&mut self.rng_state) % 50) as f32 / 100.0,
        };
        let idx = self.signals.len();
        if self.signals.len() < MAX_ANOMALY_SIGNALS {
            self.signals.push(signal);
            self.signal_index.insert(id, idx);
        }
        id
    }

    /// Generate system-wide anomaly forecast
    pub fn system_anomaly_forecast(
        &mut self,
        horizon_us: u64,
        timestamp_us: u64,
    ) -> SystemAnomalyForecast {
        self.stats.forecasts_generated += 1;
        self.generation += 1;

        let mut category_risk: BTreeMap<u8, f32> = BTreeMap::new();
        let mut category_sources: BTreeMap<u8, Vec<AnomalySource>> = BTreeMap::new();
        let mut active = 0_usize;
        let mut suppressed = 0_usize;

        for signal in &self.signals {
            let age = timestamp_us.saturating_sub(signal.timestamp_us);
            if age > SIGNAL_HALF_LIFE_US * 4 {
                suppressed += 1;
                continue;
            }
            active += 1;
            let decay = (-(age as f32) / SIGNAL_HALF_LIFE_US as f32).exp();
            let contribution = signal.magnitude * signal.confidence * decay;
            let ck = signal.category as u8;
            let entry = category_risk.entry(ck).or_insert(0.0);
            *entry += contribution;
            category_sources
                .entry(ck)
                .or_insert_with(Vec::new)
                .push(signal.source);
        }

        let mut forecasted: Vec<ForecastedAnomaly> = Vec::new();
        let mut highest_risk = 0.0_f32;
        let mut highest_cat = AnomalyCategory::CascadeFailure;

        for (&ck, &risk) in &category_risk {
            if risk > highest_risk {
                highest_risk = risk;
                highest_cat = self.u8_to_category(ck);
            }
            let category = self.u8_to_category(ck);
            let probability = (risk / 5.0).min(1.0);
            let onset = horizon_us / (1 + (risk * 10.0) as u64).max(1);
            let sev = if risk > 3.0 {
                SeverityLevel::Emergency
            } else if risk > 2.0 {
                SeverityLevel::Critical
            } else if risk > 1.0 {
                SeverityLevel::Warning
            } else {
                SeverityLevel::Advisory
            };
            let sources = category_sources.get(&ck).cloned().unwrap_or_default();
            let unique_sources = self.deduplicate_sources(&sources);

            forecasted.push(ForecastedAnomaly {
                category,
                probability,
                estimated_onset_us: onset,
                estimated_severity: sev,
                contributing_sources: unique_sources,
                confidence: (probability * 0.8).min(0.95),
                description: String::new(),
            });
        }

        let overall = (highest_risk / 5.0).min(1.0);
        self.stats.avg_risk_score = ema_update(self.stats.avg_risk_score, overall);

        SystemAnomalyForecast {
            forecasted_anomalies: forecasted,
            overall_risk: overall,
            highest_risk_category: highest_cat,
            active_signals: active,
            suppressed_signals: suppressed,
            forecast_horizon_us: horizon_us,
            timestamp_us,
        }
    }

    /// Predict cascade failures from current signals
    pub fn cascade_prediction(&mut self) -> Vec<CascadePrediction> {
        self.stats.cascades_predicted += 1;
        let mut cascades: Vec<CascadePrediction> = Vec::new();

        let sources = [
            AnomalySource::Scheduler, AnomalySource::Memory, AnomalySource::Io,
            AnomalySource::Network, AnomalySource::Thermal, AnomalySource::Power,
        ];

        for &trigger in &sources {
            let trigger_signals: Vec<&AnomalySignal> = self
                .signals
                .iter()
                .filter(|s| s.source == trigger && s.magnitude > 0.3)
                .collect();

            if trigger_signals.is_empty() {
                continue;
            }

            let trigger_mag: f32 = trigger_signals.iter().map(|s| s.magnitude).sum::<f32>()
                / trigger_signals.len() as f32;

            let mut chain: Vec<CascadeStep> = Vec::new();
            let mut prob = trigger_mag;
            let mut total_impact = 0.0_f32;
            let mut total_delay = 0_u64;
            let mut at_risk: Vec<AnomalySource> = Vec::new();

            for &next_source in &sources {
                if next_source == trigger {
                    continue;
                }
                prob *= RISK_DECAY;
                if prob < 0.05 {
                    break;
                }
                let impact = prob * (xorshift64(&mut self.rng_state) % 100) as f32 / 100.0;
                let delay = xorshift64(&mut self.rng_state) % 1_000_000;
                total_impact += impact;
                total_delay += delay;
                at_risk.push(next_source);

                chain.push(CascadeStep {
                    source: next_source,
                    category: AnomalyCategory::CascadeFailure,
                    step_probability: prob,
                    step_delay_us: delay,
                    impact,
                });
            }

            if !chain.is_empty() && cascades.len() < MAX_CASCADE_CHAINS {
                self.stats.avg_cascade_length =
                    ema_update(self.stats.avg_cascade_length, chain.len() as f32);
                cascades.push(CascadePrediction {
                    trigger_source: trigger,
                    chain,
                    total_probability: prob,
                    total_impact,
                    estimated_duration_us: total_delay,
                    subsystems_at_risk: at_risk,
                });
            }
        }

        for c in &cascades {
            if self.cascade_history.len() < MAX_CASCADE_CHAINS {
                self.cascade_history.push(c.clone());
            }
        }
        cascades
    }

    /// Detect approaching performance cliffs
    pub fn cliff_warning(&mut self) -> Vec<CliffWarning> {
        self.stats.cliff_warnings += 1;
        let mut warnings: Vec<CliffWarning> = Vec::new();

        let sources = [
            AnomalySource::Scheduler, AnomalySource::Memory, AnomalySource::Io,
            AnomalySource::Network, AnomalySource::Thermal, AnomalySource::Power,
        ];

        for &src in &sources {
            let src_signals: Vec<&AnomalySignal> = self
                .signals
                .iter()
                .filter(|s| s.source == src)
                .collect();

            if src_signals.is_empty() {
                continue;
            }

            let avg_mag = src_signals.iter().map(|s| s.magnitude).sum::<f32>()
                / src_signals.len() as f32;
            let distance = (CLIFF_THRESHOLD - avg_mag).max(0.0);

            if distance < 0.2 {
                let rate = avg_mag * 0.01;
                let tte = if rate > 0.0 {
                    (distance / rate * 1_000_000.0) as u64
                } else {
                    u64::MAX
                };
                let sev = if distance < 0.05 {
                    SeverityLevel::Emergency
                } else if distance < 0.10 {
                    SeverityLevel::Critical
                } else {
                    SeverityLevel::Warning
                };
                if warnings.len() < MAX_CLIFF_DETECTORS {
                    warnings.push(CliffWarning {
                        source: src,
                        metric_name: String::from("utilization"),
                        current_value: avg_mag,
                        cliff_threshold: CLIFF_THRESHOLD,
                        distance_to_cliff: distance,
                        rate_of_approach: rate,
                        estimated_time_to_cliff_us: tte,
                        severity: sev,
                    });
                }
            }
        }
        warnings
    }

    /// Assess overall systemic risk
    pub fn systemic_risk(&mut self) -> SystemicRisk {
        self.stats.risk_assessments += 1;
        let mut risk_by_cat: BTreeMap<u64, f32> = BTreeMap::new();
        let mut risk_by_src: BTreeMap<u64, f32> = BTreeMap::new();
        let mut total_risk = 0.0_f32;
        let mut factors: Vec<RiskFactor> = Vec::new();

        for signal in &self.signals {
            let contribution = signal.magnitude * signal.confidence;
            total_risk += contribution;
            let ck = fnv1a_hash(&[signal.category as u8]);
            let sk = fnv1a_hash(&[signal.source as u8]);
            *risk_by_cat.entry(ck).or_insert(0.0) += contribution;
            *risk_by_src.entry(sk).or_insert(0.0) += contribution;

            if contribution > 0.3 && factors.len() < 10 {
                factors.push(RiskFactor {
                    source: signal.source,
                    category: signal.category,
                    risk_contribution: contribution,
                    description: String::new(),
                });
            }
        }

        let overall = (total_risk / self.signals.len().max(1) as f32).min(1.0);
        self.cumulative_risk = self.cumulative_risk * RISK_DECAY + overall;
        let fragility = overall * 0.6 + self.cumulative_risk.min(1.0) * 0.4;
        let resilience = (1.0 - fragility).max(0.0);

        factors.sort_by(|a, b| {
            b.risk_contribution
                .partial_cmp(&a.risk_contribution)
                .unwrap_or(core::cmp::Ordering::Equal)
        });

        let risk = SystemicRisk {
            overall_risk_score: overall,
            risk_by_category: risk_by_cat,
            risk_by_source: risk_by_src,
            fragility_index: fragility,
            resilience_score: resilience,
            risk_trend: self.cumulative_risk,
            top_risk_factors: factors,
        };
        if self.risk_history.len() < MAX_RISK_HISTORY {
            self.risk_history.push(risk.clone());
        }
        risk
    }

    /// Generate early system warnings from aggregated signals
    pub fn early_system_warning(&mut self, timestamp_us: u64) -> Vec<EarlySystemWarning> {
        self.stats.early_warnings += 1;
        let mut warnings: Vec<EarlySystemWarning> = Vec::new();

        let mut cat_signals: BTreeMap<u8, Vec<u64>> = BTreeMap::new();
        let mut cat_risk: BTreeMap<u8, f32> = BTreeMap::new();

        for signal in &self.signals {
            let ck = signal.category as u8;
            cat_signals.entry(ck).or_insert_with(Vec::new).push(signal.signal_id);
            *cat_risk.entry(ck).or_insert(0.0) += signal.magnitude * signal.confidence;
        }

        for (&ck, &risk) in &cat_risk {
            if risk < 0.5 {
                continue;
            }
            let category = self.u8_to_category(ck);
            let severity = if risk > 3.0 {
                SeverityLevel::Emergency
            } else if risk > 2.0 {
                SeverityLevel::Critical
            } else {
                SeverityLevel::Warning
            };
            let lead_time = (1_000_000.0 / risk.max(0.1)) as u64;
            let contributing = cat_signals.get(&ck).cloned().unwrap_or_default();
            let confidence = (risk / 5.0).min(0.95);

            let wid = self.next_warning_id;
            self.next_warning_id += 1;

            let warning = EarlySystemWarning {
                warning_id: wid,
                category,
                severity,
                lead_time_us: lead_time,
                confidence,
                contributing_signals: contributing,
                recommended_action: String::from("investigate"),
                timestamp_us,
            };
            if warnings.len() < MAX_WARNING_LOG {
                warnings.push(warning.clone());
            }
            if self.warning_log.len() < MAX_WARNING_LOG {
                self.warning_log.push(warning);
            }
        }

        self.stats.avg_lead_time_us = if !warnings.is_empty() {
            let avg = warnings.iter().map(|w| w.lead_time_us as f32).sum::<f32>()
                / warnings.len() as f32;
            ema_update(self.stats.avg_lead_time_us, avg)
        } else {
            self.stats.avg_lead_time_us
        };

        warnings
    }

    /// Generate prevention strategies for forecasted anomalies
    pub fn prevention_strategy(
        &mut self,
        forecast: &SystemAnomalyForecast,
    ) -> Vec<PreventionStrategy> {
        self.stats.prevention_plans += 1;
        let mut strategies: Vec<PreventionStrategy> = Vec::new();

        for anomaly in &forecast.forecasted_anomalies {
            if anomaly.probability < 0.2 {
                continue;
            }

            let mut actions: Vec<PreventionAction> = Vec::new();
            for src in &anomaly.contributing_sources {
                let action_type = match anomaly.category {
                    AnomalyCategory::ResourceExhaustion => PreventionActionType::PreAllocate,
                    AnomalyCategory::CascadeFailure => PreventionActionType::Isolate,
                    AnomalyCategory::PerformanceCliff => PreventionActionType::Throttle,
                    AnomalyCategory::LatencySpiral => PreventionActionType::Shed,
                    AnomalyCategory::ThermalRunaway => PreventionActionType::Throttle,
                    AnomalyCategory::IoStarvation => PreventionActionType::Reroute,
                    _ => PreventionActionType::Reconfigure,
                };
                actions.push(PreventionAction {
                    target_source: *src,
                    action_type,
                    description: String::new(),
                    impact: anomaly.probability * 0.5,
                });
            }

            let sid = self.next_strategy_id;
            self.next_strategy_id += 1;
            let expected_reduction = anomaly.probability * 0.4;
            let cost = actions.len() as f32 * 0.1;
            let priority = anomaly.probability * (1.0 + expected_reduction);
            let feasibility = (1.0 - cost).max(0.1);

            let strategy = PreventionStrategy {
                strategy_id: sid,
                target_anomaly: anomaly.category,
                actions,
                expected_risk_reduction: expected_reduction,
                cost,
                priority,
                feasibility,
            };
            if strategies.len() < MAX_PREVENTION_PLANS {
                strategies.push(strategy.clone());
            }
            if self.prevention_log.len() < MAX_PREVENTION_PLANS {
                self.prevention_log.push(strategy);
            }
        }

        strategies.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        strategies
    }

    /// Get current statistics
    pub fn stats(&self) -> &AnomalyForecastStats {
        &self.stats
    }

    // ========================================================================
    // PRIVATE HELPERS
    // ========================================================================

    fn u8_to_category(&self, val: u8) -> AnomalyCategory {
        match val {
            0 => AnomalyCategory::CascadeFailure,
            1 => AnomalyCategory::ResourceExhaustion,
            2 => AnomalyCategory::PerformanceCliff,
            3 => AnomalyCategory::LatencySpiral,
            4 => AnomalyCategory::DeadlockFormation,
            5 => AnomalyCategory::ThermalRunaway,
            6 => AnomalyCategory::MemoryLeak,
            7 => AnomalyCategory::ContextSwitchStorm,
            8 => AnomalyCategory::IoStarvation,
            9 => AnomalyCategory::NetworkPartition,
            10 => AnomalyCategory::SecurityBreach,
            11 => AnomalyCategory::CorruptionSpread,
            _ => AnomalyCategory::CascadeFailure,
        }
    }

    fn deduplicate_sources(&self, sources: &[AnomalySource]) -> Vec<AnomalySource> {
        let mut seen: BTreeMap<u8, AnomalySource> = BTreeMap::new();
        for &s in sources {
            seen.insert(s as u8, s);
        }
        seen.values().copied().collect()
    }
}
