//! Core NUMA types and definitions.

extern crate alloc;

// ============================================================================
// NUMA TYPES
// ============================================================================

/// NUMA node identifier
pub type NodeId = u32;

/// CPU identifier
pub type CpuId = u32;

/// NUMA distance type
pub type Distance = u8;

// ============================================================================
// MEMORY BINDING
// ============================================================================

/// Memory binding policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryBinding {
    /// Default (first touch)
    Default,
    /// Bind to specific node
    Bind(NodeId),
    /// Interleave across nodes
    Interleave,
    /// Prefer specific node
    Preferred(NodeId),
    /// Local allocation
    Local,
}
