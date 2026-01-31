//! # Episodic Memory System
//!
//! Long-term memory for specific events and experiences.
//! Stores, indexes, and retrieves contextual memories.
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
// EPISODE TYPES
// ============================================================================

/// An episode (discrete memory unit)
#[derive(Debug, Clone)]
pub struct Episode {
    /// Episode ID
    pub id: u64,
    /// Episode type
    pub episode_type: EpisodeType,
    /// What happened
    pub event: EventRecord,
    /// When it happened
    pub temporal: TemporalContext,
    /// Where it happened
    pub spatial: SpatialContext,
    /// Who/what was involved
    pub entities: Vec<EntityRef>,
    /// Emotional valence (-1 to 1)
    pub valence: f64,
    /// Importance (0 to 1)
    pub importance: f64,
    /// Vividness (0 to 1)
    pub vividness: f64,
    /// Associated tags
    pub tags: Vec<String>,
    /// Related episodes
    pub related: Vec<u64>,
    /// Retrieval count
    pub retrieval_count: u64,
    /// Last retrieved
    pub last_retrieved: Option<Timestamp>,
}

/// Episode type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpisodeType {
    /// User action/command
    UserAction,
    /// System event
    SystemEvent,
    /// Error/failure
    Error,
    /// Success/completion
    Success,
    /// Learning event
    Learning,
    /// Observation
    Observation,
    /// Decision
    Decision,
    /// Interaction
    Interaction,
}

/// Event record (what happened)
#[derive(Debug, Clone)]
pub struct EventRecord {
    /// Event description
    pub description: String,
    /// Event category
    pub category: String,
    /// Input/trigger
    pub input: Option<String>,
    /// Output/result
    pub output: Option<String>,
    /// Detailed data
    pub data: BTreeMap<String, EpisodeValue>,
}

/// Episode value
#[derive(Debug, Clone)]
pub enum EpisodeValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<EpisodeValue>),
    Map(BTreeMap<String, EpisodeValue>),
}

/// Temporal context
#[derive(Debug, Clone)]
pub struct TemporalContext {
    /// Start time
    pub start: Timestamp,
    /// End time
    pub end: Option<Timestamp>,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Sequence number in session
    pub sequence: u64,
    /// Session ID
    pub session: u64,
}

/// Spatial context (logical space)
#[derive(Debug, Clone)]
pub struct SpatialContext {
    /// Module/component
    pub module: String,
    /// File path
    pub file: Option<String>,
    /// Function/scope
    pub scope: Option<String>,
    /// Domain
    pub domain: Option<u64>,
}

/// Entity reference
#[derive(Debug, Clone)]
pub struct EntityRef {
    /// Entity type
    pub entity_type: EntityType,
    /// Entity ID
    pub id: String,
    /// Entity name
    pub name: String,
    /// Role in episode
    pub role: EntityRole,
}

/// Entity type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    User,
    System,
    Module,
    Process,
    File,
    Function,
    Variable,
    External,
}

/// Entity role
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityRole {
    Actor,
    Target,
    Observer,
    Catalyst,
    Result,
}

// ============================================================================
// MEMORY CONSOLIDATION
// ============================================================================

/// Memory strength
#[derive(Debug, Clone)]
pub struct MemoryStrength {
    /// Base strength
    pub base: f64,
    /// Decay rate
    pub decay_rate: f64,
    /// Last reinforcement
    pub last_reinforced: Timestamp,
    /// Rehearsal count
    pub rehearsal_count: u32,
}

impl MemoryStrength {
    /// Create new strength
    pub fn new(base: f64) -> Self {
        Self {
            base,
            decay_rate: 0.1,
            last_reinforced: Timestamp::now(),
            rehearsal_count: 0,
        }
    }

    /// Get current strength (with decay)
    pub fn current(&self) -> f64 {
        let elapsed = Timestamp::now().elapsed_since(self.last_reinforced);
        let hours = elapsed as f64 / 3_600_000_000_000.0;

        // Ebbinghaus forgetting curve approximation
        let retention = (-self.decay_rate * hours.ln().max(0.0)).exp();
        self.base * retention
    }

    /// Reinforce memory
    pub fn reinforce(&mut self) {
        self.rehearsal_count += 1;
        self.last_reinforced = Timestamp::now();
        // Reduce decay rate with rehearsal (spaced repetition effect)
        self.decay_rate *= 0.9;
        self.base = (self.base + 0.1).min(1.0);
    }
}

// ============================================================================
// RETRIEVAL
// ============================================================================

/// Retrieval query
#[derive(Debug, Clone, Default)]
pub struct RetrievalQuery {
    /// Text search
    pub text: Option<String>,
    /// Episode types
    pub types: Option<Vec<EpisodeType>>,
    /// Time range
    pub time_range: Option<(Timestamp, Timestamp)>,
    /// Tags
    pub tags: Option<Vec<String>>,
    /// Entities
    pub entities: Option<Vec<String>>,
    /// Minimum importance
    pub min_importance: Option<f64>,
    /// Limit
    pub limit: usize,
    /// Sort by
    pub sort: RetrievalSort,
}

/// Retrieval sort
#[derive(Debug, Clone, Copy, Default)]
pub enum RetrievalSort {
    #[default]
    Relevance,
    Recency,
    Importance,
    Strength,
}

/// Retrieval result
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    /// Episode
    pub episode: Episode,
    /// Match score
    pub score: f64,
    /// Match details
    pub matches: Vec<MatchDetail>,
}

/// Match detail
#[derive(Debug, Clone)]
pub struct MatchDetail {
    /// Field matched
    pub field: String,
    /// Match score
    pub score: f64,
}

// ============================================================================
// EPISODIC MEMORY
// ============================================================================

/// Episodic memory store
pub struct EpisodicMemory {
    /// Episodes
    episodes: BTreeMap<u64, Episode>,
    /// Memory strengths
    strengths: BTreeMap<u64, MemoryStrength>,
    /// Index by type
    by_type: BTreeMap<EpisodeType, Vec<u64>>,
    /// Index by tag
    by_tag: BTreeMap<String, Vec<u64>>,
    /// Index by session
    by_session: BTreeMap<u64, Vec<u64>>,
    /// Index by entity
    by_entity: BTreeMap<String, Vec<u64>>,
    /// Next ID
    next_id: AtomicU64,
    /// Current session
    current_session: u64,
    /// Configuration
    config: EpisodicConfig,
    /// Statistics
    stats: EpisodicStats,
}

/// Episodic memory configuration
#[derive(Debug, Clone)]
pub struct EpisodicConfig {
    /// Maximum episodes
    pub max_episodes: usize,
    /// Consolidation threshold
    pub consolidation_threshold: f64,
    /// Auto-cleanup weak memories
    pub auto_cleanup: bool,
    /// Cleanup interval (ns)
    pub cleanup_interval_ns: u64,
}

impl Default for EpisodicConfig {
    fn default() -> Self {
        Self {
            max_episodes: 100000,
            consolidation_threshold: 0.1,
            auto_cleanup: true,
            cleanup_interval_ns: 3600_000_000_000, // 1 hour
        }
    }
}

/// Episodic memory statistics
#[derive(Debug, Clone, Default)]
pub struct EpisodicStats {
    /// Total episodes
    pub total_episodes: u64,
    /// Retrievals
    pub retrievals: u64,
    /// Average strength
    pub avg_strength: f64,
    /// Consolidations
    pub consolidations: u64,
}

impl EpisodicMemory {
    /// Create new episodic memory
    pub fn new(config: EpisodicConfig) -> Self {
        Self {
            episodes: BTreeMap::new(),
            strengths: BTreeMap::new(),
            by_type: BTreeMap::new(),
            by_tag: BTreeMap::new(),
            by_session: BTreeMap::new(),
            by_entity: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            current_session: 1,
            config,
            stats: EpisodicStats::default(),
        }
    }

    /// Start new session
    pub fn new_session(&mut self) -> u64 {
        self.current_session += 1;
        self.current_session
    }

    /// Store episode
    pub fn store(&mut self, mut episode: Episode) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        episode.id = id;

        // Set session
        episode.temporal.session = self.current_session;

        // Initial strength based on importance
        let strength = MemoryStrength::new(episode.importance);
        self.strengths.insert(id, strength);

        // Index
        self.by_type
            .entry(episode.episode_type)
            .or_insert_with(Vec::new)
            .push(id);

        for tag in &episode.tags {
            self.by_tag
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(id);
        }

        self.by_session
            .entry(episode.temporal.session)
            .or_insert_with(Vec::new)
            .push(id);

        for entity in &episode.entities {
            self.by_entity
                .entry(entity.id.clone())
                .or_insert_with(Vec::new)
                .push(id);
        }

        self.episodes.insert(id, episode);
        self.stats.total_episodes += 1;

        // Check capacity
        if self.episodes.len() > self.config.max_episodes {
            self.cleanup_weak();
        }

        id
    }

    /// Retrieve episodes
    pub fn retrieve(&mut self, query: &RetrievalQuery) -> Vec<RetrievalResult> {
        self.stats.retrievals += 1;

        let mut results: Vec<RetrievalResult> = self
            .episodes
            .values()
            .filter(|e| self.matches_query(e, query))
            .map(|e| self.score_episode(e, query))
            .collect();

        // Sort
        match query.sort {
            RetrievalSort::Relevance => {
                results.sort_by(|a, b| {
                    b.score
                        .partial_cmp(&a.score)
                        .unwrap_or(core::cmp::Ordering::Equal)
                });
            },
            RetrievalSort::Recency => {
                results.sort_by(|a, b| {
                    b.episode
                        .temporal
                        .start
                        .raw()
                        .cmp(&a.episode.temporal.start.raw())
                });
            },
            RetrievalSort::Importance => {
                results.sort_by(|a, b| {
                    b.episode
                        .importance
                        .partial_cmp(&a.episode.importance)
                        .unwrap_or(core::cmp::Ordering::Equal)
                });
            },
            RetrievalSort::Strength => {
                results.sort_by(|a, b| {
                    let sa = self
                        .strengths
                        .get(&a.episode.id)
                        .map(|s| s.current())
                        .unwrap_or(0.0);
                    let sb = self
                        .strengths
                        .get(&b.episode.id)
                        .map(|s| s.current())
                        .unwrap_or(0.0);
                    sb.partial_cmp(&sa).unwrap_or(core::cmp::Ordering::Equal)
                });
            },
        }

        // Reinforce retrieved memories
        for result in &results {
            if let Some(strength) = self.strengths.get_mut(&result.episode.id) {
                strength.reinforce();
            }
            if let Some(episode) = self.episodes.get_mut(&result.episode.id) {
                episode.retrieval_count += 1;
                episode.last_retrieved = Some(Timestamp::now());
            }
        }

        // Limit
        if results.len() > query.limit && query.limit > 0 {
            results.truncate(query.limit);
        }

        results
    }

    fn matches_query(&self, episode: &Episode, query: &RetrievalQuery) -> bool {
        // Type filter
        if let Some(types) = &query.types {
            if !types.contains(&episode.episode_type) {
                return false;
            }
        }

        // Time range
        if let Some((start, end)) = query.time_range {
            if episode.temporal.start.raw() < start.raw()
                || episode.temporal.start.raw() > end.raw()
            {
                return false;
            }
        }

        // Tags
        if let Some(tags) = &query.tags {
            if !tags.iter().all(|t| episode.tags.contains(t)) {
                return false;
            }
        }

        // Entities
        if let Some(entities) = &query.entities {
            if !entities
                .iter()
                .all(|e| episode.entities.iter().any(|ep| ep.id == *e))
            {
                return false;
            }
        }

        // Importance
        if let Some(min) = query.min_importance {
            if episode.importance < min {
                return false;
            }
        }

        // Text search
        if let Some(text) = &query.text {
            let text_lower = text.to_lowercase();
            let matches = episode
                .event
                .description
                .to_lowercase()
                .contains(&text_lower)
                || episode.event.category.to_lowercase().contains(&text_lower)
                || episode
                    .tags
                    .iter()
                    .any(|t| t.to_lowercase().contains(&text_lower));
            if !matches {
                return false;
            }
        }

        true
    }

    fn score_episode(&self, episode: &Episode, query: &RetrievalQuery) -> RetrievalResult {
        let mut score = 0.0;
        let mut matches = Vec::new();

        // Text match
        if let Some(text) = &query.text {
            let text_lower = text.to_lowercase();

            if episode
                .event
                .description
                .to_lowercase()
                .contains(&text_lower)
            {
                score += 0.5;
                matches.push(MatchDetail {
                    field: "description".into(),
                    score: 0.5,
                });
            }
            if episode.event.category.to_lowercase() == text_lower {
                score += 0.3;
                matches.push(MatchDetail {
                    field: "category".into(),
                    score: 0.3,
                });
            }
        }

        // Importance
        score += episode.importance * 0.2;

        // Memory strength
        if let Some(strength) = self.strengths.get(&episode.id) {
            score += strength.current() * 0.2;
        }

        // Recency boost
        let age_hours =
            (Timestamp::now().raw() - episode.temporal.start.raw()) as f64 / 3_600_000_000_000.0;
        let recency = (-0.01 * age_hours).exp();
        score += recency * 0.1;

        RetrievalResult {
            episode: episode.clone(),
            score,
            matches,
        }
    }

    /// Get episode by ID
    pub fn get(&self, id: u64) -> Option<&Episode> {
        self.episodes.get(&id)
    }

    /// Link related episodes
    pub fn link(&mut self, episode1: u64, episode2: u64) {
        if let Some(e1) = self.episodes.get_mut(&episode1) {
            if !e1.related.contains(&episode2) {
                e1.related.push(episode2);
            }
        }
        if let Some(e2) = self.episodes.get_mut(&episode2) {
            if !e2.related.contains(&episode1) {
                e2.related.push(episode1);
            }
        }
    }

    /// Get related episodes
    pub fn get_related(&self, id: u64) -> Vec<&Episode> {
        self.episodes
            .get(&id)
            .map(|e| {
                e.related
                    .iter()
                    .filter_map(|r| self.episodes.get(r))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Cleanup weak memories
    pub fn cleanup_weak(&mut self) {
        let threshold = self.config.consolidation_threshold;

        let weak_ids: Vec<u64> = self
            .strengths
            .iter()
            .filter(|(_, s)| s.current() < threshold)
            .map(|(id, _)| *id)
            .collect();

        for id in weak_ids {
            self.remove_episode(id);
        }
    }

    fn remove_episode(&mut self, id: u64) {
        if let Some(episode) = self.episodes.remove(&id) {
            self.strengths.remove(&id);

            if let Some(ids) = self.by_type.get_mut(&episode.episode_type) {
                ids.retain(|&i| i != id);
            }

            for tag in &episode.tags {
                if let Some(ids) = self.by_tag.get_mut(tag) {
                    ids.retain(|&i| i != id);
                }
            }

            if let Some(ids) = self.by_session.get_mut(&episode.temporal.session) {
                ids.retain(|&i| i != id);
            }
        }
    }

    /// Get memory strength
    pub fn get_strength(&self, id: u64) -> Option<f64> {
        self.strengths.get(&id).map(|s| s.current())
    }

    /// Get statistics
    pub fn stats(&self) -> &EpisodicStats {
        &self.stats
    }

    /// Episode count
    pub fn count(&self) -> usize {
        self.episodes.len()
    }
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new(EpisodicConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_episode(name: &str, importance: f64) -> Episode {
        Episode {
            id: 0,
            episode_type: EpisodeType::UserAction,
            event: EventRecord {
                description: name.into(),
                category: "test".into(),
                input: None,
                output: None,
                data: BTreeMap::new(),
            },
            temporal: TemporalContext {
                start: Timestamp::now(),
                end: None,
                duration_ns: 1000,
                sequence: 1,
                session: 1,
            },
            spatial: SpatialContext {
                module: "test".into(),
                file: None,
                scope: None,
                domain: None,
            },
            entities: vec![],
            valence: 0.0,
            importance,
            vividness: 1.0,
            tags: vec!["test".into()],
            related: vec![],
            retrieval_count: 0,
            last_retrieved: None,
        }
    }

    #[test]
    fn test_store_retrieve() {
        let mut memory = EpisodicMemory::default();

        let ep1 = create_test_episode("first event", 0.8);
        let ep2 = create_test_episode("second event", 0.5);

        let id1 = memory.store(ep1);
        let id2 = memory.store(ep2);

        assert!(memory.get(id1).is_some());
        assert!(memory.get(id2).is_some());

        let query = RetrievalQuery {
            text: Some("first".into()),
            limit: 10,
            ..Default::default()
        };

        let results = memory.retrieve(&query);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_importance_filter() {
        let mut memory = EpisodicMemory::default();

        memory.store(create_test_episode("high importance", 0.9));
        memory.store(create_test_episode("low importance", 0.2));

        let query = RetrievalQuery {
            min_importance: Some(0.5),
            limit: 10,
            ..Default::default()
        };

        let results = memory.retrieve(&query);
        assert_eq!(results.len(), 1);
        assert!(results[0].episode.importance >= 0.5);
    }

    #[test]
    fn test_linking() {
        let mut memory = EpisodicMemory::default();

        let id1 = memory.store(create_test_episode("event 1", 0.5));
        let id2 = memory.store(create_test_episode("event 2", 0.5));

        memory.link(id1, id2);

        let related = memory.get_related(id1);
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].id, id2);
    }
}
