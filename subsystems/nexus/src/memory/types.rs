//! Core memory access types.

// ============================================================================
// ACCESS PATTERNS
// ============================================================================

/// Memory access pattern types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPattern {
    /// Sequential forward access
    Sequential,
    /// Sequential backward access
    ReverseSequential,
    /// Strided access with fixed stride
    Strided { stride: i64 },
    /// Random/unpredictable access
    Random,
    /// Temporal locality (repeated accesses)
    Temporal,
    /// Stack-like (LIFO) pattern
    Stack,
    /// Pointer-chasing pattern
    PointerChasing,
    /// Mixed/unknown pattern
    Mixed,
}

impl AccessPattern {
    /// Is this pattern prefetchable?
    pub fn is_prefetchable(&self) -> bool {
        matches!(
            self,
            Self::Sequential | Self::ReverseSequential | Self::Strided { .. }
        )
    }

    /// Recommended prefetch distance
    pub fn prefetch_distance(&self) -> usize {
        match self {
            Self::Sequential => 8,
            Self::ReverseSequential => 8,
            Self::Strided { stride } => (8 * stride.unsigned_abs() as usize).min(64),
            Self::Temporal => 0,
            Self::Stack => 2,
            Self::PointerChasing => 1,
            _ => 0,
        }
    }
}

/// Memory access record
#[derive(Debug, Clone, Copy)]
pub struct AccessRecord {
    /// Address accessed
    pub address: u64,
    /// Access type
    pub access_type: AccessType,
    /// Size of access
    pub size: u32,
    /// Timestamp
    pub timestamp: u64,
}

/// Type of memory access
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute (instruction fetch)
    Execute,
    /// Prefetch
    Prefetch,
}
