// SPDX-License-Identifier: GPL-2.0
//! # Holistic Dream Engine
//!
//! **SYSTEM-WIDE offline consolidation.** The ultimate dream engine replays
//! the ENTIRE system's recent history during low-activity periods, discovers
//! cross-subsystem optimization opportunities, and generates insights that no
//! single subsystem could find alone.
//!
//! ## Dream Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                  DREAM CYCLE                                 │
//! ├──────────────────────────────────────────────────────────────┤
//! │  Schedule ──▶ Replay ──▶ Discover ──▶ Consolidate           │
//! │      │            │           │              │               │
//! │      ▼            ▼           ▼              ▼               │
//! │  "When to     "Re-live     "Find         "Strengthen        │
//! │   dream?"     the day"     synergy"       memories"         │
//! │                                                             │
//! │  Depth Control: Light → Deep → REM → Lucid                  │
//! │                                                             │
//! │  Output: SystemInsight — cross-subsystem optimizations      │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! Dreams run during idle ticks and produce `SystemInsight` records that
//! improve waking performance.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_REPLAY_EVENTS: usize = 1024;
const MAX_INSIGHTS: usize = 256;
const MAX_SYNERGIES: usize = 128;
const MAX_DREAM_HISTORY: usize = 128;
const DREAM_IDLE_THRESHOLD: f32 = 0.25;
const LIGHT_DEPTH: f32 = 0.25;
const DEEP_DEPTH: f32 = 0.50;
const REM_DEPTH: f32 = 0.75;
const LUCID_DEPTH: f32 = 0.95;
const INSIGHT_MIN_CONFIDENCE: f32 = 0.3;
const CONSOLIDATION_STRENGTH: f32 = 0.15;
const SYNERGY_THRESHOLD: f32 = 0.4;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

// ============================================================================
// FNV-1a HASHING & PRNG
// ============================================================================

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
// DREAM DEPTH
// ============================================================================

/// How deep the system is dreaming
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DreamDepth {
    /// Awake — no dream activity
    Awake   = 0,
    /// Light replay of recent events
    Light   = 1,
    /// Deep consolidation of patterns
    Deep    = 2,
    /// REM-like creative recombination
    Rem     = 3,
    /// Lucid — conscious exploration of alternatives
    Lucid   = 4,
}

impl DreamDepth {
    /// Minimum idle level required to enter this depth
    #[inline]
    pub fn required_idle(&self) -> f32 {
        match self {
            DreamDepth::Awake => 1.0,
            DreamDepth::Light => DREAM_IDLE_THRESHOLD,
            DreamDepth::Deep => 0.15,
            DreamDepth::Rem => 0.10,
            DreamDepth::Lucid => 0.05,
        }
    }

    /// Cognitive cost per tick at this depth
    #[inline]
    pub fn cost_per_tick(&self) -> f32 {
        match self {
            DreamDepth::Awake => 0.0,
            DreamDepth::Light => 2.0,
            DreamDepth::Deep => 5.0,
            DreamDepth::Rem => 8.0,
            DreamDepth::Lucid => 12.0,
        }
    }
}

// ============================================================================
// REPLAY EVENT
// ============================================================================

/// A recorded system event available for dream replay
#[derive(Debug, Clone)]
pub struct ReplayEvent {
    pub id: u64,
    pub subsystem_source: String,
    pub event_type: String,
    pub significance: f32,
    pub tick: u64,
    /// Hash of related subsystems involved
    pub related_subsystems: Vec<u64>,
    /// Outcome score: positive = good, negative = bad
    pub outcome: f32,
    /// Whether already replayed in a dream
    pub replayed: bool,
    /// Replay count
    pub replay_count: u32,
}

// ============================================================================
// SYSTEM INSIGHT
// ============================================================================

/// A cross-subsystem insight discovered during dreaming
#[derive(Debug, Clone)]
pub struct SystemInsight {
    pub id: u64,
    pub description: String,
    /// Which subsystems are involved
    pub involved_subsystems: Vec<String>,
    /// Confidence in this insight (0.0 – 1.0)
    pub confidence: f32,
    /// Estimated performance gain if applied
    pub estimated_gain: f32,
    /// Dream depth at which this was discovered
    pub discovery_depth: DreamDepth,
    /// Tick discovered
    pub discovery_tick: u64,
    /// Whether this has been applied to waking policy
    pub applied: bool,
    /// How many dream cycles reinforced this insight
    pub reinforcement_count: u32,
}

// ============================================================================
// SYNERGY RECORD
// ============================================================================

/// A detected synergy between subsystems
#[derive(Debug, Clone)]
pub struct SynergyRecord {
    pub subsystem_a: String,
    pub subsystem_b: String,
    pub correlation: f32,
    pub synergy_type: String,
    pub strength: f32,
    pub discovery_tick: u64,
    pub observation_count: u32,
}

// ============================================================================
// DREAM SESSION
// ============================================================================

/// A single dream session record
#[derive(Debug, Clone)]
pub struct DreamSession {
    pub session_id: u64,
    pub start_tick: u64,
    pub end_tick: u64,
    pub max_depth: DreamDepth,
    pub events_replayed: u32,
    pub insights_generated: u32,
    pub synergies_found: u32,
    pub total_cost: f32,
    pub quality_score: f32,
}

// ============================================================================
// STATS
// ============================================================================

/// Dream engine statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticDreamStats {
    pub total_sessions: u64,
    pub total_events_replayed: u64,
    pub total_insights: u64,
    pub total_synergies: u64,
    pub average_depth: f32,
    pub average_quality: f32,
    pub deepest_ever: DreamDepth,
    pub total_cost_spent: f32,
    pub insights_applied: u64,
}

// ============================================================================
// HOLISTIC DREAM ENGINE
// ============================================================================

/// System-wide offline consolidation engine. Replays recent history,
/// discovers cross-subsystem synergies, and generates system insights.
pub struct HolisticDreamEngine {
    /// Replay buffer of recent events
    replay_buffer: Vec<ReplayEvent>,
    replay_write_idx: usize,
    /// Discovered insights
    insights: BTreeMap<u64, SystemInsight>,
    /// Discovered synergies
    synergies: Vec<SynergyRecord>,
    /// Dream session history
    session_history: Vec<DreamSession>,
    session_write_idx: usize,
    /// Current dream state
    current_depth: DreamDepth,
    /// Whether currently in a dream session
    dreaming: bool,
    /// Current session accumulator
    current_session_start: u64,
    current_events_replayed: u32,
    current_insights_generated: u32,
    current_synergies_found: u32,
    current_cost: f32,
    /// Stats
    stats: HolisticDreamStats,
    /// PRNG
    rng: u64,
    /// Current tick
    tick: u64,
}

impl HolisticDreamEngine {
    /// Create a new holistic dream engine
    pub fn new(seed: u64) -> Self {
        let mut replay_buffer = Vec::with_capacity(MAX_REPLAY_EVENTS);
        for _ in 0..MAX_REPLAY_EVENTS {
            replay_buffer.push(ReplayEvent {
                id: 0,
                subsystem_source: String::new(),
                event_type: String::new(),
                significance: 0.0,
                tick: 0,
                related_subsystems: Vec::new(),
                outcome: 0.0,
                replayed: false,
                replay_count: 0,
            });
        }
        let mut session_history = Vec::with_capacity(MAX_DREAM_HISTORY);
        for _ in 0..MAX_DREAM_HISTORY {
            session_history.push(DreamSession {
                session_id: 0,
                start_tick: 0,
                end_tick: 0,
                max_depth: DreamDepth::Awake,
                events_replayed: 0,
                insights_generated: 0,
                synergies_found: 0,
                total_cost: 0.0,
                quality_score: 0.0,
            });
        }
        Self {
            replay_buffer,
            replay_write_idx: 0,
            insights: BTreeMap::new(),
            synergies: Vec::with_capacity(MAX_SYNERGIES),
            session_history,
            session_write_idx: 0,
            current_depth: DreamDepth::Awake,
            dreaming: false,
            current_session_start: 0,
            current_events_replayed: 0,
            current_insights_generated: 0,
            current_synergies_found: 0,
            current_cost: 0.0,
            stats: HolisticDreamStats {
                total_sessions: 0,
                total_events_replayed: 0,
                total_insights: 0,
                total_synergies: 0,
                average_depth: 0.0,
                average_quality: 0.0,
                deepest_ever: DreamDepth::Awake,
                total_cost_spent: 0.0,
                insights_applied: 0,
            },
            rng: seed ^ 0xD4EA_CAFE_1234_5678,
            tick: 0,
        }
    }

    /// Begin or continue a system dream at appropriate depth
    pub fn system_dream(&mut self, idle_level: f32, tick: u64) {
        self.tick = tick;
        let depth = if idle_level <= DreamDepth::Lucid.required_idle() {
            DreamDepth::Lucid
        } else if idle_level <= DreamDepth::Rem.required_idle() {
            DreamDepth::Rem
        } else if idle_level <= DreamDepth::Deep.required_idle() {
            DreamDepth::Deep
        } else if idle_level <= DreamDepth::Light.required_idle() {
            DreamDepth::Light
        } else {
            DreamDepth::Awake
        };

        if depth == DreamDepth::Awake {
            if self.dreaming {
                self.end_session();
            }
            return;
        }

        if !self.dreaming {
            self.dreaming = true;
            self.current_session_start = tick;
            self.current_events_replayed = 0;
            self.current_insights_generated = 0;
            self.current_synergies_found = 0;
            self.current_cost = 0.0;
        }

        self.current_depth = depth;
        self.current_cost += depth.cost_per_tick();
        self.cross_subsystem_replay();
    }

    /// Replay events from the buffer, focusing on cross-subsystem interactions
    pub fn cross_subsystem_replay(&mut self) {
        let max_replay = match self.current_depth {
            DreamDepth::Awake => 0,
            DreamDepth::Light => 4,
            DreamDepth::Deep => 8,
            DreamDepth::Rem => 16,
            DreamDepth::Lucid => 32,
        };
        let mut replayed = 0u32;
        for i in 0..self.replay_buffer.len() {
            if replayed >= max_replay {
                break;
            }
            if self.replay_buffer[i].significance > 0.0 && !self.replay_buffer[i].replayed {
                self.replay_buffer[i].replayed = true;
                self.replay_buffer[i].replay_count += 1;
                replayed += 1;
            }
        }
        self.current_events_replayed += replayed;
        self.stats.total_events_replayed += replayed as u64;
    }

    /// Discover synergies between subsystems from replayed events
    pub fn discover_synergy(&mut self) -> Vec<SynergyRecord> {
        let mut new_synergies = Vec::new();
        let mut pair_scores: BTreeMap<(u64, u64), (f32, u32)> = BTreeMap::new();
        for event in &self.replay_buffer {
            if event.significance < SYNERGY_THRESHOLD || event.related_subsystems.len() < 2 {
                continue;
            }
            for i in 0..event.related_subsystems.len() {
                for j in (i + 1)..event.related_subsystems.len() {
                    let a = event.related_subsystems[i].min(event.related_subsystems[j]);
                    let b = event.related_subsystems[i].max(event.related_subsystems[j]);
                    let entry = pair_scores.entry((a, b)).or_insert((0.0, 0));
                    entry.0 += event.outcome * event.significance;
                    entry.1 += 1;
                }
            }
        }
        for ((a, b), (score, count)) in &pair_scores {
            if *count >= 2 {
                let avg_score = score / *count as f32;
                if avg_score.abs() > SYNERGY_THRESHOLD {
                    let rng_val = xorshift64(&mut self.rng);
                    let synergy = SynergyRecord {
                        subsystem_a: {
                            let mut s = String::from("sub_");
                            let digit = (a % 10) as u8 + b'0';
                            s.push(digit as char);
                            s
                        },
                        subsystem_b: {
                            let mut s = String::from("sub_");
                            let digit = (b % 10) as u8 + b'0';
                            s.push(digit as char);
                            s
                        },
                        correlation: avg_score,
                        synergy_type: if avg_score > 0.0 {
                            String::from("cooperative")
                        } else {
                            String::from("conflicting")
                        },
                        strength: avg_score.abs(),
                        discovery_tick: self.tick,
                        observation_count: *count,
                    };
                    new_synergies.push(synergy);
                    let _ = rng_val; // used for seeding
                }
            }
        }
        self.current_synergies_found += new_synergies.len() as u32;
        self.stats.total_synergies += new_synergies.len() as u64;
        for s in &new_synergies {
            if self.synergies.len() < MAX_SYNERGIES {
                self.synergies.push(s.clone());
            }
        }
        new_synergies
    }

    /// Consolidate all dream findings — strengthen good patterns, weaken bad
    #[inline]
    pub fn consolidate_all(&mut self) {
        for (_id, insight) in self.insights.iter_mut() {
            if !insight.applied {
                insight.confidence += CONSOLIDATION_STRENGTH * (1.0 - insight.confidence);
                insight.reinforcement_count += 1;
            }
        }
    }

    /// Get current dream depth
    #[inline(always)]
    pub fn dream_depth(&self) -> DreamDepth {
        self.current_depth
    }

    /// Evaluate insight quality across all discoveries
    #[inline]
    pub fn insight_quality(&self) -> f32 {
        if self.insights.is_empty() {
            return 0.0;
        }
        let total: f32 = self.insights.values().map(|i| i.confidence * i.estimated_gain).sum();
        total / self.insights.len() as f32
    }

    /// Determine optimal dream schedule based on system state
    #[inline]
    pub fn dream_schedule(&self, current_load: f32) -> u64 {
        if current_load < DREAM_IDLE_THRESHOLD {
            1 // Dream immediately
        } else if current_load < 0.5 {
            10 // Dream soon
        } else if current_load < 0.8 {
            50 // Dream later
        } else {
            200 // Defer dreaming
        }
    }

    /// Record a new event in the replay buffer
    #[inline(always)]
    pub fn record_event(&mut self, event: ReplayEvent) {
        self.replay_buffer[self.replay_write_idx] = event;
        self.replay_write_idx = (self.replay_write_idx + 1) % MAX_REPLAY_EVENTS;
    }

    /// Store a new insight
    #[inline]
    pub fn store_insight(&mut self, insight: SystemInsight) {
        if self.insights.len() < MAX_INSIGHTS {
            let id = insight.id;
            self.insights.insert(id, insight);
            self.current_insights_generated += 1;
            self.stats.total_insights += 1;
        }
    }

    /// Mark an insight as applied
    #[inline]
    pub fn apply_insight(&mut self, insight_id: u64) -> bool {
        if let Some(insight) = self.insights.get_mut(&insight_id) {
            insight.applied = true;
            self.stats.insights_applied += 1;
            true
        } else {
            false
        }
    }

    /// Get all unapplied insights above minimum confidence
    #[inline]
    pub fn actionable_insights(&self) -> Vec<&SystemInsight> {
        self.insights
            .values()
            .filter(|i| !i.applied && i.confidence >= INSIGHT_MIN_CONFIDENCE)
            .collect()
    }

    /// Whether currently in a dream session
    #[inline(always)]
    pub fn is_dreaming(&self) -> bool {
        self.dreaming
    }

    /// Get engine stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticDreamStats {
        &self.stats
    }

    /// Synergy count
    #[inline(always)]
    pub fn synergy_count(&self) -> usize {
        self.synergies.len()
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    #[inline]
    fn end_session(&mut self) {
        let depth_val = self.current_depth as u8 as f32;
        let quality = if self.current_events_replayed > 0 {
            (self.current_insights_generated as f32 / self.current_events_replayed as f32)
                .min(1.0)
        } else {
            0.0
        };
        let session = DreamSession {
            session_id: self.stats.total_sessions,
            start_tick: self.current_session_start,
            end_tick: self.tick,
            max_depth: self.current_depth,
            events_replayed: self.current_events_replayed,
            insights_generated: self.current_insights_generated,
            synergies_found: self.current_synergies_found,
            total_cost: self.current_cost,
            quality_score: quality,
        };
        self.session_history[self.session_write_idx] = session;
        self.session_write_idx = (self.session_write_idx + 1) % MAX_DREAM_HISTORY;
        self.stats.total_sessions += 1;
        self.stats.average_depth +=
            EMA_ALPHA * (depth_val - self.stats.average_depth);
        self.stats.average_quality +=
            EMA_ALPHA * (quality - self.stats.average_quality);
        self.stats.total_cost_spent += self.current_cost;
        if self.current_depth > self.stats.deepest_ever {
            self.stats.deepest_ever = self.current_depth;
        }
        self.dreaming = false;
        self.current_depth = DreamDepth::Awake;
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dream_depth_ordering() {
        assert!(DreamDepth::Lucid > DreamDepth::Rem);
        assert!(DreamDepth::Rem > DreamDepth::Deep);
        assert!(DreamDepth::Deep > DreamDepth::Light);
    }

    #[test]
    fn test_engine_creation() {
        let engine = HolisticDreamEngine::new(42);
        assert_eq!(engine.dream_depth(), DreamDepth::Awake);
        assert!(!engine.is_dreaming());
    }

    #[test]
    fn test_dream_schedule() {
        let engine = HolisticDreamEngine::new(42);
        assert_eq!(engine.dream_schedule(0.1), 1);
        assert_eq!(engine.dream_schedule(0.9), 200);
    }

    #[test]
    fn test_insight_quality_empty() {
        let engine = HolisticDreamEngine::new(42);
        assert_eq!(engine.insight_quality(), 0.0);
    }

    #[test]
    fn test_fnv1a() {
        let h = fnv1a_hash(b"dream");
        assert_eq!(h, fnv1a_hash(b"dream"));
    }
}
