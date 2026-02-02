//! # CORTEX: Kernel Intelligence Framework
//!
//! **CORTEX** represents a fundamental paradigm shift in operating system design.
//! It is not an AI added to a kernel - it is a kernel that IS intelligent by construction.
//!
//! ## Why This Has Never Existed Before
//!
//! Traditional kernels are **reactive machines**: they respond to interrupts, system calls,
//! and hardware events. They have no model of themselves, no understanding of their own
//! state, and no ability to reason about their future.
//!
//! CORTEX introduces **Structural Consciousness**: the kernel maintains a live, queryable
//! model of its own invariants, contracts, and state transitions. It can:
//!
//! - **Anticipate** failures before they occur
//! - **Reason** about the correctness of its own operations
//! - **Evolve** without rebooting
//! - **Survive** even partial compromise
//!
//! ## The Five Pillars of CORTEX
//!
//! ### 1. Consciousness Layer (`consciousness/`)
//! The kernel maintains formal contracts (invariants) that are **alive** - they are
//! continuously verified, and violations are detected *before* they cause crashes.
//!
//! ### 2. Neural Decision Engine (`neural/`)
//! A deterministic, bounded, verifiable decision system. Not a black-box ML model,
//! but a transparent decision tree that can explain every choice it makes.
//!
//! ### 3. Temporal Kernel (`temporal/`)
//! The kernel exists across time. Components are versioned, can be hot-swapped,
//! and automatically rolled back if instability is detected.
//!
//! ### 4. Survivability Core (`survivability/`)
//! Assumes compromise has already occurred. The kernel can detect anomalous behavior,
//! reconstruct corrupted memory, and continue operation even under attack.
//!
//! ### 5. Meta-Kernel (`meta/`)
//! The kernel that watches the kernel. A minimal, formally verified core that
//! monitors the main kernel and can take corrective action.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                           META-KERNEL                                   │
//! │  (Minimal, formally verified, watches everything)                       │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐   │
//! │  │ CONSCIOUS-  │  │   NEURAL    │  │  TEMPORAL   │  │ SURVIVABIL- │   │
//! │  │    NESS     │◄─┤   ENGINE    │◄─┤   KERNEL    │◄─┤    ITY      │   │
//! │  │   LAYER     │  │             │  │             │  │    CORE     │   │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘   │
//! │         │                │                │                │          │
//! │         ▼                ▼                ▼                ▼          │
//! │  ┌──────────────────────────────────────────────────────────────┐     │
//! │  │                    CORTEX CORE BUS                           │     │
//! │  │  (Event routing, decision propagation, state synchronization)│     │
//! │  └──────────────────────────────────────────────────────────────┘     │
//! │                              │                                        │
//! │         ┌────────────────────┼────────────────────┐                   │
//! │         ▼                    ▼                    ▼                   │
//! │  ┌─────────────┐      ┌─────────────┐      ┌─────────────┐           │
//! │  │   MEMORY    │      │  SCHEDULER  │      │   DRIVERS   │           │
//! │  │  SUBSYSTEM  │      │  SUBSYSTEM  │      │  SUBSYSTEM  │           │
//! │  │  (watched)  │      │  (watched)  │      │  (watched)  │           │
//! │  └─────────────┘      └─────────────┘      └─────────────┘           │
//! │                                                                       │
//! └───────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Example: Intelligent Decision
//!
//! ```text
//! Scenario: Memory pressure detected
//!
//! Traditional Kernel:
//!   1. OOM killer activates
//!   2. Kills largest process (often wrong choice)
//!   3. System recovers (maybe) or crashes
//!
//! CORTEX:
//!   1. Consciousness detects memory pressure TREND (before critical)
//!   2. Neural engine analyzes:
//!      - Which processes are essential?
//!      - Which have recovery mechanisms?
//!      - What's the historical pattern?
//!   3. Decision: Migrate low-priority process to swap BEFORE pressure hits
//!   4. If prediction wrong: Temporal layer rolls back decision
//!   5. Learning: Pattern stored for future decisions
//! ```

#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate alloc;

// =============================================================================
// CORE MODULES
// =============================================================================

pub mod bus;
pub mod consciousness;
pub mod formal;
pub mod integration;
pub mod learning;
pub mod meta;
pub mod neural;
pub mod policy;
pub mod survivability;
pub mod telemetry;
pub mod temporal;

// =============================================================================
// CORE TYPES
// =============================================================================

/// Timestamp in microseconds since system boot
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Create a new timestamp
    pub const fn new(us: u64) -> Self {
        Self(us)
    }

    /// Create timestamp from current time (placeholder - returns 0)
    pub fn now() -> Self {
        Self(0)
    }

    /// Get the raw microseconds value
    pub const fn as_micros(&self) -> u64 {
        self.0
    }
}

// =============================================================================
// RE-EXPORTS
// =============================================================================

use alloc::boxed::Box;
use alloc::vec::Vec;
// =============================================================================
// GLOBAL CORTEX STATE
// =============================================================================
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

pub use bus::{
    BusStats, CortexBus, CortexEvent, EventCategory, EventHandler, EventPriority, HandlerId,
};
pub use consciousness::{
    Contract, ContractId, ContractState, Invariant, InvariantId,
    InvariantState, InvariantViolation, StructuralConsciousness,
};
pub use formal::{
    Proof, ProofMethod, Property, PropertyId, PropertyKind, State, StateMachine, Transition,
    VerificationEngine, VerificationStatus,
};
pub use integration::{
    cortex, cortex_mut, init_cortex, shutdown_cortex, CortexConfig, CortexState, CortexStats,
    IntegratedCortex,
};
pub use learning::{
    AdaptiveLearner, Experience, Feedback, FeedbackContext, FeedbackType, LearningConfig,
    LearningStats, PatternLearner, Rule, RuleId, RuleSource, RuleStatus,
};
pub use meta::{
    HealthCheck, MetaAction, MetaKernel, MetaState, SystemHealth, Watchdog, WatchdogConfig,
};
pub use neural::{
    Confidence, Decision, DecisionId, DecisionNode, DecisionTree, NeuralConfig, NeuralEngine,
    Prediction, PredictionAccuracy,
};
pub use policy::{
    Comparison, ComparisonOp, Condition, Policy, PolicyContext, PolicyEngine, PolicyEngineConfig,
    PolicyId, PolicyResult, PolicyRule, PolicyStatus, PolicyVersion, Priority, Value,
};
pub use survivability::{
    Anomaly, AnomalyDetector, Recovery, RecoveryStrategy, SurvivabilityCore, Threat, ThreatLevel,
    ThreatResponse,
};
pub use telemetry::{
    Counter, Gauge, Histogram, MetricCategory, MetricId, MetricKind, RateMeter, TelemetryCollector,
    TelemetrySnapshot, TimeSeries, Timer,
};
pub use temporal::{
    HotSwap, Rollback, Snapshot, SnapshotId, TemporalKernel, SemanticVersion as Version, VersionId, VersionState,
};

/// Global CORTEX instance
static mut CORTEX: Option<Cortex> = None;

/// CORTEX initialization state
static CORTEX_STATE: AtomicU8 = AtomicU8::new(0);

/// CORTEX decision counter
static DECISION_COUNTER: AtomicU64 = AtomicU64::new(0);

/// CORTEX event counter
static EVENT_COUNTER: AtomicU64 = AtomicU64::new(0);

// =============================================================================
// CORTEX INTELLIGENCE LEVELS
// =============================================================================

/// Intelligence level of CORTEX
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntelligenceLevel {
    /// Level 0: Disabled - CORTEX is off
    Disabled      = 0,

    /// Level 1: Monitoring - Passive observation only
    Monitoring    = 1,

    /// Level 2: Detection - Pattern detection, no action
    Detection     = 2,

    /// Level 3: Prediction - Predictive analysis, advisory
    Prediction    = 3,

    /// Level 4: Correction - Autonomous corrective action
    Correction    = 4,

    /// Level 5: Consciousness - Full structural consciousness
    Consciousness = 5,
}

impl Default for IntelligenceLevel {
    fn default() -> Self {
        Self::Monitoring
    }
}

// =============================================================================
// MAIN CORTEX STRUCTURE
// =============================================================================

/// The CORTEX Kernel Intelligence System
///
/// This is the central coordinator for all CORTEX subsystems.
/// It orchestrates the consciousness layer, neural engine, temporal kernel,
/// survivability core, and meta-kernel to provide unified intelligence.
pub struct Cortex {
    /// Configuration
    config: CortexConfig,

    /// Structural consciousness
    consciousness: StructuralConsciousness,

    /// Neural decision engine
    neural: NeuralEngine,

    /// Temporal kernel
    temporal: TemporalKernel,

    /// Survivability core
    survivability: SurvivabilityCore,

    /// Meta-kernel
    meta: MetaKernel,

    /// Core event bus
    bus: CortexBus,

    /// Runtime statistics
    stats: CortexStats,

    /// Initialization timestamp
    init_time: u64,

    /// Current intelligence level
    current_level: IntelligenceLevel,
}

impl Cortex {
    /// Create a new CORTEX instance
    pub fn new(config: CortexConfig) -> Self {
        Self {
            current_level: config.level,
            consciousness: StructuralConsciousness::new(config.consciousness_enabled),
            neural: NeuralEngine::new(NeuralConfig::from(&config)),
            temporal: TemporalKernel::new(),
            survivability: SurvivabilityCore::new(),
            meta: MetaKernel::new(),
            bus: CortexBus::new(BusConfig::default()),
            stats: CortexStats::default(),
            init_time: Self::get_timestamp(),
            config,
        }
    }

    /// Initialize CORTEX globally
    pub fn init_global(config: CortexConfig) -> Result<(), CortexError> {
        let state = CORTEX_STATE.load(Ordering::SeqCst);
        if state != 0 {
            return Err(CortexError::AlreadyInitialized);
        }

        unsafe {
            CORTEX = Some(Self::new(config));
        }

        CORTEX_STATE.store(1, Ordering::SeqCst);
        Ok(())
    }

    /// Get global CORTEX instance
    pub fn global() -> Option<&'static Cortex> {
        if CORTEX_STATE.load(Ordering::SeqCst) == 0 {
            return None;
        }
        unsafe { CORTEX.as_ref() }
    }

    /// Get global CORTEX instance (mutable)
    pub fn global_mut() -> Option<&'static mut Cortex> {
        if CORTEX_STATE.load(Ordering::SeqCst) == 0 {
            return None;
        }
        unsafe { CORTEX.as_mut() }
    }

    /// Process an event through CORTEX
    pub fn process_event(&mut self, event: CortexEvent) -> CortexResult {
        let start = Self::get_timestamp();
        EVENT_COUNTER.fetch_add(1, Ordering::Relaxed);

        // Route through core bus
        self.bus.route_event(&event);

        // Process through each layer based on intelligence level
        let result = match self.current_level {
            IntelligenceLevel::Disabled => CortexResult::Ignored,

            IntelligenceLevel::Monitoring => {
                self.consciousness.observe(&event);
                CortexResult::Observed
            },

            IntelligenceLevel::Detection => {
                self.consciousness.observe(&event);
                if let Some(pattern) = self.neural.detect_pattern(&event) {
                    CortexResult::PatternDetected(pattern)
                } else {
                    CortexResult::Observed
                }
            },

            IntelligenceLevel::Prediction => {
                self.consciousness.observe(&event);
                self.neural.detect_pattern(&event);
                if let Some(prediction) = self.neural.predict(&event) {
                    CortexResult::Prediction(prediction)
                } else {
                    CortexResult::Observed
                }
            },

            IntelligenceLevel::Correction => {
                self.consciousness.observe(&event);
                self.neural.detect_pattern(&event);

                if let Some(decision) = self.neural.decide(&event) {
                    self.execute_decision(&decision)
                } else {
                    CortexResult::Observed
                }
            },

            IntelligenceLevel::Consciousness => self.full_consciousness_processing(&event),
        };

        // Update statistics
        let elapsed = Self::get_timestamp() - start;
        self.update_stats(elapsed);

        result
    }

    /// Full consciousness processing
    fn full_consciousness_processing(&mut self, event: &CortexEvent) -> CortexResult {
        // 1. Consciousness layer processes first
        let consciousness_result = self.consciousness.process(event);

        // 2. Check for invariant violations
        if let Some(violation) = consciousness_result.violation {
            // Meta-kernel takes over for critical violations
            if violation.is_critical() {
                return self.meta.handle_critical_violation(&violation);
            }

            // Neural engine decides how to handle
            if let Some(decision) = self.neural.decide_violation(&violation) {
                return self.execute_decision(&decision);
            }
        }

        // 3. Neural engine analyzes
        let analysis = self.neural.analyze(event);

        // 4. Check for threats
        if let Some(threat) = self.survivability.detect_threat(event) {
            return self.handle_threat(threat);
        }

        // 5. Make predictive decisions
        if let Some(prediction) = analysis.prediction {
            if prediction.confidence > Confidence::HIGH_THRESHOLD {
                if let Some(decision) = self.neural.decide_prediction(&prediction) {
                    // Create temporal snapshot before action
                    let snapshot = self.temporal.create_snapshot();

                    let result = self.execute_decision(&decision);

                    // If result is unstable, rollback
                    if result.is_unstable() {
                        self.temporal.rollback(snapshot);
                        return CortexResult::RolledBack(snapshot);
                    }

                    return result;
                }
            }
        }

        CortexResult::Observed
    }

    /// Execute a decision
    fn execute_decision(&mut self, decision: &Decision) -> CortexResult {
        DECISION_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.stats.decisions_made += 1;

        match decision.action {
            DecisionAction::NoOp => CortexResult::NoAction,

            DecisionAction::AdjustScheduler(ref params) => {
                self.bus
                    .emit(CortexEvent::SchedulerAdjustment(params.clone()));
                CortexResult::ActionTaken(decision.id)
            },

            DecisionAction::AdjustMemory(ref params) => {
                self.bus.emit(CortexEvent::MemoryAdjustment(params.clone()));
                CortexResult::ActionTaken(decision.id)
            },

            DecisionAction::IsolateSubsystem(subsystem_id) => {
                self.survivability.isolate_subsystem(subsystem_id);
                CortexResult::SubsystemIsolated(subsystem_id)
            },

            DecisionAction::HotSwap(ref swap_info) => {
                if let Ok(version) = self.temporal.hot_swap(swap_info) {
                    self.stats.hot_swaps_performed += 1;
                    CortexResult::HotSwapped(version)
                } else {
                    CortexResult::ActionFailed(decision.id)
                }
            },

            DecisionAction::Rollback(snapshot_id) => {
                if let Ok(()) = self.temporal.rollback(snapshot_id) {
                    self.stats.rollbacks_performed += 1;
                    CortexResult::RolledBack(snapshot_id)
                } else {
                    CortexResult::ActionFailed(decision.id)
                }
            },

            DecisionAction::ReconfigureMMU(ref config) => {
                self.bus
                    .emit(CortexEvent::MMUReconfiguration(config.clone()));
                CortexResult::ActionTaken(decision.id)
            },

            DecisionAction::DisablePath(path_id) => {
                self.consciousness.disable_code_path(path_id);
                CortexResult::PathDisabled(path_id)
            },

            DecisionAction::Custom(ref action) => {
                self.bus.emit(CortexEvent::CustomAction(action.clone()));
                CortexResult::ActionTaken(decision.id)
            },
        }
    }

    /// Handle a detected threat
    fn handle_threat(&mut self, threat: Threat) -> CortexResult {
        self.stats.threats_detected += 1;

        let response = self.survivability.respond_to_threat(&threat);

        match response.strategy {
            ThreatResponseStrategy::Ignore => CortexResult::ThreatIgnored(threat.id),

            ThreatResponseStrategy::Monitor => {
                self.consciousness.add_threat_monitor(&threat);
                CortexResult::ThreatMonitored(threat.id)
            },

            ThreatResponseStrategy::Isolate => {
                self.survivability.isolate_threat(&threat);
                self.stats.threats_neutralized += 1;
                CortexResult::ThreatIsolated(threat.id)
            },

            ThreatResponseStrategy::Neutralize => {
                self.survivability.neutralize_threat(&threat);
                self.stats.threats_neutralized += 1;
                CortexResult::ThreatNeutralized(threat.id)
            },

            ThreatResponseStrategy::Survive => {
                // Take snapshot, isolate, attempt recovery
                let snapshot = self.temporal.create_snapshot();
                self.survivability.enter_survival_mode(&threat);
                CortexResult::SurvivalMode(threat.id, snapshot)
            },
        }
    }

    /// Update statistics
    fn update_stats(&mut self, decision_time_us: u64) {
        // Rolling average for decision time
        let alpha = 0.1;
        self.stats.avg_decision_time_us =
            self.stats.avg_decision_time_us * (1.0 - alpha) + decision_time_us as f64 * alpha;

        // Update uptime
        self.stats.uptime_us = Self::get_timestamp() - self.init_time;

        // Calculate decisions per second
        if self.stats.uptime_us > 0 {
            self.stats.decisions_per_second =
                self.stats.decisions_made as f64 / (self.stats.uptime_us as f64 / 1_000_000.0);
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &CortexStats {
        &self.stats
    }

    /// Get current intelligence level
    pub fn level(&self) -> IntelligenceLevel {
        self.current_level
    }

    /// Set intelligence level
    pub fn set_level(&mut self, level: IntelligenceLevel) {
        self.current_level = level;
    }

    /// Get configuration
    pub fn config(&self) -> &CortexConfig {
        &self.config
    }

    /// Get consciousness layer
    pub fn consciousness(&self) -> &StructuralConsciousness {
        &self.consciousness
    }

    /// Get neural engine
    pub fn neural(&self) -> &NeuralEngine {
        &self.neural
    }

    /// Get temporal kernel
    pub fn temporal(&self) -> &TemporalKernel {
        &self.temporal
    }

    /// Get survivability core
    pub fn survivability(&self) -> &SurvivabilityCore {
        &self.survivability
    }

    /// Get meta-kernel
    pub fn meta(&self) -> &MetaKernel {
        &self.meta
    }

    /// Get timestamp (architecture-specific)
    fn get_timestamp() -> u64 {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::x86_64::_rdtsc()
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let val: u64;
            core::arch::asm!("mrs {}, cntvct_el0", out(reg) val);
            val
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            let val: u64;
            core::arch::asm!("rdtime {}", out(reg) val);
            val
        }

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            0
        }
    }
}

// =============================================================================
// CORTEX RESULTS
// =============================================================================

/// Result of CORTEX processing
#[derive(Debug, Clone)]
pub enum CortexResult {
    /// Event was ignored (CORTEX disabled)
    Ignored,

    /// Event was observed (monitoring only)
    Observed,

    /// Pattern detected
    PatternDetected(PatternId),

    /// Prediction made
    Prediction(Prediction),

    /// No action needed
    NoAction,

    /// Action was taken
    ActionTaken(DecisionId),

    /// Action failed
    ActionFailed(DecisionId),

    /// Subsystem was isolated
    SubsystemIsolated(SubsystemId),

    /// Hot swap performed
    HotSwapped(VersionId),

    /// Rollback performed
    RolledBack(SnapshotId),

    /// Code path disabled
    PathDisabled(PathId),

    /// Threat was ignored
    ThreatIgnored(ThreatId),

    /// Threat is being monitored
    ThreatMonitored(ThreatId),

    /// Threat was isolated
    ThreatIsolated(ThreatId),

    /// Threat was neutralized
    ThreatNeutralized(ThreatId),

    /// Entered survival mode
    SurvivalMode(ThreatId, SnapshotId),
}

impl CortexResult {
    /// Check if result indicates instability
    pub fn is_unstable(&self) -> bool {
        matches!(self, Self::ActionFailed(_))
    }
}

// =============================================================================
// DECISION ACTIONS
// =============================================================================

/// Actions that CORTEX can take
#[derive(Debug, Clone)]
pub enum DecisionAction {
    /// No operation
    NoOp,

    /// Adjust scheduler parameters
    AdjustScheduler(SchedulerParams),

    /// Adjust memory parameters
    AdjustMemory(MemoryParams),

    /// Isolate a subsystem
    IsolateSubsystem(SubsystemId),

    /// Hot-swap a component
    HotSwap(HotSwapInfo),

    /// Rollback to snapshot
    Rollback(SnapshotId),

    /// Reconfigure MMU
    ReconfigureMMU(MMUConfig),

    /// Disable a code path
    DisablePath(PathId),

    /// Custom action
    Custom(CustomAction),
}

/// Scheduler adjustment parameters
#[derive(Debug, Clone)]
pub struct SchedulerParams {
    pub timeslice_us: Option<u64>,
    pub priority_boost: Option<i8>,
    pub affinity_mask: Option<u64>,
    pub preemption_enabled: Option<bool>,
}

/// Memory adjustment parameters
#[derive(Debug, Clone)]
pub struct MemoryParams {
    pub swap_threshold: Option<u8>,
    pub oom_score_adj: Option<i16>,
    pub memory_limit: Option<usize>,
    pub reclaim_aggressive: Option<bool>,
}

/// Hot-swap information
#[derive(Debug, Clone)]
pub struct HotSwapInfo {
    pub subsystem_id: SubsystemId,
    pub old_version: VersionId,
    pub new_version: VersionId,
    pub migration_strategy: MigrationStrategy,
}

/// Migration strategy for hot-swap
#[derive(Debug, Clone)]
pub enum MigrationStrategy {
    /// Immediate swap (fastest, riskiest)
    Immediate,

    /// Gradual migration (slower, safer)
    Gradual,

    /// Shadow mode (run both, compare)
    Shadow,
}

/// MMU configuration
#[derive(Debug, Clone)]
pub struct MMUConfig {
    pub page_size: Option<usize>,
    pub protection_flags: Option<u64>,
    pub cache_policy: Option<CachePolicy>,
}

/// Cache policy
#[derive(Debug, Clone, Copy)]
pub enum CachePolicy {
    WriteBack,
    WriteThrough,
    Uncacheable,
}

/// Custom action
#[derive(Debug, Clone)]
pub struct CustomAction {
    pub name: alloc::string::String,
    pub params: alloc::vec::Vec<(alloc::string::String, alloc::string::String)>,
}

// =============================================================================
// ID TYPES
// =============================================================================

/// Pattern identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatternId(pub u64);

/// Subsystem identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubsystemId(pub u64);

/// Code path identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PathId(pub u64);

/// Threat identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ThreatId(pub u64);

/// Threat response strategy
#[derive(Debug, Clone, Copy)]
pub enum ThreatResponseStrategy {
    Ignore,
    Monitor,
    Isolate,
    Neutralize,
    Survive,
}

/// Threat response
#[derive(Debug, Clone)]
pub struct ThreatResponse {
    pub threat_id: ThreatId,
    pub strategy: ThreatResponseStrategy,
}

// =============================================================================
// ERRORS
// =============================================================================

/// CORTEX error type
#[derive(Debug, Clone)]
pub enum CortexError {
    /// CORTEX already initialized
    AlreadyInitialized,

    /// CORTEX not initialized
    NotInitialized,

    /// Invalid configuration
    InvalidConfig(alloc::string::String),

    /// Subsystem error
    SubsystemError(SubsystemId, alloc::string::String),

    /// Decision timeout
    DecisionTimeout,

    /// Resource exhausted
    ResourceExhausted,

    /// Invariant violation
    InvariantViolation(InvariantId),

    /// Rollback failed
    RollbackFailed(SnapshotId),

    /// Hot-swap failed
    HotSwapFailed(alloc::string::String),
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cortex_creation() {
        let config = CortexConfig::default();
        let cortex = Cortex::new(config);

        assert_eq!(cortex.level(), IntelligenceLevel::Monitoring);
    }

    #[test]
    fn test_intelligence_levels() {
        assert!(IntelligenceLevel::Consciousness > IntelligenceLevel::Monitoring);
        assert!(IntelligenceLevel::Prediction > IntelligenceLevel::Detection);
    }
}
