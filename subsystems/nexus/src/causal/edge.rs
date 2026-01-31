//! Causal edge definitions

#![allow(dead_code)]

// ============================================================================
// CAUSAL EDGE TYPE
// ============================================================================

/// Type of causal edge
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CausalEdgeType {
    /// Sequential (same thread)
    Sequential,
    /// Message passing
    Message,
    /// Fork (spawned thread/task)
    Fork,
    /// Join (wait for thread/task)
    Join,
    /// Signal
    Signal,
    /// Lock dependency
    Lock,
    /// Data dependency
    Data,
}

// ============================================================================
// CAUSAL EDGE
// ============================================================================

/// An edge in the causal graph (represents causality)
#[derive(Debug, Clone)]
pub struct CausalEdge {
    /// Source node ID
    pub from: u64,
    /// Target node ID
    pub to: u64,
    /// Edge type
    pub edge_type: CausalEdgeType,
    /// Latency between events (cycles)
    pub latency: u64,
    /// Edge weight (for critical path analysis)
    pub weight: f64,
}

impl CausalEdge {
    /// Create a new edge
    pub fn new(from: u64, to: u64, edge_type: CausalEdgeType) -> Self {
        Self {
            from,
            to,
            edge_type,
            latency: 0,
            weight: 1.0,
        }
    }

    /// Set latency
    pub fn with_latency(mut self, latency: u64) -> Self {
        self.latency = latency;
        self.weight = latency as f64;
        self
    }

    /// Set weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight;
        self
    }
}
