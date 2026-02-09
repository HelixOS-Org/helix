// SPDX-License-Identifier: GPL-2.0
//! # Bridge Breakthrough Detector — Identifying Genuine Discoveries
//!
//! Not every positive result is a breakthrough. This module implements
//! rigorous breakthrough detection: a finding qualifies only if it improves
//! performance by more than 10% over the current baseline, OR reveals a
//! completely novel optimization dimension previously unexplored. The
//! detector tracks magnitude, novelty, and downstream impact, maintaining
//! a historical record to compute breakthrough rates and trends.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_BREAKTHROUGHS: usize = 512;
const MAX_CANDIDATES: usize = 1024;
const MAX_HISTORY: usize = 2048;
const BREAKTHROUGH_THRESHOLD: f32 = 0.10; // 10% improvement
const HIGH_NOVELTY_THRESHOLD: f32 = 0.80;
const IMPACT_DECAY: f32 = 0.98;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MAGNITUDE_SCALE: f32 = 100.0;
const MIN_EVIDENCE_COUNT: usize = 3;
const PARADIGM_SHIFT_THRESHOLD: f32 = 0.30; // 30% improvement
const RATE_WINDOW: usize = 100;

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

fn xorshift_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 10000) as f32 / 10000.0
}

fn sqrt_approx(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }
    let mut g = v * 0.5;
    for _ in 0..6 {
        g = 0.5 * (g + v / g);
    }
    g
}

fn abs_f32(v: f32) -> f32 {
    if v < 0.0 { -v } else { v }
}

// ============================================================================
// TYPES
// ============================================================================

/// A confirmed breakthrough.
#[derive(Clone)]
pub struct Breakthrough {
    pub id: u64,
    pub discovery: String,
    pub magnitude: f32,
    pub novelty: f32,
    pub impact: f32,
    pub baseline_before: f32,
    pub performance_after: f32,
    pub evidence_count: usize,
    pub tick: u64,
    pub category: BreakthroughCategory,
}

/// Category of breakthrough.
#[derive(Clone, Copy, PartialEq)]
pub enum BreakthroughCategory {
    PerformanceGain,
    NovelDimension,
    ParadigmShift,
    IncrementalMajor,
}

/// A candidate finding being evaluated for breakthrough status.
#[derive(Clone)]
struct Candidate {
    id: u64,
    description: String,
    baseline: f32,
    measured: f32,
    improvement_ratio: f32,
    novelty_score: f32,
    evidence_pieces: Vec<f32>,
    submit_tick: u64,
    evaluated: bool,
}

/// Detection record for history tracking.
#[derive(Clone)]
struct DetectionRecord {
    candidate_id: u64,
    was_breakthrough: bool,
    magnitude: f32,
    tick: u64,
}

/// Breakthrough detection statistics.
#[derive(Clone)]
pub struct BreakthroughStats {
    pub total_candidates: u64,
    pub confirmed_breakthroughs: u64,
    pub false_alarms: u64,
    pub avg_magnitude_ema: f32,
    pub avg_novelty_ema: f32,
    pub avg_impact_ema: f32,
    pub breakthrough_rate_ema: f32,
    pub largest_breakthrough: f32,
    pub paradigm_shifts: u64,
    pub ticks_since_last: u64,
}

/// Known optimization dimensions for novelty assessment.
#[derive(Clone)]
struct KnownDimension {
    name: String,
    discovery_tick: u64,
    best_performance: f32,
    exploration_count: u64,
}

// ============================================================================
// BRIDGE BREAKTHROUGH DETECTOR
// ============================================================================

/// Detects genuine breakthroughs in bridge research.
pub struct BridgeBreakthroughDetector {
    breakthroughs: BTreeMap<u64, Breakthrough>,
    candidates: BTreeMap<u64, Candidate>,
    history: Vec<DetectionRecord>,
    known_dimensions: BTreeMap<u64, KnownDimension>,
    stats: BreakthroughStats,
    rng_state: u64,
    tick: u64,
    current_baseline: f32,
}

impl BridgeBreakthroughDetector {
    /// Create a new breakthrough detector.
    pub fn new(seed: u64, initial_baseline: f32) -> Self {
        Self {
            breakthroughs: BTreeMap::new(),
            candidates: BTreeMap::new(),
            history: Vec::new(),
            known_dimensions: BTreeMap::new(),
            stats: BreakthroughStats {
                total_candidates: 0,
                confirmed_breakthroughs: 0,
                false_alarms: 0,
                avg_magnitude_ema: 0.0,
                avg_novelty_ema: 0.0,
                avg_impact_ema: 0.0,
                breakthrough_rate_ema: 0.0,
                largest_breakthrough: 0.0,
                paradigm_shifts: 0,
                ticks_since_last: 0,
            },
            rng_state: seed ^ 0xB4EA100400A001,
            tick: 0,
            current_baseline: initial_baseline,
        }
    }

    /// Register a known optimization dimension.
    pub fn register_dimension(&mut self, name: &str) {
        let id = fnv1a_hash(name.as_bytes());
        self.known_dimensions.insert(
            id,
            KnownDimension {
                name: String::from(name),
                discovery_tick: self.tick,
                best_performance: self.current_baseline,
                exploration_count: 0,
            },
        );
    }

    /// Submit a candidate finding for breakthrough evaluation.
    pub fn submit_candidate(
        &mut self,
        description: &str,
        baseline: f32,
        measured: f32,
    ) -> u64 {
        self.tick += 1;
        self.stats.total_candidates += 1;

        let improvement = if baseline > 1e-9 {
            (measured - baseline) / baseline
        } else {
            0.0
        };

        let id = fnv1a_hash(description.as_bytes()) ^ self.tick;
        if self.candidates.len() >= MAX_CANDIDATES {
            self.evict_oldest_candidate();
        }

        self.candidates.insert(
            id,
            Candidate {
                id,
                description: String::from(description),
                baseline,
                measured,
                improvement_ratio: improvement,
                novelty_score: 0.0,
                evidence_pieces: Vec::new(),
                submit_tick: self.tick,
                evaluated: false,
            },
        );
        id
    }

    /// Add evidence to a candidate.
    pub fn add_evidence(&mut self, candidate_id: u64, measurement: f32) {
        if let Some(c) = self.candidates.get_mut(&candidate_id) {
            c.evidence_pieces.push(measurement);
        }
    }

    /// Evaluate a candidate: is it a genuine breakthrough?
    pub fn detect_breakthrough(&mut self, candidate_id: u64) -> Option<Breakthrough> {
        self.tick += 1;
        self.stats.ticks_since_last += 1;

        let (desc, improvement, baseline, measured, evidence_count, novelty) = {
            let c = self.candidates.get(&candidate_id)?;
            if c.evaluated {
                return None;
            }
            let n = self.novelty_assessment_internal(&c.description);
            (
                c.description.clone(),
                c.improvement_ratio,
                c.baseline,
                c.measured,
                c.evidence_pieces.len(),
                n,
            )
        };

        // Mark evaluated
        if let Some(c) = self.candidates.get_mut(&candidate_id) {
            c.evaluated = true;
            c.novelty_score = novelty;
        }

        let magnitude = self.breakthrough_magnitude_internal(improvement, evidence_count);
        let is_performance_breakthrough =
            improvement >= BREAKTHROUGH_THRESHOLD && evidence_count >= MIN_EVIDENCE_COUNT;
        let is_novelty_breakthrough = novelty >= HIGH_NOVELTY_THRESHOLD;
        let is_paradigm = improvement >= PARADIGM_SHIFT_THRESHOLD;
        let is_breakthrough = is_performance_breakthrough || is_novelty_breakthrough;

        // Record in history
        if self.history.len() < MAX_HISTORY {
            self.history.push(DetectionRecord {
                candidate_id,
                was_breakthrough: is_breakthrough,
                magnitude,
                tick: self.tick,
            });
        }

        if is_breakthrough {
            let category = if is_paradigm {
                self.stats.paradigm_shifts += 1;
                BreakthroughCategory::ParadigmShift
            } else if is_novelty_breakthrough {
                BreakthroughCategory::NovelDimension
            } else if improvement >= 0.20 {
                BreakthroughCategory::PerformanceGain
            } else {
                BreakthroughCategory::IncrementalMajor
            };

            let impact = self.impact_estimation_internal(improvement, novelty, evidence_count);
            let bt = Breakthrough {
                id: candidate_id,
                discovery: desc,
                magnitude,
                novelty,
                impact,
                baseline_before: baseline,
                performance_after: measured,
                evidence_count,
                tick: self.tick,
                category,
            };

            if self.breakthroughs.len() < MAX_BREAKTHROUGHS {
                self.breakthroughs.insert(candidate_id, bt.clone());
            }

            self.stats.confirmed_breakthroughs += 1;
            self.stats.ticks_since_last = 0;
            if magnitude > self.stats.largest_breakthrough {
                self.stats.largest_breakthrough = magnitude;
            }
            self.stats.avg_magnitude_ema =
                self.stats.avg_magnitude_ema * (1.0 - EMA_ALPHA) + magnitude * EMA_ALPHA;
            self.stats.avg_novelty_ema =
                self.stats.avg_novelty_ema * (1.0 - EMA_ALPHA) + novelty * EMA_ALPHA;
            self.stats.avg_impact_ema =
                self.stats.avg_impact_ema * (1.0 - EMA_ALPHA) + impact * EMA_ALPHA;

            // Update baseline
            if measured > self.current_baseline {
                self.current_baseline = measured;
            }

            self.update_rate();
            Some(bt)
        } else {
            self.stats.false_alarms += 1;
            self.update_rate();
            None
        }
    }

    /// Compute the magnitude of a potential breakthrough.
    pub fn breakthrough_magnitude(&self, candidate_id: u64) -> f32 {
        match self.candidates.get(&candidate_id) {
            Some(c) => self.breakthrough_magnitude_internal(c.improvement_ratio, c.evidence_pieces.len()),
            None => 0.0,
        }
    }

    /// Assess the novelty of a discovery description.
    pub fn novelty_assessment(&self, description: &str) -> f32 {
        self.novelty_assessment_internal(description)
    }

    /// Estimate the downstream impact of a breakthrough.
    pub fn impact_estimation(&self, breakthrough_id: u64) -> f32 {
        match self.breakthroughs.get(&breakthrough_id) {
            Some(bt) => bt.impact,
            None => 0.0,
        }
    }

    /// Get the history of all confirmed breakthroughs.
    pub fn breakthrough_history(&self) -> Vec<&Breakthrough> {
        self.breakthroughs.values().collect()
    }

    /// Compute the current breakthrough rate (breakthroughs per candidate).
    pub fn breakthrough_rate(&self) -> f32 {
        if self.stats.total_candidates == 0 {
            return 0.0;
        }
        self.stats.confirmed_breakthroughs as f32 / self.stats.total_candidates as f32
    }

    /// Get statistics.
    pub fn stats(&self) -> &BreakthroughStats {
        &self.stats
    }

    /// Number of confirmed breakthroughs.
    pub fn breakthrough_count(&self) -> usize {
        self.breakthroughs.len()
    }

    /// Current performance baseline.
    pub fn current_baseline(&self) -> f32 {
        self.current_baseline
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn breakthrough_magnitude_internal(&self, improvement: f32, evidence_count: usize) -> f32 {
        let base_mag = improvement * MAGNITUDE_SCALE;
        let evidence_factor = (evidence_count as f32 / 10.0).min(1.0);
        let confidence_boost = evidence_factor * 0.3;
        (base_mag * (1.0 + confidence_boost)).max(0.0).min(MAGNITUDE_SCALE)
    }

    fn novelty_assessment_internal(&self, description: &str) -> f32 {
        let tokens = self.simple_tokenize(description);
        if tokens.is_empty() {
            return 0.5;
        }
        let mut known_token_matches = 0u64;
        let total_tokens = tokens.len() as f32;

        for token in &tokens {
            let token_hash = fnv1a_hash(token.as_bytes());
            for dim in self.known_dimensions.values() {
                let dim_hash = fnv1a_hash(dim.name.as_bytes());
                if token_hash == dim_hash {
                    known_token_matches += 1;
                    break;
                }
            }
        }

        let familiarity = known_token_matches as f32 / total_tokens;
        (1.0 - familiarity).max(0.0).min(1.0)
    }

    fn impact_estimation_internal(
        &self,
        improvement: f32,
        novelty: f32,
        evidence_count: usize,
    ) -> f32 {
        let perf_impact = improvement.max(0.0).min(1.0);
        let novelty_weight = novelty * 0.3;
        let evidence_weight = ((evidence_count as f32).min(10.0) / 10.0) * 0.2;
        let cascade_factor = if improvement >= PARADIGM_SHIFT_THRESHOLD {
            0.2
        } else {
            0.0
        };
        (perf_impact * 0.5 + novelty_weight + evidence_weight + cascade_factor)
            .min(1.0)
            .max(0.0)
    }

    fn simple_tokenize<'a>(&self, text: &'a str) -> Vec<&'a str> {
        text.split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn evict_oldest_candidate(&mut self) {
        let oldest = self
            .candidates
            .values()
            .filter(|c| c.evaluated)
            .min_by_key(|c| c.submit_tick)
            .map(|c| c.id);
        if let Some(oid) = oldest {
            self.candidates.remove(&oid);
        } else {
            // Evict oldest unevaluated
            let oldest2 = self
                .candidates
                .values()
                .min_by_key(|c| c.submit_tick)
                .map(|c| c.id);
            if let Some(oid2) = oldest2 {
                self.candidates.remove(&oid2);
            }
        }
    }

    fn update_rate(&mut self) {
        let window = self.history.len().min(RATE_WINDOW);
        if window == 0 {
            return;
        }
        let recent = &self.history[self.history.len() - window..];
        let bt_count = recent.iter().filter(|r| r.was_breakthrough).count();
        let rate = bt_count as f32 / window as f32;
        self.stats.breakthrough_rate_ema =
            self.stats.breakthrough_rate_ema * (1.0 - EMA_ALPHA) + rate * EMA_ALPHA;
    }
}
