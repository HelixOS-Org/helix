//! NEXUS event kinds

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

use super::types::AnomalyEventKind;
use crate::core::ComponentId;

// ============================================================================
// EVENT KIND
// ============================================================================

/// Kind of NEXUS event
#[derive(Debug, Clone, PartialEq)]
pub enum NexusEventKind {
    // === System Events ===
    /// System initialization
    SystemInit,
    /// System shutdown
    SystemShutdown,
    /// Tick event (periodic)
    Tick { cycle: u64 },
    /// Heartbeat
    Heartbeat,

    // === Prediction Events ===
    /// Crash predicted
    CrashPredicted {
        confidence: f32,
        time_to_crash_ms: u64,
        component: ComponentId,
    },
    /// Degradation detected
    DegradationDetected {
        component: ComponentId,
        severity: f32,
        pattern: String,
    },
    /// Memory leak detected
    MemoryLeakDetected {
        component: ComponentId,
        leak_rate_bytes_per_sec: u64,
    },
    /// Deadlock predicted
    DeadlockPredicted {
        components: Vec<ComponentId>,
        confidence: f32,
    },
    /// Resource exhaustion predicted
    ResourceExhaustionPredicted {
        resource: String,
        time_to_exhaustion_ms: u64,
    },

    // === Anomaly Events ===
    /// Anomaly detected
    AnomalyDetected {
        kind: AnomalyEventKind,
        severity: f32,
        component: Option<ComponentId>,
    },
    /// Pattern deviation
    PatternDeviation {
        expected: String,
        actual: String,
        deviation: f32,
    },

    // === Healing Events ===
    /// Healing started
    HealingStarted {
        component: ComponentId,
        strategy: String,
    },
    /// Healing completed
    HealingCompleted {
        component: ComponentId,
        success: bool,
        duration_ms: u64,
    },
    /// Rollback started
    RollbackStarted {
        component: ComponentId,
        checkpoint_id: u64,
    },
    /// Rollback completed
    RollbackCompleted {
        component: ComponentId,
        success: bool,
    },
    /// Component quarantined
    ComponentQuarantined {
        component: ComponentId,
        reason: String,
    },
    /// Component restored
    ComponentRestored { component: ComponentId },

    // === Component Events ===
    /// Component registered
    ComponentRegistered {
        component: ComponentId,
        name: String,
    },
    /// Component health changed
    ComponentHealthChanged {
        component: ComponentId,
        old_health: f32,
        new_health: f32,
    },
    /// Component error
    ComponentError {
        component: ComponentId,
        error: String,
    },
    /// Component panic
    ComponentPanic {
        component: ComponentId,
        message: String,
    },

    // === Tracing Events ===
    /// Span started
    SpanStarted {
        span_id: u64,
        parent_id: Option<u64>,
        name: String,
    },
    /// Span ended
    SpanEnded { span_id: u64, duration_ns: u64 },
    /// Causal link established
    CausalLinkEstablished { cause: u64, effect: u64 },

    // === Performance Events ===
    /// Performance anomaly
    PerformanceAnomaly {
        metric: String,
        expected: f64,
        actual: f64,
    },
    /// Latency spike
    LatencySpike {
        component: ComponentId,
        latency_us: u64,
        threshold_us: u64,
    },
    /// Throughput drop
    ThroughputDrop {
        component: ComponentId,
        drop_percent: f32,
    },

    // === Chaos Events ===
    /// Fault injected
    FaultInjected { kind: String, target: ComponentId },
    /// Fault recovered
    FaultRecovered {
        kind: String,
        target: ComponentId,
        recovery_time_ms: u64,
    },

    // === Custom Event ===
    /// Custom event with payload
    Custom { name: String, payload: Vec<u8> },
}
