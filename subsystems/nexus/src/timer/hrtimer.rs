//! High-Resolution Timer Manager
//!
//! Support for high-resolution timers (hrtimers).

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use super::{TimerId, TimerState};

/// Hrtimer information
#[derive(Debug, Clone)]
pub struct HrtimerInfo {
    /// Timer ID
    pub id: TimerId,
    /// Absolute deadline (ns)
    pub deadline_ns: u64,
    /// Period (ns, 0 for one-shot)
    pub period_ns: u64,
    /// Mode
    pub mode: HrtimerMode,
    /// State
    pub state: TimerState,
}

/// Hrtimer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HrtimerMode {
    /// Absolute time
    Absolute,
    /// Relative time
    Relative,
    /// Pinned (no migration)
    Pinned,
    /// Soft (can be delayed)
    Soft,
}

/// Hrtimer statistics
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HrtimerStats {
    /// Total hrtimers created
    pub total_created: u64,
    /// Total expired
    pub total_expired: u64,
    /// Total cancelled
    pub total_cancelled: u64,
    /// Average latency (ns)
    pub avg_latency_ns: f64,
    /// Maximum latency (ns)
    pub max_latency_ns: u64,
}

/// High-resolution timer management
pub struct HrtimerManager {
    /// Active hrtimers
    timers: BTreeMap<TimerId, HrtimerInfo>,
    /// Sorted by deadline
    deadline_queue: VecDeque<(u64, TimerId)>,
    /// Resolution (ns)
    resolution_ns: u64,
    /// Statistics
    stats: HrtimerStats,
}

impl HrtimerManager {
    /// Create new manager
    pub fn new(resolution_ns: u64) -> Self {
        Self {
            timers: BTreeMap::new(),
            deadline_queue: VecDeque::new(),
            resolution_ns,
            stats: HrtimerStats::default(),
        }
    }

    /// Add hrtimer
    #[inline]
    pub fn add(&mut self, timer: HrtimerInfo) {
        let deadline = timer.deadline_ns;
        let id = timer.id;

        self.timers.insert(id, timer);
        self.deadline_queue.push_back((deadline, id));
        self.deadline_queue.sort_by_key(|(d, _)| *d);

        self.stats.total_created += 1;
    }

    /// Cancel hrtimer
    #[inline]
    pub fn cancel(&mut self, timer_id: TimerId) -> bool {
        if self.timers.remove(&timer_id).is_some() {
            self.deadline_queue.retain(|(_, id)| *id != timer_id);
            self.stats.total_cancelled += 1;
            true
        } else {
            false
        }
    }

    /// Get next deadline
    #[inline(always)]
    pub fn next_deadline(&self) -> Option<u64> {
        self.deadline_queue.first().map(|(d, _)| *d)
    }

    /// Process expired timers
    pub fn process(&mut self, now_ns: u64) -> Vec<TimerId> {
        let mut expired = Vec::new();

        while let Some(&(deadline, id)) = self.deadline_queue.first() {
            if deadline > now_ns {
                break;
            }

            self.deadline_queue.pop_front();
            expired.push(id);

            // Handle periodic timers
            if let Some(timer) = self.timers.get_mut(&id) {
                if timer.period_ns > 0 {
                    timer.deadline_ns = now_ns + timer.period_ns;
                    self.deadline_queue.push_back((timer.deadline_ns, id));
                    self.deadline_queue.sort_by_key(|(d, _)| *d);
                } else {
                    self.timers.remove(&id);
                }
            }

            // Record latency
            let latency = now_ns.saturating_sub(deadline);
            let alpha = 0.1;
            self.stats.avg_latency_ns =
                alpha * latency as f64 + (1.0 - alpha) * self.stats.avg_latency_ns;
            if latency > self.stats.max_latency_ns {
                self.stats.max_latency_ns = latency;
            }
        }

        self.stats.total_expired += expired.len() as u64;
        expired
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &HrtimerStats {
        &self.stats
    }

    /// Get active count
    #[inline(always)]
    pub fn active_count(&self) -> usize {
        self.timers.len()
    }

    /// Get resolution
    #[inline(always)]
    pub fn resolution(&self) -> u64 {
        self.resolution_ns
    }

    /// Get timer info
    #[inline(always)]
    pub fn get_timer(&self, timer_id: TimerId) -> Option<&HrtimerInfo> {
        self.timers.get(&timer_id)
    }
}

impl Default for HrtimerManager {
    fn default() -> Self {
        Self::new(1000) // 1µs default resolution
    }
}
