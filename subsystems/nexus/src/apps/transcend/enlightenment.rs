// SPDX-License-Identifier: GPL-2.0
//! # Apps Enlightenment — Ultimate Understanding of App Behavior
//!
//! Represents the highest level of comprehension the kernel can achieve
//! about application behaviour. The engine progresses through discrete
//! enlightenment stages — from *Awareness* through *Comprehension*,
//! *Mastery*, *Transcendence*, and finally *Enlightenment* — as it
//! accumulates observations and proves its predictive power.
//!
//! Each application is tracked independently, so the system can report
//! per-app enlightenment levels while also computing a global
//! enlightenment metric. Purpose discovery analyses why an app exists
//! in the workload, and behaviour essence distils the core nature of
//! an application to a compact fingerprint.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x00000100000001B3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 9;
const MAX_TRACKED_APPS: usize = 4096;
const AWARENESS_THRESHOLD: u64 = 5;
const COMPREHENSION_THRESHOLD: u64 = 25;
const MASTERY_THRESHOLD: u64 = 60;
const TRANSCENDENCE_THRESHOLD: u64 = 85;
const ENLIGHTENMENT_THRESHOLD: u64 = 95;
const PREDICTION_HISTORY: usize = 32;
const PURPOSE_MIN_OBSERVATIONS: u64 = 10;
const ESSENCE_DIMENSIONS: usize = 6;

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

/// Discrete enlightenment stages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnlightenmentLevel {
    /// Minimal contact with the application.
    Ignorance = 0,
    /// The system is aware the app exists.
    Awareness = 1,
    /// Behavioural patterns are understood.
    Comprehension = 2,
    /// The system can predict and control.
    Mastery = 3,
    /// Understanding surpasses traditional metrics.
    Transcendence = 4,
    /// Perfect, effortless understanding.
    Enlightenment = 5,
}

/// Per-application enlightenment profile.
#[derive(Clone, Debug)]
pub struct AppEnlightenmentProfile {
    pub app_id: u64,
    pub name: String,
    pub level: EnlightenmentLevel,
    pub understanding_score: u64,
    pub observation_count: u64,
    pub prediction_accuracy: u64,
    pub predictions: VecDeque<u64>,
    pub actuals: VecDeque<u64>,
    pub essence_fingerprint: Vec<u64>,
    pub purpose_hash: u64,
    pub purpose_label: String,
    pub last_tick: u64,
}

/// A compact behavioural essence descriptor.
#[derive(Clone, Debug)]
pub struct BehaviorEssence {
    pub app_id: u64,
    pub dimensions: Vec<u64>,
    pub fingerprint_hash: u64,
    pub stability: u64,
}

/// Result of a purpose discovery analysis.
#[derive(Clone, Debug)]
pub struct PurposeDiscovery {
    pub app_id: u64,
    pub purpose_hash: u64,
    pub purpose_label: String,
    pub confidence: u64,
    pub supporting_observations: u64,
}

/// Transcendent insight about an application.
#[derive(Clone, Debug)]
pub struct TranscendentInsight {
    pub app_id: u64,
    pub insight_hash: u64,
    pub description: String,
    pub depth: u64,
    pub novelty: u64,
}

/// Running statistics for the enlightenment engine.
#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct EnlightenmentStats {
    pub total_apps: u64,
    pub ignorance_count: u64,
    pub awareness_count: u64,
    pub comprehension_count: u64,
    pub mastery_count: u64,
    pub transcendence_count: u64,
    pub enlightenment_count: u64,
    pub global_understanding: u64,
    pub avg_prediction_accuracy: u64,
    pub purposes_discovered: u64,
}

// ---------------------------------------------------------------------------
// AppsEnlightenment
// ---------------------------------------------------------------------------

/// Engine tracking the ultimate understanding of application behaviour.
pub struct AppsEnlightenment {
    profiles: BTreeMap<u64, AppEnlightenmentProfile>,
    insights: Vec<TranscendentInsight>,
    stats: EnlightenmentStats,
    rng: u64,
    tick: u64,
}

impl AppsEnlightenment {
    /// Create a new enlightenment engine.
    pub fn new(seed: u64) -> Self {
        Self {
            profiles: BTreeMap::new(),
            insights: Vec::new(),
            stats: EnlightenmentStats::default(),
            rng: seed | 1,
            tick: 0,
        }
    }

    // -- public API ---------------------------------------------------------

    /// Report the current enlightenment level for a specific application.
    #[inline]
    pub fn app_enlightenment(&self, app_id: u64) -> EnlightenmentLevel {
        self.profiles
            .get(&app_id)
            .map(|p| p.level)
            .unwrap_or(EnlightenmentLevel::Ignorance)
    }

    /// Provide a deep observation about an app, advancing enlightenment.
    ///
    /// `cpu`, `mem`, `io`, `ipc` are resource usage dimensions (0–100).
    /// `predicted_cpu` is the system's predicted CPU for this tick.
    pub fn deep_understanding(
        &mut self,
        app_id: u64,
        name: &str,
        cpu: u64,
        mem: u64,
        io: u64,
        ipc: u64,
        predicted_cpu: u64,
    ) {
        self.tick += 1;

        let profile = self
            .profiles
            .entry(app_id)
            .or_insert_with(|| {
                self.stats.total_apps += 1;
                AppEnlightenmentProfile {
                    app_id,
                    name: String::from(name),
                    level: EnlightenmentLevel::Ignorance,
                    understanding_score: 0,
                    observation_count: 0,
                    prediction_accuracy: 0,
                    predictions: VecDeque::new(),
                    actuals: VecDeque::new(),
                    essence_fingerprint: Vec::new(),
                    purpose_hash: 0,
                    purpose_label: String::new(),
                    last_tick: 0,
                }
            });

        profile.observation_count += 1;
        profile.last_tick = self.tick;

        // Track prediction accuracy
        if profile.predictions.len() >= PREDICTION_HISTORY {
            profile.predictions.pop_front().unwrap();
            profile.actuals.pop_front().unwrap();
        }
        profile.predictions.push(predicted_cpu);
        profile.actuals.push(cpu);

        let accuracy = self.compute_accuracy(&profile.predictions, &profile.actuals);
        profile.prediction_accuracy = ema_update(profile.prediction_accuracy, accuracy);

        // Update essence fingerprint
        let new_essence = self.compute_essence(cpu, mem, io, ipc);
        if profile.essence_fingerprint.is_empty() {
            profile.essence_fingerprint = new_essence;
        } else {
            for (i, dim) in new_essence.iter().enumerate() {
                if i < profile.essence_fingerprint.len() {
                    profile.essence_fingerprint[i] =
                        ema_update(profile.essence_fingerprint[i], *dim);
                }
            }
        }

        // Compute understanding score
        let obs_factor = (profile.observation_count * 2).min(100);
        let pred_factor = profile.prediction_accuracy;
        profile.understanding_score = (obs_factor + pred_factor) / 2;

        // Advance enlightenment level
        let new_level = self.score_to_level(profile.understanding_score);
        profile.level = new_level;

        self.refresh_stats();
    }

    /// Discover the purpose of an application in the workload.
    ///
    /// Analyses behavioural patterns to infer *why* an app exists.
    pub fn purpose_discovery(&mut self, app_id: u64) -> Option<PurposeDiscovery> {
        let profile = self.profiles.get(&app_id)?;
        if profile.observation_count < PURPOSE_MIN_OBSERVATIONS {
            return None;
        }

        let dims = &profile.essence_fingerprint;
        if dims.is_empty() {
            return None;
        }

        let (purpose_label, purpose_hash) = self.infer_purpose(dims);
        let confidence = (profile.understanding_score * profile.observation_count.min(50)) / 50;

        // Store purpose in profile
        if let Some(p) = self.profiles.get_mut(&app_id) {
            p.purpose_hash = purpose_hash;
            p.purpose_label = purpose_label.clone();
            self.stats.purposes_discovered = self
                .profiles
                .values()
                .filter(|p| p.purpose_hash != 0)
                .count() as u64;
        }

        Some(PurposeDiscovery {
            app_id,
            purpose_hash,
            purpose_label,
            confidence: confidence.min(100),
            supporting_observations: profile.observation_count,
        })
    }

    /// Extract the behavioural essence of an application.
    pub fn behavior_essence(&self, app_id: u64) -> Option<BehaviorEssence> {
        let profile = self.profiles.get(&app_id)?;
        if profile.essence_fingerprint.is_empty() {
            return None;
        }

        let fp_hash = fnv1a(
            &profile
                .essence_fingerprint
                .iter()
                .flat_map(|d| d.to_le_bytes())
                .collect::<Vec<u8>>(),
        );

        let stability = if profile.observation_count > 10 {
            profile.prediction_accuracy
        } else {
            profile.observation_count * 10
        };

        Some(BehaviorEssence {
            app_id,
            dimensions: profile.essence_fingerprint.clone(),
            fingerprint_hash: fp_hash,
            stability: stability.min(100),
        })
    }

    /// Perform enlightened management: produce allocation hints for an app.
    ///
    /// Returns a suggested adjustment factor (100 = no change, >100 = more,
    /// <100 = less).
    pub fn enlightened_management(&self, app_id: u64) -> u64 {
        let profile = match self.profiles.get(&app_id) {
            Some(p) => p,
            None => return 100,
        };

        let level_bonus = match profile.level {
            EnlightenmentLevel::Ignorance => 0,
            EnlightenmentLevel::Awareness => 2,
            EnlightenmentLevel::Comprehension => 5,
            EnlightenmentLevel::Mastery => 10,
            EnlightenmentLevel::Transcendence => 15,
            EnlightenmentLevel::Enlightenment => 20,
        };

        let accuracy_factor = if profile.prediction_accuracy > 80 {
            profile.prediction_accuracy - 80
        } else {
            0
        };

        let dims = &profile.essence_fingerprint;
        let demand_signal = if dims.len() >= 4 {
            (dims[0] + dims[1]) / 2
        } else {
            50
        };

        let adjustment = if demand_signal > 70 {
            100 + level_bonus + accuracy_factor / 2
        } else if demand_signal < 30 {
            let reduce = level_bonus + accuracy_factor / 2;
            100u64.saturating_sub(reduce)
        } else {
            100
        };

        adjustment.max(50).min(200)
    }

    /// Generate transcendent insights about an application.
    pub fn transcendent_insight(&mut self, app_id: u64) -> Option<TranscendentInsight> {
        let profile = self.profiles.get(&app_id)?;
        if profile.level < EnlightenmentLevel::Mastery {
            return None;
        }

        let depth = match profile.level {
            EnlightenmentLevel::Mastery => 1,
            EnlightenmentLevel::Transcendence => 2,
            EnlightenmentLevel::Enlightenment => 3,
            _ => 0,
        };

        let noise = xorshift64(&mut self.rng) % 20;
        let novelty = ((profile.understanding_score + noise) / 2).min(100);

        let desc = if profile.prediction_accuracy > 90 {
            String::from("near_perfect_prediction")
        } else if profile.observation_count > 100 {
            String::from("deeply_observed")
        } else {
            String::from("emerging_mastery")
        };

        let insight_hash = fnv1a(desc.as_bytes())
            ^ fnv1a(&app_id.to_le_bytes())
            ^ fnv1a(&self.tick.to_le_bytes());

        let insight = TranscendentInsight {
            app_id,
            insight_hash,
            description: desc,
            depth,
            novelty,
        };
        self.insights.push(insight.clone());
        Some(insight)
    }

    /// Return a snapshot of current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &EnlightenmentStats {
        &self.stats
    }

    // -- internal -----------------------------------------------------------

    fn compute_accuracy(&self, predictions: &[u64], actuals: &[u64]) -> u64 {
        if predictions.is_empty() || actuals.is_empty() {
            return 0;
        }
        let len = predictions.len().min(actuals.len());
        let mut total_error: u64 = 0;
        for i in 0..len {
            let diff = if predictions[i] > actuals[i] {
                predictions[i] - actuals[i]
            } else {
                actuals[i] - predictions[i]
            };
            total_error += diff;
        }
        let avg_error = total_error / len as u64;
        100u64.saturating_sub(avg_error)
    }

    fn compute_essence(&mut self, cpu: u64, mem: u64, io: u64, ipc: u64) -> Vec<u64> {
        let noise = xorshift64(&mut self.rng) % 5;
        let burstiness = if cpu > 80 || io > 80 { 80 + noise } else { 20 + noise };
        let sociality = if ipc > 50 { 70 + noise } else { 15 + noise };
        alloc::vec![cpu, mem, io, ipc, burstiness, sociality]
    }

    fn infer_purpose(&self, dims: &[u64]) -> (String, u64) {
        if dims.is_empty() {
            return (String::from("unknown"), 0);
        }
        let cpu = dims.first().copied().unwrap_or(0);
        let mem = if dims.len() > 1 { dims[1] } else { 0 };
        let io = if dims.len() > 2 { dims[2] } else { 0 };
        let ipc = if dims.len() > 3 { dims[3] } else { 0 };

        let label = if cpu > 70 && mem > 50 {
            "computation_engine"
        } else if io > 60 && cpu < 40 {
            "data_pipeline"
        } else if ipc > 60 {
            "coordination_hub"
        } else if mem > 70 && cpu < 30 {
            "cache_reservoir"
        } else if cpu < 20 && mem < 20 && io < 20 {
            "idle_sentinel"
        } else {
            "general_worker"
        };
        let hash = fnv1a(label.as_bytes());
        (String::from(label), hash)
    }

    fn score_to_level(&self, score: u64) -> EnlightenmentLevel {
        if score >= ENLIGHTENMENT_THRESHOLD {
            EnlightenmentLevel::Enlightenment
        } else if score >= TRANSCENDENCE_THRESHOLD {
            EnlightenmentLevel::Transcendence
        } else if score >= MASTERY_THRESHOLD {
            EnlightenmentLevel::Mastery
        } else if score >= COMPREHENSION_THRESHOLD {
            EnlightenmentLevel::Comprehension
        } else if score >= AWARENESS_THRESHOLD {
            EnlightenmentLevel::Awareness
        } else {
            EnlightenmentLevel::Ignorance
        }
    }

    fn refresh_stats(&mut self) {
        let mut ig = 0u64;
        let mut aw = 0u64;
        let mut co = 0u64;
        let mut ma = 0u64;
        let mut tr = 0u64;
        let mut en = 0u64;
        let mut total_understanding: u64 = 0;
        let mut total_accuracy: u64 = 0;
        let count = self.profiles.len() as u64;

        for p in self.profiles.values() {
            match p.level {
                EnlightenmentLevel::Ignorance => ig += 1,
                EnlightenmentLevel::Awareness => aw += 1,
                EnlightenmentLevel::Comprehension => co += 1,
                EnlightenmentLevel::Mastery => ma += 1,
                EnlightenmentLevel::Transcendence => tr += 1,
                EnlightenmentLevel::Enlightenment => en += 1,
            }
            total_understanding += p.understanding_score;
            total_accuracy += p.prediction_accuracy;
        }

        self.stats.ignorance_count = ig;
        self.stats.awareness_count = aw;
        self.stats.comprehension_count = co;
        self.stats.mastery_count = ma;
        self.stats.transcendence_count = tr;
        self.stats.enlightenment_count = en;
        self.stats.global_understanding = if count > 0 {
            total_understanding / count
        } else {
            0
        };
        self.stats.avg_prediction_accuracy = if count > 0 {
            total_accuracy / count
        } else {
            0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_engine() {
        let e = AppsEnlightenment::new(42);
        assert_eq!(e.stats().total_apps, 0);
        assert_eq!(e.app_enlightenment(1), EnlightenmentLevel::Ignorance);
    }

    #[test]
    fn test_deep_understanding_advances_level() {
        let mut e = AppsEnlightenment::new(42);
        for i in 0..30 {
            e.deep_understanding(1, "test_app", 50, 40, 30, 20, 50);
        }
        let level = e.app_enlightenment(1);
        assert!(level >= EnlightenmentLevel::Awareness);
    }

    #[test]
    fn test_behavior_essence() {
        let mut e = AppsEnlightenment::new(42);
        e.deep_understanding(1, "test_app", 60, 40, 30, 20, 55);
        let essence = e.behavior_essence(1);
        assert!(essence.is_some());
        let ess = essence.unwrap();
        assert_eq!(ess.dimensions.len(), ESSENCE_DIMENSIONS);
    }

    #[test]
    fn test_purpose_discovery_needs_observations() {
        let mut e = AppsEnlightenment::new(42);
        e.deep_understanding(1, "test_app", 50, 40, 30, 20, 50);
        assert!(e.purpose_discovery(1).is_none());
    }

    #[test]
    fn test_purpose_discovery_with_observations() {
        let mut e = AppsEnlightenment::new(42);
        for _ in 0..15 {
            e.deep_understanding(1, "compute_app", 85, 60, 20, 10, 80);
        }
        let purpose = e.purpose_discovery(1);
        assert!(purpose.is_some());
    }

    #[test]
    fn test_enlightened_management_default() {
        let e = AppsEnlightenment::new(42);
        assert_eq!(e.enlightened_management(999), 100);
    }

    #[test]
    fn test_transcendent_insight_requires_mastery() {
        let mut e = AppsEnlightenment::new(42);
        e.deep_understanding(1, "test_app", 50, 40, 30, 20, 50);
        assert!(e.transcendent_insight(1).is_none());
    }
}
