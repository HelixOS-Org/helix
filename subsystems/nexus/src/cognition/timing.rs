//! # Cognitive Timing System
//!
//! Manages timing and scheduling for cognitive operations.
//! Provides timers, delays, and time-based events.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

// ============================================================================
// TIMING TYPES
// ============================================================================

/// A timer
#[derive(Debug, Clone)]
pub struct Timer {
    /// Timer ID
    pub id: u64,
    /// Timer name
    pub name: String,
    /// Start time
    pub start_time: Timestamp,
    /// Duration (nanoseconds)
    pub duration_ns: u64,
    /// Is repeating
    pub repeating: bool,
    /// Callback tag
    pub callback_tag: String,
    /// Owner domain
    pub owner: DomainId,
    /// Enabled
    pub enabled: bool,
    /// Fire count
    pub fire_count: u64,
}

/// Timer event
#[derive(Debug, Clone)]
pub struct TimerEvent {
    /// Timer ID
    pub timer_id: u64,
    /// Timer name
    pub timer_name: String,
    /// Fire time
    pub fire_time: Timestamp,
    /// Callback tag
    pub callback_tag: String,
    /// Fire count
    pub fire_count: u64,
}

/// Deadline
#[derive(Debug, Clone)]
pub struct Deadline {
    /// Deadline ID
    pub id: u64,
    /// Description
    pub description: String,
    /// Due time
    pub due_time: Timestamp,
    /// Owner domain
    pub owner: DomainId,
    /// Soft deadline
    pub soft: bool,
    /// Miss count
    pub miss_count: u64,
}

/// Deadline status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadlineStatus {
    /// On time
    OnTime,
    /// Warning (approaching)
    Warning,
    /// Missed
    Missed,
}

/// Time budget
#[derive(Debug, Clone)]
pub struct TimeBudget {
    /// Budget ID
    pub id: u64,
    /// Budget name
    pub name: String,
    /// Total budget (nanoseconds)
    pub total_ns: u64,
    /// Used budget (nanoseconds)
    pub used_ns: u64,
    /// Owner domain
    pub owner: DomainId,
    /// Period (for replenishment)
    pub period_ns: Option<u64>,
    /// Last replenishment
    pub last_replenish: Timestamp,
}

// ============================================================================
// TIMING MANAGER
// ============================================================================

/// Manages timing for cognitive domains
pub struct TimingManager {
    /// Timers
    timers: BTreeMap<u64, Timer>,
    /// Deadlines
    deadlines: BTreeMap<u64, Deadline>,
    /// Time budgets
    budgets: BTreeMap<u64, TimeBudget>,
    /// Pending events
    pending_events: Vec<TimerEvent>,
    /// Next timer ID
    next_timer_id: AtomicU64,
    /// Next deadline ID
    next_deadline_id: AtomicU64,
    /// Next budget ID
    next_budget_id: AtomicU64,
    /// Configuration
    config: TimingConfig,
    /// Statistics
    stats: TimingStats,
}

/// Configuration
#[derive(Debug, Clone)]
pub struct TimingConfig {
    /// Maximum timers
    pub max_timers: usize,
    /// Maximum deadlines
    pub max_deadlines: usize,
    /// Deadline warning threshold (ns before due)
    pub warning_threshold_ns: u64,
    /// Enable automatic replenishment
    pub auto_replenish: bool,
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            max_timers: 1000,
            max_deadlines: 1000,
            warning_threshold_ns: 1_000_000, // 1ms warning
            auto_replenish: true,
        }
    }
}

/// Statistics
#[derive(Debug, Clone, Default)]
pub struct TimingStats {
    /// Total timers created
    pub total_timers: u64,
    /// Total timer fires
    pub total_fires: u64,
    /// Total deadlines created
    pub total_deadlines: u64,
    /// Deadlines met
    pub deadlines_met: u64,
    /// Deadlines missed
    pub deadlines_missed: u64,
    /// Active timers
    pub active_timers: u64,
    /// Active deadlines
    pub active_deadlines: u64,
}

impl TimingManager {
    /// Create a new timing manager
    pub fn new(config: TimingConfig) -> Self {
        Self {
            timers: BTreeMap::new(),
            deadlines: BTreeMap::new(),
            budgets: BTreeMap::new(),
            pending_events: Vec::new(),
            next_timer_id: AtomicU64::new(1),
            next_deadline_id: AtomicU64::new(1),
            next_budget_id: AtomicU64::new(1),
            config,
            stats: TimingStats::default(),
        }
    }

    // ========================================================================
    // TIMERS
    // ========================================================================

    /// Create a timer
    pub fn create_timer(
        &mut self,
        name: &str,
        duration_ns: u64,
        repeating: bool,
        callback_tag: &str,
        owner: DomainId,
    ) -> u64 {
        let id = self.next_timer_id.fetch_add(1, Ordering::Relaxed);

        let timer = Timer {
            id,
            name: name.into(),
            start_time: Timestamp::now(),
            duration_ns,
            repeating,
            callback_tag: callback_tag.into(),
            owner,
            enabled: true,
            fire_count: 0,
        };

        self.timers.insert(id, timer);
        self.stats.total_timers += 1;
        self.stats.active_timers = self.timers.len() as u64;

        id
    }

    /// Cancel a timer
    pub fn cancel_timer(&mut self, timer_id: u64) -> bool {
        let removed = self.timers.remove(&timer_id).is_some();
        if removed {
            self.stats.active_timers = self.timers.len() as u64;
        }
        removed
    }

    /// Pause a timer
    pub fn pause_timer(&mut self, timer_id: u64) {
        if let Some(timer) = self.timers.get_mut(&timer_id) {
            timer.enabled = false;
        }
    }

    /// Resume a timer
    pub fn resume_timer(&mut self, timer_id: u64) {
        if let Some(timer) = self.timers.get_mut(&timer_id) {
            timer.enabled = true;
            timer.start_time = Timestamp::now();
        }
    }

    /// Reset a timer
    pub fn reset_timer(&mut self, timer_id: u64) {
        if let Some(timer) = self.timers.get_mut(&timer_id) {
            timer.start_time = Timestamp::now();
            timer.fire_count = 0;
        }
    }

    /// Get timer
    pub fn get_timer(&self, timer_id: u64) -> Option<&Timer> {
        self.timers.get(&timer_id)
    }

    /// Get timers by owner
    pub fn get_timers_by_owner(&self, owner: DomainId) -> Vec<&Timer> {
        self.timers.values().filter(|t| t.owner == owner).collect()
    }

    // ========================================================================
    // DEADLINES
    // ========================================================================

    /// Create a deadline
    pub fn create_deadline(
        &mut self,
        description: &str,
        due_time: Timestamp,
        owner: DomainId,
        soft: bool,
    ) -> u64 {
        let id = self.next_deadline_id.fetch_add(1, Ordering::Relaxed);

        let deadline = Deadline {
            id,
            description: description.into(),
            due_time,
            owner,
            soft,
            miss_count: 0,
        };

        self.deadlines.insert(id, deadline);
        self.stats.total_deadlines += 1;
        self.stats.active_deadlines = self.deadlines.len() as u64;

        id
    }

    /// Create a deadline relative to now
    pub fn create_deadline_relative(
        &mut self,
        description: &str,
        duration_ns: u64,
        owner: DomainId,
        soft: bool,
    ) -> u64 {
        let due_time = Timestamp::from_raw(Timestamp::now().raw() + duration_ns);
        self.create_deadline(description, due_time, owner, soft)
    }

    /// Check deadline status
    pub fn check_deadline(&self, deadline_id: u64) -> Option<DeadlineStatus> {
        let deadline = self.deadlines.get(&deadline_id)?;
        let now = Timestamp::now();

        if now.raw() > deadline.due_time.raw() {
            Some(DeadlineStatus::Missed)
        } else if now.raw() + self.config.warning_threshold_ns > deadline.due_time.raw() {
            Some(DeadlineStatus::Warning)
        } else {
            Some(DeadlineStatus::OnTime)
        }
    }

    /// Meet a deadline
    pub fn meet_deadline(&mut self, deadline_id: u64) -> bool {
        if let Some(deadline) = self.deadlines.remove(&deadline_id) {
            let now = Timestamp::now();
            if now.raw() <= deadline.due_time.raw() {
                self.stats.deadlines_met += 1;
                return true;
            } else {
                self.stats.deadlines_missed += 1;
            }
        }
        self.stats.active_deadlines = self.deadlines.len() as u64;
        false
    }

    /// Cancel a deadline
    pub fn cancel_deadline(&mut self, deadline_id: u64) -> bool {
        let removed = self.deadlines.remove(&deadline_id).is_some();
        if removed {
            self.stats.active_deadlines = self.deadlines.len() as u64;
        }
        removed
    }

    /// Get deadline
    pub fn get_deadline(&self, deadline_id: u64) -> Option<&Deadline> {
        self.deadlines.get(&deadline_id)
    }

    // ========================================================================
    // TIME BUDGETS
    // ========================================================================

    /// Create a time budget
    pub fn create_budget(
        &mut self,
        name: &str,
        total_ns: u64,
        owner: DomainId,
        period_ns: Option<u64>,
    ) -> u64 {
        let id = self.next_budget_id.fetch_add(1, Ordering::Relaxed);

        let budget = TimeBudget {
            id,
            name: name.into(),
            total_ns,
            used_ns: 0,
            owner,
            period_ns,
            last_replenish: Timestamp::now(),
        };

        self.budgets.insert(id, budget);
        id
    }

    /// Use budget
    pub fn use_budget(&mut self, budget_id: u64, amount_ns: u64) -> bool {
        if let Some(budget) = self.budgets.get_mut(&budget_id) {
            if budget.used_ns + amount_ns <= budget.total_ns {
                budget.used_ns += amount_ns;
                return true;
            }
        }
        false
    }

    /// Check remaining budget
    pub fn remaining_budget(&self, budget_id: u64) -> u64 {
        self.budgets
            .get(&budget_id)
            .map(|b| b.total_ns.saturating_sub(b.used_ns))
            .unwrap_or(0)
    }

    /// Replenish budget
    pub fn replenish_budget(&mut self, budget_id: u64) {
        if let Some(budget) = self.budgets.get_mut(&budget_id) {
            budget.used_ns = 0;
            budget.last_replenish = Timestamp::now();
        }
    }

    /// Delete budget
    pub fn delete_budget(&mut self, budget_id: u64) -> bool {
        self.budgets.remove(&budget_id).is_some()
    }

    // ========================================================================
    // PROCESSING
    // ========================================================================

    /// Process tick - check timers and deadlines
    pub fn tick(&mut self) -> Vec<TimerEvent> {
        let now = Timestamp::now();
        let mut events = Vec::new();

        // Process timers
        let mut to_remove = Vec::new();

        for (id, timer) in &mut self.timers {
            if !timer.enabled {
                continue;
            }

            let elapsed = now.elapsed_since(timer.start_time);
            if elapsed >= timer.duration_ns {
                // Timer fired
                timer.fire_count += 1;
                self.stats.total_fires += 1;

                events.push(TimerEvent {
                    timer_id: timer.id,
                    timer_name: timer.name.clone(),
                    fire_time: now,
                    callback_tag: timer.callback_tag.clone(),
                    fire_count: timer.fire_count,
                });

                if timer.repeating {
                    timer.start_time = now;
                } else {
                    to_remove.push(*id);
                }
            }
        }

        for id in to_remove {
            self.timers.remove(&id);
        }
        self.stats.active_timers = self.timers.len() as u64;

        // Process missed deadlines
        for deadline in self.deadlines.values_mut() {
            if now.raw() > deadline.due_time.raw() {
                deadline.miss_count += 1;
            }
        }

        // Auto-replenish budgets
        if self.config.auto_replenish {
            for budget in self.budgets.values_mut() {
                if let Some(period) = budget.period_ns {
                    if now.elapsed_since(budget.last_replenish) >= period {
                        budget.used_ns = 0;
                        budget.last_replenish = now;
                    }
                }
            }
        }

        events
    }

    /// Get pending events
    pub fn get_pending_events(&mut self) -> Vec<TimerEvent> {
        core::mem::take(&mut self.pending_events)
    }

    /// Get statistics
    pub fn stats(&self) -> &TimingStats {
        &self.stats
    }
}

// ============================================================================
// STOPWATCH
// ============================================================================

/// Simple stopwatch for measuring durations
pub struct Stopwatch {
    /// Start time
    start: Timestamp,
    /// Accumulated time (for pause/resume)
    accumulated: u64,
    /// Is running
    running: bool,
    /// Lap times
    laps: Vec<u64>,
}

impl Stopwatch {
    /// Create and start a new stopwatch
    pub fn new() -> Self {
        Self {
            start: Timestamp::now(),
            accumulated: 0,
            running: true,
            laps: Vec::new(),
        }
    }

    /// Create a stopped stopwatch
    pub fn new_stopped() -> Self {
        Self {
            start: Timestamp::now(),
            accumulated: 0,
            running: false,
            laps: Vec::new(),
        }
    }

    /// Start the stopwatch
    pub fn start(&mut self) {
        if !self.running {
            self.start = Timestamp::now();
            self.running = true;
        }
    }

    /// Stop the stopwatch
    pub fn stop(&mut self) {
        if self.running {
            self.accumulated += Timestamp::now().elapsed_since(self.start);
            self.running = false;
        }
    }

    /// Reset the stopwatch
    pub fn reset(&mut self) {
        self.start = Timestamp::now();
        self.accumulated = 0;
        self.laps.clear();
    }

    /// Restart the stopwatch
    pub fn restart(&mut self) {
        self.reset();
        self.running = true;
    }

    /// Record a lap
    pub fn lap(&mut self) -> u64 {
        let elapsed = self.elapsed_ns();
        self.laps.push(elapsed);
        elapsed
    }

    /// Get elapsed time in nanoseconds
    pub fn elapsed_ns(&self) -> u64 {
        if self.running {
            self.accumulated + Timestamp::now().elapsed_since(self.start)
        } else {
            self.accumulated
        }
    }

    /// Get elapsed time in microseconds
    pub fn elapsed_us(&self) -> u64 {
        self.elapsed_ns() / 1_000
    }

    /// Get elapsed time in milliseconds
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ns() / 1_000_000
    }

    /// Check if running
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Get lap times
    pub fn laps(&self) -> &[u64] {
        &self.laps
    }

    /// Get last lap time
    pub fn last_lap(&self) -> Option<u64> {
        self.laps.last().copied()
    }
}

impl Default for Stopwatch {
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
    fn test_timer_creation() {
        let config = TimingConfig::default();
        let mut manager = TimingManager::new(config);

        let domain = DomainId::new(1);
        let timer_id = manager.create_timer(
            "test_timer",
            1_000_000, // 1ms
            false,
            "on_timer",
            domain,
        );

        assert!(timer_id > 0);

        let timer = manager.get_timer(timer_id).unwrap();
        assert_eq!(timer.name, "test_timer");
        assert_eq!(timer.duration_ns, 1_000_000);
        assert!(!timer.repeating);
    }

    #[test]
    fn test_deadline() {
        let config = TimingConfig::default();
        let mut manager = TimingManager::new(config);

        let domain = DomainId::new(1);
        let deadline_id = manager.create_deadline_relative(
            "test_deadline",
            100_000_000, // 100ms
            domain,
            false,
        );

        let status = manager.check_deadline(deadline_id).unwrap();
        assert_eq!(status, DeadlineStatus::OnTime);
    }

    #[test]
    fn test_budget() {
        let config = TimingConfig::default();
        let mut manager = TimingManager::new(config);

        let domain = DomainId::new(1);
        let budget_id = manager.create_budget(
            "test_budget",
            10_000_000, // 10ms
            domain,
            None,
        );

        // Use some budget
        assert!(manager.use_budget(budget_id, 5_000_000));
        assert_eq!(manager.remaining_budget(budget_id), 5_000_000);

        // Try to use more than remaining
        assert!(!manager.use_budget(budget_id, 6_000_000));

        // Replenish
        manager.replenish_budget(budget_id);
        assert_eq!(manager.remaining_budget(budget_id), 10_000_000);
    }

    #[test]
    fn test_stopwatch() {
        let mut sw = Stopwatch::new();

        assert!(sw.is_running());

        // Record a lap
        sw.lap();

        // Stop
        sw.stop();
        assert!(!sw.is_running());

        let elapsed = sw.elapsed_ns();

        // Start again
        sw.start();
        assert!(sw.is_running());

        // Elapsed should be >= previous
        assert!(sw.elapsed_ns() >= elapsed);
    }
}
