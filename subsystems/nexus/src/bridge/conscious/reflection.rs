// SPDX-License-Identifier: GPL-2.0
//! # Bridge Reflection
//!
//! Performance reflection engine. After each batch of operations, reflects
//! on what went well and what went poorly. Builds a historical pattern of
//! successes and failures, extracts lessons, and accumulates wisdom — the
//! distilled understanding that transcends individual experiences.
//!
//! Reflection is how the bridge converts raw experience into actionable
//! intelligence. A bridge that never reflects never truly learns.

extern crate alloc;

use alloc::collections::BTreeMap;
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

// ============================================================================
// REFLECTION TYPES
// ============================================================================

/// Outcome category of a batch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchOutcome {
    Excellent,
    Good,
    Mediocre,
    Poor,
    Failure,
}

impl BatchOutcome {
    fn from_score(score: f32) -> Self {
        match score {
            s if s >= 0.9 => BatchOutcome::Excellent,
            s if s >= 0.7 => BatchOutcome::Good,
            s if s >= 0.5 => BatchOutcome::Mediocre,
            s if s >= 0.3 => BatchOutcome::Poor,
            _ => BatchOutcome::Failure,
        }
    }

    fn as_weight(&self) -> f32 {
        match self {
            BatchOutcome::Excellent => 1.0,
            BatchOutcome::Good => 0.7,
            BatchOutcome::Mediocre => 0.4,
            BatchOutcome::Poor => 0.2,
            BatchOutcome::Failure => 0.0,
        }
    }
}

/// A reflection on a single batch of operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BatchReflection {
    pub id: u64,
    pub tick: u64,
    /// Number of operations in the batch
    pub batch_size: u32,
    /// Overall quality score (0.0 – 1.0)
    pub quality_score: f32,
    /// Throughput achieved (ops/tick)
    pub throughput: f32,
    /// Latency observed (ticks)
    pub latency: f32,
    /// Cache hit rate during this batch
    pub cache_hit_rate: f32,
    /// Outcome category
    pub outcome: BatchOutcome,
    /// What went well (hashed tags)
    pub strengths: Vec<u64>,
    /// What went poorly (hashed tags)
    pub weaknesses: Vec<u64>,
    /// Context signature (FNV hash of contributing factors)
    pub context_hash: u64,
}

/// A detected pattern in batch outcomes
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ReflectionPattern {
    pub id: u64,
    pub description: String,
    /// Context signature this pattern occurs in
    pub context_hash: u64,
    /// How often this pattern occurs
    pub occurrences: u64,
    /// Average outcome when this pattern is present
    pub avg_outcome: f32,
    /// Is this a positive or negative pattern?
    pub positive: bool,
    /// Confidence in the pattern (based on sample size)
    pub confidence: f32,
}

/// A lesson extracted from patterns
#[derive(Debug, Clone)]
pub struct Lesson {
    pub id: u64,
    pub description: String,
    /// Source pattern ID
    pub source_pattern: u64,
    /// How actionable (0.0 – 1.0)
    pub actionability: f32,
    /// Has this lesson been applied?
    pub applied: bool,
    /// Improvement observed after application (if applied)
    pub improvement: f32,
    /// Times this lesson has been reinforced
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
    pub patterns_detected: usize,
    pub lessons_extracted: usize,
    pub lessons_applied: usize,
    pub wisdom_score: f32,
    pub improvement_trend: f32,
    pub failure_rate: f32,
}

// ============================================================================
// BRIDGE REFLECTION ENGINE
// ============================================================================

/// Performance reflection engine — converts batch experiences into patterns,
/// lessons, and accumulated wisdom.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeReflection {
    /// Ring buffer of batch reflections
    reflections: Vec<BatchReflection>,
    refl_write_idx: usize,
    /// Detected patterns (keyed by FNV hash)
    patterns: BTreeMap<u64, ReflectionPattern>,
    /// Extracted lessons (keyed by FNV hash)
    lessons: BTreeMap<u64, Lesson>,
    /// EMA-smoothed quality score
    avg_quality: f32,
    /// EMA-smoothed failure rate
    failure_rate: f32,
    /// Total reflections ever
    total_reflections: u64,
    /// Monotonic tick
    tick: u64,
    /// Context-to-outcome map for pattern detection
    context_outcomes: BTreeMap<u64, Vec<f32>>,
}

impl BridgeReflection {
    pub fn new() -> Self {
        Self {
            reflections: Vec::new(),
            refl_write_idx: 0,
            patterns: BTreeMap::new(),
            lessons: BTreeMap::new(),
            avg_quality: 0.5,
            failure_rate: 0.0,
            total_reflections: 0,
            tick: 0,
            context_outcomes: BTreeMap::new(),
        }
    }

    /// Reflect on a completed batch of operations
    pub fn reflect_on_batch(
        &mut self,
        batch_size: u32,
        quality_score: f32,
        throughput: f32,
        latency: f32,
        cache_hit_rate: f32,
        context_factors: &[&str],
        strengths: &[&str],
        weaknesses: &[&str],
    ) -> u64 {
        self.tick += 1;
        self.total_reflections += 1;
        let clamped_quality = quality_score.max(0.0).min(1.0);

        // Build context hash from contributing factors
        let mut context_hash = FNV_OFFSET;
        for factor in context_factors {
            context_hash ^= fnv1a_hash(factor.as_bytes());
            context_hash = context_hash.wrapping_mul(FNV_PRIME);
        }

        let strength_hashes: Vec<u64> =
            strengths.iter().map(|s| fnv1a_hash(s.as_bytes())).collect();
        let weakness_hashes: Vec<u64> = weaknesses
            .iter()
            .map(|w| fnv1a_hash(w.as_bytes()))
            .collect();

        let id = fnv1a_hash(&self.total_reflections.to_le_bytes());
        let outcome = BatchOutcome::from_score(clamped_quality);
        let is_failure = matches!(outcome, BatchOutcome::Failure | BatchOutcome::Poor);

        let reflection = BatchReflection {
            id,
            tick: self.tick,
            batch_size,
            quality_score: clamped_quality,
            throughput,
            latency,
            cache_hit_rate,
            outcome,
            strengths: strength_hashes,
            weaknesses: weakness_hashes,
            context_hash,
        };

        if self.reflections.len() < MAX_REFLECTIONS {
            self.reflections.push(reflection);
        } else {
            self.reflections[self.refl_write_idx] = reflection;
        }
        self.refl_write_idx = (self.refl_write_idx + 1) % MAX_REFLECTIONS;

        // Update running averages
        self.avg_quality = EMA_ALPHA * clamped_quality + (1.0 - EMA_ALPHA) * self.avg_quality;
        let fail_sample = if is_failure { 1.0 } else { 0.0 };
        self.failure_rate = EMA_ALPHA * fail_sample + (1.0 - EMA_ALPHA) * self.failure_rate;

        // Track context → outcome for pattern detection
        let outcomes = self
            .context_outcomes
            .entry(context_hash)
            .or_insert_with(Vec::new);
        if outcomes.len() < 64 {
            outcomes.push(clamped_quality);
        }

        id
    }

    /// Scan for recurring patterns in batch outcomes
    #[inline]
    pub fn identify_pattern(&mut self) -> Vec<ReflectionPattern> {
        let mut new_patterns = Vec::new();

        for (&ctx_hash, outcomes) in self.context_outcomes.iter() {
            if (outcomes.len() as u64) < PATTERN_MIN_OCCURRENCES {
                continue;
            }
            let avg: f32 = outcomes.iter().sum::<f32>() / outcomes.len() as f32;
            let positive = avg >= 0.6;
            let desc = if positive {
                String::from("Positive recurring context pattern")
            } else {
                String::from("Negative recurring context pattern")
            };

            let confidence = (outcomes.len() as f32 / 20.0).min(1.0);
            let pattern_id = ctx_hash ^ fnv1a_hash(b"pattern");

            let pattern = self
                .patterns
                .entry(pattern_id)
                .or_insert_with(|| ReflectionPattern {
                    id: pattern_id,
                    description: desc.clone(),
                    context_hash: ctx_hash,
                    occurrences: 0,
                    avg_outcome: 0.5,
                    positive,
                    confidence: 0.0,
                });

            pattern.occurrences = outcomes.len() as u64;
            pattern.avg_outcome = EMA_ALPHA * avg + (1.0 - EMA_ALPHA) * pattern.avg_outcome;
            pattern.confidence = confidence;
            pattern.positive = positive;

            if self.patterns.len() <= MAX_PATTERNS {
                new_patterns.push(pattern.clone());
            }
        }

        new_patterns
    }

    /// Extract a lesson from a detected pattern
    pub fn extract_lesson(&mut self, pattern_id: u64) -> Option<u64> {
        let pattern = self.patterns.get(&pattern_id)?.clone();
        if pattern.confidence < 0.3 {
            return None;
        }

        let lesson_id = pattern_id ^ fnv1a_hash(b"lesson");
        let actionability = pattern.confidence * if pattern.positive { 0.8 } else { 1.0 };
        let desc = if pattern.positive {
            String::from("Reinforce conditions of this positive pattern")
        } else {
            String::from("Avoid or mitigate conditions of this negative pattern")
        };

        let lesson = self.lessons.entry(lesson_id).or_insert_with(|| Lesson {
            id: lesson_id,
            description: desc,
            source_pattern: pattern_id,
            actionability,
            applied: false,
            improvement: 0.0,
            reinforcements: 0,
        });

        lesson.reinforcements += 1;
        lesson.actionability = (lesson.actionability + actionability) / 2.0;

        if self.lessons.len() <= MAX_LESSONS {
            Some(lesson_id)
        } else {
            None
        }
    }

    /// Mark a lesson as applied and record its impact
    #[inline]
    pub fn apply_insight(&mut self, lesson_id: u64, observed_improvement: f32) {
        if let Some(lesson) = self.lessons.get_mut(&lesson_id) {
            lesson.applied = true;
            lesson.improvement =
                EMA_ALPHA * observed_improvement + (1.0 - EMA_ALPHA) * lesson.improvement;
        }
    }

    /// Wisdom score: accumulated distilled knowledge (0.0 – 1.0)
    pub fn wisdom_score(&self) -> f32 {
        if self.lessons.is_empty() {
            return 0.0;
        }
        let applied: Vec<&Lesson> = self.lessons.values().filter(|l| l.applied).collect();
        if applied.is_empty() {
            // Having lessons but not applying them is minimal wisdom
            return 0.05 * (self.lessons.len() as f32).min(10.0) / 10.0;
        }

        let avg_improvement: f32 =
            applied.iter().map(|l| l.improvement).sum::<f32>() / applied.len() as f32;
        let application_rate = applied.len() as f32 / self.lessons.len() as f32;
        let reinforcement_depth: f32 = applied
            .iter()
            .map(|l| (l.reinforcements as f32).min(10.0) / 10.0)
            .sum::<f32>()
            / applied.len() as f32;

        (avg_improvement * 0.4 + application_rate * 0.3 + reinforcement_depth * 0.3).min(1.0)
    }

    /// Quality improvement trend over time
    fn improvement_trend(&self) -> f32 {
        let len = self.reflections.len();
        if len < 4 {
            return 0.0;
        }
        let mid = len / 2;
        let first: f32 = self.reflections[..mid]
            .iter()
            .map(|r| r.quality_score)
            .sum::<f32>()
            / mid as f32;
        let second: f32 = self.reflections[mid..]
            .iter()
            .map(|r| r.quality_score)
            .sum::<f32>()
            / (len - mid) as f32;
        second - first
    }

    /// Compute aggregate reflection statistics
    pub fn stats(&self) -> ReflectionStats {
        ReflectionStats {
            total_reflections: self.total_reflections,
            avg_quality: self.avg_quality,
            patterns_detected: self.patterns.len(),
            lessons_extracted: self.lessons.len(),
            lessons_applied: self.lessons.values().filter(|l| l.applied).count(),
            wisdom_score: self.wisdom_score(),
            improvement_trend: self.improvement_trend(),
            failure_rate: self.failure_rate,
        }
    }

    /// Get the most impactful applied lessons
    #[inline]
    pub fn top_insights(&self, count: usize) -> Vec<&Lesson> {
        let mut applied: Vec<&Lesson> = self.lessons.values().filter(|l| l.applied).collect();
        applied.sort_by(|a, b| {
            b.improvement
                .partial_cmp(&a.improvement)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        applied.truncate(count);
        applied
    }
}
