//! NEXUS Year 2: Concept Spaces
//!
//! Hierarchical concept representation, concept relationships,
//! and conceptual reasoning.

#![allow(dead_code)]

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec::Vec;

use super::embeddings::{Embedding, EmbeddingId, EmbeddingSpace};
use super::similarity::{CosineSimilarity, SimilarityMetric};

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for concepts
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConceptId(pub u64);

impl ConceptId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Type of concept
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptType {
    /// Abstract concept (e.g., "performance", "safety")
    Abstract,
    /// Concrete concept (e.g., "CPU", "memory")
    Concrete,
    /// Action concept (e.g., "allocate", "schedule")
    Action,
    /// State concept (e.g., "running", "blocked")
    State,
    /// Property concept (e.g., "fast", "large")
    Property,
    /// Relation concept (e.g., "contains", "depends-on")
    Relation,
}

/// A concept in the semantic space
#[derive(Debug, Clone)]
pub struct Concept {
    pub id: ConceptId,
    pub name: String,
    pub concept_type: ConceptType,
    pub description: Option<String>,
    pub embedding: Option<EmbeddingId>,
    pub synonyms: Vec<String>,
    pub examples: Vec<String>,
    pub created_at: u64,
}

impl Concept {
    pub fn new(id: ConceptId, name: impl Into<String>, concept_type: ConceptType) -> Self {
        Self {
            id,
            name: name.into(),
            concept_type,
            description: None,
            embedding: None,
            synonyms: Vec::new(),
            examples: Vec::new(),
            created_at: 0,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_embedding(mut self, embedding_id: EmbeddingId) -> Self {
        self.embedding = Some(embedding_id);
        self
    }

    pub fn add_synonym(&mut self, synonym: impl Into<String>) {
        self.synonyms.push(synonym.into());
    }

    pub fn add_example(&mut self, example: impl Into<String>) {
        self.examples.push(example.into());
    }

    pub fn has_embedding(&self) -> bool {
        self.embedding.is_some()
    }
}

// ============================================================================
// Concept Relations
// ============================================================================

/// Type of relationship between concepts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptRelationType {
    /// Is-A relation (inheritance)
    IsA,
    /// Has-A relation (composition)
    HasA,
    /// Part-Of relation (mereology)
    PartOf,
    /// Causes relation (causality)
    Causes,
    /// Prevents relation
    Prevents,
    /// Requires relation (dependency)
    Requires,
    /// Conflicts relation (incompatibility)
    ConflictsWith,
    /// Similar-To relation
    SimilarTo,
    /// Opposite-Of relation
    OppositeOf,
    /// Used-For relation (purpose)
    UsedFor,
    /// Located-In relation (spatial)
    LocatedIn,
    /// Temporal-Before relation
    Before,
    /// Custom relation
    Custom(u32),
}

/// A relationship between two concepts
#[derive(Debug, Clone)]
pub struct ConceptRelation {
    pub source: ConceptId,
    pub target: ConceptId,
    pub relation_type: ConceptRelationType,
    pub strength: f32, // 0.0 to 1.0
    pub bidirectional: bool,
    pub metadata: BTreeMap<String, String>,
}

impl ConceptRelation {
    pub fn new(source: ConceptId, target: ConceptId, relation_type: ConceptRelationType) -> Self {
        Self {
            source,
            target,
            relation_type,
            strength: 1.0,
            bidirectional: false,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_strength(mut self, strength: f32) -> Self {
        self.strength = strength.clamp(0.0, 1.0);
        self
    }

    pub fn bidirectional(mut self) -> Self {
        self.bidirectional = true;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

// ============================================================================
// Concept Space
// ============================================================================

/// A space of concepts with their relationships
pub struct ConceptSpace {
    name: String,
    concepts: BTreeMap<ConceptId, Concept>,
    name_index: BTreeMap<String, ConceptId>,
    relations: Vec<ConceptRelation>,
    embedding_space: Option<EmbeddingSpace>,
    next_id: u64,
}

impl ConceptSpace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            concepts: BTreeMap::new(),
            name_index: BTreeMap::new(),
            relations: Vec::new(),
            embedding_space: None,
            next_id: 1,
        }
    }

    pub fn with_embedding_space(mut self, space: EmbeddingSpace) -> Self {
        self.embedding_space = Some(space);
        self
    }

    fn next_id(&mut self) -> ConceptId {
        let id = ConceptId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Add a concept to the space
    pub fn add_concept(&mut self, mut concept: Concept) -> ConceptId {
        let id = self.next_id();
        concept.id = id;

        self.name_index.insert(concept.name.clone(), id);

        // Index synonyms
        for synonym in &concept.synonyms {
            self.name_index.insert(synonym.clone(), id);
        }

        self.concepts.insert(id, concept);
        id
    }

    /// Get concept by ID
    pub fn get(&self, id: ConceptId) -> Option<&Concept> {
        self.concepts.get(&id)
    }

    /// Get concept by name
    pub fn get_by_name(&self, name: &str) -> Option<&Concept> {
        self.name_index
            .get(name)
            .and_then(|id| self.concepts.get(id))
    }

    /// Add a relation between concepts
    pub fn add_relation(&mut self, relation: ConceptRelation) {
        self.relations.push(relation);
    }

    /// Get all relations for a concept
    pub fn get_relations(&self, concept_id: ConceptId) -> Vec<&ConceptRelation> {
        self.relations
            .iter()
            .filter(|r| r.source == concept_id || (r.bidirectional && r.target == concept_id))
            .collect()
    }

    /// Get relations of specific type
    pub fn get_relations_of_type(
        &self,
        concept_id: ConceptId,
        relation_type: ConceptRelationType,
    ) -> Vec<&ConceptRelation> {
        self.relations
            .iter()
            .filter(|r| {
                r.relation_type == relation_type
                    && (r.source == concept_id || (r.bidirectional && r.target == concept_id))
            })
            .collect()
    }

    /// Find related concepts (targets of relations)
    pub fn get_related(&self, concept_id: ConceptId) -> Vec<ConceptId> {
        let mut related = BTreeSet::new();

        for relation in &self.relations {
            if relation.source == concept_id {
                related.insert(relation.target);
            }
            if relation.bidirectional && relation.target == concept_id {
                related.insert(relation.source);
            }
        }

        related.into_iter().collect()
    }

    /// Find parents (Is-A targets)
    pub fn get_parents(&self, concept_id: ConceptId) -> Vec<ConceptId> {
        self.get_relations_of_type(concept_id, ConceptRelationType::IsA)
            .into_iter()
            .map(|r| r.target)
            .collect()
    }

    /// Find children (Is-A sources)
    pub fn get_children(&self, concept_id: ConceptId) -> Vec<ConceptId> {
        self.relations
            .iter()
            .filter(|r| r.relation_type == ConceptRelationType::IsA && r.target == concept_id)
            .map(|r| r.source)
            .collect()
    }

    /// Find similar concepts using embeddings
    pub fn find_similar(&self, concept_id: ConceptId, top_k: usize) -> Vec<(ConceptId, f32)> {
        let concept = match self.concepts.get(&concept_id) {
            Some(c) => c,
            None => return Vec::new(),
        };

        let embedding_id = match concept.embedding {
            Some(id) => id,
            None => return Vec::new(),
        };

        let embedding_space = match &self.embedding_space {
            Some(s) => s,
            None => return Vec::new(),
        };

        let query_embedding = match embedding_space.get(embedding_id) {
            Some(e) => e,
            None => return Vec::new(),
        };

        let metric = CosineSimilarity;
        let mut similarities = Vec::new();

        for (id, other_concept) in &self.concepts {
            if *id == concept_id {
                continue;
            }

            if let Some(other_emb_id) = other_concept.embedding {
                if let Some(other_emb) = embedding_space.get(other_emb_id) {
                    let sim = metric.similarity(query_embedding, other_emb);
                    similarities.push((*id, sim));
                }
            }
        }

        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        similarities.truncate(top_k);
        similarities
    }

    /// Check if concept A is ancestor of concept B
    pub fn is_ancestor(&self, ancestor: ConceptId, descendant: ConceptId) -> bool {
        if ancestor == descendant {
            return false;
        }

        let mut visited = BTreeSet::new();
        let mut queue = vec![descendant];

        while let Some(current) = queue.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            for parent_id in self.get_parents(current) {
                if parent_id == ancestor {
                    return true;
                }
                queue.push(parent_id);
            }
        }

        false
    }

    /// Find common ancestors of two concepts
    pub fn common_ancestors(&self, a: ConceptId, b: ConceptId) -> Vec<ConceptId> {
        let ancestors_a = self.all_ancestors(a);
        let ancestors_b = self.all_ancestors(b);

        ancestors_a.intersection(&ancestors_b).copied().collect()
    }

    /// Get all ancestors of a concept
    pub fn all_ancestors(&self, concept_id: ConceptId) -> BTreeSet<ConceptId> {
        let mut ancestors = BTreeSet::new();
        let mut queue = vec![concept_id];

        while let Some(current) = queue.pop() {
            for parent_id in self.get_parents(current) {
                if ancestors.insert(parent_id) {
                    queue.push(parent_id);
                }
            }
        }

        ancestors
    }

    /// Concept count
    pub fn len(&self) -> usize {
        self.concepts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.concepts.is_empty()
    }

    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }
}

// ============================================================================
// Concept Hierarchy
// ============================================================================

/// Tree structure for concept hierarchies
pub struct ConceptHierarchy {
    roots: Vec<ConceptId>,
    children: BTreeMap<ConceptId, Vec<ConceptId>>,
    parents: BTreeMap<ConceptId, ConceptId>,
    depths: BTreeMap<ConceptId, usize>,
}

impl ConceptHierarchy {
    /// Build hierarchy from concept space
    pub fn from_space(space: &ConceptSpace) -> Self {
        let mut hierarchy = Self {
            roots: Vec::new(),
            children: BTreeMap::new(),
            parents: BTreeMap::new(),
            depths: BTreeMap::new(),
        };

        // Build parent-child maps from Is-A relations
        for relation in &space.relations {
            if relation.relation_type == ConceptRelationType::IsA {
                hierarchy.parents.insert(relation.source, relation.target);

                hierarchy
                    .children
                    .entry(relation.target)
                    .or_insert_with(Vec::new)
                    .push(relation.source);
            }
        }

        // Find roots (concepts without parents)
        for &id in space.concepts.keys() {
            if !hierarchy.parents.contains_key(&id) {
                hierarchy.roots.push(id);
            }
        }

        // Compute depths
        for &root in &hierarchy.roots {
            hierarchy.compute_depths(root, 0);
        }

        hierarchy
    }

    fn compute_depths(&mut self, concept_id: ConceptId, depth: usize) {
        self.depths.insert(concept_id, depth);

        if let Some(children) = self.children.get(&concept_id).cloned() {
            for child in children {
                self.compute_depths(child, depth + 1);
            }
        }
    }

    pub fn roots(&self) -> &[ConceptId] {
        &self.roots
    }

    pub fn get_children(&self, id: ConceptId) -> &[ConceptId] {
        self.children.get(&id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn get_parent(&self, id: ConceptId) -> Option<ConceptId> {
        self.parents.get(&id).copied()
    }

    pub fn get_depth(&self, id: ConceptId) -> Option<usize> {
        self.depths.get(&id).copied()
    }

    pub fn is_leaf(&self, id: ConceptId) -> bool {
        !self.children.contains_key(&id)
    }

    pub fn is_root(&self, id: ConceptId) -> bool {
        !self.parents.contains_key(&id)
    }

    /// Get path from root to concept
    pub fn path_to_root(&self, id: ConceptId) -> Vec<ConceptId> {
        let mut path = vec![id];
        let mut current = id;

        while let Some(parent) = self.get_parent(current) {
            path.push(parent);
            current = parent;
        }

        path.reverse();
        path
    }

    /// Find lowest common ancestor
    pub fn lca(&self, a: ConceptId, b: ConceptId) -> Option<ConceptId> {
        let path_a = self.path_to_root(a);
        let path_b = self.path_to_root(b);

        let set_a: BTreeSet<ConceptId> = path_a.into_iter().collect();

        for id in path_b {
            if set_a.contains(&id) {
                return Some(id);
            }
        }

        None
    }

    /// Semantic distance based on hierarchy
    pub fn semantic_distance(&self, a: ConceptId, b: ConceptId) -> Option<usize> {
        let lca = self.lca(a, b)?;
        let depth_lca = self.get_depth(lca)?;
        let depth_a = self.get_depth(a)?;
        let depth_b = self.get_depth(b)?;

        Some((depth_a - depth_lca) + (depth_b - depth_lca))
    }

    /// Get all descendants
    pub fn descendants(&self, id: ConceptId) -> Vec<ConceptId> {
        let mut result = Vec::new();
        let mut queue = vec![id];

        while let Some(current) = queue.pop() {
            if let Some(children) = self.children.get(&current) {
                for &child in children {
                    result.push(child);
                    queue.push(child);
                }
            }
        }

        result
    }
}

// ============================================================================
// Kernel Concept Space
// ============================================================================

/// Create a kernel-specific concept space
pub fn create_kernel_concept_space() -> ConceptSpace {
    let mut space = ConceptSpace::new("KernelConcepts");

    // Root concepts
    let resource = space.add_concept(
        Concept::new(ConceptId::new(0), "Resource", ConceptType::Abstract)
            .with_description("Any kernel-managed resource"),
    );

    let process_type = space.add_concept(
        Concept::new(ConceptId::new(0), "ProcessType", ConceptType::Abstract)
            .with_description("Types of executable entities"),
    );

    let operation = space.add_concept(
        Concept::new(ConceptId::new(0), "Operation", ConceptType::Action)
            .with_description("Kernel operations"),
    );

    // Resource subtypes
    let memory = space.add_concept(
        Concept::new(ConceptId::new(0), "Memory", ConceptType::Concrete)
            .with_description("Memory resources"),
    );
    space.add_relation(ConceptRelation::new(
        memory,
        resource,
        ConceptRelationType::IsA,
    ));

    let cpu = space.add_concept(
        Concept::new(ConceptId::new(0), "CPU", ConceptType::Concrete)
            .with_description("CPU resources"),
    );
    space.add_relation(ConceptRelation::new(
        cpu,
        resource,
        ConceptRelationType::IsA,
    ));

    let io = space.add_concept(
        Concept::new(ConceptId::new(0), "IO", ConceptType::Concrete)
            .with_description("I/O resources"),
    );
    space.add_relation(ConceptRelation::new(io, resource, ConceptRelationType::IsA));

    // Process subtypes
    let process = space.add_concept(
        Concept::new(ConceptId::new(0), "Process", ConceptType::Concrete)
            .with_description("User process"),
    );
    space.add_relation(ConceptRelation::new(
        process,
        process_type,
        ConceptRelationType::IsA,
    ));

    let thread = space.add_concept(
        Concept::new(ConceptId::new(0), "Thread", ConceptType::Concrete)
            .with_description("Kernel or user thread"),
    );
    space.add_relation(ConceptRelation::new(
        thread,
        process_type,
        ConceptRelationType::IsA,
    ));
    space.add_relation(ConceptRelation::new(
        thread,
        process,
        ConceptRelationType::PartOf,
    ));

    // Operations
    let allocate = space.add_concept(
        Concept::new(ConceptId::new(0), "Allocate", ConceptType::Action)
            .with_description("Allocate a resource"),
    );
    space.add_relation(ConceptRelation::new(
        allocate,
        operation,
        ConceptRelationType::IsA,
    ));
    space.add_relation(ConceptRelation::new(
        allocate,
        resource,
        ConceptRelationType::UsedFor,
    ));

    let schedule = space.add_concept(
        Concept::new(ConceptId::new(0), "Schedule", ConceptType::Action)
            .with_description("Schedule for execution"),
    );
    space.add_relation(ConceptRelation::new(
        schedule,
        operation,
        ConceptRelationType::IsA,
    ));
    space.add_relation(ConceptRelation::new(
        schedule,
        process_type,
        ConceptRelationType::UsedFor,
    ));

    // States
    let running = space.add_concept(
        Concept::new(ConceptId::new(0), "Running", ConceptType::State)
            .with_description("Currently executing"),
    );

    let blocked = space.add_concept(
        Concept::new(ConceptId::new(0), "Blocked", ConceptType::State)
            .with_description("Waiting for resource"),
    );
    space.add_relation(
        ConceptRelation::new(blocked, running, ConceptRelationType::OppositeOf).bidirectional(),
    );

    // Relationships
    space.add_relation(ConceptRelation::new(
        process,
        cpu,
        ConceptRelationType::Requires,
    ));
    space.add_relation(ConceptRelation::new(
        process,
        memory,
        ConceptRelationType::Requires,
    ));

    space
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concept_creation() {
        let concept = Concept::new(ConceptId::new(1), "Test", ConceptType::Abstract);
        assert_eq!(concept.name, "Test");
        assert_eq!(concept.concept_type, ConceptType::Abstract);
    }

    #[test]
    fn test_concept_space() {
        let mut space = ConceptSpace::new("test");
        let id = space.add_concept(Concept::new(
            ConceptId::new(0),
            "Test",
            ConceptType::Abstract,
        ));

        assert!(space.get(id).is_some());
        assert!(space.get_by_name("Test").is_some());
    }

    #[test]
    fn test_concept_relations() {
        let mut space = ConceptSpace::new("test");
        let parent = space.add_concept(Concept::new(
            ConceptId::new(0),
            "Parent",
            ConceptType::Abstract,
        ));
        let child = space.add_concept(Concept::new(
            ConceptId::new(0),
            "Child",
            ConceptType::Abstract,
        ));

        space.add_relation(ConceptRelation::new(
            child,
            parent,
            ConceptRelationType::IsA,
        ));

        let relations = space.get_relations(child);
        assert_eq!(relations.len(), 1);
    }

    #[test]
    fn test_kernel_concept_space() {
        let space = create_kernel_concept_space();
        assert!(space.get_by_name("Memory").is_some());
        assert!(space.get_by_name("CPU").is_some());
    }
}
