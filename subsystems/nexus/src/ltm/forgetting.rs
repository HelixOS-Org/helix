//! # Memory Forgetting
//!
//! Implements forgetting curves and memory decay.
//! Manages memory cleanup and consolidation.
//!
//! Part of Year 2 COGNITION - Q3: Long-Term Memory

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// FORGETTING TYPES
// ============================================================================

/// Memory item
#[derive(Debug, Clone)]
pub struct MemoryItem {
    /// Item ID
    pub id: u64,
    /// Key
    pub key: String,
    /// Strength
    pub strength: f64,
    /// Importance
    pub importance: f64,
    /// Access count
    pub access_count: u64,
    /// Created
    pub created: Timestamp,
    /// Last accessed
    pub last_accessed: Timestamp,
    /// Last rehearsed
    pub last_rehearsed: Timestamp,
    /// Tags
    pub tags: Vec<String>,
}

/// Forgetting curve type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgettingCurve {
    /// Ebbinghaus exponential decay
    Ebbinghaus,
    /// Power law
    PowerLaw,
    /// Linear decay
    Linear,
    /// Step function
    Step,
    /// Custom
    Custom,
}

/// Forgetting event
#[derive(Debug, Clone)]
pub struct ForgettingEvent {
    /// Event ID
    pub id: u64,
    /// Item ID
    pub item_id: u64,
    /// Previous strength
    pub previous_strength: f64,
    /// New strength
    pub new_strength: f64,
    /// Reason
    pub reason: ForgettingReason,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Forgetting reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgettingReason {
    TimeDecay,
    Interference,
    Consolidation,
    Capacity,
    Manual,
}

/// Rehearsal record
#[derive(Debug, Clone)]
pub struct RehearsalRecord {
    /// Item ID
    pub item_id: u64,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Strength boost
    pub boost: f64,
    /// Type
    pub rehearsal_type: RehearsalType,
}

/// Rehearsal type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RehearsalType {
    Active,
    Passive,
    Spaced,
}

// ============================================================================
// FORGETTING MANAGER
// ============================================================================

/// Forgetting manager
pub struct ForgettingManager {
    /// Items
    items: BTreeMap<u64, MemoryItem>,
    /// Events
    events: Vec<ForgettingEvent>,
    /// Rehearsals
    rehearsals: Vec<RehearsalRecord>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: ForgettingConfig,
    /// Statistics
    stats: ForgettingStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct ForgettingConfig {
    /// Forgetting curve
    pub curve: ForgettingCurve,
    /// Decay rate
    pub decay_rate: f64,
    /// Minimum strength
    pub min_strength: f64,
    /// Capacity
    pub capacity: usize,
    /// Rehearsal boost
    pub rehearsal_boost: f64,
    /// Importance weight
    pub importance_weight: f64,
}

impl Default for ForgettingConfig {
    fn default() -> Self {
        Self {
            curve: ForgettingCurve::Ebbinghaus,
            decay_rate: 0.5,
            min_strength: 0.01,
            capacity: 10000,
            rehearsal_boost: 0.3,
            importance_weight: 0.5,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct ForgettingStats {
    /// Items tracked
    pub items_tracked: u64,
    /// Items forgotten
    pub items_forgotten: u64,
    /// Rehearsals
    pub rehearsals: u64,
    /// Decay cycles
    pub decay_cycles: u64,
}

impl ForgettingManager {
    /// Create new manager
    pub fn new(config: ForgettingConfig) -> Self {
        Self {
            items: BTreeMap::new(),
            events: Vec::new(),
            rehearsals: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: ForgettingStats::default(),
        }
    }

    /// Add item
    pub fn add(&mut self, key: &str, importance: f64, tags: Vec<String>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let item = MemoryItem {
            id,
            key: key.into(),
            strength: 1.0,
            importance: importance.clamp(0.0, 1.0),
            access_count: 0,
            created: now,
            last_accessed: now,
            last_rehearsed: now,
            tags,
        };

        self.items.insert(id, item);
        self.stats.items_tracked += 1;

        // Check capacity
        self.enforce_capacity();

        id
    }

    /// Access item
    pub fn access(&mut self, id: u64) -> Option<f64> {
        let now = Timestamp::now();

        let item = self.items.get_mut(&id)?;
        item.access_count += 1;
        item.last_accessed = now;

        // Small strength boost from access
        item.strength = (item.strength + 0.1).min(1.0);

        Some(item.strength)
    }

    /// Rehearse item
    pub fn rehearse(&mut self, id: u64, rehearsal_type: RehearsalType) -> Option<f64> {
        let now = Timestamp::now();

        let item = self.items.get_mut(&id)?;

        let boost = match rehearsal_type {
            RehearsalType::Active => self.config.rehearsal_boost * 1.5,
            RehearsalType::Passive => self.config.rehearsal_boost * 0.5,
            RehearsalType::Spaced => self.config.rehearsal_boost * 2.0,
        };

        item.strength = (item.strength + boost).min(1.0);
        item.last_rehearsed = now;

        self.rehearsals.push(RehearsalRecord {
            item_id: id,
            timestamp: now,
            boost,
            rehearsal_type,
        });

        self.stats.rehearsals += 1;

        Some(item.strength)
    }

    /// Apply decay
    pub fn decay(&mut self, elapsed_ns: u64) {
        self.stats.decay_cycles += 1;
        let now = Timestamp::now();

        let mut to_forget = Vec::new();

        for (&id, item) in self.items.iter_mut() {
            let time_factor = elapsed_ns as f64 / 3600_000_000_000.0; // Hours

            let previous_strength = item.strength;
            let new_strength = self.calculate_decay(item, time_factor);

            if (previous_strength - new_strength).abs() > 0.01 {
                self.events.push(ForgettingEvent {
                    id: self.next_id.fetch_add(1, Ordering::Relaxed),
                    item_id: id,
                    previous_strength,
                    new_strength,
                    reason: ForgettingReason::TimeDecay,
                    timestamp: now,
                });
            }

            item.strength = new_strength;

            if new_strength < self.config.min_strength {
                to_forget.push(id);
            }
        }

        // Remove forgotten items
        for id in to_forget {
            self.forget(id, ForgettingReason::TimeDecay);
        }
    }

    fn calculate_decay(&self, item: &MemoryItem, time_factor: f64) -> f64 {
        let base_decay = match self.config.curve {
            ForgettingCurve::Ebbinghaus => {
                // R = e^(-t/S) where S is stability
                let stability = 1.0 + (item.access_count as f64 * 0.1);
                (-time_factor * self.config.decay_rate / stability).exp()
            }

            ForgettingCurve::PowerLaw => {
                // R = (1 + t)^(-d)
                (1.0 + time_factor).powf(-self.config.decay_rate)
            }

            ForgettingCurve::Linear => {
                // R = 1 - d*t
                1.0 - self.config.decay_rate * time_factor
            }

            ForgettingCurve::Step => {
                // All or nothing at threshold
                if time_factor > 24.0 / self.config.decay_rate {
                    0.0
                } else {
                    1.0
                }
            }

            ForgettingCurve::Custom => item.strength,
        };

        // Apply importance protection
        let importance_factor = 1.0 - (item.importance * self.config.importance_weight);
        let adjusted_decay = base_decay + (1.0 - base_decay) * (1.0 - importance_factor);

        item.strength * adjusted_decay.clamp(0.0, 1.0)
    }

    /// Forget item
    pub fn forget(&mut self, id: u64, reason: ForgettingReason) {
        if let Some(item) = self.items.remove(&id) {
            self.events.push(ForgettingEvent {
                id: self.next_id.fetch_add(1, Ordering::Relaxed),
                item_id: id,
                previous_strength: item.strength,
                new_strength: 0.0,
                reason,
                timestamp: Timestamp::now(),
            });

            self.stats.items_forgotten += 1;
        }
    }

    /// Enforce capacity
    fn enforce_capacity(&mut self) {
        if self.items.len() <= self.config.capacity {
            return;
        }

        // Remove weakest items
        let mut items: Vec<(u64, f64)> = self.items.iter()
            .map(|(&id, item)| {
                let score = item.strength * (1.0 + item.importance);
                (id, score)
            })
            .collect();

        items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(core::cmp::Ordering::Equal));

        let to_remove = self.items.len() - self.config.capacity;
        for (id, _) in items.into_iter().take(to_remove) {
            self.forget(id, ForgettingReason::Capacity);
        }
    }

    /// Get item
    pub fn get(&self, id: u64) -> Option<&MemoryItem> {
        self.items.get(&id)
    }

    /// Get strength
    pub fn strength(&self, id: u64) -> Option<f64> {
        self.items.get(&id).map(|i| i.strength)
    }

    /// Get weak items
    pub fn weak_items(&self, threshold: f64) -> Vec<&MemoryItem> {
        self.items.values()
            .filter(|i| i.strength < threshold)
            .collect()
    }

    /// Get strong items
    pub fn strong_items(&self, threshold: f64) -> Vec<&MemoryItem> {
        self.items.values()
            .filter(|i| i.strength >= threshold)
            .collect()
    }

    /// Get items needing rehearsal
    pub fn needs_rehearsal(&self, since_ns: u64) -> Vec<&MemoryItem> {
        let cutoff = Timestamp::now().0.saturating_sub(since_ns);

        self.items.values()
            .filter(|i| i.last_rehearsed.0 < cutoff && i.strength < 0.8)
            .collect()
    }

    /// Get events
    pub fn events(&self) -> &[ForgettingEvent] {
        &self.events
    }

    /// Get statistics
    pub fn stats(&self) -> &ForgettingStats {
        &self.stats
    }
}

impl Default for ForgettingManager {
    fn default() -> Self {
        Self::new(ForgettingConfig::default())
    }
}

// ============================================================================
// SPACED REPETITION
// ============================================================================

/// Spaced repetition scheduler
pub struct SpacedRepetition {
    /// Intervals (in hours)
    intervals: Vec<f64>,
    /// Review queue
    queue: Vec<ReviewItem>,
    /// Configuration
    config: SRConfig,
}

/// Review item
#[derive(Debug, Clone)]
pub struct ReviewItem {
    /// Item ID
    pub item_id: u64,
    /// Due time
    pub due: Timestamp,
    /// Interval index
    pub interval_idx: usize,
    /// Ease factor
    pub ease: f64,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct SRConfig {
    /// Initial ease
    pub initial_ease: f64,
    /// Minimum ease
    pub min_ease: f64,
    /// Easy bonus
    pub easy_bonus: f64,
}

impl Default for SRConfig {
    fn default() -> Self {
        Self {
            initial_ease: 2.5,
            min_ease: 1.3,
            easy_bonus: 1.3,
        }
    }
}

impl SpacedRepetition {
    /// Create new scheduler
    pub fn new(config: SRConfig) -> Self {
        Self {
            // Typical SM-2 intervals in hours
            intervals: vec![0.0167, 0.167, 1.0, 24.0, 72.0, 168.0, 336.0, 672.0],
            queue: Vec::new(),
            config,
        }
    }

    /// Schedule item
    pub fn schedule(&mut self, item_id: u64) {
        let item = ReviewItem {
            item_id,
            due: Timestamp::now(),
            interval_idx: 0,
            ease: self.config.initial_ease,
        };

        self.queue.push(item);
    }

    /// Get due items
    pub fn due_items(&self) -> Vec<&ReviewItem> {
        let now = Timestamp::now();

        self.queue.iter()
            .filter(|item| item.due.0 <= now.0)
            .collect()
    }

    /// Review item
    pub fn review(&mut self, item_id: u64, quality: u8) {
        let now = Timestamp::now();

        if let Some(item) = self.queue.iter_mut().find(|i| i.item_id == item_id) {
            // Update ease based on quality (0-5)
            let q = quality as f64;
            item.ease = (item.ease + (0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02)))
                .max(self.config.min_ease);

            // Update interval
            if quality >= 3 {
                item.interval_idx = (item.interval_idx + 1).min(self.intervals.len() - 1);
            } else {
                item.interval_idx = 0;
            }

            // Calculate next due time
            let interval_hours = self.intervals[item.interval_idx];
            let adjusted = interval_hours * item.ease;
            let interval_ns = (adjusted * 3600_000_000_000.0) as u64;

            item.due = Timestamp(now.0 + interval_ns);
        }
    }
}

impl Default for SpacedRepetition {
    fn default() -> Self {
        Self::new(SRConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_item() {
        let mut manager = ForgettingManager::default();

        let id = manager.add("test", 0.5, Vec::new());
        assert!(manager.get(id).is_some());
        assert_eq!(manager.strength(id), Some(1.0));
    }

    #[test]
    fn test_access() {
        let mut manager = ForgettingManager::default();

        let id = manager.add("test", 0.5, Vec::new());
        manager.access(id);

        let item = manager.get(id).unwrap();
        assert_eq!(item.access_count, 1);
    }

    #[test]
    fn test_rehearse() {
        let mut manager = ForgettingManager::default();

        let id = manager.add("test", 0.5, Vec::new());

        // Reduce strength first
        if let Some(item) = manager.items.get_mut(&id) {
            item.strength = 0.5;
        }

        let new_strength = manager.rehearse(id, RehearsalType::Active);
        assert!(new_strength.unwrap() > 0.5);
    }

    #[test]
    fn test_decay() {
        let mut manager = ForgettingManager::default();

        let id = manager.add("test", 0.0, Vec::new());
        manager.decay(3600_000_000_000); // 1 hour

        let strength = manager.strength(id).unwrap();
        assert!(strength < 1.0);
    }

    #[test]
    fn test_capacity() {
        let mut manager = ForgettingManager::new(ForgettingConfig {
            capacity: 5,
            ..Default::default()
        });

        for i in 0..10 {
            manager.add(&format!("item{}", i), 0.0, Vec::new());
        }

        assert!(manager.items.len() <= 5);
    }

    #[test]
    fn test_spaced_repetition() {
        let mut sr = SpacedRepetition::default();

        sr.schedule(1);
        let due = sr.due_items();
        assert_eq!(due.len(), 1);

        sr.review(1, 5); // Perfect recall
        let item = sr.queue.iter().find(|i| i.item_id == 1).unwrap();
        assert!(item.interval_idx > 0);
    }
}
