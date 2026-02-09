// SPDX-License-Identifier: GPL-2.0
//! # Apps Intuition Engine
//!
//! Fast application classification without full analysis. The intuition engine
//! maintains a cache of heuristic rules — pattern→classification mappings —
//! that enable near-instant classification of known application behaviors.
//!
//! When a new behavioral sample arrives, the intuition engine first attempts
//! a cache lookup using an FNV-1a hash of the feature fingerprint. If a
//! matching heuristic exists with sufficient confidence, the classification
//! is returned immediately without invoking the full analysis pipeline.
//!
//! The engine tracks hit rates, promotes frequently-validated patterns to
//! higher confidence tiers, and periodically compares intuitive classifications
//! against full-analysis results to calibrate its accuracy. Stale or
//! low-confidence heuristics are evicted to keep the cache lean.
//!
//! This mirrors the biological concept of "System 1" thinking — fast,
//! automatic, and usually correct for familiar situations.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.12;
const MAX_HEURISTICS: usize = 2048;
const PROMOTION_THRESHOLD: u64 = 10;
const CONFIDENCE_PROMOTE_DELTA: f32 = 0.05;
const CONFIDENCE_DEMOTE_DELTA: f32 = 0.08;
const EVICTION_CONFIDENCE: f32 = 0.15;
const STALE_TICKS: u64 = 5000;
const MAX_COMPARISON_LOG: usize = 256;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING
// ============================================================================

fn fnv1a_hash(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Hash a feature fingerprint (4 floats quantized to u16)
fn hash_fingerprint(cpu: f32, mem: f32, io: f32, net: f32) -> u64 {
    let c = (cpu.clamp(0.0, 1.0) * 65535.0) as u16;
    let m = (mem.clamp(0.0, 1.0) * 65535.0) as u16;
    let i = (io.clamp(0.0, 1.0) * 65535.0) as u16;
    let n = (net.clamp(0.0, 1.0) * 65535.0) as u16;
    let bytes = [
        (c >> 8) as u8,
        c as u8,
        (m >> 8) as u8,
        m as u8,
        (i >> 8) as u8,
        i as u8,
        (n >> 8) as u8,
        n as u8,
    ];
    fnv1a_hash(&bytes)
}

/// Xorshift64 PRNG for stochastic eviction
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// HEURISTIC RULE
// ============================================================================

/// A cached heuristic rule mapping a fingerprint to a classification
#[derive(Debug, Clone)]
pub struct HeuristicRule {
    pub fingerprint: u64,
    pub classification: String,
    pub classification_hash: u64,
    pub confidence: f32,
    pub hit_count: u64,
    pub miss_count: u64,
    pub last_hit_tick: u64,
    pub created_tick: u64,
    pub promoted: bool,
    variance: f32,
}

impl HeuristicRule {
    fn new(fingerprint: u64, classification: String, tick: u64) -> Self {
        let classification_hash = fnv1a_hash(classification.as_bytes());
        Self {
            fingerprint,
            classification,
            classification_hash,
            confidence: 0.5,
            hit_count: 0,
            miss_count: 0,
            last_hit_tick: tick,
            created_tick: tick,
            promoted: false,
            variance: 0.0,
        }
    }

    fn record_hit(&mut self, was_correct: bool, tick: u64) {
        self.last_hit_tick = tick;
        if was_correct {
            self.hit_count += 1;
            self.confidence = (self.confidence + CONFIDENCE_PROMOTE_DELTA).min(1.0);
        } else {
            self.miss_count += 1;
            self.confidence = (self.confidence - CONFIDENCE_DEMOTE_DELTA).max(0.0);
        }

        // EMA variance tracking
        let raw = if was_correct { 1.0_f32 } else { 0.0 };
        let diff = raw - self.confidence;
        self.variance = EMA_ALPHA * diff * diff + (1.0 - EMA_ALPHA) * self.variance;
    }

    fn accuracy(&self) -> f32 {
        let total = self.hit_count + self.miss_count;
        if total == 0 {
            return 0.5;
        }
        self.hit_count as f32 / total as f32
    }

    fn is_stale(&self, tick: u64) -> bool {
        tick.saturating_sub(self.last_hit_tick) > STALE_TICKS
    }
}

/// Comparison record: intuitive vs full analysis
#[derive(Debug, Clone)]
pub struct ClassificationComparison {
    pub tick: u64,
    pub fingerprint: u64,
    pub intuitive_class: u64,
    pub full_class: u64,
    pub matched: bool,
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate intuition engine statistics
#[derive(Debug, Clone)]
pub struct IntuitionStats {
    pub total_heuristics: usize,
    pub promoted_count: usize,
    pub total_lookups: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub hit_rate: f32,
    pub mean_confidence: f32,
    pub mean_accuracy: f32,
    pub stale_count: usize,
    pub comparison_agreement_rate: f32,
}

// ============================================================================
// APPS INTUITION ENGINE
// ============================================================================

/// Fast classification engine using cached heuristic rules
#[derive(Debug)]
pub struct AppsIntuitionEngine {
    rules: BTreeMap<u64, HeuristicRule>,
    comparisons: Vec<ClassificationComparison>,
    comp_write_idx: usize,
    total_lookups: u64,
    cache_hits: u64,
    cache_misses: u64,
    tick: u64,
    rng_state: u64,
    /// EMA-smoothed hit rate
    hit_rate_ema: f32,
}

impl AppsIntuitionEngine {
    pub fn new(seed: u64) -> Self {
        Self {
            rules: BTreeMap::new(),
            comparisons: Vec::new(),
            comp_write_idx: 0,
            total_lookups: 0,
            cache_hits: 0,
            cache_misses: 0,
            tick: 0,
            rng_state: if seed == 0 { 0x1470_CAFE_1234_ABCD } else { seed },
            hit_rate_ema: 0.5,
        }
    }

    /// Attempt fast intuitive classification of an app behavioral sample
    ///
    /// Returns `Some((classification_hash, confidence))` on cache hit, `None` on miss.
    pub fn intuitive_classify(
        &mut self,
        cpu: f32,
        mem: f32,
        io: f32,
        net: f32,
    ) -> Option<(u64, f32)> {
        self.tick += 1;
        self.total_lookups += 1;

        let fp = hash_fingerprint(cpu, mem, io, net);

        if let Some(rule) = self.rules.get(&fp) {
            if rule.confidence > EVICTION_CONFIDENCE {
                self.cache_hits += 1;
                self.hit_rate_ema =
                    EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * self.hit_rate_ema;
                return Some((rule.classification_hash, rule.confidence));
            }
        }

        // Try fuzzy match — check neighboring fingerprints
        let neighbors = [
            hash_fingerprint(
                (cpu + 0.01).min(1.0),
                mem,
                io,
                net,
            ),
            hash_fingerprint(
                (cpu - 0.01).max(0.0),
                mem,
                io,
                net,
            ),
            hash_fingerprint(
                cpu,
                (mem + 0.01).min(1.0),
                io,
                net,
            ),
            hash_fingerprint(
                cpu,
                (mem - 0.01).max(0.0),
                io,
                net,
            ),
        ];

        for nfp in &neighbors {
            if let Some(rule) = self.rules.get(nfp) {
                if rule.confidence > 0.5 {
                    self.cache_hits += 1;
                    self.hit_rate_ema =
                        EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * self.hit_rate_ema;
                    return Some((rule.classification_hash, rule.confidence * 0.9));
                }
            }
        }

        self.cache_misses += 1;
        self.hit_rate_ema = EMA_ALPHA * 0.0 + (1.0 - EMA_ALPHA) * self.hit_rate_ema;
        None
    }

    /// Build a new intuition rule from a confirmed classification
    pub fn build_intuition(
        &mut self,
        cpu: f32,
        mem: f32,
        io: f32,
        net: f32,
        classification: &str,
    ) {
        let fp = hash_fingerprint(cpu, mem, io, net);

        if let Some(existing) = self.rules.get_mut(&fp) {
            existing.record_hit(true, self.tick);
            if existing.hit_count >= PROMOTION_THRESHOLD && !existing.promoted {
                existing.promoted = true;
            }
        } else {
            // New rule
            let rule = HeuristicRule::new(fp, String::from(classification), self.tick);
            self.rules.insert(fp, rule);

            // Evict if over capacity
            if self.rules.len() > MAX_HEURISTICS {
                self.evict_weakest();
            }
        }
    }

    /// Current intuition hit rate
    pub fn intuition_hit_rate(&self) -> f32 {
        if self.total_lookups == 0 {
            return 0.0;
        }
        self.cache_hits as f32 / self.total_lookups as f32
    }

    /// Promote a pattern to higher confidence (called when classification is re-confirmed)
    pub fn promote_pattern(&mut self, cpu: f32, mem: f32, io: f32, net: f32) -> bool {
        let fp = hash_fingerprint(cpu, mem, io, net);
        if let Some(rule) = self.rules.get_mut(&fp) {
            rule.record_hit(true, self.tick);
            if rule.hit_count >= PROMOTION_THRESHOLD {
                rule.promoted = true;
            }
            true
        } else {
            false
        }
    }

    /// Compare intuitive classification against full analysis result
    pub fn intuition_vs_full_analysis(
        &mut self,
        cpu: f32,
        mem: f32,
        io: f32,
        net: f32,
        full_classification_hash: u64,
    ) -> bool {
        let fp = hash_fingerprint(cpu, mem, io, net);
        let matched = if let Some(rule) = self.rules.get_mut(&fp) {
            let matched = rule.classification_hash == full_classification_hash;
            rule.record_hit(matched, self.tick);
            matched
        } else {
            false
        };

        // Log comparison
        let intuitive_class = self
            .rules
            .get(&fp)
            .map(|r| r.classification_hash)
            .unwrap_or(0);

        let comp = ClassificationComparison {
            tick: self.tick,
            fingerprint: fp,
            intuitive_class,
            full_class: full_classification_hash,
            matched,
        };

        if self.comparisons.len() < MAX_COMPARISON_LOG {
            self.comparisons.push(comp);
        } else {
            self.comparisons[self.comp_write_idx] = comp;
        }
        self.comp_write_idx = (self.comp_write_idx + 1) % MAX_COMPARISON_LOG;

        matched
    }

    /// Cache management — evict stale and low-confidence rules
    pub fn cache_management(&mut self) -> usize {
        let tick = self.tick;
        let to_remove: Vec<u64> = self
            .rules
            .iter()
            .filter(|(_, rule)| {
                rule.is_stale(tick) || rule.confidence < EVICTION_CONFIDENCE
            })
            .map(|(fp, _)| *fp)
            .collect();

        let count = to_remove.len();
        for fp in to_remove {
            self.rules.remove(&fp);
        }
        count
    }

    /// Full stats
    pub fn stats(&self) -> IntuitionStats {
        let mut promoted = 0usize;
        let mut conf_sum = 0.0_f32;
        let mut acc_sum = 0.0_f32;
        let mut stale = 0usize;

        for (_, rule) in &self.rules {
            if rule.promoted {
                promoted += 1;
            }
            conf_sum += rule.confidence;
            acc_sum += rule.accuracy();
            if rule.is_stale(self.tick) {
                stale += 1;
            }
        }

        let n = self.rules.len().max(1) as f32;

        let agreement = if self.comparisons.is_empty() {
            1.0
        } else {
            let matched = self.comparisons.iter().filter(|c| c.matched).count();
            matched as f32 / self.comparisons.len() as f32
        };

        IntuitionStats {
            total_heuristics: self.rules.len(),
            promoted_count: promoted,
            total_lookups: self.total_lookups,
            cache_hits: self.cache_hits,
            cache_misses: self.cache_misses,
            hit_rate: self.intuition_hit_rate(),
            mean_confidence: conf_sum / n,
            mean_accuracy: acc_sum / n,
            stale_count: stale,
            comparison_agreement_rate: agreement,
        }
    }

    /// Number of cached rules
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Get the EMA-smoothed hit rate
    pub fn smoothed_hit_rate(&self) -> f32 {
        self.hit_rate_ema
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn evict_weakest(&mut self) {
        let mut weakest_fp = 0u64;
        let mut weakest_conf = f32::MAX;

        for (fp, rule) in &self.rules {
            if rule.confidence < weakest_conf {
                weakest_conf = rule.confidence;
                weakest_fp = *fp;
            }
        }

        if weakest_fp != 0 {
            self.rules.remove(&weakest_fp);
        }
    }

    /// Stochastic exploration — randomly demote a high-confidence rule for re-evaluation
    pub fn explore_rule(&mut self) -> Option<u64> {
        let high_conf: Vec<u64> = self
            .rules
            .iter()
            .filter(|(_, r)| r.confidence > 0.8 && r.promoted)
            .map(|(fp, _)| *fp)
            .collect();

        if high_conf.is_empty() {
            return None;
        }

        let idx = (xorshift64(&mut self.rng_state) as usize) % high_conf.len();
        let fp = high_conf[idx];
        if let Some(rule) = self.rules.get_mut(&fp) {
            rule.confidence *= 0.85;
            rule.promoted = false;
        }
        Some(fp)
    }
}
