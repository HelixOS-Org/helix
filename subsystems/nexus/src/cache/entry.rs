//! Cache entry representation.

use super::types::{CacheKey, CacheLineState};
use crate::core::NexusTimestamp;

// ============================================================================
// CACHE ENTRY
// ============================================================================

/// Cache entry with metadata
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Entry key
    pub key: CacheKey,
    /// Entry size in bytes
    pub size: u32,
    /// State
    pub state: CacheLineState,
    /// Access count
    pub access_count: u32,
    /// Last access time
    pub last_access: NexusTimestamp,
    /// First access time
    pub first_access: NexusTimestamp,
    /// Recency position (for LRU)
    pub recency: u64,
    /// Frequency score (for LFU/ARC)
    pub frequency: f64,
    /// Predicted next access
    pub predicted_next_access: Option<u64>,
}

impl CacheEntry {
    /// Create new entry
    pub fn new(key: CacheKey, size: u32) -> Self {
        let now = NexusTimestamp::now();
        Self {
            key,
            size,
            state: CacheLineState::Exclusive,
            access_count: 1,
            last_access: now,
            first_access: now,
            recency: 0,
            frequency: 1.0,
            predicted_next_access: None,
        }
    }

    /// Record access
    pub fn access(&mut self) {
        self.access_count += 1;
        self.last_access = NexusTimestamp::now();

        // Update frequency with decay
        let age = self.age_ticks();
        if age > 0 {
            self.frequency = self.access_count as f64 / (age as f64 / 1_000_000_000.0);
        }
    }

    /// Get age in ticks
    pub fn age_ticks(&self) -> u64 {
        NexusTimestamp::now().duration_since(self.first_access)
    }

    /// Get time since last access
    pub fn idle_ticks(&self) -> u64 {
        NexusTimestamp::now().duration_since(self.last_access)
    }

    /// Is entry hot (frequently accessed)?
    pub fn is_hot(&self) -> bool {
        self.frequency > 10.0 && self.idle_ticks() < 1_000_000_000
    }

    /// Is entry cold (rarely accessed)?
    pub fn is_cold(&self) -> bool {
        self.frequency < 1.0 || self.idle_ticks() > 10_000_000_000
    }
}
