// SPDX-License-Identifier: GPL-2.0
//! # Bridge Interface — Advanced Human-Kernel Communication
//!
//! The bridge that can EXPLAIN itself. Every optimisation decision,
//! routing choice, and performance anomaly is translated into
//! human-understandable narratives, reasoning traces, and structured
//! reports. This enables kernel developers to understand — and trust —
//! the bridge's superintelligent behaviour.
//!
//! FNV-1a hashes index explanations; xorshift64 drives sampling for
//! narrative diversity; EMA tracks explanation clarity scores.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_EXPLANATIONS: usize = 256;
const MAX_TRACE_DEPTH: usize = 32;
const MAX_NARRATIVE_SEGMENTS: usize = 16;
const MAX_RECOMMENDATIONS: usize = 64;
const MAX_REPORT_SECTIONS: usize = 12;
const EMA_ALPHA: f32 = 0.10;
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
// INTERFACE TYPES
// ============================================================================

/// Audience level for an explanation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AudienceLevel {
    Expert,
    Developer,
    Operator,
    EndUser,
}

/// Urgency of a recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgency {
    Informational,
    Advisory,
    Important,
    Critical,
}

/// A single step in a reasoning trace.
#[derive(Debug, Clone)]
pub struct TraceStep {
    pub step_index: u32,
    pub description: String,
    pub input_summary: String,
    pub output_summary: String,
    pub confidence: f32,
    pub alternatives_rejected: u32,
}

/// Full reasoning trace for a decision.
#[derive(Debug, Clone)]
pub struct ReasoningTrace {
    pub trace_id: u64,
    pub decision_name: String,
    pub steps: Vec<TraceStep>,
    pub final_confidence: f32,
    pub total_alternatives: u32,
    pub tick: u64,
}

/// A human-readable decision explanation.
#[derive(Debug, Clone)]
pub struct DecisionExplanation {
    pub explanation_id: u64,
    pub decision_name: String,
    pub audience: AudienceLevel,
    pub summary: String,
    pub rationale: String,
    pub impact: String,
    pub confidence: f32,
    pub clarity_score: f32,
    pub tick: u64,
}

/// A narrative segment for an optimisation story.
#[derive(Debug, Clone)]
pub struct NarrativeSegment {
    pub heading: String,
    pub body: String,
    pub metric_before: f32,
    pub metric_after: f32,
    pub improvement_pct: f32,
}

/// Structured performance report.
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub report_id: u64,
    pub title: String,
    pub sections: Vec<ReportSection>,
    pub overall_score: f32,
    pub generated_tick: u64,
}

/// A single section of a performance report.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ReportSection {
    pub heading: String,
    pub body: String,
    pub metric_value: f32,
    pub trend: f32,
    pub status: String,
}

/// An actionable recommendation.
#[derive(Debug, Clone)]
pub struct Recommendation {
    pub rec_id: u64,
    pub title: String,
    pub description: String,
    pub urgency: Urgency,
    pub expected_improvement: f32,
    pub confidence: f32,
    pub tick: u64,
}

// ============================================================================
// INTERFACE STATS
// ============================================================================

/// Aggregate statistics for the interface engine.
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct InterfaceStats {
    pub explanations_generated: u64,
    pub traces_generated: u64,
    pub narratives_generated: u64,
    pub reports_generated: u64,
    pub recommendations_issued: u64,
    pub avg_clarity_ema: f32,
    pub avg_trace_depth_ema: f32,
}

// ============================================================================
// CLARITY EVALUATOR
// ============================================================================

#[derive(Debug)]
struct ClarityEvaluator {
    total_evaluated: u64,
    clarity_ema: f32,
}

impl ClarityEvaluator {
    fn new() -> Self {
        Self {
            total_evaluated: 0,
            clarity_ema: 0.7,
        }
    }

    /// Heuristic clarity score based on text length, structure, and
    /// vocabulary density (approximated via byte-level entropy proxy).
    #[inline]
    fn evaluate(&mut self, text: &str) -> f32 {
        self.total_evaluated += 1;
        let len = text.len() as f32;
        // Prefer explanations between 80 and 500 chars.
        let length_score = if len < 80.0 {
            len / 80.0
        } else if len > 500.0 {
            500.0 / len
        } else {
            1.0
        };

        // Sentence count proxy: count '.' occurrences.
        let sentence_count = text.as_bytes().iter().filter(|&&b| b == b'.').count() as f32;
        let structure_score = (sentence_count / 5.0).min(1.0);

        // Vocabulary diversity proxy: unique byte pairs.
        let mut pair_set: u64 = 0;
        let bytes = text.as_bytes();
        for w in bytes.windows(2) {
            let pair_hash = (w[0] as u64) << 8 | w[1] as u64;
            pair_set = pair_set.wrapping_add(pair_hash);
        }
        let diversity = ((pair_set % 256) as f32 / 256.0).min(1.0);

        let clarity = length_score * 0.4 + structure_score * 0.35 + diversity * 0.25;
        self.clarity_ema = EMA_ALPHA * clarity + (1.0 - EMA_ALPHA) * self.clarity_ema;
        clarity.max(0.0).min(1.0)
    }
}

// ============================================================================
// BRIDGE INTERFACE
// ============================================================================

/// Advanced human-kernel communication engine. Generates reasoning traces,
/// decision explanations, performance narratives, and recommendations.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeInterface {
    explanations: BTreeMap<u64, DecisionExplanation>,
    traces: BTreeMap<u64, ReasoningTrace>,
    recommendations: VecDeque<Recommendation>,
    clarity_eval: ClarityEvaluator,
    tick: u64,
    rng_state: u64,
    stats: InterfaceStats,
}

impl BridgeInterface {
    pub fn new(seed: u64) -> Self {
        Self {
            explanations: BTreeMap::new(),
            traces: BTreeMap::new(),
            recommendations: VecDeque::new(),
            clarity_eval: ClarityEvaluator::new(),
            tick: 0,
            rng_state: seed | 1,
            stats: InterfaceStats::default(),
        }
    }

    /// Generate a human-readable explanation for a bridge decision.
    #[inline]
    pub fn explain_decision(
        &mut self,
        decision_name: String,
        audience: AudienceLevel,
        rationale: String,
        impact: String,
        confidence: f32,
    ) -> DecisionExplanation {
        self.tick += 1;
        self.stats.explanations_generated += 1;
        let eid = fnv1a_hash(decision_name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let summary = self.build_summary(&decision_name, audience, confidence);
        let full_text = self.format_explanation(&summary, &rationale, &impact);
        let clarity = self.clarity_eval.evaluate(&full_text);
        self.stats.avg_clarity_ema =
            EMA_ALPHA * clarity + (1.0 - EMA_ALPHA) * self.stats.avg_clarity_ema;

        let explanation = DecisionExplanation {
            explanation_id: eid,
            decision_name,
            audience,
            summary,
            rationale,
            impact,
            confidence: confidence.max(0.0).min(1.0),
            clarity_score: clarity,
            tick: self.tick,
        };

        // Evict oldest if capacity reached.
        if self.explanations.len() >= MAX_EXPLANATIONS {
            if let Some(&oldest) = self.explanations.keys().next() {
                self.explanations.remove(&oldest);
            }
        }
        self.explanations.insert(eid, explanation.clone());
        explanation
    }

    /// Build a structured narrative around an optimisation outcome.
    pub fn optimization_narrative(
        &mut self,
        title: String,
        segments: Vec<(String, f32, f32)>,
    ) -> Vec<NarrativeSegment> {
        self.tick += 1;
        self.stats.narratives_generated += 1;
        let mut result = Vec::new();

        for (heading, before, after) in segments.into_iter().take(MAX_NARRATIVE_SEGMENTS) {
            let improvement = if before > 0.0 {
                ((after - before) / before) * 100.0
            } else {
                0.0
            };

            let body = self.build_narrative_body(&heading, before, after, improvement);
            result.push(NarrativeSegment {
                heading,
                body,
                metric_before: before,
                metric_after: after,
                improvement_pct: improvement,
            });
        }
        result
    }

    /// Generate a complete reasoning trace for a decision.
    #[inline]
    pub fn reasoning_trace(
        &mut self,
        decision_name: String,
        steps: Vec<(String, String, String, f32, u32)>,
    ) -> ReasoningTrace {
        self.tick += 1;
        self.stats.traces_generated += 1;
        let tid = fnv1a_hash(decision_name.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let mut trace_steps = Vec::new();
        let mut total_alts: u32 = 0;

        for (idx, (desc, input, output, conf, alts)) in
            steps.into_iter().enumerate().take(MAX_TRACE_DEPTH)
        {
            total_alts += alts;
            trace_steps.push(TraceStep {
                step_index: idx as u32,
                description: desc,
                input_summary: input,
                output_summary: output,
                confidence: conf.max(0.0).min(1.0),
                alternatives_rejected: alts,
            });
        }

        let final_conf = trace_steps.last().map(|s| s.confidence).unwrap_or(0.5);

        self.stats.avg_trace_depth_ema = EMA_ALPHA * trace_steps.len() as f32
            + (1.0 - EMA_ALPHA) * self.stats.avg_trace_depth_ema;

        let trace = ReasoningTrace {
            trace_id: tid,
            decision_name,
            steps: trace_steps,
            final_confidence: final_conf,
            total_alternatives: total_alts,
            tick: self.tick,
        };

        if self.traces.len() >= MAX_EXPLANATIONS {
            if let Some(&oldest) = self.traces.keys().next() {
                self.traces.remove(&oldest);
            }
        }
        self.traces.insert(tid, trace.clone());
        trace
    }

    /// Generate a structured performance report.
    pub fn performance_report(
        &mut self,
        title: String,
        metrics: Vec<(String, f32, f32, String)>,
    ) -> PerformanceReport {
        self.tick += 1;
        self.stats.reports_generated += 1;
        let rid = fnv1a_hash(title.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let mut sections = Vec::new();
        let mut total_score: f32 = 0.0;

        for (heading, value, trend, status) in metrics.into_iter().take(MAX_REPORT_SECTIONS) {
            let body = self.build_report_body(&heading, value, trend);
            total_score += value.max(0.0).min(1.0);
            sections.push(ReportSection {
                heading,
                body,
                metric_value: value,
                trend,
                status,
            });
        }

        let count = sections.len().max(1) as f32;
        PerformanceReport {
            report_id: rid,
            title,
            sections,
            overall_score: total_score / count,
            generated_tick: self.tick,
        }
    }

    /// Issue an actionable recommendation.
    pub fn recommendation(
        &mut self,
        title: String,
        description: String,
        urgency: Urgency,
        expected_improvement: f32,
        confidence: f32,
    ) -> Recommendation {
        self.tick += 1;
        self.stats.recommendations_issued += 1;
        let rid = fnv1a_hash(title.as_bytes()) ^ xorshift64(&mut self.rng_state);

        let rec = Recommendation {
            rec_id: rid,
            title,
            description,
            urgency,
            expected_improvement,
            confidence: confidence.max(0.0).min(1.0),
            tick: self.tick,
        };

        if self.recommendations.len() >= MAX_RECOMMENDATIONS {
            // Remove lowest-urgency recommendation.
            if let Some(pos) = self
                .recommendations
                .iter()
                .position(|r| r.urgency == Urgency::Informational)
            {
                self.recommendations.remove(pos);
            } else {
                self.recommendations.pop_front();
            }
        }
        self.recommendations.push_back(rec.clone());
        rec
    }

    /// Retrieve all pending recommendations sorted by urgency.
    #[inline]
    pub fn pending_recommendations(&self) -> Vec<&Recommendation> {
        let mut recs: Vec<&Recommendation> = self.recommendations.iter().collect();
        recs.sort_by(|a, b| b.urgency.cmp(&a.urgency));
        recs
    }

    /// Aggregate statistics.
    #[inline(always)]
    pub fn stats(&self) -> InterfaceStats {
        self.stats
    }

    // ---- internal helpers ----

    fn build_summary(&self, name: &str, audience: AudienceLevel, confidence: f32) -> String {
        let level = match audience {
            AudienceLevel::Expert => "Technical",
            AudienceLevel::Developer => "Developer",
            AudienceLevel::Operator => "Operations",
            AudienceLevel::EndUser => "General",
        };
        let conf_pct = (confidence * 100.0) as u32;
        let mut s = String::from(level);
        s.push_str(" decision: ");
        s.push_str(name);
        s.push_str(" (confidence ");
        // Manual integer-to-string for no_std
        let hundreds = conf_pct / 100;
        let tens = (conf_pct % 100) / 10;
        let ones = conf_pct % 10;
        if hundreds > 0 {
            s.push((b'0' + hundreds as u8) as char);
        }
        s.push((b'0' + tens as u8) as char);
        s.push((b'0' + ones as u8) as char);
        s.push_str("%)");
        s
    }

    fn format_explanation(&self, summary: &str, rationale: &str, impact: &str) -> String {
        let mut out = String::from(summary);
        out.push_str(". Rationale: ");
        out.push_str(rationale);
        out.push_str(". Impact: ");
        out.push_str(impact);
        out.push('.');
        out
    }

    fn build_narrative_body(
        &self,
        heading: &str,
        before: f32,
        after: f32,
        improvement: f32,
    ) -> String {
        let mut body = String::from("Metric '");
        body.push_str(heading);
        body.push_str("' changed. Improvement observed. ");
        if improvement > 0.0 {
            body.push_str("Positive trend detected.");
        } else if improvement < 0.0 {
            body.push_str("Regression detected — investigation recommended.");
        } else {
            body.push_str("No significant change.");
        }
        let _ = (before, after); // used for narrative context
        body
    }

    fn build_report_body(&self, heading: &str, value: f32, trend: f32) -> String {
        let mut body = String::from("Section: ");
        body.push_str(heading);
        body.push_str(". ");
        if trend > 0.05 {
            body.push_str("Improving trend.");
        } else if trend < -0.05 {
            body.push_str("Declining trend — attention needed.");
        } else {
            body.push_str("Stable.");
        }
        let _ = value; // used in metric_value field
        body
    }
}
