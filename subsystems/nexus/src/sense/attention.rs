//! # Attention Mechanism
//!
//! Attention allocation for focusing cognitive resources.
//! Manages salience and priority of incoming information.
//!
//! Part of Year 2 COGNITION - Perception Engine

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::Timestamp;

// ============================================================================
// ATTENTION TYPES
// ============================================================================

/// Attention target
#[derive(Debug, Clone)]
pub struct AttentionTarget {
    /// Target ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Target type
    pub target_type: TargetType,
    /// Salience score
    pub salience: f64,
    /// Priority
    pub priority: Priority,
    /// Status
    pub status: AttentionStatus,
    /// Time focused
    pub focus_duration: u64,
    /// Last focused
    pub last_focused: Option<Timestamp>,
    /// Metadata
    pub metadata: BTreeMap<String, String>,
}

/// Target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetType {
    Stimulus,
    Task,
    Goal,
    Threat,
    Opportunity,
    Anomaly,
    Pattern,
}

/// Priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Background,
    Low,
    Normal,
    High,
    Critical,
}

/// Attention status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionStatus {
    Active,
    Queued,
    Suspended,
    Completed,
    Expired,
}

/// Focus
#[derive(Debug, Clone)]
pub struct Focus {
    /// Target ID
    pub target_id: u64,
    /// Focus start
    pub started: Timestamp,
    /// Intensity (0.0 - 1.0)
    pub intensity: f64,
}

// ============================================================================
// ATTENTION MANAGER
// ============================================================================

/// Attention manager
pub struct AttentionManager {
    /// Attention targets
    targets: BTreeMap<u64, AttentionTarget>,
    /// Current focus stack
    focus_stack: Vec<Focus>,
    /// Attention history
    history: Vec<AttentionEvent>,
    /// Next ID
    next_id: AtomicU64,
    /// Configuration
    config: AttentionConfig,
    /// Statistics
    stats: AttentionStats,
}

/// Attention event
#[derive(Debug, Clone)]
pub struct AttentionEvent {
    /// Event ID
    pub id: u64,
    /// Target ID
    pub target_id: u64,
    /// Event type
    pub event_type: AttentionEventType,
    /// Timestamp
    pub timestamp: Timestamp,
}

/// Attention event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttentionEventType {
    Focused,
    Unfocused,
    Prioritized,
    Suspended,
    Expired,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct AttentionConfig {
    /// Maximum focus stack size
    pub max_focus_stack: usize,
    /// Decay rate for salience
    pub salience_decay: f64,
    /// Base attention capacity
    pub capacity: f64,
    /// Priority boost factor
    pub priority_boost: f64,
    /// Expiration time (ns)
    pub expiration_ns: u64,
}

impl Default for AttentionConfig {
    fn default() -> Self {
        Self {
            max_focus_stack: 5,
            salience_decay: 0.01,
            capacity: 1.0,
            priority_boost: 0.2,
            expiration_ns: 60_000_000_000, // 60 seconds
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AttentionStats {
    /// Total targets
    pub total_targets: u64,
    /// Focus switches
    pub focus_switches: u64,
    /// Average focus duration (ns)
    pub avg_focus_duration_ns: f64,
    /// Expired targets
    pub expired: u64,
}

impl AttentionManager {
    /// Create new attention manager
    pub fn new(config: AttentionConfig) -> Self {
        Self {
            targets: BTreeMap::new(),
            focus_stack: Vec::new(),
            history: Vec::new(),
            next_id: AtomicU64::new(1),
            config,
            stats: AttentionStats::default(),
        }
    }

    /// Register attention target
    pub fn register(&mut self, name: &str, target_type: TargetType, priority: Priority) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let salience = self.compute_initial_salience(target_type, priority);

        let target = AttentionTarget {
            id,
            name: name.into(),
            target_type,
            salience,
            priority,
            status: AttentionStatus::Queued,
            focus_duration: 0,
            last_focused: None,
            metadata: BTreeMap::new(),
        };

        self.targets.insert(id, target);
        self.stats.total_targets += 1;

        id
    }

    fn compute_initial_salience(&self, target_type: TargetType, priority: Priority) -> f64 {
        let type_weight = match target_type {
            TargetType::Threat => 1.0,
            TargetType::Anomaly => 0.9,
            TargetType::Goal => 0.7,
            TargetType::Task => 0.6,
            TargetType::Opportunity => 0.5,
            TargetType::Pattern => 0.4,
            TargetType::Stimulus => 0.3,
        };

        let priority_weight = match priority {
            Priority::Critical => 1.0,
            Priority::High => 0.8,
            Priority::Normal => 0.5,
            Priority::Low => 0.3,
            Priority::Background => 0.1,
        };

        (type_weight + priority_weight) / 2.0
    }

    /// Focus on target
    pub fn focus(&mut self, target_id: u64) -> bool {
        if !self.targets.contains_key(&target_id) {
            return false;
        }

        // Unfocus current if at capacity
        if self.focus_stack.len() >= self.config.max_focus_stack {
            self.unfocus_lowest();
        }

        // Add to focus stack
        let focus = Focus {
            target_id,
            started: Timestamp::now(),
            intensity: self.config.capacity / (self.focus_stack.len() + 1) as f64,
        };

        self.focus_stack.push(focus);

        // Update target
        if let Some(target) = self.targets.get_mut(&target_id) {
            target.status = AttentionStatus::Active;
            target.last_focused = Some(Timestamp::now());
        }

        // Record event
        self.record_event(target_id, AttentionEventType::Focused);

        self.stats.focus_switches += 1;

        true
    }

    /// Unfocus target
    pub fn unfocus(&mut self, target_id: u64) {
        let now = Timestamp::now();

        // Remove from focus stack
        if let Some(pos) = self
            .focus_stack
            .iter()
            .position(|f| f.target_id == target_id)
        {
            let focus = self.focus_stack.remove(pos);

            // Update duration
            let duration = now.0 - focus.started.0;
            if let Some(target) = self.targets.get_mut(&target_id) {
                target.focus_duration += duration;
                target.status = AttentionStatus::Queued;
            }

            // Update avg duration
            let n = self.stats.focus_switches as f64;
            self.stats.avg_focus_duration_ns =
                (self.stats.avg_focus_duration_ns * (n - 1.0) + duration as f64) / n;

            self.record_event(target_id, AttentionEventType::Unfocused);

            // Redistribute intensity
            self.redistribute_intensity();
        }
    }

    fn unfocus_lowest(&mut self) {
        // Find focus with lowest salience
        let lowest = self
            .focus_stack
            .iter()
            .map(|f| {
                (
                    f.target_id,
                    self.targets
                        .get(&f.target_id)
                        .map(|t| t.salience)
                        .unwrap_or(0.0),
                )
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(id, _)| id);

        if let Some(target_id) = lowest {
            self.unfocus(target_id);
        }
    }

    fn redistribute_intensity(&mut self) {
        let count = self.focus_stack.len();
        if count == 0 {
            return;
        }

        let total_salience: f64 = self
            .focus_stack
            .iter()
            .filter_map(|f| self.targets.get(&f.target_id))
            .map(|t| t.salience)
            .sum();

        for focus in &mut self.focus_stack {
            if let Some(target) = self.targets.get(&focus.target_id) {
                focus.intensity = if total_salience > 0.0 {
                    (target.salience / total_salience) * self.config.capacity
                } else {
                    self.config.capacity / count as f64
                };
            }
        }
    }

    fn record_event(&mut self, target_id: u64, event_type: AttentionEventType) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        self.history.push(AttentionEvent {
            id,
            target_id,
            event_type,
            timestamp: Timestamp::now(),
        });
    }

    /// Update salience
    #[inline]
    pub fn update_salience(&mut self, target_id: u64, salience: f64) {
        if let Some(target) = self.targets.get_mut(&target_id) {
            target.salience = salience.clamp(0.0, 1.0);
        }

        self.redistribute_intensity();
    }

    /// Boost priority
    pub fn boost_priority(&mut self, target_id: u64) {
        if let Some(target) = self.targets.get_mut(&target_id) {
            target.salience = (target.salience + self.config.priority_boost).min(1.0);
            target.priority = match target.priority {
                Priority::Background => Priority::Low,
                Priority::Low => Priority::Normal,
                Priority::Normal => Priority::High,
                Priority::High => Priority::Critical,
                Priority::Critical => Priority::Critical,
            };

            self.record_event(target_id, AttentionEventType::Prioritized);
        }
    }

    /// Apply decay
    #[inline]
    pub fn decay(&mut self) {
        for target in self.targets.values_mut() {
            if target.status != AttentionStatus::Active {
                target.salience = (target.salience - self.config.salience_decay).max(0.0);
            }
        }
    }

    /// Expire old targets
    pub fn expire(&mut self) {
        let now = Timestamp::now();

        let expired: Vec<u64> = self
            .targets
            .iter()
            .filter(|(_, t)| {
                t.status == AttentionStatus::Queued
                    && t.last_focused
                        .map_or(true, |lf| now.0 - lf.0 > self.config.expiration_ns)
            })
            .map(|(&id, _)| id)
            .collect();

        for id in expired {
            if let Some(target) = self.targets.get_mut(&id) {
                target.status = AttentionStatus::Expired;
                self.record_event(id, AttentionEventType::Expired);
                self.stats.expired += 1;
            }
        }
    }

    /// Get most salient target
    #[inline]
    pub fn most_salient(&self) -> Option<&AttentionTarget> {
        self.targets
            .values()
            .filter(|t| t.status == AttentionStatus::Queued || t.status == AttentionStatus::Active)
            .max_by(|a, b| a.salience.partial_cmp(&b.salience).unwrap())
    }

    /// Get current focus
    #[inline]
    pub fn current_focus(&self) -> Vec<&AttentionTarget> {
        self.focus_stack
            .iter()
            .filter_map(|f| self.targets.get(&f.target_id))
            .collect()
    }

    /// Get target
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&AttentionTarget> {
        self.targets.get(&id)
    }

    /// Is focused
    #[inline(always)]
    pub fn is_focused(&self, target_id: u64) -> bool {
        self.focus_stack.iter().any(|f| f.target_id == target_id)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &AttentionStats {
        &self.stats
    }
}

impl Default for AttentionManager {
    fn default() -> Self {
        Self::new(AttentionConfig::default())
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register() {
        let mut manager = AttentionManager::default();

        let id = manager.register("test", TargetType::Task, Priority::Normal);
        assert!(manager.get(id).is_some());
    }

    #[test]
    fn test_focus() {
        let mut manager = AttentionManager::default();

        let id = manager.register("test", TargetType::Task, Priority::Normal);
        assert!(manager.focus(id));
        assert!(manager.is_focused(id));
    }

    #[test]
    fn test_unfocus() {
        let mut manager = AttentionManager::default();

        let id = manager.register("test", TargetType::Task, Priority::Normal);
        manager.focus(id);
        manager.unfocus(id);

        assert!(!manager.is_focused(id));
    }

    #[test]
    fn test_salience_ordering() {
        let mut manager = AttentionManager::default();

        let low = manager.register("low", TargetType::Stimulus, Priority::Low);
        let high = manager.register("high", TargetType::Threat, Priority::Critical);

        let most = manager.most_salient().unwrap();
        assert_eq!(most.id, high);
    }

    #[test]
    fn test_boost_priority() {
        let mut manager = AttentionManager::default();

        let id = manager.register("test", TargetType::Task, Priority::Normal);
        let initial_salience = manager.get(id).unwrap().salience;

        manager.boost_priority(id);

        let target = manager.get(id).unwrap();
        assert!(target.salience > initial_salience);
        assert_eq!(target.priority, Priority::High);
    }

    #[test]
    fn test_focus_stack_limit() {
        let mut manager = AttentionManager::new(AttentionConfig {
            max_focus_stack: 2,
            ..Default::default()
        });

        let id1 = manager.register("t1", TargetType::Task, Priority::Normal);
        let id2 = manager.register("t2", TargetType::Task, Priority::High);
        let id3 = manager.register("t3", TargetType::Task, Priority::Critical);

        manager.focus(id1);
        manager.focus(id2);
        manager.focus(id3);

        // Should have bumped lowest salience
        assert_eq!(manager.focus_stack.len(), 2);
    }
}
