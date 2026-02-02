//! # Memory Association
//!
//! Manages associative memory networks.
//! Links concepts and enables spreading activation.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory

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
// ASSOCIATION TYPES
// ============================================================================

/// Concept node
#[derive(Debug, Clone)]
pub struct ConceptNode {
    /// Node ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Activation level
    pub activation: f64,
    /// Base activation
    pub base_activation: f64,
    /// Properties
    pub properties: BTreeMap<String, String>,
    /// Created
    pub created: Timestamp,
    /// Last activated
    pub last_activated: Timestamp,
}

/// Association link
#[derive(Debug, Clone)]
pub struct AssociationLink {
    /// Link ID
    pub id: u64,
    /// Source node
    pub source: u64,
    /// Target node
    pub target: u64,
    /// Link type
    pub link_type: LinkType,
    /// Strength
    pub strength: f64,
    /// Created
    pub created: Timestamp,
}

/// Link type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkType {
    IsA,
    HasA,
    PartOf,
    RelatedTo,
    CausedBy,
    Causes,
    SimilarTo,
    OppositeTo,
    UsedFor,
    FoundIn,
}

/// Activation result
#[derive(Debug, Clone)]
pub struct ActivationResult {
    /// Nodes activated
    pub activated: Vec<(u64, f64)>,
    /// Spread depth
    pub depth: usize,
    /// Total activation
    pub total_activation: f64,
}

/// Association query
#[derive(Debug, Clone)]
pub struct AssociationQuery {
    /// Start nodes
    pub start: Vec<u64>,
    /// Link types
    pub link_types: Option<Vec<LinkType>>,
    /// Maximum depth
    pub max_depth: usize,
    /// Minimum strength
    pub min_strength: f64,
}

// ============================================================================
// ASSOCIATIVE MEMORY
// ============================================================================

/// Associative memory
pub struct AssociativeMemory {
    /// Nodes
    nodes: BTreeMap<u64, ConceptNode>,
    /// Links
    links: BTreeMap<u64, AssociationLink>,
    /// Forward index (source -> links)
    forward: BTreeMap<u64, Vec<u64>>,
    /// Backward index (target -> links)
    backward: BTreeMap<u64, Vec<u64>>,
    /// Name to ID
    name_to_id: BTreeMap<String, u64>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: AssociationConfig,
    /// Statistics
    stats: AssociationStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct AssociationConfig {
    /// Decay rate
    pub decay_rate: f64,
    /// Spreading factor
    pub spreading_factor: f64,
    /// Activation threshold
    pub activation_threshold: f64,
    /// Maximum activation
    pub max_activation: f64,
}

impl Default for AssociationConfig {
    fn default() -> Self {
        Self {
            decay_rate: 0.1,
            spreading_factor: 0.7,
            activation_threshold: 0.1,
            max_activation: 1.0,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct AssociationStats {
    /// Nodes created
    pub nodes_created: u64,
    /// Links created
    pub links_created: u64,
    /// Activations performed
    pub activations: u64,
    /// Queries performed
    pub queries: u64,
}

impl AssociativeMemory {
    /// Create new memory
    pub fn new(config: AssociationConfig) -> Self {
        Self {
            nodes: BTreeMap::new(),
            links: BTreeMap::new(),
            forward: BTreeMap::new(),
            backward: BTreeMap::new(),
            name_to_id: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: AssociationStats::default(),
        }
    }

    /// Add concept
    pub fn add_concept(&mut self, name: &str, properties: BTreeMap<String, String>) -> u64 {
        // Check if exists
        if let Some(&id) = self.name_to_id.get(name) {
            return id;
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let node = ConceptNode {
            id,
            name: name.into(),
            activation: 0.0,
            base_activation: 0.5,
            properties,
            created: now,
            last_activated: now,
        };

        self.name_to_id.insert(name.into(), id);
        self.nodes.insert(id, node);
        self.stats.nodes_created += 1;

        id
    }

    /// Get concept by name
    pub fn get_by_name(&self, name: &str) -> Option<&ConceptNode> {
        self.name_to_id.get(name)
            .and_then(|id| self.nodes.get(id))
    }

    /// Get concept by ID
    pub fn get(&self, id: u64) -> Option<&ConceptNode> {
        self.nodes.get(&id)
    }

    /// Associate concepts
    pub fn associate(
        &mut self,
        source: u64,
        target: u64,
        link_type: LinkType,
        strength: f64,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let link = AssociationLink {
            id,
            source,
            target,
            link_type,
            strength: strength.clamp(0.0, 1.0),
            created: Timestamp::now(),
        };

        // Add to indexes
        self.forward.entry(source).or_insert_with(Vec::new).push(id);
        self.backward.entry(target).or_insert_with(Vec::new).push(id);

        self.links.insert(id, link);
        self.stats.links_created += 1;

        id
    }

    /// Activate concept
    pub fn activate(&mut self, id: u64, amount: f64) {
        if let Some(node) = self.nodes.get_mut(&id) {
            node.activation = (node.activation + amount).min(self.config.max_activation);
            node.last_activated = Timestamp::now();
        }
    }

    /// Spread activation
    pub fn spread_activation(&mut self, start_ids: &[u64], initial: f64, depth: usize) -> ActivationResult {
        self.stats.activations += 1;

        let mut activated = Vec::new();
        let mut visited = BTreeSet::new();
        let mut current_level = start_ids.to_vec();
        let mut current_depth = 0;

        // Initialize starting nodes
        for &id in start_ids {
            self.activate(id, initial);
            activated.push((id, initial));
            visited.insert(id);
        }

        while current_depth < depth && !current_level.is_empty() {
            let mut next_level = Vec::new();

            for source_id in &current_level {
                let source_activation = self.nodes.get(source_id)
                    .map(|n| n.activation)
                    .unwrap_or(0.0);

                // Get outgoing links
                if let Some(link_ids) = self.forward.get(source_id).cloned() {
                    for link_id in link_ids {
                        if let Some(link) = self.links.get(&link_id) {
                            let target_id = link.target;

                            if visited.contains(&target_id) {
                                continue;
                            }

                            // Calculate spread amount
                            let spread = source_activation *
                                         self.config.spreading_factor *
                                         link.strength;

                            if spread >= self.config.activation_threshold {
                                self.activate(target_id, spread);
                                activated.push((target_id, spread));
                                visited.insert(target_id);
                                next_level.push(target_id);
                            }
                        }
                    }
                }
            }

            current_level = next_level;
            current_depth += 1;
        }

        let total_activation: f64 = activated.iter().map(|(_, a)| a).sum();

        ActivationResult {
            activated,
            depth: current_depth,
            total_activation,
        }
    }

    /// Decay activations
    pub fn decay(&mut self) {
        for node in self.nodes.values_mut() {
            node.activation *= 1.0 - self.config.decay_rate;

            if node.activation < self.config.activation_threshold {
                node.activation = 0.0;
            }
        }
    }

    /// Find associations
    pub fn find_associations(&mut self, query: &AssociationQuery) -> Vec<(u64, Vec<u64>)> {
        self.stats.queries += 1;

        let mut results = Vec::new();
        let mut visited = BTreeSet::new();
        let mut current_level: Vec<(u64, Vec<u64>)> = query.start.iter()
            .map(|&id| (id, vec![id]))
            .collect();

        for &id in &query.start {
            visited.insert(id);
        }

        for _ in 0..query.max_depth {
            let mut next_level = Vec::new();

            for (source_id, path) in &current_level {
                if let Some(link_ids) = self.forward.get(source_id).cloned() {
                    for link_id in link_ids {
                        if let Some(link) = self.links.get(&link_id) {
                            // Check link type
                            if let Some(ref types) = query.link_types {
                                if !types.contains(&link.link_type) {
                                    continue;
                                }
                            }

                            // Check strength
                            if link.strength < query.min_strength {
                                continue;
                            }

                            let target = link.target;

                            if visited.contains(&target) {
                                continue;
                            }

                            visited.insert(target);

                            let mut new_path = path.clone();
                            new_path.push(target);

                            results.push((target, new_path.clone()));
                            next_level.push((target, new_path));
                        }
                    }
                }
            }

            current_level = next_level;
        }

        results
    }

    /// Get related concepts
    pub fn related(&self, id: u64) -> Vec<&ConceptNode> {
        let mut related = BTreeSet::new();

        // Forward links
        if let Some(link_ids) = self.forward.get(&id) {
            for link_id in link_ids {
                if let Some(link) = self.links.get(link_id) {
                    related.insert(link.target);
                }
            }
        }

        // Backward links
        if let Some(link_ids) = self.backward.get(&id) {
            for link_id in link_ids {
                if let Some(link) = self.links.get(link_id) {
                    related.insert(link.source);
                }
            }
        }

        related.iter()
            .filter_map(|id| self.nodes.get(id))
            .collect()
    }

    /// Get most active concepts
    pub fn most_active(&self, limit: usize) -> Vec<&ConceptNode> {
        let mut nodes: Vec<_> = self.nodes.values().collect();
        nodes.sort_by(|a, b| b.activation.partial_cmp(&a.activation).unwrap_or(core::cmp::Ordering::Equal));
        nodes.into_iter().take(limit).collect()
    }

    /// Get statistics
    pub fn stats(&self) -> &AssociationStats {
        &self.stats
    }
}

impl Default for AssociativeMemory {
    fn default() -> Self {
        Self::new(AssociationConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_concept() {
        let mut memory = AssociativeMemory::default();

        let id = memory.add_concept("dog", BTreeMap::new());
        assert!(memory.get(id).is_some());
    }

    #[test]
    fn test_get_by_name() {
        let mut memory = AssociativeMemory::default();

        memory.add_concept("cat", BTreeMap::new());
        let node = memory.get_by_name("cat");

        assert!(node.is_some());
        assert_eq!(node.unwrap().name, "cat");
    }

    #[test]
    fn test_associate() {
        let mut memory = AssociativeMemory::default();

        let dog = memory.add_concept("dog", BTreeMap::new());
        let animal = memory.add_concept("animal", BTreeMap::new());

        memory.associate(dog, animal, LinkType::IsA, 0.9);

        let related = memory.related(dog);
        assert_eq!(related.len(), 1);
        assert_eq!(related[0].name, "animal");
    }

    #[test]
    fn test_spread_activation() {
        let mut memory = AssociativeMemory::default();

        let a = memory.add_concept("A", BTreeMap::new());
        let b = memory.add_concept("B", BTreeMap::new());
        let c = memory.add_concept("C", BTreeMap::new());

        memory.associate(a, b, LinkType::RelatedTo, 0.8);
        memory.associate(b, c, LinkType::RelatedTo, 0.8);

        let result = memory.spread_activation(&[a], 1.0, 3);

        assert!(result.activated.len() > 1);
        assert!(result.total_activation > 0.0);
    }

    #[test]
    fn test_decay() {
        let mut memory = AssociativeMemory::default();

        let id = memory.add_concept("test", BTreeMap::new());
        memory.activate(id, 1.0);

        memory.decay();

        let node = memory.get(id).unwrap();
        assert!(node.activation < 1.0);
    }

    #[test]
    fn test_find_associations() {
        let mut memory = AssociativeMemory::default();

        let a = memory.add_concept("A", BTreeMap::new());
        let b = memory.add_concept("B", BTreeMap::new());
        let c = memory.add_concept("C", BTreeMap::new());

        memory.associate(a, b, LinkType::IsA, 0.9);
        memory.associate(b, c, LinkType::IsA, 0.9);

        let results = memory.find_associations(&AssociationQuery {
            start: vec![a],
            link_types: Some(vec![LinkType::IsA]),
            max_depth: 3,
            min_strength: 0.5,
        });

        assert_eq!(results.len(), 2);
    }
}
