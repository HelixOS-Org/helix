//! Performance Monitoring Core Types
//!
//! Fundamental type definitions for performance monitoring.

// ============================================================================
// CORE IDENTIFIERS
// ============================================================================

/// CPU ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CpuId(pub u32);

impl CpuId {
    /// Create new CPU ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Event ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u64);

impl EventId {
    /// Create new event ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// PMU ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PmuId(pub u32);

impl PmuId {
    /// Create new PMU ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}
