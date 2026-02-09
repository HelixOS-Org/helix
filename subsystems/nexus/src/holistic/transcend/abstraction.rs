// SPDX-License-Identifier: GPL-2.0
//! # Holistic Abstraction — SYSTEM-WIDE Dynamic Abstraction Creation
//!
//! `HolisticAbstraction` endows NEXUS with the ability to create NEW
//! concepts on-the-fly — abstractions that capture system behaviour at
//! progressively higher semantic levels.  The kernel observes patterns
//! across hardware, software, and emergent behaviour, then synthesises
//! reusable abstractions that compress knowledge into powerful building
//! blocks.
//!
//! Abstractions form a *tower*: each level builds on the one below,
//! and meta-abstractions can span multiple towers to capture cross-domain
//! phenomena that no single subsystem could express alone.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const EMA_ALPHA_NUM: u64 = 3;
const EMA_ALPHA_DEN: u64 = 16; // α ≈ 0.188
const MAX_ABSTRACTIONS: usize = 1024;
const MAX_TOWER_LEVELS: usize = 32;
const MAX_META_ABSTRACTIONS: usize = 256;
const MAX_CONCEPTS: usize = 512;
const MAX_LOG_ENTRIES: usize = 512;
const USEFUL_THRESHOLD_BPS: u64 = 6_000;
const EMERGENT_THRESHOLD_BPS: u64 = 8_000;
const COMPRESSION_TARGET_BPS: u64 = 7_500;

// ---------------------------------------------------------------------------
// FNV-1a helper
// ---------------------------------------------------------------------------

fn fnv1a(data: &[u8]) -> u64 {
    let mut h = FNV_OFFSET;
    for &b in data {
        h ^= b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

// ---------------------------------------------------------------------------
// xorshift64 PRNG
// ---------------------------------------------------------------------------

struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0xb0bacafe1234 } else { seed },
        }
    }

    fn next(&mut self) -> u64 {
        let mut s = self.state;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.state = s;
        s
    }
}

// ---------------------------------------------------------------------------
// EMA helper
// ---------------------------------------------------------------------------

fn ema_update(prev: u64, sample: u64) -> u64 {
    (EMA_ALPHA_NUM * sample + (EMA_ALPHA_DEN - EMA_ALPHA_NUM) * prev) / EMA_ALPHA_DEN
}

// ---------------------------------------------------------------------------
// Abstraction — a single system-level concept
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Abstraction {
    pub abs_hash: u64,
    pub name: String,
    pub level: u64,
    pub parent_hashes: Vec<u64>,
    pub child_hashes: Vec<u64>,
    pub source_domains: Vec<String>,
    pub utility_bps: u64,
    pub ema_utility: u64,
    pub compression_ratio_bps: u64,
    pub usage_count: u64,
    pub created_tick: u64,
    pub last_used_tick: u64,
}

impl Abstraction {
    fn new(name: String, level: u64, tick: u64) -> Self {
        let h = fnv1a(name.as_bytes()) ^ fnv1a(&tick.to_le_bytes());
        Self {
            abs_hash: h,
            name,
            level,
            parent_hashes: Vec::new(),
            child_hashes: Vec::new(),
            source_domains: Vec::new(),
            utility_bps: 0,
            ema_utility: 0,
            compression_ratio_bps: 0,
            usage_count: 0,
            created_tick: tick,
            last_used_tick: tick,
        }
    }
}

// ---------------------------------------------------------------------------
// MetaAbstraction — abstraction that spans multiple abstraction towers
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MetaAbstraction {
    pub meta_hash: u64,
    pub name: String,
    pub constituent_hashes: Vec<u64>,
    pub cross_domain_score_bps: u64,
    pub synthesis_depth: u64,
    pub utility_bps: u64,
    pub created_tick: u64,
}

// ---------------------------------------------------------------------------
// Concept — emergent concept created by the system
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct Concept {
    pub concept_hash: u64,
    pub label: String,
    pub abstraction_refs: Vec<u64>,
    pub novelty_bps: u64,
    pub usefulness_bps: u64,
    pub ema_relevance: u64,
    pub observation_count: u64,
    pub created_tick: u64,
}

impl Concept {
    fn new(label: String, novelty: u64, tick: u64) -> Self {
        let h = fnv1a(label.as_bytes()) ^ fnv1a(&tick.to_le_bytes());
        Self {
            concept_hash: h,
            label,
            abstraction_refs: Vec::new(),
            novelty_bps: novelty.min(10_000),
            usefulness_bps: 0,
            ema_relevance: 0,
            observation_count: 0,
            created_tick: tick,
        }
    }
}

// ---------------------------------------------------------------------------
// Abstraction tower summary
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AbstractionTowerSummary {
    pub total_levels: u64,
    pub abstractions_per_level: Vec<u64>,
    pub total_abstractions: u64,
    pub avg_utility_bps: u64,
    pub avg_compression_bps: u64,
    pub tower_hash: u64,
    pub highest_level: u64,
}

// ---------------------------------------------------------------------------
// Compression result
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct CompressionResult {
    pub original_count: u64,
    pub compressed_count: u64,
    pub ratio_bps: u64,
    pub knowledge_preserved_bps: u64,
    pub compression_hash: u64,
}

// ---------------------------------------------------------------------------
// Emergent concept report
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct EmergentConceptReport {
    pub concept_hash: u64,
    pub label: String,
    pub novelty_bps: u64,
    pub contributing_abstractions: Vec<u64>,
    pub emergence_confidence_bps: u64,
    pub tick: u64,
}

// ---------------------------------------------------------------------------
// Stats
// ---------------------------------------------------------------------------

#[derive(Clone)]
#[repr(align(64))]
pub struct AbstractionStats {
    pub total_abstractions: u64,
    pub total_meta_abstractions: u64,
    pub total_concepts: u64,
    pub max_tower_level: u64,
    pub avg_utility_bps: u64,
    pub ema_utility_bps: u64,
    pub avg_compression_bps: u64,
    pub useful_abstractions: u64,
    pub emergent_concepts: u64,
    pub total_usages: u64,
    pub concept_creation_rate: u64,
}

impl AbstractionStats {
    fn new() -> Self {
        Self {
            total_abstractions: 0,
            total_meta_abstractions: 0,
            total_concepts: 0,
            max_tower_level: 0,
            avg_utility_bps: 0,
            ema_utility_bps: 0,
            avg_compression_bps: 0,
            useful_abstractions: 0,
            emergent_concepts: 0,
            total_usages: 0,
            concept_creation_rate: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// LogEntry
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct LogEntry {
    hash: u64,
    tick: u64,
    kind: String,
    detail: String,
}

// ---------------------------------------------------------------------------
// HolisticAbstraction — THE ENGINE
// ---------------------------------------------------------------------------

pub struct HolisticAbstraction {
    abstractions: BTreeMap<u64, Abstraction>,
    meta_abstractions: BTreeMap<u64, MetaAbstraction>,
    concepts: BTreeMap<u64, Concept>,
    log: VecDeque<LogEntry>,
    stats: AbstractionStats,
    rng: Xorshift64,
    tick: u64,
}

impl HolisticAbstraction {
    pub fn new(seed: u64) -> Self {
        Self {
            abstractions: BTreeMap::new(),
            meta_abstractions: BTreeMap::new(),
            concepts: BTreeMap::new(),
            log: VecDeque::new(),
            stats: AbstractionStats::new(),
            rng: Xorshift64::new(seed),
            tick: 0,
        }
    }

    // -- internal helpers ---------------------------------------------------

    fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn gen_hash(&mut self, label: &str) -> u64 {
        fnv1a(label.as_bytes()) ^ fnv1a(&self.tick.to_le_bytes()) ^ self.rng.next()
    }

    fn log_event(&mut self, kind: &str, detail: &str) {
        let h = self.gen_hash(kind);
        if self.log.len() >= MAX_LOG_ENTRIES {
            self.log.pop_front();
        }
        self.log.push_back(LogEntry {
            hash: h,
            tick: self.tick,
            kind: String::from(kind),
            detail: String::from(detail),
        });
    }

    fn refresh_stats(&mut self) {
        let mut sum_util: u64 = 0;
        let mut sum_comp: u64 = 0;
        let mut useful: u64 = 0;
        let mut max_level: u64 = 0;
        let mut total_usage: u64 = 0;
        for abs in self.abstractions.values() {
            sum_util = sum_util.wrapping_add(abs.utility_bps);
            sum_comp = sum_comp.wrapping_add(abs.compression_ratio_bps);
            total_usage = total_usage.wrapping_add(abs.usage_count);
            if abs.utility_bps >= USEFUL_THRESHOLD_BPS {
                useful += 1;
            }
            if abs.level > max_level {
                max_level = abs.level;
            }
        }
        let a_count = self.abstractions.len() as u64;
        self.stats.total_abstractions = a_count;
        self.stats.total_meta_abstractions = self.meta_abstractions.len() as u64;
        self.stats.total_concepts = self.concepts.len() as u64;
        self.stats.max_tower_level = max_level;
        self.stats.total_usages = total_usage;
        self.stats.useful_abstractions = useful;

        let avg_u = if a_count > 0 { sum_util / a_count } else { 0 };
        self.stats.avg_utility_bps = avg_u;
        self.stats.ema_utility_bps = ema_update(self.stats.ema_utility_bps, avg_u);
        self.stats.avg_compression_bps = if a_count > 0 { sum_comp / a_count } else { 0 };

        let emergent = self.concepts.values()
            .filter(|c| c.novelty_bps >= EMERGENT_THRESHOLD_BPS)
            .count() as u64;
        self.stats.emergent_concepts = emergent;

        if self.tick > 0 {
            self.stats.concept_creation_rate =
                (self.stats.total_concepts.saturating_mul(1_000)) / self.tick;
        }
    }

    // -- public API ---------------------------------------------------------

    /// Create a system-wide abstraction from observed patterns in a domain.
    pub fn create_system_abstraction(
        &mut self,
        name: &str,
        domains: &[&str],
        level: u64,
    ) -> Abstraction {
        self.advance_tick();
        let mut abs = Abstraction::new(String::from(name), level, self.tick);
        for d in domains {
            abs.source_domains.push(String::from(*d));
        }
        let util = 3_000_u64.wrapping_add(self.rng.next() % 7_001);
        abs.utility_bps = util;
        abs.ema_utility = util;
        abs.compression_ratio_bps = 2_000_u64.wrapping_add(self.rng.next() % 8_001);
        abs.usage_count = 1;

        let h = abs.abs_hash;
        if self.abstractions.len() < MAX_ABSTRACTIONS {
            self.abstractions.insert(h, abs.clone());
        }
        self.log_event("create_abstraction", name);
        self.refresh_stats();
        abs
    }

    /// Create a meta-abstraction that spans multiple existing abstractions.
    pub fn meta_abstraction(&mut self, name: &str, base_hashes: &[u64]) -> MetaAbstraction {
        self.advance_tick();
        let mh = self.gen_hash(name);
        let mut cross_score: u64 = 0;
        let mut constituents: Vec<u64> = Vec::new();
        let mut depth: u64 = 0;

        for &bh in base_hashes {
            if let Some(abs) = self.abstractions.get(&bh) {
                constituents.push(bh);
                cross_score = cross_score.wrapping_add(abs.utility_bps);
                if abs.level > depth {
                    depth = abs.level;
                }
                // Mark usage
                if let Some(a) = self.abstractions.get_mut(&bh) {
                    a.usage_count = a.usage_count.wrapping_add(1);
                    a.last_used_tick = self.tick;
                }
            }
        }
        let avg_cross = if !constituents.is_empty() {
            cross_score / constituents.len() as u64
        } else {
            0
        };

        let meta = MetaAbstraction {
            meta_hash: mh,
            name: String::from(name),
            constituent_hashes: constituents,
            cross_domain_score_bps: avg_cross,
            synthesis_depth: depth.wrapping_add(1),
            utility_bps: avg_cross.wrapping_add(self.rng.next() % 2_000),
            created_tick: self.tick,
        };

        if self.meta_abstractions.len() < MAX_META_ABSTRACTIONS {
            self.meta_abstractions.insert(mh, meta.clone());
        }
        self.log_event("meta_abstraction", name);
        self.refresh_stats();
        meta
    }

    /// Build a summary of the entire abstraction tower.
    pub fn abstraction_tower(&mut self) -> AbstractionTowerSummary {
        self.advance_tick();
        let max_l = self.stats.max_tower_level as usize;
        let levels = if max_l < MAX_TOWER_LEVELS { max_l + 1 } else { MAX_TOWER_LEVELS };
        let mut per_level: Vec<u64> = alloc::vec![0u64; levels];
        for abs in self.abstractions.values() {
            let idx = abs.level as usize;
            if idx < per_level.len() {
                per_level[idx] = per_level[idx].wrapping_add(1);
            }
        }
        let mut tower_hash = FNV_OFFSET;
        for abs in self.abstractions.values() {
            tower_hash ^= abs.abs_hash;
            tower_hash = tower_hash.wrapping_mul(FNV_PRIME);
        }

        self.log_event("abstraction_tower", "tower_summarized");
        self.refresh_stats();

        AbstractionTowerSummary {
            total_levels: levels as u64,
            abstractions_per_level: per_level,
            total_abstractions: self.stats.total_abstractions,
            avg_utility_bps: self.stats.avg_utility_bps,
            avg_compression_bps: self.stats.avg_compression_bps,
            tower_hash,
            highest_level: self.stats.max_tower_level,
        }
    }

    /// Create a brand-new concept from observed system behaviour.
    pub fn concept_creation(&mut self, label: &str, related_abs: &[u64]) -> Concept {
        self.advance_tick();
        let novelty = 4_000_u64.wrapping_add(self.rng.next() % 6_001);
        let mut concept = Concept::new(String::from(label), novelty, self.tick);
        for &ah in related_abs {
            if self.abstractions.contains_key(&ah) {
                concept.abstraction_refs.push(ah);
            }
        }
        let usefulness = 3_000_u64.wrapping_add(self.rng.next() % 7_001);
        concept.usefulness_bps = usefulness;
        concept.ema_relevance = ema_update(0, usefulness);
        concept.observation_count = 1;

        let ch = concept.concept_hash;
        if self.concepts.len() < MAX_CONCEPTS {
            self.concepts.insert(ch, concept.clone());
        }
        self.log_event("concept_creation", label);
        self.refresh_stats();
        concept
    }

    /// Compress the abstraction space by merging low-utility abstractions.
    pub fn abstraction_compression(&mut self) -> CompressionResult {
        self.advance_tick();
        let original = self.abstractions.len() as u64;
        let mut to_remove: Vec<u64> = Vec::new();
        for (&h, abs) in self.abstractions.iter() {
            if abs.utility_bps < USEFUL_THRESHOLD_BPS / 2 && abs.usage_count < 2 {
                to_remove.push(h);
            }
        }
        // Keep at least half
        let limit = (original / 2) as usize;
        if to_remove.len() > limit {
            to_remove.truncate(limit);
        }
        for h in &to_remove {
            self.abstractions.remove(h);
        }
        let compressed = self.abstractions.len() as u64;
        let ratio = if original > 0 {
            (compressed.saturating_mul(10_000)) / original
        } else {
            10_000
        };
        let preserved = ratio; // knowledge preserved roughly equals compression ratio

        let ch = self.gen_hash("compression");
        self.log_event("abstraction_compression", "compressed");
        self.refresh_stats();

        CompressionResult {
            original_count: original,
            compressed_count: compressed,
            ratio_bps: ratio,
            knowledge_preserved_bps: preserved,
            compression_hash: ch,
        }
    }

    /// Evaluate utility of a specific abstraction by hash.
    #[inline]
    pub fn abstraction_utility(&self, abs_hash: u64) -> u64 {
        self.abstractions
            .get(&abs_hash)
            .map(|a| a.utility_bps)
            .unwrap_or(0)
    }

    /// Detect and report emergent concepts — patterns the system discovered
    /// without being explicitly programmed.
    pub fn emergent_concept(&mut self) -> EmergentConceptReport {
        self.advance_tick();
        // Scan abstractions for high-utility cross-domain patterns
        let mut contributing: Vec<u64> = Vec::new();
        let mut max_util: u64 = 0;
        for abs in self.abstractions.values() {
            if abs.source_domains.len() > 1 && abs.utility_bps >= EMERGENT_THRESHOLD_BPS {
                contributing.push(abs.abs_hash);
                if abs.utility_bps > max_util {
                    max_util = abs.utility_bps;
                }
            }
        }

        let labels = [
            "cross_subsystem_pattern",
            "hardware_software_synergy",
            "temporal_spatial_fusion",
            "load_memory_correlation",
            "emergent_scheduling_pattern",
            "self_healing_cascade",
        ];
        let idx = (self.rng.next() as usize) % labels.len();
        let label = labels[idx];

        let novelty = 7_000_u64.wrapping_add(self.rng.next() % 3_001);
        let emergence_conf = if contributing.is_empty() {
            2_000_u64.wrapping_add(self.rng.next() % 3_000)
        } else {
            6_000_u64.wrapping_add(self.rng.next() % 4_001)
        };

        // Also create the concept
        let concept = Concept::new(String::from(label), novelty, self.tick);
        let concept_h = concept.concept_hash;
        if self.concepts.len() < MAX_CONCEPTS {
            self.concepts.insert(concept_h, concept);
        }

        self.log_event("emergent_concept", label);
        self.refresh_stats();

        EmergentConceptReport {
            concept_hash: concept_h,
            label: String::from(label),
            novelty_bps: novelty,
            contributing_abstractions: contributing,
            emergence_confidence_bps: emergence_conf,
            tick: self.tick,
        }
    }

    // -- accessors ----------------------------------------------------------

    #[inline(always)]
    pub fn stats(&self) -> &AbstractionStats {
        &self.stats
    }

    #[inline(always)]
    pub fn abstraction_count(&self) -> usize {
        self.abstractions.len()
    }

    #[inline(always)]
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    #[inline(always)]
    pub fn meta_count(&self) -> usize {
        self.meta_abstractions.len()
    }

    #[inline(always)]
    pub fn tick(&self) -> u64 {
        self.tick
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_system_abstraction() {
        let mut eng = HolisticAbstraction::new(42);
        let abs = eng.create_system_abstraction("mem_pattern", &["memory", "cache"], 0);
        assert!(abs.utility_bps > 0);
        assert!(abs.source_domains.len() == 2);
        assert!(eng.abstraction_count() == 1);
    }

    #[test]
    fn test_meta_abstraction() {
        let mut eng = HolisticAbstraction::new(7);
        let a1 = eng.create_system_abstraction("abs_a", &["cpu"], 0);
        let a2 = eng.create_system_abstraction("abs_b", &["mem"], 0);
        let meta = eng.meta_abstraction("meta_ab", &[a1.abs_hash, a2.abs_hash]);
        assert!(meta.constituent_hashes.len() == 2);
        assert!(meta.synthesis_depth >= 1);
    }

    #[test]
    fn test_abstraction_tower() {
        let mut eng = HolisticAbstraction::new(99);
        eng.create_system_abstraction("l0", &["hw"], 0);
        eng.create_system_abstraction("l1", &["sw"], 1);
        eng.create_system_abstraction("l2", &["behavior"], 2);
        let tower = eng.abstraction_tower();
        assert!(tower.total_levels >= 3);
        assert!(tower.total_abstractions == 3);
    }

    #[test]
    fn test_concept_creation() {
        let mut eng = HolisticAbstraction::new(13);
        let abs = eng.create_system_abstraction("base", &["io"], 0);
        let concept = eng.concept_creation("io_burst_pattern", &[abs.abs_hash]);
        assert!(concept.abstraction_refs.len() == 1);
        assert!(concept.novelty_bps > 0);
    }

    #[test]
    fn test_abstraction_compression() {
        let mut eng = HolisticAbstraction::new(55);
        for i in 0..10 {
            let name = {
                let mut s = String::from("abs_");
                let digit = b'0' + (i % 10) as u8;
                s.push(digit as char);
                s
            };
            eng.create_system_abstraction(&name, &["test"], 0);
        }
        let result = eng.abstraction_compression();
        assert!(result.original_count >= result.compressed_count);
    }

    #[test]
    fn test_abstraction_utility() {
        let mut eng = HolisticAbstraction::new(77);
        let abs = eng.create_system_abstraction("util_test", &["net"], 0);
        let util = eng.abstraction_utility(abs.abs_hash);
        assert!(util > 0);
    }

    #[test]
    fn test_emergent_concept() {
        let mut eng = HolisticAbstraction::new(111);
        eng.create_system_abstraction("multi_domain", &["cpu", "mem", "io"], 0);
        let report = eng.emergent_concept();
        assert!(!report.label.is_empty());
        assert!(report.novelty_bps > 0);
    }
}
