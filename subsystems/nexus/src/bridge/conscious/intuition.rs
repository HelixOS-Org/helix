// SPDX-License-Identifier: GPL-2.0
//! # Bridge Intuition Engine
//!
//! Fast pattern matching without full analysis. For time-critical syscalls,
//! the bridge uses "intuition" — cached pattern matches that bypass expensive
//! full-analysis pipelines.
//!
//! Intuition rules are built from experience: frequently-seen patterns
//! with consistent outcomes get promoted to intuition rules. The engine
//! tracks when intuition disagrees with full analysis, measuring accuracy
//! and demoting unreliable rules.
//!
//! This provides a fast-path optimization: O(1) hash lookup vs O(n) analysis.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_INTUITION_RULES: usize = 512;
const MAX_CANDIDATE_PATTERNS: usize = 256;
const MAX_COMPARISON_HISTORY: usize = 128;
const PROMOTION_THRESHOLD_HITS: u32 = 10;
const PROMOTION_CONFIDENCE_MIN: f32 = 0.80;
const DEMOTION_ACCURACY_MIN: f32 = 0.60;
const ACCURACY_EMA_ALPHA: f32 = 0.08;
const CONFIDENCE_DECAY_RATE: f32 = 0.005;
const HIT_BONUS: f32 = 0.02;
const MISS_PENALTY: f32 = 0.05;
const CACHE_WARM_THRESHOLD: usize = 50;
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

fn ema_update(current: f32, sample: f32, alpha: f32) -> f32 {
    current * (1.0 - alpha) + sample * alpha
}

// ============================================================================
// INTUITION DECISION
// ============================================================================

/// The result of an intuitive decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntuitionDecision {
    /// Intuition has a confident answer
    Decided(u64),
    /// No intuition available — fall back to full analysis
    NoMatch,
    /// Intuition exists but confidence too low
    LowConfidence,
}

// ============================================================================
// INTUITION RULE
// ============================================================================

/// A cached pattern→action rule for fast decisions
#[derive(Debug, Clone)]
pub struct IntuitionRule {
    pub pattern_hash: u64,
    pub action_hash: u64,
    pub action_name: String,
    pub confidence: f32,
    pub hit_count: u64,
    pub miss_count: u64,
    pub created_tick: u64,
    pub last_hit_tick: u64,
    pub accuracy: f32,
    pub total_time_saved_ns: u64,
}

impl IntuitionRule {
    fn new(pattern_hash: u64, action: &str, confidence: f32, tick: u64) -> Self {
        Self {
            pattern_hash,
            action_hash: fnv1a_hash(action.as_bytes()),
            action_name: String::from(action),
            confidence: confidence.clamp(0.0, 1.0),
            hit_count: 0,
            miss_count: 0,
            created_tick: tick,
            last_hit_tick: tick,
            accuracy: 1.0,
            total_time_saved_ns: 0,
        }
    }

    fn record_hit(&mut self, tick: u64, time_saved_ns: u64) {
        self.hit_count += 1;
        self.last_hit_tick = tick;
        self.confidence = (self.confidence + HIT_BONUS).clamp(0.0, 1.0);
        self.total_time_saved_ns += time_saved_ns;
        self.accuracy = ema_update(self.accuracy, 1.0, ACCURACY_EMA_ALPHA);
    }

    fn record_miss(&mut self) {
        self.miss_count += 1;
        self.confidence = (self.confidence - MISS_PENALTY).clamp(0.0, 1.0);
        self.accuracy = ema_update(self.accuracy, 0.0, ACCURACY_EMA_ALPHA);
    }

    fn total_uses(&self) -> u64 {
        self.hit_count + self.miss_count
    }

    fn hit_rate(&self) -> f32 {
        let total = self.total_uses();
        if total == 0 {
            return 0.0;
        }
        self.hit_count as f32 / total as f32
    }

    fn should_demote(&self) -> bool {
        self.total_uses() > 5 && self.accuracy < DEMOTION_ACCURACY_MIN
    }

    fn decay(&mut self) {
        self.confidence = (self.confidence - CONFIDENCE_DECAY_RATE).max(0.0);
    }
}

// ============================================================================
// CANDIDATE PATTERN
// ============================================================================

/// A pattern being observed for potential promotion to intuition
#[derive(Debug, Clone)]
pub struct CandidatePattern {
    pub pattern_hash: u64,
    pub observed_action: String,
    pub occurrence_count: u32,
    pub consistent_action_count: u32,
    pub first_seen_tick: u64,
    pub last_seen_tick: u64,
}

impl CandidatePattern {
    fn consistency(&self) -> f32 {
        if self.occurrence_count == 0 {
            return 0.0;
        }
        self.consistent_action_count as f32 / self.occurrence_count as f32
    }

    fn ready_for_promotion(&self) -> bool {
        self.occurrence_count >= PROMOTION_THRESHOLD_HITS
            && self.consistency() >= PROMOTION_CONFIDENCE_MIN
    }
}

// ============================================================================
// COMPARISON RECORD
// ============================================================================

/// Record of intuition vs full analysis comparison
#[derive(Debug, Clone)]
pub struct ComparisonRecord {
    pub pattern_hash: u64,
    pub intuition_action: u64,
    pub analysis_action: u64,
    pub agreed: bool,
    pub tick: u64,
    pub analysis_time_ns: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Intuition engine statistics
#[derive(Debug, Clone)]
pub struct IntuitionStats {
    pub total_rules: usize,
    pub total_candidates: usize,
    pub total_decisions: u64,
    pub intuition_hit_rate: f32,
    pub avg_accuracy: f32,
    pub total_time_saved_ns: u64,
    pub agreement_rate: f32,
    pub promotions: u64,
    pub demotions: u64,
}

// ============================================================================
// BRIDGE INTUITION ENGINE
// ============================================================================

/// Fast-path decision engine using cached pattern→action rules
#[derive(Debug, Clone)]
pub struct BridgeIntuitionEngine {
    rules: BTreeMap<u64, IntuitionRule>,
    candidates: BTreeMap<u64, CandidatePattern>,
    comparisons: Vec<ComparisonRecord>,
    current_tick: u64,
    total_decisions: u64,
    total_hits: u64,
    total_misses: u64,
    total_no_match: u64,
    promotions: u64,
    demotions: u64,
    agreement_count: u64,
    disagreement_count: u64,
    avg_accuracy_ema: f32,
    rng_state: u64,
}

impl BridgeIntuitionEngine {
    /// Create a new intuition engine
    pub fn new(seed: u64) -> Self {
        Self {
            rules: BTreeMap::new(),
            candidates: BTreeMap::new(),
            comparisons: Vec::new(),
            current_tick: 0,
            total_decisions: 0,
            total_hits: 0,
            total_misses: 0,
            total_no_match: 0,
            promotions: 0,
            demotions: 0,
            agreement_count: 0,
            disagreement_count: 0,
            avg_accuracy_ema: 0.5,
            rng_state: seed | 1,
        }
    }

    /// Try to make an intuitive decision for a given pattern
    pub fn intuitive_decision(&mut self, pattern: &str) -> IntuitionDecision {
        self.current_tick += 1;
        self.total_decisions += 1;

        let hash = fnv1a_hash(pattern.as_bytes());

        if let Some(rule) = self.rules.get(&hash) {
            if rule.confidence >= PROMOTION_CONFIDENCE_MIN {
                self.total_hits += 1;
                return IntuitionDecision::Decided(rule.action_hash);
            } else if rule.confidence > 0.3 {
                return IntuitionDecision::LowConfidence;
            }
        }

        self.total_no_match += 1;
        IntuitionDecision::NoMatch
    }

    /// Record that intuition was used and whether it was correct
    pub fn record_outcome(
        &mut self,
        pattern: &str,
        was_correct: bool,
        time_saved_ns: u64,
    ) {
        let hash = fnv1a_hash(pattern.as_bytes());
        if let Some(rule) = self.rules.get_mut(&hash) {
            if was_correct {
                rule.record_hit(self.current_tick, time_saved_ns);
            } else {
                rule.record_miss();
                self.total_misses += 1;
            }
            self.avg_accuracy_ema =
                ema_update(self.avg_accuracy_ema, rule.accuracy, ACCURACY_EMA_ALPHA);
        }
    }

    /// Build intuition by observing a pattern-action pair
    pub fn build_intuition(&mut self, pattern: &str, action: &str) {
        self.current_tick += 1;
        let pattern_hash = fnv1a_hash(pattern.as_bytes());
        let action_hash = fnv1a_hash(action.as_bytes());

        // If already an intuition rule, reinforce it
        if let Some(rule) = self.rules.get_mut(&pattern_hash) {
            if rule.action_hash == action_hash {
                rule.record_hit(self.current_tick, 0);
            }
            return;
        }

        // Otherwise, track as a candidate
        if let Some(candidate) = self.candidates.get_mut(&pattern_hash) {
            candidate.occurrence_count += 1;
            candidate.last_seen_tick = self.current_tick;
            let existing_action_hash = fnv1a_hash(candidate.observed_action.as_bytes());
            if existing_action_hash == action_hash {
                candidate.consistent_action_count += 1;
            }
        } else if self.candidates.len() < MAX_CANDIDATE_PATTERNS {
            let candidate = CandidatePattern {
                pattern_hash,
                observed_action: String::from(action),
                occurrence_count: 1,
                consistent_action_count: 1,
                first_seen_tick: self.current_tick,
                last_seen_tick: self.current_tick,
            };
            self.candidates.insert(pattern_hash, candidate);
        }

        // Check if any candidates are ready for promotion
        self.try_promote();
    }

    fn try_promote(&mut self) {
        let mut to_promote = Vec::new();

        for (&hash, candidate) in &self.candidates {
            if candidate.ready_for_promotion() && self.rules.len() < MAX_INTUITION_RULES {
                to_promote.push((hash, candidate.observed_action.clone(), candidate.consistency()));
            }
        }

        for (hash, action, confidence) in to_promote {
            self.candidates.remove(&hash);
            let rule = IntuitionRule::new(hash, &action, confidence, self.current_tick);
            self.rules.insert(hash, rule);
            self.promotions += 1;
        }
    }

    /// Compare intuition against full analysis results
    pub fn intuition_vs_analysis(
        &mut self,
        pattern: &str,
        analysis_action: &str,
        analysis_time_ns: u64,
    ) -> bool {
        let pattern_hash = fnv1a_hash(pattern.as_bytes());
        let analysis_hash = fnv1a_hash(analysis_action.as_bytes());

        let intuition_action = self.rules.get(&pattern_hash).map(|r| r.action_hash);

        let agreed = intuition_action == Some(analysis_hash);

        if agreed {
            self.agreement_count += 1;
        } else {
            self.disagreement_count += 1;
        }

        if self.comparisons.len() >= MAX_COMPARISON_HISTORY {
            self.comparisons.remove(0);
        }
        self.comparisons.push(ComparisonRecord {
            pattern_hash,
            intuition_action: intuition_action.unwrap_or(0),
            analysis_action: analysis_hash,
            agreed,
            tick: self.current_tick,
            analysis_time_ns,
        });

        agreed
    }

    /// Promote a pattern directly to intuition with a given action
    pub fn promote_to_intuition(&mut self, pattern: &str, action: &str, confidence: f32) -> bool {
        if self.rules.len() >= MAX_INTUITION_RULES {
            return false;
        }
        self.current_tick += 1;
        let hash = fnv1a_hash(pattern.as_bytes());
        let rule = IntuitionRule::new(hash, action, confidence, self.current_tick);
        self.rules.insert(hash, rule);
        self.candidates.remove(&hash);
        self.promotions += 1;
        true
    }

    /// Current intuition cache size
    pub fn intuition_cache_size(&self) -> usize {
        self.rules.len()
    }

    /// Overall intuition accuracy
    pub fn intuition_accuracy(&self) -> f32 {
        self.avg_accuracy_ema
    }

    /// Intuition agreement rate with full analysis
    pub fn agreement_rate(&self) -> f32 {
        let total = self.agreement_count + self.disagreement_count;
        if total == 0 {
            return 0.0;
        }
        self.agreement_count as f32 / total as f32
    }

    /// Decay all intuition rules and demote unreliable ones
    pub fn maintenance_cycle(&mut self) {
        self.current_tick += 1;
        let mut to_demote = Vec::new();

        for (&hash, rule) in self.rules.iter_mut() {
            rule.decay();
            if rule.should_demote() {
                to_demote.push(hash);
            }
        }

        for hash in to_demote {
            self.rules.remove(&hash);
            self.demotions += 1;
        }
    }

    /// Get the top intuition rules by hit count
    pub fn top_rules(&self, limit: usize) -> Vec<(u64, String, u64, f32)> {
        let mut rules: Vec<(u64, String, u64, f32)> = self
            .rules
            .values()
            .map(|r| (r.pattern_hash, r.action_name.clone(), r.hit_count, r.accuracy))
            .collect();
        rules.sort_by(|a, b| b.2.cmp(&a.2));
        rules.truncate(limit);
        rules
    }

    /// Total time saved by intuition across all rules
    pub fn total_time_saved(&self) -> u64 {
        self.rules.values().map(|r| r.total_time_saved_ns).sum()
    }

    /// Is the cache warm enough for reliable intuition?
    pub fn is_cache_warm(&self) -> bool {
        self.rules.len() >= CACHE_WARM_THRESHOLD
    }

    /// Statistics snapshot
    pub fn stats(&self) -> IntuitionStats {
        let avg_acc = if self.rules.is_empty() {
            0.0
        } else {
            let total: f32 = self.rules.values().map(|r| r.accuracy).sum();
            total / self.rules.len() as f32
        };

        let hit_rate = {
            let total = self.total_hits + self.total_no_match;
            if total == 0 {
                0.0
            } else {
                self.total_hits as f32 / total as f32
            }
        };

        IntuitionStats {
            total_rules: self.rules.len(),
            total_candidates: self.candidates.len(),
            total_decisions: self.total_decisions,
            intuition_hit_rate: hit_rate,
            avg_accuracy: avg_acc,
            total_time_saved_ns: self.total_time_saved(),
            agreement_rate: self.agreement_rate(),
            promotions: self.promotions,
            demotions: self.demotions,
        }
    }

    /// Reset the intuition engine
    pub fn reset(&mut self) {
        self.rules.clear();
        self.candidates.clear();
        self.comparisons.clear();
        self.avg_accuracy_ema = 0.5;
    }
}
