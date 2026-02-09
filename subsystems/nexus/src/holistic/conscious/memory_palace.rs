// SPDX-License-Identifier: GPL-2.0
//! # Holistic Memory Palace
//!
//! **The GRAND PALACE of all system knowledge.** A hierarchical memory structure
//! that organizes ALL kernel knowledge: hardware topology, software behavior,
//! historical patterns, optimization results, and research findings. This is
//! the kernel's long-term memory — everything it has ever learned, stored in
//! an efficiently navigable hierarchy.
//!
//! ## Palace Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │                  GRAND MEMORY PALACE                         │
//! ├──────────────────────────────────────────────────────────────┤
//! │  Wing: Hardware ──▶ Room: CPU Topology                      │
//! │       │           ──▶ Room: Memory Layout                   │
//! │       │           ──▶ Room: Device Map                      │
//! │                                                             │
//! │  Wing: Software ──▶ Room: Process Behavior                  │
//! │       │           ──▶ Room: Workload Profiles               │
//! │       │           ──▶ Room: Failure Modes                   │
//! │                                                             │
//! │  Wing: History ───▶ Room: Optimization Results              │
//! │       │           ──▶ Room: Incident Archive                │
//! │       │           ──▶ Room: Performance Trends              │
//! │                                                             │
//! │  Wing: Insights ──▶ Room: Cross-Subsystem Findings          │
//! │                  ──▶ Room: Research Results                  │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! Knowledge is stored with importance weights and access frequency tracking.
//! Old, unused knowledge is compressed and eventually forgotten.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const EMA_ALPHA: f32 = 0.08;
const MAX_WINGS: usize = 16;
const MAX_ROOMS_PER_WING: usize = 32;
const MAX_ITEMS_PER_ROOM: usize = 256;
const MAX_TOTAL_ITEMS: usize = 4096;
const MAX_HISTORY: usize = 128;
const FORGET_THRESHOLD: f32 = 0.05;
const COMPRESS_THRESHOLD: f32 = 0.15;
const ACCESS_DECAY: f32 = 0.995;
const IMPORTANCE_FLOOR: f32 = 0.01;
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
// KNOWLEDGE CATEGORY
// ============================================================================

/// Top-level knowledge wing in the palace
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeWing {
    /// Hardware topology and capabilities
    Hardware,
    /// Software behavior and patterns
    Software,
    /// Historical events and outcomes
    History,
    /// Cross-subsystem insights and research
    Insights,
    /// Optimization results and tuning data
    Optimization,
    /// Configuration and policy knowledge
    Configuration,
    /// Failure modes and recovery procedures
    Failures,
    /// Performance baselines and envelopes
    Performance,
}

impl KnowledgeWing {
    pub fn all() -> &'static [KnowledgeWing] {
        &[
            KnowledgeWing::Hardware,
            KnowledgeWing::Software,
            KnowledgeWing::History,
            KnowledgeWing::Insights,
            KnowledgeWing::Optimization,
            KnowledgeWing::Configuration,
            KnowledgeWing::Failures,
            KnowledgeWing::Performance,
        ]
    }
}

// ============================================================================
// KNOWLEDGE ITEM
// ============================================================================

/// A single piece of knowledge stored in the palace
#[derive(Debug, Clone)]
pub struct KnowledgeItem {
    pub id: u64,
    pub name: String,
    pub wing: KnowledgeWing,
    pub room: String,
    /// The knowledge content (serialized as string)
    pub content: String,
    /// Importance weight (0.0 – 1.0)
    pub importance: f32,
    /// Access frequency — EMA-smoothed
    pub access_frequency: f32,
    /// Number of times accessed
    pub access_count: u64,
    /// Tick when stored
    pub stored_tick: u64,
    /// Tick when last accessed
    pub last_access_tick: u64,
    /// Whether this item has been compressed
    pub compressed: bool,
    /// Links to related items
    pub related_items: Vec<u64>,
    /// Confidence in this knowledge (0.0 – 1.0)
    pub confidence: f32,
}

impl KnowledgeItem {
    pub fn new(name: String, wing: KnowledgeWing, room: String, content: String, tick: u64) -> Self {
        let id = fnv1a_hash(name.as_bytes()) ^ fnv1a_hash(room.as_bytes());
        Self {
            id,
            name,
            wing,
            room,
            content,
            importance: 0.5,
            access_frequency: 0.0,
            access_count: 0,
            stored_tick: tick,
            last_access_tick: tick,
            compressed: false,
            related_items: Vec::new(),
            confidence: 0.5,
        }
    }

    /// Record an access and update frequency EMA
    #[inline]
    pub fn record_access(&mut self, tick: u64) {
        self.access_count += 1;
        self.last_access_tick = tick;
        self.access_frequency += EMA_ALPHA * (1.0 - self.access_frequency);
    }

    /// Decay access frequency
    #[inline]
    pub fn decay(&mut self) {
        self.access_frequency *= ACCESS_DECAY;
        if self.access_frequency < IMPORTANCE_FLOOR {
            self.access_frequency = 0.0;
        }
    }

    /// Effective value: importance * access_frequency * confidence
    #[inline(always)]
    pub fn effective_value(&self) -> f32 {
        self.importance * (0.3 + 0.7 * self.access_frequency) * self.confidence
    }

    /// Whether this item should be forgotten
    #[inline(always)]
    pub fn should_forget(&self) -> bool {
        self.effective_value() < FORGET_THRESHOLD && !self.compressed
    }

    /// Whether this item should be compressed
    #[inline(always)]
    pub fn should_compress(&self) -> bool {
        self.effective_value() < COMPRESS_THRESHOLD && !self.compressed
    }
}

// ============================================================================
// PALACE ROOM
// ============================================================================

/// A room within a wing — groups related knowledge items
#[derive(Debug, Clone)]
pub struct PalaceRoom {
    pub name: String,
    pub id: u64,
    pub wing: KnowledgeWing,
    pub item_count: u32,
    /// IDs of items stored in this room
    pub item_ids: Vec<u64>,
    /// Room-level importance
    pub importance: f32,
    /// Total accesses to items in this room
    pub total_accesses: u64,
}

impl PalaceRoom {
    pub fn new(name: String, wing: KnowledgeWing) -> Self {
        let id = fnv1a_hash(name.as_bytes());
        Self {
            name,
            id,
            wing,
            item_count: 0,
            item_ids: Vec::new(),
            importance: 0.5,
            total_accesses: 0,
        }
    }
}

// ============================================================================
// RECALL RESULT
// ============================================================================

/// Result of a knowledge recall query
#[derive(Debug, Clone)]
pub struct RecallResult {
    pub item_id: u64,
    pub name: String,
    pub content: String,
    pub relevance: f32,
    pub confidence: f32,
    pub wing: KnowledgeWing,
    pub room: String,
}

// ============================================================================
// PALACE TOUR
// ============================================================================

/// A summary tour of the palace — overview of all wings and rooms
#[derive(Debug, Clone)]
pub struct PalaceTour {
    pub total_items: u32,
    pub total_rooms: u32,
    pub wing_summaries: Vec<(KnowledgeWing, u32, f32)>, // (wing, item_count, avg_importance)
    pub completeness: f32,
    pub oldest_item_tick: u64,
    pub newest_item_tick: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Memory palace statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticMemoryPalaceStats {
    pub total_stored: u64,
    pub total_recalled: u64,
    pub total_forgotten: u64,
    pub total_compressed: u64,
    pub average_importance: f32,
    pub average_access_freq: f32,
    pub recall_hit_rate: f32,
    pub knowledge_completeness: f32,
}

// ============================================================================
// HOLISTIC MEMORY PALACE
// ============================================================================

/// The grand palace of all system knowledge — hierarchical storage of
/// everything the kernel has learned, organized for efficient recall.
pub struct HolisticMemoryPalace {
    /// All knowledge items by ID
    items: BTreeMap<u64, KnowledgeItem>,
    /// Rooms organized by wing
    rooms: BTreeMap<u64, PalaceRoom>,
    /// Wing → room IDs mapping
    wing_rooms: BTreeMap<u8, Vec<u64>>,
    /// Recall attempts and hits for hit-rate tracking
    recall_attempts: u64,
    recall_hits: u64,
    /// Stats
    stats: HolisticMemoryPalaceStats,
    /// PRNG
    rng: u64,
    /// Tick
    tick: u64,
}

impl HolisticMemoryPalace {
    /// Create a new holistic memory palace
    pub fn new(seed: u64) -> Self {
        let mut wing_rooms = BTreeMap::new();
        for (i, _wing) in KnowledgeWing::all().iter().enumerate() {
            wing_rooms.insert(i as u8, Vec::new());
        }
        Self {
            items: BTreeMap::new(),
            rooms: BTreeMap::new(),
            wing_rooms,
            recall_attempts: 0,
            recall_hits: 0,
            stats: HolisticMemoryPalaceStats {
                total_stored: 0,
                total_recalled: 0,
                total_forgotten: 0,
                total_compressed: 0,
                average_importance: 0.5,
                average_access_freq: 0.0,
                recall_hit_rate: 0.0,
                knowledge_completeness: 0.0,
            },
            rng: seed ^ 0xFA1A_CE00_DEAD_BEEF,
            tick: 0,
        }
    }

    /// Get the grand palace overview
    pub fn grand_palace(&self) -> PalaceTour {
        let mut wing_summaries = Vec::new();
        for (i, wing) in KnowledgeWing::all().iter().enumerate() {
            let room_ids = self.wing_rooms.get(&(i as u8)).cloned().unwrap_or_default();
            let mut item_count = 0u32;
            let mut importance_sum = 0.0f32;
            for rid in &room_ids {
                if let Some(room) = self.rooms.get(rid) {
                    item_count += room.item_count;
                    importance_sum += room.importance;
                }
            }
            let avg_imp = if room_ids.is_empty() { 0.0 } else { importance_sum / room_ids.len() as f32 };
            wing_summaries.push((*wing, item_count, avg_imp));
        }
        let mut oldest = u64::MAX;
        let mut newest = 0u64;
        for item in self.items.values() {
            if item.stored_tick < oldest { oldest = item.stored_tick; }
            if item.stored_tick > newest { newest = item.stored_tick; }
        }
        if oldest == u64::MAX { oldest = 0; }
        PalaceTour {
            total_items: self.items.len() as u32,
            total_rooms: self.rooms.len() as u32,
            wing_summaries,
            completeness: self.knowledge_completeness(),
            oldest_item_tick: oldest,
            newest_item_tick: newest,
        }
    }

    /// Store new knowledge in the palace
    pub fn store_system_knowledge(&mut self, item: KnowledgeItem) -> u64 {
        if self.items.len() >= MAX_TOTAL_ITEMS {
            self.forget_and_compress();
        }
        let item_id = item.id;
        let wing_idx = KnowledgeWing::all().iter().position(|w| *w == item.wing).unwrap_or(0) as u8;
        let room_id = fnv1a_hash(item.room.as_bytes());
        // Ensure room exists
        if !self.rooms.contains_key(&room_id) {
            let mut room = PalaceRoom::new(item.room.clone(), item.wing);
            room.id = room_id;
            self.rooms.insert(room_id, room);
            if let Some(room_list) = self.wing_rooms.get_mut(&wing_idx) {
                room_list.push(room_id);
            }
        }
        if let Some(room) = self.rooms.get_mut(&room_id) {
            room.item_ids.push(item_id);
            room.item_count += 1;
        }
        self.items.insert(item_id, item);
        self.stats.total_stored += 1;
        self.update_stats();
        item_id
    }

    /// Recall knowledge matching a query string
    pub fn recall_anything(&mut self, query: &str, tick: u64) -> Vec<RecallResult> {
        self.tick = tick;
        self.recall_attempts += 1;
        let query_hash = fnv1a_hash(query.as_bytes());
        let query_lower = query;
        let mut results = Vec::new();
        for item in self.items.values_mut() {
            let name_match = item.name.contains(query_lower);
            let content_match = item.content.contains(query_lower);
            let hash_proximity = {
                let diff = if item.id > query_hash { item.id - query_hash } else { query_hash - item.id };
                1.0 / (1.0 + (diff % 1000) as f32 / 100.0)
            };
            let relevance = if name_match {
                0.9 + hash_proximity * 0.1
            } else if content_match {
                0.6 + hash_proximity * 0.1
            } else {
                hash_proximity * 0.3
            };
            if relevance > 0.2 || name_match || content_match {
                item.record_access(tick);
                results.push(RecallResult {
                    item_id: item.id,
                    name: item.name.clone(),
                    content: item.content.clone(),
                    relevance,
                    confidence: item.confidence,
                    wing: item.wing,
                    room: item.room.clone(),
                });
            }
        }
        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(core::cmp::Ordering::Equal));
        if !results.is_empty() {
            self.recall_hits += 1;
            self.stats.total_recalled += 1;
        }
        self.stats.recall_hit_rate = if self.recall_attempts > 0 {
            self.recall_hits as f32 / self.recall_attempts as f32
        } else {
            0.0
        };
        results
    }

    /// Organize the hierarchy — rebalance rooms and update importance
    pub fn organize_hierarchy(&mut self) {
        for room in self.rooms.values_mut() {
            let mut total_importance = 0.0f32;
            let count = room.item_ids.len().max(1);
            for iid in &room.item_ids {
                if let Some(item) = self.items.get(iid) {
                    total_importance += item.importance;
                }
            }
            room.importance = total_importance / count as f32;
        }
    }

    /// Knowledge completeness: how populated is the palace?
    #[inline]
    pub fn knowledge_completeness(&self) -> f32 {
        let wing_coverage = self.wing_rooms.values().filter(|r| !r.is_empty()).count() as f32
            / KnowledgeWing::all().len() as f32;
        let item_density = (self.items.len() as f32 / MAX_TOTAL_ITEMS as f32).min(1.0);
        let confidence_avg = if self.items.is_empty() {
            0.0
        } else {
            self.items.values().map(|i| i.confidence).sum::<f32>() / self.items.len() as f32
        };
        wing_coverage * 0.3 + item_density * 0.3 + confidence_avg * 0.4
    }

    /// Forget low-value items and compress borderline ones
    pub fn forget_and_compress(&mut self) {
        let ids_to_forget: Vec<u64> = self.items.values()
            .filter(|item| item.should_forget())
            .map(|item| item.id)
            .collect();
        for id in &ids_to_forget {
            self.items.remove(id);
            self.stats.total_forgotten += 1;
        }
        for item in self.items.values_mut() {
            if item.should_compress() && !item.compressed {
                item.compressed = true;
                // Truncate content as compression proxy
                if item.content.len() > 64 {
                    item.content.truncate(64);
                }
                self.stats.total_compressed += 1;
            }
        }
        // Clean up room references
        for room in self.rooms.values_mut() {
            room.item_ids.retain(|iid| self.items.contains_key(iid));
            room.item_count = room.item_ids.len() as u32;
        }
    }

    /// Take a tour of the palace — get a high-level summary
    #[inline(always)]
    pub fn palace_tour(&self) -> PalaceTour {
        self.grand_palace()
    }

    /// Decay all item access frequencies
    #[inline]
    pub fn decay_all(&mut self) {
        for item in self.items.values_mut() {
            item.decay();
        }
    }

    /// Get item count
    #[inline(always)]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Get room count
    #[inline(always)]
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticMemoryPalaceStats {
        &self.stats
    }

    // ========================================================================
    // INTERNAL
    // ========================================================================

    fn update_stats(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let total_imp: f32 = self.items.values().map(|i| i.importance).sum();
        let total_freq: f32 = self.items.values().map(|i| i.access_frequency).sum();
        let count = self.items.len() as f32;
        self.stats.average_importance = total_imp / count;
        self.stats.average_access_freq = total_freq / count;
        self.stats.knowledge_completeness = self.knowledge_completeness();
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_item_value() {
        let mut item = KnowledgeItem::new(
            String::from("cpu_topology"),
            KnowledgeWing::Hardware,
            String::from("CPU"),
            String::from("4 cores, 8 threads"),
            1,
        );
        item.importance = 0.9;
        item.confidence = 0.8;
        assert!(item.effective_value() > 0.0);
    }

    #[test]
    fn test_palace_creation() {
        let palace = HolisticMemoryPalace::new(42);
        assert_eq!(palace.item_count(), 0);
        assert_eq!(palace.room_count(), 0);
    }

    #[test]
    fn test_store_and_recall() {
        let mut palace = HolisticMemoryPalace::new(99);
        let item = KnowledgeItem::new(
            String::from("test_knowledge"),
            KnowledgeWing::Software,
            String::from("testing"),
            String::from("unit tests pass"),
            1,
        );
        palace.store_system_knowledge(item);
        assert_eq!(palace.item_count(), 1);
        let results = palace.recall_anything("test", 2);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_completeness() {
        let palace = HolisticMemoryPalace::new(42);
        assert_eq!(palace.knowledge_completeness(), 0.0);
    }

    #[test]
    fn test_fnv1a() {
        assert_eq!(fnv1a_hash(b"palace"), fnv1a_hash(b"palace"));
    }
}
