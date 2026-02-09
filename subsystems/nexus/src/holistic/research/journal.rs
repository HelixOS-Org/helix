// SPDX-License-Identifier: GPL-2.0
//! # Holistic Journal — System-Wide Research Archive
//!
//! Archives ALL research results from every NEXUS subsystem — bridge,
//! application, cooperation, and holistic — into a single, cross-referenced
//! knowledge repository. Every discovery, experiment, hypothesis outcome,
//! and validation certificate is recorded with full provenance.
//!
//! The journal supports cross-referencing: discoveries from the scheduler
//! domain can be linked to IPC throughput improvements, enabling the
//! system to trace causal chains across subsystem boundaries. Impact
//! analysis quantifies the downstream effects of each discovery, while
//! a continuously updated knowledge graph maps relationships between
//! all known research artefacts.
//!
//! The engine that remembers everything the kernel has ever learned.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ENTRIES: usize = 4096;
const MAX_CROSS_REFS: usize = 2048;
const MAX_GRAPH_NODES: usize = 1024;
const IMPACT_DECAY: f32 = 0.97;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const VELOCITY_WINDOW: usize = 128;
const BREAKTHROUGH_IMPACT_MIN: f32 = 0.80;
const STALE_AGE_TICKS: u64 = 100_000;

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

/// Origin domain of a research discovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResearchDomain {
    Bridge,
    Application,
    Cooperation,
    HolisticExplorer,
    HolisticHypothesis,
    HolisticExperiment,
    HolisticValidator,
    HolisticSynthesis,
}

/// Severity / importance tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportanceTier {
    Incremental,
    Notable,
    Significant,
    Breakthrough,
}

/// A journal entry recording a single discovery
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub id: u64,
    pub domain: ResearchDomain,
    pub title: String,
    pub summary: String,
    pub importance: ImportanceTier,
    pub impact_score: f32,
    pub confidence: f32,
    pub cross_ref_ids: Vec<u64>,
    pub created_tick: u64,
    pub updated_tick: u64,
}

/// A cross-reference link between two journal entries
#[derive(Debug, Clone)]
pub struct CrossReference {
    pub from_id: u64,
    pub to_id: u64,
    pub relation: CrossRefRelation,
    pub strength: f32,
}

/// Relationship type between cross-referenced entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CrossRefRelation {
    CausedBy,
    Enables,
    Contradicts,
    Reinforces,
    Supersedes,
    RelatedTo,
}

/// Impact analysis result for a discovery
#[derive(Debug, Clone)]
pub struct ImpactAnalysis {
    pub discovery_id: u64,
    pub direct_impact: f32,
    pub transitive_impact: f32,
    pub affected_domains: Vec<ResearchDomain>,
    pub downstream_count: usize,
}

/// Node in the knowledge graph
#[derive(Debug, Clone)]
pub struct KnowledgeNode {
    pub id: u64,
    pub label: String,
    pub domain: ResearchDomain,
    pub importance: f32,
    pub edges: Vec<(u64, f32)>,
}

/// Research velocity measurement
#[derive(Debug, Clone)]
pub struct VelocityReport {
    pub window_size: usize,
    pub discoveries_in_window: usize,
    pub breakthroughs_in_window: usize,
    pub avg_impact: f32,
    pub acceleration: f32,
}

/// Journal statistics
#[derive(Debug, Clone)]
pub struct JournalStats {
    pub total_entries: u64,
    pub cross_references: u64,
    pub breakthroughs: u64,
    pub avg_impact_ema: f32,
    pub research_velocity: f32,
    pub knowledge_nodes: u64,
    pub domains_active: u64,
}

// ============================================================================
// HOLISTIC JOURNAL
// ============================================================================

/// System-wide research journal and archive
pub struct HolisticJournal {
    entries: BTreeMap<u64, JournalEntry>,
    cross_refs: Vec<CrossReference>,
    knowledge_nodes: BTreeMap<u64, KnowledgeNode>,
    velocity_log: Vec<(u64, usize)>,
    rng_state: u64,
    stats: JournalStats,
}

impl HolisticJournal {
    /// Create a new holistic journal
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            cross_refs: Vec::new(),
            knowledge_nodes: BTreeMap::new(),
            velocity_log: Vec::new(),
            rng_state: seed | 1,
            stats: JournalStats {
                total_entries: 0, cross_references: 0, breakthroughs: 0,
                avg_impact_ema: 0.0, research_velocity: 0.0,
                knowledge_nodes: 0, domains_active: 0,
            },
        }
    }

    /// Record a new discovery in the journal
    pub fn record_discovery(
        &mut self, domain: ResearchDomain, title: String, summary: String,
        importance: ImportanceTier, impact: f32, confidence: f32, tick: u64,
    ) -> u64 {
        let id = fnv1a_hash(title.as_bytes()) ^ fnv1a_hash(&tick.to_le_bytes());
        if self.entries.len() >= MAX_ENTRIES {
            self.evict_oldest();
        }
        let entry = JournalEntry {
            id, domain, title, summary, importance,
            impact_score: impact, confidence,
            cross_ref_ids: Vec::new(),
            created_tick: tick, updated_tick: tick,
        };
        if importance == ImportanceTier::Breakthrough {
            self.stats.breakthroughs += 1;
        }
        self.entries.insert(id, entry);
        self.stats.total_entries = self.entries.len() as u64;
        self.stats.avg_impact_ema =
            EMA_ALPHA * impact + (1.0 - EMA_ALPHA) * self.stats.avg_impact_ema;
        self.update_velocity(tick);
        self.ensure_knowledge_node(id, domain, impact);
        id
    }

    /// Create a cross-reference between two journal entries
    pub fn cross_reference(
        &mut self, from_id: u64, to_id: u64, relation: CrossRefRelation, strength: f32,
    ) -> bool {
        if !self.entries.contains_key(&from_id) || !self.entries.contains_key(&to_id) {
            return false;
        }
        if self.cross_refs.len() >= MAX_CROSS_REFS { return false; }
        self.cross_refs.push(CrossReference {
            from_id, to_id, relation, strength,
        });
        if let Some(entry) = self.entries.get_mut(&from_id) {
            if !entry.cross_ref_ids.contains(&to_id) {
                entry.cross_ref_ids.push(to_id);
            }
        }
        if let Some(entry) = self.entries.get_mut(&to_id) {
            if !entry.cross_ref_ids.contains(&from_id) {
                entry.cross_ref_ids.push(from_id);
            }
        }
        if let Some(node) = self.knowledge_nodes.get_mut(&from_id) {
            node.edges.push((to_id, strength));
        }
        if let Some(node) = self.knowledge_nodes.get_mut(&to_id) {
            node.edges.push((from_id, strength));
        }
        self.stats.cross_references = self.cross_refs.len() as u64;
        true
    }

    /// Analyse the impact of a discovery through cross-references
    pub fn impact_analysis(&self, discovery_id: u64) -> ImpactAnalysis {
        let entry = match self.entries.get(&discovery_id) {
            Some(e) => e,
            None => return ImpactAnalysis {
                discovery_id, direct_impact: 0.0, transitive_impact: 0.0,
                affected_domains: Vec::new(), downstream_count: 0,
            },
        };
        let direct = entry.impact_score;
        let mut visited: Vec<u64> = Vec::new();
        let mut queue: Vec<u64> = entry.cross_ref_ids.clone();
        let mut transitive = 0.0f32;
        let mut domains: Vec<ResearchDomain> = Vec::new();
        while let Some(next_id) = queue.pop() {
            if visited.contains(&next_id) { continue; }
            visited.push(next_id);
            if let Some(linked) = self.entries.get(&next_id) {
                transitive += linked.impact_score * IMPACT_DECAY;
                if !domains.contains(&linked.domain) {
                    domains.push(linked.domain);
                }
                for &ref_id in &linked.cross_ref_ids {
                    if !visited.contains(&ref_id) && visited.len() < 64 {
                        queue.push(ref_id);
                    }
                }
            }
        }
        ImpactAnalysis {
            discovery_id, direct_impact: direct,
            transitive_impact: transitive,
            affected_domains: domains,
            downstream_count: visited.len(),
        }
    }

    /// Build a knowledge graph of all journal entries and cross-references
    pub fn knowledge_graph(&mut self) -> &BTreeMap<u64, KnowledgeNode> {
        self.stats.knowledge_nodes = self.knowledge_nodes.len() as u64;
        let mut domain_set: Vec<ResearchDomain> = Vec::new();
        for (_, node) in &self.knowledge_nodes {
            if !domain_set.contains(&node.domain) {
                domain_set.push(node.domain);
            }
        }
        self.stats.domains_active = domain_set.len() as u64;
        &self.knowledge_nodes
    }

    /// Measure research velocity over recent ticks
    pub fn research_velocity(&mut self, tick: u64) -> VelocityReport {
        let window = self.velocity_log.len().min(VELOCITY_WINDOW);
        if window < 2 {
            return VelocityReport {
                window_size: window, discoveries_in_window: 0,
                breakthroughs_in_window: 0, avg_impact: 0.0, acceleration: 0.0,
            };
        }
        let recent = &self.velocity_log[self.velocity_log.len() - window..];
        let total_disc: usize = recent.iter().map(|(_, c)| c).sum();
        let bt_count = self.entries.values()
            .filter(|e| e.importance == ImportanceTier::Breakthrough
                && tick.saturating_sub(e.created_tick) < STALE_AGE_TICKS)
            .count();
        let avg_impact = self.stats.avg_impact_ema;
        let first_half = &recent[..window / 2];
        let second_half = &recent[window / 2..];
        let rate_1: f32 = first_half.iter().map(|(_, c)| *c as f32).sum::<f32>()
            / first_half.len().max(1) as f32;
        let rate_2: f32 = second_half.iter().map(|(_, c)| *c as f32).sum::<f32>()
            / second_half.len().max(1) as f32;
        let accel = rate_2 - rate_1;
        self.stats.research_velocity = rate_2;
        VelocityReport {
            window_size: window, discoveries_in_window: total_disc,
            breakthroughs_in_window: bt_count, avg_impact, acceleration: accel,
        }
    }

    /// Detect breakthroughs by scanning recent high-impact entries
    pub fn breakthrough_detection(&self, tick: u64) -> Vec<u64> {
        let mut breakthroughs = Vec::new();
        for (&id, entry) in &self.entries {
            if entry.impact_score >= BREAKTHROUGH_IMPACT_MIN
                && tick.saturating_sub(entry.created_tick) < STALE_AGE_TICKS
            {
                breakthroughs.push(id);
            }
        }
        breakthroughs.sort_by(|a, b| {
            let ia = self.entries.get(a).map(|e| e.impact_score).unwrap_or(0.0);
            let ib = self.entries.get(b).map(|e| e.impact_score).unwrap_or(0.0);
            ib.partial_cmp(&ia).unwrap_or(core::cmp::Ordering::Equal)
        });
        breakthroughs
    }

    /// Current statistics snapshot
    pub fn stats(&self) -> &JournalStats { &self.stats }

    // ── private helpers ─────────────────────────────────────────────────

    fn evict_oldest(&mut self) {
        if let Some((&oldest_id, _)) = self.entries.iter().next() {
            self.entries.remove(&oldest_id);
            self.knowledge_nodes.remove(&oldest_id);
        }
    }

    fn update_velocity(&mut self, tick: u64) {
        if let Some(last) = self.velocity_log.last_mut() {
            if last.0 == tick {
                last.1 += 1;
                return;
            }
        }
        self.velocity_log.push((tick, 1));
    }

    fn ensure_knowledge_node(&mut self, id: u64, domain: ResearchDomain, imp: f32) {
        if self.knowledge_nodes.len() >= MAX_GRAPH_NODES { return; }
        if !self.knowledge_nodes.contains_key(&id) {
            self.knowledge_nodes.insert(id, KnowledgeNode {
                id, label: String::from("discovery"),
                domain, importance: imp, edges: Vec::new(),
            });
        }
    }
}
