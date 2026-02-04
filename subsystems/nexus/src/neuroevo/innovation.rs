//! Innovation tracking for NEAT historical markings.

use alloc::collections::BTreeMap;

use crate::neuroevo::types::{InnovationNumber, NodeId};

/// Global innovation tracker for maintaining historical markings
pub struct InnovationTracker {
    /// Current innovation number
    current: InnovationNumber,
    /// Map of (from, to) -> innovation for existing connections
    innovations: BTreeMap<(NodeId, NodeId), InnovationNumber>,
    /// Current node ID
    current_node: NodeId,
}

impl InnovationTracker {
    /// Create a new innovation tracker
    pub fn new() -> Self {
        Self {
            current: 0,
            innovations: BTreeMap::new(),
            current_node: 0,
        }
    }

    /// Get or create innovation number for a connection
    pub fn get_or_create(&mut self, from: NodeId, to: NodeId) -> InnovationNumber {
        let key = (from, to);
        if let Some(&innov) = self.innovations.get(&key) {
            innov
        } else {
            self.current += 1;
            self.innovations.insert(key, self.current);
            self.current
        }
    }

    /// Get a new node ID
    pub fn new_node_id(&mut self) -> NodeId {
        self.current_node += 1;
        self.current_node
    }

    /// Reset for a new generation (keeps node IDs, clears innovation cache)
    pub fn new_generation(&mut self) {
        self.innovations.clear();
    }
}

impl Default for InnovationTracker {
    fn default() -> Self {
        Self::new()
    }
}
