//! # Cognitive Priority System
//!
//! Manages priorities for cognitive operations and decisions.
//! Implements priority queues and scheduling policies.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{ComponentId, DomainId, Timestamp};

// ============================================================================
// PRIORITY TYPES
// ============================================================================

/// Priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum PriorityLevel {
    /// Background - lowest priority
    Background = 0,
    /// Low priority
    Low        = 1,
    /// Normal priority
    Normal     = 2,
    /// Elevated priority
    Elevated   = 3,
    /// High priority
    High       = 4,
    /// Critical priority
    Critical   = 5,
    /// Emergency - highest priority
    Emergency  = 6,
}

impl PriorityLevel {
    /// Get numeric value
    #[inline(always)]
    pub fn value(&self) -> u8 {
        *self as u8
    }

    /// From numeric value
    #[inline]
    pub fn from_value(v: u8) -> Self {
        match v {
            0 => Self::Background,
            1 => Self::Low,
            2 => Self::Normal,
            3 => Self::Elevated,
            4 => Self::High,
            5 => Self::Critical,
            _ => Self::Emergency,
        }
    }

    /// Get weight multiplier
    #[inline]
    pub fn weight(&self) -> f32 {
        match self {
            Self::Background => 0.1,
            Self::Low => 0.5,
            Self::Normal => 1.0,
            Self::Elevated => 2.0,
            Self::High => 4.0,
            Self::Critical => 8.0,
            Self::Emergency => 16.0,
        }
    }
}

impl Default for PriorityLevel {
    fn default() -> Self {
        Self::Normal
    }
}

/// Prioritized item
#[derive(Debug, Clone)]
pub struct PrioritizedItem<T> {
    /// Item ID
    pub id: u64,
    /// Priority
    pub priority: Priority,
    /// Item data
    pub data: T,
    /// Creation timestamp
    pub created: Timestamp,
    /// Deadline (optional)
    pub deadline: Option<Timestamp>,
}

/// Full priority specification
#[derive(Debug, Clone)]
pub struct Priority {
    /// Base level
    pub level: PriorityLevel,
    /// Numeric priority within level (0-255)
    pub sublevel: u8,
    /// Boost factor
    pub boost: f32,
    /// Age factor (priority increases with age)
    pub age_factor: f32,
    /// Source domain
    pub source: Option<DomainId>,
}

impl Priority {
    /// Create a new priority
    pub fn new(level: PriorityLevel) -> Self {
        Self {
            level,
            sublevel: 128,
            boost: 1.0,
            age_factor: 0.0,
            source: None,
        }
    }

    /// With sublevel
    #[inline(always)]
    pub fn with_sublevel(mut self, sublevel: u8) -> Self {
        self.sublevel = sublevel;
        self
    }

    /// With boost
    #[inline(always)]
    pub fn with_boost(mut self, boost: f32) -> Self {
        self.boost = boost;
        self
    }

    /// With age factor
    #[inline(always)]
    pub fn with_age_factor(mut self, factor: f32) -> Self {
        self.age_factor = factor;
        self
    }

    /// Calculate effective priority
    #[inline]
    pub fn effective(&self, age_cycles: u64) -> f32 {
        let base = (self.level.value() as f32 * 256.0) + self.sublevel as f32;
        let age_boost = age_cycles as f32 * self.age_factor;
        (base + age_boost) * self.boost * self.level.weight()
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::new(PriorityLevel::Normal)
    }
}

// ============================================================================
// PRIORITY QUEUE
// ============================================================================

/// Priority queue for cognitive items
#[repr(align(64))]
pub struct PriorityQueue<T> {
    /// Items by priority level
    levels: BTreeMap<PriorityLevel, Vec<PrioritizedItem<T>>>,
    /// Next item ID
    next_id: AtomicU64,
    /// Current cycle
    current_cycle: u64,
    /// Configuration
    config: PriorityQueueConfig,
    /// Statistics
    stats: PriorityQueueStats,
}

/// Configuration
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PriorityQueueConfig {
    /// Maximum items per level
    pub max_per_level: usize,
    /// Maximum total items
    pub max_total: usize,
    /// Enable aging
    pub enable_aging: bool,
    /// Starvation prevention threshold (cycles)
    pub starvation_threshold: u64,
}

impl Default for PriorityQueueConfig {
    fn default() -> Self {
        Self {
            max_per_level: 1000,
            max_total: 5000,
            enable_aging: true,
            starvation_threshold: 100,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct PriorityQueueStats {
    /// Total items enqueued
    pub total_enqueued: u64,
    /// Total items dequeued
    pub total_dequeued: u64,
    /// Total items dropped
    pub total_dropped: u64,
    /// Starvation preventions
    pub starvation_preventions: u64,
    /// Average wait time (cycles)
    pub avg_wait_cycles: f32,
    /// Items per level
    pub items_per_level: BTreeMap<PriorityLevel, u32>,
}

impl<T: Clone> PriorityQueue<T> {
    /// Create a new priority queue
    pub fn new(config: PriorityQueueConfig) -> Self {
        let mut levels = BTreeMap::new();
        for level in [
            PriorityLevel::Background,
            PriorityLevel::Low,
            PriorityLevel::Normal,
            PriorityLevel::Elevated,
            PriorityLevel::High,
            PriorityLevel::Critical,
            PriorityLevel::Emergency,
        ] {
            levels.insert(level, Vec::new());
        }

        Self {
            levels,
            next_id: AtomicU64::new(1),
            current_cycle: 0,
            config,
            stats: PriorityQueueStats::default(),
        }
    }

    /// Enqueue an item
    pub fn enqueue(&mut self, data: T, priority: Priority) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let item = PrioritizedItem {
            id,
            priority: priority.clone(),
            data,
            created: Timestamp::now(),
            deadline: None,
        };

        // Check capacity
        let level = priority.level;
        let level_items = self.levels.entry(level).or_default();

        if level_items.len() >= self.config.max_per_level {
            // Drop lowest priority in this level
            if let Some(pos) = level_items
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| a.priority.sublevel.cmp(&b.priority.sublevel))
                .map(|(i, _)| i)
            {
                level_items.remove(pos);
                self.stats.total_dropped += 1;
            }
        }

        level_items.push(item);
        self.stats.total_enqueued += 1;
        self.update_level_stats();

        id
    }

    /// Enqueue with deadline
    pub fn enqueue_with_deadline(
        &mut self,
        data: T,
        priority: Priority,
        deadline: Timestamp,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let item = PrioritizedItem {
            id,
            priority: priority.clone(),
            data,
            created: Timestamp::now(),
            deadline: Some(deadline),
        };

        self.levels.entry(priority.level).or_default().push(item);
        self.stats.total_enqueued += 1;
        self.update_level_stats();

        id
    }

    /// Dequeue highest priority item
    pub fn dequeue(&mut self) -> Option<PrioritizedItem<T>> {
        // Check for emergency items first
        for level in [
            PriorityLevel::Emergency,
            PriorityLevel::Critical,
            PriorityLevel::High,
            PriorityLevel::Elevated,
            PriorityLevel::Normal,
            PriorityLevel::Low,
            PriorityLevel::Background,
        ] {
            if let Some(items) = self.levels.get_mut(&level) {
                if !items.is_empty() {
                    // Find highest effective priority
                    let best_idx = items
                        .iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| {
                            let age_a = self.current_cycle - a.created.as_cycles();
                            let age_b = self.current_cycle - b.created.as_cycles();
                            let eff_a = a.priority.effective(age_a);
                            let eff_b = b.priority.effective(age_b);
                            eff_a
                                .partial_cmp(&eff_b)
                                .unwrap_or(core::cmp::Ordering::Equal)
                        })
                        .map(|(i, _)| i);

                    if let Some(idx) = best_idx {
                        let item = items.remove(idx);

                        // Update stats
                        let wait_cycles = self.current_cycle - item.created.as_cycles();
                        self.stats.avg_wait_cycles = (self.stats.avg_wait_cycles
                            * self.stats.total_dequeued as f32
                            + wait_cycles as f32)
                            / (self.stats.total_dequeued + 1) as f32;

                        self.stats.total_dequeued += 1;
                        self.update_level_stats();
                        return Some(item);
                    }
                }
            }
        }

        // Starvation prevention - check low priority items
        if self.config.enable_aging {
            self.prevent_starvation();
        }

        None
    }

    /// Peek at highest priority item
    pub fn peek(&self) -> Option<&PrioritizedItem<T>> {
        for level in [
            PriorityLevel::Emergency,
            PriorityLevel::Critical,
            PriorityLevel::High,
            PriorityLevel::Elevated,
            PriorityLevel::Normal,
            PriorityLevel::Low,
            PriorityLevel::Background,
        ] {
            if let Some(items) = self.levels.get(&level) {
                if let Some(item) = items.last() {
                    return Some(item);
                }
            }
        }
        None
    }

    /// Prevent starvation of low priority items
    fn prevent_starvation(&mut self) {
        let threshold = self.config.starvation_threshold;

        for level in [PriorityLevel::Background, PriorityLevel::Low] {
            if let Some(items) = self.levels.get_mut(&level) {
                // Find starving items
                let starving: Vec<_> = items
                    .iter()
                    .filter(|item| self.current_cycle - item.created.as_cycles() > threshold)
                    .cloned()
                    .collect();

                for item in starving {
                    // Boost to higher priority
                    let new_level = PriorityLevel::from_value(level.value() + 1);
                    items.retain(|i| i.id != item.id);

                    let mut boosted = item;
                    boosted.priority.boost *= 2.0;

                    self.levels.entry(new_level).or_default().push(boosted);
                    self.stats.starvation_preventions += 1;
                }
            }
        }
    }

    /// Process cycle
    pub fn tick(&mut self) {
        self.current_cycle += 1;

        // Check deadlines
        let now = Timestamp::now();
        for items in self.levels.values_mut() {
            // Boost items near deadline
            for item in items.iter_mut() {
                if let Some(deadline) = item.deadline {
                    if now.raw() >= deadline.raw() {
                        // Past deadline - boost to emergency
                        item.priority.level = PriorityLevel::Emergency;
                        item.priority.boost *= 4.0;
                    } else {
                        let remaining = deadline.raw() - now.raw();
                        if remaining < 1_000_000 {
                            // Less than 1ms
                            item.priority.boost *= 2.0;
                        }
                    }
                }
            }
        }
    }

    /// Get queue length
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.levels.values().map(|v| v.len()).sum()
    }

    /// Check if empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.levels.values().all(|v| v.is_empty())
    }

    /// Get length at level
    #[inline(always)]
    pub fn len_at_level(&self, level: PriorityLevel) -> usize {
        self.levels.get(&level).map(|v| v.len()).unwrap_or(0)
    }

    /// Clear all items
    #[inline]
    pub fn clear(&mut self) {
        for items in self.levels.values_mut() {
            items.clear();
        }
    }

    /// Update level statistics
    fn update_level_stats(&mut self) {
        for (level, items) in &self.levels {
            self.stats
                .items_per_level
                .insert(*level, items.len() as u32);
        }
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &PriorityQueueStats {
        &self.stats
    }
}

// ============================================================================
// PRIORITY SCHEDULER
// ============================================================================

/// Schedules items based on priority
pub struct PriorityScheduler {
    /// Scheduling weights per level
    weights: BTreeMap<PriorityLevel, u32>,
    /// Current round-robin position
    rr_position: usize,
    /// Round-robin order
    rr_order: Vec<PriorityLevel>,
}

impl PriorityScheduler {
    /// Create a new scheduler
    pub fn new() -> Self {
        let mut weights = BTreeMap::new();
        weights.insert(PriorityLevel::Emergency, 1000);
        weights.insert(PriorityLevel::Critical, 500);
        weights.insert(PriorityLevel::High, 200);
        weights.insert(PriorityLevel::Elevated, 100);
        weights.insert(PriorityLevel::Normal, 50);
        weights.insert(PriorityLevel::Low, 20);
        weights.insert(PriorityLevel::Background, 5);

        Self {
            weights,
            rr_position: 0,
            rr_order: Vec::new(),
        }
    }

    /// Build round-robin order based on weights
    #[inline]
    pub fn build_order(&mut self) {
        self.rr_order.clear();

        for (&level, &weight) in &self.weights {
            for _ in 0..weight {
                self.rr_order.push(level);
            }
        }

        self.rr_position = 0;
    }

    /// Get next level to service
    #[inline]
    pub fn next_level(&mut self) -> PriorityLevel {
        if self.rr_order.is_empty() {
            self.build_order();
        }

        let level = self.rr_order[self.rr_position];
        self.rr_position = (self.rr_position + 1) % self.rr_order.len();
        level
    }

    /// Set weight for level
    #[inline(always)]
    pub fn set_weight(&mut self, level: PriorityLevel, weight: u32) {
        self.weights.insert(level, weight);
        self.build_order();
    }

    /// Get weight for level
    #[inline(always)]
    pub fn get_weight(&self, level: PriorityLevel) -> u32 {
        self.weights.get(&level).copied().unwrap_or(0)
    }
}

impl Default for PriorityScheduler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_level() {
        assert!(PriorityLevel::Emergency > PriorityLevel::Critical);
        assert!(PriorityLevel::Critical > PriorityLevel::High);
        assert!(PriorityLevel::High > PriorityLevel::Normal);
        assert!(PriorityLevel::Normal > PriorityLevel::Low);
        assert!(PriorityLevel::Low > PriorityLevel::Background);
    }

    #[test]
    fn test_effective_priority() {
        let p1 = Priority::new(PriorityLevel::Normal);
        let p2 = Priority::new(PriorityLevel::High);

        assert!(p2.effective(0) > p1.effective(0));
    }

    #[test]
    fn test_priority_queue() {
        let config = PriorityQueueConfig::default();
        let mut queue: PriorityQueue<i32> = PriorityQueue::new(config);

        queue.enqueue(1, Priority::new(PriorityLevel::Low));
        queue.enqueue(2, Priority::new(PriorityLevel::High));
        queue.enqueue(3, Priority::new(PriorityLevel::Normal));

        // Should dequeue high priority first
        let item = queue.dequeue().unwrap();
        assert_eq!(item.data, 2);
    }

    #[test]
    fn test_priority_scheduler() {
        let mut scheduler = PriorityScheduler::new();
        scheduler.build_order();

        // Emergency should appear more often
        let mut counts = BTreeMap::new();
        for _ in 0..1000 {
            let level = scheduler.next_level();
            *counts.entry(level).or_insert(0) += 1;
        }

        assert!(
            counts.get(&PriorityLevel::Emergency).unwrap_or(&0)
                > counts.get(&PriorityLevel::Background).unwrap_or(&0)
        );
    }
}
