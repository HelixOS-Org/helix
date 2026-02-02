//! # Semantic Store
//!
//! Implements semantic memory storage and retrieval.
//! Supports conceptual organization and semantic search.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// SEMANTIC TYPES
// ============================================================================

/// Concept
#[derive(Debug, Clone)]
pub struct Concept {
    /// Concept ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Category
    pub category: String,
    /// Attributes
    pub attributes: BTreeMap<String, AttributeValue>,
    /// Relations
    pub relations: Vec<Relation>,
    /// Embedding
    pub embedding: Vec<f64>,
    /// Activation level
    pub activation: f64,
    /// Created
    pub created: Timestamp,
    /// Last accessed
    pub last_accessed: Timestamp,
}

/// Attribute value
#[derive(Debug, Clone)]
pub enum AttributeValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<AttributeValue>),
}

/// Relation
#[derive(Debug, Clone)]
pub struct Relation {
    /// Relation type
    pub relation_type: RelationType,
    /// Target concept ID
    pub target_id: u64,
    /// Strength
    pub strength: f64,
}

/// Relation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    IsA,
    HasA,
    PartOf,
    Causes,
    Enables,
    Prevents,
    SimilarTo,
    OppositeTo,
    Before,
    After,
    Contains,
    UsedFor,
    CreatedBy,
    LocatedIn,
    Custom,
}

/// Semantic query
#[derive(Debug, Clone)]
pub struct SemanticQuery {
    /// Query type
    pub query_type: QueryType,
    /// Filter
    pub filter: Option<QueryFilter>,
    /// Limit
    pub limit: Option<usize>,
}

/// Query type
#[derive(Debug, Clone)]
pub enum QueryType {
    ByName(String),
    ByCategory(String),
    BySimilarity { embedding: Vec<f64>, threshold: f64 },
    ByRelation { relation: RelationType, target_id: u64 },
    ByAttribute { key: String, value: AttributeValue },
    ByActivation { min: f64 },
}

/// Query filter
#[derive(Debug, Clone)]
pub struct QueryFilter {
    /// Categories to include
    pub categories: Option<Vec<String>>,
    /// Min activation
    pub min_activation: Option<f64>,
    /// Created after
    pub created_after: Option<Timestamp>,
}

/// Query result
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Matched concepts
    pub concepts: Vec<Concept>,
    /// Scores (for similarity queries)
    pub scores: Vec<f64>,
    /// Total matches
    pub total: usize,
}

/// Spreading activation result
#[derive(Debug, Clone)]
pub struct ActivationResult {
    /// Activated concepts
    pub activated: BTreeMap<u64, f64>,
    /// Iterations
    pub iterations: u32,
}

// ============================================================================
// SEMANTIC STORE
// ============================================================================

/// Semantic store
pub struct SemanticStore {
    /// Concepts
    concepts: BTreeMap<u64, Concept>,
    /// Name index
    name_index: BTreeMap<String, u64>,
    /// Category index
    category_index: BTreeMap<String, Vec<u64>>,
    /// Inverse relations
    inverse_relations: BTreeMap<u64, Vec<(u64, RelationType)>>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: SemanticConfig,
    /// Statistics
    stats: SemanticStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct SemanticConfig {
    /// Default activation decay
    pub activation_decay: f64,
    /// Spreading activation factor
    pub spreading_factor: f64,
    /// Max iterations for spreading
    pub max_spreading_iterations: u32,
    /// Similarity threshold
    pub similarity_threshold: f64,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            activation_decay: 0.9,
            spreading_factor: 0.3,
            max_spreading_iterations: 5,
            similarity_threshold: 0.7,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct SemanticStats {
    /// Concepts stored
    pub concepts_stored: u64,
    /// Relations added
    pub relations_added: u64,
    /// Queries executed
    pub queries_executed: u64,
}

impl SemanticStore {
    /// Create new store
    pub fn new(config: SemanticConfig) -> Self {
        Self {
            concepts: BTreeMap::new(),
            name_index: BTreeMap::new(),
            category_index: BTreeMap::new(),
            inverse_relations: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: SemanticStats::default(),
        }
    }

    /// Store concept
    pub fn store(&mut self, name: &str, category: &str, embedding: Vec<f64>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let concept = Concept {
            id,
            name: name.into(),
            category: category.into(),
            attributes: BTreeMap::new(),
            relations: Vec::new(),
            embedding,
            activation: 1.0,
            created: now,
            last_accessed: now,
        };

        // Update indexes
        self.name_index.insert(name.into(), id);
        self.category_index.entry(category.into())
            .or_insert_with(Vec::new)
            .push(id);

        self.concepts.insert(id, concept);
        self.stats.concepts_stored += 1;

        id
    }

    /// Set attribute
    pub fn set_attribute(&mut self, concept_id: u64, key: &str, value: AttributeValue) {
        if let Some(concept) = self.concepts.get_mut(&concept_id) {
            concept.attributes.insert(key.into(), value);
        }
    }

    /// Add relation
    pub fn add_relation(
        &mut self,
        source_id: u64,
        target_id: u64,
        relation_type: RelationType,
        strength: f64,
    ) {
        if let Some(concept) = self.concepts.get_mut(&source_id) {
            concept.relations.push(Relation {
                relation_type,
                target_id,
                strength,
            });

            // Add inverse relation
            self.inverse_relations.entry(target_id)
                .or_insert_with(Vec::new)
                .push((source_id, relation_type));

            self.stats.relations_added += 1;
        }
    }

    /// Query concepts
    pub fn query(&mut self, query: &SemanticQuery) -> QueryResult {
        self.stats.queries_executed += 1;

        let mut concepts = Vec::new();
        let mut scores = Vec::new();

        match &query.query_type {
            QueryType::ByName(name) => {
                if let Some(&id) = self.name_index.get(name) {
                    if let Some(concept) = self.concepts.get_mut(&id) {
                        concept.last_accessed = Timestamp::now();
                        concepts.push(concept.clone());
                        scores.push(1.0);
                    }
                }
            }
            QueryType::ByCategory(category) => {
                if let Some(ids) = self.category_index.get(category) {
                    for &id in ids {
                        if let Some(concept) = self.concepts.get_mut(&id) {
                            concept.last_accessed = Timestamp::now();
                            concepts.push(concept.clone());
                            scores.push(1.0);
                        }
                    }
                }
            }
            QueryType::BySimilarity { embedding, threshold } => {
                for concept in self.concepts.values_mut() {
                    let sim = self.cosine_similarity(embedding, &concept.embedding);
                    if sim >= *threshold {
                        concept.last_accessed = Timestamp::now();
                        concepts.push(concept.clone());
                        scores.push(sim);
                    }
                }

                // Sort by similarity
                let mut combined: Vec<_> = concepts.into_iter().zip(scores.into_iter()).collect();
                combined.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));

                concepts = combined.iter().map(|(c, _)| c.clone()).collect();
                scores = combined.iter().map(|(_, s)| *s).collect();
            }
            QueryType::ByRelation { relation, target_id } => {
                if let Some(sources) = self.inverse_relations.get(target_id) {
                    for &(source_id, rel_type) in sources {
                        if rel_type == *relation {
                            if let Some(concept) = self.concepts.get_mut(&source_id) {
                                concept.last_accessed = Timestamp::now();
                                concepts.push(concept.clone());
                                scores.push(1.0);
                            }
                        }
                    }
                }
            }
            QueryType::ByAttribute { key, value } => {
                for concept in self.concepts.values_mut() {
                    if let Some(attr) = concept.attributes.get(key) {
                        if self.attribute_matches(attr, value) {
                            concept.last_accessed = Timestamp::now();
                            concepts.push(concept.clone());
                            scores.push(1.0);
                        }
                    }
                }
            }
            QueryType::ByActivation { min } => {
                for concept in self.concepts.values() {
                    if concept.activation >= *min {
                        concepts.push(concept.clone());
                        scores.push(concept.activation);
                    }
                }
            }
        }

        // Apply filter
        if let Some(filter) = &query.filter {
            let (filtered_concepts, filtered_scores): (Vec<_>, Vec<_>) = concepts.into_iter()
                .zip(scores.into_iter())
                .filter(|(c, _)| {
                    if let Some(cats) = &filter.categories {
                        if !cats.contains(&c.category) {
                            return false;
                        }
                    }
                    if let Some(min_act) = filter.min_activation {
                        if c.activation < min_act {
                            return false;
                        }
                    }
                    if let Some(after) = filter.created_after {
                        if c.created.0 < after.0 {
                            return false;
                        }
                    }
                    true
                })
                .unzip();

            concepts = filtered_concepts;
            scores = filtered_scores;
        }

        let total = concepts.len();

        // Apply limit
        if let Some(limit) = query.limit {
            concepts.truncate(limit);
            scores.truncate(limit);
        }

        QueryResult {
            concepts,
            scores,
            total,
        }
    }

    fn cosine_similarity(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }

    fn attribute_matches(&self, a: &AttributeValue, b: &AttributeValue) -> bool {
        match (a, b) {
            (AttributeValue::Bool(a), AttributeValue::Bool(b)) => a == b,
            (AttributeValue::Int(a), AttributeValue::Int(b)) => a == b,
            (AttributeValue::Float(a), AttributeValue::Float(b)) => (a - b).abs() < 0.001,
            (AttributeValue::String(a), AttributeValue::String(b)) => a == b,
            _ => false,
        }
    }

    /// Spreading activation
    pub fn spread_activation(&mut self, source_id: u64, initial_activation: f64) -> ActivationResult {
        let mut activated: BTreeMap<u64, f64> = BTreeMap::new();
        activated.insert(source_id, initial_activation);

        let mut to_process = vec![(source_id, initial_activation)];
        let mut iterations = 0;

        while !to_process.is_empty() && iterations < self.config.max_spreading_iterations {
            iterations += 1;

            let current_batch: Vec<_> = to_process.drain(..).collect();

            for (concept_id, activation) in current_batch {
                // Get relations
                if let Some(concept) = self.concepts.get(&concept_id) {
                    let relations = concept.relations.clone();

                    for relation in relations {
                        let spread_amount = activation * relation.strength * self.config.spreading_factor;

                        if spread_amount > 0.01 {
                            let entry = activated.entry(relation.target_id).or_insert(0.0);
                            let new_activation = (*entry + spread_amount).min(1.0);

                            if new_activation > *entry {
                                *entry = new_activation;
                                to_process.push((relation.target_id, spread_amount));
                            }
                        }
                    }
                }
            }
        }

        // Update actual activations
        for (&id, &act) in &activated {
            if let Some(concept) = self.concepts.get_mut(&id) {
                concept.activation = act;
            }
        }

        ActivationResult {
            activated,
            iterations,
        }
    }

    /// Decay all activations
    pub fn decay_activations(&mut self) {
        for concept in self.concepts.values_mut() {
            concept.activation *= self.config.activation_decay;
        }
    }

    /// Get concept
    pub fn get(&self, id: u64) -> Option<&Concept> {
        self.concepts.get(&id)
    }

    /// Get by name
    pub fn get_by_name(&self, name: &str) -> Option<&Concept> {
        let id = self.name_index.get(name)?;
        self.concepts.get(id)
    }

    /// Get related concepts
    pub fn get_related(&self, id: u64, relation_type: Option<RelationType>) -> Vec<&Concept> {
        let concept = match self.concepts.get(&id) {
            Some(c) => c,
            None => return Vec::new(),
        };

        concept.relations.iter()
            .filter(|r| relation_type.is_none() || Some(r.relation_type) == relation_type)
            .filter_map(|r| self.concepts.get(&r.target_id))
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &SemanticStats {
        &self.stats
    }
}

impl Default for SemanticStore {
    fn default() -> Self {
        Self::new(SemanticConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_concept() {
        let mut store = SemanticStore::default();

        let id = store.store("dog", "animal", vec![0.1, 0.2, 0.3]);
        assert!(store.get(id).is_some());
    }

    #[test]
    fn test_query_by_name() {
        let mut store = SemanticStore::default();

        store.store("cat", "animal", vec![0.1, 0.2, 0.3]);

        let result = store.query(&SemanticQuery {
            query_type: QueryType::ByName("cat".into()),
            filter: None,
            limit: None,
        });

        assert_eq!(result.total, 1);
        assert_eq!(result.concepts[0].name, "cat");
    }

    #[test]
    fn test_query_by_category() {
        let mut store = SemanticStore::default();

        store.store("dog", "animal", vec![0.1, 0.2]);
        store.store("cat", "animal", vec![0.3, 0.4]);
        store.store("car", "vehicle", vec![0.5, 0.6]);

        let result = store.query(&SemanticQuery {
            query_type: QueryType::ByCategory("animal".into()),
            filter: None,
            limit: None,
        });

        assert_eq!(result.total, 2);
    }

    #[test]
    fn test_add_relation() {
        let mut store = SemanticStore::default();

        let dog = store.store("dog", "animal", vec![]);
        let mammal = store.store("mammal", "category", vec![]);

        store.add_relation(dog, mammal, RelationType::IsA, 1.0);

        let concept = store.get(dog).unwrap();
        assert_eq!(concept.relations.len(), 1);
    }

    #[test]
    fn test_similarity_query() {
        let mut store = SemanticStore::default();

        store.store("a", "test", vec![1.0, 0.0, 0.0]);
        store.store("b", "test", vec![0.9, 0.1, 0.0]);
        store.store("c", "test", vec![0.0, 1.0, 0.0]);

        let result = store.query(&SemanticQuery {
            query_type: QueryType::BySimilarity {
                embedding: vec![1.0, 0.0, 0.0],
                threshold: 0.8,
            },
            filter: None,
            limit: None,
        });

        assert_eq!(result.total, 2); // a and b are similar
    }

    #[test]
    fn test_spreading_activation() {
        let mut store = SemanticStore::default();

        let a = store.store("a", "test", vec![]);
        let b = store.store("b", "test", vec![]);
        let c = store.store("c", "test", vec![]);

        store.add_relation(a, b, RelationType::SimilarTo, 0.8);
        store.add_relation(b, c, RelationType::SimilarTo, 0.6);

        let result = store.spread_activation(a, 1.0);

        assert!(result.activated.contains_key(&a));
        assert!(result.activated.contains_key(&b));
    }
}
