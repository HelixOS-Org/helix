//! # Attention System
//!
//! Manages cognitive attention and focus across domains.
//! Determines what the system should focus on at any given moment.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{ComponentId, Timestamp};

// ============================================================================
// ATTENTION TYPES
// ============================================================================

/// Attention priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AttentionLevel {
    /// Background attention - minimal resources
    Background = 0,
    /// Low attention - occasional checks
    Low        = 1,
    /// Normal attention - regular monitoring
    Normal     = 2,
    /// High attention - frequent checks
    High       = 3,
    /// Critical attention - constant monitoring
    Critical   = 4,
    /// Emergency attention - all resources
    Emergency  = 5,
}

impl AttentionLevel {
    /// Get resource multiplier for this level
    #[inline]
    pub fn resource_multiplier(&self) -> f32 {
        match self {
            Self::Background => 0.1,
            Self::Low => 0.3,
            Self::Normal => 1.0,
            Self::High => 2.0,
            Self::Critical => 5.0,
            Self::Emergency => 10.0,
        }
    }

    /// Get check frequency (cycles between checks)
    #[inline]
    pub fn check_frequency(&self) -> u64 {
        match self {
            Self::Background => 1000,
            Self::Low => 100,
            Self::Normal => 10,
            Self::High => 2,
            Self::Critical => 1,
            Self::Emergency => 1,
        }
    }
}

/// Reason for attention
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttentionReason {
    /// Anomaly detected
    Anomaly,
    /// Performance degradation
    Degradation,
    /// Resource pressure
    ResourcePressure,
    /// Error rate increase
    ErrorRate,
    /// Security concern
    Security,
    /// User request
    UserRequest,
    /// Scheduled check
    Scheduled,
    /// Healing in progress
    Healing,
    /// Learning opportunity
    Learning,
    /// Prediction trigger
    Prediction,
}

/// An attention target
#[derive(Debug, Clone)]
pub struct AttentionItem {
    /// Unique identifier
    pub id: u64,
    /// What we're attending to
    pub target: AttentionTarget,
    /// Current attention level
    pub level: AttentionLevel,
    /// Reason for attention
    pub reason: AttentionReason,
    /// When attention started
    pub started: Timestamp,
    /// Last check timestamp
    pub last_check: Timestamp,
    /// Number of checks performed
    pub check_count: u64,
    /// Accumulated attention score
    pub accumulated_score: f32,
    /// Associated data
    pub context: AttentionContext,
}

/// Target of attention
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttentionTarget {
    /// A specific component
    Component(ComponentId),
    /// A process
    Process(u64),
    /// A subsystem
    Subsystem(SubsystemId),
    /// A memory region
    MemoryRegion { start: u64, size: u64 },
    /// A file descriptor
    FileDescriptor(u64),
    /// A network connection
    NetworkConnection(u64),
    /// A driver
    Driver(u64),
    /// A pattern/anomaly
    Pattern(u64),
    /// Global system state
    System,
}

/// Subsystem identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubsystemId {
    Scheduler,
    Memory,
    FileSystem,
    Network,
    Drivers,
    Security,
    IPC,
    Interrupts,
}

/// Context for attention
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AttentionContext {
    /// Relevant metrics
    pub metrics: BTreeMap<String, f64>,
    /// Related events
    pub related_events: Vec<u64>,
    /// Notes
    pub notes: Vec<String>,
}

// ============================================================================
// ATTENTION MANAGER
// ============================================================================

/// Manages attention allocation
pub struct AttentionManager {
    /// Active attention items
    items: BTreeMap<u64, AttentionItem>,
    /// Next item ID
    next_id: AtomicU64,
    /// Attention budget (total resources available)
    budget: f32,
    /// Current budget usage
    usage: f32,
    /// Configuration
    config: AttentionConfig,
    /// Statistics
    stats: AttentionStats,
}

/// Configuration for attention manager
#[derive(Debug, Clone)]
pub struct AttentionConfig {
    /// Maximum attention items
    pub max_items: usize,
    /// Default attention level
    pub default_level: AttentionLevel,
    /// Attention decay rate (per cycle)
    pub decay_rate: f32,
    /// Minimum attention threshold
    pub min_threshold: f32,
    /// Maximum attention duration (cycles)
    pub max_duration: u64,
    /// Auto-escalation enabled
    pub auto_escalate: bool,
}

impl Default for AttentionConfig {
    fn default() -> Self {
        Self {
            max_items: 100,
            default_level: AttentionLevel::Normal,
            decay_rate: 0.01,
            min_threshold: 0.1,
            max_duration: 10000,
            auto_escalate: true,
        }
    }
}

/// Statistics for attention manager
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct AttentionStats {
    /// Total attention items created
    pub total_items: u64,
    /// Currently active items
    pub active_items: u64,
    /// Items escalated
    pub escalations: u64,
    /// Items de-escalated
    pub de_escalations: u64,
    /// Items expired
    pub expired: u64,
    /// Total checks performed
    pub total_checks: u64,
    /// Average attention duration
    pub avg_duration: f64,
}

impl AttentionManager {
    /// Create a new attention manager
    pub fn new(config: AttentionConfig) -> Self {
        Self {
            items: BTreeMap::new(),
            next_id: AtomicU64::new(1),
            budget: 100.0,
            usage: 0.0,
            config,
            stats: AttentionStats::default(),
        }
    }

    /// Add a new attention item
    pub fn add_attention(
        &mut self,
        target: AttentionTarget,
        level: AttentionLevel,
        reason: AttentionReason,
    ) -> Option<u64> {
        // Check if we already have attention on this target
        for (id, item) in &mut self.items {
            if item.target == target {
                // Upgrade if new level is higher
                if level > item.level {
                    item.level = level;
                    item.reason = reason;
                    self.stats.escalations += 1;
                }
                return Some(*id);
            }
        }

        // Check capacity
        if self.items.len() >= self.config.max_items {
            // Try to remove lowest priority item
            self.evict_lowest();
        }

        if self.items.len() >= self.config.max_items {
            return None;
        }

        // Check budget
        let cost = level.resource_multiplier();
        if self.usage + cost > self.budget {
            return None;
        }

        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let now = Timestamp::now();

        let item = AttentionItem {
            id,
            target,
            level,
            reason,
            started: now,
            last_check: now,
            check_count: 0,
            accumulated_score: 0.0,
            context: AttentionContext::default(),
        };

        self.items.insert(id, item);
        self.usage += cost;
        self.stats.total_items += 1;
        self.stats.active_items = self.items.len() as u64;

        Some(id)
    }

    /// Remove an attention item
    #[inline]
    pub fn remove_attention(&mut self, id: u64) -> bool {
        if let Some(item) = self.items.remove(&id) {
            self.usage -= item.level.resource_multiplier();
            self.stats.active_items = self.items.len() as u64;
            true
        } else {
            false
        }
    }

    /// Escalate attention level
    pub fn escalate(&mut self, id: u64) -> bool {
        if let Some(item) = self.items.get_mut(&id) {
            let old_cost = item.level.resource_multiplier();

            let new_level = match item.level {
                AttentionLevel::Background => AttentionLevel::Low,
                AttentionLevel::Low => AttentionLevel::Normal,
                AttentionLevel::Normal => AttentionLevel::High,
                AttentionLevel::High => AttentionLevel::Critical,
                AttentionLevel::Critical => AttentionLevel::Emergency,
                AttentionLevel::Emergency => return false,
            };

            let new_cost = new_level.resource_multiplier();

            if self.usage - old_cost + new_cost <= self.budget {
                self.usage = self.usage - old_cost + new_cost;
                item.level = new_level;
                self.stats.escalations += 1;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// De-escalate attention level
    pub fn de_escalate(&mut self, id: u64) -> bool {
        if let Some(item) = self.items.get_mut(&id) {
            let old_cost = item.level.resource_multiplier();

            let new_level = match item.level {
                AttentionLevel::Background => return false,
                AttentionLevel::Low => AttentionLevel::Background,
                AttentionLevel::Normal => AttentionLevel::Low,
                AttentionLevel::High => AttentionLevel::Normal,
                AttentionLevel::Critical => AttentionLevel::High,
                AttentionLevel::Emergency => AttentionLevel::Critical,
            };

            let new_cost = new_level.resource_multiplier();
            self.usage = self.usage - old_cost + new_cost;
            item.level = new_level;
            self.stats.de_escalations += 1;
            true
        } else {
            false
        }
    }

    /// Get items that need checking this cycle
    #[inline]
    pub fn items_to_check(&self, cycle: u64) -> Vec<u64> {
        self.items
            .iter()
            .filter(|(_, item)| cycle % item.level.check_frequency() == 0)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Mark item as checked
    #[inline]
    pub fn mark_checked(&mut self, id: u64, score: f32) {
        if let Some(item) = self.items.get_mut(&id) {
            item.last_check = Timestamp::now();
            item.check_count += 1;
            item.accumulated_score += score;
            self.stats.total_checks += 1;
        }
    }

    /// Process decay and expiration
    pub fn tick(&mut self, cycle: u64) {
        let mut to_remove = Vec::new();
        let mut to_de_escalate = Vec::new();

        for (id, item) in &self.items {
            // Check expiration
            let duration = cycle.saturating_sub(item.started.as_cycles());
            if duration > self.config.max_duration {
                to_remove.push(*id);
                continue;
            }

            // Check for de-escalation based on score
            if item.check_count > 0 {
                let avg_score = item.accumulated_score / item.check_count as f32;
                if avg_score < self.config.min_threshold && item.level > AttentionLevel::Background
                {
                    to_de_escalate.push(*id);
                }
            }
        }

        for id in to_remove {
            self.remove_attention(id);
            self.stats.expired += 1;
        }

        for id in to_de_escalate {
            self.de_escalate(id);
        }
    }

    /// Evict lowest priority item
    fn evict_lowest(&mut self) {
        let lowest = self
            .items
            .iter()
            .min_by_key(|(_, item)| (item.level as u8, item.accumulated_score as i64));

        if let Some((id, _)) = lowest {
            let id = *id;
            self.remove_attention(id);
        }
    }

    /// Get attention item by ID
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&AttentionItem> {
        self.items.get(&id)
    }

    /// Get all items for a target
    #[inline]
    pub fn items_for_target(&self, target: &AttentionTarget) -> Vec<&AttentionItem> {
        self.items
            .values()
            .filter(|i| &i.target == target)
            .collect()
    }

    /// Get current budget usage
    #[inline(always)]
    pub fn usage(&self) -> f32 {
        self.usage
    }

    /// Get available budget
    #[inline(always)]
    pub fn available(&self) -> f32 {
        self.budget - self.usage
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &AttentionStats {
        &self.stats
    }

    /// Get item count
    #[inline(always)]
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// Get items by level
    #[inline(always)]
    pub fn items_by_level(&self, level: AttentionLevel) -> Vec<&AttentionItem> {
        self.items.values().filter(|i| i.level == level).collect()
    }

    /// Get highest priority items
    #[inline]
    pub fn highest_priority(&self, count: usize) -> Vec<&AttentionItem> {
        let mut items: Vec<_> = self.items.values().collect();
        items.sort_by(|a, b| b.level.cmp(&a.level));
        items.into_iter().take(count).collect()
    }
}

// ============================================================================
// ATTENTION SCHEDULER
// ============================================================================

/// Schedules attention across cognitive cycles
pub struct AttentionScheduler {
    /// Attention manager
    manager: AttentionManager,
    /// Current cycle
    cycle: u64,
    /// Scheduled checks this cycle
    scheduled: Vec<u64>,
    /// Configuration
    config: SchedulerConfig,
}

/// Configuration for attention scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Maximum checks per cycle
    pub max_checks_per_cycle: usize,
    /// Prioritize critical items
    pub prioritize_critical: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            max_checks_per_cycle: 50,
            prioritize_critical: true,
        }
    }
}

impl AttentionScheduler {
    /// Create a new scheduler
    pub fn new(attention_config: AttentionConfig, scheduler_config: SchedulerConfig) -> Self {
        Self {
            manager: AttentionManager::new(attention_config),
            cycle: 0,
            scheduled: Vec::new(),
            config: scheduler_config,
        }
    }

    /// Advance to next cycle and get items to check
    pub fn next_cycle(&mut self) -> Vec<u64> {
        self.cycle += 1;
        self.manager.tick(self.cycle);

        let mut items = self.manager.items_to_check(self.cycle);

        // Sort by priority if configured
        if self.config.prioritize_critical {
            items.sort_by(|a, b| {
                let level_a = self
                    .manager
                    .get(*a)
                    .map(|i| i.level)
                    .unwrap_or(AttentionLevel::Background);
                let level_b = self
                    .manager
                    .get(*b)
                    .map(|i| i.level)
                    .unwrap_or(AttentionLevel::Background);
                level_b.cmp(&level_a)
            });
        }

        // Limit to max
        items.truncate(self.config.max_checks_per_cycle);

        self.scheduled = items.clone();
        items
    }

    /// Add attention
    #[inline(always)]
    pub fn add_attention(
        &mut self,
        target: AttentionTarget,
        level: AttentionLevel,
        reason: AttentionReason,
    ) -> Option<u64> {
        self.manager.add_attention(target, level, reason)
    }

    /// Remove attention
    #[inline(always)]
    pub fn remove_attention(&mut self, id: u64) -> bool {
        self.manager.remove_attention(id)
    }

    /// Mark as checked
    #[inline(always)]
    pub fn mark_checked(&mut self, id: u64, score: f32) {
        self.manager.mark_checked(id, score);
    }

    /// Escalate
    #[inline(always)]
    pub fn escalate(&mut self, id: u64) -> bool {
        self.manager.escalate(id)
    }

    /// Get manager reference
    #[inline(always)]
    pub fn manager(&self) -> &AttentionManager {
        &self.manager
    }

    /// Get current cycle
    #[inline(always)]
    pub fn current_cycle(&self) -> u64 {
        self.cycle
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attention_level() {
        assert!(AttentionLevel::Critical > AttentionLevel::Normal);
        assert!(
            AttentionLevel::Emergency.resource_multiplier()
                > AttentionLevel::Normal.resource_multiplier()
        );
    }

    #[test]
    fn test_attention_manager() {
        let config = AttentionConfig::default();
        let mut manager = AttentionManager::new(config);

        let id = manager.add_attention(
            AttentionTarget::System,
            AttentionLevel::Normal,
            AttentionReason::Scheduled,
        );
        assert!(id.is_some());
        assert_eq!(manager.count(), 1);

        manager.remove_attention(id.unwrap());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_escalation() {
        let config = AttentionConfig::default();
        let mut manager = AttentionManager::new(config);

        let id = manager
            .add_attention(
                AttentionTarget::System,
                AttentionLevel::Normal,
                AttentionReason::Anomaly,
            )
            .unwrap();

        assert!(manager.escalate(id));
        assert_eq!(manager.get(id).unwrap().level, AttentionLevel::High);

        assert!(manager.de_escalate(id));
        assert_eq!(manager.get(id).unwrap().level, AttentionLevel::Normal);
    }

    #[test]
    fn test_scheduler() {
        let att_config = AttentionConfig::default();
        let sch_config = SchedulerConfig::default();
        let mut scheduler = AttentionScheduler::new(att_config, sch_config);

        scheduler.add_attention(
            AttentionTarget::System,
            AttentionLevel::Critical,
            AttentionReason::Anomaly,
        );

        let items = scheduler.next_cycle();
        assert!(!items.is_empty());
    }
}
