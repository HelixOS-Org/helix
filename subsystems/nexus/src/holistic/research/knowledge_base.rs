// SPDX-License-Identifier: GPL-2.0
//! # Holistic Knowledge Base — The Grand System Knowledge Repository
//!
//! THE GRAND KNOWLEDGE BASE for the entire NEXUS kernel intelligence
//! framework. Every validated discovery, every replicated finding, every
//! synthesised insight from all subsystems converges here into a unified,
//! cross-referenced, searchable knowledge graph.
//!
//! ## Capabilities
//!
//! - **Grand knowledge store** — persistent, versioned knowledge repository
//! - **Semantic query** — search knowledge by meaning, not just keywords
//! - **Knowledge graph** — cross-referenced entity-relationship mapping
//! - **Cross-referencing** — link related knowledge across subsystems
//! - **Completeness scoring** — identify gaps in system knowledge
//! - **Knowledge evolution** — track how understanding changes over time
//!
//! The repository where all kernel wisdom lives, connected and queryable.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ENTRIES: usize = 8192;
const MAX_EDGES: usize = 16384;
const MAX_QUERY_RESULTS: usize = 64;
const MAX_VERSIONS: usize = 256;
const MAX_CROSS_REFS: usize = 4096;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const RELEVANCE_THRESHOLD: f32 = 0.20;
const STALENESS_DECAY: f32 = 0.995;
const EVOLUTION_WINDOW: usize = 128;
const COMPLETENESS_DOMAINS: usize = 10;
const KNOWLEDGE_CONFIDENCE_FLOOR: f32 = 0.10;

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

fn xorshift_f32(state: &mut u64) -> f32 {
    (xorshift64(state) % 10000) as f32 / 10000.0
}

// ============================================================================
// TYPES
// ============================================================================

/// Knowledge domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeDomain {
    Scheduling,
    Memory,
    Ipc,
    FileSystem,
    Networking,
    Trust,
    Energy,
    Hardware,
    Security,
    Emergent,
}

/// Confidence level of a knowledge entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfidenceLevel {
    Speculative,
    Preliminary,
    Moderate,
    High,
    Replicated,
    Canonical,
}

/// Type of relationship in the knowledge graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationType {
    Supports,
    Contradicts,
    Extends,
    Supersedes,
    Correlates,
    Causes,
    Requires,
    Enables,
}

/// A single knowledge entry in the grand knowledge base
#[derive(Debug, Clone)]
pub struct KnowledgeEntry {
    pub id: u64,
    pub domain: KnowledgeDomain,
    pub title: String,
    pub content_hash: u64,
    pub confidence: ConfidenceLevel,
    pub confidence_score: f32,
    pub source_subsystem: u64,
    pub version: u32,
    pub created_tick: u64,
    pub updated_tick: u64,
    pub access_count: u64,
    pub citation_count: u64,
    pub tags: Vec<u64>,
}

/// An edge in the knowledge graph
#[derive(Debug, Clone)]
pub struct KnowledgeEdge {
    pub id: u64,
    pub from_entry: u64,
    pub to_entry: u64,
    pub relation: RelationType,
    pub strength: f32,
    pub created_tick: u64,
}

/// Cross-reference between knowledge entries
#[derive(Debug, Clone)]
pub struct CrossReference {
    pub id: u64,
    pub entry_a: u64,
    pub entry_b: u64,
    pub relevance: f32,
    pub shared_tags: u64,
    pub tick: u64,
}

/// Version history entry for knowledge evolution
#[derive(Debug, Clone)]
pub struct KnowledgeVersion {
    pub entry_id: u64,
    pub version: u32,
    pub confidence_at_version: f32,
    pub content_hash: u64,
    pub tick: u64,
}

/// Query result from semantic search
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub entry_id: u64,
    pub relevance_score: f32,
    pub domain: KnowledgeDomain,
    pub confidence_score: f32,
    pub title: String,
}

/// Domain completeness record
#[derive(Debug, Clone)]
pub struct DomainCompleteness {
    pub domain: KnowledgeDomain,
    pub entry_count: u64,
    pub avg_confidence: f32,
    pub coverage_score: f32,
    pub gap_severity: f32,
    pub last_update_tick: u64,
}

/// Knowledge base statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KnowledgeBaseStats {
    pub total_entries: u64,
    pub total_edges: u64,
    pub total_cross_refs: u64,
    pub total_queries: u64,
    pub avg_confidence_ema: f32,
    pub avg_relevance_ema: f32,
    pub knowledge_completeness: f32,
    pub graph_density: f32,
    pub evolution_rate_ema: f32,
    pub domains_covered: u64,
    pub canonical_entries: u64,
    pub last_tick: u64,
}

// ============================================================================
// HOLISTIC KNOWLEDGE BASE
// ============================================================================

/// The grand system-wide knowledge repository
pub struct HolisticKnowledgeBase {
    entries: BTreeMap<u64, KnowledgeEntry>,
    edges: Vec<KnowledgeEdge>,
    cross_refs: Vec<CrossReference>,
    versions: VecDeque<KnowledgeVersion>,
    domain_completeness: BTreeMap<u64, DomainCompleteness>,
    tag_index: BTreeMap<u64, Vec<u64>>,
    rng_state: u64,
    tick: u64,
    stats: KnowledgeBaseStats,
}

impl HolisticKnowledgeBase {
    /// Create a new holistic knowledge base
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            edges: Vec::new(),
            cross_refs: Vec::new(),
            versions: VecDeque::new(),
            domain_completeness: BTreeMap::new(),
            tag_index: BTreeMap::new(),
            rng_state: seed | 1,
            tick: 0,
            stats: KnowledgeBaseStats {
                total_entries: 0,
                total_edges: 0,
                total_cross_refs: 0,
                total_queries: 0,
                avg_confidence_ema: 0.0,
                avg_relevance_ema: 0.0,
                knowledge_completeness: 0.0,
                graph_density: 0.0,
                evolution_rate_ema: 0.0,
                domains_covered: 0,
                canonical_entries: 0,
                last_tick: 0,
            },
        }
    }

    /// Store knowledge in the grand repository
    pub fn grand_knowledge_store(&mut self, domain: KnowledgeDomain, title: String,
                                  confidence: ConfidenceLevel, source_subsystem: u64,
                                  tags: Vec<u64>) -> u64 {
        let content_hash = fnv1a_hash(title.as_bytes());
        let id = self.stats.total_entries;
        let conf_score = match confidence {
            ConfidenceLevel::Speculative => 0.1,
            ConfidenceLevel::Preliminary => 0.3,
            ConfidenceLevel::Moderate => 0.5,
            ConfidenceLevel::High => 0.7,
            ConfidenceLevel::Replicated => 0.9,
            ConfidenceLevel::Canonical => 1.0,
        };
        let entry = KnowledgeEntry {
            id, domain, title, content_hash, confidence,
            confidence_score: conf_score, source_subsystem,
            version: 1, created_tick: self.tick, updated_tick: self.tick,
            access_count: 0, citation_count: 0, tags: tags.clone(),
        };
        if self.entries.len() >= MAX_ENTRIES {
            let oldest = self.entries.keys().next().copied();
            if let Some(k) = oldest { self.entries.remove(&k); }
        }
        self.entries.insert(id, entry);
        self.versions.push_back(KnowledgeVersion {
            entry_id: id, version: 1, confidence_at_version: conf_score,
            content_hash, tick: self.tick,
        });
        if self.versions.len() > MAX_VERSIONS {
            self.versions.pop_front();
        }
        for &tag in &tags {
            self.tag_index.entry(tag).or_insert_with(Vec::new).push(id);
        }
        self.stats.total_entries += 1;
        self.stats.avg_confidence_ema = self.stats.avg_confidence_ema
            * (1.0 - EMA_ALPHA) + conf_score * EMA_ALPHA;
        if confidence == ConfidenceLevel::Canonical {
            self.stats.canonical_entries += 1;
        }
        self.stats.last_tick = self.tick;
        id
    }

    /// Semantic query — find knowledge entries by tag similarity
    pub fn semantic_query(&mut self, query_tags: &[u64]) -> Vec<QueryResult> {
        let mut results: Vec<QueryResult> = Vec::new();
        for entry in self.entries.values() {
            let mut shared = 0u64;
            for &qt in query_tags {
                if entry.tags.contains(&qt) { shared += 1; }
            }
            let relevance = if query_tags.is_empty() { 0.0 }
                else { shared as f32 / query_tags.len() as f32 };
            if relevance >= RELEVANCE_THRESHOLD {
                results.push(QueryResult {
                    entry_id: entry.id,
                    relevance_score: relevance * entry.confidence_score,
                    domain: entry.domain,
                    confidence_score: entry.confidence_score,
                    title: entry.title.clone(),
                });
            }
        }
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score)
            .unwrap_or(core::cmp::Ordering::Equal));
        results.truncate(MAX_QUERY_RESULTS);
        self.stats.total_queries += 1;
        if let Some(top) = results.first() {
            self.stats.avg_relevance_ema = self.stats.avg_relevance_ema
                * (1.0 - EMA_ALPHA) + top.relevance_score * EMA_ALPHA;
        }
        for r in &results {
            if let Some(entry) = self.entries.get_mut(&r.entry_id) {
                entry.access_count += 1;
            }
        }
        results
    }

    /// Build the knowledge graph — find relationships between entries
    pub fn knowledge_graph(&mut self) -> Vec<KnowledgeEdge> {
        let mut new_edges = Vec::new();
        let entry_ids: Vec<u64> = self.entries.keys().copied().collect();
        let max_pairs = 500;
        let mut pair_count = 0;
        for i in 0..entry_ids.len() {
            if pair_count >= max_pairs { break; }
            for j in (i + 1)..entry_ids.len() {
                if pair_count >= max_pairs { break; }
                let id_a = entry_ids[i];
                let id_b = entry_ids[j];
                let (shared_tags, domain_match) = {
                    let ea = match self.entries.get(&id_a) { Some(e) => e, None => continue };
                    let eb = match self.entries.get(&id_b) { Some(e) => e, None => continue };
                    let shared: u64 = ea.tags.iter()
                        .filter(|t| eb.tags.contains(t)).count() as u64;
                    (shared, ea.domain == eb.domain)
                };
                if shared_tags > 0 || domain_match {
                    let strength = shared_tags as f32 * 0.3
                        + if domain_match { 0.4 } else { 0.0 };
                    let noise = xorshift_f32(&mut self.rng_state) * 0.1;
                    let relation = if strength > 0.6 { RelationType::Supports }
                        else if domain_match { RelationType::Correlates }
                        else { RelationType::Extends };
                    let edge_id = self.stats.total_edges;
                    let edge = KnowledgeEdge {
                        id: edge_id, from_entry: id_a, to_entry: id_b,
                        relation, strength: (strength + noise).min(1.0),
                        created_tick: self.tick,
                    };
                    new_edges.push(edge.clone());
                    if self.edges.len() < MAX_EDGES {
                        self.edges.push(edge);
                        self.stats.total_edges += 1;
                    }
                    if let Some(ea) = self.entries.get_mut(&id_a) { ea.citation_count += 1; }
                    if let Some(eb) = self.entries.get_mut(&id_b) { eb.citation_count += 1; }
                }
                pair_count += 1;
            }
        }
        let n = self.entries.len() as f32;
        let max_edges = n * (n - 1.0) / 2.0;
        self.stats.graph_density = if max_edges > 0.0 {
            self.edges.len() as f32 / max_edges
        } else { 0.0 };
        new_edges
    }

    /// Cross-reference entries — find and record related knowledge
    pub fn cross_reference(&mut self) -> Vec<CrossReference> {
        let mut new_refs = Vec::new();
        let entry_ids: Vec<u64> = self.entries.keys().copied().collect();
        let max_refs = 200;
        let mut ref_count = 0;
        for i in 0..entry_ids.len() {
            if ref_count >= max_refs { break; }
            for j in (i + 1)..entry_ids.len() {
                if ref_count >= max_refs { break; }
                let id_a = entry_ids[i];
                let id_b = entry_ids[j];
                let (shared, relevance) = {
                    let ea = match self.entries.get(&id_a) { Some(e) => e, None => continue };
                    let eb = match self.entries.get(&id_b) { Some(e) => e, None => continue };
                    let s: u64 = ea.tags.iter().filter(|t| eb.tags.contains(t)).count() as u64;
                    let total_tags = (ea.tags.len() + eb.tags.len()) as f32;
                    let rel = if total_tags > 0.0 { s as f32 * 2.0 / total_tags } else { 0.0 };
                    (s, rel)
                };
                if relevance > RELEVANCE_THRESHOLD {
                    let xref_id = self.stats.total_cross_refs;
                    let xref = CrossReference {
                        id: xref_id, entry_a: id_a, entry_b: id_b,
                        relevance, shared_tags: shared, tick: self.tick,
                    };
                    new_refs.push(xref.clone());
                    if self.cross_refs.len() < MAX_CROSS_REFS {
                        self.cross_refs.push(xref);
                        self.stats.total_cross_refs += 1;
                    }
                }
                ref_count += 1;
            }
        }
        new_refs
    }

    /// Score knowledge completeness across all domains
    pub fn knowledge_completeness(&mut self) -> f32 {
        let domains = [
            KnowledgeDomain::Scheduling, KnowledgeDomain::Memory,
            KnowledgeDomain::Ipc, KnowledgeDomain::FileSystem,
            KnowledgeDomain::Networking, KnowledgeDomain::Trust,
            KnowledgeDomain::Energy, KnowledgeDomain::Hardware,
            KnowledgeDomain::Security, KnowledgeDomain::Emergent,
        ];
        let mut covered = 0u64;
        for &domain in &domains {
            let count = self.entries.values()
                .filter(|e| e.domain == domain).count() as u64;
            let avg_conf = {
                let entries: Vec<f32> = self.entries.values()
                    .filter(|e| e.domain == domain)
                    .map(|e| e.confidence_score).collect();
                if entries.is_empty() { 0.0 }
                else { entries.iter().sum::<f32>() / entries.len() as f32 }
            };
            let coverage = (count as f32 / 50.0).min(1.0) * avg_conf;
            let gap = (1.0 - coverage).max(0.0);
            let key = domain as u64;
            self.domain_completeness.insert(key, DomainCompleteness {
                domain, entry_count: count, avg_confidence: avg_conf,
                coverage_score: coverage, gap_severity: gap,
                last_update_tick: self.tick,
            });
            if coverage > 0.5 { covered += 1; }
        }
        let completeness = covered as f32 / domains.len() as f32;
        self.stats.knowledge_completeness = completeness;
        self.stats.domains_covered = covered;
        completeness
    }

    /// Track knowledge evolution — how understanding changes over time
    pub fn knowledge_evolution(&mut self) -> f32 {
        if self.versions.len() < 2 { return 0.0; }
        let recent: Vec<&KnowledgeVersion> = self.versions.iter()
            .rev().take(EVOLUTION_WINDOW).collect();
        let mut change_sum = 0.0f32;
        let mut pairs = 0u64;
        for i in 0..recent.len() {
            for j in (i + 1)..recent.len() {
                if recent[i].entry_id == recent[j].entry_id {
                    let delta = (recent[i].confidence_at_version
                        - recent[j].confidence_at_version).abs();
                    change_sum += delta;
                    pairs += 1;
                }
            }
        }
        let evolution_rate = if pairs > 0 { change_sum / pairs as f32 } else { 0.0 };
        self.stats.evolution_rate_ema = self.stats.evolution_rate_ema
            * (1.0 - EMA_ALPHA) + evolution_rate * EMA_ALPHA;
        evolution_rate
    }

    /// Update an existing knowledge entry with new confidence
    pub fn update_entry(&mut self, entry_id: u64, new_confidence: ConfidenceLevel) {
        let conf_score = match new_confidence {
            ConfidenceLevel::Speculative => 0.1,
            ConfidenceLevel::Preliminary => 0.3,
            ConfidenceLevel::Moderate => 0.5,
            ConfidenceLevel::High => 0.7,
            ConfidenceLevel::Replicated => 0.9,
            ConfidenceLevel::Canonical => 1.0,
        };
        if let Some(entry) = self.entries.get_mut(&entry_id) {
            entry.version += 1;
            entry.confidence = new_confidence;
            entry.confidence_score = conf_score;
            entry.updated_tick = self.tick;
            self.versions.push_back(KnowledgeVersion {
                entry_id, version: entry.version,
                confidence_at_version: conf_score,
                content_hash: entry.content_hash, tick: self.tick,
            });
            if new_confidence == ConfidenceLevel::Canonical {
                self.stats.canonical_entries += 1;
            }
        }
    }

    /// Advance the engine tick
    #[inline(always)]
    pub fn tick(&mut self) { self.tick += 1; }

    /// Get current statistics
    #[inline(always)]
    pub fn stats(&self) -> &KnowledgeBaseStats { &self.stats }

    /// Get all entries
    #[inline(always)]
    pub fn entries(&self) -> &BTreeMap<u64, KnowledgeEntry> { &self.entries }

    /// Get all graph edges
    #[inline(always)]
    pub fn graph_edges(&self) -> &[KnowledgeEdge] { &self.edges }

    /// Get all cross-references
    #[inline(always)]
    pub fn cross_references(&self) -> &[CrossReference] { &self.cross_refs }

    /// Get domain completeness map
    #[inline(always)]
    pub fn domain_completeness_map(&self) -> &BTreeMap<u64, DomainCompleteness> {
        &self.domain_completeness
    }
}
