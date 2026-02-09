// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Teaching — Knowledge Transfer for Cooperation
//!
//! Teaches other subsystems how to cooperate effectively through structured
//! curricula, trust lessons, and cooperation mentoring sessions.  Each lesson
//! is tracked for effectiveness via EMA, indexed through FNV-1a, and adapted
//! by xorshift64-guided stochastic curriculum evolution so that students
//! progressively master sharing, fairness, and mutual trust.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 10;
const MAX_CURRICULA: usize = 256;
const MAX_LESSONS: usize = 1024;
const MAX_STUDENTS: usize = 2048;
const MAX_MENTORING_SESSIONS: usize = 512;
const EFFECTIVENESS_DECAY_NUM: u64 = 98;
const EFFECTIVENESS_DECAY_DEN: u64 = 100;
const MASTERY_THRESHOLD: u64 = 80;
const PASSING_THRESHOLD: u64 = 50;

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

fn clamp(val: u64, lo: u64, hi: u64) -> u64 {
    if val < lo { lo } else if val > hi { hi } else { val }
}

// ---------------------------------------------------------------------------
// Lesson topic
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum LessonTopic {
    ResourceSharing,
    TrustBuilding,
    FairnessProtocol,
    ConflictResolution,
    NegotiationStrategy,
}

// ---------------------------------------------------------------------------
// Lesson
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Lesson {
    pub lesson_id: u64,
    pub topic: LessonTopic,
    pub difficulty: u64,
    pub effectiveness: u64,
    pub times_taught: u64,
    pub avg_student_score: u64,
    pub creation_tick: u64,
    pub description: String,
}

// ---------------------------------------------------------------------------
// Curriculum
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Curriculum {
    pub curriculum_id: u64,
    pub name_hash: u64,
    pub lesson_ids: Vec<u64>,
    pub completion_rate: u64,
    pub avg_effectiveness: u64,
    pub generation: u64,
    pub creation_tick: u64,
}

// ---------------------------------------------------------------------------
// Student record
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct StudentRecord {
    pub student_id: u64,
    pub subsystem_hash: u64,
    pub lessons_completed: Vec<u64>,
    pub cooperation_score: u64,
    pub trust_aptitude: u64,
    pub sharing_aptitude: u64,
    pub fairness_aptitude: u64,
    pub enrolment_tick: u64,
}

// ---------------------------------------------------------------------------
// Mentoring session
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct MentoringSession {
    pub session_id: u64,
    pub mentor_id: u64,
    pub student_id: u64,
    pub topic: LessonTopic,
    pub improvement: u64,
    pub duration_ticks: u64,
    pub start_tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
#[repr(align(64))]
pub struct TeachingStats {
    pub total_lessons_taught: u64,
    pub curricula_created: u64,
    pub students_enrolled: u64,
    pub students_mastered: u64,
    pub mentoring_sessions: u64,
    pub avg_effectiveness: u64,
    pub avg_student_score: u64,
    pub lessons_pruned: u64,
}

// ---------------------------------------------------------------------------
// Manager
// ---------------------------------------------------------------------------

pub struct CoopTeaching {
    lessons: BTreeMap<u64, Lesson>,
    curricula: BTreeMap<u64, Curriculum>,
    students: BTreeMap<u64, StudentRecord>,
    mentoring_sessions: BTreeMap<u64, MentoringSession>,
    effectiveness_index: LinearMap<u64, 64>,
    stats: TeachingStats,
    rng_state: u64,
    current_tick: u64,
}

impl CoopTeaching {
    pub fn new() -> Self {
        Self {
            lessons: BTreeMap::new(),
            curricula: BTreeMap::new(),
            students: BTreeMap::new(),
            mentoring_sessions: BTreeMap::new(),
            effectiveness_index: LinearMap::new(),
            stats: TeachingStats::default(),
            rng_state: 0xDEAD_BEEF_FACE_C0DEu64,
            current_tick: 0,
        }
    }

    // -----------------------------------------------------------------------
    // teach_cooperation — deliver a cooperation lesson to a student
    // -----------------------------------------------------------------------
    pub fn teach_cooperation(
        &mut self,
        student_subsystem: &str,
        topic: LessonTopic,
        difficulty: u64,
    ) -> u64 {
        self.current_tick += 1;
        let sub_hash = fnv1a(student_subsystem.as_bytes());
        let lesson_hash = sub_hash ^ fnv1a(&(difficulty).to_le_bytes()) ^ self.current_tick;

        // Create or update student record
        let student = self.students.entry(sub_hash).or_insert_with(|| {
            self.stats.students_enrolled += 1;
            StudentRecord {
                student_id: sub_hash,
                subsystem_hash: sub_hash,
                lessons_completed: Vec::new(),
                cooperation_score: 50,
                trust_aptitude: 50,
                sharing_aptitude: 50,
                fairness_aptitude: 50,
                enrolment_tick: self.current_tick,
            }
        });

        // Compute student's receptiveness based on aptitude and difficulty
        let aptitude = match topic {
            LessonTopic::TrustBuilding => student.trust_aptitude,
            LessonTopic::ResourceSharing => student.sharing_aptitude,
            LessonTopic::FairnessProtocol => student.fairness_aptitude,
            _ => student.cooperation_score,
        };
        let diff_clamped = clamp(difficulty, 1, 100);
        let learning_rate = if aptitude >= diff_clamped {
            clamp(aptitude - diff_clamped + 50, 20, 100)
        } else {
            clamp(50u64.saturating_sub(diff_clamped - aptitude), 10, 50)
        };

        // Generate score with stochastic element
        let noise = xorshift64(&mut self.rng_state) % 20;
        let score = clamp(learning_rate + noise, 0, 100);

        // Update student aptitudes
        match topic {
            LessonTopic::TrustBuilding => {
                student.trust_aptitude = ema_update(student.trust_aptitude, score);
            }
            LessonTopic::ResourceSharing => {
                student.sharing_aptitude = ema_update(student.sharing_aptitude, score);
            }
            LessonTopic::FairnessProtocol => {
                student.fairness_aptitude = ema_update(student.fairness_aptitude, score);
            }
            LessonTopic::ConflictResolution | LessonTopic::NegotiationStrategy => {
                student.cooperation_score = ema_update(student.cooperation_score, score);
            }
        }
        student.lessons_completed.push(lesson_hash);

        if student.cooperation_score >= MASTERY_THRESHOLD
            && student.trust_aptitude >= MASTERY_THRESHOLD
            && student.sharing_aptitude >= MASTERY_THRESHOLD
        {
            self.stats.students_mastered += 1;
        }

        // Create lesson record
        let lesson = Lesson {
            lesson_id: lesson_hash,
            topic,
            difficulty: diff_clamped,
            effectiveness: score,
            times_taught: 1,
            avg_student_score: score,
            creation_tick: self.current_tick,
            description: String::new(),
        };

        if self.lessons.len() >= MAX_LESSONS {
            self.evict_least_effective_lesson();
        }
        self.effectiveness_index.insert(lesson_hash, score);
        self.lessons.insert(lesson_hash, lesson);
        self.stats.total_lessons_taught += 1;
        self.stats.avg_effectiveness = ema_update(self.stats.avg_effectiveness, score);
        self.stats.avg_student_score = ema_update(self.stats.avg_student_score, score);

        lesson_hash
    }

    // -----------------------------------------------------------------------
    // sharing_curriculum — build a structured curriculum for sharing
    // -----------------------------------------------------------------------
    pub fn sharing_curriculum(&mut self, lesson_ids: &[u64]) -> u64 {
        self.current_tick += 1;
        let name_hash = lesson_ids.iter().fold(FNV_OFFSET, |acc, &l| {
            acc ^ fnv1a(&l.to_le_bytes())
        });
        let cid = name_hash ^ self.current_tick;

        let avg_eff = if lesson_ids.is_empty() {
            0
        } else {
            let total: u64 = lesson_ids
                .iter()
                .filter_map(|lid| self.effectiveness_index.get(lid))
                .sum();
            let count = lesson_ids
                .iter()
                .filter(|lid| self.effectiveness_index.contains_key(lid))
                .count() as u64;
            if count > 0 { total / count } else { 0 }
        };

        let curriculum = Curriculum {
            curriculum_id: cid,
            name_hash,
            lesson_ids: lesson_ids.to_vec(),
            completion_rate: 0,
            avg_effectiveness: avg_eff,
            generation: 0,
            creation_tick: self.current_tick,
        };

        if self.curricula.len() >= MAX_CURRICULA {
            let oldest = self.curricula.keys().next().copied();
            if let Some(k) = oldest { self.curricula.remove(&k); }
        }
        self.curricula.insert(cid, curriculum);
        self.stats.curricula_created += 1;

        cid
    }

    // -----------------------------------------------------------------------
    // trust_lesson — focused lesson on trust building
    // -----------------------------------------------------------------------
    #[inline]
    pub fn trust_lesson(
        &mut self,
        student_subsystem: &str,
        scenario_complexity: u64,
    ) -> u64 {
        self.teach_cooperation(
            student_subsystem,
            LessonTopic::TrustBuilding,
            scenario_complexity,
        )
    }

    // -----------------------------------------------------------------------
    // teaching_effectiveness — measure overall teaching effectiveness
    // -----------------------------------------------------------------------
    pub fn teaching_effectiveness(&mut self) -> u64 {
        if self.lessons.is_empty() {
            return 0;
        }

        let total_eff: u64 = self.effectiveness_index.values().sum();
        let count = self.effectiveness_index.len() as u64;
        let avg = total_eff / core::cmp::max(count, 1);

        let mastery_rate = if self.students.is_empty() {
            0
        } else {
            let mastered = self
                .students
                .values()
                .filter(|s| s.cooperation_score >= MASTERY_THRESHOLD)
                .count() as u64;
            mastered.wrapping_mul(100) / self.students.len() as u64
        };

        let curriculum_quality = if self.curricula.is_empty() {
            50
        } else {
            let total: u64 = self.curricula.values().map(|c| c.avg_effectiveness).sum();
            total / self.curricula.len() as u64
        };

        let effectiveness = (avg + mastery_rate + curriculum_quality) / 3;
        self.stats.avg_effectiveness = ema_update(self.stats.avg_effectiveness, effectiveness);
        effectiveness
    }

    // -----------------------------------------------------------------------
    // cooperation_mentoring — one-on-one mentoring session
    // -----------------------------------------------------------------------
    pub fn cooperation_mentoring(
        &mut self,
        mentor_id: u64,
        student_subsystem: &str,
        topic: LessonTopic,
        duration_ticks: u64,
    ) -> u64 {
        self.current_tick += 1;
        let student_hash = fnv1a(student_subsystem.as_bytes());
        let sid = mentor_id ^ student_hash ^ self.current_tick;

        let mentor_expertise = 80u64; // assumed high
        let student_level = self
            .students
            .get(&student_hash)
            .map(|s| s.cooperation_score)
            .unwrap_or(30);

        let gap = if mentor_expertise > student_level {
            mentor_expertise - student_level
        } else {
            5
        };
        let improvement = clamp(
            gap.wrapping_mul(duration_ticks) / 100 + xorshift64(&mut self.rng_state) % 10,
            1,
            50,
        );

        // Update student
        if let Some(student) = self.students.get_mut(&student_hash) {
            match topic {
                LessonTopic::TrustBuilding => {
                    student.trust_aptitude = clamp(student.trust_aptitude + improvement, 0, 100);
                }
                LessonTopic::ResourceSharing => {
                    student.sharing_aptitude =
                        clamp(student.sharing_aptitude + improvement, 0, 100);
                }
                LessonTopic::FairnessProtocol => {
                    student.fairness_aptitude =
                        clamp(student.fairness_aptitude + improvement, 0, 100);
                }
                _ => {
                    student.cooperation_score =
                        clamp(student.cooperation_score + improvement, 0, 100);
                }
            }
        }

        let session = MentoringSession {
            session_id: sid,
            mentor_id,
            student_id: student_hash,
            topic,
            improvement,
            duration_ticks,
            start_tick: self.current_tick,
        };

        if self.mentoring_sessions.len() >= MAX_MENTORING_SESSIONS {
            let oldest = self.mentoring_sessions.keys().next().copied();
            if let Some(k) = oldest { self.mentoring_sessions.remove(&k); }
        }
        self.mentoring_sessions.insert(sid, session);
        self.stats.mentoring_sessions += 1;

        sid
    }

    // -----------------------------------------------------------------------
    // student_cooperation — query a student's cooperation proficiency
    // -----------------------------------------------------------------------
    #[inline]
    pub fn student_cooperation(&self, student_subsystem: &str) -> u64 {
        let sub_hash = fnv1a(student_subsystem.as_bytes());
        self.students
            .get(&sub_hash)
            .map(|s| {
                (s.cooperation_score + s.trust_aptitude + s.sharing_aptitude
                    + s.fairness_aptitude)
                    / 4
            })
            .unwrap_or(0)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn evict_least_effective_lesson(&mut self) {
        let victim = self
            .effectiveness_index
            .iter()
            .min_by_key(|(_, &v)| v)
            .map(|(&k, _)| k);
        if let Some(k) = victim {
            self.lessons.remove(&k);
            self.effectiveness_index.remove(k);
            self.stats.lessons_pruned += 1;
        }
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;

        // Decay effectiveness scores
        let keys: Vec<u64> = self.effectiveness_index.keys().copied().collect();
        for k in keys {
            if let Some(v) = self.effectiveness_index.get_mut(&k) {
                *v = (*v * EFFECTIVENESS_DECAY_NUM) / EFFECTIVENESS_DECAY_DEN;
            }
        }

        // Stochastic curriculum evolution
        let r = xorshift64(&mut self.rng_state) % 100;
        if r < 5 {
            let cids: Vec<u64> = self.curricula.keys().copied().collect();
            if let Some(&cid) = cids.first() {
                if let Some(cur) = self.curricula.get_mut(&cid) {
                    cur.generation += 1;
                    // Shuffle one lesson position for variation
                    if cur.lesson_ids.len() > 1 {
                        let idx = xorshift64(&mut self.rng_state) as usize % cur.lesson_ids.len();
                        let swap_idx =
                            xorshift64(&mut self.rng_state) as usize % cur.lesson_ids.len();
                        cur.lesson_ids.swap(idx, swap_idx);
                    }
                }
            }
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &TeachingStats {
        &self.stats
    }

    #[inline(always)]
    pub fn lesson_count(&self) -> usize {
        self.lessons.len()
    }

    #[inline(always)]
    pub fn student_count(&self) -> usize {
        self.students.len()
    }

    #[inline(always)]
    pub fn curriculum_count(&self) -> usize {
        self.curricula.len()
    }
}
