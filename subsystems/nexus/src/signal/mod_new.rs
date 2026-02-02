//! Signal Intelligence Module
//!
//! This module provides AI-powered signal handling analysis and optimization for the NEXUS subsystem.
//! It includes signal pattern detection, handler profiling, delivery optimization, signal queue
//! management, and intelligent signal coalescing.
//!
//! ## Modules
//!
//! - [`types`] - Core signal types (ProcessId, ThreadId, SignalNumber, etc.)
//! - [`info`] - Signal information structures
//! - [`pattern`] - Signal pattern detection
//! - [`profiler`] - Handler execution profiling
//! - [`queue`] - Signal queue management
//! - [`delivery`] - Delivery optimization
//! - [`analysis`] - Analysis results and recommendations
//! - [`intelligence`] - Main intelligence engine

#![no_std]

extern crate alloc;
use alloc::vec;

pub mod types;
pub mod info;
pub mod pattern;
pub mod profiler;
pub mod queue;
pub mod delivery;
pub mod analysis;
pub mod intelligence;

// Re-export types
pub use types::{
    ProcessId, ThreadId, SignalNumber, SignalCategory, SignalAction, DeliveryState,
};

// Re-export info
pub use info::{SignalInfo, PendingSignal};

// Re-export pattern
pub use pattern::{SignalPattern, PatternType, SignalPatternDetector};

// Re-export profiler
pub use profiler::{HandlerSample, HandlerStats, HandlerProfiler};

// Re-export queue
pub use queue::{QueueStats, SignalQueueManager};

// Re-export delivery
pub use delivery::{DeliveryRecommendation, DeliveryOptimizer};

// Re-export analysis
pub use analysis::{SignalAnalysis, SignalIssue, SignalIssueType, SignalRecommendation};

// Re-export intelligence
pub use intelligence::SignalIntelligence;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_number() {
        assert_eq!(SignalNumber::SIGTERM.raw(), 15);
        assert_eq!(SignalNumber::SIGTERM.name(), "SIGTERM");
        assert!(SignalNumber::SIGKILL.is_fatal());
        assert!(!SignalNumber::SIGKILL.can_catch());
    }

    #[test]
    fn test_signal_category() {
        assert_eq!(
            SignalCategory::from_signal(SignalNumber::SIGTERM),
            SignalCategory::Termination
        );
        assert_eq!(
            SignalCategory::from_signal(SignalNumber::SIGSEGV),
            SignalCategory::Error
        );
        assert_eq!(
            SignalCategory::from_signal(SignalNumber::SIGCHLD),
            SignalCategory::Control
        );
    }

    #[test]
    fn test_pattern_detector() {
        let mut detector = SignalPatternDetector::new();

        let sender = ProcessId::new(1);
        let receiver = ProcessId::new(2);

        // Simulate signal burst
        for i in 0..15 {
            detector.record_event(SignalNumber::SIGUSR1, sender, receiver, i * 1000);
        }

        // Should detect burst pattern
        let patterns = detector.get_patterns();
        assert!(!patterns.is_empty());
    }

    #[test]
    fn test_handler_profiler() {
        let mut profiler = HandlerProfiler::new();

        let pid = ProcessId::new(1);
        let signo = SignalNumber::SIGTERM;

        // Record handler execution
        profiler.record_entry(pid, signo);
        profiler.record_exit(pid, signo, 5_000_000, false, 1000);

        let stats = profiler.get_stats(signo).unwrap();
        assert_eq!(stats.execution_count, 1);
        assert!(stats.avg_time_ns() > 0);
    }

    #[test]
    fn test_queue_manager() {
        let mut manager = SignalQueueManager::new(10);

        let pid = ProcessId::new(1);
        let info = SignalInfo::new(SignalNumber::SIGTERM, ProcessId::new(0), 1000);
        let signal = PendingSignal::new(info, pid);

        assert!(manager.enqueue(signal));
        assert_eq!(manager.pending_count(pid), 1);

        let dequeued = manager.dequeue(pid);
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().info.signo, SignalNumber::SIGTERM);
    }

    #[test]
    fn test_signal_coalescing() {
        let mut manager = SignalQueueManager::new(10);
        manager.set_coalescable_signals(alloc::vec![SignalNumber::SIGCHLD]);

        let pid = ProcessId::new(1);

        // Send multiple SIGCHLD
        for i in 0..5 {
            let info = SignalInfo::new(SignalNumber::SIGCHLD, ProcessId::new(0), i * 1000);
            let signal = PendingSignal::new(info, pid);
            manager.enqueue(signal);
        }

        // Should be coalesced to 1
        assert_eq!(manager.pending_count(pid), 1);
        assert!(manager.global_stats().coalesced > 0);
    }

    #[test]
    fn test_signal_intelligence() {
        let mut intel = SignalIntelligence::new();

        let sender = ProcessId::new(1);
        let receiver = ProcessId::new(2);

        // Send signal
        intel.send_signal(sender, receiver, SignalNumber::SIGTERM, 1000);

        // Deliver
        let signal = intel.deliver_signal(receiver);
        assert!(signal.is_some());

        // Complete
        intel.complete_delivery(receiver, SignalNumber::SIGTERM, 1_000_000, false, 2000);

        assert_eq!(intel.total_sent(), 1);
        assert_eq!(intel.total_delivered(), 1);
    }
}
