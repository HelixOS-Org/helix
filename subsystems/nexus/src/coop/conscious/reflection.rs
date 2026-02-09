// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Reflection
//!
//! Reflects on cooperation outcomes. After each negotiation cycle, evaluates
//! fairness, efficiency, and participant satisfaction. Detects repeating
//! patterns, extracts lessons, and accumulates cooperation wisdom — the
//! distilled understanding that transcends individual negotiations.
//!
//! Reflection converts raw negotiation experience into actionable
//! intelligence. A cooperation engine that never reflects never truly learns.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_REFLECTIONS: usize = 256;
const MAX_LESSONS: usize = 128;
const MAX_PATTERNS: usize = 64;
const PATTERN_MIN_OCCURRENCES: u64 = 3;
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
// REFLECTION TYPES
// ============================================================================

/// Outcome category of a negotiation cycle
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
            s if s >= 0.90 => CycleOutcome::Excellent,
            s if s >= 0.70 => CycleOutcome::Good,
            s if s >= 0.50 => CycleOutcome::Mediocre,
            s if s >= 0.30 => CycleOutcome::Poor,
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

/// A reflection on a single negotiation cycle
#[derive(Debug, Clone)]
pub struct NegotiationReflection {
    pub id: u64,
    pub tick: u64,
    /// Number of participants in this cycle
    pub participant_count: u32,
    /// Overall quality score (0.0 – 1.0)
    pub quality_score: f32,
    /// Fairness achieved (0.0 – 1.0)
    pub fairness: f32,
    /// Efficiency: resources used vs. value created
    pub efficiency: f32,
    /// Average satisfaction across participants
    pub avg_satisfaction: f32,
    /// Outcome category
    pub outcome: CycleOutcome,
    /// Context signature (FNV hash of contributing factors)
    pub context_hash: u64,
    /// Strengths observed (hashed tags)
    pub strengths: Vec<u64>,
    /// Weaknesses observed (hashed tags)
    pub weaknesses: Vec<u64>,
}

/// A detected pattern in negotiation outcomes
#[derive(Debug, Clone)]
pub struct CoopPattern {
    pub id: u64,
    pub description: String,
    pub context_hash: u64,
    pub occurrences: u64,
    pub avg_outcome: f32,
    pub positive: bool,
    pub confidence: f32,
}

/// A lesson extracted from patterns
#[derive(Debug, Clone)]
pub struct CoopLesson {
    pub id: u64,
    pub description: String,
    pub source_pattern: u64,
    /// Actionability (0.0 – 1.0)
    pub actionability: f32,
    /// Has this lesson been applied?
    pub applied: bool,
    /// Improvement observed after application
    pub improvement: f32,
    /// Times reinforced
    pub reinforcements: u64,
}

// ============================================================================
// REFLECTION STATS
// ============================================================================

/// Aggregate reflection statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct ReflectionStats {
    pub total_reflections: u64,
    pub avg_quality: f32,
    pub avg_fairness: f32,
    pub patterns_detected: usize,
    pub lessons_extracted: usize,
    pub lessons_applied: usize,
    pub wisdom_score: f32,
    pub efficiency_trend: f32,
    pub failure_rate: f32,
}

// ============================================================================
// COOPERATION REFLECTION ENGINE
// ============================================================================

/// Reflects on cooperation outcomes — converts negotiation experiences into
/// patterns, lessons, and accumulated cooperation wisdom.
#[derive(Debug)]
pub struct CoopReflection {
    /// Ring buffer of negotiation reflections
    reflections: Vec<NegotiationReflection>,
    refl_write_idx: usize,
    /// Detected patterns (keyed by FNV hash)
    patterns: BTreeMap<u64, CoopPattern>,
    /// Extracted lessons (keyed by FNV hash)
    lessons: BTreeMap<u64, CoopLesson>,
    /// EMA-smoothed quality score
    avg_quality: f32,
    /// EMA-smoothed fairness score
    avg_fairness: f32,
    /// EMA-smoothed efficiency
    avg_efficiency: f32,
    /// EMA-smoothed failure rate
    failure_rate: f32,
    /// Total reflections ever
    total_reflections: u64,
    /// Monotonic tick
    tick: u64,
    /// Context-to-outcome map for pattern detection
    context_outcomes: BTreeMap<u64, Vec<f32>>,
    /// PRNG state
    rng_state: u64,
}

impl CoopReflection {
    pub fn new() -> Self {
        Self {
            reflections: Vec::new(),
            refl_write_idx: 0,
            patterns: BTreeMap::new(),
            lessons: BTreeMap::new(),
            avg_quality: 0.5,
            avg_fairness: 0.5,
            avg_efficiency: 0.5,
            failure_rate: 0.0,
            total_reflections: 0,
            tick: 0,
            context_outcomes: BTreeMap::new(),
            rng_state: 0xBEF1_C00B_DEAD_BEEF,
        }
    }

    /// Reflect on a completed negotiation cycle
    #[inline]
    pub fn reflect_on_negotiation(
        &mut self,
        participant_count: u32,
        quality_score: f32,
        fairness: f32,
        efficiency: f32,
        avg_satisfaction: f32,
        context_factors: &[&str],
        strength_tags: &[&str],
        weakness_tags: &[&str],
    ) {
        self.tick += 1;
        self.total_reflections += 1;

        let clamped_quality = quality_score.max(0.0).min(1.0);
        let clamped_fairness = fairness.max(0.0).min(1.0);
        let clamped_efficiency = efficiency.max(0.0).min(1.0);
        let clamped_satisfaction = avg_satisfaction.max(0.0).min(1.0);

        let outcome = CycleOutcome::from_score(clamped_quality);
        let is_failure = matches!(outcome, CycleOutcome::Failure);

        // EMA updates
        self.avg_quality = EMA_ALPHA * clamped_quality + (1.0 - EMA_ALPHA) * self.avg_quality;
        self.avg_fairness = EMA_ALPHA * clamped_fairness + (1.0 - EMA_ALPHA) * self.avg_fairness;
        self.avg_efficiency =
            EMA_ALPHA * clamped_efficiency + (1.0 - EMA_ALPHA) * self.avg_efficiency;
        self.failure_rate = EMA_ALPHA * (if is_failure { 1.0 } else { 0.0 })
            + (1.0 - EMA_ALPHA) * self.failure_rate;

        // Compute context hash from factors
        let mut ctx_hash = FNV_OFFSET;
        for factor in context_factors {
            ctx_hash ^= fnv1a_hash(factor.as_bytes());
            ctx_hash = ctx_hash.wrapping_mul(FNV_PRIME);
        }

        let strengths: Vec<u64> = strength_tags
            .iter()
            .map(|t| fnv1a_hash(t.as_bytes()))
            .collect();
        let weaknesses: Vec<u64> = weakness_tags
            .iter()
            .map(|t| fnv1a_hash(t.as_bytes()))
            .collect();

        let reflection = NegotiationReflection {
            id: self.total_reflections,
            tick: self.tick,
            participant_count,
            quality_score: clamped_quality,
            fairness: clamped_fairness,
            efficiency: clamped_efficiency,
            avg_satisfaction: clamped_satisfaction,
            outcome,
            context_hash: ctx_hash,
            strengths,
            weaknesses,
        };

        if self.reflections.len() < MAX_REFLECTIONS {
            self.reflections.push(reflection);
        } else {
            self.reflections[self.refl_write_idx] = reflection;
        }
        self.refl_write_idx = (self.refl_write_idx + 1) % MAX_REFLECTIONS;

        // Track context-to-outcome for pattern detection
        let outcomes = self
            .context_outcomes
            .entry(ctx_hash)
            .or_insert_with(Vec::new);
        outcomes.push(clamped_quality);
        if outcomes.len() > MAX_PATTERNS {
            outcomes.pop_front();
        }

        // Auto-detect patterns
        self.detect_patterns();
    }

    /// Detect repeating patterns in outcomes grouped by context
    fn detect_patterns(&mut self) {
        for (&ctx_hash, outcomes) in self.context_outcomes.iter() {
            if (outcomes.len() as u64) < PATTERN_MIN_OCCURRENCES {
                continue;
            }

            let avg = outcomes.iter().sum::<f32>() / outcomes.len() as f32;
            let positive = avg >= 0.6;
            let confidence = (outcomes.len() as f32 / 20.0).min(1.0);

            let pattern = self
                .patterns
                .entry(ctx_hash)
                .or_insert_with(|| CoopPattern {
                    id: ctx_hash,
                    description: String::from(if positive {
                        "Positive cooperation pattern"
                    } else {
                        "Negative cooperation pattern"
                    }),
                    context_hash: ctx_hash,
                    occurrences: 0,
                    avg_outcome: 0.5,
                    positive,
                    confidence: 0.0,
                });
            pattern.occurrences = outcomes.len() as u64;
            pattern.avg_outcome = avg;
            pattern.positive = positive;
            pattern.confidence = confidence;
        }
    }

    /// Analyze satisfaction trends across reflections
    pub fn satisfaction_analysis(&self) -> f32 {
        if self.reflections.is_empty() {
            return 0.5;
        }

        let len = self.reflections.len();
        if len < 4 {
            return self
                .reflections
                .iter()
                .map(|r| r.avg_satisfaction)
                .sum::<f32>()
                / len as f32;
        }

        let mid = len / 2;
        let first_avg: f32 = self.reflections[..mid]
            .iter()
            .map(|r| r.avg_satisfaction)
            .sum::<f32>()
            / mid as f32;
        let second_avg: f32 = self.reflections[mid..]
            .iter()
            .map(|r| r.avg_satisfaction)
            .sum::<f32>()
            / (len - mid) as f32;

        second_avg - first_avg
    }

    /// Efficiency trend: is cooperation becoming more efficient?
    pub fn efficiency_trend(&self) -> f32 {
        if self.reflections.len() < 4 {
            return 0.0;
        }

        let len = self.reflections.len();
        let mid = len / 2;
        let first: f32 = self.reflections[..mid]
            .iter()
            .map(|r| r.efficiency)
            .sum::<f32>()
            / mid as f32;
        let second: f32 = self.reflections[mid..]
            .iter()
            .map(|r| r.efficiency)
            .sum::<f32>()
            / (len - mid) as f32;
        second - first
    }

    /// Extract lessons from detected patterns
    #[inline]
    pub fn lesson_extraction(&mut self) -> usize {
        let mut new_lessons = 0_usize;

        let pattern_snapshots: Vec<(u64, f32, bool, f32)> = self
            .patterns
            .values()
            .filter(|p| p.occurrences >= PATTERN_MIN_OCCURRENCES && p.confidence >= 0.5)
            .map(|p| (p.id, p.avg_outcome, p.positive, p.confidence))
            .collect();

        for (pid, avg_outcome, positive, confidence) in pattern_snapshots {
            let lesson_id = pid ^ fnv1a_hash(b"lesson");
            let lesson = self.lessons.entry(lesson_id).or_insert_with(|| {
                new_lessons += 1;
                CoopLesson {
                    id: lesson_id,
                    description: String::from(if positive {
                        "Reinforce positive cooperation pattern"
                    } else {
                        "Mitigate negative cooperation pattern"
                    }),
                    source_pattern: pid,
                    actionability: confidence * avg_outcome,
                    applied: false,
                    improvement: 0.0,
                    reinforcements: 0,
                }
            });
            lesson.reinforcements += 1;
            lesson.actionability =
                EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * lesson.actionability;
        }
        new_lessons
    }

    /// Cooperation wisdom: distilled understanding from all lessons
    pub fn cooperation_wisdom(&self) -> f32 {
        if self.lessons.is_empty() {
            return 0.0;
        }

        let mut wisdom = 0.0_f32;
        let mut weight_sum = 0.0_f32;

        for lesson in self.lessons.values() {
            let reinforcement_weight = (lesson.reinforcements as f32 / 10.0).min(1.0);
            let applied_bonus = if lesson.applied { 0.2 } else { 0.0 };
            let lesson_value =
                lesson.actionability * reinforcement_weight + applied_bonus + lesson.improvement;
            wisdom += lesson_value;
            weight_sum += 1.0;
        }

        if weight_sum < f32::EPSILON {
            return 0.0;
        }
        (wisdom / weight_sum).min(1.0)
    }

    /// Get aggregate statistics
    pub fn stats(&self) -> ReflectionStats {
        let applied = self.lessons.values().filter(|l| l.applied).count();

        ReflectionStats {
            total_reflections: self.total_reflections,
            avg_quality: self.avg_quality,
            avg_fairness: self.avg_fairness,
            patterns_detected: self.patterns.len(),
            lessons_extracted: self.lessons.len(),
            lessons_applied: applied,
            wisdom_score: self.cooperation_wisdom(),
            efficiency_trend: self.efficiency_trend(),
            failure_rate: self.failure_rate,
        }
    }
}
