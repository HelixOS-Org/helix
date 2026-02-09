//! # Knowledge Graph Engine for NEXUS
//!
//! Year 2 "COGNITION" - Revolutionary kernel-level knowledge graph system
//! that enables semantic understanding, relationship inference, and
//! graph-based reasoning for intelligent kernel decisions.
//!
//! ## Features
//!
//! - Entity-Relationship-Attribute model
//! - TransE/RotatE-style embeddings
//! - Path-based inference
//! - SPARQL-like query engine
//! - Temporal knowledge evolution
//! - Uncertainty-aware reasoning

#![allow(dead_code)]
#![allow(clippy::excessive_nesting)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;

use crate::math::F64Ext;

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum embedding dimension
const MAX_EMBEDDING_DIM: usize = 128;

/// Default embedding dimension
const DEFAULT_EMBEDDING_DIM: usize = 64;

/// Maximum entities in graph
const MAX_ENTITIES: usize = 100_000;

/// Maximum relations in graph
const MAX_RELATIONS: usize = 10_000;

/// Default learning rate for embeddings
const DEFAULT_LEARNING_RATE: f64 = 0.01;

/// Margin for ranking loss
const DEFAULT_MARGIN: f64 = 1.0;

// ============================================================================
// CORE TYPES
// ============================================================================

/// Unique identifier for an entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u64);

/// Unique identifier for a relation
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationId(pub u64);

/// Unique identifier for a triple
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TripleId(pub u64);

/// Entity types in the kernel knowledge graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    /// A process entity
    Process,
    /// A thread entity
    Thread,
    /// A memory region
    MemoryRegion,
    /// A file descriptor
    FileDescriptor,
    /// A device
    Device,
    /// A driver
    Driver,
    /// A module
    Module,
    /// A system call
    Syscall,
    /// An interrupt
    Interrupt,
    /// A lock/mutex
    Lock,
    /// A CPU core
    CpuCore,
    /// A NUMA node
    NumaNode,
    /// A socket
    Socket,
    /// A pipe
    Pipe,
    /// A signal
    Signal,
    /// A user
    User,
    /// A group
    Group,
    /// A capability
    Capability,
    /// A namespace
    Namespace,
    /// A cgroup
    Cgroup,
    /// Generic entity
    Generic,
}

/// Relation types in the kernel knowledge graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    /// Parent-child relationship
    ParentOf,
    /// Child-parent relationship
    ChildOf,
    /// Ownership relationship
    Owns,
    /// Usage relationship
    Uses,
    /// Dependency relationship
    DependsOn,
    /// Access relationship
    Accesses,
    /// Creation relationship
    Creates,
    /// Destruction relationship
    Destroys,
    /// Signal relationship
    Signals,
    /// Wait relationship
    WaitsFor,
    /// Lock hold relationship
    HoldsLock,
    /// Lock wait relationship
    WaitsLock,
    /// Memory mapping
    MapsMemory,
    /// Execution on CPU
    RunsOn,
    /// Affinity relationship
    HasAffinity,
    /// Priority relationship
    HasPriority,
    /// Scheduling relationship
    Schedules,
    /// Communication relationship
    CommunicatesWith,
    /// Containment relationship
    Contains,
    /// Similarity relationship
    SimilarTo,
    /// Causation relationship
    Causes,
    /// Prevention relationship
    Prevents,
    /// Generic relationship
    Generic,
}

/// An entity in the knowledge graph
#[derive(Debug, Clone)]
pub struct Entity {
    /// Unique identifier
    pub id: EntityId,
    /// Entity type
    pub entity_type: EntityType,
    /// Name or label
    pub name: String,
    /// Embedding vector
    pub embedding: Vec<f64>,
    /// Attributes (key-value pairs)
    pub attributes: BTreeMap<String, AttributeValue>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub updated_at: u64,
    /// Confidence score
    pub confidence: f64,
}

/// An attribute value
#[derive(Debug, Clone)]
pub enum AttributeValue {
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// Boolean value
    Boolean(bool),
    /// Vector of floats
    Vector(Vec<f64>),
    /// Reference to another entity
    EntityRef(EntityId),
    /// Null/undefined
    Null,
}

/// A relation in the knowledge graph
#[derive(Debug, Clone)]
pub struct Relation {
    /// Unique identifier
    pub id: RelationId,
    /// Relation type
    pub relation_type: RelationType,
    /// Name or label
    pub name: String,
    /// Embedding vector
    pub embedding: Vec<f64>,
    /// Is symmetric?
    pub symmetric: bool,
    /// Is transitive?
    pub transitive: bool,
    /// Inverse relation (if any)
    pub inverse: Option<RelationId>,
}

/// A triple (head, relation, tail)
#[derive(Debug, Clone)]
pub struct Triple {
    /// Triple ID
    pub id: TripleId,
    /// Head entity
    pub head: EntityId,
    /// Relation
    pub relation: RelationId,
    /// Tail entity
    pub tail: EntityId,
    /// Confidence score
    pub confidence: f64,
    /// Creation timestamp
    pub created_at: u64,
    /// Is inferred (vs. explicit)?
    pub inferred: bool,
    /// Provenance (source of this triple)
    pub provenance: Option<String>,
}

/// A path in the knowledge graph
#[derive(Debug, Clone)]
pub struct KnowledgePath {
    /// Sequence of (entity, relation) pairs
    pub steps: Vec<(EntityId, RelationId)>,
    /// Final entity
    pub destination: EntityId,
    /// Total path score
    pub score: f64,
    /// Path length
    pub length: usize,
}

// ============================================================================
// KNOWLEDGE GRAPH CORE
// ============================================================================

/// The main knowledge graph structure
pub struct KnowledgeGraph {
    /// All entities
    entities: BTreeMap<EntityId, Entity>,
    /// All relations
    relations: BTreeMap<RelationId, Relation>,
    /// All triples
    triples: BTreeMap<TripleId, Triple>,
    /// Index: head -> [(relation, tail)]
    head_index: BTreeMap<EntityId, Vec<(RelationId, EntityId)>>,
    /// Index: tail -> [(head, relation)]
    tail_index: BTreeMap<EntityId, Vec<(EntityId, RelationId)>>,
    /// Index: relation -> [(head, tail)]
    relation_index: BTreeMap<RelationId, Vec<(EntityId, EntityId)>>,
    /// Entity name to ID mapping
    entity_names: BTreeMap<String, EntityId>,
    /// Relation name to ID mapping
    relation_names: BTreeMap<String, RelationId>,
    /// Embedding dimension
    embedding_dim: usize,
    /// Next entity ID
    next_entity_id: u64,
    /// Next relation ID
    next_relation_id: u64,
    /// Next triple ID
    next_triple_id: u64,
    /// Learning rate
    learning_rate: f64,
    /// Margin for ranking loss
    margin: f64,
}

impl KnowledgeGraph {
    /// Create a new knowledge graph
    pub fn new(embedding_dim: usize) -> Self {
        let dim = if embedding_dim > MAX_EMBEDDING_DIM {
            MAX_EMBEDDING_DIM
        } else {
            embedding_dim
        };

        Self {
            entities: BTreeMap::new(),
            relations: BTreeMap::new(),
            triples: BTreeMap::new(),
            head_index: BTreeMap::new(),
            tail_index: BTreeMap::new(),
            relation_index: BTreeMap::new(),
            entity_names: BTreeMap::new(),
            relation_names: BTreeMap::new(),
            embedding_dim: dim,
            next_entity_id: 1,
            next_relation_id: 1,
            next_triple_id: 1,
            learning_rate: DEFAULT_LEARNING_RATE,
            margin: DEFAULT_MARGIN,
        }
    }

    /// Create with default dimension
    #[inline(always)]
    pub fn with_default_dim() -> Self {
        Self::new(DEFAULT_EMBEDDING_DIM)
    }

    /// Add a new entity
    pub fn add_entity(
        &mut self,
        entity_type: EntityType,
        name: String,
        attributes: BTreeMap<String, AttributeValue>,
    ) -> EntityId {
        let id = EntityId(self.next_entity_id);
        self.next_entity_id += 1;

        // Initialize random embedding
        let embedding = self.random_embedding();

        let entity = Entity {
            id,
            entity_type,
            name: name.clone(),
            embedding,
            attributes,
            created_at: self.current_timestamp(),
            updated_at: self.current_timestamp(),
            confidence: 1.0,
        };

        self.entity_names.insert(name, id);
        self.entities.insert(id, entity);
        self.head_index.insert(id, Vec::new());
        self.tail_index.insert(id, Vec::new());

        id
    }

    /// Add a new relation type
    pub fn add_relation(
        &mut self,
        relation_type: RelationType,
        name: String,
        symmetric: bool,
        transitive: bool,
    ) -> RelationId {
        let id = RelationId(self.next_relation_id);
        self.next_relation_id += 1;

        let embedding = self.random_embedding();

        let relation = Relation {
            id,
            relation_type,
            name: name.clone(),
            embedding,
            symmetric,
            transitive,
            inverse: None,
        };

        self.relation_names.insert(name, id);
        self.relations.insert(id, relation);
        self.relation_index.insert(id, Vec::new());

        id
    }

    /// Add a triple (head, relation, tail)
    pub fn add_triple(
        &mut self,
        head: EntityId,
        relation: RelationId,
        tail: EntityId,
        confidence: f64,
    ) -> Option<TripleId> {
        // Validate entities and relation exist
        if !self.entities.contains_key(&head)
            || !self.entities.contains_key(&tail)
            || !self.relations.contains_key(&relation)
        {
            return None;
        }

        let id = TripleId(self.next_triple_id);
        self.next_triple_id += 1;

        let triple = Triple {
            id,
            head,
            relation,
            tail,
            confidence,
            created_at: self.current_timestamp(),
            inferred: false,
            provenance: None,
        };

        self.triples.insert(id, triple);

        // Update indices
        if let Some(list) = self.head_index.get_mut(&head) {
            list.push((relation, tail));
        }
        if let Some(list) = self.tail_index.get_mut(&tail) {
            list.push((head, relation));
        }
        if let Some(list) = self.relation_index.get_mut(&relation) {
            list.push((head, tail));
        }

        // Handle symmetric relations
        if let Some(rel) = self.relations.get(&relation) {
            if rel.symmetric {
                // Add reverse triple
                self.add_symmetric_triple(tail, relation, head, confidence);
            }
        }

        Some(id)
    }

    /// Add symmetric triple (internal)
    fn add_symmetric_triple(
        &mut self,
        head: EntityId,
        relation: RelationId,
        tail: EntityId,
        confidence: f64,
    ) {
        let id = TripleId(self.next_triple_id);
        self.next_triple_id += 1;

        let triple = Triple {
            id,
            head,
            relation,
            tail,
            confidence,
            created_at: self.current_timestamp(),
            inferred: true,
            provenance: Some(String::from("symmetric")),
        };

        self.triples.insert(id, triple);

        if let Some(list) = self.head_index.get_mut(&head) {
            list.push((relation, tail));
        }
        if let Some(list) = self.tail_index.get_mut(&tail) {
            list.push((head, relation));
        }
    }

    /// Get an entity by ID
    #[inline(always)]
    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    /// Get an entity by name
    #[inline]
    pub fn get_entity_by_name(&self, name: &str) -> Option<&Entity> {
        self.entity_names
            .get(name)
            .and_then(|id| self.entities.get(id))
    }

    /// Get a relation by ID
    #[inline(always)]
    pub fn get_relation(&self, id: RelationId) -> Option<&Relation> {
        self.relations.get(&id)
    }

    /// Get all outgoing edges from an entity
    #[inline(always)]
    pub fn get_outgoing(&self, entity: EntityId) -> Vec<(RelationId, EntityId)> {
        self.head_index.get(&entity).cloned().unwrap_or_default()
    }

    /// Get all incoming edges to an entity
    #[inline(always)]
    pub fn get_incoming(&self, entity: EntityId) -> Vec<(EntityId, RelationId)> {
        self.tail_index.get(&entity).cloned().unwrap_or_default()
    }

    /// Get all entities of a specific type
    #[inline]
    pub fn get_entities_by_type(&self, entity_type: EntityType) -> Vec<EntityId> {
        self.entities
            .values()
            .filter(|e| e.entity_type == entity_type)
            .map(|e| e.id)
            .collect()
    }

    /// Check if a triple exists
    #[inline]
    pub fn has_triple(&self, head: EntityId, relation: RelationId, tail: EntityId) -> bool {
        if let Some(outgoing) = self.head_index.get(&head) {
            outgoing.contains(&(relation, tail))
        } else {
            false
        }
    }

    /// Random embedding initialization
    fn random_embedding(&self) -> Vec<f64> {
        // Simple pseudo-random initialization
        let seed = self.next_entity_id.wrapping_mul(0x5DEECE66D) ^ self.next_triple_id;
        let mut rng = seed;
        let mut embedding = Vec::with_capacity(self.embedding_dim);

        for _ in 0..self.embedding_dim {
            rng = rng.wrapping_mul(0x5DEECE66D).wrapping_add(0xB);
            let val = ((rng >> 17) as f64) / (u32::MAX as f64) * 2.0 - 1.0;
            embedding.push(val * 0.1); // Small random values
        }

        // Normalize
        let norm: f64 = embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-10 {
            for x in &mut embedding {
                *x /= norm;
            }
        }

        embedding
    }

    /// Current timestamp (simplified)
    fn current_timestamp(&self) -> u64 {
        // In real kernel, would use proper time
        self.next_triple_id
    }
}

// ============================================================================
// EMBEDDING LEARNING (TransE-style)
// ============================================================================

/// TransE embedding model for knowledge graph
pub struct TransEModel {
    /// Reference to knowledge graph
    graph: KnowledgeGraph,
    /// Entity embeddings (mutable copy)
    entity_embeddings: BTreeMap<EntityId, Vec<f64>>,
    /// Relation embeddings (mutable copy)
    relation_embeddings: BTreeMap<RelationId, Vec<f64>>,
    /// Embedding dimension
    dim: usize,
    /// Learning rate
    lr: f64,
    /// Margin
    margin: f64,
}

impl TransEModel {
    /// Create a new TransE model
    pub fn new(graph: KnowledgeGraph) -> Self {
        let dim = graph.embedding_dim;
        let mut entity_embeddings = BTreeMap::new();
        let mut relation_embeddings = BTreeMap::new();

        // Copy embeddings
        for (id, entity) in &graph.entities {
            entity_embeddings.insert(*id, entity.embedding.clone());
        }
        for (id, relation) in &graph.relations {
            relation_embeddings.insert(*id, relation.embedding.clone());
        }

        Self {
            lr: graph.learning_rate,
            margin: graph.margin,
            graph,
            entity_embeddings,
            relation_embeddings,
            dim,
        }
    }

    /// Score a triple (lower is better for TransE)
    pub fn score_triple(&self, head: EntityId, relation: RelationId, tail: EntityId) -> f64 {
        let h = match self.entity_embeddings.get(&head) {
            Some(e) => e,
            None => return f64::MAX,
        };
        let r = match self.relation_embeddings.get(&relation) {
            Some(e) => e,
            None => return f64::MAX,
        };
        let t = match self.entity_embeddings.get(&tail) {
            Some(e) => e,
            None => return f64::MAX,
        };

        // TransE score: ||h + r - t||
        let mut score = 0.0;
        for i in 0..self.dim {
            let diff = h[i] + r[i] - t[i];
            score += diff * diff;
        }
        score.sqrt()
    }

    /// Train on a batch of triples
    pub fn train_batch(&mut self, positive_triples: &[(EntityId, RelationId, EntityId)]) {
        for &(h, r, t) in positive_triples {
            // Generate negative sample (corrupt tail)
            let neg_t = self.corrupt_tail(h, r, t);

            // Compute scores
            let pos_score = self.score_triple(h, r, t);
            let neg_score = self.score_triple(h, r, neg_t);

            // Margin-based ranking loss
            let loss = pos_score - neg_score + self.margin;
            if loss > 0.0 {
                self.update_embeddings(h, r, t, neg_t, loss);
            }
        }
    }

    /// Corrupt the tail entity for negative sampling
    fn corrupt_tail(&self, _head: EntityId, _relation: RelationId, tail: EntityId) -> EntityId {
        // Simple corruption: pick a different entity
        for id in self.entity_embeddings.keys() {
            if *id != tail {
                return *id;
            }
        }
        tail
    }

    /// Update embeddings based on loss
    fn update_embeddings(
        &mut self,
        head: EntityId,
        relation: RelationId,
        tail: EntityId,
        neg_tail: EntityId,
        _loss: f64,
    ) {
        let lr = self.lr;

        // Get mutable references
        let h = self.entity_embeddings.get(&head).cloned();
        let r = self.relation_embeddings.get(&relation).cloned();
        let t = self.entity_embeddings.get(&tail).cloned();
        let nt = self.entity_embeddings.get(&neg_tail).cloned();

        if let (Some(h), Some(r), Some(t), Some(nt)) = (h, r, t, nt) {
            // Compute gradients
            let mut grad_h = vec![0.0; self.dim];
            let mut grad_r = vec![0.0; self.dim];
            let mut grad_t = vec![0.0; self.dim];
            let mut grad_nt = vec![0.0; self.dim];

            for i in 0..self.dim {
                // Positive triple gradient
                let pos_diff = h[i] + r[i] - t[i];
                grad_h[i] += pos_diff;
                grad_r[i] += pos_diff;
                grad_t[i] -= pos_diff;

                // Negative triple gradient
                let neg_diff = h[i] + r[i] - nt[i];
                grad_h[i] -= neg_diff;
                grad_r[i] -= neg_diff;
                grad_nt[i] += neg_diff;
            }

            // Update embeddings
            if let Some(h_emb) = self.entity_embeddings.get_mut(&head) {
                for i in 0..self.dim {
                    h_emb[i] -= lr * grad_h[i];
                }
                Self::normalize_vec(h_emb);
            }

            if let Some(r_emb) = self.relation_embeddings.get_mut(&relation) {
                for i in 0..self.dim {
                    r_emb[i] -= lr * grad_r[i];
                }
            }

            if let Some(t_emb) = self.entity_embeddings.get_mut(&tail) {
                for i in 0..self.dim {
                    t_emb[i] -= lr * grad_t[i];
                }
                Self::normalize_vec(t_emb);
            }

            if let Some(nt_emb) = self.entity_embeddings.get_mut(&neg_tail) {
                for i in 0..self.dim {
                    nt_emb[i] -= lr * grad_nt[i];
                }
                Self::normalize_vec(nt_emb);
            }
        }
    }

    /// Normalize a vector (static version)
    fn normalize_vec(v: &mut [f64]) {
        use crate::math::F64Ext;
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 1e-10 {
            for x in v {
                *x /= norm;
            }
        }
    }

    /// Predict tail entity given head and relation
    #[inline]
    pub fn predict_tail(
        &self,
        head: EntityId,
        relation: RelationId,
        top_k: usize,
    ) -> Vec<(EntityId, f64)> {
        let mut scores: Vec<(EntityId, f64)> = self
            .entity_embeddings
            .keys()
            .map(|&e| (e, self.score_triple(head, relation, e)))
            .collect();

        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        scores.truncate(top_k);
        scores
    }

    /// Predict head entity given relation and tail
    #[inline]
    pub fn predict_head(
        &self,
        relation: RelationId,
        tail: EntityId,
        top_k: usize,
    ) -> Vec<(EntityId, f64)> {
        let mut scores: Vec<(EntityId, f64)> = self
            .entity_embeddings
            .keys()
            .map(|&e| (e, self.score_triple(e, relation, tail)))
            .collect();

        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        scores.truncate(top_k);
        scores
    }
}

// ============================================================================
// PATH-BASED INFERENCE
// ============================================================================

/// Path-based reasoning over knowledge graph
pub struct PathReasoner<'a> {
    /// Reference to knowledge graph
    graph: &'a KnowledgeGraph,
    /// Maximum path length
    max_path_length: usize,
    /// Beam width for search
    beam_width: usize,
}

impl<'a> PathReasoner<'a> {
    /// Create a new path reasoner
    pub fn new(graph: &'a KnowledgeGraph) -> Self {
        Self {
            graph,
            max_path_length: 5,
            beam_width: 100,
        }
    }

    /// Find all paths between two entities
    pub fn find_paths(&self, source: EntityId, target: EntityId) -> Vec<KnowledgePath> {
        let mut paths = Vec::new();
        let mut queue: Vec<(EntityId, Vec<(EntityId, RelationId)>)> = vec![(source, Vec::new())];
        let mut visited = BTreeMap::new();

        while !queue.is_empty() && paths.len() < self.beam_width {
            let mut next_queue = Vec::new();

            for (current, path) in queue {
                if path.len() >= self.max_path_length {
                    continue;
                }

                // Check if we reached target
                if current == target && !path.is_empty() {
                    let score = self.score_path(&path);
                    paths.push(KnowledgePath {
                        steps: path.clone(),
                        destination: target,
                        score,
                        length: path.len(),
                    });
                    continue;
                }

                // Explore outgoing edges
                for (rel, next) in self.graph.get_outgoing(current) {
                    let visit_key = (next, path.len() + 1);
                    if visited.get(&visit_key).map_or(true, |&v| v > path.len()) {
                        visited.insert(visit_key, path.len());
                        let mut new_path = path.clone();
                        new_path.push((current, rel));
                        next_queue.push((next, new_path));
                    }
                }
            }

            queue = next_queue;
        }

        // Sort by score
        paths.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal));
        paths
    }

    /// Score a path based on relation confidence
    fn score_path(&self, path: &[(EntityId, RelationId)]) -> f64 {
        if path.is_empty() {
            return 0.0;
        }

        // Score based on relation types and transitivity
        let mut score = 1.0;
        for (_, rel) in path {
            if let Some(relation) = self.graph.get_relation(*rel) {
                if relation.transitive {
                    score *= 0.9; // Slight penalty for transitive hops
                } else {
                    score *= 0.7;
                }
            }
        }

        // Length penalty
        score / (path.len() as f64).sqrt()
    }

    /// Check if a path exists (reachability)
    #[inline(always)]
    pub fn is_reachable(&self, source: EntityId, target: EntityId) -> bool {
        !self.find_paths(source, target).is_empty()
    }

    /// Find common ancestors of two entities
    #[inline]
    pub fn find_common_ancestors(&self, e1: EntityId, e2: EntityId) -> Vec<EntityId> {
        let mut ancestors1 = self.find_ancestors(e1);
        let ancestors2 = self.find_ancestors(e2);

        ancestors1.retain(|a| ancestors2.contains(a));
        ancestors1
    }

    /// Find all ancestors of an entity
    fn find_ancestors(&self, entity: EntityId) -> Vec<EntityId> {
        let mut ancestors = Vec::new();
        let mut queue = vec![entity];
        let mut visited = vec![entity];

        while let Some(current) = queue.pop() {
            for (parent, _) in self.graph.get_incoming(current) {
                if !visited.contains(&parent) {
                    visited.push(parent);
                    ancestors.push(parent);
                    queue.push(parent);
                }
            }
        }

        ancestors
    }
}

// ============================================================================
// QUERY ENGINE (SPARQL-like)
// ============================================================================

/// A query pattern element
#[derive(Debug, Clone)]
pub enum QueryElement {
    /// Bound entity
    Entity(EntityId),
    /// Variable (unbound)
    Variable(String),
    /// Any (wildcard)
    Any,
}

/// A triple pattern for queries
#[derive(Debug, Clone)]
pub struct TriplePattern {
    /// Head pattern
    pub head: QueryElement,
    /// Relation pattern
    pub relation: QueryElement,
    /// Tail pattern
    pub tail: QueryElement,
}

/// Query result binding
#[derive(Debug, Clone)]
pub struct QueryBinding {
    /// Variable bindings
    pub bindings: BTreeMap<String, EntityId>,
    /// Confidence score
    pub confidence: f64,
}

/// Knowledge graph query engine
pub struct QueryEngine<'a> {
    /// Reference to knowledge graph
    graph: &'a KnowledgeGraph,
}

impl<'a> QueryEngine<'a> {
    /// Create a new query engine
    pub fn new(graph: &'a KnowledgeGraph) -> Self {
        Self { graph }
    }

    /// Execute a single pattern query
    pub fn query_pattern(&self, pattern: &TriplePattern) -> Vec<QueryBinding> {
        let mut results = Vec::new();

        // Iterate over all triples and match pattern
        for triple in self.graph.triples.values() {
            if let Some(binding) = self.match_pattern(pattern, triple) {
                results.push(binding);
            }
        }

        results
    }

    /// Match a pattern against a triple
    fn match_pattern(&self, pattern: &TriplePattern, triple: &Triple) -> Option<QueryBinding> {
        let mut bindings = BTreeMap::new();

        // Match head
        match &pattern.head {
            QueryElement::Entity(id) => {
                if *id != triple.head {
                    return None;
                }
            },
            QueryElement::Variable(var) => {
                bindings.insert(var.clone(), triple.head);
            },
            QueryElement::Any => {},
        }

        // Match relation (need to convert to EntityId for binding)
        match &pattern.relation {
            QueryElement::Entity(id) => {
                if id.0 != triple.relation.0 {
                    return None;
                }
            },
            QueryElement::Variable(_var) => {
                // Relations can't be bound as entities in this simple model
            },
            QueryElement::Any => {},
        }

        // Match tail
        match &pattern.tail {
            QueryElement::Entity(id) => {
                if *id != triple.tail {
                    return None;
                }
            },
            QueryElement::Variable(var) => {
                // Check for consistent binding
                if let Some(&existing) = bindings.get(var) {
                    if existing != triple.tail {
                        return None;
                    }
                }
                bindings.insert(var.clone(), triple.tail);
            },
            QueryElement::Any => {},
        }

        Some(QueryBinding {
            bindings,
            confidence: triple.confidence,
        })
    }

    /// Execute a conjunctive query (multiple patterns)
    pub fn query_conjunctive(&self, patterns: &[TriplePattern]) -> Vec<QueryBinding> {
        if patterns.is_empty() {
            return Vec::new();
        }

        // Start with first pattern
        let mut results = self.query_pattern(&patterns[0]);

        // Join with remaining patterns
        for pattern in patterns.iter().skip(1) {
            results = self.join_results(&results, pattern);
        }

        results
    }

    /// Join query results with a new pattern
    fn join_results(&self, results: &[QueryBinding], pattern: &TriplePattern) -> Vec<QueryBinding> {
        let mut new_results = Vec::new();

        for binding in results {
            // Substitute bound variables in pattern
            let substituted = self.substitute_pattern(pattern, &binding.bindings);

            // Query with substituted pattern
            let matches = self.query_pattern(&substituted);

            // Merge bindings
            for m in matches {
                let mut merged = binding.bindings.clone();
                for (k, v) in m.bindings {
                    if let Some(&existing) = merged.get(&k) {
                        if existing != v {
                            continue; // Inconsistent binding
                        }
                    }
                    merged.insert(k, v);
                }
                new_results.push(QueryBinding {
                    bindings: merged,
                    confidence: binding.confidence * m.confidence,
                });
            }
        }

        new_results
    }

    /// Substitute variables in a pattern
    fn substitute_pattern(
        &self,
        pattern: &TriplePattern,
        bindings: &BTreeMap<String, EntityId>,
    ) -> TriplePattern {
        let head = match &pattern.head {
            QueryElement::Variable(var) => {
                if let Some(&id) = bindings.get(var) {
                    QueryElement::Entity(id)
                } else {
                    pattern.head.clone()
                }
            },
            other => other.clone(),
        };

        let relation = pattern.relation.clone();

        let tail = match &pattern.tail {
            QueryElement::Variable(var) => {
                if let Some(&id) = bindings.get(var) {
                    QueryElement::Entity(id)
                } else {
                    pattern.tail.clone()
                }
            },
            other => other.clone(),
        };

        TriplePattern {
            head,
            relation,
            tail,
        }
    }
}

// ============================================================================
// KERNEL-SPECIFIC KNOWLEDGE GRAPH
// ============================================================================

/// Specialized knowledge graph for kernel entities
pub struct KernelKnowledgeGraph {
    /// Base knowledge graph
    pub graph: KnowledgeGraph,
    /// Predefined relation IDs
    pub relations: KernelRelations,
}

/// Predefined kernel relations
pub struct KernelRelations {
    pub parent_of: RelationId,
    pub child_of: RelationId,
    pub owns: RelationId,
    pub uses: RelationId,
    pub depends_on: RelationId,
    pub accesses: RelationId,
    pub holds_lock: RelationId,
    pub waits_lock: RelationId,
    pub runs_on: RelationId,
    pub communicates: RelationId,
    pub similar_to: RelationId,
    pub causes: RelationId,
}

impl KernelKnowledgeGraph {
    /// Create a new kernel knowledge graph
    pub fn new() -> Self {
        let mut graph = KnowledgeGraph::with_default_dim();

        // Add standard kernel relations
        let parent_of = graph.add_relation(
            RelationType::ParentOf,
            String::from("parent_of"),
            false,
            true, // Transitive
        );
        let child_of =
            graph.add_relation(RelationType::ChildOf, String::from("child_of"), false, true);
        let owns = graph.add_relation(RelationType::Owns, String::from("owns"), false, false);
        let uses = graph.add_relation(RelationType::Uses, String::from("uses"), false, false);
        let depends_on = graph.add_relation(
            RelationType::DependsOn,
            String::from("depends_on"),
            false,
            true,
        );
        let accesses = graph.add_relation(
            RelationType::Accesses,
            String::from("accesses"),
            false,
            false,
        );
        let holds_lock = graph.add_relation(
            RelationType::HoldsLock,
            String::from("holds_lock"),
            false,
            false,
        );
        let waits_lock = graph.add_relation(
            RelationType::WaitsLock,
            String::from("waits_lock"),
            false,
            false,
        );
        let runs_on =
            graph.add_relation(RelationType::RunsOn, String::from("runs_on"), false, false);
        let communicates = graph.add_relation(
            RelationType::CommunicatesWith,
            String::from("communicates_with"),
            true, // Symmetric
            false,
        );
        let similar_to = graph.add_relation(
            RelationType::SimilarTo,
            String::from("similar_to"),
            true, // Symmetric
            false,
        );
        let causes = graph.add_relation(RelationType::Causes, String::from("causes"), false, true);

        // Set inverse relations
        if let Some(rel) = graph.relations.get_mut(&parent_of) {
            rel.inverse = Some(child_of);
        }
        if let Some(rel) = graph.relations.get_mut(&child_of) {
            rel.inverse = Some(parent_of);
        }

        let relations = KernelRelations {
            parent_of,
            child_of,
            owns,
            uses,
            depends_on,
            accesses,
            holds_lock,
            waits_lock,
            runs_on,
            communicates,
            similar_to,
            causes,
        };

        Self { graph, relations }
    }

    /// Add a process entity
    #[inline]
    pub fn add_process(&mut self, pid: u64, name: String) -> EntityId {
        let mut attrs = BTreeMap::new();
        attrs.insert(String::from("pid"), AttributeValue::Integer(pid as i64));
        self.graph.add_entity(EntityType::Process, name, attrs)
    }

    /// Add a thread entity
    #[inline]
    pub fn add_thread(&mut self, tid: u64, name: String, parent: EntityId) -> EntityId {
        let mut attrs = BTreeMap::new();
        attrs.insert(String::from("tid"), AttributeValue::Integer(tid as i64));
        let thread = self.graph.add_entity(EntityType::Thread, name, attrs);
        self.graph
            .add_triple(parent, self.relations.parent_of, thread, 1.0);
        thread
    }

    /// Add a memory region entity
    pub fn add_memory_region(&mut self, address: u64, size: u64, owner: EntityId) -> EntityId {
        let mut attrs = BTreeMap::new();
        attrs.insert(
            String::from("address"),
            AttributeValue::Integer(address as i64),
        );
        attrs.insert(String::from("size"), AttributeValue::Integer(size as i64));
        let name = alloc::format!("mem_{:x}", address);
        let region = self.graph.add_entity(EntityType::MemoryRegion, name, attrs);
        self.graph
            .add_triple(owner, self.relations.owns, region, 1.0);
        region
    }

    /// Add a lock entity
    #[inline]
    pub fn add_lock(&mut self, lock_id: u64, name: String) -> EntityId {
        let mut attrs = BTreeMap::new();
        attrs.insert(
            String::from("lock_id"),
            AttributeValue::Integer(lock_id as i64),
        );
        self.graph.add_entity(EntityType::Lock, name, attrs)
    }

    /// Record lock acquisition
    #[inline(always)]
    pub fn record_lock_held(&mut self, holder: EntityId, lock: EntityId) {
        self.graph
            .add_triple(holder, self.relations.holds_lock, lock, 1.0);
    }

    /// Record lock wait
    #[inline(always)]
    pub fn record_lock_wait(&mut self, waiter: EntityId, lock: EntityId) {
        self.graph
            .add_triple(waiter, self.relations.waits_lock, lock, 1.0);
    }

    /// Detect potential deadlock
    pub fn detect_deadlock(&self) -> Vec<Vec<EntityId>> {
        let mut deadlocks = Vec::new();
        let reasoner = PathReasoner::new(&self.graph);

        // For each entity waiting on a lock
        for (waiter, edges) in &self.graph.head_index {
            for &(rel, lock) in edges {
                if rel == self.relations.waits_lock {
                    // Find who holds this lock
                    for (holder, h_edges) in &self.graph.head_index {
                        for &(h_rel, h_lock) in h_edges {
                            if h_rel == self.relations.holds_lock && h_lock == lock {
                                // Check if holder is waiting for a lock that waiter holds
                                if reasoner.is_reachable(*holder, *waiter) {
                                    deadlocks.push(vec![*waiter, *holder]);
                                }
                            }
                        }
                    }
                }
            }
        }

        deadlocks
    }

    /// Find processes with similar behavior
    pub fn find_similar_processes(&self, process: EntityId, top_k: usize) -> Vec<(EntityId, f64)> {
        let processes = self.graph.get_entities_by_type(EntityType::Process);
        let mut similarities: Vec<(EntityId, f64)> = Vec::new();

        if let Some(proc_entity) = self.graph.get_entity(process) {
            for other_id in processes {
                if other_id == process {
                    continue;
                }
                if let Some(other_entity) = self.graph.get_entity(other_id) {
                    let sim =
                        self.cosine_similarity(&proc_entity.embedding, &other_entity.embedding);
                    similarities.push((other_id, sim));
                }
            }
        }

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        similarities.truncate(top_k);
        similarities
    }

    /// Cosine similarity between vectors
    fn cosine_similarity(&self, a: &[f64], b: &[f64]) -> f64 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();

        if norm_a > 1e-10 && norm_b > 1e-10 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }
}

impl Default for KernelKnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TEMPORAL KNOWLEDGE
// ============================================================================

/// A temporal triple with validity interval
#[derive(Debug, Clone)]
pub struct TemporalTriple {
    /// Base triple
    pub triple: Triple,
    /// Valid from timestamp
    pub valid_from: u64,
    /// Valid until timestamp (None = still valid)
    pub valid_until: Option<u64>,
}

/// Temporal knowledge graph extension
pub struct TemporalKnowledgeGraph {
    /// Base knowledge graph
    pub base: KnowledgeGraph,
    /// Temporal triples
    temporal_triples: Vec<TemporalTriple>,
    /// Current time
    current_time: u64,
}

impl TemporalKnowledgeGraph {
    /// Create a new temporal knowledge graph
    pub fn new(embedding_dim: usize) -> Self {
        Self {
            base: KnowledgeGraph::new(embedding_dim),
            temporal_triples: Vec::new(),
            current_time: 0,
        }
    }

    /// Set current time
    #[inline(always)]
    pub fn set_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Add a temporal triple
    pub fn add_temporal_triple(
        &mut self,
        head: EntityId,
        relation: RelationId,
        tail: EntityId,
        confidence: f64,
        valid_from: u64,
    ) -> Option<TripleId> {
        let triple_id = self.base.add_triple(head, relation, tail, confidence)?;
        let triple = self.base.triples.get(&triple_id)?.clone();

        self.temporal_triples.push(TemporalTriple {
            triple,
            valid_from,
            valid_until: None,
        });

        Some(triple_id)
    }

    /// Invalidate a triple at given time
    #[inline]
    pub fn invalidate_triple(&mut self, triple_id: TripleId, end_time: u64) {
        for tt in &mut self.temporal_triples {
            if tt.triple.id == triple_id {
                tt.valid_until = Some(end_time);
                break;
            }
        }
    }

    /// Query triples valid at a specific time
    #[inline]
    pub fn query_at_time(&self, time: u64) -> Vec<&TemporalTriple> {
        self.temporal_triples
            .iter()
            .filter(|tt| tt.valid_from <= time && tt.valid_until.map_or(true, |end| end > time))
            .collect()
    }

    /// Get history of a relationship
    #[inline]
    pub fn get_relationship_history(
        &self,
        head: EntityId,
        relation: RelationId,
        tail: EntityId,
    ) -> Vec<&TemporalTriple> {
        self.temporal_triples
            .iter()
            .filter(|tt| {
                tt.triple.head == head && tt.triple.relation == relation && tt.triple.tail == tail
            })
            .collect()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_graph_basic() {
        let mut kg = KnowledgeGraph::new(32);

        // Add entities
        let proc1 = kg.add_entity(EntityType::Process, String::from("init"), BTreeMap::new());
        let proc2 = kg.add_entity(EntityType::Process, String::from("bash"), BTreeMap::new());

        // Add relation
        let parent_of = kg.add_relation(
            RelationType::ParentOf,
            String::from("parent_of"),
            false,
            true,
        );

        // Add triple
        let triple = kg.add_triple(proc1, parent_of, proc2, 1.0);
        assert!(triple.is_some());

        // Check triple exists
        assert!(kg.has_triple(proc1, parent_of, proc2));

        // Check outgoing edges
        let outgoing = kg.get_outgoing(proc1);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0], (parent_of, proc2));
    }

    #[test]
    fn test_kernel_knowledge_graph() {
        let mut kkg = KernelKnowledgeGraph::new();

        // Add processes
        let init = kkg.add_process(1, String::from("init"));
        let bash = kkg.add_process(100, String::from("bash"));

        // Add relationship
        kkg.graph
            .add_triple(init, kkg.relations.parent_of, bash, 1.0);

        // Verify
        assert!(kkg.graph.has_triple(init, kkg.relations.parent_of, bash));
    }

    #[test]
    fn test_path_reasoning() {
        let mut kg = KnowledgeGraph::new(32);

        let a = kg.add_entity(EntityType::Process, String::from("a"), BTreeMap::new());
        let b = kg.add_entity(EntityType::Process, String::from("b"), BTreeMap::new());
        let c = kg.add_entity(EntityType::Process, String::from("c"), BTreeMap::new());

        let rel = kg.add_relation(RelationType::ParentOf, String::from("parent"), false, true);

        kg.add_triple(a, rel, b, 1.0);
        kg.add_triple(b, rel, c, 1.0);

        let reasoner = PathReasoner::new(&kg);
        let paths = reasoner.find_paths(a, c);

        assert!(!paths.is_empty());
        assert_eq!(paths[0].length, 2);
    }

    #[test]
    fn test_transe_scoring() {
        let mut kg = KnowledgeGraph::new(16);

        let h = kg.add_entity(EntityType::Process, String::from("head"), BTreeMap::new());
        let t = kg.add_entity(EntityType::Process, String::from("tail"), BTreeMap::new());
        let r = kg.add_relation(RelationType::Uses, String::from("uses"), false, false);

        kg.add_triple(h, r, t, 1.0);

        let model = TransEModel::new(kg);
        let score = model.score_triple(h, r, t);
        assert!(score >= 0.0); // Score should be non-negative
    }

    #[test]
    fn test_query_engine() {
        let mut kg = KnowledgeGraph::new(32);

        let proc1 = kg.add_entity(EntityType::Process, String::from("proc1"), BTreeMap::new());
        let proc2 = kg.add_entity(EntityType::Process, String::from("proc2"), BTreeMap::new());
        let rel = kg.add_relation(RelationType::Uses, String::from("uses"), false, false);

        kg.add_triple(proc1, rel, proc2, 1.0);

        let engine = QueryEngine::new(&kg);

        // Query with variable
        let pattern = TriplePattern {
            head: QueryElement::Entity(proc1),
            relation: QueryElement::Entity(EntityId(rel.0)),
            tail: QueryElement::Variable(String::from("x")),
        };

        let results = engine.query_pattern(&pattern);
        assert!(!results.is_empty());
        assert_eq!(results[0].bindings.get("x"), Some(&proc2));
    }
}
