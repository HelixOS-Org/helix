// SPDX-License-Identifier: GPL-2.0
//! # Bridge Abstraction — Dynamic Abstraction Creation
//!
//! The bridge creates *new* abstractions it was never programmed with.
//! When certain groups of syscalls always co-occur, the engine discovers
//! the pattern and mints a meta-syscall abstraction. Abstractions are
//! organised into a hierarchy; each level compresses operational complexity
//! and is continuously evaluated for utility via EMA tracking.
//!
//! FNV-1a hashing uniquely identifies patterns; xorshift64 drives
//! stochastic pattern sampling; EMA smooths utility scores.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ABSTRACTIONS: usize = 512;
const MAX_COMPONENTS: usize = 32;
const MAX_PATTERNS: usize = 256;
const MAX_HIERARCHY_DEPTH: usize = 8;
const MIN_CO_OCCURRENCE: u64 = 5;
const EMA_ALPHA: f32 = 0.10;
const UTILITY_DECAY: f32 = 0.995;
const COMPRESSION_MIN: f32 = 1.5;
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
// ABSTRACTION TYPES
// ============================================================================

/// Category of an abstraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbstractionCategory {
    MetaSyscall,
    ResourceGroup,
    BehaviourCluster,
    LatencyClass,
    SecurityDomain,
    SchedulingPattern,
    IoBundle,
    CompositeService,
}

/// A single abstraction level discovered or created by the engine.
#[derive(Debug, Clone)]
pub struct AbstractionLevel {
    pub abs_id: u64,
    pub name: String,
    pub category: AbstractionCategory,
    pub components: Vec<String>,
    pub parent_id: Option<u64>,
    pub depth: usize,
    pub usage_count: u64,
    pub utility: f32,
    pub compression_ratio: f32,
    pub created_tick: u64,
    pub last_used_tick: u64,
}

/// A co-occurrence pattern detected among syscalls or operations.
#[derive(Debug, Clone)]
pub struct CoOccurrencePattern {
    pub pattern_id: u64,
    pub elements: Vec<String>,
    pub occurrence_count: u64,
    pub confidence: f32,
    pub first_seen_tick: u64,
    pub last_seen_tick: u64,
    pub promoted: bool,
}

/// Hierarchy snapshot describing the abstraction tree.
#[derive(Debug, Clone)]
pub struct HierarchySnapshot {
    pub total_abstractions: usize,
    pub max_depth: usize,
    pub avg_utility: f32,
    pub avg_compression: f32,
    pub roots: Vec<u64>,
}

// ============================================================================
// ABSTRACTION STATS
// ============================================================================

/// Aggregate statistics for the abstraction engine.
#[derive(Debug, Clone, Copy, Default)]
pub struct AbstractionStats {
    pub total_abstractions: u64,
    pub total_patterns: u64,
    pub patterns_promoted: u64,
    pub avg_utility: f32,
    pub avg_compression: f32,
    pub max_depth: u32,
    pub usage_total: u64,
    pub abstractions_evolved: u64,
    pub utility_ema: f32,
}

// ============================================================================
// PATTERN TRACKER
// ============================================================================

#[derive(Debug, Clone)]
struct PatternTracker {
    patterns: BTreeMap<u64, CoOccurrencePattern>,
    tick: u64,
}

impl PatternTracker {
    fn new() -> Self {
        Self { patterns: BTreeMap::new(), tick: 0 }
    }

    fn observe(&mut self, elements: &[String], tick: u64) -> u64 {
        self.tick = tick;
        let mut combined = Vec::new();
        for e in elements {
            combined.extend_from_slice(e.as_bytes());
            combined.push(0xFF);
        }
        let pid = fnv1a_hash(&combined);

        if let Some(p) = self.patterns.get_mut(&pid) {
            p.occurrence_count += 1;
            p.last_seen_tick = tick;
            p.confidence = (p.occurrence_count as f32 / (p.occurrence_count as f32 + 10.0)).min(1.0);
        } else if self.patterns.len() < MAX_PATTERNS {
            self.patterns.insert(pid, CoOccurrencePattern {
                pattern_id: pid,
                elements: elements.to_vec(),
                occurrence_count: 1,
                confidence: 0.1,
                first_seen_tick: tick,
                last_seen_tick: tick,
                promoted: false,
            });
        }

        pid
    }

    fn promotable(&self) -> Vec<u64> {
        self.patterns
            .iter()
            .filter(|(_, p)| p.occurrence_count >= MIN_CO_OCCURRENCE && !p.promoted)
            .map(|(&id, _)| id)
            .collect()
    }

    fn mark_promoted(&mut self, pid: u64) {
        if let Some(p) = self.patterns.get_mut(&pid) {
            p.promoted = true;
        }
    }
}

// ============================================================================
// BRIDGE ABSTRACTION ENGINE
// ============================================================================

/// Dynamic abstraction creation engine. Discovers co-occurring syscall
/// patterns and promotes them into first-class meta-abstractions.
#[derive(Debug)]
pub struct BridgeAbstraction {
    abstractions: BTreeMap<u64, AbstractionLevel>,
    tracker: PatternTracker,
    patterns_promoted: u64,
    abstractions_evolved: u64,
    tick: u64,
    rng_state: u64,
    utility_ema: f32,
    compression_ema: f32,
}

impl BridgeAbstraction {
    /// Create a new abstraction engine.
    pub fn new(seed: u64) -> Self {
        Self {
            abstractions: BTreeMap::new(),
            tracker: PatternTracker::new(),
            patterns_promoted: 0,
            abstractions_evolved: 0,
            tick: 0,
            rng_state: seed ^ 0xAB57_RACT_0001,
            utility_ema: 0.0,
            compression_ema: 1.0,
        }
    }

    /// Create an abstraction explicitly from a set of components.
    pub fn create_abstraction(
        &mut self,
        name: &str,
        category: AbstractionCategory,
        components: &[String],
        parent_id: Option<u64>,
    ) -> u64 {
        self.tick += 1;
        let abs_id = fnv1a_hash(name.as_bytes()) ^ self.tick;
        let depth = if let Some(pid) = parent_id {
            self.abstractions.get(&pid).map_or(0, |a| a.depth + 1)
        } else {
            0
        };

        let compression = if components.is_empty() {
            1.0
        } else {
            (components.len() as f32).max(COMPRESSION_MIN)
        };

        let abs = AbstractionLevel {
            abs_id,
            name: String::from(name),
            category,
            components: components.to_vec(),
            parent_id,
            depth: depth.min(MAX_HIERARCHY_DEPTH),
            usage_count: 0,
            utility: 0.5,
            compression_ratio: compression,
            created_tick: self.tick,
            last_used_tick: self.tick,
        };

        if self.abstractions.len() < MAX_ABSTRACTIONS {
            self.abstractions.insert(abs_id, abs);
        }

        abs_id
    }

    /// Observe a set of co-occurring elements and auto-discover patterns.
    pub fn discover_pattern(&mut self, elements: &[String]) -> u64 {
        self.tick += 1;
        let pid = self.tracker.observe(elements, self.tick);

        // Check if any pattern is ready for promotion
        let promotable = self.tracker.promotable();
        for prom_id in promotable {
            if let Some(pattern) = self.tracker.patterns.get(&prom_id) {
                let name_seed = prom_id.to_le_bytes();
                let name = {
                    let mut s = String::from("auto_abs_");
                    for b in &name_seed[..4] {
                        let hex_chars = [
                            b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
                            b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
                        ];
                        s.push(hex_chars[(*b >> 4) as usize] as char);
                        s.push(hex_chars[(*b & 0x0F) as usize] as char);
                    }
                    s
                };

                let components = pattern.elements.clone();
                self.create_abstraction(
                    &name,
                    AbstractionCategory::BehaviourCluster,
                    &components,
                    None,
                );
                self.tracker.mark_promoted(prom_id);
                self.patterns_promoted += 1;
            }
        }

        pid
    }

    /// Snapshot of the abstraction hierarchy.
    pub fn abstraction_hierarchy(&self) -> HierarchySnapshot {
        let mut max_depth = 0usize;
        let mut utility_sum = 0.0_f32;
        let mut compression_sum = 0.0_f32;
        let mut roots = Vec::new();

        for (_, abs) in &self.abstractions {
            if abs.depth > max_depth {
                max_depth = abs.depth;
            }
            utility_sum += abs.utility;
            compression_sum += abs.compression_ratio;
            if abs.parent_id.is_none() {
                roots.push(abs.abs_id);
            }
        }

        let n = self.abstractions.len().max(1) as f32;

        HierarchySnapshot {
            total_abstractions: self.abstractions.len(),
            max_depth,
            avg_utility: utility_sum / n,
            avg_compression: compression_sum / n,
            roots,
        }
    }

    /// Compress a sequence of operations using a known abstraction.
    /// Returns the compression ratio achieved, or 0.0 if no match.
    pub fn compress_via_abstraction(&mut self, operations: &[String]) -> (u64, f32) {
        self.tick += 1;
        let mut best_id = 0u64;
        let mut best_ratio = 0.0_f32;

        for (&abs_id, abs) in &self.abstractions {
            if abs.components.is_empty() {
                continue;
            }
            let matched = operations
                .iter()
                .filter(|op| abs.components.contains(op))
                .count();
            let ratio = matched as f32 / abs.components.len().max(1) as f32;
            let effective = ratio * abs.compression_ratio;
            if effective > best_ratio {
                best_ratio = effective;
                best_id = abs_id;
            }
        }

        if best_ratio > 0.0 {
            if let Some(abs) = self.abstractions.get_mut(&best_id) {
                abs.usage_count += 1;
                abs.last_used_tick = self.tick;
            }
        }

        (best_id, best_ratio)
    }

    /// Compute utility for a given abstraction based on usage and compression.
    pub fn abstraction_utility(&mut self, abs_id: u64) -> f32 {
        if let Some(abs) = self.abstractions.get_mut(&abs_id) {
            let age = (self.tick.saturating_sub(abs.created_tick) + 1) as f32;
            let usage_rate = abs.usage_count as f32 / age;
            let utility = 0.6 * usage_rate + 0.4 * (abs.compression_ratio / 10.0).min(1.0);
            abs.utility = utility;
            self.utility_ema = EMA_ALPHA * utility + (1.0 - EMA_ALPHA) * self.utility_ema;
            utility
        } else {
            0.0
        }
    }

    /// Evolve an abstraction: decay low-utility ones, strengthen high-utility.
    pub fn evolve_abstraction(&mut self, abs_id: u64) -> bool {
        self.tick += 1;
        if let Some(abs) = self.abstractions.get_mut(&abs_id) {
            let age_ticks = self.tick.saturating_sub(abs.last_used_tick);
            let decay_factor = UTILITY_DECAY.powi(age_ticks.min(1000) as i32);
            abs.utility *= decay_factor;

            if abs.utility < 0.01 && abs.usage_count < 2 {
                // Prune: mark for removal
                self.abstractions_evolved += 1;
                return false; // caller should remove if desired
            }

            // Possibly mutate: add a random component placeholder
            if abs.utility > 0.5 && abs.components.len() < MAX_COMPONENTS {
                let mutation_roll = xorshift64(&mut self.rng_state) % 100;
                if mutation_roll < 10 {
                    let idx = xorshift64(&mut self.rng_state) % 999;
                    let mut comp_name = String::from("evolved_");
                    let digits = [
                        (idx / 100 % 10) as u8 + b'0',
                        (idx / 10 % 10) as u8 + b'0',
                        (idx % 10) as u8 + b'0',
                    ];
                    for d in &digits {
                        comp_name.push(*d as char);
                    }
                    abs.components.push(comp_name);
                    abs.compression_ratio = abs.components.len() as f32;
                }
            }

            self.abstractions_evolved += 1;
            true
        } else {
            false
        }
    }

    /// Prune abstractions with zero utility.
    pub fn prune_dead(&mut self) -> usize {
        let before = self.abstractions.len();
        self.abstractions.retain(|_, a| a.utility > 0.001 || a.usage_count > 0);
        before - self.abstractions.len()
    }

    /// Look up an abstraction by ID.
    pub fn get_abstraction(&self, abs_id: u64) -> Option<&AbstractionLevel> {
        self.abstractions.get(&abs_id)
    }

    /// All children of a given parent abstraction.
    pub fn children_of(&self, parent_id: u64) -> Vec<u64> {
        self.abstractions
            .iter()
            .filter(|(_, a)| a.parent_id == Some(parent_id))
            .map(|(&id, _)| id)
            .collect()
    }

    /// Total abstractions count.
    pub fn abstraction_count(&self) -> usize {
        self.abstractions.len()
    }

    /// Aggregate statistics.
    pub fn stats(&self) -> AbstractionStats {
        let mut utility_sum = 0.0_f32;
        let mut compression_sum = 0.0_f32;
        let mut usage_total = 0u64;
        let mut max_depth = 0u32;

        for (_, abs) in &self.abstractions {
            utility_sum += abs.utility;
            compression_sum += abs.compression_ratio;
            usage_total += abs.usage_count;
            if abs.depth as u32 > max_depth {
                max_depth = abs.depth as u32;
            }
        }

        let n = self.abstractions.len().max(1) as f32;

        AbstractionStats {
            total_abstractions: self.abstractions.len() as u64,
            total_patterns: self.tracker.patterns.len() as u64,
            patterns_promoted: self.patterns_promoted,
            avg_utility: utility_sum / n,
            avg_compression: compression_sum / n,
            max_depth,
            usage_total,
            abstractions_evolved: self.abstractions_evolved,
            utility_ema: self.utility_ema,
        }
    }

    /// Current tick.
    pub fn tick(&self) -> u64 {
        self.tick
    }
}
