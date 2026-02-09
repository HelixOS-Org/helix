// SPDX-License-Identifier: GPL-2.0
//! # Apps Journal — Application Research Publication Engine
//!
//! Records every classification experiment, feature discovery, and prediction
//! improvement into a structured research journal. Entries are indexed by
//! topic, time, and impact score. The journal supports full-text search via
//! FNV hashing of terms, citation tracking between entries, and impact
//! scoring based on downstream adoption of discovered improvements.
//!
//! The engine that writes its own research papers about app understanding.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONSTANTS
// ============================================================================

const MAX_ENTRIES: usize = 1024;
const MAX_CITATIONS_PER_ENTRY: usize = 32;
const MAX_SEARCH_RESULTS: usize = 64;
const IMPACT_DECAY_RATE: f32 = 0.002;
const EMA_ALPHA: f32 = 0.10;
const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;
const TERM_INDEX_LIMIT: usize = 4096;
const TREND_WINDOW: usize = 64;

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
// JOURNAL ENTRY TYPES
// ============================================================================

/// Type of journal entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntryType {
    ClassificationExperiment,
    FeatureDiscovery,
    PredictionImprovement,
    ValidationResult,
    ReviewNote,
    SynthesisReport,
}

/// Publication status
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PublicationStatus {
    Draft,
    Published,
    Retracted,
    Superseded,
}

/// A journal entry
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub entry_id: u64,
    pub entry_type: EntryType,
    pub title: String,
    pub abstract_text: String,
    pub body: String,
    pub status: PublicationStatus,
    pub impact_score: f32,
    pub citations_outgoing: Vec<u64>,
    pub citations_incoming: u64,
    pub created_tick: u64,
    pub updated_tick: u64,
    pub experiment_id: Option<u64>,
    pub discovery_id: Option<u64>,
    pub topic_hash: u64,
}

/// Search result from journal queries
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub entry_id: u64,
    pub title: String,
    pub relevance_score: f32,
    pub entry_type: EntryType,
    pub impact_score: f32,
}

/// Impact assessment of a finding
#[derive(Debug, Clone)]
pub struct ImpactAssessment {
    pub entry_id: u64,
    pub direct_impact: f32,
    pub citation_impact: f32,
    pub total_impact: f32,
    pub downstream_entries: usize,
    pub adoption_rate: f32,
}

/// Trend in research output
#[derive(Debug, Clone)]
pub struct ResearchTrend {
    pub topic_hash: u64,
    pub entry_count: usize,
    pub avg_impact: f32,
    pub growth_rate: f32,
    pub most_recent_tick: u64,
}

// ============================================================================
// JOURNAL STATS
// ============================================================================

/// Aggregate journal statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct JournalStats {
    pub total_entries: u64,
    pub total_published: u64,
    pub total_retracted: u64,
    pub total_citations: u64,
    pub avg_impact_ema: f32,
    pub avg_citations_per_entry: f32,
    pub search_queries: u64,
    pub experiments_recorded: u64,
    pub discoveries_published: u64,
}

// ============================================================================
// TERM INDEX
// ============================================================================

/// Inverted index mapping term hashes to entry IDs for full-text search
#[derive(Debug)]
struct TermIndex {
    index: BTreeMap<u64, Vec<u64>>,
}

impl TermIndex {
    fn new() -> Self {
        Self {
            index: BTreeMap::new(),
        }
    }

    fn index_text(&mut self, entry_id: u64, text: &str) {
        let bytes = text.as_bytes();
        let mut start = 0;
        for i in 0..=bytes.len() {
            let is_sep = i == bytes.len()
                || bytes[i] == b' '
                || bytes[i] == b'\n'
                || bytes[i] == b'\t'
                || bytes[i] == b',';
            if is_sep && i > start {
                let term_hash = fnv1a_hash(&bytes[start..i]);
                let entries = self.index.entry(term_hash).or_insert_with(Vec::new);
                if !entries.contains(&entry_id) {
                    entries.push(entry_id);
                }
                start = i + 1;
            } else if is_sep {
                start = i + 1;
            }
        }
        // Enforce size limit
        while self.index.len() > TERM_INDEX_LIMIT {
            if let Some(&first_key) = self.index.keys().next() {
                self.index.remove(&first_key);
            }
        }
    }

    fn search(&self, query: &str) -> Vec<(u64, f32)> {
        let bytes = query.as_bytes();
        let mut term_hashes: Vec<u64> = Vec::new();
        let mut start = 0;
        for i in 0..=bytes.len() {
            let is_sep = i == bytes.len()
                || bytes[i] == b' '
                || bytes[i] == b'\n'
                || bytes[i] == b'\t';
            if is_sep && i > start {
                term_hashes.push(fnv1a_hash(&bytes[start..i]));
                start = i + 1;
            } else if is_sep {
                start = i + 1;
            }
        }
        if term_hashes.is_empty() {
            return Vec::new();
        }

        let mut scores: BTreeMap<u64, f32> = BTreeMap::new();
        let term_count = term_hashes.len() as f32;
        for th in &term_hashes {
            if let Some(entries) = self.index.get(th) {
                for &eid in entries {
                    let score = scores.entry(eid).or_insert(0.0);
                    *score += 1.0 / term_count;
                }
            }
        }
        let mut results: Vec<(u64, f32)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        results.truncate(MAX_SEARCH_RESULTS);
        results
    }
}

// ============================================================================
// APPS JOURNAL
// ============================================================================

/// Structured research journal for app classification research
#[derive(Debug)]
pub struct AppsJournal {
    entries: BTreeMap<u64, JournalEntry>,
    term_index: TermIndex,
    topic_trends: BTreeMap<u64, Vec<u64>>,
    rng_state: u64,
    current_tick: u64,
    stats: JournalStats,
}

impl AppsJournal {
    /// Create a new journal with a seed
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            term_index: TermIndex::new(),
            topic_trends: BTreeMap::new(),
            rng_state: seed | 1,
            current_tick: 0,
            stats: JournalStats::default(),
        }
    }

    /// Record a finding in the journal
    pub fn record_finding(
        &mut self,
        entry_type: EntryType,
        title: String,
        abstract_text: String,
        body: String,
        experiment_id: Option<u64>,
        discovery_id: Option<u64>,
        tick: u64,
    ) -> u64 {
        self.current_tick = tick;
        let id = fnv1a_hash(title.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let topic_hash = fnv1a_hash(abstract_text.as_bytes());

        let entry = JournalEntry {
            entry_id: id,
            entry_type,
            title: title.clone(),
            abstract_text: abstract_text.clone(),
            body: body.clone(),
            status: PublicationStatus::Draft,
            impact_score: 0.0,
            citations_outgoing: Vec::new(),
            citations_incoming: 0,
            created_tick: tick,
            updated_tick: tick,
            experiment_id,
            discovery_id,
            topic_hash,
        };

        if self.entries.len() < MAX_ENTRIES {
            // Index for search
            self.term_index.index_text(id, &title);
            self.term_index.index_text(id, &abstract_text);
            self.term_index.index_text(id, &body);

            // Track topic trend
            let trend = self.topic_trends.entry(topic_hash).or_insert_with(Vec::new);
            trend.push(tick);
            if trend.len() > TREND_WINDOW {
                trend.remove(0);
            }

            self.entries.insert(id, entry);
            self.stats.total_entries += 1;

            if matches!(entry_type, EntryType::ClassificationExperiment) {
                self.stats.experiments_recorded += 1;
            }
        }
        id
    }

    /// Publish a draft entry
    pub fn publish_result(&mut self, entry_id: u64, tick: u64) -> bool {
        self.current_tick = tick;
        let entry = match self.entries.get_mut(&entry_id) {
            Some(e) => e,
            None => return false,
        };
        if entry.status != PublicationStatus::Draft {
            return false;
        }
        entry.status = PublicationStatus::Published;
        entry.updated_tick = tick;
        self.stats.total_published += 1;

        if entry.discovery_id.is_some() {
            self.stats.discoveries_published += 1;
        }
        true
    }

    /// Search findings by query string
    pub fn search_findings(&mut self, query: &str) -> Vec<SearchResult> {
        self.stats.search_queries += 1;
        let raw_results = self.term_index.search(query);
        let mut results: Vec<SearchResult> = Vec::new();
        for (eid, relevance) in raw_results {
            if let Some(entry) = self.entries.get(&eid) {
                results.push(SearchResult {
                    entry_id: eid,
                    title: entry.title.clone(),
                    relevance_score: relevance,
                    entry_type: entry.entry_type,
                    impact_score: entry.impact_score,
                });
            }
        }
        results
    }

    /// Assess the impact of a specific finding
    pub fn impact_assessment(&self, entry_id: u64) -> Option<ImpactAssessment> {
        let entry = self.entries.get(&entry_id)?;
        let direct_impact = entry.impact_score;
        let citation_impact = entry.citations_incoming as f32 * 0.1;

        // Count downstream entries that cite this one
        let mut downstream = 0usize;
        for other in self.entries.values() {
            if other.citations_outgoing.contains(&entry_id) {
                downstream += 1;
            }
        }

        let adoption = if self.stats.total_published > 0 {
            downstream as f32 / self.stats.total_published as f32
        } else {
            0.0
        };

        let total = direct_impact + citation_impact + adoption * 0.5;

        Some(ImpactAssessment {
            entry_id,
            direct_impact,
            citation_impact,
            total_impact: total.clamp(0.0, 10.0),
            downstream_entries: downstream,
            adoption_rate: adoption.clamp(0.0, 1.0),
        })
    }

    /// Analyze trends in research topics
    pub fn trend_analysis(&self) -> Vec<ResearchTrend> {
        let mut trends: Vec<ResearchTrend> = Vec::new();
        for (&topic_hash, ticks) in &self.topic_trends {
            if ticks.is_empty() {
                continue;
            }
            let count = ticks.len();
            let most_recent = ticks.last().copied().unwrap_or(0);

            // Compute growth rate: compare first half to second half
            let mid = count / 2;
            let growth_rate = if mid > 0 && count > mid {
                let first_half = mid as f32;
                let second_half = (count - mid) as f32;
                (second_half - first_half) / first_half
            } else {
                0.0
            };

            // Average impact of entries in this topic
            let mut impact_sum: f32 = 0.0;
            let mut impact_count: usize = 0;
            for entry in self.entries.values() {
                if entry.topic_hash == topic_hash {
                    impact_sum += entry.impact_score;
                    impact_count += 1;
                }
            }
            let avg_impact = if impact_count > 0 {
                impact_sum / impact_count as f32
            } else {
                0.0
            };

            trends.push(ResearchTrend {
                topic_hash,
                entry_count: count,
                avg_impact,
                growth_rate,
                most_recent_tick: most_recent,
            });
        }
        trends.sort_by(|a, b| {
            b.growth_rate
                .partial_cmp(&a.growth_rate)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        trends
    }

    /// Add a citation from one entry to another
    pub fn add_citation(&mut self, from_id: u64, to_id: u64) -> bool {
        let from_entry = match self.entries.get_mut(&from_id) {
            Some(e) => e,
            None => return false,
        };
        if from_entry.citations_outgoing.len() >= MAX_CITATIONS_PER_ENTRY {
            return false;
        }
        if from_entry.citations_outgoing.contains(&to_id) {
            return false;
        }
        from_entry.citations_outgoing.push(to_id);
        self.stats.total_citations += 1;

        if let Some(to_entry) = self.entries.get_mut(&to_id) {
            to_entry.citations_incoming += 1;
            // Boost impact of cited entry
            to_entry.impact_score += 0.05;
        }
        true
    }

    /// Get aggregate stats
    pub fn stats(&self) -> JournalStats {
        self.stats
    }

    /// Get entry by id
    pub fn entry(&self, entry_id: u64) -> Option<&JournalEntry> {
        self.entries.get(&entry_id)
    }
}
