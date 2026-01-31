//! # LTM Retrieval
//!
//! Retrieval mechanisms for long-term memories.
//! Supports various retrieval strategies and cues.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// RETRIEVAL TYPES
// ============================================================================

/// Retrieval cue
#[derive(Debug, Clone)]
pub struct RetrievalCue {
    /// Cue ID
    pub id: u64,
    /// Cue type
    pub cue_type: CueType,
    /// Content
    pub content: CueContent,
    /// Weight
    pub weight: f64,
}

/// Cue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CueType {
    /// Semantic (meaning-based)
    Semantic,
    /// Episodic (context-based)
    Episodic,
    /// Temporal (time-based)
    Temporal,
    /// Associative (link-based)
    Associative,
    /// Pattern (similarity-based)
    Pattern,
}

/// Cue content
#[derive(Debug, Clone)]
pub enum CueContent {
    /// Text query
    Text(String),
    /// Keywords
    Keywords(Vec<String>),
    /// Time range
    TimeRange { start: Timestamp, end: Timestamp },
    /// Memory reference
    Reference(u64),
    /// Embedding
    Embedding(Vec<f32>),
    /// Tags
    Tags(Vec<String>),
}

/// Retrieval result
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    /// Memory ID
    pub memory_id: u64,
    /// Relevance score
    pub relevance: f64,
    /// Match type
    pub match_type: MatchType,
    /// Confidence
    pub confidence: f64,
    /// Retrieval path
    pub retrieval_path: Vec<RetrievalStep>,
}

/// Match type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    Direct,
    Semantic,
    Associative,
    Temporal,
    Partial,
}

/// Retrieval step
#[derive(Debug, Clone)]
pub struct RetrievalStep {
    /// Step type
    pub step_type: String,
    /// Source
    pub source: u64,
    /// Target
    pub target: u64,
    /// Score contribution
    pub score: f64,
}

/// Memory entry (for retrieval engine)
#[derive(Debug, Clone)]
pub struct MemoryEntry {
    /// ID
    pub id: u64,
    /// Text content
    pub text: String,
    /// Keywords
    pub keywords: Vec<String>,
    /// Tags
    pub tags: Vec<String>,
    /// Created
    pub created: Timestamp,
    /// Importance
    pub importance: f64,
    /// Associations
    pub associations: Vec<u64>,
}

// ============================================================================
// RETRIEVAL ENGINE
// ============================================================================

/// Retrieval engine
pub struct RetrievalEngine {
    /// Memories (for demonstration)
    memories: BTreeMap<u64, MemoryEntry>,
    /// Keyword index
    keyword_index: BTreeMap<String, Vec<u64>>,
    /// Association graph
    associations: BTreeMap<u64, Vec<(u64, f64)>>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: RetrievalConfig,
    /// Statistics
    stats: RetrievalStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct RetrievalConfig {
    /// Maximum results
    pub max_results: usize,
    /// Minimum relevance
    pub min_relevance: f64,
    /// Association depth
    pub association_depth: usize,
    /// Enable spreading activation
    pub spreading_activation: bool,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_relevance: 0.1,
            association_depth: 2,
            spreading_activation: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct RetrievalStats {
    /// Queries
    pub queries: u64,
    /// Hits
    pub hits: u64,
    /// Misses
    pub misses: u64,
    /// Average relevance
    pub avg_relevance: f64,
}

impl RetrievalEngine {
    /// Create new engine
    pub fn new(config: RetrievalConfig) -> Self {
        Self {
            memories: BTreeMap::new(),
            keyword_index: BTreeMap::new(),
            associations: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: RetrievalStats::default(),
        }
    }

    /// Index memory
    pub fn index(&mut self, memory: MemoryEntry) {
        let id = memory.id;

        // Index keywords
        for keyword in &memory.keywords {
            self.keyword_index
                .entry(keyword.to_lowercase())
                .or_insert_with(Vec::new)
                .push(id);
        }

        // Index tags
        for tag in &memory.tags {
            self.keyword_index
                .entry(format!("tag:{}", tag))
                .or_insert_with(Vec::new)
                .push(id);
        }

        // Index associations
        for &assoc_id in &memory.associations {
            self.associations
                .entry(id)
                .or_insert_with(Vec::new)
                .push((assoc_id, 1.0));
        }

        self.memories.insert(id, memory);
    }

    /// Retrieve with cue
    pub fn retrieve(&mut self, cue: &RetrievalCue) -> Vec<RetrievalResult> {
        self.stats.queries += 1;

        let candidates = match &cue.content {
            CueContent::Text(text) => self.retrieve_by_text(text),
            CueContent::Keywords(keywords) => self.retrieve_by_keywords(keywords),
            CueContent::TimeRange { start, end } => self.retrieve_by_time(*start, *end),
            CueContent::Reference(id) => self.retrieve_by_association(*id),
            CueContent::Tags(tags) => self.retrieve_by_tags(tags),
            CueContent::Embedding(_) => Vec::new(), // Would need embedding similarity
        };

        // Apply spreading activation if enabled
        let expanded = if self.config.spreading_activation {
            self.spread_activation(&candidates)
        } else {
            candidates
        };

        // Filter and sort
        let mut results: Vec<RetrievalResult> = expanded
            .into_iter()
            .filter(|r| r.relevance >= self.config.min_relevance)
            .collect();

        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
        results.truncate(self.config.max_results);

        // Update stats
        if results.is_empty() {
            self.stats.misses += 1;
        } else {
            self.stats.hits += 1;
            let total_relevance: f64 = results.iter().map(|r| r.relevance).sum();
            let n = self.stats.queries as f64;
            self.stats.avg_relevance =
                (self.stats.avg_relevance * (n - 1.0) + total_relevance / results.len() as f64) / n;
        }

        results
    }

    fn retrieve_by_text(&self, text: &str) -> Vec<RetrievalResult> {
        let words: Vec<String> = text.split_whitespace().map(|s| s.to_lowercase()).collect();

        self.retrieve_by_keywords(&words)
    }

    fn retrieve_by_keywords(&self, keywords: &[String]) -> Vec<RetrievalResult> {
        let mut scores: BTreeMap<u64, f64> = BTreeMap::new();

        for keyword in keywords {
            if let Some(ids) = self.keyword_index.get(&keyword.to_lowercase()) {
                for &id in ids {
                    *scores.entry(id).or_insert(0.0) += 1.0;
                }
            }
        }

        let max_score = keywords.len() as f64;

        scores
            .into_iter()
            .map(|(id, score)| RetrievalResult {
                memory_id: id,
                relevance: score / max_score,
                match_type: MatchType::Semantic,
                confidence: score / max_score,
                retrieval_path: vec![RetrievalStep {
                    step_type: "keyword".into(),
                    source: 0,
                    target: id,
                    score: score / max_score,
                }],
            })
            .collect()
    }

    fn retrieve_by_time(&self, start: Timestamp, end: Timestamp) -> Vec<RetrievalResult> {
        self.memories
            .values()
            .filter(|m| m.created.0 >= start.0 && m.created.0 <= end.0)
            .map(|m| RetrievalResult {
                memory_id: m.id,
                relevance: m.importance,
                match_type: MatchType::Temporal,
                confidence: 1.0,
                retrieval_path: vec![],
            })
            .collect()
    }

    fn retrieve_by_association(&self, source_id: u64) -> Vec<RetrievalResult> {
        let mut results = Vec::new();
        let mut visited = alloc::collections::BTreeSet::new();
        let mut queue = vec![(source_id, 1.0, 0usize)];

        while let Some((id, score, depth)) = queue.pop() {
            if visited.contains(&id) || depth > self.config.association_depth {
                continue;
            }
            visited.insert(id);

            if id != source_id {
                results.push(RetrievalResult {
                    memory_id: id,
                    relevance: score,
                    match_type: MatchType::Associative,
                    confidence: score,
                    retrieval_path: vec![RetrievalStep {
                        step_type: "association".into(),
                        source: source_id,
                        target: id,
                        score,
                    }],
                });
            }

            if let Some(associations) = self.associations.get(&id) {
                for &(assoc_id, strength) in associations {
                    queue.push((assoc_id, score * strength * 0.7, depth + 1));
                }
            }
        }

        results
    }

    fn retrieve_by_tags(&self, tags: &[String]) -> Vec<RetrievalResult> {
        let tag_keywords: Vec<String> = tags.iter().map(|t| format!("tag:{}", t)).collect();

        self.retrieve_by_keywords(&tag_keywords)
    }

    fn spread_activation(&self, initial: &[RetrievalResult]) -> Vec<RetrievalResult> {
        let mut activation: BTreeMap<u64, f64> = BTreeMap::new();

        // Initialize with direct results
        for result in initial {
            activation.insert(result.memory_id, result.relevance);
        }

        // Spread to neighbors
        for result in initial {
            if let Some(associations) = self.associations.get(&result.memory_id) {
                for &(neighbor_id, strength) in associations {
                    let spread_amount = result.relevance * strength * 0.3;
                    *activation.entry(neighbor_id).or_insert(0.0) += spread_amount;
                }
            }
        }

        // Convert back to results
        activation
            .into_iter()
            .map(|(id, relevance)| {
                let match_type = if initial.iter().any(|r| r.memory_id == id) {
                    MatchType::Direct
                } else {
                    MatchType::Associative
                };

                RetrievalResult {
                    memory_id: id,
                    relevance: relevance.min(1.0),
                    match_type,
                    confidence: relevance.min(1.0),
                    retrieval_path: Vec::new(),
                }
            })
            .collect()
    }

    /// Create cue
    pub fn create_cue(&mut self, cue_type: CueType, content: CueContent) -> RetrievalCue {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        RetrievalCue {
            id,
            cue_type,
            content,
            weight: 1.0,
        }
    }

    /// Add association
    pub fn associate(&mut self, from: u64, to: u64, strength: f64) {
        self.associations
            .entry(from)
            .or_insert_with(Vec::new)
            .push((to, strength));
    }

    /// Get statistics
    pub fn stats(&self) -> &RetrievalStats {
        &self.stats
    }
}

impl Default for RetrievalEngine {
    fn default() -> Self {
        Self::new(RetrievalConfig::default())
    }
}

// ============================================================================
// QUERY BUILDER
// ============================================================================

/// Query builder
pub struct QueryBuilder {
    cues: Vec<RetrievalCue>,
    next_id: u64,
}

impl QueryBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            cues: Vec::new(),
            next_id: 1,
        }
    }

    /// Add text cue
    pub fn text(mut self, text: &str) -> Self {
        self.cues.push(RetrievalCue {
            id: self.next_id,
            cue_type: CueType::Semantic,
            content: CueContent::Text(text.into()),
            weight: 1.0,
        });
        self.next_id += 1;
        self
    }

    /// Add keyword cue
    pub fn keywords(mut self, keywords: Vec<String>) -> Self {
        self.cues.push(RetrievalCue {
            id: self.next_id,
            cue_type: CueType::Semantic,
            content: CueContent::Keywords(keywords),
            weight: 1.0,
        });
        self.next_id += 1;
        self
    }

    /// Add time range cue
    pub fn time_range(mut self, start: Timestamp, end: Timestamp) -> Self {
        self.cues.push(RetrievalCue {
            id: self.next_id,
            cue_type: CueType::Temporal,
            content: CueContent::TimeRange { start, end },
            weight: 1.0,
        });
        self.next_id += 1;
        self
    }

    /// Add association cue
    pub fn associated_with(mut self, memory_id: u64) -> Self {
        self.cues.push(RetrievalCue {
            id: self.next_id,
            cue_type: CueType::Associative,
            content: CueContent::Reference(memory_id),
            weight: 1.0,
        });
        self.next_id += 1;
        self
    }

    /// Build cues
    pub fn build(self) -> Vec<RetrievalCue> {
        self.cues
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_memory(id: u64, text: &str, keywords: Vec<&str>) -> MemoryEntry {
        MemoryEntry {
            id,
            text: text.into(),
            keywords: keywords.into_iter().map(String::from).collect(),
            tags: Vec::new(),
            created: Timestamp::now(),
            importance: 0.5,
            associations: Vec::new(),
        }
    }

    #[test]
    fn test_index_memory() {
        let mut engine = RetrievalEngine::default();

        let memory = create_test_memory(1, "Paris is in France", vec!["paris", "france"]);
        engine.index(memory);

        assert!(engine.memories.contains_key(&1));
    }

    #[test]
    fn test_keyword_retrieval() {
        let mut engine = RetrievalEngine::default();

        engine.index(create_test_memory(1, "Paris is in France", vec![
            "paris", "france",
        ]));
        engine.index(create_test_memory(2, "London is in England", vec![
            "london", "england",
        ]));

        let cue = engine.create_cue(
            CueType::Semantic,
            CueContent::Keywords(vec!["paris".into()]),
        );

        let results = engine.retrieve(&cue);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].memory_id, 1);
    }

    #[test]
    fn test_association_retrieval() {
        let mut engine = RetrievalEngine::default();

        let mut m1 = create_test_memory(1, "Dog", vec!["dog", "animal"]);
        m1.associations = vec![2];
        engine.index(m1);
        engine.index(create_test_memory(2, "Cat", vec!["cat", "animal"]));
        engine.associate(1, 2, 0.8);

        let cue = engine.create_cue(CueType::Associative, CueContent::Reference(1));

        let results = engine.retrieve(&cue);
        assert!(!results.is_empty());
    }

    #[test]
    fn test_query_builder() {
        let cues = QueryBuilder::new()
            .text("hello world")
            .keywords(vec!["test".into()])
            .build();

        assert_eq!(cues.len(), 2);
    }

    #[test]
    fn test_spreading_activation() {
        let mut engine = RetrievalEngine::default();

        engine.index(create_test_memory(1, "A", vec!["a"]));
        engine.index(create_test_memory(2, "B", vec!["b"]));
        engine.index(create_test_memory(3, "C", vec!["c"]));

        engine.associate(1, 2, 0.9);
        engine.associate(2, 3, 0.8);

        let cue = engine.create_cue(CueType::Semantic, CueContent::Keywords(vec!["a".into()]));

        let results = engine.retrieve(&cue);

        // Should find A directly and B through spreading
        assert!(results.len() >= 1);
    }
}
