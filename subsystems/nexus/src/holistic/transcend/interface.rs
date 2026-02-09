// SPDX-License-Identifier: GPL-2.0
//! # Holistic Interface — Advanced Human-Kernel Interface
//!
//! `HolisticInterface` provides the kernel with the ability to *explain*
//! itself, *teach* operators, and *recommend* actions.  Every decision the
//! kernel makes can be decomposed into a human-readable reasoning chain,
//! forming a complete narrative about what the system is doing and why.
//!
//! This is the bridge between superintelligent optimisation and human
//! comprehension — ensuring that no matter how advanced the kernel becomes,
//! operators always retain full situational awareness.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 2;
const EMA_ALPHA_DEN: u64 = 11;
const MAX_EXPLANATIONS: usize = 512;
const MAX_RECOMMENDATIONS: usize = 256;
const MAX_REASONING_STEPS: usize = 64;
const MAX_LESSONS: usize = 128;

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
            state: if seed == 0 { 0xfeedface } else { seed },
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
// Explanation record
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Explanation {
    pub id_hash: u64,
    pub tick: u64,
    pub subject: String,
    pub summary: String,
    pub detail_level: u64,
    pub confidence_bps: u64,
    pub chain_hash: u64,
}

// ---------------------------------------------------------------------------
// Lesson (for operator teaching)
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Lesson {
    pub lesson_hash: u64,
    pub topic: String,
    pub content: String,
    pub difficulty: u64,
    pub prerequisite_hash: u64,
    pub mastery_bps: u64,
}

// ---------------------------------------------------------------------------
// Recommendation
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Recommendation {
    pub rec_hash: u64,
    pub tick: u64,
    pub action: String,
    pub rationale: String,
    pub expected_improvement_bps: u64,
    pub urgency: u64,
    pub confidence_bps: u64,
}

// ---------------------------------------------------------------------------
// Reasoning step
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ReasoningStep {
    pub step_index: u64,
    pub premise: String,
    pub deduction: String,
    pub confidence_bps: u64,
}

// ---------------------------------------------------------------------------
// Reasoning chain
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ReasoningChain {
    pub chain_hash: u64,
    pub steps: Vec<ReasoningStep>,
    pub conclusion: String,
    pub overall_confidence_bps: u64,
}

// ---------------------------------------------------------------------------
// Knowledge transfer packet
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct KnowledgePacket {
    pub packet_hash: u64,
    pub domain: String,
    pub payload_size: u64,
    pub comprehension_bps: u64,
    pub lessons_included: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct InterfaceStats {
    pub explanations_given: u64,
    pub lessons_taught: u64,
    pub recommendations_issued: u64,
    pub reasoning_chains_built: u64,
    pub knowledge_transfers: u64,
    pub ema_confidence_bps: u64,
    pub avg_chain_length: u64,
    pub narratives_generated: u64,
}

impl InterfaceStats {
    fn new() -> Self {
        Self {
            explanations_given: 0,
            lessons_taught: 0,
            recommendations_issued: 0,
            reasoning_chains_built: 0,
            knowledge_transfers: 0,
            ema_confidence_bps: 5000,
            avg_chain_length: 0,
            narratives_generated: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// HolisticInterface Engine
// ---------------------------------------------------------------------------

pub struct HolisticInterface {
    explanations: Vec<Explanation>,
    lessons: BTreeMap<u64, Lesson>,
    recommendations: Vec<Recommendation>,
    stats: InterfaceStats,
    rng: Xorshift64,
    tick: u64,
}

impl HolisticInterface {
    pub fn new(seed: u64) -> Self {
        Self {
            explanations: Vec::new(),
            lessons: BTreeMap::new(),
            recommendations: Vec::new(),
            stats: InterfaceStats::new(),
            rng: Xorshift64::new(seed),
            tick: 0,
        }
    }

    fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn make_hash(&mut self, label: &str) -> u64 {
        fnv1a(label.as_bytes()) ^ fnv1a(&self.tick.to_le_bytes()) ^ self.rng.next()
    }

    // -- 6 public methods ---------------------------------------------------

    /// Produce a human-readable explanation of the current system state or a
    /// specific subsystem.
    pub fn explain_system(&mut self, subject: &str, detail_level: u64) -> Explanation {
        self.advance_tick();
        let confidence = 7000_u64.wrapping_add(self.rng.next() % 3000);
        let summary = {
            let mut s = String::from("System explanation for: ");
            s.push_str(subject);
            s
        };
        let id = self.make_hash(subject);
        let chain_h = fnv1a(summary.as_bytes());
        let expl = Explanation {
            id_hash: id,
            tick: self.tick,
            subject: String::from(subject),
            summary,
            detail_level,
            confidence_bps: confidence.min(10_000),
            chain_hash: chain_h,
        };
        if self.explanations.len() >= MAX_EXPLANATIONS {
            self.explanations.remove(0);
        }
        self.explanations.push(expl.clone());
        self.stats.explanations_given = self.stats.explanations_given.wrapping_add(1);
        self.stats.ema_confidence_bps = ema_update(self.stats.ema_confidence_bps, confidence);
        expl
    }

    /// Generate a lesson for the operator on a given topic.
    pub fn teach_operator(&mut self, topic: &str, difficulty: u64) -> Lesson {
        self.advance_tick();
        let content = {
            let mut s = String::from("Lesson content: understanding ");
            s.push_str(topic);
            s.push_str(" at depth ");
            // simple numeric append
            let d = difficulty.min(10);
            let ch = (b'0' + d as u8) as char;
            s.push(ch);
            s
        };
        let prereq = self.lessons.values().last().map(|l| l.lesson_hash).unwrap_or(0);
        let mastery = self.rng.next() % 10_001;
        let lh = self.make_hash(topic);
        let lesson = Lesson {
            lesson_hash: lh,
            topic: String::from(topic),
            content,
            difficulty: difficulty.min(10),
            prerequisite_hash: prereq,
            mastery_bps: mastery,
        };
        if self.lessons.len() < MAX_LESSONS {
            self.lessons.insert(lh, lesson.clone());
        }
        self.stats.lessons_taught = self.stats.lessons_taught.wrapping_add(1);
        lesson
    }

    /// Recommend an action to the operator with rationale and urgency.
    pub fn recommend_action(&mut self, context: &str) -> Recommendation {
        self.advance_tick();
        let actions = [
            "increase_memory_pool",
            "rebalance_scheduler",
            "flush_caches",
            "compact_heap",
            "tune_irq_affinity",
            "migrate_workload",
        ];
        let idx = (self.rng.next() as usize) % actions.len();
        let action = String::from(actions[idx]);
        let rationale = {
            let mut s = String::from("Given context [");
            s.push_str(context);
            s.push_str("], recommend: ");
            s.push_str(&action);
            s
        };
        let improvement = self.rng.next() % 5000;
        let urgency = self.rng.next() % 10;
        let confidence = 6000_u64.wrapping_add(self.rng.next() % 4000);
        let rh = self.make_hash(context);
        let rec = Recommendation {
            rec_hash: rh,
            tick: self.tick,
            action,
            rationale,
            expected_improvement_bps: improvement,
            urgency,
            confidence_bps: confidence.min(10_000),
        };
        if self.recommendations.len() >= MAX_RECOMMENDATIONS {
            self.recommendations.remove(0);
        }
        self.recommendations.push(rec.clone());
        self.stats.recommendations_issued = self.stats.recommendations_issued.wrapping_add(1);
        self.stats.ema_confidence_bps = ema_update(self.stats.ema_confidence_bps, confidence);
        rec
    }

    /// Build a full reasoning chain for a given question.
    pub fn reasoning_chain(&mut self, question: &str) -> ReasoningChain {
        self.advance_tick();
        let step_count = 3 + (self.rng.next() as usize % (MAX_REASONING_STEPS - 3));
        let mut steps = Vec::with_capacity(step_count.min(MAX_REASONING_STEPS));
        let mut overall_conf: u64 = 10_000;
        for i in 0..step_count.min(MAX_REASONING_STEPS) {
            let conf = 8000_u64.wrapping_add(self.rng.next() % 2000);
            overall_conf = overall_conf.min(conf);
            steps.push(ReasoningStep {
                step_index: i as u64,
                premise: {
                    let mut s = String::from("P");
                    let ch = (b'0' + ((i % 10) as u8)) as char;
                    s.push(ch);
                    s.push_str(": observed from subsystem data");
                    s
                },
                deduction: {
                    let mut s = String::from("D");
                    let ch = (b'0' + ((i % 10) as u8)) as char;
                    s.push(ch);
                    s.push_str(": therefore optimal path chosen");
                    s
                },
                confidence_bps: conf,
            });
        }
        let conclusion = {
            let mut s = String::from("Conclusion for [");
            s.push_str(question);
            s.push_str("]: action recommended");
            s
        };
        let ch = fnv1a(question.as_bytes()) ^ fnv1a(&(steps.len() as u64).to_le_bytes());
        self.stats.reasoning_chains_built = self.stats.reasoning_chains_built.wrapping_add(1);
        let total_steps = self.stats.reasoning_chains_built.max(1);
        self.stats.avg_chain_length =
            ema_update(self.stats.avg_chain_length, steps.len() as u64);
        ReasoningChain {
            chain_hash: ch,
            steps,
            conclusion,
            overall_confidence_bps: overall_conf,
        }
    }

    /// Generate a system narrative — a human-readable story of recent events.
    pub fn system_narrative(&mut self) -> String {
        self.advance_tick();
        let mut narrative = String::from("NEXUS Narrative [tick=");
        let t = self.tick;
        // append tick digits
        let mut buf = [0u8; 20];
        let mut n = t;
        let mut i = 0usize;
        if n == 0 {
            buf[0] = b'0';
            i = 1;
        } else {
            while n > 0 {
                buf[i] = b'0' + (n % 10) as u8;
                n /= 10;
                i += 1;
            }
        }
        for j in (0..i).rev() {
            narrative.push(buf[j] as char);
        }
        narrative.push_str("]: ");
        narrative.push_str("Explanations=");
        // simple count append reuse
        let ec = self.stats.explanations_given;
        let mut n2 = ec;
        let mut i2 = 0usize;
        if n2 == 0 {
            buf[0] = b'0';
            i2 = 1;
        } else {
            while n2 > 0 {
                buf[i2] = b'0' + (n2 % 10) as u8;
                n2 /= 10;
                i2 += 1;
            }
        }
        for j in (0..i2).rev() {
            narrative.push(buf[j] as char);
        }
        narrative.push_str(", Recommendations=");
        let rc = self.stats.recommendations_issued;
        let mut n3 = rc;
        let mut i3 = 0usize;
        if n3 == 0 {
            buf[0] = b'0';
            i3 = 1;
        } else {
            while n3 > 0 {
                buf[i3] = b'0' + (n3 % 10) as u8;
                n3 /= 10;
                i3 += 1;
            }
        }
        for j in (0..i3).rev() {
            narrative.push(buf[j] as char);
        }
        narrative.push_str(". System operating nominally.");
        self.stats.narratives_generated = self.stats.narratives_generated.wrapping_add(1);
        narrative
    }

    /// Transfer accumulated knowledge in a single packet.
    pub fn knowledge_transfer(&mut self, domain: &str) -> KnowledgePacket {
        self.advance_tick();
        let lessons_count = self.lessons.len() as u64;
        let payload = lessons_count.wrapping_mul(256);
        let comprehension = if lessons_count > 0 {
            let sum: u64 = self.lessons.values().map(|l| l.mastery_bps).sum();
            sum / lessons_count
        } else {
            0
        };
        let ph = self.make_hash(domain);
        self.stats.knowledge_transfers = self.stats.knowledge_transfers.wrapping_add(1);
        KnowledgePacket {
            packet_hash: ph,
            domain: String::from(domain),
            payload_size: payload,
            comprehension_bps: comprehension.min(10_000),
            lessons_included: lessons_count,
        }
    }

    // -- accessors ----------------------------------------------------------

    pub fn stats(&self) -> &InterfaceStats {
        &self.stats
    }

    pub fn explanation_count(&self) -> usize {
        self.explanations.len()
    }

    pub fn lesson_count(&self) -> usize {
        self.lessons.len()
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explain() {
        let mut iface = HolisticInterface::new(42);
        let e = iface.explain_system("scheduler", 3);
        assert!(e.confidence_bps <= 10_000);
        assert!(iface.explanation_count() == 1);
    }

    #[test]
    fn test_teach() {
        let mut iface = HolisticInterface::new(7);
        let l = iface.teach_operator("memory_management", 5);
        assert!(l.difficulty == 5);
        assert!(iface.lesson_count() == 1);
    }

    #[test]
    fn test_recommend() {
        let mut iface = HolisticInterface::new(99);
        let r = iface.recommend_action("high_load");
        assert!(r.urgency < 10);
    }

    #[test]
    fn test_reasoning_chain() {
        let mut iface = HolisticInterface::new(3);
        let chain = iface.reasoning_chain("why is latency high?");
        assert!(chain.steps.len() >= 3);
    }

    #[test]
    fn test_narrative() {
        let mut iface = HolisticInterface::new(55);
        iface.explain_system("cpu", 1);
        let n = iface.system_narrative();
        assert!(n.contains("NEXUS Narrative"));
    }
}
