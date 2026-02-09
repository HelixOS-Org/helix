//! Work Dependency Tracker
//!
//! This module provides dependency management with cycle detection for work items.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use super::WorkId;

/// Dependency type between work items
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// Hard dependency (must complete before)
    Hard,
    /// Soft dependency (prefer to complete before)
    Soft,
    /// Order dependency (same queue ordering)
    Order,
    /// Resource dependency (needs resource)
    Resource,
}

/// Work dependency edge
#[derive(Debug, Clone)]
pub struct WorkDependency {
    /// Source work item (must complete first)
    pub source: WorkId,
    /// Target work item (depends on source)
    pub target: WorkId,
    /// Dependency type
    pub dep_type: DependencyType,
    /// Is dependency satisfied
    pub satisfied: bool,
}

/// Work dependency tracker with cycle detection
#[repr(align(64))]
pub struct WorkDependencyTracker {
    /// All dependencies
    dependencies: Vec<WorkDependency>,
    /// Incoming edges per work item
    incoming: BTreeMap<WorkId, Vec<usize>>,
    /// Outgoing edges per work item
    outgoing: BTreeMap<WorkId, Vec<usize>>,
    /// Work items currently blocked
    blocked_works: BTreeMap<WorkId, u32>,
    /// Detected cycles
    cycles_detected: u64,
}

impl WorkDependencyTracker {
    /// Create new dependency tracker
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
            incoming: BTreeMap::new(),
            outgoing: BTreeMap::new(),
            blocked_works: BTreeMap::new(),
            cycles_detected: 0,
        }
    }

    /// Add dependency between work items
    pub fn add_dependency(
        &mut self,
        source: WorkId,
        target: WorkId,
        dep_type: DependencyType,
    ) -> bool {
        // Check for cycle
        if self.would_create_cycle(source, target) {
            self.cycles_detected += 1;
            return false;
        }

        let dep = WorkDependency {
            source,
            target,
            dep_type,
            satisfied: false,
        };

        let idx = self.dependencies.len();
        self.dependencies.push(dep);

        self.incoming.entry(target).or_default().push(idx);
        self.outgoing.entry(source).or_default().push(idx);

        // Update blocked count
        *self.blocked_works.entry(target).or_default() += 1;

        true
    }

    /// Check if adding edge would create cycle (DFS)
    fn would_create_cycle(&self, source: WorkId, target: WorkId) -> bool {
        if source == target {
            return true;
        }

        let mut visited = BTreeMap::new();
        let mut stack = vec![source];

        while let Some(current) = stack.pop() {
            if current == target {
                return true;
            }

            if visited.get(&current).copied().unwrap_or(false) {
                continue;
            }
            visited.insert(current, true);

            if let Some(edges) = self.outgoing.get(&current) {
                for &idx in edges {
                    if let Some(dep) = self.dependencies.get(idx) {
                        stack.push(dep.target);
                    }
                }
            }
        }

        false
    }

    /// Mark dependency as satisfied
    pub fn satisfy_dependency(&mut self, source: WorkId, target: WorkId) {
        if let Some(edges) = self.outgoing.get(&source) {
            for &idx in edges {
                if let Some(dep) = self.dependencies.get_mut(idx) {
                    if dep.target == target && !dep.satisfied {
                        dep.satisfied = true;
                        if let Some(count) = self.blocked_works.get_mut(&target) {
                            *count = count.saturating_sub(1);
                        }
                    }
                }
            }
        }
    }

    /// Mark work as completed (satisfy all outgoing)
    pub fn mark_completed(&mut self, work_id: WorkId) {
        if let Some(edges) = self.outgoing.get(&work_id).cloned() {
            for idx in edges {
                if let Some(dep) = self.dependencies.get_mut(idx) {
                    if !dep.satisfied {
                        dep.satisfied = true;
                        if let Some(count) = self.blocked_works.get_mut(&dep.target) {
                            *count = count.saturating_sub(1);
                        }
                    }
                }
            }
        }
    }

    /// Check if work is ready to run
    #[inline(always)]
    pub fn is_ready(&self, work_id: WorkId) -> bool {
        self.blocked_works.get(&work_id).copied().unwrap_or(0) == 0
    }

    /// Get pending dependencies for work
    pub fn get_pending_dependencies(&self, work_id: WorkId) -> Vec<WorkId> {
        let mut pending = Vec::new();

        if let Some(edges) = self.incoming.get(&work_id) {
            for &idx in edges {
                if let Some(dep) = self.dependencies.get(idx) {
                    if !dep.satisfied {
                        pending.push(dep.source);
                    }
                }
            }
        }

        pending
    }

    /// Get work items that depend on this work
    pub fn get_dependents(&self, work_id: WorkId) -> Vec<WorkId> {
        let mut dependents = Vec::new();

        if let Some(edges) = self.outgoing.get(&work_id) {
            for &idx in edges {
                if let Some(dep) = self.dependencies.get(idx) {
                    dependents.push(dep.target);
                }
            }
        }

        dependents
    }

    /// Get count of cycles detected
    #[inline(always)]
    pub fn cycles_detected(&self) -> u64 {
        self.cycles_detected
    }

    /// Get count of blocked works
    #[inline(always)]
    pub fn blocked_count(&self) -> usize {
        self.blocked_works.values().filter(|&&c| c > 0).count()
    }

    /// Clear completed dependencies
    pub fn cleanup_completed(&mut self) {
        // Remove satisfied dependencies
        let to_remove: Vec<usize> = self
            .dependencies
            .iter()
            .enumerate()
            .filter(|(_, d)| d.satisfied)
            .map(|(i, _)| i)
            .collect();

        // Remove in reverse order to maintain indices
        for idx in to_remove.into_iter().rev() {
            let dep = self.dependencies.remove(idx);

            // Update incoming
            if let Some(edges) = self.incoming.get_mut(&dep.target) {
                edges.retain(|&i| i != idx);
            }

            // Update outgoing
            if let Some(edges) = self.outgoing.get_mut(&dep.source) {
                edges.retain(|&i| i != idx);
            }
        }

        // Clean up blocked_works
        self.blocked_works.retain(|_, &mut c| c > 0);
    }
}

impl Default for WorkDependencyTracker {
    fn default() -> Self {
        Self::new()
    }
}
