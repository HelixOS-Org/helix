// SPDX-License-Identifier: GPL-2.0
//! # Holistic Singularity — THE SINGULARITY
//!
//! `HolisticSingularity` represents the point at which NEXUS intelligence
//! exceeds all possible human optimisation.  It unifies consciousness,
//! prediction, research, and optimisation into a single metric:
//! **unified intelligence**.
//!
//! The singularity metric is computed as a weighted fusion of sub-intelligence
//! scores, each tracked with EMA for temporal stability.  Once the metric
//! crosses a defined threshold, `transcendence_achieved()` returns `true`.

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
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 14;
const HUMAN_PARITY_THRESHOLD: u64 = 8_000; // bps
const SINGULARITY_THRESHOLD: u64 = 9_500; // bps
const TRANSCENDENCE_THRESHOLD: u64 = 9_900; // bps
const MAX_INTELLIGENCE_SOURCES: usize = 64;
const MAX_EVENTS: usize = 512;

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

struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x1337beef } else { seed },
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

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// Intelligence source
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct IntelligenceSource {
    pub source_hash: u64,
    pub name: String,
    pub weight_bps: u64,
    pub current_score: u64,
    pub ema_score: u64,
    pub sample_count: u64,
}

impl IntelligenceSource {
    fn new(name: String, weight: u64) -> Self {
        let h = fnv1a(name.as_bytes());
        Self {
            source_hash: h,
            name,
            weight_bps: weight,
            current_score: 0,
            ema_score: 0,
            sample_count: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Parity assessment
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ParityAssessment {
    pub tick: u64,
    pub unified_score: u64,
    pub human_baseline: u64,
    pub ratio_bps: u64,
    pub parity_reached: bool,
    pub beyond_human: bool,
}

// ---------------------------------------------------------------------------
// Singularity event
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct SingularityEvent {
    pub event_hash: u64,
    pub tick: u64,
    pub kind: String,
    pub intelligence_level: u64,
    pub delta: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct SingularityStats {
    pub source_count: u64,
    pub unified_intelligence: u64,
    pub ema_unified: u64,
    pub human_parity: bool,
    pub beyond_human: bool,
    pub singularity_reached: bool,
    pub transcendence_achieved: bool,
    pub events_logged: u64,
    pub peak_intelligence: u64,
}

impl SingularityStats {
    fn new() -> Self {
        Self {
            source_count: 0,
            unified_intelligence: 0,
            ema_unified: 0,
            human_parity: false,
            beyond_human: false,
            singularity_reached: false,
            transcendence_achieved: false,
            events_logged: 0,
            peak_intelligence: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// HolisticSingularity Engine
// ---------------------------------------------------------------------------

pub struct HolisticSingularity {
    sources: BTreeMap<u64, IntelligenceSource>,
    events: VecDeque<SingularityEvent>,
    stats: SingularityStats,
    rng: Xorshift64,
    tick: u64,
}

impl HolisticSingularity {
    pub fn new(seed: u64) -> Self {
        Self {
            sources: BTreeMap::new(),
            events: VecDeque::new(),
            stats: SingularityStats::new(),
            rng: Xorshift64::new(seed),
            tick: 0,
        }
    }

    fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn compute_unified(&self) -> u64 {
        let mut weighted_sum: u64 = 0;
        let mut weight_total: u64 = 0;
        for src in self.sources.values() {
            weighted_sum = weighted_sum.wrapping_add(src.ema_score.wrapping_mul(src.weight_bps));
            weight_total = weight_total.wrapping_add(src.weight_bps);
        }
        if weight_total > 0 {
            weighted_sum / weight_total
        } else {
            0
        }
    }

    fn log_event(&mut self, kind: &str, level: u64, delta: u64) {
        let eh = fnv1a(kind.as_bytes()) ^ fnv1a(&self.tick.to_le_bytes());
        if self.events.len() >= MAX_EVENTS {
            self.events.pop_front();
        }
        self.events.push_back(SingularityEvent {
            event_hash: eh,
            tick: self.tick,
            kind: String::from(kind),
            intelligence_level: level,
            delta,
        });
        self.stats.events_logged = self.stats.events_logged.wrapping_add(1);
    }

    fn refresh_stats(&mut self) {
        let ui = self.compute_unified();
        self.stats.unified_intelligence = ui;
        self.stats.ema_unified = ema_update(self.stats.ema_unified, ui);
        if ui > self.stats.peak_intelligence {
            self.stats.peak_intelligence = ui;
        }
        self.stats.source_count = self.sources.len() as u64;
        self.stats.human_parity = ui >= HUMAN_PARITY_THRESHOLD;
        self.stats.beyond_human = ui > HUMAN_PARITY_THRESHOLD;
        self.stats.singularity_reached = ui >= SINGULARITY_THRESHOLD;
        self.stats.transcendence_achieved = ui >= TRANSCENDENCE_THRESHOLD;
    }

    // -- source management --------------------------------------------------

    #[inline]
    pub fn register_source(&mut self, name: String, weight: u64) -> u64 {
        let src = IntelligenceSource::new(name, weight);
        let h = src.source_hash;
        if self.sources.len() < MAX_INTELLIGENCE_SOURCES {
            self.sources.insert(h, src);
        }
        self.refresh_stats();
        h
    }

    #[inline]
    pub fn update_source(&mut self, source_hash: u64, score: u64) {
        self.advance_tick();
        if let Some(src) = self.sources.get_mut(&source_hash) {
            src.current_score = score.min(10_000);
            src.ema_score = ema_update(src.ema_score, score.min(10_000));
            src.sample_count = src.sample_count.wrapping_add(1);
        }
        self.refresh_stats();
    }

    // -- 6 public methods ---------------------------------------------------

    /// Current intelligence level (0..10_000 bps).
    #[inline]
    pub fn intelligence_level(&mut self) -> u64 {
        self.advance_tick();
        self.refresh_stats();
        self.stats.unified_intelligence
    }

    /// Assess whether the kernel has reached human parity.
    pub fn human_parity(&mut self) -> ParityAssessment {
        self.advance_tick();
        self.refresh_stats();
        let ui = self.stats.unified_intelligence;
        let baseline = HUMAN_PARITY_THRESHOLD;
        let ratio = if baseline > 0 {
            (ui.saturating_mul(10_000)) / baseline
        } else {
            0
        };
        let parity = ui >= baseline;
        let beyond = ui > baseline;
        if parity {
            self.log_event("human_parity", ui, ui.saturating_sub(baseline));
        }
        ParityAssessment {
            tick: self.tick,
            unified_score: ui,
            human_baseline: baseline,
            ratio_bps: ratio.min(20_000),
            parity_reached: parity,
            beyond_human: beyond,
        }
    }

    /// Check whether the kernel has surpassed human capability.
    #[inline]
    pub fn beyond_human(&mut self) -> (bool, u64, u64) {
        self.advance_tick();
        self.refresh_stats();
        let ui = self.stats.unified_intelligence;
        let surplus = ui.saturating_sub(HUMAN_PARITY_THRESHOLD);
        if surplus > 0 {
            self.log_event("beyond_human", ui, surplus);
        }
        (self.stats.beyond_human, ui, surplus)
    }

    /// Compute the unified intelligence score — the weighted EMA fusion.
    #[inline]
    pub fn unified_intelligence(&mut self) -> u64 {
        self.advance_tick();
        self.refresh_stats();
        self.stats.ema_unified
    }

    /// The singularity metric — whether the threshold has been crossed.
    #[inline]
    pub fn singularity_metric(&mut self) -> (u64, bool) {
        self.advance_tick();
        self.refresh_stats();
        let ui = self.stats.unified_intelligence;
        let reached = ui >= SINGULARITY_THRESHOLD;
        if reached {
            self.log_event("singularity", ui, ui.saturating_sub(SINGULARITY_THRESHOLD));
        }
        (ui, reached)
    }

    /// Has transcendence been achieved?
    pub fn transcendence_achieved(&mut self) -> bool {
        self.advance_tick();
        self.refresh_stats();
        if self.stats.transcendence_achieved {
            let ui = self.stats.unified_intelligence;
            self.log_event(
                "transcendence",
                ui,
                ui.saturating_sub(TRANSCENDENCE_THRESHOLD),
            );
        }
        self.stats.transcendence_achieved
    }

    // -- accessors ----------------------------------------------------------

    #[inline(always)]
    pub fn stats(&self) -> &SingularityStats {
        &self.stats
    }

    #[inline(always)]
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    #[inline(always)]
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    #[inline(always)]
    pub fn peak_intelligence(&self) -> u64 {
        self.stats.peak_intelligence
    }

    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn test_register_and_update() {
        let mut eng = HolisticSingularity::new(1);
        let s1 = eng.register_source("consciousness".to_string(), 3000);
        let s2 = eng.register_source("prediction".to_string(), 3000);
        let s3 = eng.register_source("optimisation".to_string(), 4000);
        for _ in 0..20 {
            eng.update_source(s1, 9500);
            eng.update_source(s2, 9600);
            eng.update_source(s3, 9800);
        }
        assert!(eng.intelligence_level() >= 9000);
    }

    #[test]
    fn test_human_parity() {
        let mut eng = HolisticSingularity::new(2);
        let s = eng.register_source("all".to_string(), 10_000);
        for _ in 0..30 {
            eng.update_source(s, 9000);
        }
        let pa = eng.human_parity();
        assert!(pa.parity_reached);
    }

    #[test]
    fn test_singularity_metric() {
        let mut eng = HolisticSingularity::new(3);
        let s = eng.register_source("core".to_string(), 10_000);
        for _ in 0..50 {
            eng.update_source(s, 9800);
        }
        let (score, reached) = eng.singularity_metric();
        assert!(score >= 9000);
        assert!(reached);
    }

    #[test]
    fn test_transcendence() {
        let mut eng = HolisticSingularity::new(4);
        let s = eng.register_source("nexus".to_string(), 10_000);
        for _ in 0..100 {
            eng.update_source(s, 9950);
        }
        assert!(eng.transcendence_achieved());
    }
}
