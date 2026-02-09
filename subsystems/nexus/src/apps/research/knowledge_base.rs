// SPDX-License-Identifier: GPL-2.0
//! # Apps Knowledge Base — Persistent Knowledge Store for App Research
//!
//! Stores app behavior patterns, classification rules, optimization strategies,
//! and validated findings from the research pipeline. The knowledge base is
//! queryable by topic, pattern type, or relevance ranking. Periodic maintenance
//! prunes stale entries and promotes high-confidence knowledge to the front.
//!
//! The engine that remembers everything the research pipeline has learned.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ENTRIES: usize = 2048;
const MAX_PATTERNS: usize = 512;
const MAX_QUERY_RESULTS: usize = 32;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const RELEVANCE_DECAY: f32 = 0.999;
const STALE_THRESHOLD: u64 = 10000;
const HIGH_CONFIDENCE: f32 = 0.80;
const MAINTENANCE_INTERVAL: u64 = 500;
const MIN_ACCESS_FOR_KEEP: u32 = 2;
const COMPLETENESS_CATEGORIES: usize = 8;

// ============================================================================
// HELPERS
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
// TYPES
// ============================================================================

/// Category of knowledge entry.
#[derive(Clone, Copy, PartialEq)]
pub enum KnowledgeCategory {
    BehaviorPattern,
    ClassificationRule,
    OptimizationStrategy,
    PerformanceInsight,
    AnomalySignature,
    ResourceModel,
    PredictionFeature,
    GeneralInsight,
}

/// A single knowledge entry stored in the base.
#[derive(Clone)]
pub struct KnowledgeEntry {
    pub entry_id: u64,
    pub title: String,
    pub category: KnowledgeCategory,
    pub content_hash: u64,
    pub confidence: f32,
    pub relevance_score: f32,
    pub access_count: u32,
    pub created_tick: u64,
    pub last_accessed_tick: u64,
    pub tags: Vec<String>,
    pub source_finding_id: u64,
    pub validated: bool,
}

/// A stored pattern in the pattern library.
#[derive(Clone)]
pub struct BehaviorPattern {
    pub pattern_id: u64,
    pub name: String,
    pub signature_hash: u64,
    pub frequency: u32,
    pub confidence: f32,
    pub associated_entries: Vec<u64>,
    pub first_seen: u64,
    pub last_seen: u64,
}

/// Result of a knowledge query.
#[derive(Clone)]
pub struct QueryResult {
    pub entries: Vec<KnowledgeEntry>,
    pub total_matches: usize,
    pub avg_relevance: f32,
    pub avg_confidence: f32,
}

/// Relevance ranking for a query.
#[derive(Clone)]
pub struct RelevanceRanking {
    pub entry_id: u64,
    pub score: f32,
    pub tag_match_count: u32,
    pub recency_bonus: f32,
    pub access_bonus: f32,
}

/// Knowledge completeness report.
#[derive(Clone)]
pub struct CompletenessReport {
    pub total_entries: usize,
    pub category_coverage: BTreeMap<u8, usize>,
    pub validated_ratio: f32,
    pub avg_confidence: f32,
    pub stale_count: usize,
    pub completeness_score: f32,
}

/// Engine-level stats.
#[derive(Clone)]
pub struct KnowledgeStats {
    pub entries_stored: u64,
    pub queries_served: u64,
    pub patterns_cataloged: u64,
    pub maintenance_runs: u64,
    pub ema_relevance: f32,
    pub ema_confidence: f32,
    pub ema_access_rate: f32,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Persistent knowledge store for app research.
pub struct AppsKnowledgeBase {
    entries: BTreeMap<u64, KnowledgeEntry>,
    patterns: BTreeMap<u64, BehaviorPattern>,
    tag_index: BTreeMap<String, Vec<u64>>,
    category_index: BTreeMap<u8, Vec<u64>>,
    stats: KnowledgeStats,
    rng_state: u64,
    tick: u64,
    last_maintenance: u64,
}

impl AppsKnowledgeBase {
    /// Create a new knowledge base.
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            patterns: BTreeMap::new(),
            tag_index: BTreeMap::new(),
            category_index: BTreeMap::new(),
            stats: KnowledgeStats {
                entries_stored: 0,
                queries_served: 0,
                patterns_cataloged: 0,
                maintenance_runs: 0,
                ema_relevance: 0.0,
                ema_confidence: 0.0,
                ema_access_rate: 0.0,
            },
            rng_state: seed ^ 0x4f8a2b6dc103e975,
            tick: 0,
            last_maintenance: 0,
        }
    }

    // ── Primary API ────────────────────────────────────────────────────

    /// Store a validated finding as a knowledge entry.
    pub fn store_finding(
        &mut self,
        title: &str,
        category: KnowledgeCategory,
        confidence: f32,
        tags: &[&str],
        source_id: u64,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(title.as_bytes()) ^ self.tick;
        self.stats.entries_stored += 1;

        let tag_vec: Vec<String> = tags.iter().map(|t| String::from(*t)).collect();

        let entry = KnowledgeEntry {
            entry_id: id,
            title: String::from(title),
            category,
            content_hash: fnv1a_hash(&id.to_le_bytes()),
            confidence: confidence.min(1.0).max(0.0),
            relevance_score: 1.0,
            access_count: 0,
            created_tick: self.tick,
            last_accessed_tick: self.tick,
            tags: tag_vec.clone(),
            source_finding_id: source_id,
            validated: confidence >= HIGH_CONFIDENCE,
        };

        // Update tag index
        for tag in &tag_vec {
            let list = self.tag_index.entry(tag.clone()).or_insert_with(Vec::new);
            list.push(id);
            if list.len() > MAX_ENTRIES {
                list.remove(0);
            }
        }

        // Update category index
        let cat_key = category as u8;
        let cat_list = self.category_index.entry(cat_key).or_insert_with(Vec::new);
        cat_list.push(id);
        if cat_list.len() > MAX_ENTRIES {
            cat_list.remove(0);
        }

        self.stats.ema_confidence =
            EMA_ALPHA * confidence + (1.0 - EMA_ALPHA) * self.stats.ema_confidence;

        // Evict if over capacity
        if self.entries.len() >= MAX_ENTRIES {
            self.evict_lowest_relevance();
        }
        self.entries.insert(id, entry);

        // Auto-maintenance check
        if self.tick - self.last_maintenance >= MAINTENANCE_INTERVAL {
            self.run_maintenance();
        }

        id
    }

    /// Query knowledge base by tags.
    pub fn query_knowledge(&mut self, tags: &[&str], max_results: usize) -> QueryResult {
        self.tick += 1;
        self.stats.queries_served += 1;

        let limit = max_results.min(MAX_QUERY_RESULTS);
        let mut candidate_scores: BTreeMap<u64, f32> = BTreeMap::new();

        // Score entries by tag overlap
        for tag in tags {
            let tag_str = String::from(*tag);
            if let Some(ids) = self.tag_index.get(&tag_str) {
                for &eid in ids {
                    let score = candidate_scores.entry(eid).or_insert(0.0);
                    *score += 1.0;
                }
            }
        }

        // Also score by content hash similarity for broad matching
        let query_hash = fnv1a_hash(tags.iter().flat_map(|t| t.as_bytes()).copied().collect::<Vec<u8>>().as_slice());
        for (eid, entry) in self.entries.iter() {
            let hash_sim = 1.0 - ((entry.content_hash ^ query_hash) % 1000) as f32 / 1000.0;
            let score = candidate_scores.entry(*eid).or_insert(0.0);
            *score += hash_sim * 0.3;
        }

        // Rank and select top results
        let mut ranked: Vec<(u64, f32)> = candidate_scores.into_iter().collect();
        for i in 0..ranked.len() {
            for j in (i + 1)..ranked.len() {
                if ranked[j].1 > ranked[i].1 {
                    ranked.swap(i, j);
                }
            }
        }
        ranked.truncate(limit);

        let mut result_entries = Vec::new();
        let mut total_rel = 0.0f32;
        let mut total_conf = 0.0f32;

        for (eid, _score) in &ranked {
            if let Some(entry) = self.entries.get_mut(eid) {
                entry.access_count += 1;
                entry.last_accessed_tick = self.tick;
                total_rel += entry.relevance_score;
                total_conf += entry.confidence;
                result_entries.push(entry.clone());
            }
        }

        let n = result_entries.len().max(1) as f32;
        let avg_rel = total_rel / n;
        let avg_conf = total_conf / n;

        self.stats.ema_relevance = EMA_ALPHA * avg_rel + (1.0 - EMA_ALPHA) * self.stats.ema_relevance;
        let access_rate = self.stats.queries_served as f32 / self.tick.max(1) as f32;
        self.stats.ema_access_rate =
            EMA_ALPHA * access_rate + (1.0 - EMA_ALPHA) * self.stats.ema_access_rate;

        QueryResult {
            total_matches: result_entries.len(),
            entries: result_entries,
            avg_relevance: avg_rel,
            avg_confidence: avg_conf,
        }
    }

    /// Access the pattern library — returns all cataloged behavior patterns.
    pub fn pattern_library(&self) -> Vec<BehaviorPattern> {
        self.patterns.values().cloned().collect()
    }

    /// Register a new behavior pattern.
    pub fn register_pattern(&mut self, name: &str, associated_entries: &[u64]) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ self.tick;
        self.stats.patterns_cataloged += 1;

        let pattern = BehaviorPattern {
            pattern_id: id,
            name: String::from(name),
            signature_hash: fnv1a_hash(&id.to_le_bytes()),
            frequency: 1,
            confidence: 0.5,
            associated_entries: Vec::from(associated_entries),
            first_seen: self.tick,
            last_seen: self.tick,
        };

        if self.patterns.len() >= MAX_PATTERNS {
            let mut min_id = 0u64;
            let mut min_freq = u32::MAX;
            for (pid, p) in self.patterns.iter() {
                if p.frequency < min_freq {
                    min_freq = p.frequency;
                    min_id = *pid;
                }
            }
            self.patterns.remove(&min_id);
        }
        self.patterns.insert(id, pattern);
        id
    }

    /// Compute relevance ranking for all entries matching given tags.
    pub fn relevance_ranking(&self, tags: &[&str]) -> Vec<RelevanceRanking> {
        let mut rankings = Vec::new();
        let current = self.tick;

        for (eid, entry) in self.entries.iter() {
            let mut tag_match = 0u32;
            for tag in tags {
                let tag_s = String::from(*tag);
                for et in &entry.tags {
                    if *et == tag_s {
                        tag_match += 1;
                    }
                }
            }

            if tag_match == 0 {
                continue;
            }

            let recency = if current > entry.last_accessed_tick {
                let age = current - entry.last_accessed_tick;
                1.0 / (1.0 + age as f32 * 0.001)
            } else {
                1.0
            };

            let access_bonus = (entry.access_count as f32).min(20.0) / 20.0;
            let score = tag_match as f32 * 0.4 + entry.confidence * 0.3 + recency * 0.2 + access_bonus * 0.1;

            rankings.push(RelevanceRanking {
                entry_id: *eid,
                score,
                tag_match_count: tag_match,
                recency_bonus: recency,
                access_bonus,
            });
        }

        // Sort descending
        for i in 0..rankings.len() {
            for j in (i + 1)..rankings.len() {
                if rankings[j].score > rankings[i].score {
                    rankings.swap(i, j);
                }
            }
        }
        rankings
    }

    /// Run knowledge base maintenance — prune stale, decay relevance.
    pub fn knowledge_maintenance(&mut self) -> usize {
        self.stats.maintenance_runs += 1;
        self.last_maintenance = self.tick;
        self.run_maintenance()
    }

    /// Compute knowledge completeness report.
    pub fn knowledge_completeness(&self) -> CompletenessReport {
        let total = self.entries.len();
        let mut category_cov: BTreeMap<u8, usize> = BTreeMap::new();
        let mut validated = 0usize;
        let mut conf_sum = 0.0f32;
        let mut stale = 0usize;

        for entry in self.entries.values() {
            let cat = entry.category as u8;
            *category_cov.entry(cat).or_insert(0) += 1;
            if entry.validated {
                validated += 1;
            }
            conf_sum += entry.confidence;
            if self.tick > entry.last_accessed_tick
                && (self.tick - entry.last_accessed_tick) > STALE_THRESHOLD
            {
                stale += 1;
            }
        }

        let n = total.max(1) as f32;
        let validated_ratio = validated as f32 / n;
        let avg_conf = conf_sum / n;

        // Completeness: coverage of categories × validated ratio × freshness
        let cat_coverage = category_cov.len() as f32 / COMPLETENESS_CATEGORIES as f32;
        let freshness = 1.0 - (stale as f32 / n);
        let completeness = cat_coverage * 0.3 + validated_ratio * 0.4 + freshness * 0.3;

        CompletenessReport {
            total_entries: total,
            category_coverage: category_cov,
            validated_ratio,
            avg_confidence: avg_conf,
            stale_count: stale,
            completeness_score: completeness.min(1.0),
        }
    }

    /// Return engine stats.
    pub fn stats(&self) -> &KnowledgeStats {
        &self.stats
    }

    // ── Internal Helpers ───────────────────────────────────────────────

    fn run_maintenance(&mut self) -> usize {
        let mut to_remove = Vec::new();
        let current = self.tick;

        for (eid, entry) in self.entries.iter_mut() {
            // Decay relevance
            entry.relevance_score *= RELEVANCE_DECAY;

            // Mark stale entries for removal if low access
            if current > entry.last_accessed_tick {
                let age = current - entry.last_accessed_tick;
                if age > STALE_THRESHOLD && entry.access_count < MIN_ACCESS_FOR_KEEP && !entry.validated {
                    to_remove.push(*eid);
                }
            }
        }

        for eid in &to_remove {
            self.entries.remove(eid);
        }
        to_remove.len()
    }

    fn evict_lowest_relevance(&mut self) {
        let mut min_id = 0u64;
        let mut min_rel = f32::MAX;
        for (eid, entry) in self.entries.iter() {
            if entry.relevance_score < min_rel {
                min_rel = entry.relevance_score;
                min_id = *eid;
            }
        }
        if min_id != 0 {
            self.entries.remove(&min_id);
        }
    }
}
