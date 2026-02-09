// SPDX-License-Identifier: GPL-2.0
//! # Bridge Knowledge Base — Persistent Research Knowledge Store
//!
//! All confirmed research findings, optimizations, causal links and validated
//! hypotheses are stored here. The knowledge base supports relevance-based
//! retrieval using TF-IDF-inspired scoring, category-filtered queries,
//! knowledge graph traversal for causal chains, and time-based decay to
//! ensure stale knowledge is gradually deprioritised.
//!
//! The bridge's long-term memory. Everything it learns lives here.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ENTRIES: usize = 4096;
const MAX_CATEGORIES: usize = 128;
const MAX_LINKS: usize = 8192;
const MAX_QUERY_RESULTS: usize = 32;
const DECAY_RATE: f32 = 0.995;
const USE_BOOST: f32 = 0.02;
const RELEVANCE_THRESHOLD: f32 = 0.1;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const MAX_TOKENS_PER_ENTRY: usize = 64;
const LINK_WEIGHT_DEFAULT: f32 = 0.5;
const HIGH_CONFIDENCE: f32 = 0.85;
const MIN_CONFIDENCE: f32 = 0.1;

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

fn sqrt_approx(v: f32) -> f32 {
    if v <= 0.0 {
        return 0.0;
    }
    let mut g = v * 0.5;
    for _ in 0..6 {
        g = 0.5 * (g + v / g);
    }
    g
}

fn ln_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return -10.0;
    }
    let y = (x - 1.0) / (x + 1.0);
    let y2 = y * y;
    2.0 * y * (1.0 + y2 / 3.0 + y2 * y2 / 5.0 + y2 * y2 * y2 / 7.0)
}

/// Simple tokenizer: split on non-alphanumeric, lowercase-ish via masking.
fn tokenize(text: &str) -> Vec<u64> {
    let mut tokens = Vec::new();
    let mut current = Vec::new();
    for &b in text.as_bytes() {
        if b.is_ascii_alphanumeric() || b == b'_' {
            current.push(b | 0x20); // poor-man's lowercase
        } else if !current.is_empty() {
            tokens.push(fnv1a_hash(&current));
            current.clear();
            if tokens.len() >= MAX_TOKENS_PER_ENTRY {
                break;
            }
        }
    }
    if !current.is_empty() && tokens.len() < MAX_TOKENS_PER_ENTRY {
        tokens.push(fnv1a_hash(&current));
    }
    tokens
}

// ============================================================================
// TYPES
// ============================================================================

/// A single knowledge entry.
#[derive(Clone)]
pub struct KnowledgeEntry {
    pub id: u64,
    pub category: String,
    pub content: String,
    pub confidence: f32,
    pub times_used: u64,
    pub created_tick: u64,
    pub last_used_tick: u64,
    pub relevance_score: f32,
    tokens: Vec<u64>,
    category_id: u64,
}

/// A causal link between two knowledge entries.
#[derive(Clone)]
struct KnowledgeLink {
    source_id: u64,
    target_id: u64,
    relationship: String,
    weight: f32,
    created_tick: u64,
}

/// Category metadata.
#[derive(Clone)]
struct Category {
    id: u64,
    name: String,
    entry_count: u64,
    avg_confidence_ema: f32,
}

/// Knowledge base statistics.
#[derive(Clone)]
pub struct KnowledgeStats {
    pub total_entries: u64,
    pub total_queries: u64,
    pub total_links: u64,
    pub avg_confidence_ema: f32,
    pub avg_relevance_ema: f32,
    pub total_uses: u64,
    pub high_confidence_count: u64,
    pub decayed_entries: u64,
    pub category_count: usize,
    pub knowledge_utilization: f32,
}

/// Result of a knowledge graph traversal.
#[derive(Clone)]
pub struct GraphPath {
    pub entry_ids: Vec<u64>,
    pub relationships: Vec<String>,
    pub total_weight: f32,
    pub path_length: usize,
}

/// Query result.
#[derive(Clone)]
pub struct QueryResult {
    pub entry_id: u64,
    pub category: String,
    pub content: String,
    pub relevance: f32,
    pub confidence: f32,
    pub times_used: u64,
}

// ============================================================================
// BRIDGE KNOWLEDGE BASE
// ============================================================================

/// Persistent knowledge store for bridge research.
pub struct BridgeKnowledgeBase {
    entries: BTreeMap<u64, KnowledgeEntry>,
    links: Vec<KnowledgeLink>,
    categories: BTreeMap<u64, Category>,
    /// Inverted index: token_hash -> list of entry ids
    inverted_index: BTreeMap<u64, Vec<u64>>,
    stats: KnowledgeStats,
    rng_state: u64,
    tick: u64,
    total_documents: u64,
}

impl BridgeKnowledgeBase {
    /// Create a new knowledge base.
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            links: Vec::new(),
            categories: BTreeMap::new(),
            inverted_index: BTreeMap::new(),
            stats: KnowledgeStats {
                total_entries: 0,
                total_queries: 0,
                total_links: 0,
                avg_confidence_ema: 0.5,
                avg_relevance_ema: 0.5,
                total_uses: 0,
                high_confidence_count: 0,
                decayed_entries: 0,
                category_count: 0,
                knowledge_utilization: 0.0,
            },
            rng_state: seed ^ 0xA0B1ED6EBA5E01,
            tick: 0,
            total_documents: 0,
        }
    }

    /// Store a new knowledge entry.
    pub fn store_knowledge(
        &mut self,
        category: &str,
        content: &str,
        confidence: f32,
    ) -> u64 {
        self.tick += 1;
        let clamped_conf = confidence.max(MIN_CONFIDENCE).min(1.0);
        let id = fnv1a_hash(content.as_bytes()) ^ self.tick;
        let cat_id = fnv1a_hash(category.as_bytes());
        let tokens = tokenize(content);

        // Evict if at capacity (lowest confidence entry)
        if self.entries.len() >= MAX_ENTRIES {
            self.evict_lowest_value();
        }

        let entry = KnowledgeEntry {
            id,
            category: String::from(category),
            content: String::from(content),
            confidence: clamped_conf,
            times_used: 0,
            created_tick: self.tick,
            last_used_tick: self.tick,
            relevance_score: clamped_conf,
            tokens: tokens.clone(),
            category_id: cat_id,
        };

        self.entries.insert(id, entry);

        // Update inverted index
        for &token in &tokens {
            self.inverted_index
                .entry(token)
                .or_insert_with(Vec::new)
                .push(id);
        }

        // Update category
        self.ensure_category(category, cat_id);
        if let Some(cat) = self.categories.get_mut(&cat_id) {
            cat.entry_count += 1;
            cat.avg_confidence_ema =
                cat.avg_confidence_ema * (1.0 - EMA_ALPHA) + clamped_conf * EMA_ALPHA;
        }

        self.stats.total_entries += 1;
        self.total_documents += 1;
        if clamped_conf >= HIGH_CONFIDENCE {
            self.stats.high_confidence_count += 1;
        }
        self.stats.avg_confidence_ema =
            self.stats.avg_confidence_ema * (1.0 - EMA_ALPHA) + clamped_conf * EMA_ALPHA;
        self.stats.category_count = self.categories.len();

        id
    }

    /// Query the knowledge base with a text query, returning ranked results.
    pub fn query_knowledge(&mut self, query: &str) -> Vec<QueryResult> {
        self.tick += 1;
        self.stats.total_queries += 1;

        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        // TF-IDF-inspired scoring
        let mut scores: BTreeMap<u64, f32> = BTreeMap::new();
        let n_docs = self.entries.len().max(1) as f32;

        for &qt in &query_tokens {
            let doc_freq = self
                .inverted_index
                .get(&qt)
                .map(|v| v.len())
                .unwrap_or(0) as f32;
            let idf = if doc_freq > 0.0 {
                ln_approx(n_docs / doc_freq) + 1.0
            } else {
                0.0
            };

            if let Some(entry_ids) = self.inverted_index.get(&qt) {
                for &eid in entry_ids {
                    if let Some(entry) = self.entries.get(&eid) {
                        // TF: count of token in entry
                        let tf = entry.tokens.iter().filter(|&&t| t == qt).count() as f32;
                        let tf_norm = tf / entry.tokens.len().max(1) as f32;
                        let tfidf = tf_norm * idf;
                        // Boost by confidence and recency
                        let recency = 1.0
                            / (1.0 + (self.tick - entry.last_used_tick) as f32 * 0.001);
                        let boost = entry.confidence * 0.3 + recency * 0.2;
                        *scores.entry(eid).or_insert(0.0) += tfidf + boost;
                    }
                }
            }
        }

        // Rank and return top results
        let mut ranked: Vec<(u64, f32)> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        ranked.truncate(MAX_QUERY_RESULTS);

        let mut results = Vec::new();
        for (eid, relevance) in ranked {
            if relevance < RELEVANCE_THRESHOLD {
                continue;
            }
            if let Some(entry) = self.entries.get_mut(&eid) {
                entry.times_used += 1;
                entry.last_used_tick = self.tick;
                entry.relevance_score =
                    entry.relevance_score * (1.0 - EMA_ALPHA) + relevance * EMA_ALPHA;
                self.stats.total_uses += 1;
                results.push(QueryResult {
                    entry_id: eid,
                    category: entry.category.clone(),
                    content: entry.content.clone(),
                    relevance,
                    confidence: entry.confidence,
                    times_used: entry.times_used,
                });
            }
        }

        self.stats.avg_relevance_ema = if !results.is_empty() {
            let avg_rel = results.iter().map(|r| r.relevance).sum::<f32>()
                / results.len() as f32;
            self.stats.avg_relevance_ema * (1.0 - EMA_ALPHA) + avg_rel * EMA_ALPHA
        } else {
            self.stats.avg_relevance_ema * (1.0 - EMA_ALPHA)
        };

        self.update_utilization();
        results
    }

    /// Build a knowledge graph traversal from a source entry.
    pub fn knowledge_graph(&self, source_id: u64, max_depth: usize) -> Vec<GraphPath> {
        let mut paths = Vec::new();
        let mut visited: Vec<u64> = Vec::new();
        self.traverse(source_id, &mut visited, &mut Vec::new(), &mut Vec::new(), 0.0, max_depth, &mut paths);
        paths
    }

    /// Add a causal link between two entries.
    pub fn add_link(&mut self, source_id: u64, target_id: u64, relationship: &str) {
        if self.links.len() >= MAX_LINKS {
            return;
        }
        if self.entries.contains_key(&source_id) && self.entries.contains_key(&target_id) {
            self.links.push(KnowledgeLink {
                source_id,
                target_id,
                relationship: String::from(relationship),
                weight: LINK_WEIGHT_DEFAULT,
                created_tick: self.tick,
            });
            self.stats.total_links += 1;
        }
    }

    /// Search by relevance within a specific category.
    pub fn relevance_search(&mut self, query: &str, category: &str) -> Vec<QueryResult> {
        let cat_id = fnv1a_hash(category.as_bytes());
        let all_results = self.query_knowledge(query);
        all_results
            .into_iter()
            .filter(|r| {
                self.entries
                    .get(&r.entry_id)
                    .map(|e| e.category_id == cat_id)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Apply time-based decay to all knowledge entries.
    pub fn knowledge_decay(&mut self) {
        self.tick += 1;
        let mut decayed = 0u64;
        for entry in self.entries.values_mut() {
            let age = (self.tick - entry.last_used_tick) as f32;
            let decay_factor = DECAY_RATE;
            // Entries used more often decay slower
            let use_protection = (entry.times_used as f32 * USE_BOOST).min(0.3);
            let effective_decay = decay_factor + use_protection * (1.0 - decay_factor);
            entry.confidence *= effective_decay;
            entry.relevance_score *= effective_decay;
            if entry.confidence < MIN_CONFIDENCE {
                entry.confidence = MIN_CONFIDENCE;
                decayed += 1;
            }
        }
        self.stats.decayed_entries += decayed;
    }

    /// Total size of the knowledge base.
    pub fn knowledge_size(&self) -> KnowledgeSizeReport {
        let total_entries = self.entries.len();
        let total_links = self.links.len();
        let total_tokens: usize = self.entries.values().map(|e| e.tokens.len()).sum();
        let high_conf = self
            .entries
            .values()
            .filter(|e| e.confidence >= HIGH_CONFIDENCE)
            .count();
        let avg_conf = if total_entries > 0 {
            self.entries.values().map(|e| e.confidence).sum::<f32>() / total_entries as f32
        } else {
            0.0
        };
        KnowledgeSizeReport {
            total_entries,
            total_links,
            total_tokens,
            categories: self.categories.len(),
            high_confidence_entries: high_conf,
            average_confidence: avg_conf,
            index_size: self.inverted_index.len(),
        }
    }

    /// Current stats.
    pub fn stats(&self) -> &KnowledgeStats {
        &self.stats
    }

    /// Number of entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    // ========================================================================
    // INTERNAL HELPERS
    // ========================================================================

    fn ensure_category(&mut self, name: &str, cat_id: u64) {
        if self.categories.len() >= MAX_CATEGORIES && !self.categories.contains_key(&cat_id) {
            return;
        }
        self.categories.entry(cat_id).or_insert_with(|| Category {
            id: cat_id,
            name: String::from(name),
            entry_count: 0,
            avg_confidence_ema: 0.5,
        });
    }

    fn evict_lowest_value(&mut self) {
        let worst = self
            .entries
            .values()
            .min_by(|a, b| {
                let va = a.confidence * 0.5 + (a.times_used as f32 * 0.01).min(0.5);
                let vb = b.confidence * 0.5 + (b.times_used as f32 * 0.01).min(0.5);
                va.partial_cmp(&vb).unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|e| e.id);
        if let Some(wid) = worst {
            // Remove from inverted index
            if let Some(entry) = self.entries.get(&wid) {
                for &token in &entry.tokens {
                    if let Some(ids) = self.inverted_index.get_mut(&token) {
                        ids.retain(|&id| id != wid);
                    }
                }
            }
            self.entries.remove(&wid);
        }
    }

    fn traverse(
        &self,
        current: u64,
        visited: &mut Vec<u64>,
        path_ids: &mut Vec<u64>,
        rels: &mut Vec<String>,
        weight: f32,
        max_depth: usize,
        paths: &mut Vec<GraphPath>,
    ) {
        if visited.contains(&current) || path_ids.len() > max_depth {
            return;
        }
        visited.push(current);
        path_ids.push(current);

        if path_ids.len() > 1 {
            paths.push(GraphPath {
                entry_ids: path_ids.clone(),
                relationships: rels.clone(),
                total_weight: weight,
                path_length: path_ids.len(),
            });
        }

        for link in &self.links {
            if link.source_id == current {
                rels.push(link.relationship.clone());
                self.traverse(
                    link.target_id,
                    visited,
                    path_ids,
                    rels,
                    weight + link.weight,
                    max_depth,
                    paths,
                );
                rels.pop();
            }
        }
        path_ids.pop();
        visited.pop();
    }

    fn update_utilization(&mut self) {
        if self.entries.is_empty() {
            self.stats.knowledge_utilization = 0.0;
            return;
        }
        let used = self.entries.values().filter(|e| e.times_used > 0).count();
        self.stats.knowledge_utilization = used as f32 / self.entries.len() as f32;
    }
}

// ============================================================================
// SIZE REPORT
// ============================================================================

/// Report on knowledge base size.
#[derive(Clone)]
pub struct KnowledgeSizeReport {
    pub total_entries: usize,
    pub total_links: usize,
    pub total_tokens: usize,
    pub categories: usize,
    pub high_confidence_entries: usize,
    pub average_confidence: f32,
    pub index_size: usize,
}
