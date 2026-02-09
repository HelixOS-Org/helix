// SPDX-License-Identifier: GPL-2.0
//! # Apps Teaching — Knowledge Transfer About App Behaviour
//!
//! Transfers accumulated knowledge about application behaviour to other
//! kernel subsystems. The engine designs curricula, assesses whether a
//! learner subsystem is ready, evaluates learning outcomes, and measures
//! the long-term impact of the teaching on system-wide performance.
//!
//! Teaching is structured: knowledge is exported as pattern descriptors,
//! organised into lessons, and sequenced according to prerequisite
//! dependencies. Each lesson carries a difficulty rating and an expected
//! skill gain.

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
const EMA_ALPHA_DEN: u64 = 9;
const MAX_PATTERNS: usize = 1024;
const MAX_LESSONS: usize = 512;
const MAX_STUDENTS: usize = 128;
const READINESS_THRESHOLD: u64 = 40;
const MASTERY_THRESHOLD: u64 = 80;

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

/// A behavioural pattern that can be taught to another subsystem.
#[derive(Clone, Debug)]
pub struct AppPattern {
    pub pattern_id: u64,
    pub label: String,
    pub feature_vector: Vec<u64>,
    pub frequency: u64,
    pub confidence: u64,
    pub prerequisite_ids: Vec<u64>,
}

/// A single lesson in a curriculum.
#[derive(Clone, Debug)]
pub struct Lesson {
    pub lesson_id: u64,
    pub label: String,
    pub pattern_ids: Vec<u64>,
    pub difficulty: u64,
    pub expected_skill_gain: u64,
    pub prerequisite_lesson_ids: Vec<u64>,
    pub times_taught: u64,
    pub avg_score_ema: u64,
}

/// A curriculum — an ordered sequence of lessons.
#[derive(Clone, Debug)]
pub struct Curriculum {
    pub curriculum_id: u64,
    pub label: String,
    pub lesson_sequence: Vec<u64>,
    pub total_difficulty: u64,
    pub completion_rate_ema: u64,
    pub creation_tick: u64,
}

/// Represents a student subsystem that receives knowledge.
#[derive(Clone, Debug)]
pub struct Student {
    pub student_id: u64,
    pub label: String,
    pub skill_level: u64,
    pub lessons_completed: Vec<u64>,
    pub assessment_scores: Vec<u64>,
    pub readiness: u64,
    pub mastery_ema: u64,
}

/// A knowledge export packet sent to another subsystem.
#[derive(Clone, Debug)]
pub struct KnowledgeExport {
    pub export_id: u64,
    pub student_id: u64,
    pub pattern_ids: Vec<u64>,
    pub encoded_knowledge: Vec<u64>,
    pub tick: u64,
}

/// Aggregated teaching statistics.
#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct TeachingStats {
    pub total_patterns: u64,
    pub total_lessons: u64,
    pub total_curricula: u64,
    pub total_students: u64,
    pub total_exports: u64,
    pub avg_mastery_ema: u64,
    pub avg_impact_ema: u64,
    pub lessons_taught: u64,
}

// ---------------------------------------------------------------------------
// AppsTeaching
// ---------------------------------------------------------------------------

/// Engine for structured knowledge transfer about app behaviour to other
/// kernel subsystems.
pub struct AppsTeaching {
    patterns: BTreeMap<u64, AppPattern>,
    lessons: BTreeMap<u64, Lesson>,
    curricula: BTreeMap<u64, Curriculum>,
    students: BTreeMap<u64, Student>,
    exports: Vec<KnowledgeExport>,
    stats: TeachingStats,
    rng: u64,
    tick: u64,
}

impl AppsTeaching {
    /// Create a new teaching engine.
    pub fn new(seed: u64) -> Self {
        Self {
            patterns: BTreeMap::new(),
            lessons: BTreeMap::new(),
            curricula: BTreeMap::new(),
            students: BTreeMap::new(),
            exports: Vec::new(),
            stats: TeachingStats::default(),
            rng: seed | 1,
            tick: 0,
        }
    }

    // -- pattern registration -----------------------------------------------

    /// Register an observed app behaviour pattern.
    pub fn register_pattern(
        &mut self,
        label: &str,
        features: &[u64],
        confidence: u64,
        prereqs: &[u64],
    ) -> u64 {
        let pid = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        let pattern = AppPattern {
            pattern_id: pid,
            label: String::from(label),
            feature_vector: features.to_vec(),
            frequency: 1,
            confidence: confidence.min(100),
            prerequisite_ids: prereqs.to_vec(),
        };
        self.patterns.insert(pid, pattern);
        if self.patterns.len() > MAX_PATTERNS {
            self.evict_weakest_pattern();
        }
        self.stats.total_patterns = self.patterns.len() as u64;
        pid
    }

    /// Increment the frequency of an existing pattern.
    #[inline]
    pub fn observe_pattern(&mut self, pattern_id: u64) {
        if let Some(p) = self.patterns.get_mut(&pattern_id) {
            p.frequency += 1;
        }
    }

    // -- student registration -----------------------------------------------

    /// Register a student subsystem.
    pub fn register_student(&mut self, label: &str) -> Option<u64> {
        if self.students.len() >= MAX_STUDENTS {
            return None;
        }
        let sid = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        self.students.insert(sid, Student {
            student_id: sid,
            label: String::from(label),
            skill_level: 0,
            lessons_completed: Vec::new(),
            assessment_scores: Vec::new(),
            readiness: 0,
            mastery_ema: 0,
        });
        self.stats.total_students = self.students.len() as u64;
        Some(sid)
    }

    // -- public API ---------------------------------------------------------

    /// Teach a set of app patterns to a student subsystem.
    pub fn teach_app_patterns(&mut self, student_id: u64, pattern_ids: &[u64]) -> bool {
        let student = match self.students.get_mut(&student_id) {
            Some(s) => s,
            None => return false,
        };

        if student.readiness < READINESS_THRESHOLD {
            return false;
        }

        self.tick += 1;
        let mut taught_count: u64 = 0;
        for &pid in pattern_ids {
            if self.patterns.contains_key(&pid) {
                taught_count += 1;
            }
        }

        if taught_count == 0 {
            return false;
        }

        // Simulate learning — skill gain proportional to taught patterns.
        let skill_gain = (taught_count * 3).min(20);
        let student = self.students.get_mut(&student_id).unwrap();
        student.skill_level = (student.skill_level + skill_gain).min(100);
        student.mastery_ema = ema_update(student.mastery_ema, student.skill_level);

        self.stats.lessons_taught += 1;
        self.refresh_avg_mastery();
        true
    }

    /// Design a curriculum for a student based on their current skill.
    pub fn curriculum_design(&mut self, student_id: u64, label: &str) -> Option<u64> {
        let student = self.students.get(&student_id)?;
        let skill = student.skill_level;
        let completed: Vec<u64> = student.lessons_completed.clone();

        // Select lessons the student hasn't completed, ordered by difficulty.
        let mut candidate_lessons: Vec<(u64, u64)> = self
            .lessons
            .values()
            .filter(|l| !completed.contains(&l.lesson_id))
            .filter(|l| l.difficulty <= skill + 30)
            .map(|l| (l.lesson_id, l.difficulty))
            .collect();
        candidate_lessons.sort_by_key(|(_, d)| *d);

        let lesson_sequence: Vec<u64> = candidate_lessons.iter().map(|(id, _)| *id).collect();
        if lesson_sequence.is_empty() {
            return None;
        }

        let total_difficulty: u64 = candidate_lessons.iter().map(|(_, d)| *d).sum();
        let cid = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        self.curricula.insert(cid, Curriculum {
            curriculum_id: cid,
            label: String::from(label),
            lesson_sequence,
            total_difficulty,
            completion_rate_ema: 0,
            creation_tick: self.tick,
        });
        self.stats.total_curricula = self.curricula.len() as u64;
        Some(cid)
    }

    /// Assess a student's learning by testing them on a set of patterns.
    pub fn learning_assessment(
        &mut self,
        student_id: u64,
        test_pattern_ids: &[u64],
    ) -> Option<u64> {
        let student = self.students.get(&student_id)?;
        if test_pattern_ids.is_empty() {
            return Some(0);
        }

        // Score is based on how many test patterns the student has been exposed
        // to and their skill level relative to pattern confidence.
        let mut correct: u64 = 0;
        for &pid in test_pattern_ids {
            if let Some(pattern) = self.patterns.get(&pid) {
                let threshold = pattern.confidence / 2;
                if student.skill_level >= threshold {
                    correct += 1;
                }
            }
        }

        let score = correct * 100 / test_pattern_ids.len() as u64;
        if let Some(s) = self.students.get_mut(&student_id) {
            s.assessment_scores.push(score);
            s.mastery_ema = ema_update(s.mastery_ema, score);
        }

        self.refresh_avg_mastery();
        Some(score)
    }

    /// Measure the teaching impact — how much the student's performance has
    /// improved since teaching began.
    #[inline]
    pub fn teaching_impact(&self, student_id: u64) -> Option<u64> {
        let student = self.students.get(&student_id)?;
        if student.assessment_scores.is_empty() {
            return Some(0);
        }
        let latest = *student.assessment_scores.last().unwrap_or(&0);
        let first = *student.assessment_scores.first().unwrap_or(&0);
        Some(latest.saturating_sub(first))
    }

    /// Export knowledge about patterns to a student subsystem.
    pub fn knowledge_export(&mut self, student_id: u64, pattern_ids: &[u64]) -> Option<u64> {
        if !self.students.contains_key(&student_id) {
            return None;
        }
        self.tick += 1;

        let mut encoded = Vec::new();
        let mut valid_pids = Vec::new();
        for &pid in pattern_ids {
            if let Some(pat) = self.patterns.get(&pid) {
                valid_pids.push(pid);
                // Encode as hash of feature vector.
                let mut buf = Vec::new();
                for &f in &pat.feature_vector {
                    buf.extend_from_slice(&f.to_le_bytes());
                }
                encoded.push(fnv1a(&buf));
            }
        }

        if valid_pids.is_empty() {
            return None;
        }

        let eid = fnv1a(&student_id.to_le_bytes()) ^ xorshift64(&mut self.rng);
        self.exports.push(KnowledgeExport {
            export_id: eid,
            student_id,
            pattern_ids: valid_pids,
            encoded_knowledge: encoded,
            tick: self.tick,
        });
        self.stats.total_exports = self.exports.len() as u64;
        Some(eid)
    }

    /// Assess whether a student is ready to learn a specific lesson.
    pub fn student_readiness(&self, student_id: u64, lesson_id: u64) -> Option<u64> {
        let student = self.students.get(&student_id)?;
        let lesson = self.lessons.get(&lesson_id)?;

        // Check prerequisite completion.
        let prereqs_met: u64 = lesson
            .prerequisite_lesson_ids
            .iter()
            .filter(|pid| student.lessons_completed.contains(pid))
            .count() as u64;
        let prereqs_total = lesson.prerequisite_lesson_ids.len() as u64;
        let prereq_score = if prereqs_total == 0 {
            100
        } else {
            prereqs_met * 100 / prereqs_total
        };

        // Skill relative to difficulty.
        let skill_score = if student.skill_level >= lesson.difficulty {
            100
        } else {
            student.skill_level * 100 / lesson.difficulty.max(1)
        };

        let readiness = (prereq_score + skill_score) / 2;
        Some(readiness)
    }

    /// Create a lesson from a set of patterns.
    pub fn create_lesson(
        &mut self,
        label: &str,
        pattern_ids: &[u64],
        difficulty: u64,
        expected_gain: u64,
        prereqs: &[u64],
    ) -> u64 {
        let lid = fnv1a(label.as_bytes()) ^ xorshift64(&mut self.rng);
        let lesson = Lesson {
            lesson_id: lid,
            label: String::from(label),
            pattern_ids: pattern_ids.to_vec(),
            difficulty: difficulty.min(100),
            expected_skill_gain: expected_gain.min(30),
            prerequisite_lesson_ids: prereqs.to_vec(),
            times_taught: 0,
            avg_score_ema: 0,
        };
        self.lessons.insert(lid, lesson);
        if self.lessons.len() > MAX_LESSONS {
            self.evict_least_used_lesson();
        }
        self.stats.total_lessons = self.lessons.len() as u64;
        lid
    }

    /// Mark a lesson as completed for a student and update their skill.
    pub fn complete_lesson(&mut self, student_id: u64, lesson_id: u64, score: u64) -> bool {
        let lesson = match self.lessons.get_mut(&lesson_id) {
            Some(l) => l,
            None => return false,
        };
        lesson.times_taught += 1;
        lesson.avg_score_ema = ema_update(lesson.avg_score_ema, score.min(100));
        let gain = lesson.expected_skill_gain;

        if let Some(student) = self.students.get_mut(&student_id) {
            if !student.lessons_completed.contains(&lesson_id) {
                student.lessons_completed.push(lesson_id);
            }
            let actual_gain = if score >= MASTERY_THRESHOLD { gain } else { gain / 2 };
            student.skill_level = (student.skill_level + actual_gain).min(100);
            student.readiness = student.skill_level;
            student.mastery_ema = ema_update(student.mastery_ema, score.min(100));
            return true;
        }
        false
    }

    /// Return current statistics.
    #[inline(always)]
    pub fn stats(&self) -> &TeachingStats {
        &self.stats
    }

    /// Return a reference to a student record.
    #[inline(always)]
    pub fn get_student(&self, student_id: u64) -> Option<&Student> {
        self.students.get(&student_id)
    }

    // -- internal -----------------------------------------------------------

    fn evict_weakest_pattern(&mut self) {
        let weakest = self
            .patterns
            .iter()
            .min_by_key(|(_, p)| p.frequency * p.confidence)
            .map(|(k, _)| *k);
        if let Some(key) = weakest {
            self.patterns.remove(&key);
        }
    }

    fn evict_least_used_lesson(&mut self) {
        let least = self
            .lessons
            .iter()
            .min_by_key(|(_, l)| l.times_taught)
            .map(|(k, _)| *k);
        if let Some(key) = least {
            self.lessons.remove(&key);
        }
    }

    fn refresh_avg_mastery(&mut self) {
        if self.students.is_empty() {
            return;
        }
        let sum: u64 = self.students.values().map(|s| s.mastery_ema).sum();
        let avg = sum / self.students.len() as u64;
        self.stats.avg_mastery_ema = ema_update(self.stats.avg_mastery_ema, avg);
        self.refresh_impact();
    }

    fn refresh_impact(&mut self) {
        if self.students.is_empty() {
            return;
        }
        let sum: u64 = self
            .students
            .values()
            .map(|s| {
                let first = s.assessment_scores.first().copied().unwrap_or(0);
                let last = s.assessment_scores.last().copied().unwrap_or(0);
                last.saturating_sub(first)
            })
            .sum();
        let avg = sum / self.students.len() as u64;
        self.stats.avg_impact_ema = ema_update(self.stats.avg_impact_ema, avg);
    }
}
