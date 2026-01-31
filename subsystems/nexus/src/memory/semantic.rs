//! # Semantic Memory System
//!
//! Long-term memory for concepts, facts, and relationships.
//! Implements knowledge representation and retrieval.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// CONCEPT TYPES
// ============================================================================

/// A concept (semantic memory unit)
#[derive(Debug, Clone)]
pub struct Concept {
    /// Concept ID
    pub id: u64,
    /// Concept name
    pub name: String,
    /// Concept type
    pub concept_type: ConceptType,
    /// Definition
    pub definition: Option<String>,
    /// Properties
    pub properties: BTreeMap<String, PropertyValue>,
    /// Examples
    pub examples: Vec<String>,
    /// Activation level
    pub activation: f64,
    /// Learning strength
    pub strength: f64,
    /// Created timestamp
    pub created: Timestamp,
    /// Last accessed
    pub last_accessed: Timestamp,
    /// Access count
    pub access_count: u64,
}

/// Concept type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptType {
    /// Entity (concrete thing)
    Entity,
    /// Action/verb
    Action,
    /// Property/attribute
    Property,
    /// Relation
    Relation,
    /// Event
    Event,
    /// Abstract concept
    Abstract,
    /// Category/class
    Category,
    /// Instance
    Instance,
}

/// Property value
#[derive(Debug, Clone)]
pub enum PropertyValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    List(Vec<PropertyValue>),
    Concept(u64),
}

// ============================================================================
// SEMANTIC RELATIONS
// ============================================================================

/// Semantic relation between concepts
#[derive(Debug, Clone)]
pub struct SemanticRelation {
    /// Relation ID
    pub id: u64,
    /// Source concept
    pub source: u64,
    /// Target concept
    pub target: u64,
    /// Relation type
    pub relation_type: RelationType,
    /// Strength (0 to 1)
    pub strength: f64,
    /// Bidirectional
    pub bidirectional: bool,
    /// Context
    pub context: Option<String>,
}

/// Relation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    /// Is a kind of (inheritance)
    IsA,
    /// Has part
    HasPart,
    /// Part of
    PartOf,
    /// Has property
    HasProperty,
    /// Causes
    Causes,
    /// Used for
    UsedFor,
    /// Located in
    LocatedIn,
    /// Similar to
    SimilarTo,
    /// Opposite of
    OppositeOf,
    /// Instance of
    InstanceOf,
    /// Defined by
    DefinedBy,
    /// Related to (generic)
    RelatedTo,
    /// Precedes
    Precedes,
    /// Requires
    Requires,
}

// ============================================================================
// KNOWLEDGE GRAPH
// ============================================================================

/// Semantic knowledge graph
pub struct SemanticGraph {
    /// Concepts
    concepts: BTreeMap<u64, Concept>,
    /// Concepts by name
    by_name: BTreeMap<String, u64>,
    /// Concepts by type
    by_type: BTreeMap<ConceptType, Vec<u64>>,
    /// Relations
    relations: BTreeMap<u64, SemanticRelation>,
    /// Outgoing relations
    outgoing: BTreeMap<u64, Vec<u64>>,
    /// Incoming relations
    incoming: BTreeMap<u64, Vec<u64>>,
    /// Next ID
    next_id: AtomicU64,
}

impl SemanticGraph {
    /// Create new graph
    pub fn new() -> Self {
        Self {
            concepts: BTreeMap::new(),
            by_name: BTreeMap::new(),
            by_type: BTreeMap::new(),
            relations: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Add concept
    pub fn add_concept(&mut self, name: &str, concept_type: ConceptType) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let concept = Concept {
            id,
            name: name.into(),
            concept_type,
            definition: None,
            properties: BTreeMap::new(),
            examples: Vec::new(),
            activation: 0.0,
            strength: 1.0,
            created: Timestamp::now(),
            last_accessed: Timestamp::now(),
            access_count: 0,
        };

        self.by_name.insert(name.into(), id);
        self.by_type.entry(concept_type).or_insert_with(Vec::new).push(id);
        self.concepts.insert(id, concept);

        id
    }

    /// Get concept
    pub fn get_concept(&self, id: u64) -> Option<&Concept> {
        self.concepts.get(&id)
    }

    /// Get concept by name
    pub fn get_by_name(&self, name: &str) -> Option<&Concept> {
        let id = self.by_name.get(name)?;
        self.concepts.get(id)
    }

    /// Set property
    pub fn set_property(&mut self, concept_id: u64, key: &str, value: PropertyValue) {
        if let Some(concept) = self.concepts.get_mut(&concept_id) {
            concept.properties.insert(key.into(), value);
        }
    }

    /// Set definition
    pub fn set_definition(&mut self, concept_id: u64, definition: &str) {
        if let Some(concept) = self.concepts.get_mut(&concept_id) {
            concept.definition = Some(definition.into());
        }
    }

    /// Add example
    pub fn add_example(&mut self, concept_id: u64, example: &str) {
        if let Some(concept) = self.concepts.get_mut(&concept_id) {
            concept.examples.push(example.into());
        }
    }

    /// Add relation
    pub fn add_relation(
        &mut self,
        source: u64,
        target: u64,
        relation_type: RelationType,
        strength: f64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let relation = SemanticRelation {
            id,
            source,
            target,
            relation_type,
            strength,
            bidirectional: false,
            context: None,
        };

        self.relations.insert(id, relation);
        self.outgoing.entry(source).or_insert_with(Vec::new).push(id);
        self.incoming.entry(target).or_insert_with(Vec::new).push(id);

        id
    }

    /// Get related concepts
    pub fn get_related(&self, concept_id: u64, relation_type: Option<RelationType>) -> Vec<&Concept> {
        let mut related = Vec::new();

        if let Some(rels) = self.outgoing.get(&concept_id) {
            for rel_id in rels {
                if let Some(rel) = self.relations.get(rel_id) {
                    if relation_type.is_none() || Some(rel.relation_type) == relation_type {
                        if let Some(concept) = self.concepts.get(&rel.target) {
                            related.push(concept);
                        }
                    }
                }
            }
        }

        related
    }

    /// Get ancestors (via IsA)
    pub fn get_ancestors(&self, concept_id: u64) -> Vec<&Concept> {
        let mut ancestors = Vec::new();
        let mut stack = vec![concept_id];
        let mut visited = Vec::new();

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);

            for parent in self.get_related(current, Some(RelationType::IsA)) {
                if !ancestors.iter().any(|a: &&Concept| a.id == parent.id) {
                    ancestors.push(parent);
                    stack.push(parent.id);
                }
            }
        }

        ancestors
    }

    /// Get descendants (inverse IsA)
    pub fn get_descendants(&self, concept_id: u64) -> Vec<&Concept> {
        let mut descendants = Vec::new();
        let mut stack = vec![concept_id];
        let mut visited = Vec::new();

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.push(current);

            if let Some(rels) = self.incoming.get(&current) {
                for rel_id in rels {
                    if let Some(rel) = self.relations.get(rel_id) {
                        if rel.relation_type == RelationType::IsA {
                            if let Some(child) = self.concepts.get(&rel.source) {
                                if !descendants.iter().any(|d: &&Concept| d.id == child.id) {
                                    descendants.push(child);
                                    stack.push(child.id);
                                }
                            }
                        }
                    }
                }
            }
        }

        descendants
    }

    /// Activate concept (spreading activation)
    pub fn activate(&mut self, concept_id: u64, initial: f64, decay: f64, depth: usize) {
        let mut to_activate: Vec<(u64, f64, usize)> = vec![(concept_id, initial, 0)];
        let mut visited = Vec::new();

        while let Some((id, activation, d)) = to_activate.pop() {
            if visited.contains(&id) || d > depth {
                continue;
            }
            visited.push(id);

            if let Some(concept) = self.concepts.get_mut(&id) {
                concept.activation = (concept.activation + activation).min(1.0);
                concept.last_accessed = Timestamp::now();
                concept.access_count += 1;
            }

            // Spread to related
            let next_activation = activation * decay;
            if next_activation > 0.01 {
                if let Some(rels) = self.outgoing.get(&id) {
                    for rel_id in rels.clone() {
                        if let Some(rel) = self.relations.get(&rel_id) {
                            let spread = next_activation * rel.strength;
                            to_activate.push((rel.target, spread, d + 1));
                        }
                    }
                }
            }
        }
    }

    /// Get most active concepts
    pub fn get_active(&self, min_activation: f64, limit: usize) -> Vec<&Concept> {
        let mut active: Vec<&Concept> = self.concepts.values()
            .filter(|c| c.activation >= min_activation)
            .collect();

        active.sort_by(|a, b| b.activation.partial_cmp(&a.activation).unwrap_or(core::cmp::Ordering::Equal));

        if active.len() > limit {
            active.truncate(limit);
        }

        active
    }

    /// Decay all activations
    pub fn decay_all(&mut self, factor: f64) {
        for concept in self.concepts.values_mut() {
            concept.activation *= factor;
            if concept.activation < 0.01 {
                concept.activation = 0.0;
            }
        }
    }

    /// Find path between concepts
    pub fn find_path(&self, from: u64, to: u64, max_depth: usize) -> Option<Vec<u64>> {
        let mut queue: Vec<(u64, Vec<u64>)> = vec![(from, vec![from])];
        let mut visited = Vec::new();

        while let Some((current, path)) = queue.pop() {
            if current == to {
                return Some(path);
            }

            if visited.contains(&current) || path.len() > max_depth {
                continue;
            }
            visited.push(current);

            if let Some(rels) = self.outgoing.get(&current) {
                for rel_id in rels {
                    if let Some(rel) = self.relations.get(rel_id) {
                        let mut new_path = path.clone();
                        new_path.push(rel.target);
                        queue.push((rel.target, new_path));
                    }
                }
            }
        }

        None
    }

    /// Concept count
    pub fn concept_count(&self) -> usize {
        self.concepts.len()
    }

    /// Relation count
    pub fn relation_count(&self) -> usize {
        self.relations.len()
    }
}

impl Default for SemanticGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SEMANTIC MEMORY
// ============================================================================

/// Semantic memory system
pub struct SemanticMemory {
    /// Knowledge graph
    graph: SemanticGraph,
    /// Configuration
    config: SemanticConfig,
    /// Statistics
    stats: SemanticStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct SemanticConfig {
    /// Activation decay per cycle
    pub activation_decay: f64,
    /// Spreading activation depth
    pub spread_depth: usize,
    /// Spreading decay factor
    pub spread_decay: f64,
    /// Minimum strength threshold
    pub min_strength: f64,
}

impl Default for SemanticConfig {
    fn default() -> Self {
        Self {
            activation_decay: 0.9,
            spread_depth: 3,
            spread_decay: 0.5,
            min_strength: 0.1,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct SemanticStats {
    /// Concepts stored
    pub concepts_stored: u64,
    /// Relations stored
    pub relations_stored: u64,
    /// Retrievals
    pub retrievals: u64,
    /// Average path length
    pub avg_path_length: f64,
}

impl SemanticMemory {
    /// Create new memory
    pub fn new(config: SemanticConfig) -> Self {
        Self {
            graph: SemanticGraph::new(),
            config,
            stats: SemanticStats::default(),
        }
    }

    /// Learn concept
    pub fn learn(&mut self, name: &str, concept_type: ConceptType, definition: Option<&str>) -> u64 {
        let id = self.graph.add_concept(name, concept_type);

        if let Some(def) = definition {
            self.graph.set_definition(id, def);
        }

        self.stats.concepts_stored += 1;
        id
    }

    /// Learn relation
    pub fn relate(
        &mut self,
        source: u64,
        target: u64,
        relation: RelationType,
    ) -> u64 {
        let id = self.graph.add_relation(source, target, relation, 1.0);
        self.stats.relations_stored += 1;
        id
    }

    /// Query by name
    pub fn query(&mut self, name: &str) -> Option<&Concept> {
        self.stats.retrievals += 1;

        if let Some(id) = self.graph.by_name.get(name) {
            self.graph.activate(*id, 1.0, self.config.spread_decay, self.config.spread_depth);
        }

        self.graph.get_by_name(name)
    }

    /// Query related
    pub fn query_related(&mut self, name: &str, relation: RelationType) -> Vec<&Concept> {
        let concept = match self.graph.by_name.get(name) {
            Some(id) => *id,
            None => return Vec::new(),
        };

        self.stats.retrievals += 1;
        self.graph.activate(concept, 1.0, self.config.spread_decay, self.config.spread_depth);
        self.graph.get_related(concept, Some(relation))
    }

    /// Check if concept is instance of category
    pub fn is_instance_of(&self, concept: u64, category: u64) -> bool {
        // Check direct InstanceOf relation
        if self.graph.get_related(concept, Some(RelationType::InstanceOf))
            .iter().any(|c| c.id == category) {
            return true;
        }

        // Check IsA hierarchy
        self.graph.get_ancestors(concept).iter().any(|a| a.id == category)
    }

    /// Get all properties (inherited)
    pub fn get_all_properties(&self, concept_id: u64) -> BTreeMap<String, PropertyValue> {
        let mut props = BTreeMap::new();

        // Get from ancestors first (so local props override)
        for ancestor in self.graph.get_ancestors(concept_id) {
            for (key, value) in &ancestor.properties {
                props.insert(key.clone(), value.clone());
            }
        }

        // Get local properties
        if let Some(concept) = self.graph.get_concept(concept_id) {
            for (key, value) in &concept.properties {
                props.insert(key.clone(), value.clone());
            }
        }

        props
    }

    /// Decay cycle
    pub fn decay_cycle(&mut self) {
        self.graph.decay_all(self.config.activation_decay);
    }

    /// Get graph
    pub fn graph(&self) -> &SemanticGraph {
        &self.graph
    }

    /// Get mutable graph
    pub fn graph_mut(&mut self) -> &mut SemanticGraph {
        &mut self.graph
    }

    /// Get statistics
    pub fn stats(&self) -> &SemanticStats {
        &self.stats
    }
}

impl Default for SemanticMemory {
    fn default() -> Self {
        Self::new(SemanticConfig::default())
    }
}

// ============================================================================
// INFERENCE
// ============================================================================

/// Semantic inference engine
pub struct SemanticInference {
    /// Memory reference
    config: InferenceConfig,
}

/// Inference configuration
#[derive(Debug, Clone)]
pub struct InferenceConfig {
    /// Minimum confidence threshold
    pub min_confidence: f64,
    /// Maximum inference depth
    pub max_depth: usize,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            max_depth: 5,
        }
    }
}

/// Inference result
#[derive(Debug, Clone)]
pub struct InferenceResult {
    /// Inferred concept
    pub concept: u64,
    /// Confidence
    pub confidence: f64,
    /// Reasoning chain
    pub reasoning: Vec<String>,
}

impl SemanticInference {
    /// Create new inference engine
    pub fn new(config: InferenceConfig) -> Self {
        Self { config }
    }

    /// Infer properties from category
    pub fn infer_properties(
        &self,
        memory: &SemanticMemory,
        concept_id: u64,
    ) -> Vec<(String, PropertyValue, f64)> {
        let mut inferred = Vec::new();

        // Get properties from IsA hierarchy
        for ancestor in memory.graph().get_ancestors(concept_id) {
            let distance = 1; // Simplified, would compute actual distance
            let confidence = 1.0 / (distance as f64 + 1.0);

            if confidence >= self.config.min_confidence {
                for (key, value) in &ancestor.properties {
                    // Check if not overridden
                    if let Some(concept) = memory.graph().get_concept(concept_id) {
                        if !concept.properties.contains_key(key) {
                            inferred.push((key.clone(), value.clone(), confidence));
                        }
                    }
                }
            }
        }

        inferred
    }

    /// Infer category from properties
    pub fn infer_category(
        &self,
        memory: &SemanticMemory,
        properties: &BTreeMap<String, PropertyValue>,
    ) -> Vec<InferenceResult> {
        let mut results = Vec::new();

        // Find concepts with matching properties
        for concept in memory.graph().concepts.values() {
            let mut match_count = 0;
            let total = properties.len();

            for (key, value) in properties {
                if let Some(prop_value) = concept.properties.get(key) {
                    if Self::values_match(prop_value, value) {
                        match_count += 1;
                    }
                }
            }

            if total > 0 {
                let confidence = match_count as f64 / total as f64;
                if confidence >= self.config.min_confidence {
                    results.push(InferenceResult {
                        concept: concept.id,
                        confidence,
                        reasoning: vec![format!(
                            "Matched {} of {} properties",
                            match_count, total
                        )],
                    });
                }
            }
        }

        results.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(core::cmp::Ordering::Equal));
        results
    }

    fn values_match(a: &PropertyValue, b: &PropertyValue) -> bool {
        match (a, b) {
            (PropertyValue::Bool(x), PropertyValue::Bool(y)) => x == y,
            (PropertyValue::Int(x), PropertyValue::Int(y)) => x == y,
            (PropertyValue::String(x), PropertyValue::String(y)) => x == y,
            _ => false,
        }
    }
}

impl Default for SemanticInference {
    fn default() -> Self {
        Self::new(InferenceConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concept_creation() {
        let mut memory = SemanticMemory::default();

        let animal = memory.learn("Animal", ConceptType::Category, Some("Living organism"));
        let dog = memory.learn("Dog", ConceptType::Category, Some("Domestic canine"));

        memory.relate(dog, animal, RelationType::IsA);

        assert!(memory.query("Dog").is_some());
    }

    #[test]
    fn test_inheritance() {
        let mut memory = SemanticMemory::default();

        let animal = memory.learn("Animal", ConceptType::Category, None);
        let mammal = memory.learn("Mammal", ConceptType::Category, None);
        let dog = memory.learn("Dog", ConceptType::Category, None);

        memory.relate(mammal, animal, RelationType::IsA);
        memory.relate(dog, mammal, RelationType::IsA);

        memory.graph_mut().set_property(animal, "alive", PropertyValue::Bool(true));

        let props = memory.get_all_properties(dog);
        assert!(props.contains_key("alive"));
    }

    #[test]
    fn test_spreading_activation() {
        let mut memory = SemanticMemory::default();

        let a = memory.learn("A", ConceptType::Abstract, None);
        let b = memory.learn("B", ConceptType::Abstract, None);
        let c = memory.learn("C", ConceptType::Abstract, None);

        memory.relate(a, b, RelationType::RelatedTo);
        memory.relate(b, c, RelationType::RelatedTo);

        // Activate A
        memory.graph_mut().activate(a, 1.0, 0.5, 3);

        // Check that B and C got some activation
        let b_concept = memory.graph().get_concept(b).unwrap();
        assert!(b_concept.activation > 0.0);
    }

    #[test]
    fn test_path_finding() {
        let mut memory = SemanticMemory::default();

        let a = memory.learn("A", ConceptType::Abstract, None);
        let b = memory.learn("B", ConceptType::Abstract, None);
        let c = memory.learn("C", ConceptType::Abstract, None);

        memory.relate(a, b, RelationType::RelatedTo);
        memory.relate(b, c, RelationType::RelatedTo);

        let path = memory.graph().find_path(a, c, 5);
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 3); // A -> B -> C
    }
}
