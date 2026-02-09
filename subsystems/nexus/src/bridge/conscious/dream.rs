// SPDX-License-Identifier: GPL-2.0
//! # Bridge Dream Engine
//!
//! Offline consolidation when the system is idle. When CPU utilization drops
//! below threshold, the bridge "dreams": replaying recent syscall sequences,
//! identifying missed optimization opportunities, and consolidating patterns
//! into long-term memory.
//!
//! Dream sessions produce insights — discovered patterns that become new
//! optimization rules. Dream quality is measured by insight density and
//! applicability scores. The dreaming process mimics memory consolidation:
//! replaying events to strengthen important patterns and prune irrelevant ones.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const IDLE_THRESHOLD: f32 = 0.15;
const MAX_DREAM_SESSIONS: usize = 64;
const MAX_REPLAY_EVENTS: usize = 512;
const MAX_INSIGHTS: usize = 256;
const MAX_CONSOLIDATED_PATTERNS: usize = 128;
const INSIGHT_CONFIDENCE_THRESHOLD: f32 = 0.50;
const DREAM_QUALITY_EMA_ALPHA: f32 = 0.10;
const CONSOLIDATION_STRENGTH_BOOST: f32 = 0.15;
const REPLAY_DECAY_FACTOR: f32 = 0.95;
const MIN_REPLAY_LENGTH: usize = 4;
const PATTERN_MATCH_THRESHOLD: f32 = 0.70;
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
// DREAM SESSION STATE
// ============================================================================

/// State of a dream session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DreamPhase {
    /// Waiting for idle detection
    Waiting,
    /// Actively replaying events
    Replaying,
    /// Extracting insights from replays
    Extracting,
    /// Consolidating into long-term memory
    Consolidating,
    /// Dream complete
    Complete,
}

// ============================================================================
// REPLAY EVENT
// ============================================================================

/// A recorded event available for dream replay
#[derive(Debug, Clone)]
pub struct ReplayEvent {
    pub event_hash: u64,
    pub category: String,
    pub sequence_id: u64,
    pub timestamp_tick: u64,
    pub latency_ns: u64,
    pub was_optimized: bool,
    pub replay_count: u32,
}

// ============================================================================
// DREAM INSIGHT
// ============================================================================

/// An insight discovered during dreaming
#[derive(Debug, Clone)]
pub struct DreamInsight {
    pub pattern_hash: u64,
    pub pattern_description: String,
    pub confidence: f32,
    pub applicability: f32,
    pub discovered_tick: u64,
    pub source_events: usize,
    pub improvement_estimate: f32,
}

impl DreamInsight {
    fn quality_score(&self) -> f32 {
        self.confidence * 0.4 + self.applicability * 0.4 + self.improvement_estimate * 0.2
    }
}

// ============================================================================
// DREAM SESSION
// ============================================================================

/// A single dream session
#[derive(Debug, Clone)]
pub struct DreamSession {
    pub session_id: u64,
    pub start_tick: u64,
    pub end_tick: Option<u64>,
    pub phase: DreamPhase,
    pub replayed_events: usize,
    pub insights_found: usize,
    pub quality_score: f32,
    pub cpu_utilization_at_start: f32,
    pub patterns_consolidated: usize,
}

// ============================================================================
// CONSOLIDATED PATTERN
// ============================================================================

/// A pattern moved into long-term memory through consolidation
#[derive(Debug, Clone)]
pub struct ConsolidatedPattern {
    pub pattern_hash: u64,
    pub description: String,
    pub strength: f32,
    pub times_consolidated: u32,
    pub last_consolidated_tick: u64,
    pub applicability: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Dream engine statistics
#[derive(Debug, Clone)]
pub struct DreamStats {
    pub total_sessions: u64,
    pub total_insights: u64,
    pub total_consolidations: u64,
    pub avg_dream_quality: f32,
    pub avg_insights_per_session: f32,
    pub consolidated_pattern_count: usize,
    pub replay_event_count: usize,
    pub current_phase: DreamPhase,
}

// ============================================================================
// BRIDGE DREAM ENGINE
// ============================================================================

/// Offline consolidation engine for bridge pattern discovery
#[derive(Debug, Clone)]
pub struct BridgeDreamEngine {
    replay_buffer: Vec<ReplayEvent>,
    insights: Vec<DreamInsight>,
    sessions: Vec<DreamSession>,
    consolidated: BTreeMap<u64, ConsolidatedPattern>,
    current_session: Option<DreamSession>,
    current_phase: DreamPhase,
    session_counter: u64,
    sequence_counter: u64,
    total_insights: u64,
    total_consolidations: u64,
    avg_quality_ema: f32,
    current_tick: u64,
    rng_state: u64,
    // Working buffers for active dreaming
    replay_cursor: usize,
    working_patterns: BTreeMap<u64, (usize, f32)>,
}

impl BridgeDreamEngine {
    /// Create a new dream engine
    pub fn new(seed: u64) -> Self {
        Self {
            replay_buffer: Vec::new(),
            insights: Vec::new(),
            sessions: Vec::new(),
            consolidated: BTreeMap::new(),
            current_session: None,
            current_phase: DreamPhase::Waiting,
            session_counter: 0,
            sequence_counter: 0,
            total_insights: 0,
            total_consolidations: 0,
            avg_quality_ema: 0.0,
            current_tick: 0,
            rng_state: seed | 1,
            replay_cursor: 0,
            working_patterns: BTreeMap::new(),
        }
    }

    /// Record an event for potential replay during dreaming
    pub fn record_event(&mut self, category: &str, latency_ns: u64, was_optimized: bool) {
        self.current_tick += 1;
        self.sequence_counter += 1;

        if self.replay_buffer.len() >= MAX_REPLAY_EVENTS {
            self.replay_buffer.remove(0);
        }

        let event = ReplayEvent {
            event_hash: fnv1a_hash(category.as_bytes()) ^ self.sequence_counter,
            category: String::from(category),
            sequence_id: self.sequence_counter,
            timestamp_tick: self.current_tick,
            latency_ns,
            was_optimized,
            replay_count: 0,
        };
        self.replay_buffer.push(event);
    }

    /// Detect idle state and potentially start dreaming
    pub fn idle_detected(&mut self, cpu_utilization: f32) -> bool {
        self.current_tick += 1;
        if cpu_utilization < IDLE_THRESHOLD && self.current_phase == DreamPhase::Waiting {
            if self.replay_buffer.len() >= MIN_REPLAY_LENGTH {
                return true;
            }
        }
        false
    }

    /// Start a dream session
    pub fn start_dream(&mut self, cpu_utilization: f32) {
        self.current_tick += 1;
        self.session_counter += 1;

        let session = DreamSession {
            session_id: self.session_counter,
            start_tick: self.current_tick,
            end_tick: None,
            phase: DreamPhase::Replaying,
            replayed_events: 0,
            insights_found: 0,
            quality_score: 0.0,
            cpu_utilization_at_start: cpu_utilization,
            patterns_consolidated: 0,
        };
        self.current_session = Some(session);
        self.current_phase = DreamPhase::Replaying;
        self.replay_cursor = 0;
        self.working_patterns.clear();
    }

    /// Replay a batch of events during dreaming. Returns the number replayed.
    pub fn replay_sequence(&mut self, batch_size: usize) -> usize {
        if self.current_phase != DreamPhase::Replaying {
            return 0;
        }

        let start = self.replay_cursor;
        let end = (start + batch_size).min(self.replay_buffer.len());
        let mut replayed = 0;

        for i in start..end {
            let event = &mut self.replay_buffer[i];
            event.replay_count += 1;
            replayed += 1;

            // Build pattern signatures from sequences
            let cat_hash = fnv1a_hash(event.category.as_bytes());
            let entry = self.working_patterns.entry(cat_hash).or_insert((0, 0.0));
            entry.0 += 1;

            // Accumulate latency-weighted scores
            let latency_score = if event.latency_ns > 0 {
                1.0 / (1.0 + (event.latency_ns as f32 / 1_000_000.0))
            } else {
                1.0
            };
            entry.1 = entry.1 * REPLAY_DECAY_FACTOR + latency_score;
        }

        self.replay_cursor = end;

        if let Some(ref mut session) = self.current_session {
            session.replayed_events += replayed;
        }

        // If we've replayed everything, move to extraction phase
        if self.replay_cursor >= self.replay_buffer.len() {
            self.current_phase = DreamPhase::Extracting;
            if let Some(ref mut session) = self.current_session {
                session.phase = DreamPhase::Extracting;
            }
        }

        replayed
    }

    /// Extract insights from replayed patterns
    pub fn extract_insight(&mut self) -> Vec<DreamInsight> {
        if self.current_phase != DreamPhase::Extracting {
            return Vec::new();
        }

        let mut new_insights = Vec::new();

        // Analyze working patterns for insights
        for (&pattern_hash, &(count, score)) in &self.working_patterns {
            if count < 2 {
                continue;
            }

            let frequency_signal = (count as f32).ln() / 10.0;
            let confidence = (frequency_signal + score * 0.5).clamp(0.0, 1.0);

            if confidence < INSIGHT_CONFIDENCE_THRESHOLD {
                continue;
            }

            // Check if this event was often unoptimized (opportunity)
            let unoptimized_count = self
                .replay_buffer
                .iter()
                .filter(|e| fnv1a_hash(e.category.as_bytes()) == pattern_hash && !e.was_optimized)
                .count();

            let total_for_pattern = self
                .replay_buffer
                .iter()
                .filter(|e| fnv1a_hash(e.category.as_bytes()) == pattern_hash)
                .count();

            let improvement_ratio = if total_for_pattern > 0 {
                unoptimized_count as f32 / total_for_pattern as f32
            } else {
                0.0
            };

            let applicability = (improvement_ratio * confidence).clamp(0.0, 1.0);

            // Use xorshift to add some controlled randomness to discovery
            let noise = (xorshift64(&mut self.rng_state) % 100) as f32 / 1000.0;
            let final_confidence = (confidence + noise).clamp(0.0, 1.0);

            if self.insights.len() < MAX_INSIGHTS {
                let insight = DreamInsight {
                    pattern_hash,
                    pattern_description: {
                        let mut desc = String::from("pattern_");
                        // Encode hash into description
                        let h = pattern_hash;
                        for shift in (0..8).rev() {
                            let nibble = ((h >> (shift * 4)) & 0xF) as u8;
                            let c = if nibble < 10 {
                                b'0' + nibble
                            } else {
                                b'a' + nibble - 10
                            };
                            desc.push(c as char);
                        }
                        desc
                    },
                    confidence: final_confidence,
                    applicability,
                    discovered_tick: self.current_tick,
                    source_events: count,
                    improvement_estimate: improvement_ratio,
                };

                new_insights.push(insight.clone());
                self.insights.push(insight);
                self.total_insights += 1;
            }
        }

        if let Some(ref mut session) = self.current_session {
            session.insights_found += new_insights.len();
        }

        // Move to consolidation phase
        self.current_phase = DreamPhase::Consolidating;
        if let Some(ref mut session) = self.current_session {
            session.phase = DreamPhase::Consolidating;
        }

        new_insights
    }

    /// Consolidate insights into long-term memory
    pub fn consolidate_memory(&mut self) -> usize {
        if self.current_phase != DreamPhase::Consolidating {
            return 0;
        }

        let mut consolidated_count = 0;

        // Consolidate high-quality insights into long-term patterns
        let insights_to_consolidate: Vec<DreamInsight> = self
            .insights
            .iter()
            .filter(|i| i.quality_score() > PATTERN_MATCH_THRESHOLD)
            .cloned()
            .collect();

        for insight in &insights_to_consolidate {
            if let Some(existing) = self.consolidated.get_mut(&insight.pattern_hash) {
                existing.strength =
                    (existing.strength + CONSOLIDATION_STRENGTH_BOOST).clamp(0.0, 1.0);
                existing.times_consolidated += 1;
                existing.last_consolidated_tick = self.current_tick;
                existing.applicability = ema_update(
                    existing.applicability,
                    insight.applicability,
                    DREAM_QUALITY_EMA_ALPHA,
                );
                consolidated_count += 1;
            } else if self.consolidated.len() < MAX_CONSOLIDATED_PATTERNS {
                let pattern = ConsolidatedPattern {
                    pattern_hash: insight.pattern_hash,
                    description: insight.pattern_description.clone(),
                    strength: insight.confidence * CONSOLIDATION_STRENGTH_BOOST + 0.3,
                    times_consolidated: 1,
                    last_consolidated_tick: self.current_tick,
                    applicability: insight.applicability,
                };
                self.consolidated.insert(insight.pattern_hash, pattern);
                consolidated_count += 1;
                self.total_consolidations += 1;
            }
        }

        // Complete the dream session
        self.current_phase = DreamPhase::Complete;
        let quality = if let Some(ref mut session) = self.current_session {
            session.phase = DreamPhase::Complete;
            session.end_tick = Some(self.current_tick);
            session.patterns_consolidated = consolidated_count;

            let insight_density = if session.replayed_events > 0 {
                session.insights_found as f32 / session.replayed_events as f32
            } else {
                0.0
            };
            let q = (insight_density * 10.0 + consolidated_count as f32 * 0.2).clamp(0.0, 1.0);
            session.quality_score = q;
            q
        } else {
            0.0
        };

        self.avg_quality_ema = ema_update(self.avg_quality_ema, quality, DREAM_QUALITY_EMA_ALPHA);

        // Archive the session
        if let Some(session) = self.current_session.take() {
            if self.sessions.len() >= MAX_DREAM_SESSIONS {
                self.sessions.remove(0);
            }
            self.sessions.push(session);
        }

        self.current_phase = DreamPhase::Waiting;
        consolidated_count
    }

    /// Dream quality — EMA of session quality scores
    pub fn dream_quality(&self) -> f32 {
        self.avg_quality_ema
    }

    /// Is a consolidated pattern known?
    pub fn has_consolidated_pattern(&self, category: &str) -> bool {
        let hash = fnv1a_hash(category.as_bytes());
        self.consolidated.contains_key(&hash)
    }

    /// Get the strength of a consolidated pattern
    pub fn pattern_strength(&self, category: &str) -> f32 {
        let hash = fnv1a_hash(category.as_bytes());
        self.consolidated
            .get(&hash)
            .map(|p| p.strength)
            .unwrap_or(0.0)
    }

    /// Get all consolidated patterns sorted by strength
    pub fn strongest_patterns(&self) -> Vec<(String, f32)> {
        let mut patterns: Vec<(String, f32)> = self
            .consolidated
            .values()
            .map(|p| (p.description.clone(), p.strength))
            .collect();
        patterns.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        patterns
    }

    /// Decay consolidated patterns that haven't been reinforced
    pub fn decay_consolidated(&mut self, decay_amount: f32) {
        let mut to_remove = Vec::new();
        for (&hash, pattern) in self.consolidated.iter_mut() {
            let age = self.current_tick.saturating_sub(pattern.last_consolidated_tick);
            let age_factor = 1.0 / (1.0 + age as f32 / 1000.0);
            pattern.strength = (pattern.strength - decay_amount * (1.0 - age_factor)).max(0.0);
            if pattern.strength < 0.01 {
                to_remove.push(hash);
            }
        }
        for hash in to_remove {
            self.consolidated.remove(&hash);
        }
    }

    /// Statistics snapshot
    pub fn stats(&self) -> DreamStats {
        let avg_insights = if self.sessions.is_empty() {
            0.0
        } else {
            self.total_insights as f32 / self.sessions.len() as f32
        };

        DreamStats {
            total_sessions: self.session_counter,
            total_insights: self.total_insights,
            total_consolidations: self.total_consolidations,
            avg_dream_quality: self.avg_quality_ema,
            avg_insights_per_session: avg_insights,
            consolidated_pattern_count: self.consolidated.len(),
            replay_event_count: self.replay_buffer.len(),
            current_phase: self.current_phase,
        }
    }

    /// Reset the dream engine
    pub fn reset(&mut self) {
        self.replay_buffer.clear();
        self.insights.clear();
        self.sessions.clear();
        self.consolidated.clear();
        self.current_session = None;
        self.current_phase = DreamPhase::Waiting;
        self.replay_cursor = 0;
        self.working_patterns.clear();
        self.avg_quality_ema = 0.0;
    }
}
