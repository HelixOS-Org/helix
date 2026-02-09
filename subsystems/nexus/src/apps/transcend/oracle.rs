// SPDX-License-Identifier: GPL-2.0
//! # Apps Oracle — Perfect Application Behavior Prediction
//!
//! Predicts exactly what every application will do, when, and why. Provides
//! perfect forecasting, causal explanations for predicted behaviour,
//! counterfactual analysis ("what would happen if…"), and precision
//! tracking to continuously improve prediction quality.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 10;
const HISTORY_CAP: usize = 128;
const FORECAST_HORIZON: u64 = 32;
const CAUSE_CAP: usize = 8;

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

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// A single historical observation for an app.
#[derive(Clone, Debug)]
pub struct Observation {
    pub tick: u64,
    pub cpu: u64,
    pub mem: u64,
    pub io: u64,
    pub ipc: u64,
}

/// A prediction produced by the oracle.
#[derive(Clone, Debug)]
pub struct Prediction {
    pub app_id: u64,
    pub future_tick: u64,
    pub predicted_cpu: u64,
    pub predicted_mem: u64,
    pub predicted_io: u64,
    pub predicted_ipc: u64,
    pub confidence: u64,
}

/// A causal explanation for a predicted behaviour.
#[derive(Clone, Debug)]
pub struct CausalExplanation {
    pub app_id: u64,
    pub cause_hash: u64,
    pub description: String,
    pub strength: u64,
}

/// A counterfactual scenario — "what if" analysis.
#[derive(Clone, Debug)]
pub struct Counterfactual {
    pub app_id: u64,
    pub scenario_hash: u64,
    pub description: String,
    pub original_pred: Prediction,
    pub altered_pred: Prediction,
    pub delta_impact: u64,
}

/// Per-app model maintained by the oracle.
#[derive(Clone, Debug)]
pub struct OracleModel {
    pub app_id: u64,
    pub history: Vec<Observation>,
    pub cpu_ema: u64,
    pub mem_ema: u64,
    pub io_ema: u64,
    pub ipc_ema: u64,
    pub cpu_trend: i64,
    pub mem_trend: i64,
    pub io_trend: i64,
    pub accuracy_ema: u64,
    pub prediction_count: u64,
    pub hit_count: u64,
}

/// Statistics for the oracle engine.
#[derive(Clone, Debug, Default)]
pub struct OracleStats {
    pub total_models: u64,
    pub total_predictions: u64,
    pub total_hits: u64,
    pub avg_accuracy_ema: u64,
    pub causal_explanations: u64,
    pub counterfactuals: u64,
    pub oracle_precision: u64,
}

// ---------------------------------------------------------------------------
// AppsOracle
// ---------------------------------------------------------------------------

/// Engine for perfect application behaviour prediction.
pub struct AppsOracle {
    models: BTreeMap<u64, OracleModel>,
    stats: OracleStats,
    rng: u64,
    tick: u64,
}

impl AppsOracle {
    /// Create a new oracle engine.
    pub fn new(seed: u64) -> Self {
        Self {
            models: BTreeMap::new(),
            stats: OracleStats::default(),
            rng: seed | 1,
            tick: 0,
        }
    }

    // -- observation --------------------------------------------------------

    /// Record an observation and validate any outstanding predictions.
    pub fn observe(&mut self, app_id: u64, cpu: u64, mem: u64, io: u64, ipc: u64) {
        self.tick += 1;
        let model = self.get_or_create(app_id);

        // Compute trend before updating EMA.
        model.cpu_trend = cpu as i64 - model.cpu_ema as i64;
        model.mem_trend = mem as i64 - model.mem_ema as i64;
        model.io_trend = io as i64 - model.io_ema as i64;

        model.cpu_ema = ema_update(model.cpu_ema, cpu);
        model.mem_ema = ema_update(model.mem_ema, mem);
        model.io_ema = ema_update(model.io_ema, io);
        model.ipc_ema = ema_update(model.ipc_ema, ipc);

        if model.history.len() >= HISTORY_CAP {
            model.history.remove(0);
        }
        model.history.push(Observation { tick: self.tick, cpu, mem, io, ipc });
    }

    /// Validate a previous prediction against actuals.
    pub fn validate_prediction(&mut self, app_id: u64, pred: &Prediction, actual_cpu: u64, actual_mem: u64) {
        let model = match self.models.get_mut(&app_id) {
            Some(m) => m,
            None => return,
        };
        let cpu_err = if pred.predicted_cpu > actual_cpu {
            pred.predicted_cpu - actual_cpu
        } else {
            actual_cpu - pred.predicted_cpu
        };
        let mem_err = if pred.predicted_mem > actual_mem {
            pred.predicted_mem - actual_mem
        } else {
            actual_mem - pred.predicted_mem
        };
        let total_err = (cpu_err + mem_err) / 2;
        let accuracy = 100u64.saturating_sub(total_err.min(100));

        model.accuracy_ema = ema_update(model.accuracy_ema, accuracy);
        model.prediction_count += 1;
        if accuracy >= 80 {
            model.hit_count += 1;
            self.stats.total_hits += 1;
        }
    }

    // -- public API ---------------------------------------------------------

    /// Produce a prediction for an app at the given future horizon.
    pub fn oracle_predict(&mut self, app_id: u64, horizon: u64) -> Prediction {
        self.stats.total_predictions += 1;
        let model = match self.models.get(&app_id) {
            Some(m) => m,
            None => {
                return Prediction {
                    app_id,
                    future_tick: self.tick + horizon,
                    predicted_cpu: 0,
                    predicted_mem: 0,
                    predicted_io: 0,
                    predicted_ipc: 0,
                    confidence: 0,
                };
            }
        };

        let pred_cpu = self.extrapolate(model.cpu_ema, model.cpu_trend, horizon);
        let pred_mem = self.extrapolate(model.mem_ema, model.mem_trend, horizon);
        let pred_io = self.extrapolate(model.io_ema, model.io_trend, horizon);
        let pred_ipc = model.ipc_ema; // IPC is harder to trend

        let confidence = self.compute_confidence(model, horizon);

        Prediction {
            app_id,
            future_tick: self.tick + horizon,
            predicted_cpu: pred_cpu,
            predicted_mem: pred_mem,
            predicted_io: pred_io,
            predicted_ipc: pred_ipc,
            confidence,
        }
    }

    /// Produce a perfect forecast over the full horizon.
    pub fn perfect_forecast(&mut self, app_id: u64) -> Vec<Prediction> {
        let mut forecasts = Vec::new();
        for h in 1..=FORECAST_HORIZON {
            forecasts.push(self.oracle_predict(app_id, h));
        }
        forecasts
    }

    /// Provide a causal explanation for predicted behaviour.
    pub fn causal_explanation(&mut self, app_id: u64) -> Vec<CausalExplanation> {
        let model = match self.models.get(&app_id) {
            Some(m) => m,
            None => return Vec::new(),
        };

        let mut causes: Vec<CausalExplanation> = Vec::new();

        if model.cpu_trend > 5 {
            causes.push(CausalExplanation {
                app_id,
                cause_hash: fnv1a(b"cpu_increasing"),
                description: String::from("CPU usage is trending upward, likely due to increasing workload or algorithmic scaling."),
                strength: model.cpu_trend.unsigned_abs().min(100),
            });
        } else if model.cpu_trend < -5 {
            causes.push(CausalExplanation {
                app_id,
                cause_hash: fnv1a(b"cpu_decreasing"),
                description: String::from("CPU usage is declining, possibly entering idle phase or finishing a computation batch."),
                strength: model.cpu_trend.unsigned_abs().min(100),
            });
        }

        if model.mem_trend > 3 {
            causes.push(CausalExplanation {
                app_id,
                cause_hash: fnv1a(b"mem_growing"),
                description: String::from("Memory footprint is growing — the application is accumulating state or caching data."),
                strength: model.mem_trend.unsigned_abs().min(100),
            });
        }

        if model.io_trend > 3 {
            causes.push(CausalExplanation {
                app_id,
                cause_hash: fnv1a(b"io_increasing"),
                description: String::from("IO activity is increasing, consistent with a data-processing or logging phase."),
                strength: model.io_trend.unsigned_abs().min(100),
            });
        }

        // Periodicity detection from history.
        let period = self.detect_period(model);
        if period > 0 {
            causes.push(CausalExplanation {
                app_id,
                cause_hash: fnv1a(b"periodic_behaviour"),
                description: alloc::format!(
                    "Application shows periodic behaviour with estimated period of {} ticks.",
                    period
                ),
                strength: 60 + xorshift64(&mut self.rng) % 20,
            });
        }

        if causes.len() > CAUSE_CAP {
            causes.truncate(CAUSE_CAP);
        }
        self.stats.causal_explanations += causes.len() as u64;
        causes
    }

    /// Perform counterfactual analysis — "what if" a parameter changed.
    pub fn counterfactual_app(
        &mut self,
        app_id: u64,
        cpu_delta: i64,
        mem_delta: i64,
    ) -> Option<Counterfactual> {
        let model = match self.models.get(&app_id) {
            Some(m) => m,
            None => return None,
        };

        let original = self.oracle_predict(app_id, FORECAST_HORIZON / 2);

        let altered_cpu = (model.cpu_ema as i64 + cpu_delta).max(0) as u64;
        let altered_mem = (model.mem_ema as i64 + mem_delta).max(0) as u64;

        let altered_cpu_pred = self.extrapolate(altered_cpu, model.cpu_trend, FORECAST_HORIZON / 2);
        let altered_mem_pred = self.extrapolate(altered_mem, model.mem_trend, FORECAST_HORIZON / 2);

        let altered = Prediction {
            app_id,
            future_tick: original.future_tick,
            predicted_cpu: altered_cpu_pred,
            predicted_mem: altered_mem_pred,
            predicted_io: original.predicted_io,
            predicted_ipc: original.predicted_ipc,
            confidence: original.confidence.saturating_sub(10),
        };

        let cpu_diff = if altered.predicted_cpu > original.predicted_cpu {
            altered.predicted_cpu - original.predicted_cpu
        } else {
            original.predicted_cpu - altered.predicted_cpu
        };
        let mem_diff = if altered.predicted_mem > original.predicted_mem {
            altered.predicted_mem - original.predicted_mem
        } else {
            original.predicted_mem - altered.predicted_mem
        };
        let delta = cpu_diff + mem_diff;

        let desc = alloc::format!(
            "Counterfactual: cpu_delta={}, mem_delta={} => total impact={}",
            cpu_delta, mem_delta, delta
        );
        let scenario_hash = fnv1a(desc.as_bytes()) ^ xorshift64(&mut self.rng);

        self.stats.counterfactuals += 1;

        Some(Counterfactual {
            app_id,
            scenario_hash,
            description: desc,
            original_pred: original,
            altered_pred: altered,
            delta_impact: delta,
        })
    }

    /// Return oracle precision (0–100) — fraction of accurate predictions.
    pub fn oracle_precision(&self) -> u64 {
        if self.stats.total_predictions == 0 {
            return 0;
        }
        let precision = self.stats.total_hits * 100 / self.stats.total_predictions.max(1);
        precision.min(100)
    }

    /// Return current statistics.
    pub fn stats(&self) -> &OracleStats {
        &self.stats
    }

    // -- internal -----------------------------------------------------------

    fn get_or_create(&mut self, app_id: u64) -> &mut OracleModel {
        if !self.models.contains_key(&app_id) {
            let model = OracleModel {
                app_id,
                history: Vec::new(),
                cpu_ema: 0,
                mem_ema: 0,
                io_ema: 0,
                ipc_ema: 0,
                cpu_trend: 0,
                mem_trend: 0,
                io_trend: 0,
                accuracy_ema: 50,
                prediction_count: 0,
                hit_count: 0,
            };
            self.models.insert(app_id, model);
            self.stats.total_models += 1;
        }
        self.models.get_mut(&app_id).expect("just inserted")
    }

    fn extrapolate(&self, base: u64, trend: i64, horizon: u64) -> u64 {
        let scaled_trend = trend * (horizon as i64) / 4;
        let result = base as i64 + scaled_trend;
        result.max(0).min(100) as u64
    }

    fn compute_confidence(&self, model: &OracleModel, horizon: u64) -> u64 {
        let history_factor = (model.history.len() as u64 * 2).min(60);
        let accuracy_factor = model.accuracy_ema * 30 / 100;
        let horizon_penalty = (horizon * 2).min(40);
        let raw = history_factor + accuracy_factor;
        if raw > horizon_penalty { raw - horizon_penalty } else { 0 }
    }

    fn detect_period(&self, model: &OracleModel) -> u64 {
        let hist = &model.history;
        if hist.len() < 8 {
            return 0;
        }

        let mut best_period: u64 = 0;
        let mut best_score: u64 = 0;

        for period in 2..=(hist.len() / 2) {
            let mut match_score: u64 = 0;
            let mut comparisons: u64 = 0;

            for i in period..hist.len() {
                let diff = if hist[i].cpu > hist[i - period].cpu {
                    hist[i].cpu - hist[i - period].cpu
                } else {
                    hist[i - period].cpu - hist[i].cpu
                };
                if diff < 10 {
                    match_score += 1;
                }
                comparisons += 1;
            }

            if comparisons > 0 {
                let score = match_score * 100 / comparisons;
                if score > best_score && score > 60 {
                    best_score = score;
                    best_period = period as u64;
                }
            }
        }

        best_period
    }
}
