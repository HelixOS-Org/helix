//! RCU (Read-Copy-Update) Intelligence Module
//!
//! This module provides AI-powered RCU analysis and optimization for the NEXUS subsystem.
//! It includes grace period prediction, callback coalescing, reader tracking, memory
//! pressure analysis, and adaptive RCU configuration.
//!
//! ## Architecture
//!
//! The module is organized into focused submodules:
//!
//! - [`types`] - Core identifiers and enumerations
//! - [`grace_period`] - Grace period tracking and statistics
//! - [`predictor`] - Grace period duration prediction
//! - [`callback`] - Callback management and coalescing
//! - [`reader`] - Reader tracking for quiescent state detection
//! - [`pressure`] - Memory pressure analysis
//! - [`config`] - Adaptive RCU configuration
//! - [`intelligence`] - Comprehensive RCU analysis

// Submodules
pub mod callback;
pub mod config;
pub mod grace_period;
pub mod intelligence;
pub mod predictor;
pub mod pressure;
pub mod reader;
pub mod types;

// Re-export core types
// Re-export callback
pub use callback::{CallbackBatch, CallbackCoalescer, CallbackInfo, CallbackPriority};
// Re-export config
pub use config::{AdaptiveConfigurator, ConfigRecommendation, RcuConfig, RcuConfigParam};
// Re-export grace period
pub use grace_period::{GracePeriodInfo, GracePeriodStats};
// Re-export intelligence
pub use intelligence::{RcuAnalysis, RcuDomainInfo, RcuIntelligence, RcuIssue, RcuIssueType};
// Re-export predictor
pub use predictor::{GpSample, GracePeriodPredictor};
// Re-export pressure
pub use pressure::{MemoryPressureAnalyzer, MemoryPressureLevel, MemoryPressureSample};
// Re-export reader
pub use reader::{ReaderInfo, ReaderTracker};
pub use types::{CallbackId, CpuId, GracePeriodId, RcuDomainId, RcuDomainState, RcuFlavor};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use alloc::string::String;

    use super::*;

    #[test]
    fn test_rcu_domain_id() {
        let id = RcuDomainId::new(42);
        assert_eq!(id.raw(), 42);
    }

    #[test]
    fn test_rcu_flavor() {
        assert_eq!(RcuFlavor::Classic.name(), "rcu");
        assert!(!RcuFlavor::Classic.supports_sleeping());
        assert!(RcuFlavor::Srcu.supports_sleeping());
    }

    #[test]
    fn test_grace_period_predictor() {
        let mut predictor = GracePeriodPredictor::new(RcuDomainId::new(1));

        // Record some samples
        let mut gp = GracePeriodInfo::new(GracePeriodId::new(1), RcuDomainId::new(1), 0);
        gp.end_ns = Some(10_000_000); // 10ms
        predictor.record_sample(&gp, 4, 100);

        // Predict should be close to recorded
        let predicted = predictor.predict_duration(false, 4);
        assert!(predicted > 0);
    }

    #[test]
    fn test_callback_coalescer() {
        let mut coalescer = CallbackCoalescer::new();

        // Add callbacks
        for i in 0..10 {
            let callback = CallbackInfo {
                id: CallbackId::new(i),
                registered_ns: i * 1000,
                target_gp: None,
                source_cpu: CpuId::new(0),
                priority: CallbackPriority::Normal,
                estimated_exec_ns: 100,
                memory_bytes: 1024,
                function_name: String::from("test_cb"),
            };
            coalescer.add_callback(callback);
        }

        assert_eq!(coalescer.current_batch_size(), 10);

        // Flush batch
        let batch = coalescer.flush_batch(GracePeriodId::new(1), 100000);
        assert!(batch.is_some());
        assert_eq!(batch.unwrap().callbacks.len(), 10);
    }

    #[test]
    fn test_reader_tracker() {
        let mut tracker = ReaderTracker::new();

        let cpu = CpuId::new(0);
        tracker.register_cpu(cpu);

        // Enter critical section
        tracker.record_cs_entry(cpu, 1000);
        assert!(tracker.get_reader(cpu).unwrap().in_critical_section());

        // Exit critical section
        tracker.record_cs_exit(cpu, 2000);
        assert!(!tracker.get_reader(cpu).unwrap().in_critical_section());
    }

    #[test]
    fn test_memory_pressure_analyzer() {
        let mut analyzer = MemoryPressureAnalyzer::new(RcuDomainId::new(1));

        // Record low pressure sample
        analyzer.record_sample(MemoryPressureSample {
            timestamp_ns: 1000,
            pending_callbacks: 10,
            pending_memory_bytes: 1024,
            gp_rate: 1.0,
            callback_rate: 10.0,
        });
        assert_eq!(analyzer.current_level(), MemoryPressureLevel::Low);

        // Record high pressure sample
        analyzer.record_sample(MemoryPressureSample {
            timestamp_ns: 2000,
            pending_callbacks: 100000,
            pending_memory_bytes: 1024 * 1024 * 100,
            gp_rate: 1.0,
            callback_rate: 10.0,
        });
        assert!(analyzer.current_level() >= MemoryPressureLevel::High);
    }

    #[test]
    fn test_rcu_intelligence() {
        let mut intel = RcuIntelligence::new();

        let domain_id = RcuDomainId::new(1);
        intel.register_domain(domain_id, String::from("rcu"), RcuFlavor::Classic);

        let cpu = CpuId::new(0);
        intel.register_cpu(cpu);

        // Start grace period
        let gp_id = GracePeriodId::new(1);
        intel.start_grace_period(domain_id, gp_id, 0, false);

        // Record quiescent state
        intel.record_quiescent_state(domain_id, cpu, 1000);

        // Complete grace period
        intel.complete_grace_period(domain_id, 10_000_000);

        assert_eq!(intel.total_gps_completed(), 1);
    }

    #[test]
    fn test_adaptive_configurator() {
        let mut configurator = AdaptiveConfigurator::new();

        assert!(configurator.is_auto_tune());

        let config = configurator.config();
        assert_eq!(config.jiffies_till_first_fqs, 3);

        // Apply a change
        configurator.apply_recommendation(RcuConfigParam::JiffiesTillFirstFqs, 2, 1000);
        assert_eq!(configurator.config().jiffies_till_first_fqs, 2);

        // Rollback
        assert!(configurator.rollback());
        assert_eq!(configurator.config().jiffies_till_first_fqs, 3);
    }

    #[test]
    fn test_grace_period_info() {
        let mut gp = GracePeriodInfo::new(GracePeriodId::new(1), RcuDomainId::new(1), 1000);

        assert!(!gp.is_completed());
        assert_eq!(gp.duration_ns(), None);

        gp.end_ns = Some(2000);
        assert!(gp.is_completed());
        assert_eq!(gp.duration_ns(), Some(1000));
    }
}
