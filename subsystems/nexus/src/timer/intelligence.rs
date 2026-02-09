//! Timer Intelligence
//!
//! Central coordinator for all timer analysis components.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{
    CoalescedGroup, DeadlinePredictor, HrtimerInfo, HrtimerManager, HrtimerMode,
    JitterAnalyzer, PowerAwareScheduler, TimerCoalescer, TimerId, TimerInfo, TimerState,
    TimerType, TimerWheel,
};

/// Central timer intelligence coordinator
#[repr(align(64))]
pub struct TimerIntelligence {
    /// Registered timers
    timers: BTreeMap<TimerId, TimerInfo>,
    /// Timer wheel
    wheel: TimerWheel,
    /// Deadline predictor
    predictor: DeadlinePredictor,
    /// Timer coalescer
    coalescer: TimerCoalescer,
    /// Jitter analyzer
    jitter: JitterAnalyzer,
    /// Power-aware scheduler
    scheduler: PowerAwareScheduler,
    /// Hrtimer manager
    hrtimer: HrtimerManager,
    /// Total timer operations
    total_ops: AtomicU64,
}

impl TimerIntelligence {
    /// Create new timer intelligence
    pub fn new() -> Self {
        Self {
            timers: BTreeMap::new(),
            wheel: TimerWheel::default(),
            predictor: DeadlinePredictor::default(),
            coalescer: TimerCoalescer::default(),
            jitter: JitterAnalyzer::default(),
            scheduler: PowerAwareScheduler::default(),
            hrtimer: HrtimerManager::default(),
            total_ops: AtomicU64::new(0),
        }
    }

    /// Register timer
    #[inline(always)]
    pub fn register(&mut self, info: TimerInfo) {
        self.total_ops.fetch_add(1, Ordering::Relaxed);
        self.timers.insert(info.id, info);
    }

    /// Schedule timer
    pub fn schedule(&mut self, timer_id: TimerId, deadline_ns: u64) {
        if let Some(timer) = self.timers.get_mut(&timer_id) {
            timer.deadline_ns = deadline_ns;
            timer.state = TimerState::Pending;

            // Add to appropriate structure
            if timer.timer_type == TimerType::HighRes {
                self.hrtimer.add(HrtimerInfo {
                    id: timer_id,
                    deadline_ns,
                    period_ns: timer.period_ns,
                    mode: HrtimerMode::Absolute,
                    state: TimerState::Pending,
                });
            } else {
                self.wheel.add(timer_id, deadline_ns);
            }

            // Consider coalescing
            if timer.timer_type.is_coalescable() {
                self.coalescer
                    .add(timer_id, deadline_ns, timer.slack_ns, timer.priority);
            }
        }
    }

    /// Process expired timers
    pub fn process(&mut self, now_ns: u64) -> Vec<TimerId> {
        let mut expired = Vec::new();

        // Process timer wheel
        let wheel_expired = self.wheel.advance(now_ns);
        for id in wheel_expired {
            if let Some(timer) = self.timers.get_mut(&id) {
                timer.state = TimerState::Running;

                // Record jitter
                self.jitter.record(id, timer.deadline_ns, now_ns);

                // Record for prediction
                self.predictor.record(id, timer.deadline_ns, now_ns);

                // Schedule next if periodic
                timer.schedule_next(now_ns);
            }
            expired.push(id);
        }

        // Process hrtimers
        let hrtimer_expired = self.hrtimer.process(now_ns);
        expired.extend(hrtimer_expired);

        expired
    }

    /// Cancel timer
    #[inline]
    pub fn cancel(&mut self, timer_id: TimerId) {
        if let Some(timer) = self.timers.get_mut(&timer_id) {
            timer.state = TimerState::Cancelled;
        }
        self.hrtimer.cancel(timer_id);
    }

    /// Get timer info
    #[inline(always)]
    pub fn get_timer(&self, timer_id: TimerId) -> Option<&TimerInfo> {
        self.timers.get(&timer_id)
    }

    /// Get predictor
    #[inline(always)]
    pub fn predictor(&self) -> &DeadlinePredictor {
        &self.predictor
    }

    /// Get coalescer
    #[inline(always)]
    pub fn coalescer(&self) -> &TimerCoalescer {
        &self.coalescer
    }

    /// Get mutable coalescer
    #[inline(always)]
    pub fn coalescer_mut(&mut self) -> &mut TimerCoalescer {
        &mut self.coalescer
    }

    /// Coalesce pending timers
    #[inline(always)]
    pub fn coalesce(&mut self) -> Vec<CoalescedGroup> {
        self.coalescer.coalesce()
    }

    /// Get jitter analyzer
    #[inline(always)]
    pub fn jitter(&self) -> &JitterAnalyzer {
        &self.jitter
    }

    /// Get scheduler
    #[inline(always)]
    pub fn scheduler(&self) -> &PowerAwareScheduler {
        &self.scheduler
    }

    /// Get mutable scheduler
    #[inline(always)]
    pub fn scheduler_mut(&mut self) -> &mut PowerAwareScheduler {
        &mut self.scheduler
    }

    /// Get hrtimer manager
    #[inline(always)]
    pub fn hrtimer(&self) -> &HrtimerManager {
        &self.hrtimer
    }

    /// Get total operations
    #[inline(always)]
    pub fn total_ops(&self) -> u64 {
        self.total_ops.load(Ordering::Relaxed)
    }

    /// Get active timer count
    #[inline]
    pub fn active_count(&self) -> usize {
        self.timers
            .values()
            .filter(|t| t.state == TimerState::Pending)
            .count()
    }
}

impl Default for TimerIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
