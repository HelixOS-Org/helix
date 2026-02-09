// SPDX-License-Identifier: GPL-2.0
//! # Holistic Literature — Complete System Knowledge Base
//!
//! Maintains the unified knowledge base of the entire NEXUS kernel: every
//! known fact, proven theorem, validated invariant, best practice, and
//! failure lesson across all subsystems. While the journal *records*
//! discoveries, the literature *organises and distils* them into an
//! authoritative reference that research engines can query before
//! exploring already-settled territory.
//!
//! The literature supports cross-domain queries ("what do we know about
//! latency in the scheduler AND memory subsystems?"), knowledge
//! completeness scoring, gap prioritisation (where are we most
//! ignorant?), wisdom extraction (distilling principles from many
//! individual results), and a state-of-knowledge dashboard.
//!
//! The engine that knows everything the kernel knows — and what it doesn't.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_KNOWLEDGE_ITEMS: usize = 2048;
const MAX_DOMAINS: usize = 16;
const MAX_WISDOM_RULES: usize = 256;
const STALENESS_TICKS: u64 = 80_000;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const GAP_PRIORITY_THRESHOLD: f32 = 0.40;
const COMPLETENESS_TARGET: f32 = 0.80;
const WISDOM_MIN_EVIDENCE: usize = 5;

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

/// Knowledge domain (subsystem area)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeDomain {
    Scheduling,
    Memory,
    Ipc,
    FileSystem,
    Networking,
    Trust,
    Energy,
    Cooperation,
    Bridge,
    Application,
    SystemWide,
}

/// Kind of knowledge item
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeKind {
    Fact,
    Theorem,
    Invariant,
    BestPractice,
    FailureLesson,
    Heuristic,
    OpenQuestion,
}

/// Confidence level for a knowledge item
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfidenceLevel {
    Speculative,
    Preliminary,
    Supported,
    WellEstablished,
    Proven,
}

/// A single item in the unified knowledge base
#[derive(Debug, Clone)]
pub struct KnowledgeItem {
    pub id: u64,
    pub domain: KnowledgeDomain,
    pub kind: KnowledgeKind,
    pub statement: String,
    pub confidence: ConfidenceLevel,
    pub evidence_count: usize,
    pub relevance_score: f32,
    pub created_tick: u64,
    pub last_verified: u64,
    pub cross_domains: Vec<KnowledgeDomain>,
}

/// Query result from a cross-domain search
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub items: Vec<u64>,
    pub domains_searched: Vec<KnowledgeDomain>,
    pub total_matches: usize,
    pub avg_confidence: f32,
}

/// Completeness assessment for a domain
#[derive(Debug, Clone)]
pub struct CompletenessReport {
    pub domain: KnowledgeDomain,
    pub total_items: usize,
    pub proven_items: usize,
    pub open_questions: usize,
    pub completeness_score: f32,
    pub staleness_fraction: f32,
}

/// A knowledge gap that needs research
#[derive(Debug, Clone)]
pub struct KnowledgeGap {
    pub id: u64,
    pub domain: KnowledgeDomain,
    pub description: String,
    pub priority: f32,
    pub estimated_difficulty: f32,
    pub related_items: Vec<u64>,
}

/// A distilled wisdom rule derived from multiple findings
#[derive(Debug, Clone)]
pub struct WisdomRule {
    pub id: u64,
    pub principle: String,
    pub supporting_evidence: Vec<u64>,
    pub domains: Vec<KnowledgeDomain>,
    pub strength: f32,
    pub applicability: f32,
}

/// State of knowledge summary
#[derive(Debug, Clone)]
pub struct StateOfKnowledge {
    pub total_items: usize,
    pub domain_coverage: Vec<(KnowledgeDomain, f32)>,
    pub overall_completeness: f32,
    pub top_gaps: Vec<KnowledgeGap>,
    pub wisdom_rules: usize,
    pub avg_confidence: f32,
}

/// Literature statistics
#[derive(Debug, Clone)]
pub struct LiteratureStats {
    pub total_items: u64,
    pub domains_covered: u64,
    pub proven_count: u64,
    pub open_questions: u64,
    pub wisdom_rules: u64,
    pub gaps_identified: u64,
    pub avg_completeness_ema: f32,
    pub avg_relevance_ema: f32,
    pub queries_served: u64,
}

// ============================================================================
// HOLISTIC LITERATURE
// ============================================================================

/// Complete system knowledge base
pub struct HolisticLiterature {
    items: BTreeMap<u64, KnowledgeItem>,
    gaps: BTreeMap<u64, KnowledgeGap>,
    wisdom: Vec<WisdomRule>,
    domain_item_count: BTreeMap<KnowledgeDomain, usize>,
    rng_state: u64,
    stats: LiteratureStats,
}

impl HolisticLiterature {
    /// Create a new literature engine
    pub fn new(seed: u64) -> Self {
        Self {
            items: BTreeMap::new(),
            gaps: BTreeMap::new(),
            wisdom: Vec::new(),
            domain_item_count: BTreeMap::new(),
            rng_state: seed | 1,
            stats: LiteratureStats {
                total_items: 0, domains_covered: 0, proven_count: 0,
                open_questions: 0, wisdom_rules: 0, gaps_identified: 0,
                avg_completeness_ema: 0.0, avg_relevance_ema: 0.0,
                queries_served: 0,
            },
        }
    }

    /// Add or update a knowledge item in the unified base
    pub fn unified_knowledge(
        &mut self, domain: KnowledgeDomain, kind: KnowledgeKind,
        statement: String, confidence: ConfidenceLevel,
        evidence: usize, tick: u64,
    ) -> u64 {
        let id = fnv1a_hash(statement.as_bytes()) ^ fnv1a_hash(&tick.to_le_bytes());
        if self.items.len() >= MAX_KNOWLEDGE_ITEMS {
            self.evict_stale(tick);
        }
        let relevance = match confidence {
            ConfidenceLevel::Proven => 1.0,
            ConfidenceLevel::WellEstablished => 0.85,
            ConfidenceLevel::Supported => 0.65,
            ConfidenceLevel::Preliminary => 0.40,
            ConfidenceLevel::Speculative => 0.20,
        };
        let item = KnowledgeItem {
            id, domain, kind, statement, confidence,
            evidence_count: evidence, relevance_score: relevance,
            created_tick: tick, last_verified: tick,
            cross_domains: Vec::new(),
        };
        self.items.insert(id, item);
        *self.domain_item_count.entry(domain).or_insert(0) += 1;
        self.refresh_stats(tick);
        id
    }

    /// Query knowledge across multiple domains
    pub fn cross_domain_query(
        &mut self, domains: &[KnowledgeDomain], kind_filter: Option<KnowledgeKind>,
    ) -> QueryResult {
        self.stats.queries_served += 1;
        let mut matching_ids = Vec::new();
        let mut conf_sum = 0.0f32;
        for (&id, item) in &self.items {
            let domain_match = domains.contains(&item.domain)
                || item.cross_domains.iter().any(|d| domains.contains(d));
            let kind_match = kind_filter.map_or(true, |k| item.kind == k);
            if domain_match && kind_match {
                matching_ids.push(id);
                conf_sum += item.relevance_score;
            }
        }
        let avg = if matching_ids.is_empty() { 0.0 }
            else { conf_sum / matching_ids.len() as f32 };
        QueryResult {
            total_matches: matching_ids.len(),
            items: matching_ids,
            domains_searched: domains.to_vec(),
            avg_confidence: avg,
        }
    }

    /// Assess knowledge completeness per domain
    pub fn knowledge_completeness(&mut self, tick: u64) -> Vec<CompletenessReport> {
        let all_domains = [
            KnowledgeDomain::Scheduling, KnowledgeDomain::Memory,
            KnowledgeDomain::Ipc, KnowledgeDomain::FileSystem,
            KnowledgeDomain::Networking, KnowledgeDomain::Trust,
            KnowledgeDomain::Energy, KnowledgeDomain::Cooperation,
            KnowledgeDomain::Bridge, KnowledgeDomain::Application,
            KnowledgeDomain::SystemWide,
        ];
        let mut reports = Vec::new();
        for &domain in &all_domains {
            let domain_items: Vec<&KnowledgeItem> = self.items.values()
                .filter(|i| i.domain == domain).collect();
            let total = domain_items.len();
            let proven = domain_items.iter()
                .filter(|i| i.confidence == ConfidenceLevel::Proven
                    || i.confidence == ConfidenceLevel::WellEstablished)
                .count();
            let open = domain_items.iter()
                .filter(|i| i.kind == KnowledgeKind::OpenQuestion).count();
            let stale = domain_items.iter()
                .filter(|i| tick.saturating_sub(i.last_verified) > STALENESS_TICKS)
                .count();
            let completeness = if total > 0 {
                proven as f32 / total as f32
            } else { 0.0 };
            let staleness_frac = if total > 0 {
                stale as f32 / total as f32
            } else { 0.0 };
            reports.push(CompletenessReport {
                domain, total_items: total, proven_items: proven,
                open_questions: open, completeness_score: completeness,
                staleness_fraction: staleness_frac,
            });
        }
        let avg_comp: f32 = reports.iter().map(|r| r.completeness_score).sum::<f32>()
            / reports.len().max(1) as f32;
        self.stats.avg_completeness_ema =
            EMA_ALPHA * avg_comp + (1.0 - EMA_ALPHA) * self.stats.avg_completeness_ema;
        reports
    }

    /// Identify and prioritise knowledge gaps
    pub fn gap_prioritization(&mut self, tick: u64) -> Vec<&KnowledgeGap> {
        let completeness = self.knowledge_completeness(tick);
        for report in &completeness {
            if report.completeness_score < GAP_PRIORITY_THRESHOLD {
                let gap_id = fnv1a_hash(&(report.domain as u64).to_le_bytes());
                if !self.gaps.contains_key(&gap_id) {
                    let priority = 1.0 - report.completeness_score;
                    let diff = 0.5 + report.open_questions as f32 * 0.05;
                    self.gaps.insert(gap_id, KnowledgeGap {
                        id: gap_id, domain: report.domain,
                        description: String::from("low_coverage"),
                        priority, estimated_difficulty: diff.min(1.0),
                        related_items: Vec::new(),
                    });
                }
            }
        }
        self.stats.gaps_identified = self.gaps.len() as u64;
        let mut sorted: Vec<&KnowledgeGap> = self.gaps.values().collect();
        sorted.sort_by(|a, b|
            b.priority.partial_cmp(&a.priority).unwrap_or(core::cmp::Ordering::Equal));
        sorted
    }

    /// Extract wisdom rules from accumulated evidence
    pub fn wisdom_extraction(&mut self) -> &[WisdomRule] {
        let mut domain_groups: BTreeMap<KnowledgeDomain, Vec<u64>> = BTreeMap::new();
        for (&id, item) in &self.items {
            if item.evidence_count >= WISDOM_MIN_EVIDENCE
                && (item.confidence == ConfidenceLevel::Proven
                    || item.confidence == ConfidenceLevel::WellEstablished)
            {
                domain_groups.entry(item.domain).or_insert_with(Vec::new).push(id);
            }
        }
        for (domain, ids) in &domain_groups {
            if ids.len() < 2 { continue; }
            if self.wisdom.len() >= MAX_WISDOM_RULES { break; }
            let w_id = fnv1a_hash(&(*domain as u64).to_le_bytes())
                ^ fnv1a_hash(&(ids.len() as u64).to_le_bytes());
            let already = self.wisdom.iter().any(|w| w.id == w_id);
            if already { continue; }
            let strength = ids.len() as f32 / MAX_KNOWLEDGE_ITEMS as f32;
            self.wisdom.push(WisdomRule {
                id: w_id,
                principle: String::from("derived_principle"),
                supporting_evidence: ids.clone(),
                domains: alloc::vec![*domain],
                strength: strength.min(1.0),
                applicability: 0.75,
            });
        }
        self.stats.wisdom_rules = self.wisdom.len() as u64;
        &self.wisdom
    }

    /// Generate a complete state-of-knowledge report
    pub fn state_of_knowledge(&mut self, tick: u64) -> StateOfKnowledge {
        let completeness_reports = self.knowledge_completeness(tick);
        let domain_coverage: Vec<(KnowledgeDomain, f32)> = completeness_reports.iter()
            .map(|r| (r.domain, r.completeness_score)).collect();
        let overall = domain_coverage.iter().map(|(_, c)| c).sum::<f32>()
            / domain_coverage.len().max(1) as f32;
        let top_gaps: Vec<KnowledgeGap> = self.gaps.values()
            .take(5).cloned().collect();
        let avg_conf: f32 = self.items.values()
            .map(|i| i.relevance_score).sum::<f32>()
            / self.items.len().max(1) as f32;
        StateOfKnowledge {
            total_items: self.items.len(),
            domain_coverage,
            overall_completeness: overall,
            top_gaps,
            wisdom_rules: self.wisdom.len(),
            avg_confidence: avg_conf,
        }
    }

    /// Current statistics snapshot
    pub fn stats(&self) -> &LiteratureStats { &self.stats }

    // ── private helpers ─────────────────────────────────────────────────

    fn evict_stale(&mut self, tick: u64) {
        let stale: Vec<u64> = self.items.iter()
            .filter(|(_, i)| tick.saturating_sub(i.last_verified) > STALENESS_TICKS)
            .map(|(&id, _)| id).take(64).collect();
        for id in stale {
            self.items.remove(&id);
        }
    }

    fn refresh_stats(&mut self, _tick: u64) {
        self.stats.total_items = self.items.len() as u64;
        self.stats.domains_covered = self.domain_item_count.len() as u64;
        self.stats.proven_count = self.items.values()
            .filter(|i| i.confidence == ConfidenceLevel::Proven).count() as u64;
        self.stats.open_questions = self.items.values()
            .filter(|i| i.kind == KnowledgeKind::OpenQuestion).count() as u64;
        let avg_rel: f32 = self.items.values()
            .map(|i| i.relevance_score).sum::<f32>()
            / self.items.len().max(1) as f32;
        self.stats.avg_relevance_ema =
            EMA_ALPHA * avg_rel + (1.0 - EMA_ALPHA) * self.stats.avg_relevance_ema;
    }
}
