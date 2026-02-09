//! Lock Order Optimizer
//!
//! Optimizes lock acquisition order to prevent deadlocks.

#![allow(clippy::excessive_nesting)]

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::{LockId, ThreadId};
use crate::core::NexusTimestamp;

/// Order violation
#[derive(Debug, Clone)]
pub struct OrderViolation {
    /// Thread
    pub thread: ThreadId,
    /// Lock acquired out of order
    pub lock: LockId,
    /// Expected before
    pub expected_before: Vec<LockId>,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Optimizes lock acquisition order
pub struct LockOrderOptimizer {
    /// Lock dependencies
    dependencies: BTreeMap<LockId, Vec<LockId>>,
    /// Recommended order
    order: Vec<LockId>,
    /// Order violations
    violations: Vec<OrderViolation>,
    /// Lock levels
    levels: BTreeMap<LockId, u32>,
}

impl LockOrderOptimizer {
    /// Create new optimizer
    pub fn new() -> Self {
        Self {
            dependencies: BTreeMap::new(),
            order: Vec::new(),
            violations: Vec::new(),
            levels: BTreeMap::new(),
        }
    }

    /// Record acquisition order
    pub fn record_order(&mut self, lock: LockId, held_locks: &[LockId]) {
        // Lock was acquired while holding held_locks
        // So held_locks should come before lock
        for &held in held_locks {
            let deps = self.dependencies.entry(lock).or_default();
            if !deps.contains(&held) {
                deps.push(held);
            }
        }

        self.rebuild_order();
    }

    /// Rebuild recommended order
    fn rebuild_order(&mut self) {
        // Simple topological sort
        let mut in_degree: BTreeMap<LockId, usize> = BTreeMap::new();
        let mut graph: BTreeMap<LockId, Vec<LockId>> = BTreeMap::new();

        // Build reverse graph
        for (&lock, deps) in &self.dependencies {
            in_degree.entry(lock).or_insert(0);
            for &dep in deps {
                graph.entry(dep).or_default().push(lock);
                *in_degree.entry(lock).or_insert(0) += 1;
                in_degree.entry(dep).or_insert(0);
            }
        }

        // Kahn's algorithm
        let mut queue: Vec<_> = in_degree
            .iter()
            .filter(|&(_, d)| *d == 0)
            .map(|(&id, _)| id)
            .collect();
        let mut result = Vec::new();
        let mut level = 0u32;

        while !queue.is_empty() {
            let mut next_queue = Vec::new();

            for id in queue {
                result.push(id);
                self.levels.insert(id, level);

                if let Some(neighbors) = graph.get(&id) {
                    for &neighbor in neighbors {
                        if let Some(degree) = in_degree.get_mut(&neighbor) {
                            *degree -= 1;
                            if *degree == 0 {
                                next_queue.push(neighbor);
                            }
                        }
                    }
                }
            }

            queue = next_queue;
            level += 1;
        }

        self.order = result;
    }

    /// Check order
    pub fn check_order(
        &mut self,
        thread: ThreadId,
        lock: LockId,
        held_locks: &[LockId],
    ) -> Option<OrderViolation> {
        let lock_level = self.levels.get(&lock).copied().unwrap_or(u32::MAX);

        let violations: Vec<_> = held_locks
            .iter()
            .filter(|&&held| {
                let held_level = self.levels.get(&held).copied().unwrap_or(0);
                held_level > lock_level
            })
            .copied()
            .collect();

        if !violations.is_empty() {
            let violation = OrderViolation {
                thread,
                lock,
                expected_before: violations,
                timestamp: NexusTimestamp::now(),
            };
            self.violations.push(violation.clone());
            Some(violation)
        } else {
            None
        }
    }

    /// Get recommended order
    #[inline(always)]
    pub fn recommended_order(&self) -> &[LockId] {
        &self.order
    }

    /// Get lock level
    #[inline(always)]
    pub fn get_level(&self, lock: LockId) -> Option<u32> {
        self.levels.get(&lock).copied()
    }

    /// Get violations
    #[inline(always)]
    pub fn violations(&self) -> &[OrderViolation] {
        &self.violations
    }
}

impl Default for LockOrderOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
