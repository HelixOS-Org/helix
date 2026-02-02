//! Interrupt Intelligence Module
//!
//! This module provides intelligent interrupt analysis and optimization including:
//! - Interrupt pattern detection (periodic, burst, random)
//! - Interrupt storm detection and mitigation
//! - NUMA-aware IRQ affinity optimization
//! - Adaptive interrupt coalescing
//!
//! # Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Core types (Irq, CpuId, InterruptType, Priority, DeliveryMode)
//! - `record`: Interrupt record tracking
//! - `stats`: IRQ statistics collection
//! - `pattern`: Pattern detection algorithms
//! - `storm`: Storm detection and handling
//! - `affinity`: NUMA-aware affinity optimization
//! - `coalescing`: Adaptive coalescing optimization
//! - `intelligence`: Central coordinator

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod affinity;
pub mod coalescing;
pub mod intelligence;
pub mod pattern;
pub mod record;
pub mod stats;
pub mod storm;
pub mod types;

// Re-export core types
// Re-export affinity types
pub use affinity::{AffinityChange, AffinityOptimizer};
// Re-export coalescing types
pub use coalescing::{CoalescingMetrics, CoalescingOptimizer, CoalescingSettings};
// Re-export intelligence types
pub use intelligence::InterruptIntelligence;
// Re-export pattern types
pub use pattern::{InterruptPattern, InterruptPatternDetector};
// Re-export record types
pub use record::InterruptRecord;
// Re-export stats types
pub use stats::IrqStats;
// Re-export storm types
pub use storm::{StormDetector, StormEvent, StormEventType, StormInfo};
pub use types::{CpuId, DeliveryMode, InterruptPriority, InterruptType, Irq};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_record() {
        let record = InterruptRecord::new(10, InterruptType::Device, 0).with_latency(1000);
        assert_eq!(record.irq, 10);
        assert_eq!(record.latency_ns, 1000);
    }

    #[test]
    fn test_irq_stats() {
        let mut stats = IrqStats::new();

        for cpu in 0..4 {
            let record = InterruptRecord::new(1, InterruptType::Timer, cpu);
            stats.record(&record);
        }

        assert_eq!(stats.total, 4);
        assert_eq!(stats.per_cpu.len(), 4);
    }

    #[test]
    fn test_pattern_detector() {
        let mut detector = InterruptPatternDetector::default();

        // Simulate periodic interrupts
        for i in 0..100 {
            detector.record(1, i * 1_000_000);
        }

        assert_eq!(
            detector.get_pattern(1),
            Some(InterruptPattern::Periodic {
                period_ns: 1_000_000
            })
        );
    }

    #[test]
    fn test_storm_detector() {
        let mut detector = StormDetector::new(100, 1000);

        // Generate storm
        for _ in 0..150 {
            detector.record(1, 0);
        }

        assert!(detector.is_storm_active(1));
    }

    #[test]
    fn test_coalescing_settings() {
        let settings = CoalescingSettings::default();
        assert!(settings.adaptive);
        assert!(settings.max_delay_us > 0);
    }
}
