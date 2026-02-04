//! NEXUS Year 2: Knowledge Representation
//!
//! Entity-relationship knowledge base for semantic reasoning.
//! Supports entities, relations, facts, and queries.

#![allow(dead_code)]

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use super::embeddings::{Embedding, EmbeddingId};

// ============================================================================
// Core Types
// ============================================================================

/// Unique identifier for entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u64);

impl EntityId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for relations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RelationId(pub u64);

impl RelationId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Unique identifier for facts
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FactId(pub u64);

impl FactId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Entity type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityType {
    /// Physical entity (CPU, memory, device)
    Physical,
    /// Abstract entity (process, thread, lock)
    Abstract,
    /// Event entity (interrupt, signal)
    Event,
    /// Measurement (metric, counter)
    Measurement,
    /// Named value
    Value,
}

/// An entity in the knowledge base
#[derive(Debug, Clone)]
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub entity_type: EntityType,
    pub attributes: BTreeMap<String, AttributeValue>,
    pub embedding: Option<EmbeddingId>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Attribute value types
#[derive(Debug, Clone)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    EntityRef(EntityId),
    List(Vec<AttributeValue>),
}

impl AttributeValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

impl Entity {
    pub fn new(id: EntityId, name: impl Into<String>, entity_type: EntityType) -> Self {
        Self {
            id,
            name: name.into(),
            entity_type,
            attributes: BTreeMap::new(),
            embedding: None,
            created_at: 0,
            updated_at: 0,
        }
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    pub fn with_embedding(mut self, embedding: EmbeddingId) -> Self {
        self.embedding = Some(embedding);
        self
    }

    pub fn set_attribute(&mut self, key: impl Into<String>, value: AttributeValue) {
        self.attributes.insert(key.into(), value);
    }

    pub fn get_attribute(&self, key: &str) -> Option<&AttributeValue> {
        self.attributes.get(key)
    }

    pub fn has_attribute(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }
}

// ============================================================================
// Relations
// ============================================================================

/// Relation cardinality
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    /// One-to-one
    OneToOne,
    /// One-to-many
    OneToMany,
    /// Many-to-one
    ManyToOne,
    /// Many-to-many
    ManyToMany,
}

/// A relation type definition
#[derive(Debug, Clone)]
pub struct Relation {
    pub id: RelationId,
    pub name: String,
    pub inverse_name: Option<String>,
    pub cardinality: Cardinality,
    pub symmetric: bool,
    pub transitive: bool,
    pub source_types: Vec<EntityType>,
    pub target_types: Vec<EntityType>,
}

impl Relation {
    pub fn new(id: RelationId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            inverse_name: None,
            cardinality: Cardinality::ManyToMany,
            symmetric: false,
            transitive: false,
            source_types: Vec::new(),
            target_types: Vec::new(),
        }
    }

    pub fn with_inverse(mut self, name: impl Into<String>) -> Self {
        self.inverse_name = Some(name.into());
        self
    }

    pub fn with_cardinality(mut self, cardinality: Cardinality) -> Self {
        self.cardinality = cardinality;
        self
    }

    pub fn symmetric(mut self) -> Self {
        self.symmetric = true;
        self
    }

    pub fn transitive(mut self) -> Self {
        self.transitive = true;
        self
    }
}

// ============================================================================
// Facts
// ============================================================================

/// A fact (triple) in the knowledge base
#[derive(Debug, Clone)]
pub struct Fact {
    pub id: FactId,
    pub subject: EntityId,
    pub relation: RelationId,
    pub object: EntityId,
    pub confidence: f32,
    pub source: Option<String>,
    pub valid_from: u64,
    pub valid_until: Option<u64>,
}

impl Fact {
    pub fn new(id: FactId, subject: EntityId, relation: RelationId, object: EntityId) -> Self {
        Self {
            id,
            subject,
            relation,
            object,
            confidence: 1.0,
            source: None,
            valid_from: 0,
            valid_until: None,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_validity(mut self, from: u64, until: Option<u64>) -> Self {
        self.valid_from = from;
        self.valid_until = until;
        self
    }

    pub fn is_valid_at(&self, time: u64) -> bool {
        if time < self.valid_from {
            return false;
        }
        if let Some(until) = self.valid_until {
            if time > until {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// Knowledge Base
// ============================================================================

/// Indexes for efficient knowledge base queries
struct KnowledgeIndexes {
    /// Facts by subject
    by_subject: BTreeMap<EntityId, Vec<FactId>>,
    /// Facts by object
    by_object: BTreeMap<EntityId, Vec<FactId>>,
    /// Facts by relation
    by_relation: BTreeMap<RelationId, Vec<FactId>>,
    /// Facts by (subject, relation)
    by_subject_relation: BTreeMap<(EntityId, RelationId), Vec<FactId>>,
    /// Facts by (relation, object)
    by_relation_object: BTreeMap<(RelationId, EntityId), Vec<FactId>>,
}

impl KnowledgeIndexes {
    fn new() -> Self {
        Self {
            by_subject: BTreeMap::new(),
            by_object: BTreeMap::new(),
            by_relation: BTreeMap::new(),
            by_subject_relation: BTreeMap::new(),
            by_relation_object: BTreeMap::new(),
        }
    }

    fn index_fact(&mut self, fact: &Fact) {
        self.by_subject
            .entry(fact.subject)
            .or_default()
            .push(fact.id);
        self.by_object.entry(fact.object).or_default().push(fact.id);
        self.by_relation
            .entry(fact.relation)
            .or_default()
            .push(fact.id);
        self.by_subject_relation
            .entry((fact.subject, fact.relation))
            .or_default()
            .push(fact.id);
        self.by_relation_object
            .entry((fact.relation, fact.object))
            .or_default()
            .push(fact.id);
    }

    fn remove_fact(&mut self, fact: &Fact) {
        if let Some(facts) = self.by_subject.get_mut(&fact.subject) {
            facts.retain(|&f| f != fact.id);
        }
        if let Some(facts) = self.by_object.get_mut(&fact.object) {
            facts.retain(|&f| f != fact.id);
        }
        if let Some(facts) = self.by_relation.get_mut(&fact.relation) {
            facts.retain(|&f| f != fact.id);
        }
        if let Some(facts) = self
            .by_subject_relation
            .get_mut(&(fact.subject, fact.relation))
        {
            facts.retain(|&f| f != fact.id);
        }
        if let Some(facts) = self
            .by_relation_object
            .get_mut(&(fact.relation, fact.object))
        {
            facts.retain(|&f| f != fact.id);
        }
    }
}

/// The knowledge base containing entities, relations, and facts
pub struct KnowledgeBase {
    name: String,
    entities: BTreeMap<EntityId, Entity>,
    relations: BTreeMap<RelationId, Relation>,
    facts: BTreeMap<FactId, Fact>,
    indexes: KnowledgeIndexes,
    entity_name_index: BTreeMap<String, EntityId>,
    relation_name_index: BTreeMap<String, RelationId>,
    next_entity_id: u64,
    next_relation_id: u64,
    next_fact_id: u64,
}

impl KnowledgeBase {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            entities: BTreeMap::new(),
            relations: BTreeMap::new(),
            facts: BTreeMap::new(),
            indexes: KnowledgeIndexes::new(),
            entity_name_index: BTreeMap::new(),
            relation_name_index: BTreeMap::new(),
            next_entity_id: 1,
            next_relation_id: 1,
            next_fact_id: 1,
        }
    }

    // ========== Entity Operations ==========

    pub fn add_entity(&mut self, mut entity: Entity) -> EntityId {
        let id = EntityId::new(self.next_entity_id);
        self.next_entity_id += 1;
        entity.id = id;

        self.entity_name_index.insert(entity.name.clone(), id);
        self.entities.insert(id, entity);
        id
    }

    pub fn get_entity(&self, id: EntityId) -> Option<&Entity> {
        self.entities.get(&id)
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<&mut Entity> {
        self.entities.get_mut(&id)
    }

    pub fn get_entity_by_name(&self, name: &str) -> Option<&Entity> {
        self.entity_name_index
            .get(name)
            .and_then(|id| self.entities.get(id))
    }

    pub fn remove_entity(&mut self, id: EntityId) -> Option<Entity> {
        // Remove all facts involving this entity
        let facts_to_remove: Vec<FactId> = self
            .facts
            .values()
            .filter(|f| f.subject == id || f.object == id)
            .map(|f| f.id)
            .collect();

        for fact_id in facts_to_remove {
            self.remove_fact(fact_id);
        }

        // Remove entity
        if let Some(entity) = self.entities.remove(&id) {
            self.entity_name_index.remove(&entity.name);
            Some(entity)
        } else {
            None
        }
    }

    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    // ========== Relation Operations ==========

    pub fn add_relation(&mut self, mut relation: Relation) -> RelationId {
        let id = RelationId::new(self.next_relation_id);
        self.next_relation_id += 1;
        relation.id = id;

        self.relation_name_index.insert(relation.name.clone(), id);
        if let Some(ref inverse) = relation.inverse_name {
            self.relation_name_index.insert(inverse.clone(), id);
        }

        self.relations.insert(id, relation);
        id
    }

    pub fn get_relation(&self, id: RelationId) -> Option<&Relation> {
        self.relations.get(&id)
    }

    pub fn get_relation_by_name(&self, name: &str) -> Option<&Relation> {
        self.relation_name_index
            .get(name)
            .and_then(|id| self.relations.get(id))
    }

    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }

    // ========== Fact Operations ==========

    pub fn add_fact(
        &mut self,
        subject: EntityId,
        relation: RelationId,
        object: EntityId,
    ) -> FactId {
        let id = FactId::new(self.next_fact_id);
        self.next_fact_id += 1;

        let fact = Fact::new(id, subject, relation, object);
        self.indexes.index_fact(&fact);
        self.facts.insert(id, fact);

        // Handle symmetric relations
        if let Some(rel) = self.relations.get(&relation) {
            if rel.symmetric && subject != object {
                let sym_id = FactId::new(self.next_fact_id);
                self.next_fact_id += 1;

                let sym_fact = Fact::new(sym_id, object, relation, subject);
                self.indexes.index_fact(&sym_fact);
                self.facts.insert(sym_id, sym_fact);
            }
        }

        id
    }

    pub fn add_fact_with_confidence(
        &mut self,
        subject: EntityId,
        relation: RelationId,
        object: EntityId,
        confidence: f32,
    ) -> FactId {
        let id = self.add_fact(subject, relation, object);
        if let Some(fact) = self.facts.get_mut(&id) {
            fact.confidence = confidence;
        }
        id
    }

    pub fn get_fact(&self, id: FactId) -> Option<&Fact> {
        self.facts.get(&id)
    }

    pub fn remove_fact(&mut self, id: FactId) -> Option<Fact> {
        if let Some(fact) = self.facts.remove(&id) {
            self.indexes.remove_fact(&fact);
            Some(fact)
        } else {
            None
        }
    }

    pub fn fact_count(&self) -> usize {
        self.facts.len()
    }

    // ========== Query Operations ==========

    /// Get all facts about an entity (as subject)
    pub fn facts_about(&self, entity: EntityId) -> Vec<&Fact> {
        self.indexes
            .by_subject
            .get(&entity)
            .map(|ids| ids.iter().filter_map(|id| self.facts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all facts referencing an entity (as object)
    pub fn facts_referencing(&self, entity: EntityId) -> Vec<&Fact> {
        self.indexes
            .by_object
            .get(&entity)
            .map(|ids| ids.iter().filter_map(|id| self.facts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get objects related to subject by relation
    pub fn get_objects(&self, subject: EntityId, relation: RelationId) -> Vec<EntityId> {
        self.indexes
            .by_subject_relation
            .get(&(subject, relation))
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.facts.get(id))
                    .map(|f| f.object)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get subjects related to object by relation
    pub fn get_subjects(&self, relation: RelationId, object: EntityId) -> Vec<EntityId> {
        self.indexes
            .by_relation_object
            .get(&(relation, object))
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.facts.get(id))
                    .map(|f| f.subject)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if a fact exists
    pub fn has_fact(&self, subject: EntityId, relation: RelationId, object: EntityId) -> bool {
        self.indexes
            .by_subject_relation
            .get(&(subject, relation))
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.facts.get(id))
                    .any(|f| f.object == object)
            })
            .unwrap_or(false)
    }

    /// Find path between two entities
    pub fn find_path(
        &self,
        start: EntityId,
        end: EntityId,
        max_depth: usize,
    ) -> Option<Vec<(EntityId, RelationId)>> {
        if start == end {
            return Some(vec![]);
        }

        let mut visited = BTreeSet::new();
        let mut queue: Vec<(EntityId, Vec<(EntityId, RelationId)>)> = vec![(start, vec![])];

        while let Some((current, path)) = queue.pop() {
            if path.len() >= max_depth {
                continue;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            // Get all outgoing relations
            for fact in self.facts_about(current) {
                if fact.object == end {
                    let mut result = path.clone();
                    result.push((current, fact.relation));
                    return Some(result);
                }

                if !visited.contains(&fact.object) {
                    let mut new_path = path.clone();
                    new_path.push((current, fact.relation));
                    queue.push((fact.object, new_path));
                }
            }
        }

        None
    }

    /// Transitive closure for a relation
    pub fn transitive_closure(&self, entity: EntityId, relation: RelationId) -> Vec<EntityId> {
        let rel = match self.relations.get(&relation) {
            Some(r) if r.transitive => r,
            _ => return self.get_objects(entity, relation),
        };

        let _ = rel; // Just checking transitivity

        let mut result = BTreeSet::new();
        let mut queue = vec![entity];

        while let Some(current) = queue.pop() {
            for object in self.get_objects(current, relation) {
                if result.insert(object) {
                    queue.push(object);
                }
            }
        }

        result.into_iter().collect()
    }
}

// ============================================================================
// Knowledge Query
// ============================================================================

/// Query pattern for knowledge base
#[derive(Debug, Clone)]
pub enum QueryPattern {
    /// Match specific entity
    Entity(EntityId),
    /// Match by entity name
    EntityName(String),
    /// Match by entity type
    EntityType(EntityType),
    /// Any entity (variable)
    Variable(String),
}

/// A query against the knowledge base
pub struct KnowledgeQuery {
    patterns: Vec<(QueryPattern, RelationId, QueryPattern)>,
    bindings: BTreeMap<String, EntityId>,
}

impl KnowledgeQuery {
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            bindings: BTreeMap::new(),
        }
    }

    pub fn add_pattern(
        mut self,
        subject: QueryPattern,
        relation: RelationId,
        object: QueryPattern,
    ) -> Self {
        self.patterns.push((subject, relation, object));
        self
    }

    pub fn with_binding(mut self, var: impl Into<String>, entity: EntityId) -> Self {
        self.bindings.insert(var.into(), entity);
        self
    }

    /// Execute query and return all matching bindings
    pub fn execute(&self, kb: &KnowledgeBase) -> Vec<BTreeMap<String, EntityId>> {
        let mut results = vec![self.bindings.clone()];

        for (subject, relation, object) in &self.patterns {
            let mut new_results = Vec::new();

            for bindings in &results {
                let matching = self.match_pattern(kb, subject, *relation, object, bindings);
                new_results.extend(matching);
            }

            results = new_results;
        }

        results
    }

    fn match_pattern(
        &self,
        kb: &KnowledgeBase,
        subject: &QueryPattern,
        relation: RelationId,
        object: &QueryPattern,
        bindings: &BTreeMap<String, EntityId>,
    ) -> Vec<BTreeMap<String, EntityId>> {
        let mut results = Vec::new();

        // Resolve subject
        let subjects = self.resolve_pattern(kb, subject, bindings);

        for subject_id in subjects {
            let objects = kb.get_objects(subject_id, relation);

            for object_id in objects {
                if self.matches_pattern(kb, object, object_id, bindings) {
                    let mut new_bindings = bindings.clone();

                    // Bind variables
                    if let QueryPattern::Variable(var) = subject {
                        new_bindings.insert(var.clone(), subject_id);
                    }
                    if let QueryPattern::Variable(var) = object {
                        new_bindings.insert(var.clone(), object_id);
                    }

                    results.push(new_bindings);
                }
            }
        }

        results
    }

    fn resolve_pattern(
        &self,
        kb: &KnowledgeBase,
        pattern: &QueryPattern,
        bindings: &BTreeMap<String, EntityId>,
    ) -> Vec<EntityId> {
        match pattern {
            QueryPattern::Entity(id) => vec![*id],
            QueryPattern::EntityName(name) => kb
                .get_entity_by_name(name)
                .map(|e| vec![e.id])
                .unwrap_or_default(),
            QueryPattern::EntityType(et) => kb
                .entities
                .values()
                .filter(|e| e.entity_type == *et)
                .map(|e| e.id)
                .collect(),
            QueryPattern::Variable(var) => {
                if let Some(&id) = bindings.get(var) {
                    vec![id]
                } else {
                    // All entities
                    kb.entities.keys().copied().collect()
                }
            },
        }
    }

    fn matches_pattern(
        &self,
        kb: &KnowledgeBase,
        pattern: &QueryPattern,
        entity_id: EntityId,
        bindings: &BTreeMap<String, EntityId>,
    ) -> bool {
        match pattern {
            QueryPattern::Entity(id) => *id == entity_id,
            QueryPattern::EntityName(name) => kb
                .get_entity(entity_id)
                .map(|e| e.name == *name)
                .unwrap_or(false),
            QueryPattern::EntityType(et) => kb
                .get_entity(entity_id)
                .map(|e| e.entity_type == *et)
                .unwrap_or(false),
            QueryPattern::Variable(var) => {
                if let Some(&bound_id) = bindings.get(var) {
                    bound_id == entity_id
                } else {
                    true // Unbound variable matches anything
                }
            },
        }
    }
}

impl Default for KnowledgeQuery {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Kernel Knowledge Base
// ============================================================================

/// Create a kernel knowledge base with basic entities and relations
pub fn create_kernel_knowledge_base() -> KnowledgeBase {
    let mut kb = KnowledgeBase::new("KernelKB");

    // Define relations
    let owns = kb.add_relation(Relation::new(RelationId::new(0), "owns").with_inverse("owned_by"));

    let runs_on =
        kb.add_relation(Relation::new(RelationId::new(0), "runs_on").with_inverse("runs"));

    let uses = kb.add_relation(Relation::new(RelationId::new(0), "uses"));

    let part_of = kb.add_relation(
        Relation::new(RelationId::new(0), "part_of")
            .with_inverse("contains")
            .transitive(),
    );

    // Add kernel entity
    let kernel = kb.add_entity(
        Entity::new(EntityId::new(0), "kernel", EntityType::Abstract)
            .with_attribute("version", AttributeValue::String("1.0".into())),
    );

    // Add CPU entities
    let cpu0 = kb.add_entity(
        Entity::new(EntityId::new(0), "cpu0", EntityType::Physical)
            .with_attribute("core_id", AttributeValue::Int(0)),
    );

    let cpu1 = kb.add_entity(
        Entity::new(EntityId::new(0), "cpu1", EntityType::Physical)
            .with_attribute("core_id", AttributeValue::Int(1)),
    );

    // Add memory entity
    let memory = kb.add_entity(
        Entity::new(EntityId::new(0), "memory", EntityType::Physical)
            .with_attribute("total_bytes", AttributeValue::Int(1024 * 1024 * 1024)),
    );

    // Add relationships
    kb.add_fact(kernel, owns, cpu0);
    kb.add_fact(kernel, owns, cpu1);
    kb.add_fact(kernel, owns, memory);
    kb.add_fact(cpu0, part_of, kernel);
    kb.add_fact(cpu1, part_of, kernel);
    kb.add_fact(memory, part_of, kernel);

    kb
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let entity = Entity::new(EntityId::new(1), "test", EntityType::Abstract)
            .with_attribute("key", AttributeValue::Int(42));

        assert_eq!(entity.name, "test");
        assert!(entity.has_attribute("key"));
    }

    #[test]
    fn test_knowledge_base() {
        let mut kb = KnowledgeBase::new("test");

        let e1 = kb.add_entity(Entity::new(EntityId::new(0), "e1", EntityType::Abstract));
        let e2 = kb.add_entity(Entity::new(EntityId::new(0), "e2", EntityType::Abstract));
        let r = kb.add_relation(Relation::new(RelationId::new(0), "relates_to"));

        kb.add_fact(e1, r, e2);

        assert!(kb.has_fact(e1, r, e2));
        assert!(!kb.has_fact(e2, r, e1));
    }

    #[test]
    fn test_symmetric_relation() {
        let mut kb = KnowledgeBase::new("test");

        let e1 = kb.add_entity(Entity::new(EntityId::new(0), "e1", EntityType::Abstract));
        let e2 = kb.add_entity(Entity::new(EntityId::new(0), "e2", EntityType::Abstract));
        let r = kb.add_relation(Relation::new(RelationId::new(0), "connected").symmetric());

        kb.add_fact(e1, r, e2);

        assert!(kb.has_fact(e1, r, e2));
        assert!(kb.has_fact(e2, r, e1));
    }

    #[test]
    fn test_kernel_kb() {
        let kb = create_kernel_knowledge_base();
        assert!(kb.get_entity_by_name("kernel").is_some());
        assert!(kb.get_entity_by_name("cpu0").is_some());
    }
}
