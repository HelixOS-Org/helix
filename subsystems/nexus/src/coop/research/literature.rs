// SPDX-License-Identifier: GPL-2.0
//! # Cooperation Literature — Protocol Knowledge Base
//!
//! A living knowledge base of known-good cooperation protocols, fairness
//! theorems, proven negotiation strategies, and documented vulnerabilities.
//! The literature module supports protocol lookup by category and contention
//! level, fairness theorem retrieval, known vulnerability queries, best
//! practice recommendations, and knowledge completeness assessment. New
//! discoveries are integrated and cross-referenced with existing knowledge,
//! while novelty detection flags truly original contributions.
//!
//! The engine that knows everything cooperation research has ever found.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_PROTOCOLS: usize = 512;
const MAX_THEOREMS: usize = 256;
const MAX_VULNERABILITIES: usize = 256;
const MAX_PRACTICES: usize = 256;
const EMA_ALPHA: f32 = 0.12;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const NOVELTY_THRESHOLD: f32 = 0.70;
const RELEVANCE_DECAY: f32 = 0.001;
const HIGH_RELEVANCE: f32 = 0.80;

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
// KNOWLEDGE BASE TYPES
// ============================================================================

/// Category of cooperation protocol in the knowledge base
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProtocolCategory {
    Auction,
    FixedQuota,
    ProportionalShare,
    PriorityBased,
    NashBargaining,
    VickreyAuction,
    RoundRobin,
}

/// Contention level under which a protocol is known to perform
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContentionLevel {
    Low,
    Medium,
    High,
    Extreme,
}

/// A known-good protocol record
#[derive(Debug, Clone)]
pub struct KnownProtocol {
    pub id: u64,
    pub name: String,
    pub category: ProtocolCategory,
    pub best_contention: ContentionLevel,
    pub fairness_score: f32,
    pub throughput_score: f32,
    pub latency_score: f32,
    pub validated: bool,
    pub added_tick: u64,
    pub relevance: f32,
    pub citation_count: u32,
}

/// A fairness theorem record
#[derive(Debug, Clone)]
pub struct FairnessTheorem {
    pub id: u64,
    pub name: String,
    pub statement: String,
    pub applies_to: ProtocolCategory,
    pub proven: bool,
    pub conditions: String,
    pub added_tick: u64,
    pub relevance: f32,
}

/// A documented vulnerability in a cooperation protocol
#[derive(Debug, Clone)]
pub struct Vulnerability {
    pub id: u64,
    pub protocol_category: ProtocolCategory,
    pub description: String,
    pub severity: f32,
    pub exploitable_under: ContentionLevel,
    pub mitigation: String,
    pub discovered_tick: u64,
}

/// A best practice recommendation
#[derive(Debug, Clone)]
pub struct BestPractice {
    pub id: u64,
    pub category: ProtocolCategory,
    pub contention: ContentionLevel,
    pub recommendation: String,
    pub confidence: f32,
    pub supporting_evidence: u32,
    pub added_tick: u64,
}

// ============================================================================
// LITERATURE STATS
// ============================================================================

/// Aggregate knowledge base statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct LiteratureStats {
    pub total_protocols: u64,
    pub total_theorems: u64,
    pub total_vulnerabilities: u64,
    pub total_practices: u64,
    pub avg_relevance_ema: f32,
    pub novelty_detections: u64,
    pub lookups_performed: u64,
    pub knowledge_completeness: f32,
    pub high_relevance_count: u64,
}

// ============================================================================
// COOPERATION LITERATURE
// ============================================================================

/// Cooperation protocol knowledge base and literature review engine
#[derive(Debug)]
pub struct CoopLiterature {
    protocols: BTreeMap<u64, KnownProtocol>,
    theorems: BTreeMap<u64, FairnessTheorem>,
    vulnerabilities: BTreeMap<u64, Vulnerability>,
    practices: BTreeMap<u64, BestPractice>,
    category_index: BTreeMap<u64, Vec<u64>>,
    tick: u64,
    rng_state: u64,
    stats: LiteratureStats,
}

impl CoopLiterature {
    /// Create a new cooperation literature knowledge base
    pub fn new(seed: u64) -> Self {
        Self {
            protocols: BTreeMap::new(),
            theorems: BTreeMap::new(),
            vulnerabilities: BTreeMap::new(),
            practices: BTreeMap::new(),
            category_index: BTreeMap::new(),
            tick: 0,
            rng_state: seed | 1,
            stats: LiteratureStats::default(),
        }
    }

    /// Add a known protocol to the knowledge base
    pub fn add_protocol(
        &mut self,
        name: String,
        category: ProtocolCategory,
        contention: ContentionLevel,
        fairness: f32,
        throughput: f32,
        latency: f32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let proto = KnownProtocol {
            id,
            name,
            category,
            best_contention: contention,
            fairness_score: fairness.clamp(0.0, 1.0),
            throughput_score: throughput.clamp(0.0, 1.0),
            latency_score: latency.clamp(0.0, 1.0),
            validated: false,
            added_tick: self.tick,
            relevance: 1.0,
            citation_count: 0,
        };
        if self.protocols.len() < MAX_PROTOCOLS {
            let cat_key = category as u64;
            let index = self.category_index.entry(cat_key).or_insert_with(Vec::new);
            index.push(id);
            self.protocols.insert(id, proto);
            self.stats.total_protocols += 1;
        }
        id
    }

    /// Look up protocols matching category and contention level
    pub fn protocol_lookup(
        &mut self,
        category: ProtocolCategory,
        contention: ContentionLevel,
    ) -> Vec<&KnownProtocol> {
        self.stats.lookups_performed += 1;
        let mut results: Vec<&KnownProtocol> = self
            .protocols
            .values()
            .filter(|p| p.category == category && p.best_contention <= contention)
            .collect();
        results.sort_by(|a, b| {
            b.fairness_score
                .partial_cmp(&a.fairness_score)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        results
    }

    /// Add a fairness theorem to the knowledge base
    pub fn add_theorem(
        &mut self,
        name: String,
        statement: String,
        applies_to: ProtocolCategory,
        proven: bool,
        conditions: String,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(name.as_bytes()) ^ fnv1a_hash(statement.as_bytes());
        let theorem = FairnessTheorem {
            id,
            name,
            statement,
            applies_to,
            proven,
            conditions,
            added_tick: self.tick,
            relevance: 1.0,
        };
        if self.theorems.len() < MAX_THEOREMS {
            self.theorems.insert(id, theorem);
            self.stats.total_theorems += 1;
        }
        id
    }

    /// Retrieve fairness theorems applicable to a protocol category
    pub fn fairness_theorem(&self, category: ProtocolCategory) -> Vec<&FairnessTheorem> {
        self.theorems
            .values()
            .filter(|t| t.applies_to == category && t.proven)
            .collect()
    }

    /// Register a known vulnerability
    pub fn add_vulnerability(
        &mut self,
        category: ProtocolCategory,
        description: String,
        severity: f32,
        contention: ContentionLevel,
        mitigation: String,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(description.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let vuln = Vulnerability {
            id,
            protocol_category: category,
            description,
            severity: severity.clamp(0.0, 1.0),
            exploitable_under: contention,
            mitigation,
            discovered_tick: self.tick,
        };
        if self.vulnerabilities.len() < MAX_VULNERABILITIES {
            self.vulnerabilities.insert(id, vuln);
            self.stats.total_vulnerabilities += 1;
        }
        id
    }

    /// Query known vulnerabilities for a protocol category
    pub fn known_vulnerability(&self, category: ProtocolCategory) -> Vec<&Vulnerability> {
        let mut results: Vec<&Vulnerability> = self
            .vulnerabilities
            .values()
            .filter(|v| v.protocol_category == category)
            .collect();
        results.sort_by(|a, b| {
            b.severity
                .partial_cmp(&a.severity)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        results
    }

    /// Add a best practice recommendation
    pub fn add_practice(
        &mut self,
        category: ProtocolCategory,
        contention: ContentionLevel,
        recommendation: String,
        confidence: f32,
        evidence_count: u32,
    ) -> u64 {
        self.tick += 1;
        let id = fnv1a_hash(recommendation.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let practice = BestPractice {
            id,
            category,
            contention,
            recommendation,
            confidence: confidence.clamp(0.0, 1.0),
            supporting_evidence: evidence_count,
            added_tick: self.tick,
        };
        if self.practices.len() < MAX_PRACTICES {
            self.practices.insert(id, practice);
            self.stats.total_practices += 1;
        }
        id
    }

    /// Retrieve best practices for a given category and contention level
    pub fn best_practice(
        &self,
        category: ProtocolCategory,
        contention: ContentionLevel,
    ) -> Vec<&BestPractice> {
        let mut results: Vec<&BestPractice> = self
            .practices
            .values()
            .filter(|p| p.category == category && p.contention <= contention)
            .collect();
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        results
    }

    /// Assess knowledge completeness — what fraction of the protocol × contention
    /// space has at least one validated protocol entry
    pub fn knowledge_completeness(&mut self) -> f32 {
        let categories = [
            ProtocolCategory::Auction,
            ProtocolCategory::FixedQuota,
            ProtocolCategory::ProportionalShare,
            ProtocolCategory::PriorityBased,
            ProtocolCategory::NashBargaining,
            ProtocolCategory::VickreyAuction,
            ProtocolCategory::RoundRobin,
        ];
        let contentions = [
            ContentionLevel::Low,
            ContentionLevel::Medium,
            ContentionLevel::High,
            ContentionLevel::Extreme,
        ];
        let total_cells = categories.len() * contentions.len();
        let mut covered: usize = 0;

        for &cat in &categories {
            for &cont in &contentions {
                let has_entry = self
                    .protocols
                    .values()
                    .any(|p| p.category == cat && p.best_contention <= cont && p.validated);
                if has_entry {
                    covered += 1;
                }
            }
        }

        let completeness = covered as f32 / total_cells as f32;
        self.stats.knowledge_completeness = completeness;

        // Update relevance for aging entries
        let mut high_count: u64 = 0;
        let current_tick = self.tick;
        for proto in self.protocols.values_mut() {
            let age = current_tick.saturating_sub(proto.added_tick) as f32;
            proto.relevance = (1.0 - RELEVANCE_DECAY * age).max(0.0);
            if proto.relevance >= HIGH_RELEVANCE {
                high_count += 1;
            }
        }
        self.stats.high_relevance_count = high_count;
        self.stats.avg_relevance_ema = EMA_ALPHA * completeness
            + (1.0 - EMA_ALPHA) * self.stats.avg_relevance_ema;

        completeness
    }

    /// Detect novelty — does a proposed protocol represent new knowledge?
    pub fn is_novel(&mut self, name: &str, category: ProtocolCategory) -> bool {
        let name_hash = fnv1a_hash(name.as_bytes());
        let novel = !self.protocols.values().any(|p| {
            let p_hash = fnv1a_hash(p.name.as_bytes());
            let xor_dist = (name_hash ^ p_hash).count_ones();
            xor_dist < 10 && p.category == category
        });
        if novel {
            self.stats.novelty_detections += 1;
        }
        novel
    }

    /// Get current literature statistics
    pub fn stats(&self) -> &LiteratureStats {
        &self.stats
    }
}
