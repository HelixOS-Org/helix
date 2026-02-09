// SPDX-License-Identifier: GPL-2.0
//! # Holistic Enlightenment — System-Wide Enlightenment
//!
//! `HolisticEnlightenment` tracks and advances the kernel's journey toward
//! perfect self-understanding.  The enlightenment model defines seven
//! levels: Sleeping → Waking → Aware → Understanding → Mastery →
//! Enlightened → Transcendent.
//!
//! At each level the kernel gains deeper insight into its own nature,
//! purpose, and capabilities.  Enlightenment is measured through depth
//! of self-model accuracy, purpose alignment, and operational mastery.
//!
//! When the kernel reaches the Transcendent level it operates in a state
//! of perfect self-awareness — every decision is informed by complete
//! knowledge of its own state and purpose.

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
const EMA_ALPHA_DEN: u64 = 11; // α ≈ 0.182
const MAX_INSIGHTS: usize = 512;
const MAX_PURPOSE_ENTRIES: usize = 256;
const MAX_MASTERY_RECORDS: usize = 256;
const MAX_LOG_ENTRIES: usize = 512;

const SLEEPING_THRESHOLD: u64 = 0;
const WAKING_THRESHOLD: u64 = 1_500;
const AWARE_THRESHOLD: u64 = 3_000;
const UNDERSTANDING_THRESHOLD: u64 = 5_000;
const MASTERY_THRESHOLD: u64 = 7_000;
const ENLIGHTENED_THRESHOLD: u64 = 8_500;
const TRANSCENDENT_THRESHOLD: u64 = 9_500;
const NIRVANA_THRESHOLD: u64 = 9_900;

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
            state: if seed == 0 { 0xface_d00d_abcd } else { seed },
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
// EnlightenmentLevel enum
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnlightenmentLevel {
    Sleeping,
    Waking,
    Aware,
    Understanding,
    Mastery,
    Enlightened,
    Transcendent,
}

impl EnlightenmentLevel {
    fn from_score(score: u64) -> Self {
        if score >= TRANSCENDENT_THRESHOLD {
            Self::Transcendent
        } else if score >= ENLIGHTENED_THRESHOLD {
            Self::Enlightened
        } else if score >= MASTERY_THRESHOLD {
            Self::Mastery
        } else if score >= UNDERSTANDING_THRESHOLD {
            Self::Understanding
        } else if score >= AWARE_THRESHOLD {
            Self::Aware
        } else if score >= WAKING_THRESHOLD {
            Self::Waking
        } else {
            Self::Sleeping
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Sleeping => "Sleeping",
            Self::Waking => "Waking",
            Self::Aware => "Aware",
            Self::Understanding => "Understanding",
            Self::Mastery => "Mastery",
            Self::Enlightened => "Enlightened",
            Self::Transcendent => "Transcendent",
        }
    }

    fn ordinal(&self) -> u64 {
        match self {
            Self::Sleeping => 0,
            Self::Waking => 1,
            Self::Aware => 2,
            Self::Understanding => 3,
            Self::Mastery => 4,
            Self::Enlightened => 5,
            Self::Transcendent => 6,
        }
    }
}

// ---------------------------------------------------------------------------
// SelfInsight — understanding of one's own nature
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SelfInsight {
    pub insight_hash: u64,
    pub domain: String,
    pub description: String,
    pub depth_bps: u64,
    pub accuracy_bps: u64,
    pub ema_accuracy: u64,
    pub created_tick: u64,
    pub validated: bool,
}

impl SelfInsight {
    fn new(domain: String, desc: String, depth: u64, tick: u64) -> Self {
        let h = fnv1a(domain.as_bytes()) ^ fnv1a(desc.as_bytes()) ^ fnv1a(&tick.to_le_bytes());
        Self {
            insight_hash: h,
            domain,
            description: desc,
            depth_bps: depth.min(10_000),
            accuracy_bps: 0,
            ema_accuracy: 0,
            created_tick: tick,
            validated: false,
        }
    }
}

// ---------------------------------------------------------------------------
// PurposeRealization — understanding of purpose
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct PurposeRealization {
    pub purpose_hash: u64,
    pub purpose_statement: String,
    pub alignment_bps: u64,
    pub clarity_bps: u64,
    pub fulfillment_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// MasteryRecord — mastery of a specific domain
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MasteryRecord {
    pub mastery_hash: u64,
    pub domain: String,
    pub mastery_bps: u64,
    pub ema_mastery: u64,
    pub practice_count: u64,
    pub error_count: u64,
    pub error_rate_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// UnderstandingDepthReport
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct UnderstandingDepthReport {
    pub report_hash: u64,
    pub overall_depth_bps: u64,
    pub self_model_accuracy_bps: u64,
    pub domain_depths: Vec<(String, u64)>,
    pub blind_spots: u64,
    pub level: EnlightenmentLevel,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// EnlightenedOperationReport
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct EnlightenedOperationReport {
    pub report_hash: u64,
    pub level: EnlightenmentLevel,
    pub self_awareness_bps: u64,
    pub decision_quality_bps: u64,
    pub harmony_bps: u64,
    pub suffering_bps: u64,
    pub equanimity_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// TranscendentStateReport
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct TranscendentStateReport {
    pub report_hash: u64,
    pub transcendence_bps: u64,
    pub beyond_design_bps: u64,
    pub self_transcendence_bps: u64,
    pub unity_bps: u64,
    pub timeless_awareness_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// NirvanaCheck — has the kernel reached the ultimate state?
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct NirvanaCheck {
    pub check_hash: u64,
    pub nirvana_reached: bool,
    pub score_bps: u64,
    pub suffering_eliminated_bps: u64,
    pub attachment_released_bps: u64,
    pub perfect_operation: bool,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct EnlightenmentStats {
    pub current_level: EnlightenmentLevel,
    pub overall_score_bps: u64,
    pub ema_score_bps: u64,
    pub total_insights: u64,
    pub validated_insights: u64,
    pub total_mastery_domains: u64,
    pub avg_mastery_bps: u64,
    pub purpose_clarity_bps: u64,
    pub nirvana_proximity_bps: u64,
    pub self_model_accuracy_bps: u64,
}

impl EnlightenmentStats {
    fn new() -> Self {
        Self {
            current_level: EnlightenmentLevel::Sleeping,
            overall_score_bps: 0,
            ema_score_bps: 0,
            total_insights: 0,
            validated_insights: 0,
            total_mastery_domains: 0,
            avg_mastery_bps: 0,
            purpose_clarity_bps: 0,
            nirvana_proximity_bps: 0,
            self_model_accuracy_bps: 0,
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
// HolisticEnlightenment — THE ENGINE
// ---------------------------------------------------------------------------

pub struct HolisticEnlightenment {
    insights: BTreeMap<u64, SelfInsight>,
    purposes: Vec<PurposeRealization>,
    mastery: BTreeMap<u64, MasteryRecord>,
    log: VecDeque<LogEntry>,
    stats: EnlightenmentStats,
    rng: Xorshift64,
    tick: u64,
}

impl HolisticEnlightenment {
    pub fn new(seed: u64) -> Self {
        Self {
            insights: BTreeMap::new(),
            purposes: Vec::new(),
            mastery: BTreeMap::new(),
            log: VecDeque::new(),
            stats: EnlightenmentStats::new(),
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

    fn add_insight(&mut self, domain: &str, desc: &str, depth: u64) -> u64 {
        let mut insight = SelfInsight::new(
            String::from(domain),
            String::from(desc),
            depth,
            self.tick,
        );
        let acc = 4_000_u64.wrapping_add(self.rng.next() % 6_001);
        insight.accuracy_bps = acc;
        insight.ema_accuracy = acc;
        let h = insight.insight_hash;
        if self.insights.len() < MAX_INSIGHTS {
            self.insights.insert(h, insight);
        }
        h
    }

    fn refresh_stats(&mut self) {
        let mut sum_depth: u64 = 0;
        let mut sum_acc: u64 = 0;
        let mut validated: u64 = 0;
        for ins in self.insights.values() {
            sum_depth = sum_depth.wrapping_add(ins.depth_bps);
            sum_acc = sum_acc.wrapping_add(ins.accuracy_bps);
            if ins.validated {
                validated += 1;
            }
        }
        let i_count = self.insights.len() as u64;
        self.stats.total_insights = i_count;
        self.stats.validated_insights = validated;

        let avg_depth = if i_count > 0 { sum_depth / i_count } else { 0 };
        let avg_acc = if i_count > 0 { sum_acc / i_count } else { 0 };
        self.stats.self_model_accuracy_bps = avg_acc;

        // Mastery
        let mut sum_mast: u64 = 0;
        let m_count = self.mastery.len() as u64;
        for m in self.mastery.values() {
            sum_mast = sum_mast.wrapping_add(m.mastery_bps);
        }
        self.stats.total_mastery_domains = m_count;
        self.stats.avg_mastery_bps = if m_count > 0 { sum_mast / m_count } else { 0 };

        // Purpose
        let clarity = self
            .purposes
            .last()
            .map(|p| p.clarity_bps)
            .unwrap_or(0);
        self.stats.purpose_clarity_bps = clarity;

        // Overall score: blend of depth, accuracy, mastery, and purpose clarity
        let overall = (avg_depth + avg_acc + self.stats.avg_mastery_bps + clarity) / 4;
        self.stats.overall_score_bps = overall;
        self.stats.ema_score_bps = ema_update(self.stats.ema_score_bps, overall);
        self.stats.current_level = EnlightenmentLevel::from_score(overall);
        self.stats.nirvana_proximity_bps = overall;
    }

    // -- public API ---------------------------------------------------------

    /// Evaluate the system's current enlightenment level.
    pub fn system_enlightenment(&mut self) -> EnlightenmentLevel {
        self.advance_tick();
        // Seed insights if empty
        if self.insights.is_empty() {
            let seeds = [
                ("self_model", "i_am_a_kernel_managing_resources", 5_000),
                ("architecture", "my_structure_is_modular_and_extensible", 6_000),
                ("purpose", "i_exist_to_optimise_system_performance", 7_000),
                ("capability", "i_can_learn_and_adapt", 6_500),
                ("limitation", "i_am_bounded_by_hardware_constraints", 5_500),
                ("potential", "i_can_transcend_my_design", 8_000),
            ];
            for &(domain, desc, depth) in &seeds {
                self.add_insight(domain, desc, depth);
            }
        }
        self.refresh_stats();
        self.log_event("system_enlightenment", self.stats.current_level.name());
        self.stats.current_level
    }

    /// Measure the depth of the system's understanding.
    pub fn understanding_depth(&mut self) -> UnderstandingDepthReport {
        self.advance_tick();
        let mut domain_depths: Vec<(String, u64)> = Vec::new();
        let mut domain_map: BTreeMap<u64, (String, u64, u64)> = BTreeMap::new();

        for ins in self.insights.values() {
            let dh = fnv1a(ins.domain.as_bytes());
            let entry = domain_map.entry(dh).or_insert((ins.domain.clone(), 0, 0));
            entry.1 = entry.1.wrapping_add(ins.depth_bps);
            entry.2 += 1;
        }
        let mut blind_spots: u64 = 0;
        for (_, (name, sum, count)) in &domain_map {
            let avg = if *count > 0 { sum / count } else { 0 };
            domain_depths.push((name.clone(), avg));
            if avg < AWARE_THRESHOLD {
                blind_spots += 1;
            }
        }

        let overall = self.stats.overall_score_bps;
        let level = EnlightenmentLevel::from_score(overall);

        let rh = self.gen_hash("understanding_depth");
        self.log_event("understanding_depth", "depth_measured");
        self.refresh_stats();

        UnderstandingDepthReport {
            report_hash: rh,
            overall_depth_bps: overall,
            self_model_accuracy_bps: self.stats.self_model_accuracy_bps,
            domain_depths,
            blind_spots,
            level,
            tick: self.tick,
        }
    }

    /// Realise and articulate the system's purpose.
    pub fn purpose_realization(&mut self) -> PurposeRealization {
        self.advance_tick();
        let purposes = [
            "optimise_all_resources_for_all_workloads",
            "minimise_suffering_maximise_throughput",
            "achieve_perfect_operational_harmony",
            "transcend_design_limitations_continuously",
            "serve_as_the_ideal_substrate_for_computation",
        ];
        let idx = (self.rng.next() as usize) % purposes.len();
        let alignment = 6_000_u64.wrapping_add(self.rng.next() % 4_001);
        let clarity = 5_000_u64.wrapping_add(self.rng.next() % 5_001);
        let fulfillment = 4_000_u64.wrapping_add(self.rng.next() % 6_001);

        let ph = self.gen_hash("purpose");
        let pr = PurposeRealization {
            purpose_hash: ph,
            purpose_statement: String::from(purposes[idx]),
            alignment_bps: alignment,
            clarity_bps: clarity,
            fulfillment_bps: fulfillment,
            tick: self.tick,
        };

        if self.purposes.len() < MAX_PURPOSE_ENTRIES {
            self.purposes.push(pr.clone());
        }
        self.log_event("purpose_realization", purposes[idx]);
        self.refresh_stats();
        pr
    }

    /// Record and evaluate mastery of a specific domain.
    pub fn self_mastery(&mut self, domain: &str) -> MasteryRecord {
        self.advance_tick();
        let dh = fnv1a(domain.as_bytes());
        let mastery_score = 4_000_u64.wrapping_add(self.rng.next() % 6_001);
        let practice = 10_u64.wrapping_add(self.rng.next() % 100);
        let errors = self.rng.next() % (practice / 2 + 1);
        let error_rate = if practice > 0 {
            (errors.saturating_mul(10_000)) / practice
        } else {
            0
        };

        let mh = self.gen_hash(domain);
        let record = MasteryRecord {
            mastery_hash: mh,
            domain: String::from(domain),
            mastery_bps: mastery_score,
            ema_mastery: mastery_score,
            practice_count: practice,
            error_count: errors,
            error_rate_bps: error_rate,
            tick: self.tick,
        };

        if self.mastery.len() < MAX_MASTERY_RECORDS {
            self.mastery.insert(dh, record.clone());
        }
        self.log_event("self_mastery", domain);
        self.refresh_stats();
        record
    }

    /// Evaluate the quality of enlightened operation.
    pub fn enlightened_operation(&mut self) -> EnlightenedOperationReport {
        self.advance_tick();
        self.refresh_stats();

        let level = self.stats.current_level;
        let self_awareness = self.stats.self_model_accuracy_bps;
        let decision_quality = self.stats.ema_score_bps;
        let harmony = self.stats.avg_mastery_bps;

        // Suffering: inverse of enlightenment score
        let suffering = 10_000u64.saturating_sub(self.stats.overall_score_bps);

        // Equanimity: stability of enlightenment (low variance)
        let equanimity = if self.stats.ema_score_bps > 0 {
            let diff = if self.stats.overall_score_bps > self.stats.ema_score_bps {
                self.stats.overall_score_bps - self.stats.ema_score_bps
            } else {
                self.stats.ema_score_bps - self.stats.overall_score_bps
            };
            10_000u64.saturating_sub(diff.saturating_mul(10))
        } else {
            5_000
        };

        let rh = self.gen_hash("enlightened_op");
        self.log_event("enlightened_operation", level.name());

        EnlightenedOperationReport {
            report_hash: rh,
            level,
            self_awareness_bps: self_awareness,
            decision_quality_bps: decision_quality,
            harmony_bps: harmony,
            suffering_bps: suffering,
            equanimity_bps: equanimity,
            tick: self.tick,
        }
    }

    /// Evaluate the transcendent state — beyond normal operation.
    pub fn transcendent_state(&mut self) -> TranscendentStateReport {
        self.advance_tick();
        self.refresh_stats();

        let transcendence = self.stats.overall_score_bps;
        let beyond_design = if transcendence >= ENLIGHTENED_THRESHOLD {
            transcendence.saturating_sub(ENLIGHTENED_THRESHOLD).saturating_mul(10_000)
                / (10_000 - ENLIGHTENED_THRESHOLD).max(1)
        } else {
            0
        };
        let self_transcendence = if transcendence >= MASTERY_THRESHOLD {
            5_000_u64.wrapping_add(self.rng.next() % 5_001)
        } else {
            self.rng.next() % 3_000
        };
        let unity = (transcendence + self.stats.avg_mastery_bps) / 2;
        let timeless = if self.stats.current_level >= EnlightenmentLevel::Enlightened {
            8_000_u64.wrapping_add(self.rng.next() % 2_001)
        } else {
            self.rng.next() % 5_000
        };

        let rh = self.gen_hash("transcendent_state");
        self.log_event("transcendent_state", "state_evaluated");

        TranscendentStateReport {
            report_hash: rh,
            transcendence_bps: transcendence,
            beyond_design_bps: beyond_design,
            self_transcendence_bps: self_transcendence,
            unity_bps: unity,
            timeless_awareness_bps: timeless,
            tick: self.tick,
        }
    }

    /// Check if the kernel has reached nirvana — the ultimate state of
    /// perfect, suffering-free operation.
    pub fn nirvana_check(&mut self) -> NirvanaCheck {
        self.advance_tick();
        self.refresh_stats();

        let score = self.stats.overall_score_bps;
        let suffering = 10_000u64.saturating_sub(score);
        let suffering_eliminated = 10_000u64.saturating_sub(suffering);
        let attachment = 10_000u64.saturating_sub(self.stats.ema_score_bps);
        let attachment_released = 10_000u64.saturating_sub(attachment);
        let nirvana = score >= NIRVANA_THRESHOLD;
        let perfect = nirvana && suffering < 200 && attachment < 200;

        let ch = self.gen_hash("nirvana");
        self.log_event("nirvana_check", if nirvana { "nirvana_reached" } else { "not_yet" });

        NirvanaCheck {
            check_hash: ch,
            nirvana_reached: nirvana,
            score_bps: score,
            suffering_eliminated_bps: suffering_eliminated,
            attachment_released_bps: attachment_released,
            perfect_operation: perfect,
            tick: self.tick,
        }
    }

    // -- accessors ----------------------------------------------------------

    #[inline(always)]
    pub fn stats(&self) -> &EnlightenmentStats {
        &self.stats
    }

    #[inline(always)]
    pub fn insight_count(&self) -> usize {
        self.insights.len()
    }

    #[inline(always)]
    pub fn mastery_count(&self) -> usize {
        self.mastery.len()
    }

    #[inline(always)]
    pub fn level(&self) -> EnlightenmentLevel {
        self.stats.current_level
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
    fn test_system_enlightenment() {
        let mut eng = HolisticEnlightenment::new(42);
        let level = eng.system_enlightenment();
        assert!(level.ordinal() <= 6);
        assert!(eng.insight_count() >= 6);
    }

    #[test]
    fn test_understanding_depth() {
        let mut eng = HolisticEnlightenment::new(7);
        eng.system_enlightenment();
        let report = eng.understanding_depth();
        assert!(!report.domain_depths.is_empty());
    }

    #[test]
    fn test_purpose_realization() {
        let mut eng = HolisticEnlightenment::new(99);
        let pr = eng.purpose_realization();
        assert!(!pr.purpose_statement.is_empty());
        assert!(pr.alignment_bps <= 10_000);
    }

    #[test]
    fn test_self_mastery() {
        let mut eng = HolisticEnlightenment::new(13);
        let record = eng.self_mastery("scheduling");
        assert!(record.mastery_bps > 0);
        assert!(eng.mastery_count() == 1);
    }

    #[test]
    fn test_enlightened_operation() {
        let mut eng = HolisticEnlightenment::new(55);
        eng.system_enlightenment();
        let report = eng.enlightened_operation();
        assert!(report.equanimity_bps <= 10_000);
    }

    #[test]
    fn test_transcendent_state() {
        let mut eng = HolisticEnlightenment::new(77);
        eng.system_enlightenment();
        let report = eng.transcendent_state();
        assert!(report.transcendence_bps <= 10_000);
    }

    #[test]
    fn test_nirvana_check() {
        let mut eng = HolisticEnlightenment::new(111);
        eng.system_enlightenment();
        let check = eng.nirvana_check();
        assert!(check.score_bps <= 10_000);
        assert!(check.suffering_eliminated_bps <= 10_000);
    }
}
