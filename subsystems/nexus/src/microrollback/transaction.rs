//! Transactional rollback support.

use alloc::vec::Vec;

use super::engine::MicroRollbackEngine;
use super::entry::RollbackEntry;
use crate::core::ComponentId;
use crate::error::NexusResult;

/// A transaction that can be rolled back atomically
pub struct RollbackTransaction<'a> {
    engine: &'a mut MicroRollbackEngine,
    components: Vec<ComponentId>,
    checkpoints: Vec<(ComponentId, u64)>,
    committed: bool,
}

impl<'a> RollbackTransaction<'a> {
    /// Create a new transaction
    pub fn new(engine: &'a mut MicroRollbackEngine) -> Self {
        Self {
            engine,
            components: Vec::new(),
            checkpoints: Vec::new(),
            committed: false,
        }
    }

    /// Add a component to the transaction
    #[inline]
    pub fn add_component(&mut self, component: ComponentId, checkpoint_id: u64, state_hash: u64) {
        let point_id = self
            .engine
            .create_point(component, checkpoint_id, state_hash);
        self.components.push(component);
        self.checkpoints.push((component, point_id));
    }

    /// Commit the transaction (make rollback points permanent)
    #[inline(always)]
    pub fn commit(mut self) {
        self.committed = true;
    }

    /// Rollback all components
    #[inline]
    pub fn rollback(self) -> NexusResult<Vec<RollbackEntry>> {
        let mut entries = Vec::new();

        for (component, point_id) in &self.checkpoints {
            if let Ok(entry) = self.engine.rollback(*component, Some(*point_id)) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }
}

impl Drop for RollbackTransaction<'_> {
    fn drop(&mut self) {
        // If not committed, invalidate all checkpoints
        if !self.committed {
            for (_, point_id) in &self.checkpoints {
                self.engine.invalidate_point(*point_id);
            }
        }
    }
}
