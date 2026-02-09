// SPDX-License-Identifier: GPL-2.0
//! # Bridge Literature — Internal Literature Review Engine
//!
//! Maintains a living knowledge base of all previous research, discoveries,
//! and validated optimizations. Before any new hypothesis is formed, the
//! literature engine checks for novelty — preventing wasteful re-research
//! of already-known facts. It identifies related prior work, detects
//! knowledge gaps, and tracks the state of the art for each optimization
//! domain.
//!
//! The bridge that knows what it already knows.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_KNOWLEDGE_ENTRIES: usize = 512;
const MAX_RELATED_RESULTS: usize = 32;
const NOVELTY_THRESHOLD: f32 = 0.70;
const STALENESS_TICKS: u64 = 100_000;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const FINGERPRINT_SHINGLE_SIZE: usize = 3;
const MAX_FINGERPRINTS: usize = 64;
const GAP_THRESHOLD: f32 = 0.30;

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

/// Confidence in the state of knowledge
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeConfidence {
    Speculative,
    Preliminary,
    Established,
    WellEstablished,
}

/// Optimization domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationDomain {
    Routing,
    Batching,
    Caching,
    Prefetching,
    Coalescing,
    Scheduling,
    MemoryManagement,
}

/// A knowledge entry in the literature base
#[derive(Debug, Clone)]
pub struct KnowledgeEntry {
    pub entry_id: u64,
    pub domain: OptimizationDomain,
    pub title: String,
    pub summary: String,
    pub confidence: KnowledgeConfidence,
    pub impact: f32,
    pub validated: bool,
    pub discovery_id: Option<u64>,
    pub fingerprints: Vec<u64>,
    pub created_tick: u64,
    pub last_cited_tick: u64,
    pub citation_count: u32,
}

/// Novelty check result
#[derive(Debug, Clone)]
pub struct NoveltyResult {
    pub is_novel: bool,
    pub novelty_score: f32,
    pub most_similar_id: Option<u64>,
    pub most_similar_score: f32,
    pub related_count: usize,
}

/// Related work entry
#[derive(Debug, Clone)]
pub struct RelatedWork {
    pub entry_id: u64,
    pub title: String,
    pub similarity: f32,
    pub domain: OptimizationDomain,
    pub confidence: KnowledgeConfidence,
}

/// Knowledge gap in a domain
#[derive(Debug, Clone)]
pub struct KnowledgeGap {
    pub domain: OptimizationDomain,
    pub gap_score: f32,
    pub total_entries: usize,
    pub established_count: usize,
    pub staleness_ratio: f32,
    pub description: String,
}

/// State of the art summary for a domain
#[derive(Debug, Clone)]
pub struct StateOfArt {
    pub domain: OptimizationDomain,
    pub best_known_impact: f32,
    pub best_entry_id: Option<u64>,
    pub total_knowledge: usize,
    pub avg_confidence: f32,
    pub avg_impact: f32,
    pub gap_score: f32,
}

// ============================================================================
// LITERATURE STATS
// ============================================================================

/// Aggregate literature engine statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct LiteratureStats {
    pub total_entries: u64,
    pub total_novelty_checks: u64,
    pub total_novel: u64,
    pub total_duplicate: u64,
    pub avg_novelty_score_ema: f32,
    pub total_searches: u64,
    pub domains_covered: u32,
    pub avg_knowledge_depth: f32,
    pub gap_count: u32,
}

// ============================================================================
// FINGERPRINT ENGINE
// ============================================================================

/// Generates content fingerprints for similarity comparison (shingling)
#[derive(Debug)]
struct FingerprintEngine {
    shingle_size: usize,
}

impl FingerprintEngine {
    fn new() -> Self {
        Self {
            shingle_size: FINGERPRINT_SHINGLE_SIZE,
        }
    }

    fn generate(&self, text: &str) -> Vec<u64> {
        let words: Vec<&[u8]> = self.split_words(text.as_bytes());
        if words.len() < self.shingle_size {
            // Hash the whole text as one fingerprint
            return alloc::vec![fnv1a_hash(text.as_bytes())];
        }

        let mut fingerprints: Vec<u64> = Vec::new();
        for i in 0..=(words.len() - self.shingle_size) {
            let mut shingle_data: Vec<u8> = Vec::new();
            for j in 0..self.shingle_size {
                if j > 0 {
                    shingle_data.push(b' ');
                }
                shingle_data.extend_from_slice(words[i + j]);
            }
            let fp = fnv1a_hash(&shingle_data);
            if !fingerprints.contains(&fp) {
                fingerprints.push(fp);
            }
            if fingerprints.len() >= MAX_FINGERPRINTS {
                break;
            }
        }
        fingerprints
    }

    fn split_words<'a>(&self, bytes: &'a [u8]) -> Vec<&'a [u8]> {
        let mut words: Vec<&'a [u8]> = Vec::new();
        let mut start = 0;
        for i in 0..=bytes.len() {
            let is_sep = i == bytes.len()
                || bytes[i] == b' '
                || bytes[i] == b'\n'
                || bytes[i] == b'\t'
                || bytes[i] == b','
                || bytes[i] == b'.';
            if is_sep && i > start {
                words.push(&bytes[start..i]);
                start = i + 1;
            } else if is_sep {
                start = i + 1;
            }
        }
        words
    }

    fn similarity(fps_a: &[u64], fps_b: &[u64]) -> f32 {
        if fps_a.is_empty() || fps_b.is_empty() {
            return 0.0;
        }
        let intersection = fps_a.iter().filter(|fp| fps_b.contains(fp)).count();
        // Jaccard similarity
        let union = fps_a.len() + fps_b.len() - intersection;
        if union == 0 {
            return 0.0;
        }
        intersection as f32 / union as f32
    }
}

// ============================================================================
// BRIDGE LITERATURE
// ============================================================================

/// Internal literature review and knowledge base engine
#[derive(Debug)]
pub struct BridgeLiterature {
    entries: BTreeMap<u64, KnowledgeEntry>,
    domain_index: BTreeMap<u64, Vec<u64>>,
    fingerprint_engine: FingerprintEngine,
    rng_state: u64,
    current_tick: u64,
    stats: LiteratureStats,
}

impl BridgeLiterature {
    /// Create a new literature engine
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            domain_index: BTreeMap::new(),
            fingerprint_engine: FingerprintEngine::new(),
            rng_state: seed | 1,
            current_tick: 0,
            stats: LiteratureStats::default(),
        }
    }

    /// Add knowledge to the base
    pub fn add_knowledge(
        &mut self,
        domain: OptimizationDomain,
        title: String,
        summary: String,
        impact: f32,
        validated: bool,
        discovery_id: Option<u64>,
        tick: u64,
    ) -> u64 {
        self.current_tick = tick;
        let entry_id = fnv1a_hash(title.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let fingerprints = self.fingerprint_engine.generate(&summary);

        let confidence = if validated {
            KnowledgeConfidence::Established
        } else {
            KnowledgeConfidence::Preliminary
        };

        let entry = KnowledgeEntry {
            entry_id,
            domain,
            title,
            summary,
            confidence,
            impact: impact.clamp(0.0, 1.0),
            validated,
            discovery_id,
            fingerprints,
            created_tick: tick,
            last_cited_tick: tick,
            citation_count: 0,
        };

        self.entries.insert(entry_id, entry);
        let domain_key = domain as u64;
        let domain_list = self.domain_index.entry(domain_key).or_insert_with(Vec::new);
        domain_list.push(entry_id);
        self.stats.total_entries += 1;

        // Update domain coverage
        self.stats.domains_covered = self.domain_index.len() as u32;

        // Evict oldest if over capacity
        while self.entries.len() > MAX_KNOWLEDGE_ENTRIES {
            let oldest = self
                .entries
                .iter()
                .filter(|(_, e)| e.confidence == KnowledgeConfidence::Speculative)
                .min_by_key(|(_, e)| e.last_cited_tick)
                .or_else(|| self.entries.iter().min_by_key(|(_, e)| e.last_cited_tick))
                .map(|(&k, _)| k);
            if let Some(k) = oldest {
                self.entries.remove(&k);
            } else {
                break;
            }
        }
        entry_id
    }

    /// Search the knowledge base by text query
    pub fn search_knowledge(&mut self, query: &str) -> Vec<RelatedWork> {
        self.stats.total_searches += 1;
        let query_fps = self.fingerprint_engine.generate(query);

        let mut results: Vec<RelatedWork> = self
            .entries
            .values()
            .map(|entry| {
                let sim = FingerprintEngine::similarity(&query_fps, &entry.fingerprints);
                RelatedWork {
                    entry_id: entry.entry_id,
                    title: entry.title.clone(),
                    similarity: sim,
                    domain: entry.domain,
                    confidence: entry.confidence,
                }
            })
            .filter(|r| r.similarity > 0.0)
            .collect();

        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        results.truncate(MAX_RELATED_RESULTS);
        results
    }

    /// Check if a proposed research topic is novel
    pub fn novelty_check(&mut self, title: &str, summary: &str) -> NoveltyResult {
        self.stats.total_novelty_checks += 1;
        let query_fps = self.fingerprint_engine.generate(summary);
        let title_hash = fnv1a_hash(title.as_bytes());

        let mut most_similar_id: Option<u64> = None;
        let mut most_similar_score: f32 = 0.0;
        let mut related_count: usize = 0;

        for entry in self.entries.values() {
            let sim = FingerprintEngine::similarity(&query_fps, &entry.fingerprints);
            let title_sim = if fnv1a_hash(entry.title.as_bytes()) == title_hash {
                0.5
            } else {
                0.0
            };
            let combined = (sim + title_sim).min(1.0);

            if combined > 0.1 {
                related_count += 1;
            }
            if combined > most_similar_score {
                most_similar_score = combined;
                most_similar_id = Some(entry.entry_id);
            }
        }

        let novelty_score = 1.0 - most_similar_score;
        let is_novel = novelty_score >= NOVELTY_THRESHOLD;

        if is_novel {
            self.stats.total_novel += 1;
        } else {
            self.stats.total_duplicate += 1;
        }
        self.stats.avg_novelty_score_ema =
            EMA_ALPHA * novelty_score + (1.0 - EMA_ALPHA) * self.stats.avg_novelty_score_ema;

        NoveltyResult {
            is_novel,
            novelty_score,
            most_similar_id,
            most_similar_score,
            related_count,
        }
    }

    /// Find related prior work for a given topic
    pub fn related_work(&mut self, summary: &str, domain: OptimizationDomain) -> Vec<RelatedWork> {
        let query_fps = self.fingerprint_engine.generate(summary);
        let domain_key = domain as u64;
        let domain_entries = self.domain_index.get(&domain_key);

        let mut results: Vec<RelatedWork> = Vec::new();

        if let Some(entry_ids) = domain_entries {
            for &eid in entry_ids {
                if let Some(entry) = self.entries.get(&eid) {
                    let sim = FingerprintEngine::similarity(&query_fps, &entry.fingerprints);
                    if sim > 0.0 {
                        results.push(RelatedWork {
                            entry_id: entry.entry_id,
                            title: entry.title.clone(),
                            similarity: sim,
                            domain: entry.domain,
                            confidence: entry.confidence,
                        });
                    }
                }
            }
        }

        // Also include cross-domain results with lower weight
        for entry in self.entries.values() {
            if entry.domain != domain {
                let sim = FingerprintEngine::similarity(&query_fps, &entry.fingerprints) * 0.5;
                if sim > 0.1 {
                    results.push(RelatedWork {
                        entry_id: entry.entry_id,
                        title: entry.title.clone(),
                        similarity: sim,
                        domain: entry.domain,
                        confidence: entry.confidence,
                    });
                }
            }
        }

        results.sort_by(|a, b| {
            b.similarity
                .partial_cmp(&a.similarity)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        results.truncate(MAX_RELATED_RESULTS);
        results
    }

    /// Identify knowledge gaps in a domain
    pub fn knowledge_gap(&self, domain: OptimizationDomain) -> KnowledgeGap {
        let domain_key = domain as u64;
        let entry_ids = self.domain_index.get(&domain_key);

        let entries: Vec<&KnowledgeEntry> = entry_ids
            .map(|ids| ids.iter().filter_map(|id| self.entries.get(id)).collect())
            .unwrap_or_default();

        let total = entries.len();
        let established = entries
            .iter()
            .filter(|e| {
                e.confidence == KnowledgeConfidence::Established
                    || e.confidence == KnowledgeConfidence::WellEstablished
            })
            .count();

        let stale_count = entries
            .iter()
            .filter(|e| self.current_tick.saturating_sub(e.last_cited_tick) > STALENESS_TICKS)
            .count();
        let staleness_ratio = if total > 0 {
            stale_count as f32 / total as f32
        } else {
            1.0
        };

        let coverage = if total > 0 {
            established as f32 / total as f32
        } else {
            0.0
        };
        let gap_score = (1.0 - coverage + staleness_ratio * 0.3).clamp(0.0, 1.0);

        let description = if total == 0 {
            String::from("No knowledge exists for this domain")
        } else if gap_score > 0.7 {
            String::from("Significant knowledge gap — needs research")
        } else if gap_score > GAP_THRESHOLD {
            String::from("Moderate gap — some areas under-explored")
        } else {
            String::from("Well-covered domain")
        };

        KnowledgeGap {
            domain,
            gap_score,
            total_entries: total,
            established_count: established,
            staleness_ratio,
            description,
        }
    }

    /// Get state of the art for a domain
    pub fn state_of_art(&self, domain: OptimizationDomain) -> StateOfArt {
        let domain_key = domain as u64;
        let entry_ids = self.domain_index.get(&domain_key);

        let entries: Vec<&KnowledgeEntry> = entry_ids
            .map(|ids| ids.iter().filter_map(|id| self.entries.get(id)).collect())
            .unwrap_or_default();

        let total = entries.len();
        let (best_impact, best_id) = entries.iter().fold((0.0_f32, None), |(best, id), e| {
            if e.impact > best {
                (e.impact, Some(e.entry_id))
            } else {
                (best, id)
            }
        });

        let avg_confidence = if total > 0 {
            entries
                .iter()
                .map(|e| match e.confidence {
                    KnowledgeConfidence::Speculative => 0.25,
                    KnowledgeConfidence::Preliminary => 0.50,
                    KnowledgeConfidence::Established => 0.75,
                    KnowledgeConfidence::WellEstablished => 1.0,
                })
                .sum::<f32>()
                / total as f32
        } else {
            0.0
        };

        let avg_impact = if total > 0 {
            entries.iter().map(|e| e.impact).sum::<f32>() / total as f32
        } else {
            0.0
        };

        let gap = self.knowledge_gap(domain);

        StateOfArt {
            domain,
            best_known_impact: best_impact,
            best_entry_id: best_id,
            total_knowledge: total,
            avg_confidence,
            avg_impact,
            gap_score: gap.gap_score,
        }
    }

    /// Get an entry by ID
    pub fn get_entry(&self, entry_id: u64) -> Option<&KnowledgeEntry> {
        self.entries.get(&entry_id)
    }

    /// Cite an entry (bumps its recency and citation count)
    pub fn cite(&mut self, entry_id: u64, tick: u64) {
        if let Some(entry) = self.entries.get_mut(&entry_id) {
            entry.last_cited_tick = tick;
            entry.citation_count += 1;
        }
    }

    /// Get aggregate stats
    pub fn stats(&self) -> LiteratureStats {
        self.stats
    }
}
