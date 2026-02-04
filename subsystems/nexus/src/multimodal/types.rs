//! Type definitions and constants for the multimodal fusion system.

// ============================================================================
// CONSTANTS
// ============================================================================

/// Maximum number of modalities
pub const MAX_MODALITIES: usize = 8;

/// Maximum modality dimension
pub const MAX_MODALITY_DIM: usize = 512;

/// Default attention heads
pub const DEFAULT_ATTENTION_HEADS: usize = 8;

/// Default hidden dimension
pub const DEFAULT_HIDDEN_DIM: usize = 256;

// ============================================================================
// MODALITY TYPES
// ============================================================================

/// Types of modalities in the kernel
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModalityType {
    /// System metrics (CPU, memory, etc.)
    Metrics,
    /// Log entries (text/encoded)
    Logs,
    /// Distributed traces
    Traces,
    /// System events
    Events,
    /// Network data
    Network,
    /// Filesystem operations
    Filesystem,
    /// Process information
    Process,
    /// Custom modality
    Custom,
}

impl ModalityType {
    /// Get modality name
    pub fn name(&self) -> &'static str {
        match self {
            ModalityType::Metrics => "metrics",
            ModalityType::Logs => "logs",
            ModalityType::Traces => "traces",
            ModalityType::Events => "events",
            ModalityType::Network => "network",
            ModalityType::Filesystem => "filesystem",
            ModalityType::Process => "process",
            ModalityType::Custom => "custom",
        }
    }
}

/// Strategy for handling missing modalities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingModalityStrategy {
    /// Use zero vector
    Zero,
    /// Use learned default
    LearnedDefault,
    /// Impute from other modalities
    Impute,
    /// Drop and reweight
    DropReweight,
}
