//! Rollback point definitions.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::{ComponentId, NexusTimestamp};

/// A point in time that can be rolled back to
#[derive(Debug, Clone)]
pub struct RollbackPoint {
    /// Unique ID
    pub id: u64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Component
    pub component: ComponentId,
    /// Checkpoint ID
    pub checkpoint_id: u64,
    /// State hash for verification
    pub state_hash: u64,
    /// Dependencies at this point
    pub dependencies: Vec<ComponentId>,
    /// Is this a safe point?
    pub is_safe: bool,
}

impl RollbackPoint {
    /// Create a new rollback point
    pub fn new(component: ComponentId, checkpoint_id: u64) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: NexusTimestamp::now(),
            component,
            checkpoint_id,
            state_hash: 0,
            dependencies: Vec::new(),
            is_safe: true,
        }
    }

    /// Set state hash
    #[inline(always)]
    pub fn with_hash(mut self, hash: u64) -> Self {
        self.state_hash = hash;
        self
    }

    /// Add a dependency
    #[inline(always)]
    pub fn with_dependency(mut self, dep: ComponentId) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Mark as unsafe
    #[inline(always)]
    pub fn mark_unsafe(&mut self) {
        self.is_safe = false;
    }
}
