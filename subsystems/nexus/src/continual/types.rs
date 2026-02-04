//! Core types for continual learning strategies.

/// Types of continual learning strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinualStrategy {
    /// Elastic Weight Consolidation
    EWC,
    /// Synaptic Intelligence
    SI,
    /// Learning without Forgetting
    LwF,
    /// Progressive Neural Networks
    Progressive,
    /// Experience Replay
    Replay,
    /// Memory Aware Synapses
    MAS,
    /// PackNet (pruning-based)
    PackNet,
    /// Gradient Episodic Memory
    GEM,
}
