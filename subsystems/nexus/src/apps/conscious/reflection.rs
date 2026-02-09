// SPDX-License-Identifier: GPL-2.0
//! # Apps Reflection
//!
//! Reflects on application analysis accuracy. After each classification and
//! prediction cycle, evaluates quality — what went well, what went poorly,
//! and why. Builds a pattern library of successes and failures, extracts
//! actionable insights, and accumulates wisdom: the distilled understanding
//! that transcends individual classification events.
//!
//! Reflection is how the apps engine converts raw classification experience
//! into deep application intelligence.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_REFLECTIONS: usize = 256;
const MAX_INSIGHTS: usize = 128;
const MAX_PATTERNS: usize = 64;
const PATTERN_MIN_OCCURRENCES: u64 = 3;
const WISDOM_DECAY: f32 = 0.999;
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

// ============================================================================
// REFLECTION TYPES
// ============================================================================

/// Outcome category of a classification/prediction cycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOutcome {
    Excellent,
    Good,
    Mediocre,
    Poor,
    Failure,
}

impl CycleOutcome {
    fn from_score(score: f32) -> Self {
        match score {
            s if s >= 0.9 => CycleOutcome::Excellent,
            s if s >= 0.7 => CycleOutcome::Good,
            s if s >= 0.5 => CycleOutcome::Mediocre,
            s if s >= 0.3 => CycleOutcome::Poor,
            _ => CycleOutcome::Failure,
        }
    }

    fn as_weight(&self) -> f32 {
        match self {
            CycleOutcome::Excellent => 1.0,
            CycleOutcome::Good => 0.7,
            CycleOutcome::Mediocre => 0.4,
            CycleOutcome::Poor => 0.2,
            CycleOutcome::Failure => 0.0,
        }
    }
}

/// A reflection on a single classification/prediction cycle
#[derive(Debug, Clone)]
pub struct CycleReflection {
    pub id: u64,
    pub tick: u64,
    /// Number of classifications in the cycle
    pub classifications: u32,
    /// Number of predictions in the cycle
    pub predictions: u32,
    /// Overall quality score (0.0 – 1.0)
    pub quality_score: f32,
    /// Classification accuracy for this cycle
    pub accuracy: f32,
    /// Prediction hit rate for this cycle
    pub prediction_hit_rate: f32,
    /// False positive rate for this cycle
    pub false_positive_rate: f32,
    /// Outcome category
    pub outcome: CycleOutcome,
    /// Strength tags (FNV hashes)
    pub strengths: Vec<u64>,
    /// Weakness tags (FNV hashes)
    pub weaknesses: Vec<u64>,
    /// Context signature (FNV hash of contributing factors)
    pub context_hash: u64,
}

/// A detected pattern in cycle outcomes
#[derive(Debug, Clone)]
pub struct ReflectionPattern {
    pub id: u64,
    pub description: String,
    pub context_hash: u64,
    pub occurrences: u64,
    pub avg_outcome: f32,
    pub positive: bool,
    pub confidence: f32,
}

/// An extracted insight from reflection
#[derive(Debug, Clone)]
pub struct Insight {
    pub id: u64,
    pub description: String,
    /// How actionable this insight is (0.0 – 1.0)
    pub actionability: f32,
    /// Confidence in the insight
    pub confidence: f32,
    /// Evidence strength (number of supporting patterns)
    pub evidence_count: u64,
    /// Tick when first extracted
    pub extracted_tick: u64,
    /// Has this insight been applied?
    pub applied: bool,
}

/// Accumulated wisdom — distilled knowledge that persists
#[derive(Debug, Clone)]
pub struct WisdomEntry {
    pub id: u64,
    pub principle: String,
    pub strength: f32,
    pub applications: u64,
    pub last_applied_tick: u64,
}

// ============================================================================
// REFLECTION STATS
// ============================================================================

/// Aggregate reflection statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct ReflectionStats {
    pub total_reflections: u64,
    pub avg_cycle_quality: f32,
    pub avg_accuracy: f32,
    pub avg_false_positive_rate: f32,
    pub patterns_detected: usize,
    pub insights_extracted: usize,
    pub wisdom_entries: usize,
    pub accuracy_trend: f32,
}

// ============================================================================
// APPS REFLECTION ENGINE
// ============================================================================

/// Reflects on app analysis accuracy — cycle-level evaluation, pattern
/// detection, insight extraction, and wisdom accumulation.
#[derive(Debug)]
pub struct AppsReflection {
    /// Ring buffer of cycle reflections
    reflections: Vec<CycleReflection>,
    write_idx: usize,
    /// Detected patterns keyed by context hash
    patterns: BTreeMap<u64, ReflectionPattern>,
    /// Extracted insights keyed by FNV hash
    insights: BTreeMap<u64, Insight>,
    /// Accumulated wisdom keyed by FNV hash
    wisdom: BTreeMap<u64, WisdomEntry>,
    /// Monotonic tick
    tick: u64,
    /// Total reflections performed
    total_reflections: u64,
    /// EMA-smoothed cycle quality
    quality_ema: f32,
    /// EMA-smoothed accuracy
    accuracy_ema: f32,
    /// EMA-smoothed false positive rate
    false_positive_ema: f32,
}

impl AppsReflection {
    pub fn new() -> Self {
        Self {
            reflections: Vec::new(),
            write_idx: 0,
            patterns: BTreeMap::new(),
            insights: BTreeMap::new(),
            wisdom: BTreeMap::new(),
            tick: 0,
            total_reflections: 0,
            quality_ema: 0.5,
            accuracy_ema: 0.5,
            false_positive_ema: 0.1,
        }
    }

    /// Reflect on a completed classification/prediction cycle
    pub fn reflect_on_cycle(
        &mut self,
        classifications: u32,
        predictions: u32,
        accuracy: f32,
        prediction_hit_rate: f32,
        false_positive_rate: f32,
        context_desc: &str,
    ) -> u64 {
        self.tick += 1;
        self.total_reflections += 1;

        let quality =
            accuracy * 0.4 + prediction_hit_rate * 0.3 + (1.0 - false_positive_rate) * 0.3;
        let clamped_quality = quality.max(0.0).min(1.0);
        let outcome = CycleOutcome::from_score(clamped_quality);
        let context_hash = fnv1a_hash(context_desc.as_bytes());

        let id = fnv1a_hash(&self.total_reflections.to_le_bytes());

        // Determine strengths and weaknesses
        let mut strengths = Vec::new();
        let mut weaknesses = Vec::new();
        if accuracy > 0.8 {
            strengths.push(fnv1a_hash(b"high_accuracy"));
        }
        if accuracy < 0.5 {
            weaknesses.push(fnv1a_hash(b"low_accuracy"));
        }
        if prediction_hit_rate > 0.7 {
            strengths.push(fnv1a_hash(b"good_prediction"));
        }
        if false_positive_rate > 0.2 {
            weaknesses.push(fnv1a_hash(b"high_false_positive"));
        }

        let reflection = CycleReflection {
            id,
            tick: self.tick,
            classifications,
            predictions,
            quality_score: clamped_quality,
            accuracy: accuracy.max(0.0).min(1.0),
            prediction_hit_rate: prediction_hit_rate.max(0.0).min(1.0),
            false_positive_rate: false_positive_rate.max(0.0).min(1.0),
            outcome,
            strengths,
            weaknesses,
            context_hash,
        };

        if self.reflections.len() < MAX_REFLECTIONS {
            self.reflections.push(reflection);
        } else {
            self.reflections[self.write_idx] = reflection;
        }
        self.write_idx = (self.write_idx + 1) % MAX_REFLECTIONS;

        // Update EMAs
        self.quality_ema = EMA_ALPHA * clamped_quality + (1.0 - EMA_ALPHA) * self.quality_ema;
        self.accuracy_ema =
            EMA_ALPHA * accuracy.max(0.0).min(1.0) + (1.0 - EMA_ALPHA) * self.accuracy_ema;
        self.false_positive_ema = EMA_ALPHA * false_positive_rate.max(0.0).min(1.0)
            + (1.0 - EMA_ALPHA) * self.false_positive_ema;

        // Update pattern tracking
        let pattern = self
            .patterns
            .entry(context_hash)
            .or_insert_with(|| ReflectionPattern {
                id: context_hash,
                description: String::from(context_desc),
                context_hash,
                occurrences: 0,
                avg_outcome: 0.5,
                positive: true,
                confidence: 0.0,
            });
        pattern.occurrences += 1;
        pattern.avg_outcome = EMA_ALPHA * clamped_quality + (1.0 - EMA_ALPHA) * pattern.avg_outcome;
        pattern.positive = pattern.avg_outcome > 0.5;
        pattern.confidence =
            (pattern.occurrences as f32 / (PATTERN_MIN_OCCURRENCES as f32 * 5.0)).min(1.0);

        id
    }

    /// Accuracy trend: slope of recent accuracy (positive = improving)
    pub fn accuracy_trend(&self) -> f32 {
        let n = self.reflections.len();
        if n < 4 {
            return 0.0;
        }
        let mid = n / 2;
        let early_avg: f32 = self.reflections[..mid]
            .iter()
            .map(|r| r.accuracy)
            .sum::<f32>()
            / mid as f32;
        let recent_avg: f32 = self.reflections[mid..]
            .iter()
            .map(|r| r.accuracy)
            .sum::<f32>()
            / (n - mid) as f32;
        recent_avg - early_avg
    }

    /// Analyze false positives: which contexts produce the most?
    pub fn false_positive_analysis(&self) -> Vec<(String, f32, u64)> {
        let mut analysis: Vec<(String, f32, u64)> = self
            .patterns
            .values()
            .filter(|p| p.occurrences >= PATTERN_MIN_OCCURRENCES)
            .map(|p| {
                // Find average false positive rate for this context
                let matching: Vec<&CycleReflection> = self
                    .reflections
                    .iter()
                    .filter(|r| r.context_hash == p.context_hash)
                    .collect();
                let avg_fp = if matching.is_empty() {
                    0.0
                } else {
                    matching.iter().map(|r| r.false_positive_rate).sum::<f32>()
                        / matching.len() as f32
                };
                (p.description.clone(), avg_fp, p.occurrences)
            })
            .collect();
        analysis.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        analysis
    }

    /// Extract actionable insights from accumulated patterns
    pub fn insight_extraction(&mut self) -> Vec<Insight> {
        let mut new_insights = Vec::new();
        for pattern in self.patterns.values() {
            if pattern.occurrences < PATTERN_MIN_OCCURRENCES || pattern.confidence < 0.3 {
                continue;
            }
            let seed: u64 = 0xABCD_EF01_2345_6789;
            let insight_id =
                fnv1a_hash(pattern.description.as_bytes()) ^ seed.wrapping_mul(pattern.occurrences);

            if self.insights.contains_key(&insight_id) {
                continue;
            }
            if self.insights.len() >= MAX_INSIGHTS {
                continue;
            }

            let actionability = if !pattern.positive { 0.8 } else { 0.4 };
            let insight = Insight {
                id: insight_id,
                description: pattern.description.clone(),
                actionability,
                confidence: pattern.confidence,
                evidence_count: pattern.occurrences,
                extracted_tick: self.tick,
                applied: false,
            };
            new_insights.push(insight.clone());
            self.insights.insert(insight_id, insight);
        }
        new_insights
    }

    /// Accumulate wisdom from confirmed insights
    #[inline]
    pub fn wisdom_accumulate(&mut self, principle: &str, strength: f32) {
        self.tick += 1;
        let id = fnv1a_hash(principle.as_bytes());
        let tick = self.tick;

        let entry = self.wisdom.entry(id).or_insert_with(|| WisdomEntry {
            id,
            principle: String::from(principle),
            strength: 0.0,
            applications: 0,
            last_applied_tick: tick,
        });

        entry.strength =
            EMA_ALPHA * strength.max(0.0).min(1.0) + (1.0 - EMA_ALPHA) * entry.strength;
        entry.applications += 1;
        entry.last_applied_tick = tick;

        // Apply wisdom decay to older entries
        for w in self.wisdom.values_mut() {
            if w.id != id {
                w.strength *= WISDOM_DECAY;
            }
        }
    }

    /// Compute aggregate reflection statistics
    pub fn stats(&self) -> ReflectionStats {
        ReflectionStats {
            total_reflections: self.total_reflections,
            avg_cycle_quality: self.quality_ema,
            avg_accuracy: self.accuracy_ema,
            avg_false_positive_rate: self.false_positive_ema,
            patterns_detected: self.patterns.len(),
            insights_extracted: self.insights.len(),
            wisdom_entries: self.wisdom.len(),
            accuracy_trend: self.accuracy_trend(),
        }
    }
}
