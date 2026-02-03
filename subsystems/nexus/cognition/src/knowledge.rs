//! # Knowledge Graph
//!
//! Semantic knowledge graph of the kernel - a neural-symbolic database
//! that stores relationships between code, concepts, and runtime behavior.
//!
//! ## Features
//!
//! - **Semantic Storage**: Stores meaning, not just data
//! - **Relationship Queries**: "What depends on X?"
//! - **Inference**: Derive new facts from existing ones
//! - **Temporal Knowledge**: Knowledge evolves over time

use alloc::{string::String, vec::Vec, boxed::Box, collections::BTreeMap, format};
use super::understanding::{Ast, Semantics};
use super::{Observation, Event};

/// Knowledge graph
pub struct KnowledgeGraph {
    /// Nodes in the graph
    nodes: BTreeMap<u64, KnowledgeNode>,
    /// Relations between nodes
    relations: Vec<Relation>,
    /// Index by name
    name_index: BTreeMap<String, u64>,
    /// Index by type
    type_index: BTreeMap<NodeType, Vec<u64>>,
    /// Next node ID
    next_id: u64,
    /// Inference engine
    inference: InferenceEngine,
    /// Statistics
    stats: GraphStats,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
            relations: Vec::new(),
            name_index: BTreeMap::new(),
            type_index: BTreeMap::new(),
            next_id: 1,
            inference: InferenceEngine::new(),
            stats: GraphStats::default(),
        }
    }
    
    /// Add a node to the graph
    pub fn add_node(&mut self, mut node: KnowledgeNode) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        
        node.id = id;
        
        // Update indices
        self.name_index.insert(node.name.clone(), id);
        self.type_index.entry(node.node_type.clone())
            .or_insert_with(Vec::new)
            .push(id);
        
        self.nodes.insert(id, node);
        self.stats.node_count += 1;
        
        id
    }
    
    /// Add a relation between nodes
    pub fn add_relation(&mut self, relation: Relation) {
        self.relations.push(relation);
        self.stats.relation_count += 1;
    }
    
    /// Get a node by ID
    pub fn get_node(&self, id: u64) -> Option<&KnowledgeNode> {
        self.nodes.get(&id)
    }
    
    /// Get a node by name
    pub fn get_by_name(&self, name: &str) -> Option<&KnowledgeNode> {
        self.name_index.get(name)
            .and_then(|id| self.nodes.get(id))
    }
    
    /// Query the knowledge graph
    pub fn query(&self, query_str: &str) -> super::QueryResult {
        let query = self.parse_query(query_str);
        
        match query.query_type {
            QueryType::FindByName(name) => {
                let nodes: Vec<KnowledgeNode> = self.name_index.get(&name)
                    .and_then(|id| self.nodes.get(id))
                    .cloned()
                    .into_iter()
                    .collect();
                
                super::QueryResult {
                    results: nodes,
                    confidence: if nodes.is_empty() { 0.0 } else { 1.0 },
                    related: Vec::new(),
                }
            }
            QueryType::FindByType(node_type) => {
                let nodes: Vec<KnowledgeNode> = self.type_index.get(&node_type)
                    .map(|ids| {
                        ids.iter()
                            .filter_map(|id| self.nodes.get(id).cloned())
                            .collect()
                    })
                    .unwrap_or_default();
                
                super::QueryResult {
                    results: nodes,
                    confidence: 0.9,
                    related: Vec::new(),
                }
            }
            QueryType::FindRelated(name, relation_type) => {
                let source_id = self.name_index.get(&name).copied();
                
                if let Some(id) = source_id {
                    let related: Vec<Relation> = self.relations.iter()
                        .filter(|r| r.source == id || r.target == id)
                        .filter(|r| relation_type.is_none() || Some(&r.relation_type) == relation_type.as_ref())
                        .cloned()
                        .collect();
                    
                    let nodes: Vec<KnowledgeNode> = related.iter()
                        .flat_map(|r| {
                            let other_id = if r.source == id { r.target } else { r.source };
                            self.nodes.get(&other_id).cloned()
                        })
                        .collect();
                    
                    super::QueryResult {
                        results: nodes,
                        confidence: 0.85,
                        related,
                    }
                } else {
                    super::QueryResult {
                        results: Vec::new(),
                        confidence: 0.0,
                        related: Vec::new(),
                    }
                }
            }
            QueryType::Infer(question) => {
                let inferred = self.inference.infer(&question, &self.nodes, &self.relations);
                
                super::QueryResult {
                    results: inferred.nodes,
                    confidence: inferred.confidence,
                    related: inferred.relations,
                }
            }
        }
    }
    
    fn parse_query(&self, query_str: &str) -> Query {
        let lower = query_str.to_lowercase();
        
        if lower.starts_with("find ") {
            let name = query_str[5..].trim().to_string();
            Query {
                query_type: QueryType::FindByName(name),
            }
        } else if lower.starts_with("type ") {
            let type_str = query_str[5..].trim();
            let node_type = match type_str {
                "function" => NodeType::Function,
                "struct" => NodeType::Struct,
                "module" => NodeType::Module,
                "concept" => NodeType::Concept,
                _ => NodeType::Other,
            };
            Query {
                query_type: QueryType::FindByType(node_type),
            }
        } else if lower.contains("related to") || lower.contains("depends on") {
            let parts: Vec<&str> = query_str.split("related to").collect();
            let name = parts.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
            Query {
                query_type: QueryType::FindRelated(name, None),
            }
        } else {
            Query {
                query_type: QueryType::Infer(query_str.to_string()),
            }
        }
    }
    
    /// Integrate code AST into knowledge graph
    pub fn integrate_code(&mut self, ast: &Ast, semantics: &Semantics) {
        for node in &ast.nodes {
            let kg_node = match &node.kind {
                super::understanding::NodeKind::Function { name, .. } => {
                    KnowledgeNode {
                        id: 0,
                        name: name.clone(),
                        node_type: NodeType::Function,
                        properties: BTreeMap::new(),
                        description: semantics.meanings.get(&node.id)
                            .map(|m| m.description.clone())
                            .unwrap_or_default(),
                        confidence: 1.0,
                        timestamp: 0,
                    }
                }
                super::understanding::NodeKind::Struct { name, .. } => {
                    KnowledgeNode {
                        id: 0,
                        name: name.clone(),
                        node_type: NodeType::Struct,
                        properties: BTreeMap::new(),
                        description: format!("Data structure {}", name),
                        confidence: 1.0,
                        timestamp: 0,
                    }
                }
                super::understanding::NodeKind::Mod { name } => {
                    KnowledgeNode {
                        id: 0,
                        name: name.clone(),
                        node_type: NodeType::Module,
                        properties: BTreeMap::new(),
                        description: format!("Module {}", name),
                        confidence: 1.0,
                        timestamp: 0,
                    }
                }
                _ => continue,
            };
            
            self.add_node(kg_node);
        }
        
        // Add relationships from semantics
        for rel in &semantics.relationships {
            if let (Some(from_node), Some(to_node)) = (
                self.nodes.get(&rel.from),
                self.nodes.get(&rel.to)
            ) {
                let relation_type = match rel.kind {
                    super::understanding::RelationshipKind::Calls => RelationType::Calls,
                    super::understanding::RelationshipKind::Uses => RelationType::Uses,
                    super::understanding::RelationshipKind::Implements => RelationType::Implements,
                    super::understanding::RelationshipKind::Extends => RelationType::Extends,
                    super::understanding::RelationshipKind::Contains => RelationType::Contains,
                    super::understanding::RelationshipKind::DependsOn => RelationType::DependsOn,
                };
                
                self.add_relation(Relation {
                    source: from_node.id,
                    target: to_node.id,
                    relation_type,
                    properties: BTreeMap::new(),
                    confidence: 1.0,
                });
            }
        }
    }
    
    /// Integrate observation into knowledge graph
    pub fn integrate_observation(&mut self, observation: &Observation) {
        // Add event as a node
        let event_node = KnowledgeNode {
            id: 0,
            name: format!("event_{}", observation.event.id),
            node_type: NodeType::Event,
            properties: observation.event.context.clone(),
            description: format!("{:?}", observation.event.event_type),
            confidence: 1.0,
            timestamp: observation.event.timestamp,
        };
        
        let event_id = self.add_node(event_node);
        
        // Add outcome as a node
        let outcome_node = KnowledgeNode {
            id: 0,
            name: format!("outcome_{}", observation.event.id),
            node_type: NodeType::Outcome,
            properties: BTreeMap::new(),
            description: observation.outcome.clone(),
            confidence: 1.0,
            timestamp: observation.event.timestamp,
        };
        
        let outcome_id = self.add_node(outcome_node);
        
        // Link event to outcome
        self.add_relation(Relation {
            source: event_id,
            target: outcome_id,
            relation_type: RelationType::CausedBy,
            properties: BTreeMap::new(),
            confidence: 0.8,
        });
    }
    
    /// Get all nodes of a specific type
    pub fn nodes_of_type(&self, node_type: &NodeType) -> Vec<&KnowledgeNode> {
        self.type_index.get(node_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.nodes.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }
    
    /// Find path between two nodes
    pub fn find_path(&self, from: u64, to: u64) -> Option<Vec<u64>> {
        // BFS
        let mut visited = BTreeMap::new();
        let mut queue = Vec::new();
        
        queue.push(from);
        visited.insert(from, None);
        
        while let Some(current) = queue.pop() {
            if current == to {
                // Reconstruct path
                let mut path = Vec::new();
                let mut node = Some(to);
                
                while let Some(n) = node {
                    path.push(n);
                    node = visited.get(&n).and_then(|&p| p);
                }
                
                path.reverse();
                return Some(path);
            }
            
            // Find neighbors
            for rel in &self.relations {
                let neighbor = if rel.source == current {
                    Some(rel.target)
                } else if rel.target == current {
                    Some(rel.source)
                } else {
                    None
                };
                
                if let Some(n) = neighbor {
                    if !visited.contains_key(&n) {
                        visited.insert(n, Some(current));
                        queue.push(n);
                    }
                }
            }
        }
        
        None
    }
    
    /// Export graph to DOT format for visualization
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph KnowledgeGraph {\n");
        dot.push_str("  rankdir=LR;\n");
        dot.push_str("  node [shape=box];\n\n");
        
        // Nodes
        for (id, node) in &self.nodes {
            let color = match node.node_type {
                NodeType::Function => "lightblue",
                NodeType::Struct => "lightgreen",
                NodeType::Module => "lightyellow",
                NodeType::Concept => "lightpink",
                NodeType::Event => "lightgray",
                NodeType::Outcome => "lightcoral",
                NodeType::Other => "white",
            };
            
            dot.push_str(&format!(
                "  n{} [label=\"{}\\n({:?})\" style=filled fillcolor={}];\n",
                id, node.name, node.node_type, color
            ));
        }
        
        dot.push_str("\n");
        
        // Edges
        for rel in &self.relations {
            dot.push_str(&format!(
                "  n{} -> n{} [label=\"{:?}\"];\n",
                rel.source, rel.target, rel.relation_type
            ));
        }
        
        dot.push_str("}\n");
        dot
    }
    
    /// Get statistics
    pub fn stats(&self) -> &GraphStats {
        &self.stats
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Knowledge node
#[derive(Debug, Clone)]
pub struct KnowledgeNode {
    pub id: u64,
    pub name: String,
    pub node_type: NodeType,
    pub properties: BTreeMap<String, String>,
    pub description: String,
    pub confidence: f32,
    pub timestamp: u64,
}

/// Node type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NodeType {
    Function,
    Struct,
    Module,
    Concept,
    Event,
    Outcome,
    Other,
}

/// Relation between nodes
#[derive(Debug, Clone)]
pub struct Relation {
    pub source: u64,
    pub target: u64,
    pub relation_type: RelationType,
    pub properties: BTreeMap<String, String>,
    pub confidence: f32,
}

/// Relation type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelationType {
    Calls,
    Uses,
    Implements,
    Extends,
    Contains,
    DependsOn,
    CausedBy,
    RelatedTo,
    InstanceOf,
    PartOf,
}

/// Query
struct Query {
    query_type: QueryType,
}

/// Query type
enum QueryType {
    FindByName(String),
    FindByType(NodeType),
    FindRelated(String, Option<RelationType>),
    Infer(String),
}

/// Inference engine
struct InferenceEngine {
    rules: Vec<InferenceRule>,
}

impl InferenceEngine {
    fn new() -> Self {
        Self {
            rules: Self::init_rules(),
        }
    }
    
    fn init_rules() -> Vec<InferenceRule> {
        vec![
            InferenceRule {
                name: "transitivity".into(),
                pattern: "A depends on B, B depends on C => A depends on C".into(),
                conclusion: "transitive_dependency".into(),
            },
            InferenceRule {
                name: "causality".into(),
                pattern: "A caused B, B caused C => A indirectly caused C".into(),
                conclusion: "indirect_causality".into(),
            },
        ]
    }
    
    fn infer(&self, question: &str, nodes: &BTreeMap<u64, KnowledgeNode>, relations: &[Relation]) -> InferenceResult {
        // Simple keyword-based inference
        let lower = question.to_lowercase();
        
        if lower.contains("cause") || lower.contains("why") {
            // Find causal chains
            let causal_relations: Vec<Relation> = relations.iter()
                .filter(|r| r.relation_type == RelationType::CausedBy)
                .cloned()
                .collect();
            
            let related_nodes: Vec<KnowledgeNode> = causal_relations.iter()
                .flat_map(|r| {
                    nodes.get(&r.source).cloned()
                        .into_iter()
                        .chain(nodes.get(&r.target).cloned())
                })
                .collect();
            
            InferenceResult {
                nodes: related_nodes,
                relations: causal_relations,
                confidence: 0.7,
            }
        } else if lower.contains("depend") {
            let dep_relations: Vec<Relation> = relations.iter()
                .filter(|r| r.relation_type == RelationType::DependsOn)
                .cloned()
                .collect();
            
            let related_nodes: Vec<KnowledgeNode> = dep_relations.iter()
                .flat_map(|r| {
                    nodes.get(&r.source).cloned()
                        .into_iter()
                        .chain(nodes.get(&r.target).cloned())
                })
                .collect();
            
            InferenceResult {
                nodes: related_nodes,
                relations: dep_relations,
                confidence: 0.8,
            }
        } else {
            InferenceResult {
                nodes: Vec::new(),
                relations: Vec::new(),
                confidence: 0.3,
            }
        }
    }
}

impl Default for InferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Inference rule
struct InferenceRule {
    name: String,
    pattern: String,
    conclusion: String,
}

/// Inference result
struct InferenceResult {
    nodes: Vec<KnowledgeNode>,
    relations: Vec<Relation>,
    confidence: f32,
}

/// Graph statistics
#[derive(Debug, Clone, Default)]
pub struct GraphStats {
    pub node_count: u64,
    pub relation_count: u64,
    pub query_count: u64,
    pub inference_count: u64,
}

/// Ontology for kernel concepts
pub struct KernelOntology {
    concepts: BTreeMap<String, Concept>,
    hierarchy: Vec<(String, String)>, // (child, parent)
}

impl KernelOntology {
    pub fn new() -> Self {
        let mut ontology = Self {
            concepts: BTreeMap::new(),
            hierarchy: Vec::new(),
        };
        
        ontology.init_kernel_concepts();
        ontology
    }
    
    fn init_kernel_concepts(&mut self) {
        // Core concepts
        self.add_concept(Concept {
            name: "Process".into(),
            description: "Execution context with resources".into(),
            properties: vec!["pid".into(), "memory_map".into(), "file_descriptors".into()],
        });
        
        self.add_concept(Concept {
            name: "Thread".into(),
            description: "Unit of execution within a process".into(),
            properties: vec!["tid".into(), "stack".into(), "registers".into()],
        });
        
        self.add_concept(Concept {
            name: "Memory".into(),
            description: "Virtual or physical memory region".into(),
            properties: vec!["address".into(), "size".into(), "permissions".into()],
        });
        
        self.add_concept(Concept {
            name: "Synchronization".into(),
            description: "Mechanism for coordinating concurrent access".into(),
            properties: vec!["type".into(), "owner".into()],
        });
        
        // Hierarchy
        self.hierarchy.push(("Thread".into(), "Process".into()));
        self.hierarchy.push(("Mutex".into(), "Synchronization".into()));
        self.hierarchy.push(("Spinlock".into(), "Synchronization".into()));
    }
    
    fn add_concept(&mut self, concept: Concept) {
        self.concepts.insert(concept.name.clone(), concept);
    }
    
    /// Check if concept A is a subtype of B
    pub fn is_subtype(&self, child: &str, parent: &str) -> bool {
        if child == parent {
            return true;
        }
        
        // Find direct parent
        for (c, p) in &self.hierarchy {
            if c == child {
                if p == parent {
                    return true;
                }
                // Recursive check
                return self.is_subtype(p, parent);
            }
        }
        
        false
    }
    
    /// Get all subtypes of a concept
    pub fn subtypes(&self, parent: &str) -> Vec<&str> {
        self.hierarchy.iter()
            .filter(|(_, p)| p == parent)
            .map(|(c, _)| c.as_str())
            .collect()
    }
}

impl Default for KernelOntology {
    fn default() -> Self {
        Self::new()
    }
}

/// Concept in the ontology
#[derive(Debug, Clone)]
pub struct Concept {
    pub name: String,
    pub description: String,
    pub properties: Vec<String>,
}
