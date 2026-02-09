// SPDX-License-Identifier: GPL-2.0
//! # Bridge Memory Palace
//!
//! Structured memory organization for bridge knowledge using a spatial
//! metaphor. All bridge knowledge is organized into:
//!
//! - **Rooms** — categories of knowledge (one room per syscall family)
//! - **Corridors** — relationships between rooms
//! - **Vaults** — critical, high-importance patterns stored securely
//!
//! Implements spaced repetition: important patterns are reinforced at
//! increasing intervals, while unused patterns decay along a forgetting
//! curve. Memory strength is a function of recency, frequency, and
//! importance.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ROOMS: usize = 64;
const MAX_ENTRIES_PER_ROOM: usize = 128;
const MAX_CORRIDORS: usize = 256;
const MAX_VAULT_ENTRIES: usize = 64;
const FORGETTING_HALF_LIFE: f32 = 500.0;
const SPACED_REPETITION_BASE: f32 = 1.5;
const STRENGTH_RECALL_BOOST: f32 = 0.20;
const STRENGTH_STORE_INITIAL: f32 = 0.50;
const IMPORTANCE_THRESHOLD_VAULT: f32 = 0.85;
const PRUNE_THRESHOLD: f32 = 0.05;
const EMA_ALPHA: f32 = 0.08;
const MAX_RECALL_HISTORY: usize = 256;
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

fn ema_update(current: f32, sample: f32, alpha: f32) -> f32 {
    current * (1.0 - alpha) + sample * alpha
}

/// Forgetting curve: strength decays exponentially with time
fn forgetting_curve(initial_strength: f32, ticks_elapsed: u64) -> f32 {
    let decay = (-1.0 * ticks_elapsed as f32 / FORGETTING_HALF_LIFE).exp();
    initial_strength * decay
}

/// Spaced repetition interval: grows exponentially with review count
fn spaced_repetition_interval(review_count: u32) -> u64 {
    let interval = SPACED_REPETITION_BASE.powi(review_count as i32);
    (interval * 100.0) as u64
}

// ============================================================================
// MEMORY ENTRY
// ============================================================================

/// A single memory entry within a room
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub key_hash: u64,
    pub key: String,
    pub value: String,
    pub importance: f32,
    pub strength: f32,
    pub created_tick: u64,
    pub last_accessed_tick: u64,
    pub access_count: u32,
    pub review_count: u32,
    pub next_review_tick: u64,
}

impl MemoryEntry {
    fn new(key: &str, value: &str, importance: f32, tick: u64) -> Self {
        let key_hash = fnv1a_hash(key.as_bytes());
        Self {
            key_hash,
            key: String::from(key),
            value: String::from(value),
            importance: importance.clamp(0.0, 1.0),
            strength: STRENGTH_STORE_INITIAL,
            created_tick: tick,
            last_accessed_tick: tick,
            access_count: 0,
            review_count: 0,
            next_review_tick: tick + spaced_repetition_interval(0),
        }
    }

    fn recall(&mut self, tick: u64) {
        self.access_count += 1;
        self.last_accessed_tick = tick;
        self.strength = (self.strength + STRENGTH_RECALL_BOOST).clamp(0.0, 1.0);
        self.review_count += 1;
        self.next_review_tick = tick + spaced_repetition_interval(self.review_count);
    }

    fn current_strength(&self, current_tick: u64) -> f32 {
        let elapsed = current_tick.saturating_sub(self.last_accessed_tick);
        forgetting_curve(self.strength, elapsed)
    }

    fn needs_review(&self, current_tick: u64) -> bool {
        current_tick >= self.next_review_tick
    }

    fn is_weak(&self, current_tick: u64) -> bool {
        self.current_strength(current_tick) < PRUNE_THRESHOLD
    }
}

// ============================================================================
// MEMORY ROOM
// ============================================================================

/// A room in the memory palace — a category of knowledge
#[derive(Debug, Clone)]
pub struct MemoryRoom {
    pub topic: String,
    pub topic_hash: u64,
    pub entries: BTreeMap<u64, MemoryEntry>,
    pub importance: f32,
    pub last_accessed: u64,
    pub total_accesses: u64,
    pub created_tick: u64,
}

impl MemoryRoom {
    fn new(topic: &str, tick: u64) -> Self {
        Self {
            topic: String::from(topic),
            topic_hash: fnv1a_hash(topic.as_bytes()),
            entries: BTreeMap::new(),
            importance: 0.5,
            last_accessed: tick,
            total_accesses: 0,
            created_tick: tick,
        }
    }

    fn store(&mut self, key: &str, value: &str, importance: f32, tick: u64) -> bool {
        let key_hash = fnv1a_hash(key.as_bytes());
        if let Some(existing) = self.entries.get_mut(&key_hash) {
            existing.value = String::from(value);
            existing.recall(tick);
            existing.importance = importance.clamp(0.0, 1.0);
            self.last_accessed = tick;
            self.total_accesses += 1;
            true
        } else if self.entries.len() < MAX_ENTRIES_PER_ROOM {
            let entry = MemoryEntry::new(key, value, importance, tick);
            self.entries.insert(key_hash, entry);
            self.last_accessed = tick;
            self.total_accesses += 1;
            true
        } else {
            false
        }
    }

    fn recall(&mut self, key: &str, tick: u64) -> Option<String> {
        let key_hash = fnv1a_hash(key.as_bytes());
        if let Some(entry) = self.entries.get_mut(&key_hash) {
            entry.recall(tick);
            self.last_accessed = tick;
            self.total_accesses += 1;
            Some(entry.value.clone())
        } else {
            None
        }
    }

    fn prune_weak(&mut self, tick: u64) -> usize {
        let to_remove: Vec<u64> = self
            .entries
            .iter()
            .filter(|(_, e)| e.is_weak(tick))
            .map(|(&h, _)| h)
            .collect();
        let count = to_remove.len();
        for h in to_remove {
            self.entries.remove(&h);
        }
        count
    }

    fn avg_strength(&self, tick: u64) -> f32 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let total: f32 = self
            .entries
            .values()
            .map(|e| e.current_strength(tick))
            .sum();
        total / self.entries.len() as f32
    }

    fn entries_needing_review(&self, tick: u64) -> usize {
        self.entries.values().filter(|e| e.needs_review(tick)).count()
    }
}

// ============================================================================
// CORRIDOR
// ============================================================================

/// A corridor linking two rooms — a relationship between knowledge areas
#[derive(Debug, Clone)]
pub struct Corridor {
    pub room_a_hash: u64,
    pub room_b_hash: u64,
    pub strength: f32,
    pub traversal_count: u64,
    pub last_traversed: u64,
}

// ============================================================================
// VAULT ENTRY
// ============================================================================

/// A critical pattern stored in the vault for permanent retention
#[derive(Debug, Clone)]
pub struct VaultEntry {
    pub key_hash: u64,
    pub key: String,
    pub value: String,
    pub importance: f32,
    pub stored_tick: u64,
}

// ============================================================================
// STATS
// ============================================================================

/// Memory palace statistics
#[derive(Debug, Clone)]
pub struct PalaceStats {
    pub total_rooms: usize,
    pub total_entries: usize,
    pub total_corridors: usize,
    pub vault_entries: usize,
    pub avg_memory_strength: f32,
    pub entries_needing_review: usize,
    pub total_recalls: u64,
    pub total_stores: u64,
}

// ============================================================================
// BRIDGE MEMORY PALACE
// ============================================================================

/// Structured memory organization for all bridge knowledge
#[derive(Debug, Clone)]
pub struct BridgeMemoryPalace {
    rooms: BTreeMap<u64, MemoryRoom>,
    corridors: Vec<Corridor>,
    vault: BTreeMap<u64, VaultEntry>,
    recall_history: Vec<(u64, u64, bool)>,
    current_tick: u64,
    total_recalls: u64,
    total_stores: u64,
    total_forgotten: u64,
    avg_strength_ema: f32,
}

impl BridgeMemoryPalace {
    /// Create a new memory palace
    pub fn new() -> Self {
        Self {
            rooms: BTreeMap::new(),
            corridors: Vec::new(),
            vault: BTreeMap::new(),
            recall_history: Vec::new(),
            current_tick: 0,
            total_recalls: 0,
            total_stores: 0,
            total_forgotten: 0,
            avg_strength_ema: 0.5,
        }
    }

    /// Store a memory in the appropriate room
    pub fn store_memory(
        &mut self,
        room_topic: &str,
        key: &str,
        value: &str,
        importance: f32,
    ) -> bool {
        self.current_tick += 1;
        self.total_stores += 1;

        let room_hash = fnv1a_hash(room_topic.as_bytes());

        // Create room if it doesn't exist
        if !self.rooms.contains_key(&room_hash) && self.rooms.len() < MAX_ROOMS {
            let room = MemoryRoom::new(room_topic, self.current_tick);
            self.rooms.insert(room_hash, room);
        }

        let stored = if let Some(room) = self.rooms.get_mut(&room_hash) {
            room.store(key, value, importance, self.current_tick)
        } else {
            false
        };

        // Auto-vault critical memories
        if importance >= IMPORTANCE_THRESHOLD_VAULT && self.vault.len() < MAX_VAULT_ENTRIES {
            let key_hash = fnv1a_hash(key.as_bytes());
            if !self.vault.contains_key(&key_hash) {
                self.vault.insert(
                    key_hash,
                    VaultEntry {
                        key_hash,
                        key: String::from(key),
                        value: String::from(value),
                        importance,
                        stored_tick: self.current_tick,
                    },
                );
            }
        }

        stored
    }

    /// Recall a memory from a specific room
    pub fn recall_memory(&mut self, room_topic: &str, key: &str) -> Option<String> {
        self.current_tick += 1;
        self.total_recalls += 1;

        let room_hash = fnv1a_hash(room_topic.as_bytes());
        let key_hash = fnv1a_hash(key.as_bytes());

        let result = if let Some(room) = self.rooms.get_mut(&room_hash) {
            room.recall(key, self.current_tick)
        } else {
            None
        };

        // If not found in room, check vault
        let result = result.or_else(|| {
            self.vault.get(&key_hash).map(|v| v.value.clone())
        });

        let found = result.is_some();
        if self.recall_history.len() >= MAX_RECALL_HISTORY {
            self.recall_history.remove(0);
        }
        self.recall_history.push((room_hash, self.current_tick, found));

        result
    }

    /// Organize the palace — create corridors between related rooms
    pub fn organize_palace(&mut self) {
        self.current_tick += 1;

        // Build corridors based on co-access patterns
        let room_hashes: Vec<u64> = self.rooms.keys().copied().collect();
        for i in 0..room_hashes.len() {
            for j in (i + 1)..room_hashes.len() {
                let hash_a = room_hashes[i];
                let hash_b = room_hashes[j];

                // Count co-accesses in recall history
                let co_access = self
                    .recall_history
                    .windows(2)
                    .filter(|w| {
                        (w[0].0 == hash_a && w[1].0 == hash_b)
                            || (w[0].0 == hash_b && w[1].0 == hash_a)
                    })
                    .count();

                if co_access > 0 && self.corridors.len() < MAX_CORRIDORS {
                    let strength = (co_access as f32 / 10.0).clamp(0.0, 1.0);
                    // Check if corridor already exists
                    let existing = self.corridors.iter_mut().find(|c| {
                        (c.room_a_hash == hash_a && c.room_b_hash == hash_b)
                            || (c.room_a_hash == hash_b && c.room_b_hash == hash_a)
                    });

                    if let Some(corridor) = existing {
                        corridor.strength = ema_update(corridor.strength, strength, EMA_ALPHA);
                        corridor.traversal_count += 1;
                        corridor.last_traversed = self.current_tick;
                    } else {
                        self.corridors.push(Corridor {
                            room_a_hash: hash_a,
                            room_b_hash: hash_b,
                            strength,
                            traversal_count: 1,
                            last_traversed: self.current_tick,
                        });
                    }
                }
            }
        }
    }

    /// Forget irrelevant memories — prune weak entries across all rooms
    pub fn forget_irrelevant(&mut self) -> usize {
        self.current_tick += 1;
        let mut total_forgotten = 0;

        for room in self.rooms.values_mut() {
            let pruned = room.prune_weak(self.current_tick);
            total_forgotten += pruned;
        }

        self.total_forgotten += total_forgotten as u64;

        // Also prune empty rooms
        let empty_rooms: Vec<u64> = self
            .rooms
            .iter()
            .filter(|(_, r)| r.entries.is_empty())
            .map(|(&h, _)| h)
            .collect();
        for h in empty_rooms {
            self.rooms.remove(&h);
        }

        total_forgotten
    }

    /// Memory strength for a specific entry
    pub fn memory_strength(&self, room_topic: &str, key: &str) -> f32 {
        let room_hash = fnv1a_hash(room_topic.as_bytes());
        let key_hash = fnv1a_hash(key.as_bytes());

        if let Some(room) = self.rooms.get(&room_hash) {
            if let Some(entry) = room.entries.get(&key_hash) {
                return entry.current_strength(self.current_tick);
            }
        }

        // Vault entries never decay
        if self.vault.contains_key(&key_hash) {
            return 1.0;
        }

        0.0
    }

    /// Total size of the memory palace
    pub fn palace_size(&self) -> usize {
        let room_entries: usize = self.rooms.values().map(|r| r.entries.len()).sum();
        room_entries + self.vault.len()
    }

    /// Get rooms that need review (contain entries due for spaced repetition)
    pub fn rooms_needing_review(&self) -> Vec<(String, usize)> {
        let mut result = Vec::new();
        for room in self.rooms.values() {
            let count = room.entries_needing_review(self.current_tick);
            if count > 0 {
                result.push((room.topic.clone(), count));
            }
        }
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }

    /// Average memory strength across the entire palace
    pub fn avg_strength(&self) -> f32 {
        if self.rooms.is_empty() {
            return 0.0;
        }
        let total: f32 = self
            .rooms
            .values()
            .map(|r| r.avg_strength(self.current_tick))
            .sum();
        total / self.rooms.len() as f32
    }

    /// Statistics snapshot
    pub fn stats(&self) -> PalaceStats {
        let total_entries: usize = self.rooms.values().map(|r| r.entries.len()).sum();
        let review_count: usize = self
            .rooms
            .values()
            .map(|r| r.entries_needing_review(self.current_tick))
            .sum();

        PalaceStats {
            total_rooms: self.rooms.len(),
            total_entries,
            total_corridors: self.corridors.len(),
            vault_entries: self.vault.len(),
            avg_memory_strength: self.avg_strength(),
            entries_needing_review: review_count,
            total_recalls: self.total_recalls,
            total_stores: self.total_stores,
        }
    }

    /// Reset the entire palace
    pub fn reset(&mut self) {
        self.rooms.clear();
        self.corridors.clear();
        self.vault.clear();
        self.recall_history.clear();
        self.avg_strength_ema = 0.5;
    }
}
