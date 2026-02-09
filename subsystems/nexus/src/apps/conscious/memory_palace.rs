// SPDX-License-Identifier: GPL-2.0
//! # Apps Memory Palace
//!
//! Structured knowledge store for application understanding. The memory palace
//! organizes accumulated knowledge about application behavior into logical
//! "rooms" — categorical groupings such as compute-bound, I/O-bound,
//! memory-hungry, and network-heavy applications.
//!
//! Each room contains app profiles and discovered behavioral patterns. Knowledge
//! is reinforced through a **spaced repetition** mechanism: important patterns
//! that are frequently validated get longer retention intervals, while stale
//! or invalidated knowledge is gradually forgotten.
//!
//! The palace supports:
//! - **Storage** — Insert new knowledge with category and confidence
//! - **Recall** — Retrieve relevant patterns for a given behavioral fingerprint
//! - **Organization** — Periodically re-sort knowledge across rooms
//! - **Forgetting** — Evict stale knowledge based on spaced repetition schedule
//! - **Depth** — Measure how deep knowledge is in each category

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.11;
const MAX_ROOMS: usize = 32;
const MAX_PROFILES_PER_ROOM: usize = 256;
const MAX_PATTERNS_PER_ROOM: usize = 128;
const INITIAL_RETENTION_INTERVAL: u64 = 100;
const RETENTION_GROWTH_FACTOR: u64 = 2;
const MAX_RETENTION_INTERVAL: u64 = 50000;
const STALE_FACTOR: u64 = 3;
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

/// Xorshift64 PRNG for stochastic forgetting
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

// ============================================================================
// ROOM CATEGORY
// ============================================================================

/// Categories for organizing application knowledge
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoomCategory {
    ComputeBound,
    IoBound,
    MemoryHungry,
    NetworkHeavy,
    Balanced,
    Interactive,
    Batch,
    Latency,
}

impl RoomCategory {
    pub fn label(&self) -> &'static str {
        match self {
            RoomCategory::ComputeBound => "compute_bound",
            RoomCategory::IoBound => "io_bound",
            RoomCategory::MemoryHungry => "memory_hungry",
            RoomCategory::NetworkHeavy => "network_heavy",
            RoomCategory::Balanced => "balanced",
            RoomCategory::Interactive => "interactive",
            RoomCategory::Batch => "batch",
            RoomCategory::Latency => "latency_sensitive",
        }
    }

    fn all() -> &'static [RoomCategory] {
        &[
            RoomCategory::ComputeBound,
            RoomCategory::IoBound,
            RoomCategory::MemoryHungry,
            RoomCategory::NetworkHeavy,
            RoomCategory::Balanced,
            RoomCategory::Interactive,
            RoomCategory::Batch,
            RoomCategory::Latency,
        ]
    }

    /// Classify based on resource profile
    pub fn classify(cpu: f32, mem: f32, io: f32, net: f32) -> Self {
        let vals = [
            (cpu, RoomCategory::ComputeBound),
            (io, RoomCategory::IoBound),
            (mem, RoomCategory::MemoryHungry),
            (net, RoomCategory::NetworkHeavy),
        ];

        let mut best = vals[0];
        for v in &vals[1..] {
            if v.0 > best.0 {
                best = *v;
            }
        }

        // If no dimension dominates, classify as balanced
        if best.0 < 0.4 {
            RoomCategory::Balanced
        } else {
            best.1
        }
    }
}

// ============================================================================
// APP KNOWLEDGE PROFILE
// ============================================================================

/// Stored knowledge about a specific app
#[derive(Debug, Clone)]
pub struct AppKnowledgeProfile {
    pub app_id: u64,
    pub app_name: String,
    pub category: RoomCategory,
    pub confidence: f32,
    pub cpu_profile: f32,
    pub mem_profile: f32,
    pub io_profile: f32,
    pub net_profile: f32,
    /// Spaced repetition: current retention interval
    pub retention_interval: u64,
    /// Last tick this knowledge was validated
    pub last_validated_tick: u64,
    /// Number of successful validations
    pub validation_count: u64,
    /// Is this knowledge considered stale?
    pub stale: bool,
}

impl AppKnowledgeProfile {
    fn new(
        app_id: u64,
        app_name: String,
        category: RoomCategory,
        cpu: f32,
        mem: f32,
        io: f32,
        net: f32,
        tick: u64,
    ) -> Self {
        Self {
            app_id,
            app_name,
            category,
            confidence: 0.5,
            cpu_profile: cpu,
            mem_profile: mem,
            io_profile: io,
            net_profile: net,
            retention_interval: INITIAL_RETENTION_INTERVAL,
            last_validated_tick: tick,
            validation_count: 0,
            stale: false,
        }
    }

    fn validate(&mut self, tick: u64) {
        self.validation_count += 1;
        self.last_validated_tick = tick;
        self.confidence = (self.confidence + 0.05).min(1.0);
        // Spaced repetition: increase interval on successful validation
        self.retention_interval =
            (self.retention_interval * RETENTION_GROWTH_FACTOR).min(MAX_RETENTION_INTERVAL);
        self.stale = false;
    }

    fn check_stale(&mut self, tick: u64) {
        let elapsed = tick.saturating_sub(self.last_validated_tick);
        if elapsed > self.retention_interval * STALE_FACTOR {
            self.stale = true;
            self.confidence *= 0.95;
        }
    }

    #[inline]
    fn update_profile(&mut self, cpu: f32, mem: f32, io: f32, net: f32) {
        self.cpu_profile = EMA_ALPHA * cpu + (1.0 - EMA_ALPHA) * self.cpu_profile;
        self.mem_profile = EMA_ALPHA * mem + (1.0 - EMA_ALPHA) * self.mem_profile;
        self.io_profile = EMA_ALPHA * io + (1.0 - EMA_ALPHA) * self.io_profile;
        self.net_profile = EMA_ALPHA * net + (1.0 - EMA_ALPHA) * self.net_profile;
    }
}

/// A discovered behavioral pattern stored in a room
#[derive(Debug, Clone)]
pub struct KnowledgePattern {
    pub pattern_hash: u64,
    pub description: String,
    pub confidence: f32,
    pub app_ids: Vec<u64>,
    pub retention_interval: u64,
    pub last_validated_tick: u64,
    pub validation_count: u64,
}

impl KnowledgePattern {
    fn new(description: String, app_ids: Vec<u64>, tick: u64) -> Self {
        let pattern_hash = fnv1a_hash(description.as_bytes());
        Self {
            pattern_hash,
            description,
            confidence: 0.5,
            app_ids,
            retention_interval: INITIAL_RETENTION_INTERVAL,
            last_validated_tick: tick,
            validation_count: 0,
        }
    }

    fn validate(&mut self, tick: u64) {
        self.validation_count += 1;
        self.last_validated_tick = tick;
        self.confidence = (self.confidence + 0.04).min(1.0);
        self.retention_interval =
            (self.retention_interval * RETENTION_GROWTH_FACTOR).min(MAX_RETENTION_INTERVAL);
    }

    fn is_stale(&self, tick: u64) -> bool {
        tick.saturating_sub(self.last_validated_tick) > self.retention_interval * STALE_FACTOR
    }
}

// ============================================================================
// MEMORY ROOM
// ============================================================================

/// A room in the memory palace — a categorical knowledge store
#[derive(Debug, Clone)]
pub struct MemoryRoom {
    pub category: RoomCategory,
    pub app_profiles: BTreeMap<u64, AppKnowledgeProfile>,
    pub patterns: Vec<KnowledgePattern>,
    pub depth: f32,
    pub last_organized_tick: u64,
}

impl MemoryRoom {
    fn new(category: RoomCategory) -> Self {
        Self {
            category,
            app_profiles: BTreeMap::new(),
            patterns: Vec::new(),
            depth: 0.0,
            last_organized_tick: 0,
        }
    }

    fn recompute_depth(&mut self) {
        let profile_depth = if self.app_profiles.is_empty() {
            0.0
        } else {
            let conf_sum: f32 = self.app_profiles.values().map(|p| p.confidence).sum();
            conf_sum / self.app_profiles.len() as f32
        };

        let pattern_depth = if self.patterns.is_empty() {
            0.0
        } else {
            let conf_sum: f32 = self.patterns.iter().map(|p| p.confidence).sum();
            conf_sum / self.patterns.len() as f32
        };

        let profile_weight = self.app_profiles.len().min(50) as f32 / 50.0;
        let pattern_weight = self.patterns.len().min(20) as f32 / 20.0;

        self.depth = 0.5 * (profile_depth * profile_weight)
            + 0.5 * (pattern_depth * pattern_weight);
    }

    fn evict_stale_profiles(&mut self, tick: u64) -> usize {
        let stale_ids: Vec<u64> = self
            .app_profiles
            .iter()
            .filter(|(_, p)| {
                p.stale && p.confidence < 0.2
            })
            .map(|(id, _)| *id)
            .collect();
        let count = stale_ids.len();
        for id in stale_ids {
            self.app_profiles.remove(&id);
        }
        let _ = tick;
        count
    }

    fn evict_stale_patterns(&mut self, tick: u64) -> usize {
        let before = self.patterns.len();
        self.patterns.retain(|p| !p.is_stale(tick) || p.confidence > 0.3);
        before - self.patterns.len()
    }
}

// ============================================================================
// STATS
// ============================================================================

/// Aggregate memory palace statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PalaceStats {
    pub total_rooms: usize,
    pub total_profiles: usize,
    pub total_patterns: usize,
    pub mean_depth: f32,
    pub deepest_room: RoomCategory,
    pub stale_profile_count: usize,
    pub total_validations: u64,
}

// ============================================================================
// APPS MEMORY PALACE
// ============================================================================

/// Structured knowledge store for application understanding
#[derive(Debug)]
pub struct AppsMemoryPalace {
    rooms: BTreeMap<u8, MemoryRoom>,
    tick: u64,
    total_stores: u64,
    total_recalls: u64,
    total_validations: u64,
    rng_state: u64,
}

impl AppsMemoryPalace {
    pub fn new(seed: u64) -> Self {
        let mut rooms = BTreeMap::new();
        for (idx, cat) in RoomCategory::all().iter().enumerate() {
            rooms.insert(idx as u8, MemoryRoom::new(*cat));
        }
        Self {
            rooms,
            tick: 0,
            total_stores: 0,
            total_recalls: 0,
            total_validations: 0,
            rng_state: if seed == 0 { 0xA1AC_CAFE_1234_5678 } else { seed },
        }
    }

    /// Store knowledge about an application
    pub fn store_app_knowledge(
        &mut self,
        app_id: u64,
        app_name: &str,
        cpu: f32,
        mem: f32,
        io: f32,
        net: f32,
    ) {
        self.tick += 1;
        self.total_stores += 1;

        let category = RoomCategory::classify(cpu, mem, io, net);
        let room_idx = RoomCategory::all()
            .iter()
            .position(|c| *c == category)
            .unwrap_or(0) as u8;

        if let Some(room) = self.rooms.get_mut(&room_idx) {
            if let Some(profile) = room.app_profiles.get_mut(&app_id) {
                profile.update_profile(cpu, mem, io, net);
                profile.validate(self.tick);
                self.total_validations += 1;
            } else {
                let profile = AppKnowledgeProfile::new(
                    app_id,
                    String::from(app_name),
                    category,
                    cpu,
                    mem,
                    io,
                    net,
                    self.tick,
                );
                room.app_profiles.insert(app_id, profile);

                // Evict if over capacity
                if room.app_profiles.len() > MAX_PROFILES_PER_ROOM {
                    self.evict_from_room(room_idx, app_id);
                }
            }
            room.recompute_depth();
        }
    }

    /// Recall knowledge about an app pattern
    pub fn recall_app_pattern(
        &mut self,
        cpu: f32,
        mem: f32,
        io: f32,
        net: f32,
    ) -> Vec<(u64, f32)> {
        self.total_recalls += 1;
        let category = RoomCategory::classify(cpu, mem, io, net);
        let room_idx = RoomCategory::all()
            .iter()
            .position(|c| *c == category)
            .unwrap_or(0) as u8;

        let mut matches = Vec::new();

        if let Some(room) = self.rooms.get(&room_idx) {
            for (id, profile) in &room.app_profiles {
                if profile.stale {
                    continue;
                }
                // Simple distance metric
                let d_cpu = (profile.cpu_profile - cpu).abs();
                let d_mem = (profile.mem_profile - mem).abs();
                let d_io = (profile.io_profile - io).abs();
                let d_net = (profile.net_profile - net).abs();
                let dist = (d_cpu + d_mem + d_io + d_net) / 4.0;
                let similarity = 1.0 - dist.min(1.0);

                if similarity > 0.5 {
                    matches.push((*id, similarity * profile.confidence));
                }
            }
        }

        matches.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        matches
    }

    /// Organize knowledge — re-categorize apps that may have shifted
    pub fn organize_knowledge(&mut self) -> usize {
        let mut moves = Vec::new();

        for (room_idx, room) in &self.rooms {
            for (app_id, profile) in &room.app_profiles {
                let new_cat = RoomCategory::classify(
                    profile.cpu_profile,
                    profile.mem_profile,
                    profile.io_profile,
                    profile.net_profile,
                );
                if new_cat != profile.category {
                    moves.push((*room_idx, *app_id, new_cat));
                }
            }
        }

        let move_count = moves.len();
        for (old_room_idx, app_id, new_cat) in moves {
            if let Some(old_room) = self.rooms.get_mut(&old_room_idx) {
                if let Some(mut profile) = old_room.app_profiles.remove(&app_id) {
                    profile.category = new_cat;
                    let new_room_idx = RoomCategory::all()
                        .iter()
                        .position(|c| *c == new_cat)
                        .unwrap_or(0) as u8;
                    if let Some(new_room) = self.rooms.get_mut(&new_room_idx) {
                        new_room.app_profiles.insert(app_id, profile);
                    }
                }
            }
        }

        // Recompute depths
        for (_, room) in self.rooms.iter_mut() {
            room.recompute_depth();
        }

        move_count
    }

    /// Forget stale knowledge across all rooms
    pub fn forget_stale(&mut self) -> usize {
        let mut total_forgotten = 0usize;

        // First mark stale profiles
        for (_, room) in self.rooms.iter_mut() {
            for (_, profile) in room.app_profiles.iter_mut() {
                profile.check_stale(self.tick);
            }
            total_forgotten += room.evict_stale_profiles(self.tick);
            total_forgotten += room.evict_stale_patterns(self.tick);
            room.recompute_depth();
        }

        total_forgotten
    }

    /// Measure knowledge depth for a specific category
    #[inline]
    pub fn knowledge_depth(&self, category: RoomCategory) -> f32 {
        let room_idx = RoomCategory::all()
            .iter()
            .position(|c| *c == category)
            .unwrap_or(0) as u8;
        self.rooms.get(&room_idx).map(|r| r.depth).unwrap_or(0.0)
    }

    /// Store a pattern in a room
    pub fn store_pattern(
        &mut self,
        category: RoomCategory,
        description: &str,
        app_ids: Vec<u64>,
    ) {
        let room_idx = RoomCategory::all()
            .iter()
            .position(|c| *c == category)
            .unwrap_or(0) as u8;

        if let Some(room) = self.rooms.get_mut(&room_idx) {
            let pattern = KnowledgePattern::new(
                String::from(description),
                app_ids,
                self.tick,
            );

            if room.patterns.len() < MAX_PATTERNS_PER_ROOM {
                room.patterns.push(pattern);
            } else {
                // Replace lowest confidence pattern
                if let Some(min_idx) = room
                    .patterns
                    .iter()
                    .enumerate()
                    .min_by(|(_, a), (_, b)| {
                        a.confidence
                            .partial_cmp(&b.confidence)
                            .unwrap_or(core::cmp::Ordering::Equal)
                    })
                    .map(|(i, _)| i)
                {
                    room.patterns[min_idx] = pattern;
                }
            }
            room.recompute_depth();
        }
    }

    /// Full stats
    pub fn palace_stats(&self) -> PalaceStats {
        let mut total_profiles = 0usize;
        let mut total_patterns = 0usize;
        let mut depth_sum = 0.0_f32;
        let mut deepest_depth = -1.0_f32;
        let mut deepest_cat = RoomCategory::Balanced;
        let mut stale_count = 0usize;

        for (_, room) in &self.rooms {
            total_profiles += room.app_profiles.len();
            total_patterns += room.patterns.len();
            depth_sum += room.depth;
            if room.depth > deepest_depth {
                deepest_depth = room.depth;
                deepest_cat = room.category;
            }
            stale_count += room.app_profiles.values().filter(|p| p.stale).count();
        }

        let n = self.rooms.len().max(1) as f32;

        PalaceStats {
            total_rooms: self.rooms.len(),
            total_profiles,
            total_patterns,
            mean_depth: depth_sum / n,
            deepest_room: deepest_cat,
            stale_profile_count: stale_count,
            total_validations: self.total_validations,
        }
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn evict_from_room(&mut self, room_idx: u8, keep_id: u64) {
        if let Some(room) = self.rooms.get_mut(&room_idx) {
            let mut worst_id = 0u64;
            let mut worst_conf = f32::MAX;
            for (id, profile) in &room.app_profiles {
                if *id != keep_id && profile.confidence < worst_conf {
                    worst_conf = profile.confidence;
                    worst_id = *id;
                }
            }
            if worst_id != 0 {
                room.app_profiles.remove(&worst_id);
            }
        }
    }
}
