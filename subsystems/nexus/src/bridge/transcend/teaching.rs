// SPDX-License-Identifier: GPL-2.0
//! # Bridge Teaching — Knowledge Transfer to Other Subsystems
//!
//! The bridge teaches what it learns about syscall optimisation to other
//! subsystems. Each `Lesson` carries a topic, content, difficulty, and
//! effectiveness rating. Lessons are organised into curricula; student
//! progress is tracked per-subsystem.
//!
//! FNV-1a hashing indexes lessons by topic; xorshift64 drives stochastic
//! assessments; EMA smooths teaching effectiveness over time.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_LESSONS: usize = 512;
const MAX_CURRICULA: usize = 64;
const MAX_STUDENTS: usize = 64;
const MAX_LESSONS_PER_CURRICULUM: usize = 32;
const MAX_ASSESSMENT_HISTORY: usize = 128;
const EMA_ALPHA: f32 = 0.10;
const MASTERY_THRESHOLD: f32 = 0.85;
const DIFFICULTY_SCALE: f32 = 10.0;
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
// TEACHING TYPES
// ============================================================================

/// Difficulty tier for a lesson.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DifficultyTier {
    Introductory,
    Intermediate,
    Advanced,
    Expert,
    Mastery,
}

/// Kind of knowledge being transferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeDomain {
    SyscallOptimisation,
    ResourceManagement,
    LatencyReduction,
    ThroughputScaling,
    SecurityHardening,
    SchedulingStrategy,
    MemoryEfficiency,
    ErrorRecovery,
}

/// A single lesson prepared by the bridge.
#[derive(Debug, Clone)]
pub struct Lesson {
    pub lesson_id: u64,
    pub topic: String,
    pub content: String,
    pub domain: KnowledgeDomain,
    pub difficulty: DifficultyTier,
    pub difficulty_score: f32,
    pub effectiveness: f32,
    pub times_taught: u64,
    pub avg_student_score: f32,
    pub created_tick: u64,
}

/// A curriculum — ordered sequence of lessons.
#[derive(Debug, Clone)]
pub struct Curriculum {
    pub curriculum_id: u64,
    pub name: String,
    pub domain: KnowledgeDomain,
    pub lesson_ids: Vec<u64>,
    pub total_difficulty: f32,
    pub completion_rate: f32,
}

/// Per-student progress tracker.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct StudentProgress {
    pub student_id: u64,
    pub name: String,
    pub lessons_completed: LinearMap<f32, 64>, // lesson_id -> score
    pub mastery_level: f32,
    pub total_assessments: u64,
    pub knowledge_ema: f32,
}

/// Assessment result for a single quiz/test.
#[derive(Debug, Clone)]
pub struct Assessment {
    pub student_id: u64,
    pub lesson_id: u64,
    pub score: f32,
    pub tick: u64,
}

// ============================================================================
// TEACHING STATS
// ============================================================================

/// Aggregate statistics for the teaching engine.
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct TeachingStats {
    pub total_lessons: u64,
    pub total_curricula: u64,
    pub total_students: u64,
    pub total_assessments: u64,
    pub avg_effectiveness: f32,
    pub avg_student_mastery: f32,
    pub knowledge_transfer_rate: f32,
    pub lessons_taught: u64,
    pub effectiveness_ema: f32,
}

// ============================================================================
// BRIDGE TEACHING ENGINE
// ============================================================================

/// Knowledge transfer engine. Prepares lessons and curricula from bridge
/// intelligence and tracks how effectively other subsystems absorb them.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeTeaching {
    lessons: BTreeMap<u64, Lesson>,
    curricula: BTreeMap<u64, Curriculum>,
    students: BTreeMap<u64, StudentProgress>,
    assessments: Vec<Assessment>,
    lessons_taught: u64,
    tick: u64,
    rng_state: u64,
    effectiveness_ema: f32,
    transfer_rate_ema: f32,
}

impl BridgeTeaching {
    /// Create a new teaching engine.
    pub fn new(seed: u64) -> Self {
        Self {
            lessons: BTreeMap::new(),
            curricula: BTreeMap::new(),
            students: BTreeMap::new(),
            assessments: Vec::new(),
            lessons_taught: 0,
            tick: 0,
            rng_state: seed ^ 0x7EAC_4000_BEEF,
            effectiveness_ema: 0.5,
            transfer_rate_ema: 0.0,
        }
    }

    /// Create and register a new lesson.
    pub fn teach_lesson(
        &mut self,
        topic: &str,
        content: &str,
        domain: KnowledgeDomain,
        difficulty: DifficultyTier,
    ) -> u64 {
        self.tick += 1;
        let lesson_id = fnv1a_hash(topic.as_bytes()) ^ self.tick;
        let difficulty_score = match difficulty {
            DifficultyTier::Introductory => 1.0,
            DifficultyTier::Intermediate => 3.0,
            DifficultyTier::Advanced => 5.0,
            DifficultyTier::Expert => 7.5,
            DifficultyTier::Mastery => DIFFICULTY_SCALE,
        };

        let lesson = Lesson {
            lesson_id,
            topic: String::from(topic),
            content: String::from(content),
            domain,
            difficulty,
            difficulty_score,
            effectiveness: 0.5,
            times_taught: 0,
            avg_student_score: 0.0,
            created_tick: self.tick,
        };

        if self.lessons.len() < MAX_LESSONS {
            self.lessons.insert(lesson_id, lesson);
        }
        self.lessons_taught += 1;
        lesson_id
    }

    /// Build a curriculum from existing lessons, ordered by difficulty.
    pub fn prepare_curriculum(
        &mut self,
        name: &str,
        domain: KnowledgeDomain,
        lesson_ids: &[u64],
    ) -> u64 {
        self.tick += 1;
        let cid = fnv1a_hash(name.as_bytes()) ^ self.tick;

        let mut valid_ids = Vec::new();
        let mut total_diff = 0.0_f32;

        for &lid in lesson_ids {
            if valid_ids.len() >= MAX_LESSONS_PER_CURRICULUM {
                break;
            }
            if let Some(lesson) = self.lessons.get(&lid) {
                total_diff += lesson.difficulty_score;
                valid_ids.push(lid);
            }
        }

        // Sort by difficulty (ascending)
        valid_ids.sort_by(|a, b| {
            let da = self.lessons.get(a).map_or(0.0, |l| l.difficulty_score);
            let db = self.lessons.get(b).map_or(0.0, |l| l.difficulty_score);
            da.partial_cmp(&db).unwrap_or(core::cmp::Ordering::Equal)
        });

        let curriculum = Curriculum {
            curriculum_id: cid,
            name: String::from(name),
            domain,
            lesson_ids: valid_ids,
            total_difficulty: total_diff,
            completion_rate: 0.0,
        };

        if self.curricula.len() < MAX_CURRICULA {
            self.curricula.insert(cid, curriculum);
        }

        cid
    }

    /// Register a student (subsystem) for learning.
    pub fn register_student(&mut self, name: &str) -> u64 {
        let sid = fnv1a_hash(name.as_bytes());
        if self.students.len() < MAX_STUDENTS && !self.students.contains_key(&sid) {
            self.students.insert(sid, StudentProgress {
                student_id: sid,
                name: String::from(name),
                lessons_completed: LinearMap::new(),
                mastery_level: 0.0,
                total_assessments: 0,
                knowledge_ema: 0.0,
            });
        }
        sid
    }

    /// Assess a student on a lesson, returning the score.
    #[inline]
    pub fn assess_learning(
        &mut self,
        student_id: u64,
        lesson_id: u64,
    ) -> Option<f32> {
        self.tick += 1;
        let lesson = self.lessons.get(&lesson_id)?;
        let difficulty = lesson.difficulty_score;

        // Simulate score based on student mastery + randomness + difficulty
        let student = self.students.get(&student_id)?;
        let base = student.knowledge_ema;
        let noise = ((xorshift64(&mut self.rng_state) % 200) as f32 - 100.0) / 500.0;
        let raw_score = base + noise - (difficulty / DIFFICULTY_SCALE) * 0.3;
        let score = raw_score.max(0.0).min(1.0);

        // Record assessment
        if self.assessments.len() < MAX_ASSESSMENT_HISTORY {
            self.assessments.push(Assessment {
                student_id,
                lesson_id,
                score,
                tick: self.tick,
            });
        }

        // Update student
        if let Some(student_mut) = self.students.get_mut(&student_id) {
            student_mut.lessons_completed.insert(lesson_id, score);
            student_mut.total_assessments += 1;
            student_mut.knowledge_ema =
                EMA_ALPHA * score + (1.0 - EMA_ALPHA) * student_mut.knowledge_ema;
            student_mut.mastery_level = self.compute_mastery(student_id);
        }

        // Update lesson effectiveness
        if let Some(lesson_mut) = self.lessons.get_mut(&lesson_id) {
            lesson_mut.times_taught += 1;
            let n = lesson_mut.times_taught as f32;
            lesson_mut.avg_student_score =
                lesson_mut.avg_student_score * ((n - 1.0) / n) + score / n;
            lesson_mut.effectiveness =
                EMA_ALPHA * score + (1.0 - EMA_ALPHA) * lesson_mut.effectiveness;
        }

        self.effectiveness_ema = EMA_ALPHA * score + (1.0 - EMA_ALPHA) * self.effectiveness_ema;

        Some(score)
    }

    /// Overall teaching effectiveness across all lessons.
    #[inline]
    pub fn teaching_effectiveness(&self) -> f32 {
        if self.lessons.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.lessons.values().map(|l| l.effectiveness).sum();
        sum / self.lessons.len() as f32
    }

    /// Knowledge transfer rate: fraction of students at mastery.
    #[inline]
    pub fn knowledge_transfer_rate(&self) -> f32 {
        if self.students.is_empty() {
            return 0.0;
        }
        let mastered = self
            .students
            .values()
            .filter(|s| s.mastery_level >= MASTERY_THRESHOLD)
            .count();
        mastered as f32 / self.students.len() as f32
    }

    /// Get a specific student's progress.
    #[inline(always)]
    pub fn student_progress(&self, student_id: u64) -> Option<&StudentProgress> {
        self.students.get(&student_id)
    }

    /// Curriculum completion rate for a student.
    pub fn curriculum_completion(
        &self,
        student_id: u64,
        curriculum_id: u64,
    ) -> f32 {
        let curriculum = match self.curricula.get(&curriculum_id) {
            Some(c) => c,
            None => return 0.0,
        };
        let student = match self.students.get(&student_id) {
            Some(s) => s,
            None => return 0.0,
        };

        if curriculum.lesson_ids.is_empty() {
            return 0.0;
        }

        let completed = curriculum
            .lesson_ids
            .iter()
            .filter(|lid| student.lessons_completed.contains_key(lid))
            .count();

        completed as f32 / curriculum.lesson_ids.len() as f32
    }

    /// Recommend the next lesson for a student based on what they haven't done.
    #[inline]
    pub fn recommend_next_lesson(&self, student_id: u64, curriculum_id: u64) -> Option<u64> {
        let curriculum = self.curricula.get(&curriculum_id)?;
        let student = self.students.get(&student_id)?;

        for lid in &curriculum.lesson_ids {
            if !student.lessons_completed.contains_key(lid) {
                return Some(*lid);
            }
        }
        None // all complete
    }

    /// Get a lesson by ID.
    #[inline(always)]
    pub fn get_lesson(&self, lesson_id: u64) -> Option<&Lesson> {
        self.lessons.get(&lesson_id)
    }

    /// List all students.
    #[inline(always)]
    pub fn all_students(&self) -> Vec<u64> {
        self.students.keys().copied().collect()
    }

    /// Aggregate statistics.
    pub fn stats(&self) -> TeachingStats {
        let avg_mastery = if self.students.is_empty() {
            0.0
        } else {
            let sum: f32 = self.students.values().map(|s| s.mastery_level).sum();
            sum / self.students.len() as f32
        };

        TeachingStats {
            total_lessons: self.lessons.len() as u64,
            total_curricula: self.curricula.len() as u64,
            total_students: self.students.len() as u64,
            total_assessments: self.assessments.len() as u64,
            avg_effectiveness: self.teaching_effectiveness(),
            avg_student_mastery: avg_mastery,
            knowledge_transfer_rate: self.knowledge_transfer_rate(),
            lessons_taught: self.lessons_taught,
            effectiveness_ema: self.effectiveness_ema,
        }
    }

    /// Current tick.
    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Total lesson count.
    #[inline(always)]
    pub fn lesson_count(&self) -> usize {
        self.lessons.len()
    }

    // --- private helpers ---

    fn compute_mastery(&self, student_id: u64) -> f32 {
        let student = match self.students.get(&student_id) {
            Some(s) => s,
            None => return 0.0,
        };
        if student.lessons_completed.is_empty() {
            return 0.0;
        }
        let sum: f32 = student.lessons_completed.values().sum();
        let avg = sum / student.lessons_completed.len() as f32;

        // Weight by difficulty of completed lessons
        let mut weighted_sum = 0.0_f32;
        let mut weight_total = 0.0_f32;
        for (&lid, &score) in &student.lessons_completed {
            let diff = self.lessons.get(&lid).map_or(1.0, |l| l.difficulty_score);
            weighted_sum += score * diff;
            weight_total += diff;
        }

        let weighted_avg = if weight_total > 0.0 {
            weighted_sum / weight_total
        } else {
            avg
        };

        // Combine simple and weighted averages
        0.5 * avg + 0.5 * weighted_avg
    }
}
