//! Checkpoint management for rollback
//!
//! This module provides checkpoint creation, storage, and management
//! for micro-rollback and state recovery operations.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::core::{ComponentId, NexusTimestamp};
use crate::error::{HealingError, NexusResult};

/// A checkpoint for rollback
#[derive(Debug, Clone)]
pub struct Checkpoint {
    /// Unique checkpoint ID
    pub id: u64,
    /// Component this checkpoint is for
    pub component: ComponentId,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// State data
    pub state: Vec<u8>,
    /// State size
    pub size: usize,
    /// Is this checkpoint valid?
    pub valid: bool,
    /// Parent checkpoint (for incremental)
    pub parent: Option<u64>,
}

impl Checkpoint {
    /// Create a new checkpoint
    pub fn new(component: ComponentId, state: Vec<u8>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let size = state.len();
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            component,
            timestamp: NexusTimestamp::now(),
            state,
            size,
            valid: true,
            parent: None,
        }
    }

    /// Create an incremental checkpoint
    pub fn incremental(component: ComponentId, state: Vec<u8>, parent: u64) -> Self {
        let mut cp = Self::new(component, state);
        cp.parent = Some(parent);
        cp
    }

    /// Invalidate this checkpoint
    pub fn invalidate(&mut self) {
        self.valid = false;
    }
}

/// Checkpoint store statistics
#[derive(Debug, Clone)]
pub struct CheckpointStats {
    /// Total checkpoints
    pub total_checkpoints: usize,
    /// Valid checkpoints
    pub valid_checkpoints: usize,
    /// Total size in bytes
    pub total_size: usize,
    /// Number of components with checkpoints
    pub components: usize,
}

/// Store for checkpoints
pub struct CheckpointStore {
    /// Checkpoints by ID
    checkpoints: BTreeMap<u64, Checkpoint>,
    /// Latest checkpoint per component
    latest: BTreeMap<u64, u64>, // component_id -> checkpoint_id
    /// Maximum checkpoints to keep per component
    max_per_component: usize,
    /// Maximum total checkpoints
    max_total: usize,
    /// Total size in bytes
    total_size: usize,
    /// Maximum total size
    max_size: usize,
}

impl CheckpointStore {
    /// Create a new checkpoint store
    pub fn new(max_per_component: usize, max_total: usize, max_size: usize) -> Self {
        Self {
            checkpoints: BTreeMap::new(),
            latest: BTreeMap::new(),
            max_per_component,
            max_total,
            total_size: 0,
            max_size,
        }
    }

    /// Save a checkpoint
    pub fn save(&mut self, checkpoint: Checkpoint) -> NexusResult<u64> {
        let id = checkpoint.id;
        let component_id = checkpoint.component.raw();
        let size = checkpoint.size;

        // Check size limits
        if self.total_size + size > self.max_size {
            // Try to free space by removing old checkpoints
            self.garbage_collect();

            if self.total_size + size > self.max_size {
                return Err(HealingError::CheckpointCorrupted.into());
            }
        }

        // Update latest
        self.latest.insert(component_id, id);

        // Store checkpoint
        self.total_size += size;
        self.checkpoints.insert(id, checkpoint);

        // Check total limit
        while self.checkpoints.len() > self.max_total {
            self.remove_oldest();
        }

        Ok(id)
    }

    /// Get a checkpoint by ID
    pub fn get(&self, id: u64) -> Option<&Checkpoint> {
        self.checkpoints.get(&id).filter(|cp| cp.valid)
    }

    /// Get latest checkpoint for a component
    pub fn latest_for(&self, component: ComponentId) -> Option<&Checkpoint> {
        self.latest
            .get(&component.raw())
            .and_then(|id| self.get(*id))
    }

    /// Get checkpoint history for a component
    pub fn history_for(&self, component: ComponentId) -> Vec<&Checkpoint> {
        self.checkpoints
            .values()
            .filter(|cp| cp.component == component && cp.valid)
            .collect()
    }

    /// Remove a checkpoint
    pub fn remove(&mut self, id: u64) -> Option<Checkpoint> {
        if let Some(cp) = self.checkpoints.remove(&id) {
            self.total_size -= cp.size;

            // Update latest if needed
            if self.latest.get(&cp.component.raw()) == Some(&id) {
                // Find new latest
                let new_latest = self
                    .checkpoints
                    .values()
                    .filter(|c| c.component == cp.component && c.valid)
                    .max_by_key(|c| c.timestamp.ticks())
                    .map(|c| c.id);

                if let Some(new_id) = new_latest {
                    self.latest.insert(cp.component.raw(), new_id);
                } else {
                    self.latest.remove(&cp.component.raw());
                }
            }

            Some(cp)
        } else {
            None
        }
    }

    /// Remove oldest checkpoint
    fn remove_oldest(&mut self) {
        if let Some(oldest_id) = self.checkpoints.keys().next().copied() {
            self.remove(oldest_id);
        }
    }

    /// Garbage collect invalid checkpoints
    fn garbage_collect(&mut self) {
        let invalid_ids: Vec<_> = self
            .checkpoints
            .iter()
            .filter(|(_, cp)| !cp.valid)
            .map(|(id, _)| *id)
            .collect();

        for id in invalid_ids {
            self.remove(id);
        }
    }

    /// Get statistics
    pub fn stats(&self) -> CheckpointStats {
        CheckpointStats {
            total_checkpoints: self.checkpoints.len(),
            valid_checkpoints: self.checkpoints.values().filter(|cp| cp.valid).count(),
            total_size: self.total_size,
            components: self.latest.len(),
        }
    }
}
