//! Node types and node genes for NEAT genomes.

use crate::neuroevo::activation::ActivationFunction;
use crate::neuroevo::types::NodeId;

/// Type of node in the neural network
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// Input sensor
    Input,
    /// Hidden processing node
    Hidden,
    /// Output effector
    Output,
    /// Bias node (always outputs 1.0)
    Bias,
}

/// A node gene in the NEAT genome
#[derive(Debug, Clone)]
pub struct NodeGene {
    /// Unique node identifier
    pub id: NodeId,
    /// Type of this node
    pub node_type: NodeType,
    /// Activation function
    pub activation: ActivationFunction,
    /// Layer for feedforward ordering (-1 for input, 0+ for hidden/output)
    pub layer: i32,
    /// Position for substrate (x, y) in HyperNEAT
    pub position: (f64, f64),
    /// Bias value
    pub bias: f64,
    /// Response multiplier
    pub response: f64,
}

impl NodeGene {
    /// Create a new node gene
    pub fn new(id: NodeId, node_type: NodeType, activation: ActivationFunction) -> Self {
        Self {
            id,
            node_type,
            activation,
            layer: match node_type {
                NodeType::Input | NodeType::Bias => -1,
                NodeType::Output => 1,
                NodeType::Hidden => 0,
            },
            position: (0.0, 0.0),
            bias: 0.0,
            response: 1.0,
        }
    }

    /// Create an input node
    pub fn input(id: NodeId) -> Self {
        Self::new(id, NodeType::Input, ActivationFunction::Identity)
    }

    /// Create an output node
    pub fn output(id: NodeId, activation: ActivationFunction) -> Self {
        Self::new(id, NodeType::Output, activation)
    }

    /// Create a hidden node
    pub fn hidden(id: NodeId, activation: ActivationFunction) -> Self {
        Self::new(id, NodeType::Hidden, activation)
    }
}
