// SPDX-License-Identifier: GPL-2.0
//! # Bridge Journal — Research Publication Engine
//!
//! Records every experiment, discovery, and validation into a structured
//! research journal. Entries are indexed by topic, time, and impact score.
//! The journal supports full-text search via FNV hashing of terms, citation
//! tracking between entries, and impact scoring based on downstream adoption.
//!
//! The bridge that writes its own research papers.

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
    ExperimentRecord,
    DiscoveryPublication,
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

/// A citation from one entry to another
#[derive(Debug, Clone, Copy)]
pub struct Citation {
    pub from_entry_id: u64,
    pub to_entry_id: u64,
    pub relevance: f32,
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

/// Citation graph edge for analysis
#[derive(Debug, Clone, Copy)]
pub struct CitationEdge {
    pub from_id: u64,
    pub to_id: u64,
    pub weight: f32,
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
        // Split on whitespace and index each word by FNV hash
        let mut start = 0;
        let bytes = text.as_bytes();
        for i in 0..=bytes.len() {
            let is_sep = i == bytes.len()
                || bytes[i] == b' '
                || bytes[i] == b'\n'
                || bytes[i] == b'\t'
                || bytes[i] == b','
                || bytes[i] == b'.';
            if is_sep && i > start {
                let word = &bytes[start..i];
                // Lowercase for case-insensitive matching
                let mut lower: Vec<u8> = Vec::with_capacity(word.len());
                for &b in word {
                    lower.push(if b >= b'A' && b <= b'Z' { b + 32 } else { b });
                }
                let hash = fnv1a_hash(&lower);
                let entries = self.index.entry(hash).or_insert_with(Vec::new);
                if !entries.contains(&entry_id) {
                    entries.push(entry_id);
                }
                start = i + 1;
            } else if is_sep {
                start = i + 1;
            }
        }

        // Limit index size
        while self.index.len() > TERM_INDEX_LIMIT {
            if let Some(&first_key) = self.index.keys().next() {
                self.index.remove(&first_key);
            }
        }
    }

    fn search(&self, query: &str) -> BTreeMap<u64, u32> {
        let mut scores: BTreeMap<u64, u32> = BTreeMap::new();
        let bytes = query.as_bytes();
        let mut start = 0;
        for i in 0..=bytes.len() {
            let is_sep = i == bytes.len()
                || bytes[i] == b' '
                || bytes[i] == b'\n'
                || bytes[i] == b'\t';
            if is_sep && i > start {
                let word = &bytes[start..i];
                let mut lower: Vec<u8> = Vec::with_capacity(word.len());
                for &b in word {
                    lower.push(if b >= b'A' && b <= b'Z' { b + 32 } else { b });
                }
                let hash = fnv1a_hash(&lower);
                if let Some(entries) = self.index.get(&hash) {
                    for &eid in entries {
                        let count = scores.entry(eid).or_insert(0);
                        *count += 1;
                    }
                }
                start = i + 1;
            } else if is_sep {
                start = i + 1;
            }
        }
        scores
    }
}

// ============================================================================
// BRIDGE JOURNAL
// ============================================================================

/// Research journal and publication engine
#[derive(Debug)]
pub struct BridgeJournal {
    entries: BTreeMap<u64, JournalEntry>,
    term_index: TermIndex,
    citation_edges: Vec<CitationEdge>,
    rng_state: u64,
    current_tick: u64,
    stats: JournalStats,
}

impl BridgeJournal {
    /// Create a new journal
    pub fn new(seed: u64) -> Self {
        Self {
            entries: BTreeMap::new(),
            term_index: TermIndex::new(),
            citation_edges: Vec::new(),
            rng_state: seed | 1,
            current_tick: 0,
            stats: JournalStats::default(),
        }
    }

    /// Record an experiment as a journal entry
    pub fn record_experiment(
        &mut self,
        experiment_id: u64,
        title: String,
        abstract_text: String,
        body: String,
        tick: u64,
    ) -> u64 {
        self.current_tick = tick;
        let entry_id = fnv1a_hash(title.as_bytes()) ^ xorshift64(&mut self.rng_state);
        let topic_hash = fnv1a_hash(abstract_text.as_bytes());

        let entry = JournalEntry {
            entry_id,
            entry_type: EntryType::ExperimentRecord,
            title: title.clone(),
            abstract_text: abstract_text.clone(),
            body: body.clone(),
            status: PublicationStatus::Draft,
            impact_score: 0.0,
            citations_outgoing: Vec::new(),
            citations_incoming: 0,
            created_tick: tick,
            updated_tick: tick,
            experiment_id: Some(experiment_id),
            discovery_id: None,
            topic_hash,
        };

        // Index text for search
        self.term_index.index_text(entry_id, &title);
        self.term_index.index_text(entry_id, &abstract_text);
        self.term_index.index_text(entry_id, &body);

        self.entries.insert(entry_id, entry);
        self.stats.total_entries += 1;
        self.stats.experiments_recorded += 1;
        self.evict_if_needed();
        entry_id
    }

    /// Publish a discovery as a journal entry
    pub fn publish_discovery(
        &mut self,
        discovery_id: u64,
        title: String,
        abstract_text: String,
        body: String,
        cited_entries: Vec<u64>,
        tick: u64,
    ) -> u64 {
        self.current_tick = tick;
        let entry_id =
            fnv1a_hash(title.as_bytes()) ^ fnv1a_hash(&discovery_id.to_le_bytes());
        let topic_hash = fnv1a_hash(abstract_text.as_bytes());

        // Track citations
        let mut valid_citations: Vec<u64> = Vec::new();
        for &cited_id in &cited_entries {
            if self.entries.contains_key(&cited_id) {
                valid_citations.push(cited_id);
                if let Some(cited_entry) = self.entries.get_mut(&cited_id) {
                    cited_entry.citations_incoming += 1;
                    cited_entry.impact_score += 0.1;
                }
                self.citation_edges.push(CitationEdge {
                    from_id: entry_id,
                    to_id: cited_id,
                    weight: 1.0,
                });
                self.stats.total_citations += 1;
            }
        }

        let entry = JournalEntry {
            entry_id,
            entry_type: EntryType::DiscoveryPublication,
            title: title.clone(),
            abstract_text: abstract_text.clone(),
            body: body.clone(),
            status: PublicationStatus::Published,
            impact_score: 0.1,
            citations_outgoing: valid_citations,
            citations_incoming: 0,
            created_tick: tick,
            updated_tick: tick,
            experiment_id: None,
            discovery_id: Some(discovery_id),
            topic_hash,
        };

        self.term_index.index_text(entry_id, &title);
        self.term_index.index_text(entry_id, &abstract_text);
        self.term_index.index_text(entry_id, &body);

        self.entries.insert(entry_id, entry);
        self.stats.total_entries += 1;
        self.stats.total_published += 1;
        self.stats.discoveries_published += 1;
        self.evict_if_needed();
        entry_id
    }

    /// Search the journal by query string
    pub fn search_journal(&mut self, query: &str) -> Vec<SearchResult> {
        self.stats.search_queries += 1;
        let term_scores = self.term_index.search(query);

        let mut results: Vec<SearchResult> = term_scores
            .iter()
            .filter_map(|(&eid, &score)| {
                self.entries.get(&eid).map(|entry| {
                    let relevance = score as f32 * (1.0 + entry.impact_score);
                    SearchResult {
                        entry_id: eid,
                        title: entry.title.clone(),
                        relevance_score: relevance,
                        entry_type: entry.entry_type,
                        impact_score: entry.impact_score,
                    }
                })
            })
            .collect();

        results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(core::cmp::Ordering::Equal)
        });
        results.truncate(MAX_SEARCH_RESULTS);
        results
    }

    /// Compute impact score for an entry based on citation count and downstream effects
    pub fn impact_score(&mut self, entry_id: u64) -> f32 {
        let entry = match self.entries.get(&entry_id) {
            Some(e) => e,
            None => return 0.0,
        };

        // Impact = direct citations + recursive downstream citations (depth 2)
        let direct = entry.citations_incoming as f32;
        let mut downstream: f32 = 0.0;
        for edge in &self.citation_edges {
            if edge.to_id == entry_id {
                // Someone cites this entry; check if *they* are cited
                let citing_entry_id = edge.from_id;
                let citing_citations = self
                    .citation_edges
                    .iter()
                    .filter(|e| e.to_id == citing_entry_id)
                    .count() as f32;
                downstream += citing_citations * 0.5;
            }
        }

        let age_ticks = self.current_tick.saturating_sub(entry.created_tick);
        let recency = 1.0 / (1.0 + age_ticks as f32 * IMPACT_DECAY_RATE);
        let impact = (direct + downstream * 0.5) * recency + 0.01;

        if let Some(e) = self.entries.get_mut(&entry_id) {
            e.impact_score = impact;
        }

        self.stats.avg_impact_ema =
            EMA_ALPHA * impact + (1.0 - EMA_ALPHA) * self.stats.avg_impact_ema;
        impact
    }

    /// Build and return the citation graph
    pub fn citation_graph(&self) -> Vec<CitationEdge> {
        self.citation_edges.clone()
    }

    /// Get total citation count for an entry
    pub fn citation_count(&self, entry_id: u64) -> u64 {
        self.entries
            .get(&entry_id)
            .map_or(0, |e| e.citations_incoming)
    }

    /// Add a citation between two existing entries
    pub fn add_citation(&mut self, from_id: u64, to_id: u64) -> bool {
        if !self.entries.contains_key(&from_id) || !self.entries.contains_key(&to_id) {
            return false;
        }
        // Avoid duplicate
        let exists = self
            .citation_edges
            .iter()
            .any(|e| e.from_id == from_id && e.to_id == to_id);
        if exists {
            return false;
        }

        if let Some(from_entry) = self.entries.get_mut(&from_id) {
            if from_entry.citations_outgoing.len() < MAX_CITATIONS_PER_ENTRY {
                from_entry.citations_outgoing.push(to_id);
            }
        }
        if let Some(to_entry) = self.entries.get_mut(&to_id) {
            to_entry.citations_incoming += 1;
        }
        self.citation_edges.push(CitationEdge {
            from_id,
            to_id,
            weight: 1.0,
        });
        self.stats.total_citations += 1;

        if self.stats.total_entries > 0 {
            self.stats.avg_citations_per_entry =
                self.stats.total_citations as f32 / self.stats.total_entries as f32;
        }
        true
    }

    /// Get an entry by ID
    pub fn get_entry(&self, entry_id: u64) -> Option<&JournalEntry> {
        self.entries.get(&entry_id)
    }

    /// Retract a publication
    pub fn retract(&mut self, entry_id: u64) -> bool {
        if let Some(e) = self.entries.get_mut(&entry_id) {
            e.status = PublicationStatus::Retracted;
            self.stats.total_retracted += 1;
            true
        } else {
            false
        }
    }

    fn evict_if_needed(&mut self) {
        while self.entries.len() > MAX_ENTRIES {
            let oldest_retracted = self
                .entries
                .iter()
                .filter(|(_, e)| e.status == PublicationStatus::Retracted)
                .min_by_key(|(_, e)| e.created_tick)
                .map(|(&k, _)| k);
            let target = oldest_retracted.or_else(|| {
                self.entries
                    .iter()
                    .min_by_key(|(_, e)| e.created_tick)
                    .map(|(&k, _)| k)
            });
            if let Some(k) = target {
                self.entries.remove(&k);
            } else {
                break;
            }
        }
    }

    /// Get aggregate stats
    pub fn stats(&self) -> JournalStats {
        self.stats
    }
}
