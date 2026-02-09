// SPDX-License-Identifier: GPL-2.0
//! # Apps Breakthrough Detector — Detects Genuine Breakthroughs in App Understanding
//!
//! Monitors the research pipeline for discoveries that represent genuine
//! breakthroughs in app understanding — not incremental improvements but
//! qualitative leaps. Uses novelty scoring, magnitude assessment, and impact
//! forecasting to separate breakthroughs from routine findings. Maintains
//! a catalog of all detected breakthroughs and tracks breakthrough frequency
//! to monitor the health of the research pipeline.
//!
//! The engine that recognizes when something truly new has been discovered.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_CATALOG: usize = 256;
const MAX_CANDIDATES: usize = 512;
const BREAKTHROUGH_THRESHOLD: f32 = 0.75;
const NOVELTY_WEIGHT: f32 = 0.35;
const MAGNITUDE_WEIGHT: f32 = 0.35;
const IMPACT_WEIGHT: f32 = 0.30;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const IMPACT_HORIZON: u64 = 1000;
const MAGNITUDE_SMALL: f32 = 0.30;
const MAGNITUDE_LARGE: f32 = 0.70;
const FREQUENCY_WINDOW: u64 = 5000;
const MAX_FREQUENCY_HISTORY: usize = 128;
const DECAY_RATE: f32 = 0.995;

// ============================================================================
// HELPERS
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

fn abs_f32(x: f32) -> f32 {
    if x < 0.0 { -x } else { x }
}

// ============================================================================
// TYPES
// ============================================================================

/// Magnitude classification for a potential breakthrough.
#[derive(Clone, Copy, PartialEq)]
pub enum BreakthroughMagnitude {
    Incremental,
    Significant,
    Major,
    Transformative,
}

/// A candidate finding being evaluated for breakthrough status.
#[derive(Clone)]
pub struct BreakthroughCandidate {
    pub candidate_id: u64,
    pub title: String,
    pub novelty: f32,
    pub magnitude: f32,
    pub impact_estimate: f32,
    pub composite_score: f32,
    pub is_breakthrough: bool,
    pub submitted_tick: u64,
}

/// A confirmed breakthrough in the catalog.
#[derive(Clone)]
pub struct BreakthroughEntry {
    pub breakthrough_id: u64,
    pub title: String,
    pub magnitude_class: BreakthroughMagnitude,
    pub novelty_score: f32,
    pub magnitude_score: f32,
    pub impact_score: f32,
    pub composite_score: f32,
    pub confirmed_tick: u64,
    pub impact_realized: f32,
    pub citations: u32,
}

/// Magnitude assessment result.
#[derive(Clone)]
pub struct MagnitudeAssessment {
    pub candidate_id: u64,
    pub raw_magnitude: f32,
    pub relative_magnitude: f32,
    pub classification: BreakthroughMagnitude,
    pub percentile: f32,
}

/// Impact forecast for a potential breakthrough.
#[derive(Clone)]
pub struct ImpactForecast {
    pub candidate_id: u64,
    pub short_term: f32,
    pub medium_term: f32,
    pub long_term: f32,
    pub aggregate: f32,
    pub confidence: f32,
}

/// Breakthrough frequency report.
#[derive(Clone)]
pub struct FrequencyReport {
    pub total_breakthroughs: usize,
    pub window_breakthroughs: usize,
    pub rate_per_1k_ticks: f32,
    pub ema_frequency: f32,
    pub trend: FrequencyTrend,
}

/// Trend direction for breakthrough frequency.
#[derive(Clone, Copy, PartialEq)]
pub enum FrequencyTrend {
    Increasing,
    Stable,
    Decreasing,
    Stalled,
}

/// Engine-level stats.
#[derive(Clone)]
#[repr(align(64))]
pub struct BreakthroughStats {
    pub candidates_evaluated: u64,
    pub breakthroughs_confirmed: u64,
    pub ema_novelty: f32,
    pub ema_magnitude: f32,
    pub ema_impact: f32,
    pub ema_composite: f32,
    pub false_positive_rate: f32,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Detector for genuine breakthroughs in app understanding.
pub struct AppsBreakthroughDetector {
    candidates: BTreeMap<u64, BreakthroughCandidate>,
    catalog: BTreeMap<u64, BreakthroughEntry>,
    frequency_ticks: VecDeque<u64>,
    baseline_scores: VecDeque<f32>,
    stats: BreakthroughStats,
    rng_state: u64,
    tick: u64,
}

impl AppsBreakthroughDetector {
    /// Create a new breakthrough detector.
    pub fn new(seed: u64) -> Self {
        Self {
            candidates: BTreeMap::new(),
            catalog: BTreeMap::new(),
            frequency_ticks: VecDeque::new(),
            baseline_scores: VecDeque::new(),
            stats: BreakthroughStats {
                candidates_evaluated: 0,
                breakthroughs_confirmed: 0,
                ema_novelty: 0.0,
                ema_magnitude: 0.0,
                ema_impact: 0.0,
                ema_composite: 0.0,
                false_positive_rate: 0.0,
            },
            rng_state: seed ^ 0xb2d58c71ea03f694,
            tick: 0,
        }
    }

    // ── Primary API ────────────────────────────────────────────────────

    /// Evaluate a finding for potential breakthrough status.
    #[inline]
    pub fn detect_breakthrough(
        &mut self,
        title: &str,
        novelty: f32,
        magnitude: f32,
        impact_est: f32,
    ) -> BreakthroughCandidate {
        self.tick += 1;
        self.stats.candidates_evaluated += 1;

        let id = fnv1a_hash(title.as_bytes()) ^ self.tick;
        let n = novelty.min(1.0).max(0.0);
        let m = magnitude.min(1.0).max(0.0);
        let imp = impact_est.min(1.0).max(0.0);

        let composite = n * NOVELTY_WEIGHT + m * MAGNITUDE_WEIGHT + imp * IMPACT_WEIGHT;
        let is_bt = composite >= BREAKTHROUGH_THRESHOLD;

        // Track baseline for relative comparisons
        self.baseline_scores.push_back(composite);
        if self.baseline_scores.len() > MAX_CANDIDATES {
            self.baseline_scores.pop_front();
        }

        let candidate = BreakthroughCandidate {
            candidate_id: id,
            title: String::from(title),
            novelty: n,
            magnitude: m,
            impact_estimate: imp,
            composite_score: composite,
            is_breakthrough: is_bt,
            submitted_tick: self.tick,
        };

        if is_bt {
            self.confirm_breakthrough(&candidate);
        }

        // Update EMAs
        self.stats.ema_novelty = EMA_ALPHA * n + (1.0 - EMA_ALPHA) * self.stats.ema_novelty;
        self.stats.ema_magnitude = EMA_ALPHA * m + (1.0 - EMA_ALPHA) * self.stats.ema_magnitude;
        self.stats.ema_impact = EMA_ALPHA * imp + (1.0 - EMA_ALPHA) * self.stats.ema_impact;
        self.stats.ema_composite =
            EMA_ALPHA * composite + (1.0 - EMA_ALPHA) * self.stats.ema_composite;

        if self.candidates.len() >= MAX_CANDIDATES {
            if let Some(oldest) = self.candidates.keys().next().cloned() {
                self.candidates.remove(&oldest);
            }
        }
        self.candidates.insert(id, candidate.clone());
        candidate
    }

    /// Assess the magnitude of a breakthrough candidate.
    pub fn magnitude_assessment(&self, candidate_id: u64) -> Option<MagnitudeAssessment> {
        let candidate = self.candidates.get(&candidate_id)?;
        let raw = candidate.magnitude;

        // Compute relative magnitude against baseline
        let baseline_mean = if self.baseline_scores.is_empty() {
            0.5
        } else {
            let s: f32 = self.baseline_scores.iter().sum();
            s / self.baseline_scores.len() as f32
        };
        let relative = if baseline_mean > 0.01 {
            raw / baseline_mean
        } else {
            raw * 2.0
        };

        // Percentile calculation
        let mut below_count = 0u32;
        for &bs in &self.baseline_scores {
            if bs < candidate.composite_score {
                below_count += 1;
            }
        }
        let percentile = if self.baseline_scores.is_empty() {
            0.5
        } else {
            below_count as f32 / self.baseline_scores.len() as f32
        };

        let classification = if raw >= MAGNITUDE_LARGE && relative >= 2.0 {
            BreakthroughMagnitude::Transformative
        } else if raw >= MAGNITUDE_LARGE {
            BreakthroughMagnitude::Major
        } else if raw >= MAGNITUDE_SMALL {
            BreakthroughMagnitude::Significant
        } else {
            BreakthroughMagnitude::Incremental
        };

        Some(MagnitudeAssessment {
            candidate_id,
            raw_magnitude: raw,
            relative_magnitude: relative,
            classification,
            percentile,
        })
    }

    /// Compute novelty score relative to existing knowledge.
    pub fn novelty_score(&self, candidate_id: u64) -> Option<f32> {
        let candidate = self.candidates.get(&candidate_id)?;

        // Compare against all catalog entries for uniqueness
        let mut min_distance = f32::MAX;
        let candidate_hash = fnv1a_hash(candidate.title.as_bytes());

        for entry in self.catalog.values() {
            let entry_hash = fnv1a_hash(entry.title.as_bytes());
            let hash_diff = (candidate_hash ^ entry_hash) % 10000;
            let distance = hash_diff as f32 / 10000.0;
            if distance < min_distance {
                min_distance = distance;
            }
        }

        if self.catalog.is_empty() {
            Some(candidate.novelty)
        } else {
            // Higher distance from existing = more novel
            Some((candidate.novelty * 0.6 + min_distance * 0.4).min(1.0))
        }
    }

    /// Forecast the impact of a breakthrough candidate.
    pub fn impact_forecast(&self, candidate_id: u64) -> Option<ImpactForecast> {
        let candidate = self.candidates.get(&candidate_id)?;

        let base = candidate.impact_estimate;
        let novelty_multiplier = 1.0 + candidate.novelty * 0.5;
        let magnitude_multiplier = 1.0 + candidate.magnitude * 0.3;

        let short_term = (base * 0.4 * magnitude_multiplier).min(1.0);
        let medium_term = (base * 0.7 * novelty_multiplier).min(1.0);
        let long_term = (base * novelty_multiplier * magnitude_multiplier).min(1.0);
        let aggregate = short_term * 0.2 + medium_term * 0.3 + long_term * 0.5;

        // Confidence based on how much data we have
        let data_factor = (self.baseline_scores.len() as f32 / 100.0).min(1.0);
        let confidence = 0.3 + data_factor * 0.5;

        Some(ImpactForecast {
            candidate_id,
            short_term,
            medium_term,
            long_term,
            aggregate,
            confidence,
        })
    }

    /// Get the full breakthrough catalog.
    #[inline(always)]
    pub fn breakthrough_catalog(&self) -> Vec<BreakthroughEntry> {
        self.catalog.values().cloned().collect()
    }

    /// Report on breakthrough frequency and trends.
    pub fn breakthrough_frequency(&self) -> FrequencyReport {
        let total = self.catalog.len();
        let current = self.tick;

        // Count breakthroughs in recent window
        let window_start = current.saturating_sub(FREQUENCY_WINDOW);
        let window_count = self
            .frequency_ticks
            .iter()
            .filter(|&&t| t >= window_start)
            .count();

        let rate = if current > 0 {
            (total as f32 / current as f32) * 1000.0
        } else {
            0.0
        };

        // Trend detection: compare recent half vs older half
        let mid = self.frequency_ticks.len() / 2;
        let trend = if self.frequency_ticks.len() < 4 {
            FrequencyTrend::Stable
        } else {
            let recent_count = self.frequency_ticks.len() - mid;
            let older_count = mid;
            let ratio = recent_count as f32 / older_count.max(1) as f32;
            if ratio > 1.3 {
                FrequencyTrend::Increasing
            } else if ratio < 0.7 {
                FrequencyTrend::Decreasing
            } else if window_count == 0 && current > FREQUENCY_WINDOW {
                FrequencyTrend::Stalled
            } else {
                FrequencyTrend::Stable
            }
        };

        FrequencyReport {
            total_breakthroughs: total,
            window_breakthroughs: window_count,
            rate_per_1k_ticks: rate,
            ema_frequency: self.stats.ema_composite,
            trend,
        }
    }

    /// Return engine stats.
    #[inline(always)]
    pub fn stats(&self) -> &BreakthroughStats {
        &self.stats
    }

    // ── Internal Helpers ───────────────────────────────────────────────

    fn confirm_breakthrough(&mut self, candidate: &BreakthroughCandidate) {
        self.stats.breakthroughs_confirmed += 1;

        let mag_class = if candidate.magnitude >= MAGNITUDE_LARGE && candidate.novelty >= MAGNITUDE_LARGE {
            BreakthroughMagnitude::Transformative
        } else if candidate.magnitude >= MAGNITUDE_LARGE {
            BreakthroughMagnitude::Major
        } else if candidate.magnitude >= MAGNITUDE_SMALL {
            BreakthroughMagnitude::Significant
        } else {
            BreakthroughMagnitude::Incremental
        };

        let entry = BreakthroughEntry {
            breakthrough_id: candidate.candidate_id,
            title: candidate.title.clone(),
            magnitude_class: mag_class,
            novelty_score: candidate.novelty,
            magnitude_score: candidate.magnitude,
            impact_score: candidate.impact_estimate,
            composite_score: candidate.composite_score,
            confirmed_tick: self.tick,
            impact_realized: 0.0,
            citations: 0,
        };

        self.frequency_ticks.push_back(self.tick);
        if self.frequency_ticks.len() > MAX_FREQUENCY_HISTORY {
            self.frequency_ticks.pop_front();
        }

        if self.catalog.len() >= MAX_CATALOG {
            // Evict lowest-impact entry
            let mut min_id = 0u64;
            let mut min_score = f32::MAX;
            for (bid, b) in self.catalog.iter() {
                if b.composite_score < min_score {
                    min_score = b.composite_score;
                    min_id = *bid;
                }
            }
            self.catalog.remove(&min_id);
        }
        self.catalog.insert(candidate.candidate_id, entry);
    }
}
