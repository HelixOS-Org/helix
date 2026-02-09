// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Memory Palace
//!
//! Structured cooperation knowledge organized by type. The memory palace
//! stores cooperation patterns in distinct "rooms" — sharing, contention,
//! mediation, and trust-building — enabling fast retrieval of relevant
//! knowledge when similar situations arise.
//!
//! Unlike flat caches, the palace maintains semantic organization: patterns
//! are filed by their cooperation category, cross-referenced by process
//! pairs, and ranked by relevance and recency. Obsolete patterns are
//! periodically forgotten to keep the palace manageable.
//!
//! ## Rooms
//!
//! - **Sharing** — Successful resource sharing patterns
//! - **Contention** — Resource conflict patterns and resolutions
//! - **Mediation** — Mediation strategies and their outcomes
//! - **TrustBuilding** — Patterns that built or eroded trust
//!
//! ## Key Methods
//!
//! - `store_cooperation_pattern()` — Store a pattern in the appropriate room
//! - `recall_mediation()` — Recall relevant mediation patterns
//! - `organize_by_type()` — Reorganize and re-rank patterns
//! - `forget_obsolete()` — Prune patterns that are no longer relevant
//! - `knowledge_richness()` — How rich is the palace's knowledge?
//! - `palace_architecture()` — Structural overview of the palace

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.10;
const MAX_PATTERNS_PER_ROOM: usize = 256;
const MAX_TOTAL_PATTERNS: usize = 1024;
const RELEVANCE_DECAY: f32 = 0.995;
const FORGET_THRESHOLD: f32 = 0.05;
const RICHNESS_SCALE: f32 = 0.001;
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

/// Xorshift64 PRNG for forget jitter
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// COOPERATION PATTERN TYPE
// ============================================================================

/// The room (category) a cooperation pattern belongs to
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PatternRoom {
    /// Successful resource sharing patterns
    Sharing,
    /// Resource conflict patterns and resolutions
    Contention,
    /// Mediation strategies and outcomes
    Mediation,
    /// Patterns that built or eroded trust
    TrustBuilding,
}

impl PatternRoom {
    pub fn all() -> &'static [PatternRoom] {
        &[
            PatternRoom::Sharing,
            PatternRoom::Contention,
            PatternRoom::Mediation,
            PatternRoom::TrustBuilding,
        ]
    }
}

// ============================================================================
// COOPERATION PATTERN
// ============================================================================

/// A single stored cooperation pattern
#[derive(Debug, Clone)]
pub struct CoopPattern {
    pub pattern_id: u64,
    pub name: String,
    pub room: PatternRoom,
    /// Processes involved
    pub process_ids: Vec<u64>,
    /// Relevance score (decays over time)
    pub relevance: f32,
    /// Quality of the outcome this pattern represents
    pub outcome_quality: f32,
    /// Number of times recalled
    pub recall_count: u64,
    /// Tick when stored
    pub stored_tick: u64,
    /// Tick of last recall
    pub last_recall_tick: u64,
    /// EMA-smoothed recall frequency
    pub recall_frequency: f32,
    /// Context hash for matching
    pub context_hash: u64,
    /// Description of the pattern
    pub description: String,
    /// Tags for cross-referencing
    pub tags: Vec<String>,
}

impl CoopPattern {
    pub fn new(name: String, room: PatternRoom, description: String, tick: u64) -> Self {
        let pattern_id = fnv1a_hash(name.as_bytes());
        let context_hash = fnv1a_hash(description.as_bytes());
        Self {
            pattern_id,
            name,
            room,
            process_ids: Vec::new(),
            relevance: 1.0,
            outcome_quality: 0.5,
            recall_count: 0,
            stored_tick: tick,
            last_recall_tick: tick,
            recall_frequency: 0.0,
            context_hash,
            description,
            tags: Vec::new(),
        }
    }

    /// Record a recall event, boosting relevance
    pub fn record_recall(&mut self, tick: u64) {
        self.recall_count += 1;
        self.last_recall_tick = tick;
        // Boost relevance on recall
        self.relevance = (self.relevance + 0.1).min(1.0);
        self.recall_frequency += EMA_ALPHA * (1.0 - self.recall_frequency);
    }

    /// Decay relevance over time
    pub fn decay_relevance(&mut self, rng: &mut u64) {
        let jitter = (xorshift64(rng) % 30) as f32 / 100_000.0;
        self.relevance *= RELEVANCE_DECAY - jitter;
        self.recall_frequency *= RELEVANCE_DECAY;
        if self.relevance < 0.001 {
            self.relevance = 0.0;
        }
    }

    /// Composite score combining relevance, quality, and recall frequency
    pub fn composite_score(&self) -> f32 {
        self.relevance * 0.4 + self.outcome_quality * 0.35 + self.recall_frequency * 0.25
    }
}

// ============================================================================
// PALACE ROOM
// ============================================================================

/// A room in the memory palace containing patterns of one type
#[derive(Debug, Clone)]
pub struct PalaceRoom {
    pub room_type: PatternRoom,
    pub patterns: BTreeMap<u64, CoopPattern>,
    pub total_stored: u64,
    pub total_recalled: u64,
    pub avg_relevance: f32,
    pub avg_quality: f32,
}

impl PalaceRoom {
    pub fn new(room_type: PatternRoom) -> Self {
        Self {
            room_type,
            patterns: BTreeMap::new(),
            total_stored: 0,
            total_recalled: 0,
            avg_relevance: 0.0,
            avg_quality: 0.0,
        }
    }

    /// Store a pattern, evicting the least relevant if full
    pub fn store(&mut self, pattern: CoopPattern) {
        if self.patterns.len() >= MAX_PATTERNS_PER_ROOM {
            self.evict_least_relevant();
        }
        self.patterns.insert(pattern.pattern_id, pattern);
        self.total_stored += 1;
        self.recompute_averages();
    }

    /// Find patterns matching a context hash
    pub fn find_by_context(&self, context_hash: u64) -> Vec<&CoopPattern> {
        self.patterns
            .values()
            .filter(|p| p.context_hash == context_hash)
            .collect()
    }

    /// Find patterns involving a specific process
    pub fn find_by_process(&self, process_id: u64) -> Vec<&CoopPattern> {
        self.patterns
            .values()
            .filter(|p| p.process_ids.contains(&process_id))
            .collect()
    }

    /// Get top N patterns by composite score
    pub fn top_patterns(&self, n: usize) -> Vec<&CoopPattern> {
        let mut sorted: Vec<&CoopPattern> = self.patterns.values().collect();
        sorted.sort_by(|a, b| {
            b.composite_score()
                .partial_cmp(&a.composite_score())
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        sorted.truncate(n);
        sorted
    }

    fn evict_least_relevant(&mut self) {
        let mut worst_id: Option<u64> = None;
        let mut worst_score = f32::MAX;
        for (id, p) in self.patterns.iter() {
            if p.composite_score() < worst_score {
                worst_score = p.composite_score();
                worst_id = Some(*id);
            }
        }
        if let Some(id) = worst_id {
            self.patterns.remove(&id);
        }
    }

    fn recompute_averages(&mut self) {
        let count = self.patterns.len();
        if count == 0 {
            self.avg_relevance = 0.0;
            self.avg_quality = 0.0;
            return;
        }
        let mut rel_sum = 0.0f32;
        let mut qual_sum = 0.0f32;
        for (_, p) in self.patterns.iter() {
            rel_sum += p.relevance;
            qual_sum += p.outcome_quality;
        }
        self.avg_relevance = rel_sum / count as f32;
        self.avg_quality = qual_sum / count as f32;
    }
}

// ============================================================================
// PALACE STATS
// ============================================================================

#[derive(Debug, Clone)]
pub struct CoopPalaceStats {
    pub total_patterns: usize,
    pub sharing_count: usize,
    pub contention_count: usize,
    pub mediation_count: usize,
    pub trust_building_count: usize,
    pub total_recalls: u64,
    pub total_stores: u64,
    pub total_forgotten: u64,
    pub knowledge_richness: f32,
    pub avg_relevance: f32,
}

impl CoopPalaceStats {
    pub fn new() -> Self {
        Self {
            total_patterns: 0,
            sharing_count: 0,
            contention_count: 0,
            mediation_count: 0,
            trust_building_count: 0,
            total_recalls: 0,
            total_stores: 0,
            total_forgotten: 0,
            knowledge_richness: 0.0,
            avg_relevance: 0.0,
        }
    }
}

// ============================================================================
// COOPERATION MEMORY PALACE
// ============================================================================

/// Structured cooperation knowledge organized by room type
pub struct CoopMemoryPalace {
    rooms: BTreeMap<u8, PalaceRoom>,
    pub stats: CoopPalaceStats,
    rng_state: u64,
    tick: u64,
    /// EMA-smoothed richness score
    richness_ema: f32,
}

impl CoopMemoryPalace {
    pub fn new(seed: u64) -> Self {
        let mut rooms = BTreeMap::new();
        for room_type in PatternRoom::all() {
            rooms.insert(*room_type as u8, PalaceRoom::new(*room_type));
        }
        Self {
            rooms,
            stats: CoopPalaceStats::new(),
            rng_state: seed | 1,
            tick: 0,
            richness_ema: 0.0,
        }
    }

    // ========================================================================
    // STORE COOPERATION PATTERN
    // ========================================================================

    /// Store a cooperation pattern in the appropriate room
    pub fn store_cooperation_pattern(
        &mut self,
        name: String,
        room: PatternRoom,
        description: String,
        process_ids: Vec<u64>,
        outcome_quality: f32,
        tags: Vec<String>,
    ) -> u64 {
        self.tick += 1;

        let mut pattern = CoopPattern::new(name, room, description, self.tick);
        pattern.process_ids = process_ids;
        pattern.outcome_quality = if outcome_quality < 0.0 {
            0.0
        } else if outcome_quality > 1.0 {
            1.0
        } else {
            outcome_quality
        };
        pattern.tags = tags;
        let id = pattern.pattern_id;

        if let Some(palace_room) = self.rooms.get_mut(&(room as u8)) {
            palace_room.store(pattern);
        }

        self.stats.total_stores += 1;
        self.update_stats();
        id
    }

    // ========================================================================
    // RECALL MEDIATION
    // ========================================================================

    /// Recall relevant mediation patterns for a given context
    pub fn recall_mediation(
        &mut self,
        context_description: &str,
        max_results: usize,
    ) -> Vec<CoopPattern> {
        self.tick += 1;
        let context_hash = fnv1a_hash(context_description.as_bytes());

        let mut results = Vec::new();

        if let Some(room) = self.rooms.get(&(PatternRoom::Mediation as u8)) {
            // First try exact context match
            let exact = room.find_by_context(context_hash);
            for p in exact.iter().take(max_results) {
                results.push((*p).clone());
            }

            // Fill remaining with top patterns
            if results.len() < max_results {
                let top = room.top_patterns(max_results - results.len());
                for p in top {
                    let already = results.iter().any(|r| r.pattern_id == p.pattern_id);
                    if !already {
                        results.push(p.clone());
                    }
                }
            }
        }

        // Record recall for found patterns
        let result_ids: Vec<u64> = results.iter().map(|r| r.pattern_id).collect();
        let tick = self.tick;
        if let Some(room) = self.rooms.get_mut(&(PatternRoom::Mediation as u8)) {
            for id in result_ids {
                if let Some(p) = room.patterns.get_mut(&id) {
                    p.record_recall(tick);
                }
            }
            room.total_recalled += results.len() as u64;
        }

        self.stats.total_recalls += results.len() as u64;
        results
    }

    /// Recall patterns for a specific process across all rooms
    pub fn recall_for_process(&self, process_id: u64) -> Vec<&CoopPattern> {
        let mut results = Vec::new();
        for (_, room) in self.rooms.iter() {
            let matches = room.find_by_process(process_id);
            results.extend(matches);
        }
        results
    }

    // ========================================================================
    // ORGANIZE BY TYPE
    // ========================================================================

    /// Reorganize and re-rank all patterns within each room
    pub fn organize_by_type(&mut self) {
        self.tick += 1;

        for (_, room) in self.rooms.iter_mut() {
            room.recompute_averages();
        }

        self.update_stats();
    }

    // ========================================================================
    // FORGET OBSOLETE
    // ========================================================================

    /// Prune patterns that are no longer relevant
    pub fn forget_obsolete(&mut self) -> usize {
        self.tick += 1;
        let mut forgotten = 0usize;
        let rng = &mut self.rng_state;

        for (_, room) in self.rooms.iter_mut() {
            // Decay all
            let ids: Vec<u64> = room.patterns.keys().copied().collect();
            for id in ids.iter() {
                if let Some(p) = room.patterns.get_mut(id) {
                    p.decay_relevance(rng);
                }
            }

            // Remove below threshold
            let to_forget: Vec<u64> = room
                .patterns
                .iter()
                .filter(|(_, p)| p.relevance < FORGET_THRESHOLD)
                .map(|(k, _)| *k)
                .collect();
            for id in to_forget {
                room.patterns.remove(&id);
                forgotten += 1;
            }

            room.recompute_averages();
        }

        self.stats.total_forgotten += forgotten as u64;
        self.update_stats();
        forgotten
    }

    // ========================================================================
    // KNOWLEDGE RICHNESS
    // ========================================================================

    /// How rich is the palace's cooperation knowledge?
    ///
    /// Considers pattern count, diversity across rooms, average quality,
    /// and recall frequency.
    pub fn knowledge_richness(&mut self) -> f32 {
        let total = self.total_pattern_count();
        if total == 0 {
            return 0.0;
        }

        // Diversity: how evenly distributed across rooms
        let mut room_counts = Vec::new();
        for (_, room) in self.rooms.iter() {
            room_counts.push(room.patterns.len() as f32);
        }
        let max_count = room_counts
            .iter()
            .cloned()
            .fold(0.0f32, |a, b| if b > a { b } else { a });
        let diversity = if max_count > 0.0 {
            let min_count = room_counts
                .iter()
                .cloned()
                .fold(f32::MAX, |a, b| if b < a { b } else { a });
            min_count / max_count
        } else {
            0.0
        };

        // Volume factor
        let volume = (total as f32 * RICHNESS_SCALE).min(1.0);

        // Average quality across all rooms
        let mut total_quality = 0.0f32;
        let mut room_count = 0usize;
        for (_, room) in self.rooms.iter() {
            if !room.patterns.is_empty() {
                total_quality += room.avg_quality;
                room_count += 1;
            }
        }
        let avg_quality = if room_count > 0 {
            total_quality / room_count as f32
        } else {
            0.0
        };

        let raw = volume * 0.3 + diversity * 0.3 + avg_quality * 0.4;
        let clamped = if raw < 0.0 {
            0.0
        } else if raw > 1.0 {
            1.0
        } else {
            raw
        };

        self.richness_ema += EMA_ALPHA * (clamped - self.richness_ema);
        self.stats.knowledge_richness = self.richness_ema;
        self.richness_ema
    }

    // ========================================================================
    // PALACE ARCHITECTURE
    // ========================================================================

    /// Structural overview of the memory palace
    pub fn palace_architecture(&self) -> Vec<(PatternRoom, usize, f32, f32)> {
        let mut arch = Vec::new();
        for room_type in PatternRoom::all() {
            if let Some(room) = self.rooms.get(&(*room_type as u8)) {
                arch.push((
                    *room_type,
                    room.patterns.len(),
                    room.avg_relevance,
                    room.avg_quality,
                ));
            }
        }
        arch
    }

    // ========================================================================
    // QUERIES & MAINTENANCE
    // ========================================================================

    pub fn total_pattern_count(&self) -> usize {
        self.rooms.values().map(|r| r.patterns.len()).sum()
    }

    pub fn room_pattern_count(&self, room: PatternRoom) -> usize {
        self.rooms
            .get(&(room as u8))
            .map(|r| r.patterns.len())
            .unwrap_or(0)
    }

    pub fn snapshot_stats(&self) -> CoopPalaceStats {
        self.stats.clone()
    }

    fn update_stats(&mut self) {
        self.stats.total_patterns = self.total_pattern_count();
        self.stats.sharing_count = self.room_pattern_count(PatternRoom::Sharing);
        self.stats.contention_count = self.room_pattern_count(PatternRoom::Contention);
        self.stats.mediation_count = self.room_pattern_count(PatternRoom::Mediation);
        self.stats.trust_building_count = self.room_pattern_count(PatternRoom::TrustBuilding);

        let mut total_rel = 0.0f32;
        let mut count = 0usize;
        for (_, room) in self.rooms.iter() {
            for (_, p) in room.patterns.iter() {
                total_rel += p.relevance;
                count += 1;
            }
        }
        self.stats.avg_relevance = if count > 0 {
            total_rel / count as f32
        } else {
            0.0
        };
    }
}
