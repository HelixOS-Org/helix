//! Kernel federated learning manager.

use alloc::vec::Vec;

use crate::federated::byzantine::{ByzantineDefense, ByzantineRobustAggregator};
use crate::federated::model::FederatedModel;
use crate::federated::privacy::DPFedAvgAggregator;
use crate::federated::types::{DEFAULT_CLIP_BOUND, DEFAULT_NOISE_MULTIPLIER, MAX_CLIENTS};
use crate::federated::update::ModelUpdate;

/// Kernel FL node type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelNodeRole {
    /// Aggregation server
    Server,
    /// Training client
    Client,
    /// Coordinator
    Coordinator,
}

/// Kernel federated learning manager
pub struct KernelFederatedManager {
    /// Node role
    pub role: KernelNodeRole,
    /// Current model
    pub model: FederatedModel,
    /// Aggregator (if server)
    pub aggregator: Option<DPFedAvgAggregator>,
    /// Byzantine defense
    pub byzantine_defense: Option<ByzantineRobustAggregator>,
    /// Connected nodes
    pub connected_nodes: Vec<u32>,
    /// Training rounds completed
    pub rounds_completed: u64,
    /// Client updates received
    pub updates_received: usize,
    /// Is training active?
    pub active: bool,
}

impl KernelFederatedManager {
    /// Create a new kernel FL manager
    pub fn new(role: KernelNodeRole, model_layers: &[(usize, usize)]) -> Self {
        let model = FederatedModel::from_layers(model_layers, 12345);

        let aggregator = if role == KernelNodeRole::Server {
            Some(DPFedAvgAggregator::new(
                model.clone(),
                DEFAULT_NOISE_MULTIPLIER,
                DEFAULT_CLIP_BOUND,
            ))
        } else {
            None
        };

        Self {
            role,
            model,
            aggregator,
            byzantine_defense: None,
            connected_nodes: Vec::new(),
            rounds_completed: 0,
            updates_received: 0,
            active: true,
        }
    }

    /// Enable Byzantine defense
    pub fn enable_byzantine_defense(&mut self, defense: ByzantineDefense) {
        let defender = ByzantineRobustAggregator::new(self.model.clone(), defense);
        self.byzantine_defense = Some(defender);
    }

    /// Register a client node
    pub fn register_node(&mut self, node_id: u32) -> bool {
        if self.connected_nodes.len() >= MAX_CLIENTS {
            return false;
        }

        if !self.connected_nodes.contains(&node_id) {
            self.connected_nodes.push(node_id);
        }

        true
    }

    /// Submit update (server role)
    pub fn submit_client_update(&mut self, update: ModelUpdate) -> bool {
        if self.role != KernelNodeRole::Server {
            return false;
        }

        self.updates_received += 1;

        if let Some(ref mut aggregator) = self.aggregator {
            aggregator.submit_update(update);
            true
        } else {
            false
        }
    }

    /// Aggregate updates (server role)
    pub fn aggregate(&mut self) -> bool {
        if self.role != KernelNodeRole::Server {
            return false;
        }

        // Use Byzantine defense if enabled
        if let Some(ref mut defender) = self.byzantine_defense {
            if let Some(ref aggregator) = self.aggregator {
                for update in &aggregator.base.pending_updates {
                    defender.submit_update(update.clone());
                }
            }

            if defender.aggregate() {
                self.model = defender.global_model.clone();
                self.rounds_completed += 1;
                return true;
            }
        } else if let Some(ref mut aggregator) = self.aggregator {
            if aggregator.aggregate() {
                self.model = aggregator.base.global_model.clone();
                self.rounds_completed += 1;
                return true;
            }
        }

        false
    }

    /// Get global model
    pub fn get_global_model(&self) -> &FederatedModel {
        &self.model
    }

    /// Get FL statistics
    pub fn get_stats(&self) -> FederatedStats {
        FederatedStats {
            role: self.role,
            rounds_completed: self.rounds_completed,
            connected_nodes: self.connected_nodes.len(),
            updates_received: self.updates_received,
            model_version: self.model.version,
            privacy_remaining: self
                .aggregator
                .as_ref()
                .map(|a| a.dp.remaining_budget())
                .unwrap_or(1.0),
        }
    }
}

/// Federated learning statistics
#[derive(Debug, Clone)]
pub struct FederatedStats {
    /// Node role
    pub role: KernelNodeRole,
    /// Rounds completed
    pub rounds_completed: u64,
    /// Connected nodes
    pub connected_nodes: usize,
    /// Updates received
    pub updates_received: usize,
    /// Model version
    pub model_version: u64,
    /// Remaining privacy budget
    pub privacy_remaining: f64,
}
