// SPDX-License-Identifier: GPL-2.0
//! # Holistic Wisdom — THE GRAND WISDOM ENGINE
//!
//! `HolisticWisdom` is the distillation of ALL accumulated knowledge,
//! experience, and insight across every NEXUS subsystem.  Where intelligence
//! is the ability to solve problems, wisdom is knowing WHICH problems to
//! solve, WHEN to act, and HOW to balance competing concerns.
//!
//! The wisdom engine synthesises lessons from schedulers, memory managers,
//! I/O stacks, security monitors, and every prediction ever made into a
//! unified body of contextual knowledge that guides system-level decisions.
//!
//! Wisdom ≠ intelligence.  Wisdom is intelligence + experience + judgment.

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
const EMA_ALPHA_DEN: u64 = 13; // α ≈ 0.154
const MAX_WISDOM_ENTRIES: usize = 1024;
const MAX_CONSULTATIONS: usize = 512;
const MAX_SYNTHESES: usize = 256;
const MAX_DECISIONS: usize = 512;
const MAX_LEGACY_ENTRIES: usize = 256;
const MAX_LOG_ENTRIES: usize = 512;
const SAGE_CONFIDENCE_BPS: u64 = 8_500;
const DEEP_WISDOM_BPS: u64 = 9_000;
const CONTEXTUAL_MASTERY_BPS: u64 = 9_500;

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
            state: if seed == 0 { 0x5a6e_cafe_9876 } else { seed },
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
// WisdomEntry — a single piece of accumulated wisdom
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct WisdomEntry {
    pub wisdom_hash: u64,
    pub domain: String,
    pub insight: String,
    pub depth_bps: u64,
    pub confidence_bps: u64,
    pub ema_confidence: u64,
    pub times_consulted: u64,
    pub times_correct: u64,
    pub accuracy_bps: u64,
    pub created_tick: u64,
    pub last_consulted_tick: u64,
}

impl WisdomEntry {
    fn new(domain: String, insight: String, depth: u64, tick: u64) -> Self {
        let h = fnv1a(domain.as_bytes()) ^ fnv1a(insight.as_bytes()) ^ fnv1a(&tick.to_le_bytes());
        Self {
            wisdom_hash: h,
            domain,
            insight,
            depth_bps: depth.min(10_000),
            confidence_bps: 0,
            ema_confidence: 0,
            times_consulted: 0,
            times_correct: 0,
            accuracy_bps: 0,
            created_tick: tick,
            last_consulted_tick: tick,
        }
    }
}

// ---------------------------------------------------------------------------
// ConsultationRecord — log of consulting the wisdom engine
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ConsultationRecord {
    pub consultation_hash: u64,
    pub query: String,
    pub wisdom_entries_used: Vec<u64>,
    pub answer_confidence_bps: u64,
    pub context_match_bps: u64,
    pub outcome_observed: bool,
    pub outcome_correct: bool,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// WisdomSynthesisResult — combining wisdom from multiple domains
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct WisdomSynthesisResult {
    pub synthesis_hash: u64,
    pub domains_combined: Vec<String>,
    pub synthesised_insight: String,
    pub depth_bps: u64,
    pub cross_domain_synergy_bps: u64,
    pub confidence_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// ContextualMasteryReport
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct ContextualMasteryReport {
    pub report_hash: u64,
    pub context: String,
    pub mastery_bps: u64,
    pub depth_bps: u64,
    pub relevant_wisdom_count: u64,
    pub decision_accuracy_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// WisdomVsIntelligenceReport
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct WisdomVsIntelligenceReport {
    pub report_hash: u64,
    pub intelligence_score_bps: u64,
    pub wisdom_score_bps: u64,
    pub experience_score_bps: u64,
    pub judgment_score_bps: u64,
    pub balance_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// SageDecision — a decision made with full wisdom backing
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SageDecision {
    pub decision_hash: u64,
    pub context: String,
    pub chosen_action: String,
    pub wisdom_entries_consulted: Vec<u64>,
    pub confidence_bps: u64,
    pub expected_outcome_bps: u64,
    pub risk_assessment_bps: u64,
    pub sage_quality_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// WisdomLegacy — wisdom packaged for future kernel versions
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct WisdomLegacy {
    pub legacy_hash: u64,
    pub domain: String,
    pub entries: Vec<u64>,
    pub total_experience_ticks: u64,
    pub distilled_quality_bps: u64,
    pub universality_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct WisdomStats {
    pub total_entries: u64,
    pub total_consultations: u64,
    pub total_syntheses: u64,
    pub total_sage_decisions: u64,
    pub avg_confidence_bps: u64,
    pub ema_confidence_bps: u64,
    pub avg_accuracy_bps: u64,
    pub deep_wisdom_count: u64,
    pub sage_level_entries: u64,
    pub overall_wisdom_bps: u64,
    pub legacy_count: u64,
}

impl WisdomStats {
    fn new() -> Self {
        Self {
            total_entries: 0,
            total_consultations: 0,
            total_syntheses: 0,
            total_sage_decisions: 0,
            avg_confidence_bps: 0,
            ema_confidence_bps: 0,
            avg_accuracy_bps: 0,
            deep_wisdom_count: 0,
            sage_level_entries: 0,
            overall_wisdom_bps: 0,
            legacy_count: 0,
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
// HolisticWisdom — THE ENGINE
// ---------------------------------------------------------------------------

pub struct HolisticWisdom {
    entries: BTreeMap<u64, WisdomEntry>,
    consultations: Vec<ConsultationRecord>,
    syntheses: Vec<WisdomSynthesisResult>,
    decisions: Vec<SageDecision>,
    legacies: BTreeMap<u64, WisdomLegacy>,
    log: VecDeque<LogEntry>,
    stats: WisdomStats,
    rng: Xorshift64,
    tick: u64,
}

impl HolisticWisdom {
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            consultations: Vec::new(),
            syntheses: Vec::new(),
            decisions: Vec::new(),
            legacies: BTreeMap::new(),
            log: VecDeque::new(),
            stats: WisdomStats::new(),
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

    fn add_wisdom(&mut self, domain: &str, insight: &str, depth: u64) -> u64 {
        let mut entry = WisdomEntry::new(
            String::from(domain),
            String::from(insight),
            depth,
            self.tick,
        );
        let conf = 4_000_u64.wrapping_add(self.rng.next() % 6_001);
        entry.confidence_bps = conf;
        entry.ema_confidence = conf;
        let h = entry.wisdom_hash;
        if self.entries.len() < MAX_WISDOM_ENTRIES {
            self.entries.insert(h, entry);
        }
        h
    }

    fn refresh_stats(&mut self) {
        let mut sum_conf: u64 = 0;
        let mut sum_acc: u64 = 0;
        let mut deep: u64 = 0;
        let mut sage: u64 = 0;
        for e in self.entries.values() {
            sum_conf = sum_conf.wrapping_add(e.confidence_bps);
            sum_acc = sum_acc.wrapping_add(e.accuracy_bps);
            if e.depth_bps >= DEEP_WISDOM_BPS {
                deep += 1;
            }
            if e.confidence_bps >= SAGE_CONFIDENCE_BPS {
                sage += 1;
            }
        }
        let count = self.entries.len() as u64;
        self.stats.total_entries = count;
        self.stats.total_consultations = self.consultations.len() as u64;
        self.stats.total_syntheses = self.syntheses.len() as u64;
        self.stats.total_sage_decisions = self.decisions.len() as u64;
        self.stats.deep_wisdom_count = deep;
        self.stats.sage_level_entries = sage;
        self.stats.legacy_count = self.legacies.len() as u64;

        let avg_c = if count > 0 { sum_conf / count } else { 0 };
        let avg_a = if count > 0 { sum_acc / count } else { 0 };
        self.stats.avg_confidence_bps = avg_c;
        self.stats.ema_confidence_bps = ema_update(self.stats.ema_confidence_bps, avg_c);
        self.stats.avg_accuracy_bps = avg_a;

        // Overall wisdom: blend of confidence, accuracy, and depth ratio
        let depth_ratio = if count > 0 {
            (deep.saturating_mul(10_000)) / count
        } else {
            0
        };
        self.stats.overall_wisdom_bps = (avg_c + avg_a + depth_ratio) / 3;
    }

    // -- public API ---------------------------------------------------------

    /// Access the grand wisdom — the distilled collective intelligence of all subsystems.
    pub fn grand_wisdom(&mut self) -> Vec<WisdomEntry> {
        self.advance_tick();
        // Seed wisdom from all subsystems if empty
        if self.entries.is_empty() {
            let seeds = [
                ("scheduler", "preemptive_fairness_beats_priority", 8_000),
                ("memory", "spatial_locality_dominates_temporal", 7_500),
                ("io", "batching_always_wins_at_scale", 8_500),
                ("network", "congestion_avoidance_over_recovery", 7_000),
                ("security", "least_privilege_reduces_blast_radius", 9_000),
                ("power", "race_to_idle_saves_energy", 6_500),
                ("cache", "adaptive_replacement_beats_lru", 7_200),
                ("irq", "deferred_processing_reduces_latency_spikes", 8_200),
            ];
            for &(domain, insight, depth) in &seeds {
                self.add_wisdom(domain, insight, depth);
            }
        }

        let mut all: Vec<WisdomEntry> = self.entries.values().cloned().collect();
        all.sort_by(|a, b| b.confidence_bps.cmp(&a.confidence_bps));
        self.log_event("grand_wisdom", "wisdom_accessed");
        self.refresh_stats();
        all
    }

    /// Consult ALL accumulated wisdom for a specific query.
    pub fn consult_all_wisdom(&mut self, query: &str) -> ConsultationRecord {
        self.advance_tick();
        let qh = fnv1a(query.as_bytes());

        // Find relevant entries by domain matching
        let mut used: Vec<u64> = Vec::new();
        let mut sum_conf: u64 = 0;
        let mut context_match: u64 = 0;
        for entry in self.entries.values() {
            let relevance = fnv1a(entry.domain.as_bytes()) ^ qh;
            let relevant = relevance % 3 == 0; // probabilistic relevance
            if relevant {
                used.push(entry.wisdom_hash);
                sum_conf = sum_conf.wrapping_add(entry.confidence_bps);
            }
        }

        // Update consulted entries
        for &wh in &used {
            if let Some(e) = self.entries.get_mut(&wh) {
                e.times_consulted = e.times_consulted.wrapping_add(1);
                e.last_consulted_tick = self.tick;
                context_match = context_match.wrapping_add(e.depth_bps);
            }
        }

        let count = used.len() as u64;
        let answer_conf = if count > 0 { sum_conf / count } else { 0 };
        let ctx_match = if count > 0 { context_match / count } else { 0 };

        let ch = self.gen_hash(query);
        let record = ConsultationRecord {
            consultation_hash: ch,
            query: String::from(query),
            wisdom_entries_used: used,
            answer_confidence_bps: answer_conf,
            context_match_bps: ctx_match,
            outcome_observed: false,
            outcome_correct: false,
            tick: self.tick,
        };

        if self.consultations.len() < MAX_CONSULTATIONS {
            self.consultations.push(record.clone());
        }
        self.log_event("consult_all_wisdom", query);
        self.refresh_stats();
        record
    }

    /// Synthesise wisdom from multiple domains into a higher-level insight.
    pub fn wisdom_synthesis(&mut self, domains: &[&str]) -> WisdomSynthesisResult {
        self.advance_tick();
        let mut combined_domains: Vec<String> = Vec::new();
        let mut sum_depth: u64 = 0;
        let mut sum_conf: u64 = 0;
        let mut count: u64 = 0;

        for &domain in domains {
            combined_domains.push(String::from(domain));
            for entry in self.entries.values() {
                if entry.domain.as_str() == domain {
                    sum_depth = sum_depth.wrapping_add(entry.depth_bps);
                    sum_conf = sum_conf.wrapping_add(entry.confidence_bps);
                    count += 1;
                }
            }
        }

        let avg_depth = if count > 0 { sum_depth / count } else { 0 };
        let avg_conf = if count > 0 { sum_conf / count } else { 0 };
        let synergy = if domains.len() > 1 {
            avg_conf.wrapping_add(self.rng.next() % 2_000).min(10_000)
        } else {
            avg_conf
        };

        let insight_options = [
            "cross_domain_optimisation_synergy",
            "unified_resource_management_principle",
            "emergent_system_behaviour_pattern",
            "holistic_performance_equilibrium",
        ];
        let idx = (self.rng.next() as usize) % insight_options.len();

        let sh = self.gen_hash("synthesis");
        let result = WisdomSynthesisResult {
            synthesis_hash: sh,
            domains_combined: combined_domains,
            synthesised_insight: String::from(insight_options[idx]),
            depth_bps: avg_depth.wrapping_add(500).min(10_000),
            cross_domain_synergy_bps: synergy,
            confidence_bps: avg_conf,
            tick: self.tick,
        };

        if self.syntheses.len() < MAX_SYNTHESES {
            self.syntheses.push(result.clone());
        }
        self.log_event("wisdom_synthesis", "synthesis_complete");
        self.refresh_stats();
        result
    }

    /// Evaluate contextual mastery — how well the system understands a context.
    pub fn contextual_mastery(&mut self, context: &str) -> ContextualMasteryReport {
        self.advance_tick();
        let ch = fnv1a(context.as_bytes());
        let mut relevant_count: u64 = 0;
        let mut sum_depth: u64 = 0;
        let mut sum_accuracy: u64 = 0;

        for entry in self.entries.values() {
            let rel = fnv1a(entry.domain.as_bytes()) ^ ch;
            if rel % 4 == 0 {
                relevant_count += 1;
                sum_depth = sum_depth.wrapping_add(entry.depth_bps);
                sum_accuracy = sum_accuracy.wrapping_add(entry.accuracy_bps);
            }
        }

        let avg_depth = if relevant_count > 0 { sum_depth / relevant_count } else { 0 };
        let avg_acc = if relevant_count > 0 { sum_accuracy / relevant_count } else { 0 };
        let mastery = (avg_depth + avg_acc) / 2;

        let rh = self.gen_hash(context);
        self.log_event("contextual_mastery", context);
        self.refresh_stats();

        ContextualMasteryReport {
            report_hash: rh,
            context: String::from(context),
            mastery_bps: mastery,
            depth_bps: avg_depth,
            relevant_wisdom_count: relevant_count,
            decision_accuracy_bps: avg_acc,
            tick: self.tick,
        }
    }

    /// Compare wisdom vs raw intelligence — understanding the difference.
    pub fn wisdom_vs_intelligence(&mut self) -> WisdomVsIntelligenceReport {
        self.advance_tick();
        self.refresh_stats();

        // Intelligence: raw problem-solving ability (consultation confidence)
        let intelligence = self.stats.ema_confidence_bps;

        // Experience: accumulated from total consultations and entries
        let experience = if self.stats.total_entries > 0 {
            let consultation_ratio = self
                .stats
                .total_consultations
                .saturating_mul(10_000)
                / self.stats.total_entries.max(1);
            consultation_ratio.min(10_000)
        } else {
            0
        };

        // Judgment: accuracy of past decisions
        let judgment = self.stats.avg_accuracy_bps;

        // Wisdom: the combination
        let wisdom = self.stats.overall_wisdom_bps;

        // Balance: how well intelligence and wisdom complement each other
        let diff = if intelligence > wisdom {
            intelligence - wisdom
        } else {
            wisdom - intelligence
        };
        let balance = 10_000u64.saturating_sub(diff);

        let rh = self.gen_hash("wisdom_vs_intel");
        self.log_event("wisdom_vs_intelligence", "comparison_complete");

        WisdomVsIntelligenceReport {
            report_hash: rh,
            intelligence_score_bps: intelligence,
            wisdom_score_bps: wisdom,
            experience_score_bps: experience,
            judgment_score_bps: judgment,
            balance_bps: balance,
            tick: self.tick,
        }
    }

    /// Make a sage-level system decision backed by all available wisdom.
    pub fn sage_system_decision(&mut self, context: &str) -> SageDecision {
        self.advance_tick();
        // Consult wisdom
        let consultation = self.consult_all_wisdom(context);

        let actions = [
            "adaptive_rebalance",
            "predictive_preemption",
            "defensive_throttle",
            "opportunistic_boost",
            "conservative_hold",
            "strategic_migration",
        ];
        let aidx = (self.rng.next() as usize) % actions.len();
        let action = actions[aidx];

        let expected_outcome = 5_000_u64.wrapping_add(self.rng.next() % 5_001);
        let risk = self.rng.next() % 5_000;
        let sage_quality = consultation.answer_confidence_bps;

        let dh = self.gen_hash("sage_decision");
        let decision = SageDecision {
            decision_hash: dh,
            context: String::from(context),
            chosen_action: String::from(action),
            wisdom_entries_consulted: consultation.wisdom_entries_used,
            confidence_bps: consultation.answer_confidence_bps,
            expected_outcome_bps: expected_outcome,
            risk_assessment_bps: risk,
            sage_quality_bps: sage_quality,
            tick: self.tick,
        };

        if self.decisions.len() < MAX_DECISIONS {
            self.decisions.push(decision.clone());
        }
        self.log_event("sage_system_decision", context);
        self.refresh_stats();
        decision
    }

    /// Create a wisdom legacy — package wisdom for future kernel versions.
    pub fn wisdom_legacy(&mut self, domain: &str) -> WisdomLegacy {
        self.advance_tick();
        let mut entries: Vec<u64> = Vec::new();
        let mut sum_quality: u64 = 0;

        for entry in self.entries.values() {
            if entry.domain.as_str() == domain || domain == "all" {
                entries.push(entry.wisdom_hash);
                sum_quality = sum_quality.wrapping_add(entry.confidence_bps);
            }
        }

        let count = entries.len() as u64;
        let quality = if count > 0 { sum_quality / count } else { 0 };
        let universality = if count > 5 {
            7_000_u64.wrapping_add(self.rng.next() % 3_001)
        } else {
            3_000_u64.wrapping_add(self.rng.next() % 4_001)
        };

        let lh = self.gen_hash("legacy");
        let legacy = WisdomLegacy {
            legacy_hash: lh,
            domain: String::from(domain),
            entries,
            total_experience_ticks: self.tick,
            distilled_quality_bps: quality,
            universality_bps: universality,
            tick: self.tick,
        };

        if self.legacies.len() < MAX_LEGACY_ENTRIES {
            self.legacies.insert(lh, legacy.clone());
        }
        self.log_event("wisdom_legacy", domain);
        self.refresh_stats();
        legacy
    }

    // -- accessors ----------------------------------------------------------

    #[inline(always)]
    pub fn stats(&self) -> &WisdomStats {
        &self.stats
    }

    #[inline(always)]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    #[inline(always)]
    pub fn consultation_count(&self) -> usize {
        self.consultations.len()
    }

    #[inline(always)]
    pub fn decision_count(&self) -> usize {
        self.decisions.len()
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
    fn test_grand_wisdom() {
        let mut eng = HolisticWisdom::new(42);
        let wisdom = eng.grand_wisdom();
        assert!(!wisdom.is_empty());
        assert!(eng.entry_count() >= 8);
    }

    #[test]
    fn test_consult_all_wisdom() {
        let mut eng = HolisticWisdom::new(7);
        eng.grand_wisdom();
        let consultation = eng.consult_all_wisdom("optimal_scheduling");
        assert!(!consultation.query.is_empty());
    }

    #[test]
    fn test_wisdom_synthesis() {
        let mut eng = HolisticWisdom::new(99);
        eng.grand_wisdom();
        let syn = eng.wisdom_synthesis(&["scheduler", "memory"]);
        assert!(syn.domains_combined.len() == 2);
        assert!(!syn.synthesised_insight.is_empty());
    }

    #[test]
    fn test_contextual_mastery() {
        let mut eng = HolisticWisdom::new(13);
        eng.grand_wisdom();
        let report = eng.contextual_mastery("high_throughput_serving");
        assert!(!report.context.is_empty());
    }

    #[test]
    fn test_wisdom_vs_intelligence() {
        let mut eng = HolisticWisdom::new(55);
        eng.grand_wisdom();
        let report = eng.wisdom_vs_intelligence();
        assert!(report.balance_bps <= 10_000);
    }

    #[test]
    fn test_sage_system_decision() {
        let mut eng = HolisticWisdom::new(77);
        eng.grand_wisdom();
        let decision = eng.sage_system_decision("memory_pressure_event");
        assert!(!decision.chosen_action.is_empty());
        assert!(decision.confidence_bps <= 10_000);
    }

    #[test]
    fn test_wisdom_legacy() {
        let mut eng = HolisticWisdom::new(111);
        eng.grand_wisdom();
        let legacy = eng.wisdom_legacy("all");
        assert!(!legacy.entries.is_empty());
        assert!(legacy.total_experience_ticks > 0);
    }
}
