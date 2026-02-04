//! Memory subsystem types.

/// Memory access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute access
    Execute,
}

/// Memory access record
#[derive(Debug, Clone)]
pub struct AccessRecord {
    /// Memory address
    pub address: u64,
    /// Access type
    pub access_type: AccessType,
    /// Access size in bytes
    pub size: u64,
    /// Timestamp
    pub timestamp: u64,
}

/// Memory access pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPattern {
    /// Sequential access (forward)
    Sequential,
    /// Sequential access (reverse)
    ReverseSequential,
    /// Strided access
    Strided { stride: i64 },
    /// Temporal locality
    Temporal,
    /// Stack-like pattern
    Stack,
    /// Pointer chasing
    PointerChasing,
    /// Mixed patterns
    Mixed,
    /// Random access
    Random,
}

impl AccessPattern {
    /// Check if this pattern is prefetchable
    pub fn is_prefetchable(&self) -> bool {
        matches!(
            self,
            AccessPattern::Sequential
                | AccessPattern::ReverseSequential
                | AccessPattern::Strided { .. }
        )
    }
}
