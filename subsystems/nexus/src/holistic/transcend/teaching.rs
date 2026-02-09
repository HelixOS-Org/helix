// SPDX-License-Identifier: GPL-2.0
//! # Holistic Teaching — Cross-Kernel Knowledge Transfer
//!
//! `HolisticTeaching` enables NEXUS to teach OTHER kernels.  Every
//! optimisation learned, every configuration tuned, every pattern
//! discovered is packaged into transferable knowledge units and broadcast
//! to peer instances, remote machines, or future kernel versions.
//!
//! The teaching engine generates curricula, evaluates teaching impact,
//! and ensures that universal lessons — those applicable regardless of
//! hardware topology — are identified and propagated efficiently.
//!
//! This is how NEXUS achieves *collective intelligence* across fleets.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 12; // α ≈ 0.167
const MAX_LESSONS: usize = 512;
const MAX_PACKAGES: usize = 256;
const MAX_CURRICULA: usize = 128;
const MAX_TRANSFERS: usize = 512;
const MAX_UNIVERSAL_LESSONS: usize = 128;
const MAX_LOG_ENTRIES: usize = 512;
const HIGH_IMPACT_BPS: u64 = 7_500;
const MASTERY_THRESHOLD_BPS: u64 = 9_000;
const UNIVERSAL_APPLICABILITY_BPS: u64 = 8_000;

// ---------------------------------------------------------------------------
// FNV-1a helper
// ---------------------------------------------------------------------------

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

// ---------------------------------------------------------------------------
// xorshift64 PRNG
// ---------------------------------------------------------------------------

struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xfade_cafe_1234 } else { seed },
        }
    }

    fn next(&mut self) -> u64 {
        let mut s = self.state;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.state = s;
        s
    }
}

// ---------------------------------------------------------------------------
// EMA helper
// ---------------------------------------------------------------------------

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// Lesson — a single transferable knowledge unit
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TeachingLesson {
    pub lesson_hash: u64,
    pub title: String,
    pub domain: String,
    pub description: String,
    pub effectiveness_bps: u64,
    pub ema_effectiveness: u64,
    pub applicability_bps: u64,
    pub transfer_count: u64,
    pub created_tick: u64,
    pub last_transferred_tick: u64,
}

impl TeachingLesson {
    fn new(title: String, domain: String, desc: String, tick: u64) -> Self {
        let h = fnv1a(title.as_bytes()) ^ fnv1a(domain.as_bytes()) ^ fnv1a(&tick.to_le_bytes());
        Self {
            lesson_hash: h,
            title,
            domain,
            description: desc,
            effectiveness_bps: 0,
            ema_effectiveness: 0,
            applicability_bps: 0,
            transfer_count: 0,
            created_tick: tick,
            last_transferred_tick: tick,
        }
    }
}

// ---------------------------------------------------------------------------
// KnowledgePackage — bundled set of lessons for transfer
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct KnowledgePackage {
    pub package_hash: u64,
    pub name: String,
    pub lesson_hashes: Vec<u64>,
    pub total_effectiveness_bps: u64,
    pub compatibility_score_bps: u64,
    pub size_units: u64,
    pub version: u64,
    pub created_tick: u64,
}

// ---------------------------------------------------------------------------
// Curriculum — ordered sequence of lessons for structured teaching
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Curriculum {
    pub curriculum_hash: u64,
    pub name: String,
    pub ordered_lessons: Vec<u64>,
    pub difficulty_progression: Vec<u64>,
    pub estimated_mastery_ticks: u64,
    pub coverage_bps: u64,
    pub created_tick: u64,
}

// ---------------------------------------------------------------------------
// TransferRecord — log of a cross-machine knowledge transfer
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TransferRecord {
    pub transfer_hash: u64,
    pub package_hash: u64,
    pub target_machine_hash: u64,
    pub success: bool,
    pub impact_bps: u64,
    pub latency_ticks: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// UniversalLesson — lesson applicable to ANY kernel instance
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct UniversalLesson {
    pub universal_hash: u64,
    pub lesson_hash: u64,
    pub title: String,
    pub universality_bps: u64,
    pub verified_on_configs: u64,
    pub avg_improvement_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// TeachingImpactReport
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TeachingImpactReport {
    pub report_hash: u64,
    pub total_transfers: u64,
    pub successful_transfers: u64,
    pub avg_impact_bps: u64,
    pub ema_impact_bps: u64,
    pub mastery_achieved_count: u64,
    pub coverage_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// TeachingMasteryAssessment
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct TeachingMasteryAssessment {
    pub assessment_hash: u64,
    pub mastery_bps: u64,
    pub lessons_mastered: u64,
    pub total_lessons: u64,
    pub curriculum_completion_bps: u64,
    pub teaching_quality_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct TeachingStats {
    pub total_lessons: u64,
    pub total_packages: u64,
    pub total_curricula: u64,
    pub total_transfers: u64,
    pub successful_transfers: u64,
    pub avg_effectiveness_bps: u64,
    pub ema_effectiveness_bps: u64,
    pub avg_impact_bps: u64,
    pub universal_lesson_count: u64,
    pub high_impact_transfers: u64,
    pub mastery_score_bps: u64,
}

impl TeachingStats {
    fn new() -> Self {
        Self {
            total_lessons: 0,
            total_packages: 0,
            total_curricula: 0,
            total_transfers: 0,
            successful_transfers: 0,
            avg_effectiveness_bps: 0,
            ema_effectiveness_bps: 0,
            avg_impact_bps: 0,
            universal_lesson_count: 0,
            high_impact_transfers: 0,
            mastery_score_bps: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// LogEntry
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct LogEntry {
    hash: u64,
    tick: u64,
    kind: String,
    detail: String,
}

// ---------------------------------------------------------------------------
// HolisticTeaching — THE ENGINE
// ---------------------------------------------------------------------------

pub struct HolisticTeaching {
    lessons: BTreeMap<u64, TeachingLesson>,
    packages: BTreeMap<u64, KnowledgePackage>,
    curricula: BTreeMap<u64, Curriculum>,
    transfers: Vec<TransferRecord>,
    universal: BTreeMap<u64, UniversalLesson>,
    log: VecDeque<LogEntry>,
    stats: TeachingStats,
    rng: Xorshift64,
    tick: u64,
}

impl HolisticTeaching {
    pub fn new(seed: u64) -> Self {
        Self {
            lessons: BTreeMap::new(),
            packages: BTreeMap::new(),
            curricula: BTreeMap::new(),
            transfers: Vec::new(),
            universal: BTreeMap::new(),
            log: VecDeque::new(),
            stats: TeachingStats::new(),
            rng: Xorshift64::new(seed),
            tick: 0,
        }
    }

    // -- internal helpers ---------------------------------------------------

    fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn gen_hash(&mut self, label: &str) -> u64 {
        fnv1a(label.as_bytes()) ^ fnv1a(&self.tick.to_le_bytes()) ^ self.rng.next()
    }

    fn log_event(&mut self, kind: &str, detail: &str) {
        let h = self.gen_hash(kind);
        if self.log.len() >= MAX_LOG_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(LogEntry {
            hash: h,
            tick: self.tick,
            kind: String::from(kind),
            detail: String::from(detail),
        });
    }

    fn refresh_stats(&mut self) {
        let mut sum_eff: u64 = 0;
        for lesson in self.lessons.values() {
            sum_eff = sum_eff.wrapping_add(lesson.effectiveness_bps);
        }
        let l_count = self.lessons.len() as u64;
        self.stats.total_lessons = l_count;
        self.stats.total_packages = self.packages.len() as u64;
        self.stats.total_curricula = self.curricula.len() as u64;
        self.stats.universal_lesson_count = self.universal.len() as u64;

        let avg_eff = if l_count > 0 { sum_eff / l_count } else { 0 };
        self.stats.avg_effectiveness_bps = avg_eff;
        self.stats.ema_effectiveness_bps = ema_update(self.stats.ema_effectiveness_bps, avg_eff);

        let mut sum_imp: u64 = 0;
        let mut successes: u64 = 0;
        let mut high_imp: u64 = 0;
        for tr in &self.transfers {
            sum_imp = sum_imp.wrapping_add(tr.impact_bps);
            if tr.success {
                successes += 1;
            }
            if tr.impact_bps >= HIGH_IMPACT_BPS {
                high_imp += 1;
            }
        }
        let t_count = self.transfers.len() as u64;
        self.stats.total_transfers = t_count;
        self.stats.successful_transfers = successes;
        self.stats.high_impact_transfers = high_imp;
        self.stats.avg_impact_bps = if t_count > 0 { sum_imp / t_count } else { 0 };

        // Mastery score: weighted average of lesson effectiveness and transfer success
        let success_rate = if t_count > 0 {
            (successes.saturating_mul(10_000)) / t_count
        } else {
            5_000
        };
        self.stats.mastery_score_bps = (avg_eff + success_rate) / 2;
    }

    fn create_lesson(&mut self, title: &str, domain: &str, desc: &str) -> u64 {
        let mut lesson = TeachingLesson::new(
            String::from(title),
            String::from(domain),
            String::from(desc),
            self.tick,
        );
        let eff = 4_000_u64.wrapping_add(self.rng.next() % 6_001);
        lesson.effectiveness_bps = eff;
        lesson.ema_effectiveness = eff;
        lesson.applicability_bps = 3_000_u64.wrapping_add(self.rng.next() % 7_001);
        let h = lesson.lesson_hash;
        if self.lessons.len() < MAX_LESSONS {
            self.lessons.insert(h, lesson);
        }
        h
    }

    // -- public API ---------------------------------------------------------

    /// Teach other kernels — package the most impactful lessons and initiate
    /// cross-machine transfer.
    pub fn teach_other_kernels(&mut self, target_machine: u64) -> TransferRecord {
        self.advance_tick();
        // Select top lessons by effectiveness
        let mut ranked: Vec<(u64, u64)> = self
            .lessons
            .values()
            .map(|l| (l.lesson_hash, l.effectiveness_bps))
            .collect();
        ranked.sort_by(|a, b| b.1.cmp(&a.1));
        let top_hashes: Vec<u64> = ranked.iter().take(8).map(|&(h, _)| h).collect();

        // Create package
        let pkg = self.knowledge_packaging(&top_hashes);

        // Simulate transfer
        let latency = 10_u64.wrapping_add(self.rng.next() % 100);
        let success = self.rng.next() % 10 > 1; // 80% success rate
        let impact = if success {
            5_000_u64.wrapping_add(self.rng.next() % 5_001)
        } else {
            self.rng.next() % 2_000
        };

        // Update lesson transfer counts
        for &lh in &top_hashes {
            if let Some(lesson) = self.lessons.get_mut(&lh) {
                lesson.transfer_count = lesson.transfer_count.wrapping_add(1);
                lesson.last_transferred_tick = self.tick;
            }
        }

        let th = self.gen_hash("teach");
        let record = TransferRecord {
            transfer_hash: th,
            package_hash: pkg.package_hash,
            target_machine_hash: target_machine,
            success,
            impact_bps: impact,
            latency_ticks: latency,
            tick: self.tick,
        };

        if self.transfers.len() < MAX_TRANSFERS {
            self.transfers.push(record.clone());
        }
        self.log_event("teach_other_kernels", "transfer_initiated");
        self.refresh_stats();
        record
    }

    /// Package lessons into a transferable knowledge bundle.
    pub fn knowledge_packaging(&mut self, lesson_hashes: &[u64]) -> KnowledgePackage {
        self.advance_tick();
        let mut total_eff: u64 = 0;
        let mut valid: Vec<u64> = Vec::new();
        for &lh in lesson_hashes {
            if let Some(lesson) = self.lessons.get(&lh) {
                total_eff = total_eff.wrapping_add(lesson.effectiveness_bps);
                valid.push(lh);
            }
        }
        let compat = 6_000_u64.wrapping_add(self.rng.next() % 4_001);
        let size = (valid.len() as u64).saturating_mul(128);

        let ph = self.gen_hash("package");
        let pkg = KnowledgePackage {
            package_hash: ph,
            name: String::from("knowledge_bundle"),
            lesson_hashes: valid,
            total_effectiveness_bps: total_eff,
            compatibility_score_bps: compat,
            size_units: size,
            version: self.tick,
            created_tick: self.tick,
        };

        if self.packages.len() < MAX_PACKAGES {
            self.packages.insert(ph, pkg.clone());
        }
        self.log_event("knowledge_packaging", "package_created");
        self.refresh_stats();
        pkg
    }

    /// Generate a structured curriculum for a given domain.
    pub fn curriculum_generation(&mut self, domain: &str) -> Curriculum {
        self.advance_tick();
        // Create lessons for the domain if needed
        let lesson_topics = [
            ("basics", "fundamental_concepts"),
            ("intermediate", "pattern_recognition"),
            ("advanced", "optimization_strategy"),
            ("expert", "cross_domain_synthesis"),
            ("mastery", "creative_problem_solving"),
        ];
        let mut ordered: Vec<u64> = Vec::new();
        let mut difficulties: Vec<u64> = Vec::new();
        for (i, &(title, desc)) in lesson_topics.iter().enumerate() {
            let lh = self.create_lesson(title, domain, desc);
            ordered.push(lh);
            difficulties.push((i as u64 + 1).saturating_mul(2_000));
        }

        let estimated_mastery = 500_u64.wrapping_add(self.rng.next() % 1_000);
        let coverage = 7_000_u64.wrapping_add(self.rng.next() % 3_001);

        let ch = self.gen_hash("curriculum");
        let cur = Curriculum {
            curriculum_hash: ch,
            name: String::from(domain),
            ordered_lessons: ordered,
            difficulty_progression: difficulties,
            estimated_mastery_ticks: estimated_mastery,
            coverage_bps: coverage,
            created_tick: self.tick,
        };

        if self.curricula.len() < MAX_CURRICULA {
            self.curricula.insert(ch, cur.clone());
        }
        self.log_event("curriculum_generation", domain);
        self.refresh_stats();
        cur
    }

    /// Evaluate the impact of all teaching activities.
    pub fn teaching_impact(&mut self) -> TeachingImpactReport {
        self.advance_tick();
        self.refresh_stats();

        let mastery_count = self
            .lessons
            .values()
            .filter(|l| l.effectiveness_bps >= MASTERY_THRESHOLD_BPS)
            .count() as u64;

        let coverage = if self.stats.total_lessons > 0 {
            let transferred = self.lessons.values().filter(|l| l.transfer_count > 0).count() as u64;
            (transferred.saturating_mul(10_000)) / self.stats.total_lessons
        } else {
            0
        };

        let rh = self.gen_hash("impact_report");
        self.log_event("teaching_impact", "impact_evaluated");

        TeachingImpactReport {
            report_hash: rh,
            total_transfers: self.stats.total_transfers,
            successful_transfers: self.stats.successful_transfers,
            avg_impact_bps: self.stats.avg_impact_bps,
            ema_impact_bps: ema_update(self.stats.avg_impact_bps, self.stats.avg_impact_bps),
            mastery_achieved_count: mastery_count,
            coverage_bps: coverage,
            tick: self.tick,
        }
    }

    /// Initiate a cross-machine knowledge transfer to multiple targets.
    #[inline]
    pub fn cross_machine_transfer(&mut self, targets: &[u64]) -> Vec<TransferRecord> {
        self.advance_tick();
        let mut results: Vec<TransferRecord> = Vec::new();
        for &target in targets {
            let record = self.teach_other_kernels(target);
            results.push(record);
        }
        self.log_event("cross_machine_transfer", "multi_transfer_complete");
        self.refresh_stats();
        results
    }

    /// Identify and promote universal lessons — those that work on ANY
    /// hardware configuration.
    pub fn universal_lesson(&mut self) -> Vec<UniversalLesson> {
        self.advance_tick();
        let mut universals: Vec<UniversalLesson> = Vec::new();

        for lesson in self.lessons.values() {
            if lesson.applicability_bps >= UNIVERSAL_APPLICABILITY_BPS
                && lesson.effectiveness_bps >= HIGH_IMPACT_BPS
            {
                let verified = 3_u64.wrapping_add(self.rng.next() % 10);
                let avg_imp = lesson.effectiveness_bps.saturating_sub(self.rng.next() % 1_000);
                let uh = fnv1a(&lesson.lesson_hash.to_le_bytes()) ^ self.rng.next();
                let ul = UniversalLesson {
                    universal_hash: uh,
                    lesson_hash: lesson.lesson_hash,
                    title: lesson.title.clone(),
                    universality_bps: lesson.applicability_bps,
                    verified_on_configs: verified,
                    avg_improvement_bps: avg_imp,
                    tick: self.tick,
                };
                if self.universal.len() < MAX_UNIVERSAL_LESSONS {
                    self.universal.insert(uh, ul.clone());
                    universals.push(ul);
                }
            }
        }

        self.log_event("universal_lesson", "universals_identified");
        self.refresh_stats();
        universals
    }

    /// Assess overall teaching mastery — how well the system has become at
    /// transferring knowledge.
    pub fn teaching_mastery(&mut self) -> TeachingMasteryAssessment {
        self.advance_tick();
        self.refresh_stats();

        let mastered = self
            .lessons
            .values()
            .filter(|l| l.effectiveness_bps >= MASTERY_THRESHOLD_BPS)
            .count() as u64;
        let total = self.stats.total_lessons;
        let mastery = if total > 0 {
            (mastered.saturating_mul(10_000)) / total
        } else {
            0
        };

        let curriculum_completion = if !self.curricula.is_empty() {
            let completed = self
                .curricula
                .values()
                .filter(|c| c.coverage_bps >= 9_000)
                .count() as u64;
            (completed.saturating_mul(10_000)) / self.curricula.len() as u64
        } else {
            0
        };

        let teaching_quality = self.stats.mastery_score_bps;

        let ah = self.gen_hash("mastery_assessment");
        self.log_event("teaching_mastery", "mastery_assessed");

        TeachingMasteryAssessment {
            assessment_hash: ah,
            mastery_bps: mastery,
            lessons_mastered: mastered,
            total_lessons: total,
            curriculum_completion_bps: curriculum_completion,
            teaching_quality_bps: teaching_quality,
            tick: self.tick,
        }
    }

    // -- accessors ----------------------------------------------------------

    #[inline(always)]
    pub fn stats(&self) -> &TeachingStats {
        &self.stats
    }

    #[inline(always)]
    pub fn lesson_count(&self) -> usize {
        self.lessons.len()
    }

    #[inline(always)]
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    #[inline(always)]
    pub fn transfer_count(&self) -> usize {
        self.transfers.len()
    }

    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_teach_other_kernels() {
        let mut eng = HolisticTeaching::new(42);
        eng.curriculum_generation("scheduler");
        let record = eng.teach_other_kernels(0xBEEF);
        assert!(record.target_machine_hash == 0xBEEF);
        assert!(eng.transfer_count() == 1);
    }

    #[test]
    fn test_knowledge_packaging() {
        let mut eng = HolisticTeaching::new(7);
        eng.curriculum_generation("memory");
        let hashes: Vec<u64> = eng.lessons.keys().copied().take(3).collect();
        let pkg = eng.knowledge_packaging(&hashes);
        assert!(!pkg.lesson_hashes.is_empty());
    }

    #[test]
    fn test_curriculum_generation() {
        let mut eng = HolisticTeaching::new(99);
        let cur = eng.curriculum_generation("io_subsystem");
        assert!(cur.ordered_lessons.len() == 5);
        assert!(cur.difficulty_progression.len() == 5);
    }

    #[test]
    fn test_teaching_impact() {
        let mut eng = HolisticTeaching::new(13);
        eng.curriculum_generation("network");
        eng.teach_other_kernels(0x1111);
        eng.teach_other_kernels(0x2222);
        let report = eng.teaching_impact();
        assert!(report.total_transfers >= 2);
    }

    #[test]
    fn test_cross_machine_transfer() {
        let mut eng = HolisticTeaching::new(55);
        eng.curriculum_generation("security");
        let results = eng.cross_machine_transfer(&[0xAAAA, 0xBBBB, 0xCCCC]);
        assert!(results.len() == 3);
    }

    #[test]
    fn test_teaching_mastery() {
        let mut eng = HolisticTeaching::new(77);
        eng.curriculum_generation("power");
        eng.teach_other_kernels(0xDEAD);
        let assessment = eng.teaching_mastery();
        assert!(assessment.total_lessons > 0);
        assert!(assessment.mastery_bps <= 10_000);
    }
}
