// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Knowledge Base — Persistent Knowledge Store for Research
//!
//! A structured, queryable knowledge store for all cooperation research
//! findings. Stores fairness algorithms, trust models, contention resolution
//! strategies, negotiation protocols, and their empirical performance data.
//! Knowledge entries carry provenance (which experiment validated them),
//! currency scores (how recently relevant), and cross-references to related
//! strategies. This is the institutional memory of cooperation research —
//! nothing discovered is ever forgotten or has to be rediscovered.
//!
//! The engine that remembers everything about cooperation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ENTRIES: usize = 1024;
const MAX_TAGS_PER_ENTRY: usize = 16;
const MAX_CROSS_REFS: usize = 32;
const MAX_QUERY_RESULTS: usize = 64;
const CURRENCY_DECAY_RATE: f32 = 0.002;
const CURRENCY_ACCESS_BOOST: f32 = 0.10;
const RELEVANCE_THRESHOLD: f32 = 0.30;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const ARCHIVE_CURRENCY_MIN: f32 = 0.05;
const MAX_FAIRNESS_LIBRARY: usize = 128;
const MAX_TRUST_ARCHIVE: usize = 128;
const TAG_MATCH_WEIGHT: f32 = 0.40;
const CONTENT_MATCH_WEIGHT: f32 = 0.35;
const CURRENCY_MATCH_WEIGHT: f32 = 0.25;

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
// KNOWLEDGE TYPES
// ============================================================================

/// Category of cooperation knowledge
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeCategory {
    FairnessAlgorithm,
    TrustModel,
    ContentionResolution,
    NegotiationProtocol,
    ResourceSharing,
    AuctionMechanism,
    CoalitionFormation,
    ConflictMediation,
}

/// Provenance of a knowledge entry — where it came from
#[derive(Debug, Clone)]
pub struct KnowledgeProvenance {
    pub experiment_id: u64,
    pub hypothesis_id: u64,
    pub validation_tick: u64,
    pub effect_size: f32,
    pub confidence: f32,
    pub replication_count: u32,
}

/// A single knowledge entry in the cooperation knowledge base
#[derive(Debug, Clone)]
pub struct KnowledgeEntry {
    pub id: u64,
    pub category: KnowledgeCategory,
    pub title: String,
    pub description: String,
    pub parameters: Vec<f32>,
    pub performance_score: f32,
    pub tags: Vec<String>,
    pub cross_refs: Vec<u64>,
    pub provenance: KnowledgeProvenance,
    pub currency: f32,
    pub access_count: u64,
    pub created_tick: u64,
    pub last_accessed_tick: u64,
}

/// Result of a knowledge query
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub entry_id: u64,
    pub relevance: f32,
    pub category: KnowledgeCategory,
    pub title: String,
    pub performance_score: f32,
    pub currency: f32,
}

/// Summary of the fairness algorithm library
#[derive(Debug, Clone)]
pub struct FairnessLibraryEntry {
    pub id: u64,
    pub name: String,
    pub fairness_score: f32,
    pub throughput_impact: f32,
    pub parameters: Vec<f32>,
    pub validated: bool,
}

/// Archived trust model entry
#[derive(Debug, Clone)]
pub struct TrustModelEntry {
    pub id: u64,
    pub name: String,
    pub trust_convergence_rate: f32,
    pub stability_score: f32,
    pub decay_factor: f32,
    pub validated: bool,
}

// ============================================================================
// KNOWLEDGE BASE STATS
// ============================================================================

/// Aggregate statistics for the knowledge base
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
pub struct KnowledgeBaseStats {
    pub total_entries: u64,
    pub total_queries: u64,
    pub total_accesses: u64,
    pub avg_currency_ema: f32,
    pub avg_performance_ema: f32,
    pub fairness_entries: u64,
    pub trust_entries: u64,
    pub contention_entries: u64,
    pub archived_entries: u64,
    pub cross_ref_density: f32,
}

// ============================================================================
// COOPERATION KNOWLEDGE BASE
// ============================================================================

/// Persistent knowledge store for cooperation research findings
#[derive(Debug)]
pub struct CoopKnowledgeBase {
    entries: BTreeMap<u64, KnowledgeEntry>,
    category_index: BTreeMap<u64, Vec<u64>>,
    tag_index: BTreeMap<u64, Vec<u64>>,
    fairness_library: Vec<FairnessLibraryEntry>,
    trust_archive: Vec<TrustModelEntry>,
    rng_state: u64,
    tick: u64,
    stats: KnowledgeBaseStats,
}

impl CoopKnowledgeBase {
    /// Create a new cooperation knowledge base with the given PRNG seed
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            category_index: BTreeMap::new(),
            tag_index: BTreeMap::new(),
            fairness_library: Vec::new(),
            trust_archive: Vec::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: KnowledgeBaseStats::default(),
        }
    }

    /// Store a new piece of cooperation knowledge
    pub fn store_cooperation_knowledge(
        &mut self,
        category: KnowledgeCategory,
        title: String,
        description: String,
        parameters: Vec<f32>,
        performance_score: f32,
        tags: Vec<String>,
        provenance: KnowledgeProvenance,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(title.as_bytes()) ^ fnv1a_hash(&self.tick.to_le_bytes());
        let entry = KnowledgeEntry {
            id,
            category,
            title: title.clone(),
            description,
            parameters: parameters.clone(),
            performance_score,
            tags: if tags.len() > MAX_TAGS_PER_ENTRY {
                tags[..MAX_TAGS_PER_ENTRY].to_vec()
            } else {
                tags.clone()
            },
            cross_refs: Vec::new(),
            provenance,
            currency: 1.0,
            access_count: 0,
            created_tick: self.tick,
            last_accessed_tick: self.tick,
        };

        // Update category index
        let cat_key = category as u64;
        let cat_list = self.category_index.entry(cat_key).or_insert_with(Vec::new);
        cat_list.push(id);

        // Update tag index
        for tag in &tags {
            let tag_hash = fnv1a_hash(tag.as_bytes());
            let tag_list = self.tag_index.entry(tag_hash).or_insert_with(Vec::new);
            tag_list.push(id);
        }

        // Add to specialized libraries
        match category {
            KnowledgeCategory::FairnessAlgorithm => {
                if self.fairness_library.len() < MAX_FAIRNESS_LIBRARY {
                    self.fairness_library.push(FairnessLibraryEntry {
                        id,
                        name: title,
                        fairness_score: performance_score,
                        throughput_impact: if parameters.len() > 1 { parameters[1] } else { 0.0 },
                        parameters,
                        validated: provenance.replication_count > 0,
                    });
                }
                self.stats.fairness_entries += 1;
            }
            KnowledgeCategory::TrustModel => {
                if self.trust_archive.len() < MAX_TRUST_ARCHIVE {
                    self.trust_archive.push(TrustModelEntry {
                        id,
                        name: title,
                        trust_convergence_rate: if !parameters.is_empty() { parameters[0] } else { 0.5 },
                        stability_score: performance_score,
                        decay_factor: if parameters.len() > 2 { parameters[2] } else { 0.01 },
                        validated: provenance.replication_count > 0,
                    });
                }
                self.stats.trust_entries += 1;
            }
            KnowledgeCategory::ContentionResolution => {
                self.stats.contention_entries += 1;
            }
            _ => {}
        }

        // Evict old entries if at capacity
        if self.entries.len() >= MAX_ENTRIES {
            self.evict_lowest_currency();
        }
        self.entries.insert(id, entry);
        self.stats.total_entries = self.entries.len() as u64;
        self.update_cross_ref_density();
        id
    }

    /// Query the knowledge base for strategies matching a description
    pub fn query_strategy(&mut self, query: &str, category: Option<KnowledgeCategory>) -> Vec<QueryResult> {
        self.tick += 1;
        self.stats.total_queries += 1;
        let query_hash = fnv1a_hash(query.as_bytes());
        let query_tokens: Vec<u64> = query
            .split_whitespace()
            .map(|w| fnv1a_hash(w.as_bytes()))
            .collect();

        let mut results: Vec<QueryResult> = Vec::new();

        for entry in self.entries.values() {
            if let Some(cat) = category {
                if entry.category != cat {
                    continue;
                }
            }
            let relevance = self.compute_relevance(entry, &query_tokens, query_hash);
            if relevance >= RELEVANCE_THRESHOLD {
                results.push(QueryResult {
                    entry_id: entry.id,
                    relevance,
                    category: entry.category,
                    title: entry.title.clone(),
                    performance_score: entry.performance_score,
                    currency: entry.currency,
                });
            }
        }
        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(core::cmp::Ordering::Equal));
        if results.len() > MAX_QUERY_RESULTS {
            results.truncate(MAX_QUERY_RESULTS);
        }

        // Boost currency of accessed entries
        for result in &results {
            if let Some(entry) = self.entries.get_mut(&result.entry_id) {
                entry.currency = (entry.currency + CURRENCY_ACCESS_BOOST).min(1.0);
                entry.access_count += 1;
                entry.last_accessed_tick = self.tick;
                self.stats.total_accesses += 1;
            }
        }
        results
    }

    /// Get the fairness algorithm library
    #[inline(always)]
    pub fn fairness_library(&self) -> &[FairnessLibraryEntry] {
        &self.fairness_library
    }

    /// Get the trust model archive
    #[inline(always)]
    pub fn trust_model_archive(&self) -> &[TrustModelEntry] {
        &self.trust_archive
    }

    /// Decay currency of all entries and return average currency
    #[inline]
    pub fn knowledge_currency(&mut self) -> f32 {
        self.tick += 1;
        let mut total_currency = 0.0f32;
        let mut count = 0u32;
        let mut to_archive: Vec<u64> = Vec::new();

        for entry in self.entries.values_mut() {
            entry.currency = (entry.currency - CURRENCY_DECAY_RATE).max(0.0);
            total_currency += entry.currency;
            count += 1;
            if entry.currency < ARCHIVE_CURRENCY_MIN {
                to_archive.push(entry.id);
            }
        }
        self.stats.archived_entries += to_archive.len() as u64;
        let avg = if count > 0 { total_currency / count as f32 } else { 0.0 };
        self.stats.avg_currency_ema = EMA_ALPHA * avg + (1.0 - EMA_ALPHA) * self.stats.avg_currency_ema;
        avg
    }

    /// Get the total size of the knowledge base
    #[inline(always)]
    pub fn knowledge_size(&self) -> usize {
        self.entries.len()
    }

    /// Get current knowledge base statistics
    #[inline(always)]
    pub fn stats(&self) -> &KnowledgeBaseStats {
        &self.stats
    }

    /// Add a cross-reference between two knowledge entries
    pub fn add_cross_ref(&mut self, from_id: u64, to_id: u64) -> bool {
        if !self.entries.contains_key(&to_id) {
            return false;
        }
        if let Some(entry) = self.entries.get_mut(&from_id) {
            if entry.cross_refs.len() < MAX_CROSS_REFS && !entry.cross_refs.contains(&to_id) {
                entry.cross_refs.push(to_id);
                self.update_cross_ref_density();
                return true;
            }
        }
        false
    }

    /// Get entries in a specific category
    #[inline]
    pub fn category_entries(&self, category: KnowledgeCategory) -> Vec<&KnowledgeEntry> {
        let cat_key = category as u64;
        match self.category_index.get(&cat_key) {
            Some(ids) => ids
                .iter()
                .filter_map(|id| self.entries.get(id))
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get the top-performing entries across all categories
    #[inline]
    pub fn top_performers(&self, limit: usize) -> Vec<&KnowledgeEntry> {
        let mut sorted: Vec<&KnowledgeEntry> = self.entries.values().collect();
        sorted.sort_by(|a, b| {
            b.performance_score
                .partial_cmp(&a.performance_score)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        sorted.truncate(limit);
        sorted
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn compute_relevance(&self, entry: &KnowledgeEntry, query_tokens: &[u64], query_hash: u64) -> f32 {
        // Tag matching
        let tag_score = if query_tokens.is_empty() {
            0.0
        } else {
            let mut matches = 0u32;
            for tag in &entry.tags {
                let tag_hash = fnv1a_hash(tag.as_bytes());
                for &qt in query_tokens {
                    if tag_hash == qt {
                        matches += 1;
                    }
                }
            }
            matches as f32 / query_tokens.len().max(1) as f32
        };

        // Content hash similarity (approximate)
        let title_hash = fnv1a_hash(entry.title.as_bytes());
        let hash_dist = (title_hash ^ query_hash).count_ones() as f32 / 64.0;
        let content_score = 1.0 - hash_dist;

        // Currency factor
        let currency_score = entry.currency;

        TAG_MATCH_WEIGHT * tag_score
            + CONTENT_MATCH_WEIGHT * content_score
            + CURRENCY_MATCH_WEIGHT * currency_score
    }

    fn evict_lowest_currency(&mut self) {
        let mut lowest_id: Option<u64> = None;
        let mut lowest_currency = f32::MAX;
        for (id, entry) in &self.entries {
            if entry.currency < lowest_currency {
                lowest_currency = entry.currency;
                lowest_id = Some(*id);
            }
        }
        if let Some(id) = lowest_id {
            self.entries.remove(&id);
            self.stats.archived_entries += 1;
        }
    }

    fn update_cross_ref_density(&mut self) {
        let total_entries = self.entries.len() as f32;
        if total_entries < 2.0 {
            self.stats.cross_ref_density = 0.0;
            return;
        }
        let total_refs: usize = self.entries.values().map(|e| e.cross_refs.len()).sum();
        let max_possible = total_entries * (total_entries - 1.0);
        self.stats.cross_ref_density = if max_possible > 0.0 {
            total_refs as f32 / max_possible
        } else {
            0.0
        };
    }
}
