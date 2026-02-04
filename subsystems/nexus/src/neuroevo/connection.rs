//! Connection genes for NEAT genomes.

use crate::neuroevo::types::{InnovationNumber, NodeId};

/// A connection gene in the NEAT genome
#[derive(Debug, Clone)]
pub struct ConnectionGene {
    /// Source node
    pub from: NodeId,
    /// Destination node
    pub to: NodeId,
    /// Connection weight
    pub weight: f64,
    /// Is this connection enabled?
    pub enabled: bool,
    /// Global innovation number for crossover alignment
    pub innovation: InnovationNumber,
    /// Recurrent flag
    pub recurrent: bool,
}

impl ConnectionGene {
    /// Create a new connection gene
    pub fn new(from: NodeId, to: NodeId, weight: f64, innovation: InnovationNumber) -> Self {
        Self {
            from,
            to,
            weight,
            enabled: true,
            innovation,
            recurrent: false,
        }
    }

    /// Disable this connection
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if connection is active
    pub fn is_active(&self) -> bool {
        self.enabled
    }
}
