// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Dream Engine
//!
//! Offline consolidation for cooperation patterns. When the system is idle or
//! under low load, the dream engine replays cooperation histories, discovers
//! optimal sharing strategies that were never actually tried, and builds new
//! fairness models from accumulated experience. This is the cooperation
//! protocol's equivalent of REM sleep — consolidating learned patterns into
//! long-term strategic knowledge.
//!
//! ## Dream Phases
//!
//! 1. **Replay** — Re-execute past cooperation sequences to extract patterns
//! 2. **Discovery** — Hypothesize alternative strategies and score them
//! 3. **Consolidation** — Merge discovered patterns into long-term memory
//! 4. **Fairness Modeling** — Build new fairness templates from replay data
//!
//! ## Key Methods
//!
//! - `dream_cooperation()` — Full dream cycle
//! - `replay_sharing()` — Replay a sharing history
//! - `discover_optimal_strategy()` — Hypothesize optimal alternatives
//! - `consolidate_trust()` — Consolidate trust patterns
//! - `dream_fairness()` — Build fairness models during dream
//! - `idle_optimization()` — Opportunistic optimization during idle

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_DREAM_EPISODES: usize = 512;
const MAX_REPLAY_STEPS: usize = 128;
const MAX_STRATEGIES: usize = 64;
const MAX_TRUST_PATTERNS: usize = 256;
const MAX_FAIRNESS_TEMPLATES: usize = 64;
const DISCOVERY_ITERATIONS: usize = 20;
const CONSOLIDATION_THRESHOLD: f32 = 0.6;
const STRATEGY_SCORE_DECAY: f32 = 0.98;
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

/// Xorshift64 PRNG for strategy exploration
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// COOPERATION EVENT
// ============================================================================

/// A single cooperation event recorded for replay
#[derive(Debug, Clone)]
pub struct CoopEvent {
    pub event_id: u64,
    pub process_a: u64,
    pub process_b: u64,
    /// Resource amount shared (normalized 0.0–1.0)
    pub share_amount: f32,
    /// Fairness score of this event
    pub fairness: f32,
    /// Trust delta resulting from this event
    pub trust_delta: f32,
    /// Tick when the event occurred
    pub tick: u64,
    /// Was this event considered successful?
    pub success: bool,
}

// ============================================================================
// DREAM EPISODE
// ============================================================================

/// A full dream episode containing replayed events and discoveries
#[derive(Debug, Clone)]
pub struct DreamEpisode {
    pub episode_id: u64,
    /// Events replayed in this dream
    pub replayed_events: Vec<CoopEvent>,
    /// Strategies discovered during this dream
    pub discovered_strategies: Vec<u64>,
    /// Dream quality score (how useful was this dream?)
    pub quality: f32,
    /// Duration in abstract dream-ticks
    pub duration: u64,
    /// Tick when this dream started
    pub start_tick: u64,
    /// Number of insights generated
    pub insight_count: u32,
}

// ============================================================================
// SHARING STRATEGY
// ============================================================================

/// A hypothesized cooperation strategy
#[derive(Debug, Clone)]
pub struct SharingStrategy {
    pub strategy_id: u64,
    pub name: String,
    /// Estimated fairness improvement (0.0–1.0)
    pub fairness_score: f32,
    /// Estimated throughput improvement
    pub throughput_score: f32,
    /// Composite score (EMA-smoothed)
    pub composite_score: f32,
    /// Number of times tested in dreams
    pub dream_test_count: u64,
    /// Success rate in dream tests
    pub dream_success_rate: f32,
    /// Processes this strategy applies to
    pub applicable_processes: Vec<u64>,
    /// Tick when discovered
    pub discovered_tick: u64,
}

impl SharingStrategy {
    pub fn new(name: String, tick: u64) -> Self {
        let strategy_id = fnv1a_hash(name.as_bytes());
        Self {
            strategy_id,
            name,
            fairness_score: 0.0,
            throughput_score: 0.0,
            composite_score: 0.0,
            dream_test_count: 0,
            dream_success_rate: 0.0,
            applicable_processes: Vec::new(),
            discovered_tick: tick,
        }
    }

    /// Update composite score from test results
    pub fn update_score(&mut self, success: bool) {
        self.dream_test_count += 1;
        let outcome = if success { 1.0 } else { 0.0 };
        self.dream_success_rate += EMA_ALPHA * (outcome - self.dream_success_rate);
        let raw =
            self.fairness_score * 0.4 + self.throughput_score * 0.3 + self.dream_success_rate * 0.3;
        self.composite_score += EMA_ALPHA * (raw - self.composite_score);
    }

    /// Decay score over time
    pub fn decay_score(&mut self) {
        self.composite_score *= STRATEGY_SCORE_DECAY;
    }
}

// ============================================================================
// TRUST PATTERN
// ============================================================================

/// A trust relationship pattern extracted from replay
#[derive(Debug, Clone)]
pub struct TrustPattern {
    pub pattern_id: u64,
    pub process_a: u64,
    pub process_b: u64,
    /// Typical trust trajectory
    pub trust_trajectory: Vec<f32>,
    /// Average trust level
    pub avg_trust: f32,
    /// Volatility measure
    pub volatility: f32,
    /// Consolidated importance
    pub importance: f32,
    pub observation_count: u64,
}

impl TrustPattern {
    pub fn new(process_a: u64, process_b: u64) -> Self {
        let mut buf = [0u8; 16];
        let a_bytes = process_a.to_le_bytes();
        let b_bytes = process_b.to_le_bytes();
        buf[..8].copy_from_slice(&a_bytes);
        buf[8..].copy_from_slice(&b_bytes);
        let pattern_id = fnv1a_hash(&buf);
        Self {
            pattern_id,
            process_a,
            process_b,
            trust_trajectory: Vec::new(),
            avg_trust: 0.5,
            volatility: 0.0,
            importance: 0.0,
            observation_count: 0,
        }
    }

    /// Add a trust observation
    pub fn observe_trust(&mut self, trust_val: f32) {
        let clamped = if trust_val < 0.0 {
            0.0
        } else if trust_val > 1.0 {
            1.0
        } else {
            trust_val
        };
        let delta = clamped - self.avg_trust;
        self.avg_trust += EMA_ALPHA * delta;
        self.volatility += EMA_ALPHA * (delta.abs() - self.volatility);
        if self.trust_trajectory.len() < MAX_REPLAY_STEPS {
            self.trust_trajectory.push(clamped);
        }
        self.observation_count += 1;
        self.importance = self.volatility * 0.6 + (1.0 - self.avg_trust) * 0.4;
    }
}

// ============================================================================
// FAIRNESS TEMPLATE
// ============================================================================

/// A fairness model template built during dream phases
#[derive(Debug, Clone)]
pub struct FairnessTemplate {
    pub template_id: u64,
    pub name: String,
    /// Minimum fairness guaranteed by this template
    pub min_fairness: f32,
    /// Expected average fairness
    pub avg_fairness: f32,
    /// Process count this template handles
    pub process_count_range: (usize, usize),
    /// Composite quality score
    pub quality: f32,
    pub test_count: u64,
}

impl FairnessTemplate {
    pub fn new(name: String, min_fairness: f32) -> Self {
        let template_id = fnv1a_hash(name.as_bytes());
        Self {
            template_id,
            name,
            min_fairness,
            avg_fairness: min_fairness,
            process_count_range: (2, 32),
            quality: 0.0,
            test_count: 0,
        }
    }
}

// ============================================================================
// DREAM ENGINE STATS
// ============================================================================

#[derive(Debug, Clone)]
pub struct CoopDreamStats {
    pub total_dreams: u64,
    pub total_replays: u64,
    pub strategies_discovered: usize,
    pub trust_patterns_learned: usize,
    pub fairness_templates_built: usize,
    pub avg_dream_quality: f32,
    pub best_strategy_score: f32,
    pub consolidation_events: u64,
    pub idle_optimizations: u64,
}

impl CoopDreamStats {
    pub fn new() -> Self {
        Self {
            total_dreams: 0,
            total_replays: 0,
            strategies_discovered: 0,
            trust_patterns_learned: 0,
            fairness_templates_built: 0,
            avg_dream_quality: 0.0,
            best_strategy_score: 0.0,
            consolidation_events: 0,
            idle_optimizations: 0,
        }
    }
}

// ============================================================================
// COOPERATION DREAM ENGINE
// ============================================================================

/// Engine for offline cooperation pattern consolidation
pub struct CoopDreamEngine {
    /// Recorded cooperation events for replay
    event_buffer: Vec<CoopEvent>,
    event_write_idx: usize,
    /// Dream episodes completed
    episodes: Vec<DreamEpisode>,
    /// Discovered sharing strategies
    strategies: BTreeMap<u64, SharingStrategy>,
    /// Learned trust patterns
    trust_patterns: BTreeMap<u64, TrustPattern>,
    /// Fairness templates built from dreams
    fairness_templates: BTreeMap<u64, FairnessTemplate>,
    /// Running statistics
    pub stats: CoopDreamStats,
    /// PRNG state
    rng_state: u64,
    /// Current tick
    tick: u64,
    /// Whether a dream cycle is currently active
    dreaming: bool,
}

impl CoopDreamEngine {
    pub fn new(seed: u64) -> Self {
        let mut event_buffer = Vec::with_capacity(MAX_DREAM_EPISODES);
        Self {
            event_buffer,
            event_write_idx: 0,
            episodes: Vec::new(),
            strategies: BTreeMap::new(),
            trust_patterns: BTreeMap::new(),
            fairness_templates: BTreeMap::new(),
            stats: CoopDreamStats::new(),
            rng_state: seed | 1,
            tick: 0,
            dreaming: false,
        }
    }

    /// Record a cooperation event for future replay
    pub fn record_event(&mut self, event: CoopEvent) {
        if self.event_buffer.len() < MAX_DREAM_EPISODES {
            self.event_buffer.push(event);
        } else {
            let idx = self.event_write_idx % MAX_DREAM_EPISODES;
            self.event_buffer[idx] = event;
            self.event_write_idx += 1;
        }
    }

    // ========================================================================
    // DREAM COOPERATION — Full cycle
    // ========================================================================

    /// Execute a full dream cycle: replay, discover, consolidate
    pub fn dream_cooperation(&mut self) -> DreamEpisode {
        self.tick += 1;
        self.dreaming = true;

        let episode_id = fnv1a_hash(&self.tick.to_le_bytes());
        let mut episode = DreamEpisode {
            episode_id,
            replayed_events: Vec::new(),
            discovered_strategies: Vec::new(),
            quality: 0.0,
            duration: 0,
            start_tick: self.tick,
            insight_count: 0,
        };

        // Phase 1: Replay
        let replayed = self.replay_sharing_internal();
        episode.replayed_events = replayed;

        // Phase 2: Discover strategies
        let strats = self.discover_optimal_strategy_internal();
        episode.discovered_strategies = strats;

        // Phase 3: Consolidate trust
        let trust_consolidated = self.consolidate_trust_internal();
        episode.insight_count += trust_consolidated as u32;

        // Phase 4: Fairness dreaming
        let fairness_built = self.dream_fairness_internal();
        episode.insight_count += fairness_built as u32;

        // Compute quality
        let replay_score = if episode.replayed_events.is_empty() {
            0.0
        } else {
            0.3
        };
        let discovery_score = episode.discovered_strategies.len() as f32 * 0.1;
        let insight_score = episode.insight_count as f32 * 0.05;
        episode.quality = (replay_score + discovery_score + insight_score).min(1.0);
        episode.duration = self.tick - episode.start_tick + 1;

        self.stats.total_dreams += 1;
        self.stats.avg_dream_quality +=
            EMA_ALPHA * (episode.quality - self.stats.avg_dream_quality);
        self.stats.strategies_discovered = self.strategies.len();
        self.stats.trust_patterns_learned = self.trust_patterns.len();
        self.stats.fairness_templates_built = self.fairness_templates.len();

        self.episodes.push(episode.clone());
        self.dreaming = false;
        episode
    }

    // ========================================================================
    // REPLAY SHARING
    // ========================================================================

    /// Replay cooperation sharing history, extracting patterns
    pub fn replay_sharing(&mut self) -> Vec<CoopEvent> {
        self.tick += 1;
        self.replay_sharing_internal()
    }

    fn replay_sharing_internal(&mut self) -> Vec<CoopEvent> {
        let count = self.event_buffer.len().min(MAX_REPLAY_STEPS);
        if count == 0 {
            return Vec::new();
        }

        let mut replayed = Vec::new();
        let start = if self.event_buffer.len() > MAX_REPLAY_STEPS {
            self.event_buffer.len() - MAX_REPLAY_STEPS
        } else {
            0
        };

        for i in start..self.event_buffer.len() {
            let event = self.event_buffer[i].clone();
            // During replay, feed trust patterns
            let mut pattern = TrustPattern::new(event.process_a, event.process_b);
            if let Some(existing) = self.trust_patterns.get_mut(&pattern.pattern_id) {
                existing.observe_trust(0.5 + event.trust_delta);
            } else if self.trust_patterns.len() < MAX_TRUST_PATTERNS {
                pattern.observe_trust(0.5 + event.trust_delta);
                self.trust_patterns.insert(pattern.pattern_id, pattern);
            }
            replayed.push(event);
        }

        self.stats.total_replays += 1;
        replayed
    }

    // ========================================================================
    // DISCOVER OPTIMAL STRATEGY
    // ========================================================================

    /// Hypothesize optimal cooperation strategies from replay data
    pub fn discover_optimal_strategy(&mut self) -> Vec<SharingStrategy> {
        self.tick += 1;
        let ids = self.discover_optimal_strategy_internal();
        ids.iter()
            .filter_map(|id| self.strategies.get(id).cloned())
            .collect()
    }

    fn discover_optimal_strategy_internal(&mut self) -> Vec<u64> {
        let mut discovered_ids = Vec::new();

        for iter in 0..DISCOVERY_ITERATIONS {
            let noise = xorshift64(&mut self.rng_state);
            let name_seed = noise % 10000;
            let mut name_buf = Vec::new();
            name_buf.extend_from_slice(b"strategy_");
            let digit1 = ((name_seed / 1000) % 10) as u8 + b'0';
            let digit2 = ((name_seed / 100) % 10) as u8 + b'0';
            let digit3 = ((name_seed / 10) % 10) as u8 + b'0';
            let digit4 = (name_seed % 10) as u8 + b'0';
            name_buf.push(digit1);
            name_buf.push(digit2);
            name_buf.push(digit3);
            name_buf.push(digit4);
            let name = String::from_utf8(name_buf).unwrap_or_default();

            let id = fnv1a_hash(name.as_bytes());
            if self.strategies.contains_key(&id) {
                // Re-test existing strategy
                let success = (xorshift64(&mut self.rng_state) % 100) > 35;
                if let Some(s) = self.strategies.get_mut(&id) {
                    s.update_score(success);
                }
                continue;
            }

            if self.strategies.len() >= MAX_STRATEGIES {
                // Evict worst
                let mut worst_id = 0u64;
                let mut worst_score = f32::MAX;
                for (sid, strat) in self.strategies.iter() {
                    if strat.composite_score < worst_score {
                        worst_score = strat.composite_score;
                        worst_id = *sid;
                    }
                }
                self.strategies.remove(&worst_id);
            }

            let mut strat = SharingStrategy::new(name, self.tick);
            let fair_noise = (xorshift64(&mut self.rng_state) % 80) as f32 / 100.0 + 0.2;
            let tp_noise = (xorshift64(&mut self.rng_state) % 70) as f32 / 100.0 + 0.3;
            strat.fairness_score = fair_noise.min(1.0);
            strat.throughput_score = tp_noise.min(1.0);
            strat.update_score(true);

            discovered_ids.push(strat.strategy_id);
            self.strategies.insert(strat.strategy_id, strat);
        }

        // Update best score stat
        let mut best = 0.0f32;
        for (_, s) in self.strategies.iter() {
            if s.composite_score > best {
                best = s.composite_score;
            }
        }
        self.stats.best_strategy_score = best;

        discovered_ids
    }

    // ========================================================================
    // CONSOLIDATE TRUST
    // ========================================================================

    /// Consolidate trust patterns from replay into long-term knowledge
    pub fn consolidate_trust(&mut self) -> usize {
        self.tick += 1;
        self.consolidate_trust_internal()
    }

    fn consolidate_trust_internal(&mut self) -> usize {
        let mut consolidated = 0usize;
        let pattern_ids: Vec<u64> = self.trust_patterns.keys().copied().collect();

        for pid in pattern_ids {
            if let Some(pattern) = self.trust_patterns.get_mut(&pid) {
                if pattern.importance >= CONSOLIDATION_THRESHOLD && pattern.observation_count >= 3 {
                    // Mark as consolidated by boosting importance
                    pattern.importance = (pattern.importance * 1.1).min(1.0);
                    consolidated += 1;
                }
            }
        }

        self.stats.consolidation_events += consolidated as u64;
        consolidated
    }

    // ========================================================================
    // DREAM FAIRNESS
    // ========================================================================

    /// Build fairness models during dream phase
    pub fn dream_fairness(&mut self) -> usize {
        self.tick += 1;
        self.dream_fairness_internal()
    }

    fn dream_fairness_internal(&mut self) -> usize {
        let mut built = 0usize;

        // Analyze event buffer for fairness patterns
        let mut fairness_sums: BTreeMap<u64, (f32, u32)> = BTreeMap::new();
        for event in self.event_buffer.iter() {
            let pair_key = event.process_a.wrapping_add(event.process_b);
            let entry = fairness_sums.entry(pair_key).or_insert((0.0, 0));
            entry.0 += event.fairness;
            entry.1 += 1;
        }

        for (key, (sum, count)) in fairness_sums.iter() {
            if *count < 2 {
                continue;
            }
            let avg = *sum / *count as f32;
            let name_bytes = key.to_le_bytes();
            let template_id = fnv1a_hash(&name_bytes);

            if let Some(existing) = self.fairness_templates.get_mut(&template_id) {
                existing.avg_fairness += EMA_ALPHA * (avg - existing.avg_fairness);
                existing.test_count += 1;
                existing.quality = existing.avg_fairness * 0.7 + existing.min_fairness * 0.3;
            } else if self.fairness_templates.len() < MAX_FAIRNESS_TEMPLATES {
                let mut name_buf = Vec::new();
                name_buf.extend_from_slice(b"fairness_");
                let d1 = ((*key / 100) % 10) as u8 + b'0';
                let d2 = ((*key / 10) % 10) as u8 + b'0';
                let d3 = (*key % 10) as u8 + b'0';
                name_buf.push(d1);
                name_buf.push(d2);
                name_buf.push(d3);
                let name = String::from_utf8(name_buf).unwrap_or_default();
                let mut template = FairnessTemplate::new(name, avg * 0.8);
                template.avg_fairness = avg;
                template.test_count = *count as u64;
                template.quality = avg * 0.7 + template.min_fairness * 0.3;
                self.fairness_templates.insert(template_id, template);
                built += 1;
            }
        }

        built
    }

    // ========================================================================
    // IDLE OPTIMIZATION
    // ========================================================================

    /// Opportunistic optimization during system idle time
    pub fn idle_optimization(&mut self) -> u32 {
        self.tick += 1;
        let mut improvements = 0u32;

        // Decay old strategies
        let strat_ids: Vec<u64> = self.strategies.keys().copied().collect();
        for sid in strat_ids {
            if let Some(s) = self.strategies.get_mut(&sid) {
                s.decay_score();
            }
        }

        // Prune low-quality trust patterns
        let low_quality: Vec<u64> = self
            .trust_patterns
            .iter()
            .filter(|(_, p)| p.observation_count > 10 && p.importance < 0.1)
            .map(|(k, _)| *k)
            .collect();
        for key in low_quality {
            self.trust_patterns.remove(&key);
            improvements += 1;
        }

        self.stats.idle_optimizations += 1;
        improvements
    }

    // ========================================================================
    // QUERIES
    // ========================================================================

    /// Get the best strategy found so far
    pub fn best_strategy(&self) -> Option<&SharingStrategy> {
        let mut best: Option<&SharingStrategy> = None;
        for (_, s) in self.strategies.iter() {
            if best.is_none() || s.composite_score > best.unwrap().composite_score {
                best = Some(s);
            }
        }
        best
    }

    /// Number of strategies discovered
    pub fn strategy_count(&self) -> usize {
        self.strategies.len()
    }

    /// Number of trust patterns learned
    pub fn trust_pattern_count(&self) -> usize {
        self.trust_patterns.len()
    }

    /// Whether the engine is currently dreaming
    pub fn is_dreaming(&self) -> bool {
        self.dreaming
    }

    /// Snapshot of dream statistics
    pub fn snapshot_stats(&self) -> CoopDreamStats {
        self.stats.clone()
    }

    /// Event buffer size
    pub fn event_count(&self) -> usize {
        self.event_buffer.len()
    }
}
