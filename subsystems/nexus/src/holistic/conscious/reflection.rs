// SPDX-License-Identifier: GPL-2.0
//! # Holistic Reflection
//!
//! System-wide reflection engine. Reflects on the kernel's overall
//! performance, decisions, and evolution trajectory. Generates wisdom
//! from accumulated experience by distilling patterns-of-patterns,
//! synthesizing cross-module insights, and projecting growth trajectories.
//!
//! Reflection is how the kernel turns experience into wisdom.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_REFLECTIONS: usize = 256;
const MAX_WISDOMS: usize = 128;
const MAX_PATTERNS: usize = 64;
const MAX_INSIGHTS: usize = 64;
const MAX_TRAJECTORY_POINTS: usize = 256;
const EMA_ALPHA: f32 = 0.08;
const PATTERN_MIN_OCCURRENCES: u64 = 5;
const WISDOM_THRESHOLD: f32 = 0.60;
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

/// Category of reflection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReflectionCategory {
    Performance,
    Decision,
    Evolution,
    Architecture,
    Resource,
    Security,
    Resilience,
    Growth,
}

/// A single reflection entry
#[derive(Debug, Clone)]
pub struct ReflectionEntry {
    pub id: u64,
    pub category: ReflectionCategory,
    pub tick: u64,
    pub subject: String,
    pub observation: String,
    pub insight_score: f32,
    pub actionable: bool,
    pub confidence: f32,
}

/// A distilled piece of wisdom
#[derive(Debug, Clone)]
pub struct Wisdom {
    pub id: u64,
    pub principle: String,
    pub evidence_count: u64,
    pub confidence: f32,
    pub applicability: f32,
    pub tick_crystallized: u64,
    pub source_reflections: Vec<u64>,
}

/// A meta-pattern: a pattern of patterns
#[derive(Debug, Clone)]
pub struct MetaPattern {
    pub id: u64,
    pub description: String,
    pub occurrence_count: u64,
    pub strength: f32,
    pub related_categories: Vec<ReflectionCategory>,
    pub tick_detected: u64,
}

/// A philosophical insight about the kernel's nature
#[derive(Debug, Clone)]
pub struct PhilosophicalInsight {
    pub id: u64,
    pub insight: String,
    pub depth: f32,
    pub novelty: f32,
    pub tick: u64,
}

/// Growth trajectory point
#[derive(Debug, Clone, Copy)]
pub struct GrowthPoint {
    pub tick: u64,
    pub wisdom_count: u32,
    pub pattern_count: u32,
    pub reflection_depth: f32,
    pub growth_velocity: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate reflection statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct ReflectionStats {
    pub total_reflections: u64,
    pub wisdom_count: usize,
    pub pattern_count: usize,
    pub insight_count: usize,
    pub avg_insight_score: f32,
    pub wisdom_confidence: f32,
    pub growth_velocity: f32,
    pub reflection_depth: f32,
}

// ============================================================================
// HOLISTIC REFLECTION
// ============================================================================

/// System-wide reflection engine. Accumulates experience, distills wisdom,
/// detects meta-patterns, generates philosophical insights, and tracks
/// growth trajectory over the kernel's lifetime.
#[derive(Debug)]
pub struct HolisticReflection {
    reflections: Vec<ReflectionEntry>,
    write_idx: usize,
    wisdoms: BTreeMap<u64, Wisdom>,
    patterns: BTreeMap<u64, MetaPattern>,
    insights: BTreeMap<u64, PhilosophicalInsight>,
    growth_trajectory: Vec<GrowthPoint>,
    category_counts: BTreeMap<u8, u64>,
    tick: u64,
    rng_state: u64,
    total_reflections: u64,
    insight_score_ema: f32,
    depth_ema: f32,
    growth_velocity_ema: f32,
}

impl HolisticReflection {
    pub fn new() -> Self {
        Self {
            reflections: Vec::new(),
            write_idx: 0,
            wisdoms: BTreeMap::new(),
            patterns: BTreeMap::new(),
            insights: BTreeMap::new(),
            growth_trajectory: Vec::new(),
            category_counts: BTreeMap::new(),
            tick: 0,
            rng_state: 0xBEEF_DEAD_CAFE_F00D,
            total_reflections: 0,
            insight_score_ema: 0.3,
            depth_ema: 1.0,
            growth_velocity_ema: 0.0,
        }
    }

    /// Record a system-wide reflection
    #[inline]
    pub fn reflect(
        &mut self,
        category: ReflectionCategory,
        subject: String,
        observation: String,
        insight_score: f32,
        actionable: bool,
        confidence: f32,
    ) -> u64 {
        self.tick += 1;
        self.total_reflections += 1;

        let id = fnv1a_hash(subject.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let entry = ReflectionEntry {
            id,
            category,
            tick: self.tick,
            subject,
            observation,
            insight_score: insight_score.clamp(0.0, 1.0),
            actionable,
            confidence: confidence.clamp(0.0, 1.0),
        };

        self.insight_score_ema =
            EMA_ALPHA * entry.insight_score + (1.0 - EMA_ALPHA) * self.insight_score_ema;

        *self.category_counts.entry(category as u8).or_insert(0) += 1;

        if self.reflections.len() < MAX_REFLECTIONS {
            self.reflections.push(entry);
        } else {
            self.reflections[self.write_idx] = entry;
        }
        self.write_idx = (self.write_idx + 1) % MAX_REFLECTIONS;

        id
    }

    /// Perform a comprehensive system reflection cycle
    #[inline]
    pub fn system_reflection(&mut self) -> f32 {
        self.tick += 1;

        let recent_count = self
            .reflections
            .iter()
            .filter(|r| self.tick.saturating_sub(r.tick) < 100)
            .count();
        let recent_quality: f32 = self
            .reflections
            .iter()
            .filter(|r| self.tick.saturating_sub(r.tick) < 100)
            .map(|r| r.insight_score)
            .sum::<f32>()
            / recent_count.max(1) as f32;

        let depth = (recent_count as f32 / 10.0).min(1.0) * 0.5 + recent_quality * 0.5;
        self.depth_ema = EMA_ALPHA * depth + (1.0 - EMA_ALPHA) * self.depth_ema;
        self.depth_ema
    }

    /// Generate wisdom from accumulated experience
    pub fn wisdom_generation(&mut self) -> Vec<Wisdom> {
        self.tick += 1;
        let mut new_wisdom = Vec::new();

        // Group reflections by category and look for recurring high-quality insights
        for (&cat_key, &count) in self.category_counts.iter() {
            if count < PATTERN_MIN_OCCURRENCES {
                continue;
            }

            let cat_reflections: Vec<&ReflectionEntry> = self
                .reflections
                .iter()
                .filter(|r| r.category as u8 == cat_key && r.insight_score > WISDOM_THRESHOLD)
                .collect();

            if cat_reflections.len() < 3 {
                continue;
            }

            let avg_confidence = cat_reflections.iter().map(|r| r.confidence).sum::<f32>()
                / cat_reflections.len() as f32;
            let avg_insight = cat_reflections.iter().map(|r| r.insight_score).sum::<f32>()
                / cat_reflections.len() as f32;

            let id = fnv1a_hash(&cat_key.to_le_bytes()) ^ xorshift64(&mut self.rng_state);
            let sources: Vec<u64> = cat_reflections.iter().take(5).map(|r| r.id).collect();

            let wisdom = Wisdom {
                id,
                principle: String::from("Distilled principle from sustained observations"),
                evidence_count: cat_reflections.len() as u64,
                confidence: avg_confidence,
                applicability: avg_insight,
                tick_crystallized: self.tick,
                source_reflections: sources,
            };

            if self.wisdoms.len() < MAX_WISDOMS {
                self.wisdoms.insert(id, wisdom.clone());
            }
            new_wisdom.push(wisdom);
        }
        new_wisdom
    }

    /// Synthesize experience across categories into cross-cutting insights
    pub fn experience_synthesis(&self) -> Vec<(ReflectionCategory, f32, u64)> {
        self.category_counts
            .iter()
            .map(|(&cat_key, &count)| {
                let cat_quality: f32 = self
                    .reflections
                    .iter()
                    .filter(|r| r.category as u8 == cat_key)
                    .map(|r| r.insight_score)
                    .sum::<f32>()
                    / count.max(1) as f32;
                (int_to_category(cat_key), cat_quality, count)
            })
            .collect()
    }

    /// Detect patterns-of-patterns: meta-level recurring structures
    pub fn pattern_of_patterns(&mut self) -> Vec<MetaPattern> {
        self.tick += 1;
        let mut detected = Vec::new();

        // Look for categories that co-occur in high-insight reflections
        let high_insight: Vec<&ReflectionEntry> = self
            .reflections
            .iter()
            .filter(|r| r.insight_score > WISDOM_THRESHOLD)
            .collect();

        let mut co_occurrence: BTreeMap<(u8, u8), u64> = BTreeMap::new();
        for i in 0..high_insight.len() {
            for j in (i + 1)..high_insight.len() {
                let a = high_insight[i].category as u8;
                let b = high_insight[j].category as u8;
                if a != b && high_insight[i].tick.abs_diff(high_insight[j].tick) < 20 {
                    let key = if a < b { (a, b) } else { (b, a) };
                    *co_occurrence.entry(key).or_insert(0) += 1;
                }
            }
        }

        for (&(a, b), &count) in co_occurrence.iter() {
            if count >= PATTERN_MIN_OCCURRENCES {
                let id = fnv1a_hash(&[a, b]) ^ self.tick;
                let strength = (count as f32 / 20.0).min(1.0);
                let pattern = MetaPattern {
                    id,
                    description: String::from("Cross-category co-occurrence pattern"),
                    occurrence_count: count,
                    strength,
                    related_categories: alloc::vec![int_to_category(a), int_to_category(b)],
                    tick_detected: self.tick,
                };
                if self.patterns.len() < MAX_PATTERNS {
                    self.patterns.insert(id, pattern.clone());
                }
                detected.push(pattern);
            }
        }
        detected
    }

    /// Generate philosophical insights about the kernel's nature
    pub fn philosophical_insight(&mut self) -> Vec<PhilosophicalInsight> {
        self.tick += 1;
        let mut generated = Vec::new();

        let wisdom_depth = self.wisdoms.len() as f32 / MAX_WISDOMS as f32;
        let pattern_depth = self.patterns.len() as f32 / MAX_PATTERNS as f32;
        let experience_breadth = self.category_counts.len() as f32 / 8.0;

        if wisdom_depth > 0.1 {
            let id = xorshift64(&mut self.rng_state);
            let depth = (wisdom_depth + pattern_depth) * 0.5;
            let novelty = 1.0 - (self.insights.len() as f32 / MAX_INSIGHTS as f32).min(0.9);
            let insight = PhilosophicalInsight {
                id,
                insight: String::from(
                    "The kernel's consciousness emerges from the integration of \
                     subsystem awareness into unified experience",
                ),
                depth: depth.clamp(0.0, 1.0),
                novelty: novelty.clamp(0.0, 1.0),
                tick: self.tick,
            };
            if self.insights.len() < MAX_INSIGHTS {
                self.insights.insert(id, insight.clone());
            }
            generated.push(insight);
        }

        if experience_breadth > 0.5 && pattern_depth > 0.1 {
            let id = xorshift64(&mut self.rng_state);
            let insight = PhilosophicalInsight {
                id,
                insight: String::from(
                    "Patterns repeat across domains because optimal strategies \
                     converge regardless of the specific subsystem",
                ),
                depth: experience_breadth.clamp(0.0, 1.0),
                novelty: (1.0 - pattern_depth).clamp(0.1, 1.0),
                tick: self.tick,
            };
            if self.insights.len() < MAX_INSIGHTS {
                self.insights.insert(id, insight.clone());
            }
            generated.push(insight);
        }

        generated
    }

    /// Compute and record the growth trajectory
    #[inline]
    pub fn growth_trajectory(&mut self) -> f32 {
        self.tick += 1;

        let prev_velocity = self.growth_velocity_ema;
        let current_wisdom = self.wisdoms.len() as f32;
        let current_patterns = self.patterns.len() as f32;
        let depth = self.depth_ema;

        let growth_score = current_wisdom * 0.01 + current_patterns * 0.02 + depth * 0.5;
        let velocity = growth_score - prev_velocity;
        self.growth_velocity_ema =
            EMA_ALPHA * growth_score + (1.0 - EMA_ALPHA) * self.growth_velocity_ema;

        let point = GrowthPoint {
            tick: self.tick,
            wisdom_count: self.wisdoms.len() as u32,
            pattern_count: self.patterns.len() as u32,
            reflection_depth: depth,
            growth_velocity: velocity,
        };
        if self.growth_trajectory.len() < MAX_TRAJECTORY_POINTS {
            self.growth_trajectory.push(point);
        } else {
            let idx = (self.tick as usize) % MAX_TRAJECTORY_POINTS;
            self.growth_trajectory[idx] = point;
        }

        self.growth_velocity_ema
    }

    /// Compute aggregate statistics
    pub fn stats(&self) -> ReflectionStats {
        let wisdom_conf = if self.wisdoms.is_empty() {
            0.0
        } else {
            self.wisdoms.values().map(|w| w.confidence).sum::<f32>() / self.wisdoms.len() as f32
        };

        ReflectionStats {
            total_reflections: self.total_reflections,
            wisdom_count: self.wisdoms.len(),
            pattern_count: self.patterns.len(),
            insight_count: self.insights.len(),
            avg_insight_score: self.insight_score_ema,
            wisdom_confidence: wisdom_conf,
            growth_velocity: self.growth_velocity_ema,
            reflection_depth: self.depth_ema,
        }
    }
}

fn int_to_category(v: u8) -> ReflectionCategory {
    match v {
        0 => ReflectionCategory::Performance,
        1 => ReflectionCategory::Decision,
        2 => ReflectionCategory::Evolution,
        3 => ReflectionCategory::Architecture,
        4 => ReflectionCategory::Resource,
        5 => ReflectionCategory::Security,
        6 => ReflectionCategory::Resilience,
        _ => ReflectionCategory::Growth,
    }
}
