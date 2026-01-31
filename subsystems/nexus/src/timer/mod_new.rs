//! Timer Intelligence Module
//!
//! AI-powered timer and time management optimization.
//!
//! ## Key Features
//!
//! - **Timer Wheel**: Efficient timer scheduling
//! - **Deadline Prediction**: Predict timer deadlines
//! - **Coalescing**: Intelligent timer coalescing
//! - **Jitter Analysis**: Analyze timer jitter
//! - **Power-Aware Scheduling**: Energy-efficient timer management
//! - **High-Resolution Timers**: Support for hrtimers

mod coalescer;
mod hrtimer;
mod intelligence;
mod jitter;
mod predictor;
mod scheduler;
mod types;
mod wheel;

pub use coalescer::{CoalescedGroup, CoalescingStats, TimerCoalescer};
pub use hrtimer::{HrtimerInfo, HrtimerManager, HrtimerMode, HrtimerStats};
pub use intelligence::TimerIntelligence;
pub use jitter::{JitterAnalyzer, JitterStats};
pub use predictor::{DeadlinePredictor, PatternType, TimerPattern};
pub use scheduler::{
    DecisionType, MigrationReason, PowerAwareScheduler, SchedulingDecision, TimerMigration,
};
pub use types::{CpuId, TimerInfo, TimerId, TimerPriority, TimerState, TimerType};
pub use wheel::TimerWheel;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_info() {
        let timer = TimerInfo::new(1, "test_timer", TimerType::Periodic)
            .with_period(1_000_000)
            .with_priority(TimerPriority::High);

        assert!(timer.is_periodic());
        assert_eq!(timer.priority, TimerPriority::High);
    }

    #[test]
    fn test_timer_wheel() {
        let mut wheel = TimerWheel::new(1_000_000, 4, 6);
        wheel.set_time(0);

        wheel.add(1, 1_000_000);
        wheel.add(2, 2_000_000);
        wheel.add(3, 5_000_000);

        let expired = wheel.advance(3_000_000);
        assert!(expired.contains(&1));
        assert!(expired.contains(&2));
        assert!(!expired.contains(&3));
    }

    #[test]
    fn test_coalescing() {
        let mut coalescer = TimerCoalescer::new(1_000_000);

        coalescer.add(1, 1_000_000, 500_000, TimerPriority::Normal);
        coalescer.add(2, 1_100_000, 500_000, TimerPriority::Normal);
        coalescer.add(3, 1_200_000, 500_000, TimerPriority::Normal);

        let groups = coalescer.coalesce();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].timers.len(), 3);
    }

    #[test]
    fn test_jitter_stats() {
        let mut stats = JitterStats::default();
        stats.min_jitter_ns = i64::MAX;

        stats.record(100);
        stats.record(-50);
        stats.record(200);

        assert_eq!(stats.max_jitter_ns, 200);
        assert_eq!(stats.min_jitter_ns, -50);
    }
}
