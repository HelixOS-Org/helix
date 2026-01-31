//! # CORTEX Architecture Integration
//!
//! This module integrates all CORTEX components into a unified intelligence
//! framework. It provides:
//!
//! - **Unified API**: Single entry point for kernel intelligence
//! - **Component coordination**: Ensures all subsystems work together
//! - **Lifecycle management**: Proper initialization and shutdown
//! - **Health monitoring**: Continuous self-monitoring
//!
//! ## Integration Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         CORTEX CORE                                     │
//! │                                                                         │
//! │  ┌─────────────────────────────────────────────────────────────────┐   │
//! │  │                    Event Bus (CortexBus)                         │   │
//! │  └─────────────────────────────────────────────────────────────────┘   │
//! │       ▲            ▲             ▲             ▲            ▲          │
//! │       │            │             │             │            │          │
//! │  ┌────┴────┐  ┌────┴────┐  ┌────┴────┐  ┌────┴────┐  ┌────┴────┐     │
//! │  │Conscious│  │ Neural  │  │Temporal │  │Survive  │  │  Meta   │     │
//! │  │  ness   │  │ Engine  │  │ Kernel  │  │ -ability│  │ Kernel  │     │
//! │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────┘     │
//! │       ▲            ▲             ▲             ▲            ▲          │
//! │       │            │             │             │            │          │
//! │  ┌────┴────┐  ┌────┴────┐  ┌────┴────┐  ┌────┴────┐  ┌────┴────┐     │
//! │  │ Formal  │  │ Policy  │  │Learning │  │Telemetry│  │ Config  │     │
//! │  │ Verify  │  │ Engine  │  │         │  │         │  │         │     │
//! │  └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────┘     │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::bus::{CortexBus, CortexEvent, EventCategory, EventHandler, HandlerId};
use crate::consciousness::StructuralConsciousness;
use crate::formal::VerificationEngine;
use crate::learning::{AdaptiveLearner, LearningConfig};
use crate::meta::MetaKernel;
use crate::neural::NeuralEngine;
use crate::policy::{PolicyEngine, PolicyEngineConfig};
use crate::survivability::SurvivabilityCore;
use crate::telemetry::TelemetryCollector;
use crate::temporal::TemporalKernel;
use crate::{CortexResult, DecisionAction, IntelligenceLevel, SubsystemId, Timestamp};

// =============================================================================
// CORTEX CONFIGURATION
// =============================================================================

/// Complete CORTEX configuration
#[derive(Clone)]
pub struct CortexConfig {
    /// Intelligence level
    pub level: IntelligenceLevel,

    /// Memory budget (bytes)
    pub memory_budget: usize,

    /// CPU budget (percent)
    pub cpu_budget_percent: u8,

    /// Decision timeout (microseconds)
    pub decision_timeout_us: u64,

    /// Event queue size
    pub event_queue_size: usize,

    /// Maximum event handlers
    pub max_handlers: usize,

    /// Enable consciousness layer
    pub consciousness_enabled: bool,

    /// Enable neural engine
    pub neural_enabled: bool,

    /// Enable temporal kernel
    pub temporal_enabled: bool,

    /// Enable survivability core
    pub survivability_enabled: bool,

    /// Enable meta-kernel
    pub meta_enabled: bool,

    /// Enable formal verification
    pub formal_enabled: bool,

    /// Enable adaptive learning
    pub learning_enabled: bool,

    /// Enable policy engine
    pub policy_enabled: bool,

    /// Enable telemetry
    pub telemetry_enabled: bool,

    /// Learning configuration
    pub learning_config: LearningConfig,

    /// Policy engine configuration
    pub policy_config: PolicyEngineConfig,
}

impl Default for CortexConfig {
    fn default() -> Self {
        Self {
            level: IntelligenceLevel::Monitoring,
            memory_budget: 64 * 1024 * 1024, // 64 MB
            cpu_budget_percent: 5,
            decision_timeout_us: 1000,
            event_queue_size: 1024,
            max_handlers: 32,
            consciousness_enabled: true,
            neural_enabled: true,
            temporal_enabled: true,
            survivability_enabled: true,
            meta_enabled: true,
            formal_enabled: true,
            learning_enabled: true,
            policy_enabled: true,
            telemetry_enabled: true,
            learning_config: LearningConfig::default(),
            policy_config: PolicyEngineConfig::default(),
        }
    }
}

impl CortexConfig {
    /// Minimal configuration (for resource-constrained systems)
    pub fn minimal() -> Self {
        Self {
            level: IntelligenceLevel::Monitoring,
            memory_budget: 4 * 1024 * 1024, // 4 MB
            cpu_budget_percent: 1,
            decision_timeout_us: 500,
            event_queue_size: 256,
            max_handlers: 8,
            consciousness_enabled: true,
            neural_enabled: false,
            temporal_enabled: false,
            survivability_enabled: true,
            meta_enabled: true,
            formal_enabled: false,
            learning_enabled: false,
            policy_enabled: false,
            telemetry_enabled: false,
            learning_config: LearningConfig::default(),
            policy_config: PolicyEngineConfig::default(),
        }
    }

    /// Full configuration (all features enabled)
    pub fn full() -> Self {
        Self {
            level: IntelligenceLevel::Consciousness,
            memory_budget: 256 * 1024 * 1024, // 256 MB
            cpu_budget_percent: 10,
            decision_timeout_us: 5000,
            event_queue_size: 4096,
            max_handlers: 64,
            consciousness_enabled: true,
            neural_enabled: true,
            temporal_enabled: true,
            survivability_enabled: true,
            meta_enabled: true,
            formal_enabled: true,
            learning_enabled: true,
            policy_enabled: true,
            telemetry_enabled: true,
            learning_config: LearningConfig::default(),
            policy_config: PolicyEngineConfig::default(),
        }
    }
}

// =============================================================================
// CORTEX STATE
// =============================================================================

/// CORTEX operational state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CortexState {
    /// Not initialized
    Uninitialized,

    /// Initializing
    Initializing,

    /// Running normally
    Running,

    /// Paused
    Paused,

    /// Degraded (some components failed)
    Degraded,

    /// Recovery mode
    Recovering,

    /// Shutting down
    ShuttingDown,

    /// Stopped
    Stopped,
}

// =============================================================================
// CORTEX STATISTICS
// =============================================================================

/// Comprehensive CORTEX statistics
#[derive(Debug, Clone, Default)]
pub struct CortexStats {
    /// Current state
    pub state: u8,

    /// Uptime (milliseconds)
    pub uptime_ms: u64,

    /// Total events processed
    pub events_processed: u64,

    /// Total decisions made
    pub decisions_made: u64,

    /// Violations detected
    pub violations_detected: u64,

    /// Violations prevented
    pub violations_prevented: u64,

    /// Threats detected
    pub threats_detected: u64,

    /// Threats neutralized
    pub threats_neutralized: u64,

    /// Hot-swaps performed
    pub hot_swaps: u64,

    /// Rollbacks performed
    pub rollbacks: u64,

    /// Memory used (bytes)
    pub memory_used: usize,

    /// CPU usage (percent)
    pub cpu_usage: u8,

    /// Average decision time (microseconds)
    pub avg_decision_time_us: u64,

    /// Maximum decision time (microseconds)
    pub max_decision_time_us: u64,

    /// Health score (0.0 to 1.0)
    pub health_score: f64,
}

// =============================================================================
// INTEGRATED CORTEX
// =============================================================================

/// The integrated CORTEX system
pub struct IntegratedCortex {
    /// Configuration
    config: CortexConfig,

    /// Current state
    state: CortexState,

    /// Event bus
    bus: CortexBus,

    /// Consciousness layer
    consciousness: Option<StructuralConsciousness>,

    /// Neural engine
    neural: Option<NeuralEngine>,

    /// Temporal kernel
    temporal: Option<TemporalKernel>,

    /// Survivability core
    survivability: Option<SurvivabilityCore>,

    /// Meta-kernel
    meta: Option<MetaKernel>,

    /// Formal verification engine
    formal: Option<VerificationEngine>,

    /// Adaptive learner
    learner: Option<AdaptiveLearner>,

    /// Policy engine
    policy: Option<PolicyEngine>,

    /// Telemetry collector
    telemetry: Option<TelemetryCollector>,

    /// Start timestamp
    start_timestamp: Timestamp,

    /// Event counter
    event_counter: AtomicU64,

    /// Decision counter
    decision_counter: AtomicU64,

    /// Is active?
    active: AtomicBool,
}

impl IntegratedCortex {
    /// Create new integrated CORTEX
    pub fn new(config: CortexConfig) -> Self {
        let mut bus = CortexBus::new(config.event_queue_size, config.max_handlers);

        Self {
            consciousness: if config.consciousness_enabled {
                Some(StructuralConsciousness::new())
            } else {
                None
            },

            neural: if config.neural_enabled {
                Some(NeuralEngine::new())
            } else {
                None
            },

            temporal: if config.temporal_enabled {
                Some(TemporalKernel::new())
            } else {
                None
            },

            survivability: if config.survivability_enabled {
                Some(SurvivabilityCore::new())
            } else {
                None
            },

            meta: if config.meta_enabled {
                Some(MetaKernel::new())
            } else {
                None
            },

            formal: if config.formal_enabled {
                Some(VerificationEngine::new())
            } else {
                None
            },

            learner: if config.learning_enabled {
                Some(AdaptiveLearner::new(config.learning_config.clone()))
            } else {
                None
            },

            policy: if config.policy_enabled {
                Some(PolicyEngine::new(config.policy_config.clone()))
            } else {
                None
            },

            telemetry: if config.telemetry_enabled {
                Some(TelemetryCollector::new())
            } else {
                None
            },

            config,
            state: CortexState::Uninitialized,
            bus,
            start_timestamp: 0,
            event_counter: AtomicU64::new(0),
            decision_counter: AtomicU64::new(0),
            active: AtomicBool::new(false),
        }
    }

    /// Initialize CORTEX
    pub fn initialize(&mut self, timestamp: Timestamp) -> CortexResult {
        self.state = CortexState::Initializing;
        self.start_timestamp = timestamp;

        // Initialize meta-kernel first (it watches everything)
        if let Some(ref mut meta) = self.meta {
            meta.initialize(0x1000_0000, 1024 * 1024);
        }

        // Initialize other components
        // (In real implementation, each would have its own init)

        self.state = CortexState::Running;
        self.active.store(true, Ordering::SeqCst);

        // Publish init event
        self.bus.publish(
            CortexEvent::SystemInit {
                timestamp,
                level: self.config.level,
            },
            timestamp,
        );

        CortexResult::Observed
    }

    /// Process a kernel event
    pub fn process_event(&mut self, event: CortexEvent, timestamp: Timestamp) -> CortexResult {
        if !self.active.load(Ordering::SeqCst) {
            return CortexResult::Ignored;
        }

        self.event_counter.fetch_add(1, Ordering::SeqCst);

        // Feed meta-kernel heartbeat
        if let Some(ref mut meta) = self.meta {
            meta.heartbeat(timestamp);
        }

        // Route through bus
        self.bus.publish(event.clone(), timestamp);

        // Process pending events
        let results = self.bus.process_with_budget(
            timestamp,
            self.config.decision_timeout_us * 1000, // Convert to cycles
        );

        // Return most significant result
        results
            .into_iter()
            .max_by_key(|r| match r {
                CortexResult::Ignored => 0,
                CortexResult::Observed => 1,
                CortexResult::PatternDetected(_) => 2,
                CortexResult::Prediction(_) => 3,
                CortexResult::ActionTaken(_) => 4,
                CortexResult::SubsystemIsolated(_) => 5,
                CortexResult::HotSwapped(_) => 6,
                CortexResult::RolledBack(_) => 7,
                CortexResult::ThreatNeutralized(_) => 8,
                CortexResult::SurvivalMode => 9,
            })
            .unwrap_or(CortexResult::Observed)
    }

    /// Make a decision based on current state
    pub fn decide(
        &mut self,
        context: &crate::policy::PolicyContext,
        timestamp: Timestamp,
    ) -> Option<DecisionAction> {
        if !self.active.load(Ordering::SeqCst) {
            return None;
        }

        self.decision_counter.fetch_add(1, Ordering::SeqCst);

        // Check policy engine first
        if let Some(ref mut policy) = self.policy {
            if let Some(result) = policy.evaluate(context, None, timestamp) {
                return Some(result.action);
            }
        }

        // Fall back to neural engine
        if let Some(ref mut neural) = self.neural {
            // Would call neural.decide() here
        }

        None
    }

    /// Get current state
    pub fn state(&self) -> CortexState {
        self.state
    }

    /// Get intelligence level
    pub fn level(&self) -> IntelligenceLevel {
        self.config.level
    }

    /// Set intelligence level
    pub fn set_level(&mut self, level: IntelligenceLevel, timestamp: Timestamp) {
        let old_level = self.config.level;
        self.config.level = level;

        self.bus.publish(
            CortexEvent::LevelChanged {
                from: old_level,
                to: level,
                timestamp,
            },
            timestamp,
        );
    }

    /// Get statistics
    pub fn stats(&self, timestamp: Timestamp) -> CortexStats {
        let uptime = timestamp.saturating_sub(self.start_timestamp);

        CortexStats {
            state: self.state as u8,
            uptime_ms: uptime / 1_000_000,
            events_processed: self.event_counter.load(Ordering::SeqCst),
            decisions_made: self.decision_counter.load(Ordering::SeqCst),
            violations_detected: 0, // Would get from consciousness
            violations_prevented: 0,
            threats_detected: 0, // Would get from survivability
            threats_neutralized: 0,
            hot_swaps: 0, // Would get from temporal
            rollbacks: 0,
            memory_used: 0, // Would calculate
            cpu_usage: 0,
            avg_decision_time_us: 0, // Would calculate from telemetry
            max_decision_time_us: 0,
            health_score: self.calculate_health(),
        }
    }

    /// Calculate overall health score
    fn calculate_health(&self) -> f64 {
        let mut score = 1.0;
        let mut factors = 0;

        // Check each component
        if self.consciousness.is_some() {
            factors += 1;
        }
        if self.neural.is_some() {
            factors += 1;
        }
        if self.temporal.is_some() {
            factors += 1;
        }
        if self.survivability.is_some() {
            factors += 1;
        }
        if self.meta.is_some() {
            if let Some(ref meta) = self.meta {
                if !meta.health().is_operational() {
                    score -= 0.3;
                }
            }
            factors += 1;
        }

        // Degraded state reduces score
        if self.state == CortexState::Degraded {
            score -= 0.2;
        }

        score.max(0.0)
    }

    /// Pause CORTEX
    pub fn pause(&mut self) {
        self.state = CortexState::Paused;
        self.bus.pause();
    }

    /// Resume CORTEX
    pub fn resume(&mut self) {
        self.state = CortexState::Running;
        self.bus.resume();
    }

    /// Shutdown CORTEX
    pub fn shutdown(&mut self, timestamp: Timestamp) {
        self.state = CortexState::ShuttingDown;
        self.active.store(false, Ordering::SeqCst);

        self.bus.publish(
            CortexEvent::SystemShutdown {
                timestamp,
                reason: String::from("Normal shutdown"),
            },
            timestamp,
        );

        self.bus.clear_queue();
        self.state = CortexState::Stopped;
    }

    /// Register event handler
    pub fn register_handler(
        &mut self,
        handler: alloc::boxed::Box<dyn EventHandler>,
    ) -> Option<HandlerId> {
        self.bus.register(handler)
    }

    /// Get bus statistics
    pub fn bus_stats(&self) -> &crate::bus::BusStats {
        self.bus.stats()
    }

    /// Access consciousness layer
    pub fn consciousness(&self) -> Option<&StructuralConsciousness> {
        self.consciousness.as_ref()
    }

    /// Access consciousness layer (mutable)
    pub fn consciousness_mut(&mut self) -> Option<&mut StructuralConsciousness> {
        self.consciousness.as_mut()
    }

    /// Access neural engine
    pub fn neural(&self) -> Option<&NeuralEngine> {
        self.neural.as_ref()
    }

    /// Access neural engine (mutable)
    pub fn neural_mut(&mut self) -> Option<&mut NeuralEngine> {
        self.neural.as_mut()
    }

    /// Access temporal kernel
    pub fn temporal(&self) -> Option<&TemporalKernel> {
        self.temporal.as_ref()
    }

    /// Access temporal kernel (mutable)
    pub fn temporal_mut(&mut self) -> Option<&mut TemporalKernel> {
        self.temporal.as_mut()
    }

    /// Access survivability core
    pub fn survivability(&self) -> Option<&SurvivabilityCore> {
        self.survivability.as_ref()
    }

    /// Access survivability core (mutable)
    pub fn survivability_mut(&mut self) -> Option<&mut SurvivabilityCore> {
        self.survivability.as_mut()
    }

    /// Access meta-kernel
    pub fn meta(&self) -> Option<&MetaKernel> {
        self.meta.as_ref()
    }

    /// Access meta-kernel (mutable)
    pub fn meta_mut(&mut self) -> Option<&mut MetaKernel> {
        self.meta.as_mut()
    }

    /// Access policy engine
    pub fn policy(&self) -> Option<&PolicyEngine> {
        self.policy.as_ref()
    }

    /// Access policy engine (mutable)
    pub fn policy_mut(&mut self) -> Option<&mut PolicyEngine> {
        self.policy.as_mut()
    }

    /// Access adaptive learner
    pub fn learner(&self) -> Option<&AdaptiveLearner> {
        self.learner.as_ref()
    }

    /// Access adaptive learner (mutable)
    pub fn learner_mut(&mut self) -> Option<&mut AdaptiveLearner> {
        self.learner.as_mut()
    }

    /// Access telemetry
    pub fn telemetry(&self) -> Option<&TelemetryCollector> {
        self.telemetry.as_ref()
    }

    /// Access telemetry (mutable)
    pub fn telemetry_mut(&mut self) -> Option<&mut TelemetryCollector> {
        self.telemetry.as_mut()
    }

    /// Access verification engine
    pub fn formal(&self) -> Option<&VerificationEngine> {
        self.formal.as_ref()
    }

    /// Access verification engine (mutable)
    pub fn formal_mut(&mut self) -> Option<&mut VerificationEngine> {
        self.formal.as_mut()
    }
}

// =============================================================================
// GLOBAL INSTANCE
// =============================================================================

static mut INTEGRATED_CORTEX: Option<IntegratedCortex> = None;
static CORTEX_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize global CORTEX instance
pub fn init_cortex(config: CortexConfig, timestamp: Timestamp) -> CortexResult {
    if CORTEX_INITIALIZED.swap(true, Ordering::SeqCst) {
        return CortexResult::Ignored; // Already initialized
    }

    unsafe {
        let mut cortex = IntegratedCortex::new(config);
        let result = cortex.initialize(timestamp);
        INTEGRATED_CORTEX = Some(cortex);
        result
    }
}

/// Get reference to global CORTEX
pub fn cortex() -> Option<&'static IntegratedCortex> {
    if !CORTEX_INITIALIZED.load(Ordering::SeqCst) {
        return None;
    }
    unsafe { INTEGRATED_CORTEX.as_ref() }
}

/// Get mutable reference to global CORTEX
///
/// # Safety
/// Caller must ensure no other code is accessing CORTEX
pub unsafe fn cortex_mut() -> Option<&'static mut IntegratedCortex> {
    if !CORTEX_INITIALIZED.load(Ordering::SeqCst) {
        return None;
    }
    INTEGRATED_CORTEX.as_mut()
}

/// Shutdown global CORTEX
pub fn shutdown_cortex(timestamp: Timestamp) {
    if !CORTEX_INITIALIZED.load(Ordering::SeqCst) {
        return;
    }

    unsafe {
        if let Some(ref mut cortex) = INTEGRATED_CORTEX {
            cortex.shutdown(timestamp);
        }
    }

    CORTEX_INITIALIZED.store(false, Ordering::SeqCst);
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cortex_creation() {
        let cortex = IntegratedCortex::new(CortexConfig::default());
        assert_eq!(cortex.state(), CortexState::Uninitialized);
    }

    #[test]
    fn test_cortex_init() {
        let mut cortex = IntegratedCortex::new(CortexConfig::default());
        cortex.initialize(1000);
        assert_eq!(cortex.state(), CortexState::Running);
    }

    #[test]
    fn test_minimal_config() {
        let config = CortexConfig::minimal();
        let cortex = IntegratedCortex::new(config);

        assert!(cortex.consciousness().is_some());
        assert!(cortex.neural().is_none());
        assert!(cortex.temporal().is_none());
    }

    #[test]
    fn test_full_config() {
        let config = CortexConfig::full();
        let cortex = IntegratedCortex::new(config);

        assert!(cortex.consciousness().is_some());
        assert!(cortex.neural().is_some());
        assert!(cortex.temporal().is_some());
        assert!(cortex.survivability().is_some());
        assert!(cortex.meta().is_some());
        assert!(cortex.policy().is_some());
        assert!(cortex.learner().is_some());
    }
}
