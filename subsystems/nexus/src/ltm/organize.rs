//! # LTM Organization
//!
//! Organizes and structures long-term memories.
//! Supports hierarchies, schemas, and knowledge graphs.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory Engine

#![allow(dead_code)]

extern crate alloc;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// ORGANIZATION TYPES
// ============================================================================

/// Memory category
#[derive(Debug, Clone)]
pub struct Category {
    /// Category ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
    /// Parent category
    pub parent: Option<u64>,
    /// Children
    pub children: Vec<u64>,
    /// Members (memory IDs)
    pub members: BTreeSet<u64>,
    /// Schema
    pub schema: Option<Schema>,
}

/// Schema
#[derive(Debug, Clone)]
pub struct Schema {
    /// Schema ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Fields
    pub fields: Vec<SchemaField>,
    /// Constraints
    pub constraints: Vec<Constraint>,
}

/// Schema field
#[derive(Debug, Clone)]
pub struct SchemaField {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Required
    pub required: bool,
    /// Default value
    pub default: Option<String>,
}

/// Field type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    Text,
    Number,
    Boolean,
    Date,
    Reference,
    List,
    Map,
}

/// Constraint
#[derive(Debug, Clone)]
pub struct Constraint {
    /// Constraint type
    pub constraint_type: ConstraintType,
    /// Fields involved
    pub fields: Vec<String>,
    /// Value
    pub value: Option<String>,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintType {
    Unique,
    NotNull,
    Range,
    Pattern,
    Reference,
}

/// Knowledge node
#[derive(Debug, Clone)]
pub struct KnowledgeNode {
    /// Node ID
    pub id: u64,
    /// Label
    pub label: String,
    /// Node type
    pub node_type: String,
    /// Properties
    pub properties: BTreeMap<String, String>,
    /// Memory references
    pub memory_refs: Vec<u64>,
}

/// Knowledge edge
#[derive(Debug, Clone)]
pub struct KnowledgeEdge {
    /// Edge ID
    pub id: u64,
    /// Source node
    pub source: u64,
    /// Target node
    pub target: u64,
    /// Relation type
    pub relation: String,
    /// Weight
    pub weight: f64,
    /// Properties
    pub properties: BTreeMap<String, String>,
}

// ============================================================================
// MEMORY ORGANIZER
// ============================================================================

/// Memory organizer
pub struct MemoryOrganizer {
    /// Categories
    categories: BTreeMap<u64, Category>,
    /// Knowledge nodes
    nodes: BTreeMap<u64, KnowledgeNode>,
    /// Knowledge edges
    edges: BTreeMap<u64, KnowledgeEdge>,
    /// Memory to categories mapping
    memory_categories: BTreeMap<u64, Vec<u64>>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: OrganizerConfig,
    /// Statistics
    stats: OrganizerStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct OrganizerConfig {
    /// Maximum category depth
    pub max_depth: usize,
    /// Auto-categorize
    pub auto_categorize: bool,
    /// Enable knowledge graph
    pub enable_graph: bool,
}

impl Default for OrganizerConfig {
    fn default() -> Self {
        Self {
            max_depth: 10,
            auto_categorize: true,
            enable_graph: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct OrganizerStats {
    /// Total categories
    pub total_categories: u64,
    /// Total nodes
    pub total_nodes: u64,
    /// Total edges
    pub total_edges: u64,
    /// Organized memories
    pub organized_memories: u64,
}

impl MemoryOrganizer {
    /// Create new organizer
    pub fn new(config: OrganizerConfig) -> Self {
        Self {
            categories: BTreeMap::new(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            memory_categories: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: OrganizerStats::default(),
        }
    }

    /// Create category
    pub fn create_category(&mut self, name: &str, description: &str, parent: Option<u64>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let category = Category {
            id,
            name: name.into(),
            description: description.into(),
            parent,
            children: Vec::new(),
            members: BTreeSet::new(),
            schema: None,
        };

        // Update parent
        if let Some(parent_id) = parent {
            if let Some(parent_cat) = self.categories.get_mut(&parent_id) {
                parent_cat.children.push(id);
            }
        }

        self.categories.insert(id, category);
        self.stats.total_categories += 1;

        id
    }

    /// Set category schema
    pub fn set_schema(&mut self, category_id: u64, schema: Schema) {
        if let Some(category) = self.categories.get_mut(&category_id) {
            category.schema = Some(schema);
        }
    }

    /// Categorize memory
    pub fn categorize(&mut self, memory_id: u64, category_id: u64) {
        if let Some(category) = self.categories.get_mut(&category_id) {
            if category.members.insert(memory_id) {
                self.stats.organized_memories += 1;
            }
        }

        self.memory_categories.entry(memory_id)
            .or_insert_with(Vec::new)
            .push(category_id);
    }

    /// Uncategorize memory
    pub fn uncategorize(&mut self, memory_id: u64, category_id: u64) {
        if let Some(category) = self.categories.get_mut(&category_id) {
            category.members.remove(&memory_id);
        }

        if let Some(cats) = self.memory_categories.get_mut(&memory_id) {
            cats.retain(|&c| c != category_id);
        }
    }

    /// Get category
    pub fn get_category(&self, id: u64) -> Option<&Category> {
        self.categories.get(&id)
    }

    /// Get memory categories
    pub fn get_memory_categories(&self, memory_id: u64) -> Vec<&Category> {
        self.memory_categories.get(&memory_id)
            .map(|cats| {
                cats.iter()
                    .filter_map(|id| self.categories.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get category hierarchy
    pub fn get_hierarchy(&self, category_id: u64) -> Vec<&Category> {
        let mut path = Vec::new();
        let mut current = Some(category_id);

        while let Some(id) = current {
            if let Some(cat) = self.categories.get(&id) {
                path.push(cat);
                current = cat.parent;
            } else {
                break;
            }
        }

        path.reverse();
        path
    }

    /// Create knowledge node
    pub fn create_node(&mut self, label: &str, node_type: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let node = KnowledgeNode {
            id,
            label: label.into(),
            node_type: node_type.into(),
            properties: BTreeMap::new(),
            memory_refs: Vec::new(),
        };

        self.nodes.insert(id, node);
        self.stats.total_nodes += 1;

        id
    }

    /// Create knowledge edge
    pub fn create_edge(&mut self, source: u64, target: u64, relation: &str) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let edge = KnowledgeEdge {
            id,
            source,
            target,
            relation: relation.into(),
            weight: 1.0,
            properties: BTreeMap::new(),
        };

        self.edges.insert(id, edge);
        self.stats.total_edges += 1;

        id
    }

    /// Link memory to node
    pub fn link_memory_to_node(&mut self, memory_id: u64, node_id: u64) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            if !node.memory_refs.contains(&memory_id) {
                node.memory_refs.push(memory_id);
            }
        }
    }

    /// Get node
    pub fn get_node(&self, id: u64) -> Option<&KnowledgeNode> {
        self.nodes.get(&id)
    }

    /// Get edges from node
    pub fn get_edges_from(&self, node_id: u64) -> Vec<&KnowledgeEdge> {
        self.edges.values()
            .filter(|e| e.source == node_id)
            .collect()
    }

    /// Get edges to node
    pub fn get_edges_to(&self, node_id: u64) -> Vec<&KnowledgeEdge> {
        self.edges.values()
            .filter(|e| e.target == node_id)
            .collect()
    }

    /// Find path between nodes
    pub fn find_path(&self, start: u64, end: u64) -> Option<Vec<u64>> {
        use alloc::collections::VecDeque;

        let mut visited = BTreeSet::new();
        let mut queue = VecDeque::new();
        let mut parents: BTreeMap<u64, u64> = BTreeMap::new();

        queue.push_back(start);
        visited.insert(start);

        while let Some(current) = queue.pop_front() {
            if current == end {
                // Reconstruct path
                let mut path = vec![end];
                let mut node = end;

                while let Some(&parent) = parents.get(&node) {
                    path.push(parent);
                    node = parent;
                }

                path.reverse();
                return Some(path);
            }

            for edge in self.get_edges_from(current) {
                if !visited.contains(&edge.target) {
                    visited.insert(edge.target);
                    parents.insert(edge.target, current);
                    queue.push_back(edge.target);
                }
            }
        }

        None
    }

    /// Find related nodes
    pub fn find_related(&self, node_id: u64, relation: &str) -> Vec<&KnowledgeNode> {
        self.edges.values()
            .filter(|e| e.source == node_id && e.relation == relation)
            .filter_map(|e| self.nodes.get(&e.target))
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &OrganizerStats {
        &self.stats
    }
}

impl Default for MemoryOrganizer {
    fn default() -> Self {
        Self::new(OrganizerConfig::default())
    }
}

// ============================================================================
// SCHEMA BUILDER
// ============================================================================

/// Schema builder
pub struct SchemaBuilder {
    name: String,
    fields: Vec<SchemaField>,
    constraints: Vec<Constraint>,
}

impl SchemaBuilder {
    /// Create new builder
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
            constraints: Vec::new(),
        }
    }

    /// Add field
    pub fn field(mut self, name: &str, field_type: FieldType, required: bool) -> Self {
        self.fields.push(SchemaField {
            name: name.into(),
            field_type,
            required,
            default: None,
        });
        self
    }

    /// Add constraint
    pub fn constraint(mut self, constraint_type: ConstraintType, fields: Vec<&str>) -> Self {
        self.constraints.push(Constraint {
            constraint_type,
            fields: fields.into_iter().map(String::from).collect(),
            value: None,
        });
        self
    }

    /// Build
    pub fn build(self, id: u64) -> Schema {
        Schema {
            id,
            name: self.name,
            fields: self.fields,
            constraints: self.constraints,
        }
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_category() {
        let mut organizer = MemoryOrganizer::default();

        let id = organizer.create_category("Test", "Test category", None);
        assert!(organizer.get_category(id).is_some());
    }

    #[test]
    fn test_category_hierarchy() {
        let mut organizer = MemoryOrganizer::default();

        let parent = organizer.create_category("Parent", "", None);
        let child = organizer.create_category("Child", "", Some(parent));

        let hierarchy = organizer.get_hierarchy(child);
        assert_eq!(hierarchy.len(), 2);
    }

    #[test]
    fn test_categorize_memory() {
        let mut organizer = MemoryOrganizer::default();

        let cat = organizer.create_category("Test", "", None);
        organizer.categorize(1, cat);

        let category = organizer.get_category(cat).unwrap();
        assert!(category.members.contains(&1));
    }

    #[test]
    fn test_knowledge_graph() {
        let mut organizer = MemoryOrganizer::default();

        let node1 = organizer.create_node("Paris", "City");
        let node2 = organizer.create_node("France", "Country");

        organizer.create_edge(node1, node2, "located_in");

        let related = organizer.find_related(node1, "located_in");
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].label, "France");
    }

    #[test]
    fn test_find_path() {
        let mut organizer = MemoryOrganizer::default();

        let a = organizer.create_node("A", "Node");
        let b = organizer.create_node("B", "Node");
        let c = organizer.create_node("C", "Node");

        organizer.create_edge(a, b, "to");
        organizer.create_edge(b, c, "to");

        let path = organizer.find_path(a, c);
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![a, b, c]);
    }

    #[test]
    fn test_schema_builder() {
        let schema = SchemaBuilder::new("Person")
            .field("name", FieldType::Text, true)
            .field("age", FieldType::Number, false)
            .constraint(ConstraintType::NotNull, vec!["name"])
            .build(1);

        assert_eq!(schema.fields.len(), 2);
        assert_eq!(schema.constraints.len(), 1);
    }
}
