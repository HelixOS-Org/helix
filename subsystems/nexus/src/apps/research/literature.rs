// SPDX-License-Identifier: GPL-2.0
//! # Apps Literature — Application Knowledge Base Engine
//!
//! Maintains a living knowledge base of all previous app classification
//! research, feature discoveries, and validated optimizations. Before any
//! new hypothesis is formed, the literature engine checks for novelty —
//! preventing wasteful re-research of already-known classification facts.
//! It identifies related prior work, detects knowledge gaps per workload
//! domain, and tracks the state of the art for each classification approach.
//!
//! The engine that knows what it already knows about applications.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
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
const EVOLUTION_WINDOW: usize = 64;

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

/// App classification domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClassificationDomain {
    IoPattern,
    CpuProfile,
    MemoryBehavior,
    NetworkSignature,
    SyscallPattern,
    ResourcePrediction,
    AdaptationStrategy,
}

/// A knowledge entry in the literature base
#[derive(Debug, Clone)]
pub struct KnowledgeEntry {
    pub entry_id: u64,
    pub domain: ClassificationDomain,
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
    pub domain: ClassificationDomain,
    pub confidence: KnowledgeConfidence,
}

/// Knowledge gap in a domain
#[derive(Debug, Clone)]
pub struct KnowledgeGap {
    pub domain: ClassificationDomain,
    pub gap_score: f32,
    pub total_entries: usize,
    pub established_count: usize,
    pub staleness_ratio: f32,
    pub description: String,
}

/// Coverage report across all domains
#[derive(Debug, Clone)]
pub struct CoverageReport {
    pub total_entries: usize,
    pub domains_covered: usize,
    pub avg_confidence: f32,
    pub avg_impact: f32,
    pub gaps: Vec<KnowledgeGap>,
    pub overall_coverage: f32,
}

/// Knowledge evolution snapshot
#[derive(Debug, Clone)]
pub struct KnowledgeEvolution {
    pub domain: ClassificationDomain,
    pub entry_count: usize,
    pub avg_impact: f32,
    pub growth_rate: f32,
    pub confidence_trend: f32,
}

// ============================================================================
// LITERATURE STATS
// ============================================================================

/// Aggregate literature engine statistics
#[derive(Debug, Clone, Copy, Default)]
#[repr(align(64))]
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

    fn compute_fingerprints(&self, text: &str) -> Vec<u64> {
        let bytes = text.as_bytes();
        if bytes.len() < self.shingle_size {
            return Vec::new();
        }
        let mut fingerprints = Vec::new();
        for i in 0..=(bytes.len() - self.shingle_size) {
            let shingle = &bytes[i..i + self.shingle_size];
            let fp = fnv1a_hash(shingle);
            if fingerprints.len() < MAX_FINGERPRINTS && !fingerprints.contains(&fp) {
                fingerprints.push(fp);
            }
        }
        fingerprints
    }

    fn jaccard_similarity(a: &[u64], b: &[u64]) -> f32 {
        if a.is_empty() && b.is_empty() {
            return 1.0;
        }
        if a.is_empty() || b.is_empty() {
            return 0.0;
        }
        let mut intersection = 0usize;
        for fp in a {
            if b.contains(fp) {
                intersection += 1;
            }
        }
        let union = a.len() + b.len() - intersection;
        if union == 0 {
            return 0.0;
        }
        intersection as f32 / union as f32
    }
}

// ============================================================================
// APPS LITERATURE
// ============================================================================

/// Living knowledge base for app classification research
#[derive(Debug)]
pub struct AppsLiterature {
    entries: BTreeMap<u64, KnowledgeEntry>,
    fingerprint_engine: FingerprintEngine,
    domain_history: BTreeMap<u64, Vec<u64>>,
    rng_state: u64,
    current_tick: u64,
    stats: LiteratureStats,
}

impl AppsLiterature {
    /// Create a new literature engine with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            fingerprint_engine: FingerprintEngine::new(),
            domain_history: BTreeMap::new(),
            rng_state: seed | 1,
            current_tick: 0,
            stats: LiteratureStats::default(),
        }
    }

    /// Add knowledge entry to the base
    pub fn add_knowledge(
        &mut self,
        domain: ClassificationDomain,
        title: String,
        summary: String,
        impact: f32,
        discovery_id: Option<u64>,
        tick: u64,
    ) -> u64 {
        self.current_tick = tick;
        let id = fnv1a_hash(title.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let fingerprints = self.fingerprint_engine.compute_fingerprints(&summary);

        let entry = KnowledgeEntry {
            entry_id: id,
            domain,
            title,
            summary,
            confidence: KnowledgeConfidence::Preliminary,
            impact: impact.clamp(0.0, 1.0),
            validated: false,
            discovery_id,
            fingerprints,
            created_tick: tick,
            last_cited_tick: tick,
            citation_count: 0,
        };

        if self.entries.len() < MAX_KNOWLEDGE_ENTRIES {
            self.entries.insert(id, entry);
            self.stats.total_entries += 1;
            let dom_key = domain as u64;
            let history = self.domain_history.entry(dom_key).or_insert_with(Vec::new);
            history.push(tick);
            if history.len() > EVOLUTION_WINDOW {
                history.pop_front();
            }
        }
        id
    }

    /// Look up knowledge entries by domain
    #[inline]
    pub fn knowledge_lookup(&self, domain: ClassificationDomain) -> Vec<&KnowledgeEntry> {
        self.entries
            .values()
            .filter(|e| e.domain == domain)
            .collect()
    }

    /// Compute novelty score for a proposed research topic
    #[inline]
    pub fn novelty_score(&mut self, summary: &str) -> NoveltyResult {
        self.stats.total_novelty_checks += 1;
        let query_fps = self.fingerprint_engine.compute_fingerprints(summary);
        let mut best_similarity: f32 = 0.0;
        let mut best_id: Option<u64> = None;
        let mut related = 0usize;

        for entry in self.entries.values() {
            let sim = FingerprintEngine::jaccard_similarity(&query_fps, &entry.fingerprints);
            if sim > 0.1 {
                related += 1;
            }
            if sim > best_similarity {
                best_similarity = sim;
                best_id = Some(entry.entry_id);
            }
        }

        let novelty = 1.0 - best_similarity;
        let is_novel = novelty >= (1.0 - NOVELTY_THRESHOLD);

        if is_novel {
            self.stats.total_novel += 1;
        } else {
            self.stats.total_duplicate += 1;
        }
        self.stats.avg_novelty_score_ema =
            EMA_ALPHA * novelty + (1.0 - EMA_ALPHA) * self.stats.avg_novelty_score_ema;

        NoveltyResult {
            is_novel,
            novelty_score: novelty,
            most_similar_id: best_id,
            most_similar_score: best_similarity,
            related_count: related,
        }
    }

    /// Identify knowledge gaps in each classification domain
    pub fn gap_analysis(&self, tick: u64) -> Vec<KnowledgeGap> {
        let all_domains = [
            ClassificationDomain::IoPattern,
            ClassificationDomain::CpuProfile,
            ClassificationDomain::MemoryBehavior,
            ClassificationDomain::NetworkSignature,
            ClassificationDomain::SyscallPattern,
            ClassificationDomain::ResourcePrediction,
            ClassificationDomain::AdaptationStrategy,
        ];

        let mut gaps: Vec<KnowledgeGap> = Vec::new();
        for &domain in &all_domains {
            let domain_entries: Vec<&KnowledgeEntry> =
                self.entries.values().filter(|e| e.domain == domain).collect();
            let total = domain_entries.len();
            let established = domain_entries
                .iter()
                .filter(|e| {
                    matches!(
                        e.confidence,
                        KnowledgeConfidence::Established | KnowledgeConfidence::WellEstablished
                    )
                })
                .count();
            let stale_count = domain_entries
                .iter()
                .filter(|e| tick.saturating_sub(e.last_cited_tick) > STALENESS_TICKS)
                .count();
            let staleness_ratio = if total > 0 {
                stale_count as f32 / total as f32
            } else {
                1.0
            };
            let depth = if total > 0 {
                established as f32 / total as f32
            } else {
                0.0
            };
            let gap_score = 1.0 - depth * (1.0 - staleness_ratio * 0.5);
            if gap_score > GAP_THRESHOLD || total == 0 {
                gaps.push(KnowledgeGap {
                    domain,
                    gap_score,
                    total_entries: total,
                    established_count: established,
                    staleness_ratio,
                    description: String::from("Knowledge gap detected in classification domain"),
                });
            }
        }
        gaps.sort_by(|a, b| {
            b.gap_score
                .partial_cmp(&a.gap_score)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        gaps
    }

    /// Generate comprehensive coverage report
    pub fn coverage_report(&self, tick: u64) -> CoverageReport {
        let gaps = self.gap_analysis(tick);
        let mut domains_covered = 0u32;
        let mut total_impact: f32 = 0.0;
        let mut total_conf_score: f32 = 0.0;
        let count = self.entries.len() as f32;

        for entry in self.entries.values() {
            total_impact += entry.impact;
            total_conf_score += match entry.confidence {
                KnowledgeConfidence::Speculative => 0.25,
                KnowledgeConfidence::Preliminary => 0.50,
                KnowledgeConfidence::Established => 0.75,
                KnowledgeConfidence::WellEstablished => 1.00,
            };
        }

        let mut seen_domains: Vec<u64> = Vec::new();
        for entry in self.entries.values() {
            let dk = entry.domain as u64;
            if !seen_domains.contains(&dk) {
                seen_domains.push(dk);
                domains_covered += 1;
            }
        }

        let avg_impact = if count > 0.0 { total_impact / count } else { 0.0 };
        let avg_conf = if count > 0.0 {
            total_conf_score / count
        } else {
            0.0
        };
        let total_domains = 7.0;
        let overall = domains_covered as f32 / total_domains * (1.0 - gaps.len() as f32 / total_domains * 0.5);

        CoverageReport {
            total_entries: self.entries.len(),
            domains_covered: domains_covered as usize,
            avg_confidence: avg_conf,
            avg_impact,
            gaps,
            overall_coverage: overall.clamp(0.0, 1.0),
        }
    }

    /// Track knowledge evolution over time per domain
    pub fn knowledge_evolution(&self) -> Vec<KnowledgeEvolution> {
        let all_domains = [
            ClassificationDomain::IoPattern,
            ClassificationDomain::CpuProfile,
            ClassificationDomain::MemoryBehavior,
            ClassificationDomain::NetworkSignature,
            ClassificationDomain::SyscallPattern,
            ClassificationDomain::ResourcePrediction,
            ClassificationDomain::AdaptationStrategy,
        ];

        let mut evolutions: Vec<KnowledgeEvolution> = Vec::new();
        for &domain in &all_domains {
            let domain_entries: Vec<&KnowledgeEntry> =
                self.entries.values().filter(|e| e.domain == domain).collect();
            let count = domain_entries.len();
            if count == 0 {
                continue;
            }
            let avg_impact = domain_entries.iter().map(|e| e.impact).sum::<f32>() / count as f32;
            let avg_conf = domain_entries
                .iter()
                .map(|e| match e.confidence {
                    KnowledgeConfidence::Speculative => 0.25,
                    KnowledgeConfidence::Preliminary => 0.50,
                    KnowledgeConfidence::Established => 0.75,
                    KnowledgeConfidence::WellEstablished => 1.00,
                })
                .sum::<f32>()
                / count as f32;

            let dom_key = domain as u64;
            let growth = self
                .domain_history
                .get(&dom_key)
                .map(|h| {
                    if h.len() < 2 {
                        return 0.0;
                    }
                    let mid = h.len() / 2;
                    let first_half = mid as f32;
                    let second_half = (h.len() - mid) as f32;
                    (second_half - first_half) / first_half.max(1.0)
                })
                .unwrap_or(0.0);

            evolutions.push(KnowledgeEvolution {
                domain,
                entry_count: count,
                avg_impact,
                growth_rate: growth,
                confidence_trend: avg_conf,
            });
        }
        evolutions
    }

    /// Get aggregate stats
    #[inline(always)]
    pub fn stats(&self) -> LiteratureStats {
        self.stats
    }

    /// Mark an entry as validated
    #[inline]
    pub fn validate_entry(&mut self, entry_id: u64) {
        if let Some(e) = self.entries.get_mut(&entry_id) {
            e.validated = true;
            e.confidence = KnowledgeConfidence::Established;
        }
    }
}
