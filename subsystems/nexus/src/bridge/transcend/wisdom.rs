// SPDX-License-Identifier: GPL-2.0
//! # Bridge Wisdom — Accumulated Wisdom Engine
//!
//! Unlike knowledge (facts), wisdom = knowing *when* and *how* to apply
//! knowledge. Each `WisdomEntry` links a context to advice, a confidence
//! level, and a track record (times_successful / times_applied). The engine
//! distinguishes wisdom from raw knowledge, builds contextual advice
//! indices, and exposes a `sage_decision()` that merges multiple wisdom
//! entries into a weighted recommendation.
//!
//! FNV-1a hashing indexes entries by context; xorshift64 drives stochastic
//! wisdom audits; EMA tracks running wisdom depth.

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_WISDOM_ENTRIES: usize = 1024;
const MAX_CONTEXT_TAGS: usize = 16;
const MAX_ADVICE_CANDIDATES: usize = 16;
const MAX_AUDIT_SAMPLE: usize = 64;
const EMA_ALPHA: f32 = 0.10;
const WISDOM_MATURITY_THRESHOLD: u64 = 10;
const SAGE_CONFIDENCE_MIN: f32 = 0.50;
const DEPTH_SCALE: f32 = 100.0;
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
// WISDOM TYPES
// ============================================================================

/// Domain the wisdom pertains to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WisdomDomain {
    SyscallRouting,
    ResourceAllocation,
    ErrorHandling,
    PerformanceTuning,
    SecurityPolicy,
    SchedulingHeuristic,
    MemoryLayout,
    ConcurrencyControl,
}

/// How the wisdom was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum WisdomOrigin {
    Experience,
    Inference,
    Analogy,
    Synthesis,
    Serendipity,
    Teaching,
}

/// A single wisdom entry.
#[derive(Debug, Clone)]
pub struct WisdomEntry {
    pub wisdom_id: u64,
    pub context: String,
    pub context_tags: Vec<String>,
    pub advice: String,
    pub domain: WisdomDomain,
    pub origin: WisdomOrigin,
    pub confidence: f32,
    pub times_applied: u64,
    pub times_successful: u64,
    pub success_rate: f32,
    pub depth_score: f32,
    pub created_tick: u64,
    pub last_applied_tick: u64,
}

/// Contextual advice recommendation.
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ContextualAdvice {
    pub context: String,
    pub candidates: Vec<(u64, f32)>, // wisdom_id, relevance
    pub best_advice: Option<String>,
    pub best_confidence: f32,
}

/// Wisdom vs knowledge comparison.
#[derive(Debug, Clone)]
pub struct WisdomKnowledgeComparison {
    pub total_wisdom: u64,
    pub mature_wisdom: u64,
    pub avg_success_rate: f32,
    pub depth_score: f32,
    pub is_wise: bool,
}

/// A sage decision merging multiple wisdom entries.
#[derive(Debug, Clone)]
pub struct SageDecision {
    pub decision_context: String,
    pub contributing_entries: Vec<(u64, f32)>,
    pub merged_advice: String,
    pub merged_confidence: f32,
    pub wisdom_depth: f32,
}

// ============================================================================
// WISDOM STATS
// ============================================================================

/// Aggregate statistics for the wisdom engine.
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct WisdomStats {
    pub total_entries: u64,
    pub mature_entries: u64,
    pub avg_confidence: f32,
    pub avg_success_rate: f32,
    pub avg_depth: f32,
    pub consultations: u64,
    pub successful_advice: u64,
    pub wisdom_depth_ema: f32,
}

// ============================================================================
// CONTEXT INDEX
// ============================================================================

#[derive(Debug, Clone)]
struct ContextIndex {
    tag_to_entries: BTreeMap<u64, Vec<u64>>, // tag_hash -> wisdom_ids
}

impl ContextIndex {
    fn new() -> Self {
        Self { tag_to_entries: BTreeMap::new() }
    }

    fn index_entry(&mut self, wisdom_id: u64, tags: &[String]) {
        for tag in tags {
            let h = fnv1a_hash(tag.as_bytes());
            let list = self.tag_to_entries.entry(h).or_insert_with(Vec::new);
            if !list.contains(&wisdom_id) {
                list.push(wisdom_id);
            }
        }
    }

    fn lookup(&self, tags: &[String]) -> Vec<(u64, usize)> {
        // Returns wisdom_ids with a count of how many tags matched
        let mut hits: LinearMap<usize, 64> = BTreeMap::new();
        for tag in tags {
            let h = fnv1a_hash(tag.as_bytes());
            if let Some(ids) = self.tag_to_entries.get(&h) {
                for &wid in ids {
                    *hits.entry(wid).or_insert(0) += 1;
                }
            }
        }
        let mut result: Vec<(u64, usize)> = hits.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }
}

// ============================================================================
// BRIDGE WISDOM ENGINE
// ============================================================================

/// Accumulated wisdom engine. Tracks contextual advice, success rates,
/// and provides sage-level decision support by merging relevant wisdom.
#[derive(Debug)]
#[repr(align(64))]
pub struct BridgeWisdom {
    entries: BTreeMap<u64, WisdomEntry>,
    index: ContextIndex,
    consultations: u64,
    successful_advice: u64,
    tick: u64,
    rng_state: u64,
    depth_ema: f32,
    confidence_ema: f32,
}

impl BridgeWisdom {
    /// Create a new wisdom engine.
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            index: ContextIndex::new(),
            consultations: 0,
            successful_advice: 0,
            tick: 0,
            rng_state: seed ^ 0xW15D_0000_SAGE,
            depth_ema: 0.0,
            confidence_ema: 0.5,
        }
    }

    /// Accumulate a new wisdom entry.
    #[inline]
    pub fn accumulate_wisdom(
        &mut self,
        context: &str,
        tags: &[String],
        advice: &str,
        domain: WisdomDomain,
        origin: WisdomOrigin,
        confidence: f32,
    ) -> u64 {
        self.tick += 1;
        let wid = fnv1a_hash(context.as_bytes()) ^ fnv1a_hash(advice.as_bytes()) ^ self.tick;

        let clamped_tags: Vec<String> = tags.iter().take(MAX_CONTEXT_TAGS).cloned().collect();

        let entry = WisdomEntry {
            wisdom_id: wid,
            context: String::from(context),
            context_tags: clamped_tags.clone(),
            advice: String::from(advice),
            domain,
            origin,
            confidence: confidence.max(0.0).min(1.0),
            times_applied: 0,
            times_successful: 0,
            success_rate: 0.0,
            depth_score: 0.0,
            created_tick: self.tick,
            last_applied_tick: 0,
        };

        if self.entries.len() >= MAX_WISDOM_ENTRIES {
            // Evict the entry with lowest success rate among mature entries,
            // or oldest immature entry
            let evict_id = self.find_eviction_candidate();
            if let Some(eid) = evict_id {
                self.entries.remove(&eid);
            }
        }

        self.index.index_entry(wid, &clamped_tags);
        self.entries.insert(wid, entry);

        self.confidence_ema = EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.confidence_ema;
        wid
    }

    /// Consult the wisdom engine for a context.
    pub fn consult_wisdom(&mut self, context_tags: &[String]) -> ContextualAdvice {
        self.tick += 1;
        self.consultations += 1;

        let hits = self.index.lookup(context_tags);
        let mut candidates = Vec::new();
        let mut best_advice = None;
        let mut best_conf = 0.0_f32;

        for (wid, match_count) in hits.iter().take(MAX_ADVICE_CANDIDATES) {
            if let Some(entry) = self.entries.get(wid) {
                let relevance = *match_count as f32 / context_tags.len().max(1) as f32;
                let weighted = relevance * entry.confidence * (1.0 + entry.success_rate);
                candidates.push((*wid, weighted));

                if weighted > best_conf {
                    best_conf = weighted;
                    best_advice = Some(entry.advice.clone());
                }
            }
        }

        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

        ContextualAdvice {
            context: {
                let mut s = String::new();
                for (i, tag) in context_tags.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    s.push_str(tag);
                }
                s
            },
            candidates,
            best_advice,
            best_confidence: best_conf,
        }
    }

    /// Record the outcome of applying wisdom.
    #[inline]
    pub fn record_outcome(&mut self, wisdom_id: u64, was_successful: bool) {
        self.tick += 1;
        if let Some(entry) = self.entries.get_mut(&wisdom_id) {
            entry.times_applied += 1;
            entry.last_applied_tick = self.tick;
            if was_successful {
                entry.times_successful += 1;
                self.successful_advice += 1;
            }
            entry.success_rate = if entry.times_applied > 0 {
                entry.times_successful as f32 / entry.times_applied as f32
            } else {
                0.0
            };
            // Update depth: depth grows with successful applications
            entry.depth_score = (entry.times_successful as f32 / DEPTH_SCALE).min(1.0);
            self.depth_ema =
                EMA_ALPHA * entry.depth_score + (1.0 - EMA_ALPHA) * self.depth_ema;
        }
    }

    /// Compare wisdom vs raw knowledge: how much wisdom is mature and proven.
    pub fn wisdom_vs_knowledge(&self) -> WisdomKnowledgeComparison {
        let total = self.entries.len() as u64;
        let mature = self
            .entries
            .values()
            .filter(|e| e.times_applied >= WISDOM_MATURITY_THRESHOLD)
            .count() as u64;
        let avg_sr = if self.entries.is_empty() {
            0.0
        } else {
            let sum: f32 = self.entries.values().map(|e| e.success_rate).sum();
            sum / self.entries.len() as f32
        };

        WisdomKnowledgeComparison {
            total_wisdom: total,
            mature_wisdom: mature,
            avg_success_rate: avg_sr,
            depth_score: self.depth_ema,
            is_wise: mature as f32 > total as f32 * 0.3 && avg_sr > 0.6,
        }
    }

    /// Get contextual advice for a specific context string (hashed to tags).
    #[inline(always)]
    pub fn contextual_advice(&mut self, context: &str) -> ContextualAdvice {
        let tags = self.extract_tags(context);
        self.consult_wisdom(&tags)
    }

    /// Wisdom depth: how deep and proven the overall wisdom corpus is.
    #[inline]
    pub fn wisdom_depth(&self) -> f32 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.entries.values().map(|e| e.depth_score).sum();
        sum / self.entries.len() as f32
    }

    /// Sage decision: merge the top-N relevant wisdom entries into a single
    /// weighted recommendation.
    pub fn sage_decision(&mut self, context_tags: &[String]) -> SageDecision {
        self.tick += 1;
        let advice = self.consult_wisdom(context_tags);

        let mut contributing = Vec::new();
        let mut merged_parts = Vec::new();
        let mut weight_sum = 0.0_f32;
        let mut conf_sum = 0.0_f32;
        let mut depth_sum = 0.0_f32;

        for (wid, relevance) in advice.candidates.iter().take(MAX_ADVICE_CANDIDATES) {
            if *relevance < SAGE_CONFIDENCE_MIN * 0.5 {
                continue;
            }
            if let Some(entry) = self.entries.get(wid) {
                contributing.push((*wid, *relevance));
                merged_parts.push(entry.advice.clone());
                conf_sum += entry.confidence * relevance;
                weight_sum += relevance;
                depth_sum += entry.depth_score;
            }
        }

        let merged_advice = {
            let mut s = String::new();
            for (i, part) in merged_parts.iter().enumerate() {
                if i > 0 {
                    s.push_str(" | ");
                }
                s.push_str(part);
            }
            s
        };

        let merged_conf = if weight_sum > 0.0 { conf_sum / weight_sum } else { 0.0 };
        let n = contributing.len().max(1) as f32;

        SageDecision {
            decision_context: {
                let mut s = String::new();
                for (i, tag) in context_tags.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    s.push_str(tag);
                }
                s
            },
            contributing_entries: contributing,
            merged_advice,
            merged_confidence: merged_conf,
            wisdom_depth: depth_sum / n,
        }
    }

    /// Get a wisdom entry by ID.
    #[inline(always)]
    pub fn get_entry(&self, wisdom_id: u64) -> Option<&WisdomEntry> {
        self.entries.get(&wisdom_id)
    }

    /// Total entries.
    #[inline(always)]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Aggregate statistics.
    pub fn stats(&self) -> WisdomStats {
        let total = self.entries.len() as u64;
        let mature = self
            .entries
            .values()
            .filter(|e| e.times_applied >= WISDOM_MATURITY_THRESHOLD)
            .count() as u64;

        let (avg_conf, avg_sr, avg_depth) = if self.entries.is_empty() {
            (0.0, 0.0, 0.0)
        } else {
            let n = self.entries.len() as f32;
            let conf: f32 = self.entries.values().map(|e| e.confidence).sum();
            let sr: f32 = self.entries.values().map(|e| e.success_rate).sum();
            let depth: f32 = self.entries.values().map(|e| e.depth_score).sum();
            (conf / n, sr / n, depth / n)
        };

        WisdomStats {
            total_entries: total,
            mature_entries: mature,
            avg_confidence: avg_conf,
            avg_success_rate: avg_sr,
            avg_depth,
            consultations: self.consultations,
            successful_advice: self.successful_advice,
            wisdom_depth_ema: self.depth_ema,
        }
    }

    /// Current tick.
    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }

    // --- private helpers ---

    fn find_eviction_candidate(&self) -> Option<u64> {
        // Prefer evicting immature entries with low confidence
        let mut worst_id = None;
        let mut worst_score = f32::MAX;

        for (&wid, entry) in &self.entries {
            let score = entry.confidence * 0.5
                + entry.success_rate * 0.3
                + (entry.times_applied as f32 / DEPTH_SCALE) * 0.2;
            if score < worst_score {
                worst_score = score;
                worst_id = Some(wid);
            }
        }

        worst_id
    }

    fn extract_tags(&self, context: &str) -> Vec<String> {
        // Split on common delimiters to produce tag-like tokens
        let mut tags = Vec::new();
        let mut current = String::new();
        for ch in context.chars() {
            if ch == ' ' || ch == ',' || ch == ';' || ch == ':' || ch == '/' {
                if !current.is_empty() {
                    tags.push(core::mem::replace(&mut current, String::new()));
                }
            } else {
                current.push(ch);
            }
        }
        if !current.is_empty() {
            tags.push(current);
        }
        tags.truncate(MAX_CONTEXT_TAGS);
        tags
    }
}
