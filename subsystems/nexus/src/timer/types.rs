//! Timer Core Types
//!
//! Fundamental types for timer management.

use alloc::string::String;

use crate::core::NexusTimestamp;

/// Timer identifier
pub type TimerId = u64;

/// CPU identifier
pub type CpuId = u32;

/// Timer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    /// One-shot timer
    OneShot,
    /// Periodic timer
    Periodic,
    /// High-resolution timer
    HighRes,
    /// Deferrable timer
    Deferrable,
    /// Pinned timer (CPU affinity)
    Pinned,
    /// Watchdog timer
    Watchdog,
}

impl TimerType {
    /// Is coalescable?
    #[inline(always)]
    pub fn is_coalescable(&self) -> bool {
        matches!(self, Self::OneShot | Self::Periodic | Self::Deferrable)
    }

    /// Is deferrable?
    #[inline(always)]
    pub fn is_deferrable(&self) -> bool {
        matches!(self, Self::Deferrable)
    }

    /// Requires precision?
    #[inline(always)]
    pub fn requires_precision(&self) -> bool {
        matches!(self, Self::HighRes | Self::Watchdog)
    }
}

/// Timer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerState {
    /// Inactive
    Inactive,
    /// Pending (scheduled)
    Pending,
    /// Running (callback executing)
    Running,
    /// Migrating
    Migrating,
    /// Cancelled
    Cancelled,
}

/// Timer priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TimerPriority {
    /// Low priority
    Low = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High = 2,
    /// Critical priority
    Critical = 3,
}

/// Timer information
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct TimerInfo {
    /// Timer ID
    pub id: TimerId,
    /// Name/description
    pub name: String,
    /// Timer type
    pub timer_type: TimerType,
    /// State
    pub state: TimerState,
    /// Priority
    pub priority: TimerPriority,
    /// Deadline (ns since boot)
    pub deadline_ns: u64,
    /// Period (ns, 0 for one-shot)
    pub period_ns: u64,
    /// Slack allowed (ns)
    pub slack_ns: u64,
    /// CPU affinity
    pub cpu: Option<CpuId>,
    /// Created timestamp
    pub created_at: NexusTimestamp,
    /// Callback count
    pub callback_count: u64,
    /// Last callback duration (ns)
    pub last_callback_ns: u64,
}

impl TimerInfo {
    /// Create new timer info
    pub fn new(id: TimerId, name: &str, timer_type: TimerType) -> Self {
        Self {
            id,
            name: String::from(name),
            timer_type,
            state: TimerState::Inactive,
            priority: TimerPriority::Normal,
            deadline_ns: 0,
            period_ns: 0,
            slack_ns: 0,
            cpu: None,
            created_at: NexusTimestamp::now(),
            callback_count: 0,
            last_callback_ns: 0,
        }
    }

    /// Set as periodic
    #[inline(always)]
    pub fn with_period(mut self, period_ns: u64) -> Self {
        self.period_ns = period_ns;
        self
    }

    /// Set slack
    #[inline(always)]
    pub fn with_slack(mut self, slack_ns: u64) -> Self {
        self.slack_ns = slack_ns;
        self
    }

    /// Set priority
    #[inline(always)]
    pub fn with_priority(mut self, priority: TimerPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set CPU affinity
    #[inline(always)]
    pub fn on_cpu(mut self, cpu: CpuId) -> Self {
        self.cpu = Some(cpu);
        self
    }

    /// Is periodic?
    #[inline(always)]
    pub fn is_periodic(&self) -> bool {
        self.period_ns > 0
    }

    /// Is expired?
    #[inline(always)]
    pub fn is_expired(&self, now_ns: u64) -> bool {
        self.deadline_ns <= now_ns
    }

    /// Time until deadline
    #[inline(always)]
    pub fn time_until(&self, now_ns: u64) -> i64 {
        self.deadline_ns as i64 - now_ns as i64
    }

    /// Schedule next occurrence
    #[inline]
    pub fn schedule_next(&mut self, now_ns: u64) {
        if self.is_periodic() {
            self.deadline_ns = now_ns + self.period_ns;
            self.state = TimerState::Pending;
        } else {
            self.state = TimerState::Inactive;
        }
    }

    /// Record callback
    #[inline(always)]
    pub fn record_callback(&mut self, duration_ns: u64) {
        self.callback_count += 1;
        self.last_callback_ns = duration_ns;
    }
}
