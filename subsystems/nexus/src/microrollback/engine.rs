//! Micro-rollback engine.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::entry::RollbackEntry;
use super::point::RollbackPoint;
use super::policy::RollbackPolicy;
use crate::core::{ComponentId, NexusTimestamp};
use crate::error::{HealingError, NexusResult};

/// The micro-rollback engine
pub struct MicroRollbackEngine {
    /// Rollback points by component
    pub(crate) points: BTreeMap<u64, Vec<RollbackPoint>>,
    /// Rollback history
    history: Vec<RollbackEntry>,
    /// Maximum history entries
    max_history: usize,
    /// Policy
    pub(crate) policy: RollbackPolicy,
    /// Total rollbacks performed
    total_rollbacks: AtomicU64,
    /// Successful rollbacks
    successful_rollbacks: AtomicU64,
}

impl MicroRollbackEngine {
    /// Create a new micro-rollback engine
    pub fn new(policy: RollbackPolicy) -> Self {
        Self {
            points: BTreeMap::new(),
            history: Vec::new(),
            max_history: 1000,
            policy,
            total_rollbacks: AtomicU64::new(0),
            successful_rollbacks: AtomicU64::new(0),
        }
    }

    /// Create a rollback point
    pub fn create_point(
        &mut self,
        component: ComponentId,
        checkpoint_id: u64,
        state_hash: u64,
    ) -> u64 {
        let point = RollbackPoint::new(component, checkpoint_id).with_hash(state_hash);

        let point_id = point.id;
        let comp_id = component.raw();

        // Add to points
        let points = self.points.entry(comp_id).or_default();
        points.push(point);

        // Enforce max points
        while points.len() > self.policy.max_points {
            points.remove(0);
        }

        point_id
    }

    /// Create a rollback point with dependencies
    pub fn create_point_with_deps(
        &mut self,
        component: ComponentId,
        checkpoint_id: u64,
        state_hash: u64,
        dependencies: Vec<ComponentId>,
    ) -> u64 {
        let mut point = RollbackPoint::new(component, checkpoint_id).with_hash(state_hash);
        point.dependencies = dependencies;

        let point_id = point.id;
        let comp_id = component.raw();

        let points = self.points.entry(comp_id).or_default();
        points.push(point);

        while points.len() > self.policy.max_points {
            points.remove(0);
        }

        point_id
    }

    /// Get latest rollback point for a component
    pub fn latest_point(&self, component: ComponentId) -> Option<&RollbackPoint> {
        self.points
            .get(&component.raw())
            .and_then(|points| points.last())
            .filter(|p| self.policy.allow_unsafe || p.is_safe)
    }

    /// Get rollback point by ID
    pub fn get_point(&self, point_id: u64) -> Option<&RollbackPoint> {
        for points in self.points.values() {
            if let Some(point) = points.iter().find(|p| p.id == point_id) {
                return Some(point);
            }
        }
        None
    }

    /// Get all rollback points for a component
    pub fn points_for(&self, component: ComponentId) -> &[RollbackPoint] {
        self.points
            .get(&component.raw())
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Perform a rollback
    pub fn rollback(
        &mut self,
        component: ComponentId,
        point_id: Option<u64>,
    ) -> NexusResult<RollbackEntry> {
        // Find the rollback point
        let point = if let Some(id) = point_id {
            self.get_point(id)
        } else {
            self.latest_point(component)
        };

        let point = point.ok_or(HealingError::NoCheckpoint)?;

        // Check if safe
        if !point.is_safe && !self.policy.allow_unsafe {
            return Err(HealingError::CheckpointCorrupted.into());
        }

        let point_id = point.id;
        let _checkpoint_id = point.checkpoint_id;
        let _dependencies = point.dependencies.clone();

        self.total_rollbacks.fetch_add(1, Ordering::Relaxed);

        // Create entry
        let mut entry = RollbackEntry {
            id: point_id,
            component,
            rollback_point: point_id,
            started: NexusTimestamp::now(),
            ended: None,
            success: false,
            error: None,
            pre_rollback_state: None,
        };

        // Perform the actual rollback
        // In real implementation, this would:
        // 1. Get current state (for undo)
        // 2. Load checkpoint
        // 3. Restore state
        // 4. Verify if policy.verify_after

        // For now, simulate success
        entry.success = true;
        entry.ended = Some(NexusTimestamp::now());

        if entry.success {
            self.successful_rollbacks.fetch_add(1, Ordering::Relaxed);
        }

        // Add to history
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(entry.clone());

        // Cleanup points after this one
        if let Some(points) = self.points.get_mut(&component.raw()) {
            points.retain(|p| p.id <= point_id);
        }

        Ok(entry)
    }

    /// Rollback to a specific timestamp
    pub fn rollback_to_time(
        &mut self,
        component: ComponentId,
        timestamp: NexusTimestamp,
    ) -> NexusResult<RollbackEntry> {
        // Find the latest point before the timestamp
        let point_id = self.points.get(&component.raw()).and_then(|points| {
            points
                .iter()
                .filter(|p| p.timestamp.ticks() <= timestamp.ticks())
                .filter(|p| self.policy.allow_unsafe || p.is_safe)
                .max_by_key(|p| p.timestamp.ticks())
                .map(|p| p.id)
        });

        match point_id {
            Some(id) => self.rollback(component, Some(id)),
            None => Err(HealingError::NoCheckpoint.into()),
        }
    }

    /// Cascade rollback to dependent components
    pub fn cascade_rollback(
        &mut self,
        component: ComponentId,
        point_id: u64,
    ) -> NexusResult<Vec<RollbackEntry>> {
        let mut entries = Vec::new();

        // First, rollback the main component
        let entry = self.rollback(component, Some(point_id))?;
        let target_time = entry.started;
        entries.push(entry);

        // Find components that depend on this one
        // and rollback them to the same time
        if self.policy.cascade_on_failure {
            let deps: Vec<ComponentId> = if let Some(point) = self.get_point(point_id) {
                point.dependencies.clone()
            } else {
                Vec::new()
            };
            for dep in deps {
                if let Ok(dep_entry) = self.rollback_to_time(dep, target_time) {
                    entries.push(dep_entry);
                }
            }
        }

        Ok(entries)
    }

    /// Invalidate rollback point
    pub fn invalidate_point(&mut self, point_id: u64) {
        for points in self.points.values_mut() {
            if let Some(point) = points.iter_mut().find(|p| p.id == point_id) {
                point.mark_unsafe();
            }
        }
    }

    /// Cleanup old rollback points
    pub fn cleanup(&mut self) {
        let now = NexusTimestamp::now();
        let max_age = self.policy.max_age;

        for points in self.points.values_mut() {
            points.retain(|p| now.duration_since(p.timestamp) < max_age);
        }
    }

    /// Get rollback history
    pub fn history(&self) -> &[RollbackEntry] {
        &self.history
    }

    /// Get statistics
    pub fn stats(&self) -> MicroRollbackStats {
        let total = self.total_rollbacks.load(Ordering::Relaxed);
        let successful = self.successful_rollbacks.load(Ordering::Relaxed);

        MicroRollbackStats {
            total_rollbacks: total,
            successful_rollbacks: successful,
            failed_rollbacks: total - successful,
            total_points: self.points.values().map(|v| v.len()).sum(),
            components_with_points: self.points.len(),
        }
    }

    /// Get policy
    pub fn policy(&self) -> &RollbackPolicy {
        &self.policy
    }

    /// Set policy
    pub fn set_policy(&mut self, policy: RollbackPolicy) {
        self.policy = policy;
    }
}

impl Default for MicroRollbackEngine {
    fn default() -> Self {
        Self::new(RollbackPolicy::default())
    }
}

/// Micro-rollback statistics
#[derive(Debug, Clone)]
pub struct MicroRollbackStats {
    /// Total rollbacks attempted
    pub total_rollbacks: u64,
    /// Successful rollbacks
    pub successful_rollbacks: u64,
    /// Failed rollbacks
    pub failed_rollbacks: u64,
    /// Total rollback points stored
    pub total_points: usize,
    /// Components with active rollback points
    pub components_with_points: usize,
}
