//! Core event types

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// EVENT ID
// ============================================================================

/// Unique event identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EventId(u64);

impl EventId {
    /// Generate a new unique event ID
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }

    /// Get raw ID
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// EVENT PRIORITY
// ============================================================================

/// Event priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EventPriority {
    /// Background processing
    Background = 0,
    /// Low priority
    Low        = 1,
    /// Normal priority
    Normal     = 2,
    /// High priority
    High       = 3,
    /// Critical - must be processed immediately
    Critical   = 4,
    /// Emergency - system-threatening
    Emergency  = 5,
}

impl Default for EventPriority {
    fn default() -> Self {
        Self::Normal
    }
}

// ============================================================================
// ANOMALY EVENT KIND
// ============================================================================

/// Kind of anomaly for AnomalyDetected event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnomalyEventKind {
    /// Statistical outlier
    StatisticalOutlier,
    /// Unexpected pattern
    UnexpectedPattern,
    /// Timing anomaly
    TimingAnomaly,
    /// Sequence anomaly
    SequenceAnomaly,
    /// Resource anomaly
    ResourceAnomaly,
    /// Security anomaly
    SecurityAnomaly,
}
