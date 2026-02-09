// SPDX-License-Identifier: GPL-2.0
//! # Bridge Omniscient — Total Knowledge of the Syscall Space
//!
//! The bridge that knows *everything*. Every syscall pattern, every process
//! behaviour, every optimisation path is captured in a knowledge graph. Each
//! knowledge node links syscalls to their effects and the resources they
//! touch, enabling the bridge to answer arbitrary queries about the
//! syscall space instantly.
//!
//! FNV-1a hashing indexes the graph; xorshift64 drives stochastic sampling
//! for completeness audits; EMA tracks knowledge freshness.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_KNOWLEDGE_NODES: usize = 2048;
const MAX_EDGES_PER_NODE: usize = 32;
const MAX_QUERY_RESULTS: usize = 64;
const MAX_MISSING_REPORT: usize = 128;
const EMA_ALPHA: f32 = 0.10;
const FRESHNESS_DECAY: f32 = 0.998;
const COMPLETENESS_TARGET: f32 = 0.95;
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
// KNOWLEDGE TYPES
// ============================================================================

/// Category of knowledge stored in a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum KnowledgeCategory {
    SyscallPattern,
    ProcessBehaviour,
    ResourceEffect,
    OptimisationPath,
    SecurityConstraint,
    LatencyProfile,
    ThroughputProfile,
    FailureMode,
}

/// Strength of an edge between two knowledge nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeStrength {
    Weak,
    Moderate,
    Strong,
    Causal,
}

/// A directed edge in the knowledge graph.
#[derive(Debug, Clone)]
pub struct KnowledgeEdge {
    pub target_id: u64,
    pub strength: EdgeStrength,
    pub label: String,
    pub weight: f32,
    pub observation_count: u64,
}

/// A single node in the knowledge graph.
#[derive(Debug, Clone)]
pub struct KnowledgeNode {
    pub node_id: u64,
    pub name: String,
    pub category: KnowledgeCategory,
    pub edges: Vec<KnowledgeEdge>,
    pub freshness: f32,
    pub confidence: f32,
    pub access_count: u64,
    pub last_update_tick: u64,
    pub data_hash: u64,
}

/// Result of a knowledge query.
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub node_id: u64,
    pub name: String,
    pub relevance: f32,
    pub category: KnowledgeCategory,
    pub connected_nodes: usize,
    pub confidence: f32,
}

/// A gap identified in the knowledge base.
#[derive(Debug, Clone)]
pub struct KnowledgeGap {
    pub description: String,
    pub category: KnowledgeCategory,
    pub severity: f32,
    pub estimated_effort: f32,
    pub related_nodes: Vec<u64>,
}

// ============================================================================
// OMNISCIENCE STATS
// ============================================================================

/// Aggregate statistics for the omniscient knowledge graph.
#[derive(Debug, Clone, Copy, Default)]
pub struct OmniscienceStats {
    pub total_nodes: u64,
    pub total_edges: u64,
    pub avg_freshness: f32,
    pub avg_confidence: f32,
    pub completeness: f32,
    pub categories_covered: u32,
    pub queries_served: u64,
    pub knowledge_updates: u64,
    pub omniscience_score: f32,
}

// ============================================================================
// CATEGORY COVERAGE TRACKER
// ============================================================================

#[derive(Debug, Clone)]
struct CategoryCoverage {
    expected_nodes: u64,
    actual_nodes: u64,
    total_confidence: f32,
    freshness_ema: f32,
}

impl CategoryCoverage {
    fn new(expected: u64) -> Self {
        Self {
            expected_nodes: expected,
            actual_nodes: 0,
            total_confidence: 0.0,
            freshness_ema: 1.0,
        }
    }

    fn record_node(&mut self, confidence: f32, freshness: f32) {
        self.actual_nodes += 1;
        self.total_confidence += confidence;
        self.freshness_ema = EMA_ALPHA * freshness + (1.0 - EMA_ALPHA) * self.freshness_ema;
    }

    fn coverage_ratio(&self) -> f32 {
        if self.expected_nodes == 0 {
            return 1.0;
        }
        (self.actual_nodes as f32 / self.expected_nodes as f32).min(1.0)
    }

    fn avg_confidence(&self) -> f32 {
        if self.actual_nodes == 0 {
            return 0.0;
        }
        self.total_confidence / self.actual_nodes as f32
    }
}

// ============================================================================
// BRIDGE OMNISCIENT
// ============================================================================

/// Total knowledge engine for the syscall space. Maintains a complete knowledge
/// graph linking syscalls ↔ effects ↔ resources, enabling omniscient queries.
#[derive(Debug)]
pub struct BridgeOmniscient {
    nodes: BTreeMap<u64, KnowledgeNode>,
    category_coverage: BTreeMap<u8, CategoryCoverage>,
    queries_served: u64,
    knowledge_updates: u64,
    tick: u64,
    rng_state: u64,
    global_freshness_ema: f32,
    global_confidence_ema: f32,
}

impl BridgeOmniscient {
    /// Create a new omniscient knowledge engine.
    pub fn new(seed: u64) -> Self {
        let mut coverage = BTreeMap::new();
        // Each category has an expected baseline of 256 nodes.
        for cat in 0u8..8 {
            coverage.insert(cat, CategoryCoverage::new(256));
        }
        Self {
            nodes: BTreeMap::new(),
            category_coverage: coverage,
            queries_served: 0,
            knowledge_updates: 0,
            tick: 0,
            rng_state: seed | 1,
            global_freshness_ema: 1.0,
            global_confidence_ema: 0.5,
        }
    }

    /// Retrieve the complete knowledge graph snapshot — all nodes & edges.
    pub fn total_knowledge(&self) -> Vec<&KnowledgeNode> {
        self.nodes.values().collect()
    }

    /// Query the knowledge graph by keyword. Returns the most relevant nodes
    /// ranked by a relevance score combining name-hash proximity, freshness,
    /// confidence, and edge count.
    pub fn query_knowledge(&mut self, keyword: &str) -> Vec<QueryResult> {
        self.queries_served += 1;
        let keyword_hash = fnv1a_hash(keyword.as_bytes());
        let mut results = Vec::new();

        for node in self.nodes.values() {
            let name_hash = fnv1a_hash(node.name.as_bytes());
            let hash_dist = (keyword_hash ^ name_hash).count_ones() as f32 / 64.0;
            let name_match = if node
                .name
                .as_bytes()
                .windows(keyword.len().min(node.name.len()))
                .any(|w| {
                    let kw = keyword.as_bytes();
                    w.len() >= kw.len() && w[..kw.len()].iter().zip(kw).all(|(a, b)| a == b)
                }) {
                0.5
            } else {
                0.0
            };
            let relevance =
                name_match + (1.0 - hash_dist) * 0.3 + node.freshness * 0.1 + node.confidence * 0.1;

            if relevance > 0.2 {
                results.push(QueryResult {
                    node_id: node.node_id,
                    name: node.name.clone(),
                    relevance,
                    category: node.category,
                    connected_nodes: node.edges.len(),
                    confidence: node.confidence,
                });
            }
        }

        results.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        results.truncate(MAX_QUERY_RESULTS);
        results
    }

    /// Compute overall knowledge completeness as a weighted average of
    /// per-category coverage ratios and confidence levels.
    pub fn knowledge_completeness(&self) -> f32 {
        if self.category_coverage.is_empty() {
            return 0.0;
        }
        let (sum, count) = self
            .category_coverage
            .values()
            .fold((0.0f32, 0u32), |(s, c), cov| {
                let score = cov.coverage_ratio() * 0.6
                    + cov.avg_confidence() * 0.3
                    + cov.freshness_ema * 0.1;
                (s + score, c + 1)
            });
        if count == 0 { 0.0 } else { sum / count as f32 }
    }

    /// Identify gaps in the knowledge base — categories or areas where
    /// coverage falls below the target threshold.
    pub fn missing_knowledge(&mut self) -> Vec<KnowledgeGap> {
        let mut gaps = Vec::new();
        let cat_names = [
            "SyscallPattern",
            "ProcessBehaviour",
            "ResourceEffect",
            "OptimisationPath",
            "SecurityConstraint",
            "LatencyProfile",
            "ThroughputProfile",
            "FailureMode",
        ];
        let categories = [
            KnowledgeCategory::SyscallPattern,
            KnowledgeCategory::ProcessBehaviour,
            KnowledgeCategory::ResourceEffect,
            KnowledgeCategory::OptimisationPath,
            KnowledgeCategory::SecurityConstraint,
            KnowledgeCategory::LatencyProfile,
            KnowledgeCategory::ThroughputProfile,
            KnowledgeCategory::FailureMode,
        ];

        for (idx, cov) in self.category_coverage.iter() {
            let ratio = cov.coverage_ratio();
            if ratio < COMPLETENESS_TARGET {
                let severity = (COMPLETENESS_TARGET - ratio) / COMPLETENESS_TARGET;
                let i = (*idx as usize).min(cat_names.len() - 1);
                let related: Vec<u64> = self
                    .nodes
                    .values()
                    .filter(|n| n.category as u8 == *idx)
                    .map(|n| n.node_id)
                    .take(8)
                    .collect();
                gaps.push(KnowledgeGap {
                    description: String::from(cat_names[i]),
                    category: categories[i],
                    severity,
                    estimated_effort: severity * 100.0,
                    related_nodes: related,
                });
            }
        }

        // Stochastic probe: randomly sample to detect hidden gaps
        let probe_count = 4;
        for _ in 0..probe_count {
            let r = xorshift64(&mut self.rng_state);
            let cat_idx = (r % 8) as u8;
            if let Some(cov) = self.category_coverage.get(&cat_idx) {
                if cov.freshness_ema < 0.3 && cov.actual_nodes > 0 {
                    let i = (cat_idx as usize).min(cat_names.len() - 1);
                    gaps.push(KnowledgeGap {
                        description: String::from("Stale knowledge detected"),
                        category: categories[i],
                        severity: 1.0 - cov.freshness_ema,
                        estimated_effort: 20.0,
                        related_nodes: Vec::new(),
                    });
                }
            }
        }

        gaps.truncate(MAX_MISSING_REPORT);
        gaps
    }

    /// Update the knowledge graph with a new or modified node. Automatically
    /// updates coverage, confidence, freshness, and edge weights.
    pub fn knowledge_update(
        &mut self,
        name: String,
        category: KnowledgeCategory,
        confidence: f32,
        edges: Vec<(u64, EdgeStrength, String, f32)>,
    ) -> u64 {
        self.tick += 1;
        self.knowledge_updates += 1;
        let node_id = fnv1a_hash(name.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let conf = confidence.max(0.0).min(1.0);

        let edge_list: Vec<KnowledgeEdge> = edges
            .into_iter()
            .take(MAX_EDGES_PER_NODE)
            .map(|(target, strength, label, weight)| KnowledgeEdge {
                target_id: target,
                strength,
                label,
                weight: weight.max(0.0).min(1.0),
                observation_count: 1,
            })
            .collect();

        let node = KnowledgeNode {
            node_id,
            name,
            category,
            edges: edge_list,
            freshness: 1.0,
            confidence: conf,
            access_count: 0,
            last_update_tick: self.tick,
            data_hash: fnv1a_hash(&node_id.to_le_bytes()),
        };

        if let Some(cov) = self.category_coverage.get_mut(&(category as u8)) {
            cov.record_node(conf, 1.0);
        }

        self.global_freshness_ema = EMA_ALPHA * 1.0 + (1.0 - EMA_ALPHA) * self.global_freshness_ema;
        self.global_confidence_ema =
            EMA_ALPHA * conf + (1.0 - EMA_ALPHA) * self.global_confidence_ema;

        // Enforce capacity
        if self.nodes.len() >= MAX_KNOWLEDGE_NODES && !self.nodes.contains_key(&node_id) {
            // Evict the stalest node
            if let Some((&evict_id, _)) = self.nodes.iter().min_by(|a, b| {
                a.1.freshness
                    .partial_cmp(&b.1.freshness)
                    .unwrap_or(core::cmp::Ordering::Equal)
            }) {
                self.nodes.remove(&evict_id);
            }
        }

        self.nodes.insert(node_id, node);

        // Decay freshness of all other nodes
        for n in self.nodes.values_mut() {
            if n.node_id != node_id {
                n.freshness *= FRESHNESS_DECAY;
            }
        }

        node_id
    }

    /// Composite omniscience score: combines completeness, freshness,
    /// confidence, and edge density into a single [0, 1] metric.
    pub fn omniscience_score(&self) -> f32 {
        let completeness = self.knowledge_completeness();
        let freshness = self.global_freshness_ema;
        let confidence = self.global_confidence_ema;

        let total_edges: usize = self.nodes.values().map(|n| n.edges.len()).sum();
        let node_count = self.nodes.len().max(1) as f32;
        let edge_density = (total_edges as f32 / (node_count * MAX_EDGES_PER_NODE as f32)).min(1.0);

        completeness * 0.35 + freshness * 0.25 + confidence * 0.25 + edge_density * 0.15
    }

    /// Compute aggregate statistics.
    pub fn stats(&self) -> OmniscienceStats {
        let total_edges: u64 = self.nodes.values().map(|n| n.edges.len() as u64).sum();
        let categories_covered = self
            .category_coverage
            .values()
            .filter(|c| c.actual_nodes > 0)
            .count() as u32;

        OmniscienceStats {
            total_nodes: self.nodes.len() as u64,
            total_edges,
            avg_freshness: self.global_freshness_ema,
            avg_confidence: self.global_confidence_ema,
            completeness: self.knowledge_completeness(),
            categories_covered,
            queries_served: self.queries_served,
            knowledge_updates: self.knowledge_updates,
            omniscience_score: self.omniscience_score(),
        }
    }
}
