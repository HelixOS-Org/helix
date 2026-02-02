//! # NEXUS Sense Domain — Cognitive Layer 1
//!
//! The first cognitive domain. SENSE captures raw signals from the kernel
//! and transforms them into normalized, actionable data for higher cognitive layers.
//!
//! # Philosophy
//!
//! "Percevoir avant de comprendre" — Perceive before understanding
//!
//! SENSE is the eyes and ears of NEXUS. It must:
//! - Capture kernel events with minimal overhead
//! - Normalize heterogeneous signals into a common format
//! - Filter noise and prioritize significant events
//! - Provide real-time streaming to UNDERSTAND domain
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                            SENSE DOMAIN                                  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                      KERNEL EVENTS                           │       │
//! │  │  (syscalls, interrupts, memory, scheduling, I/O, ...)        │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                        PROBES                                │       │
//! │  │  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐              │       │
//! │  │  │ CPU  │ │Memory│ │ I/O  │ │Sched │ │ Net  │ ...          │       │
//! │  │  └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘ └──┬───┘              │       │
//! │  │     └────────┴────────┴────────┴────────┘                   │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                      COLLECTOR                               │       │
//! │  │  • Ring buffer for events                                    │       │
//! │  │  • Priority-based queuing                                    │       │
//! │  │  • Overflow handling                                         │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                      NORMALIZER                              │       │
//! │  │  • Convert to unified Signal format                          │       │
//! │  │  • Apply noise filtering                                     │       │
//! │  │  • Compute derived metrics                                   │       │
//! │  └───────────────────────────┬──────────────────────────────────┘       │
//! │                              │                                          │
//! │                              ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────┐       │
//! │  │                    OUTPUT SIGNALS                            │       │
//! │  │  → Signal → Signal → Signal → ...                           │       │
//! │  │  To: UNDERSTAND domain                                       │       │
//! │  └──────────────────────────────────────────────────────────────┘       │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Module Structure
//!
//! - [`events`] - Raw kernel event types
//! - [`probe`] - Probe trait and base implementation
//! - [`probes`] - Concrete probe implementations (CPU, Memory, I/O, etc.)
//! - [`collector`] - Event collection and buffering
//! - [`signal`] - Normalized signal types and transformation
//! - [`registry`] - Probe registration and management
//! - [`domain`] - Main sense domain orchestrator

#![allow(dead_code)]

extern crate alloc;

// Submodules
pub mod collector;
pub mod domain;
pub mod events;
pub mod probe;
pub mod probes;
pub mod registry;
pub mod signal;

// Re-exports - Events
// Re-exports - Collector
pub use collector::{CollectorStats, EventCollector, EventCollectorConfig};
// Re-exports - Domain
pub use domain::{SenseConfig, SenseDomain};
// Re-exports - Events (using actual types from events.rs)
pub use events::{
    BlockIoEvent, CpuSample, EventData, IoOperation, MemorySample, NetworkDirection,
    NetworkIoEvent, NetworkProtocol, RawEvent, SchedulerEventData, SchedulerEventType,
};
// Re-exports - Probe
pub use probe::{Probe, ProbeConfig, ProbeError, ProbeState, ProbeStats};
// Re-exports - Probes
pub use probes::{CpuProbe, MemoryProbe};
// Re-exports - Registry
pub use registry::ProbeRegistry;
// Re-exports - Signal
pub use signal::{
    NormalizerStats, Signal, SignalMetadata, SignalNormalizer, SignalType, SignalValue,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Timestamp;

    #[test]
    fn test_cpu_event() {
        let event = CpuEvent::new(0, 75, 50, Timestamp::now());
        assert_eq!(event.cpu_id, 0);
        assert_eq!(event.utilization, 75);
    }

    #[test]
    fn test_memory_event() {
        let event = MemoryEvent::new(1024 * 1024 * 1024, 512 * 1024 * 1024, Timestamp::now());
        assert_eq!(event.total_bytes, 1024 * 1024 * 1024);
        assert_eq!(event.used_bytes, 512 * 1024 * 1024);
    }

    #[test]
    fn test_probe_state() {
        assert!(ProbeState::Active.is_active());
        assert!(!ProbeState::Paused.is_active());
        assert!(!ProbeState::Failed.is_active());
    }

    #[test]
    fn test_signal_priority() {
        assert!(SignalPriority::Critical > SignalPriority::High);
        assert!(SignalPriority::High > SignalPriority::Normal);
        assert!(SignalPriority::Normal > SignalPriority::Low);
    }

    #[test]
    fn test_collector_config() {
        let config = EventCollectorConfig::default();
        assert!(config.buffer_size > 0);
        assert!(config.max_events_per_tick > 0);
    }

    #[test]
    fn test_sense_domain() {
        let config = SenseConfig::minimal();
        let domain = SenseDomain::new(config);
        assert!(domain.is_ok());

        let mut domain = domain.unwrap();
        assert_eq!(domain.probe_count(), 0);

        let stats = domain.stats();
        assert_eq!(stats.events_collected, 0);
    }

    #[test]
    fn test_probe_registry() {
        let mut registry = ProbeRegistry::new(16);
        assert_eq!(registry.count(), 0);

        // Registry operations tested in registry module
    }

    #[test]
    fn test_signal_normalizer() {
        let normalizer = SignalNormalizer::new();
        let stats = normalizer.stats();
        assert_eq!(stats.signals_processed, 0);
    }
}
