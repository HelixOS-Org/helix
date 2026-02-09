//! Timer Coalescer
//!
//! Intelligent timer coalescing for power efficiency.

use alloc::vec::Vec;

use super::{TimerId, TimerPriority};

/// Coalescing candidate
#[derive(Debug, Clone)]
struct CoalescingCandidate {
    /// Timer ID
    timer_id: TimerId,
    /// Deadline
    deadline_ns: u64,
    /// Slack
    slack_ns: u64,
    /// Priority
    priority: TimerPriority,
}

/// Coalesced group
#[derive(Debug, Clone)]
pub struct CoalescedGroup {
    /// Group deadline
    pub deadline_ns: u64,
    /// Timer IDs
    pub timers: Vec<TimerId>,
    /// Original earliest deadline
    pub earliest_ns: u64,
    /// Original latest deadline
    pub latest_ns: u64,
}

/// Coalescing statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoalescingStats {
    /// Total timers processed
    pub total_timers: u64,
    /// Coalesced count
    pub coalesced: u64,
    /// Groups created
    pub groups_created: u64,
    /// Average group size
    pub avg_group_size: f64,
    /// Total delay added (ns)
    pub total_delay_ns: u64,
}

impl CoalescingStats {
    /// Coalescing ratio
    #[inline]
    pub fn ratio(&self) -> f64 {
        if self.total_timers == 0 {
            0.0
        } else {
            self.coalesced as f64 / self.total_timers as f64
        }
    }
}

/// Intelligent timer coalescing
#[repr(align(64))]
pub struct TimerCoalescer {
    /// Coalescing window (ns)
    window_ns: u64,
    /// Pending timers
    pending: Vec<CoalescingCandidate>,
    /// Coalesced groups
    groups: Vec<CoalescedGroup>,
    /// Coalescing stats
    stats: CoalescingStats,
}

impl TimerCoalescer {
    /// Create new coalescer
    pub fn new(window_ns: u64) -> Self {
        Self {
            window_ns,
            pending: Vec::new(),
            groups: Vec::new(),
            stats: CoalescingStats::default(),
        }
    }

    /// Add timer for potential coalescing
    #[inline]
    pub fn add(
        &mut self,
        timer_id: TimerId,
        deadline_ns: u64,
        slack_ns: u64,
        priority: TimerPriority,
    ) {
        self.pending.push(CoalescingCandidate {
            timer_id,
            deadline_ns,
            slack_ns,
            priority,
        });
        self.stats.total_timers += 1;
    }

    /// Process pending timers and create groups
    pub fn coalesce(&mut self) -> Vec<CoalescedGroup> {
        if self.pending.is_empty() {
            return Vec::new();
        }

        // Sort by deadline
        self.pending.sort_by_key(|c| c.deadline_ns);

        let mut groups = Vec::new();
        let mut current_group: Option<CoalescedGroup> = None;

        for candidate in self.pending.drain(..) {
            if let Some(ref mut group) = current_group {
                // Check if can coalesce
                let can_coalesce = candidate.deadline_ns <= group.latest_ns + self.window_ns
                    && candidate.deadline_ns
                        >= group.deadline_ns.saturating_sub(candidate.slack_ns);

                if can_coalesce {
                    // Add to group
                    group.timers.push(candidate.timer_id);
                    group.latest_ns = group.latest_ns.max(candidate.deadline_ns);
                    // Adjust group deadline if needed
                    if candidate.priority >= TimerPriority::High {
                        group.deadline_ns = group.deadline_ns.min(candidate.deadline_ns);
                    }
                    self.stats.coalesced += 1;
                } else {
                    // Start new group
                    groups.push(current_group.take().unwrap());
                    current_group = Some(CoalescedGroup {
                        deadline_ns: candidate.deadline_ns,
                        timers: vec![candidate.timer_id],
                        earliest_ns: candidate.deadline_ns,
                        latest_ns: candidate.deadline_ns + candidate.slack_ns,
                    });
                }
            } else {
                // Start first group
                current_group = Some(CoalescedGroup {
                    deadline_ns: candidate.deadline_ns,
                    timers: vec![candidate.timer_id],
                    earliest_ns: candidate.deadline_ns,
                    latest_ns: candidate.deadline_ns + candidate.slack_ns,
                });
            }
        }

        if let Some(group) = current_group {
            groups.push(group);
        }

        // Update stats
        self.stats.groups_created += groups.len() as u64;
        if !groups.is_empty() {
            let total_timers: usize = groups.iter().map(|g| g.timers.len()).sum();
            let alpha = 0.1;
            self.stats.avg_group_size = alpha * (total_timers as f64 / groups.len() as f64)
                + (1.0 - alpha) * self.stats.avg_group_size;
        }

        self.groups = groups.clone();
        groups
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &CoalescingStats {
        &self.stats
    }

    /// Set window
    #[inline(always)]
    pub fn set_window(&mut self, window_ns: u64) {
        self.window_ns = window_ns;
    }

    /// Get current groups
    #[inline(always)]
    pub fn current_groups(&self) -> &[CoalescedGroup] {
        &self.groups
    }
}

impl Default for TimerCoalescer {
    fn default() -> Self {
        Self::new(1_000_000) // 1ms default window
    }
}
