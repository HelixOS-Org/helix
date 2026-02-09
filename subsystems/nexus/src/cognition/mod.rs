//! # NEXUS Cognition Orchestrator
//!
//! Central orchestration module for the 7 cognitive domains of NEXUS.
//! This module implements the cognitive loop that processes information
//! from raw perception to action and learning.
//!
//! # The Cognitive Loop
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         NEXUS COGNITIVE LOOP                                 │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                              │
//! │                              ┌─────────┐                                     │
//! │                              │ REFLECT │ ◄── Domain 7: Metacognition        │
//! │                              │         │     (think about thinking)         │
//! │                              └────┬────┘                                     │
//! │                                   │                                          │
//! │   ┌───────┐    ┌───────────┐    ┌─▼─────┐    ┌────────┐    ┌─────┐         │
//! │   │ SENSE │───►│UNDERSTAND │───►│REASON │───►│ DECIDE │───►│ ACT │         │
//! │   │       │    │           │    │       │    │        │    │     │         │
//! │   │ D1    │    │   D2      │    │  D3   │    │  D4    │    │ D5  │         │
//! │   └───────┘    └───────────┘    └───────┘    └────────┘    └──┬──┘         │
//! │       ▲                                                        │             │
//! │       │                      ┌────────┐                        │             │
//! │       │                      │ MEMORY │◄───────────────────────┘             │
//! │       │                      │  (LTM) │  Domain 6: Memory                   │
//! │       │                      │        │                                      │
//! │       │                      └───┬────┘                                      │
//! │       │                          │                                           │
//! │       │                      ┌───▼────┐                                      │
//! │       │                      │ LEARN  │  Domain 8: Learning                 │
//! │       └──────────────────────┤        │  (feedback to all domains)          │
//! │                              └────────┘                                      │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Cognitive Domains
//!
//! 1. **SENSE** - Capture raw kernel signals
//! 2. **UNDERSTAND** - Parse and model code semantics
//! 3. **REASON** - Causal inference and explanation
//! 4. **DECIDE** - Option generation and selection
//! 5. **ACT** - Controlled action execution
//! 6. **MEMORY (LTM)** - Long-term pattern storage
//! 7. **REFLECT** - Metacognitive self-monitoring
//! 8. **LEARN** - Continuous improvement

#![allow(dead_code)]

extern crate alloc;
use alloc::format;
use alloc::vec;

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::types::{DomainId, Timestamp};

pub mod attention;
pub mod bandwidth;
pub mod blackboard;
pub mod bus;
pub mod context;
pub mod coordinator;
pub mod cycle;
pub mod executor;
pub mod factory;
pub mod fusion;
pub mod health;
pub mod insight;
pub mod integration;
pub mod latency;
pub mod lifecycle;
pub mod load;
pub mod message;
pub mod metrics;
pub mod mode;
pub mod monitor;
pub mod oracle;
pub mod output;
pub mod pipeline;
pub mod priority;
pub mod protocol;
pub mod quality;
pub mod queue;
pub mod registry;
pub mod router;
pub mod scheduler;
pub mod session;
pub mod state;
pub mod stream;
pub mod sync;
pub mod telemetry;
pub mod throttle;
pub mod timing;
pub mod trace;
pub mod transfer;
pub mod transition;
pub mod trigger;
pub mod validator;

// ============================================================================
// COGNITIVE STATE
// ============================================================================

/// Current cognitive mode of NEXUS
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CognitiveMode {
    /// System is starting up, limited cognition
    Booting      = 0,
    /// Normal operation, all domains active
    Normal       = 1,
    /// High load, prioritize critical signals
    HighLoad     = 2,
    /// Healing mode, focus on recovery
    Healing      = 3,
    /// Learning mode, focus on pattern extraction
    Learning     = 4,
    /// Reflection mode, analyzing own performance
    Reflecting   = 5,
    /// Survival mode, minimal cognition for stability
    Survival     = 6,
    /// Shutting down, graceful termination
    ShuttingDown = 7,
}

impl CognitiveMode {
    /// Check if mode allows full cognitive processing
    #[inline(always)]
    pub fn is_full_cognition(&self) -> bool {
        matches!(self, Self::Normal | Self::Learning | Self::Reflecting)
    }

    /// Check if mode is a degraded state
    #[inline(always)]
    pub fn is_degraded(&self) -> bool {
        matches!(self, Self::HighLoad | Self::Healing | Self::Survival)
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Booting => "Booting",
            Self::Normal => "Normal",
            Self::HighLoad => "HighLoad",
            Self::Healing => "Healing",
            Self::Learning => "Learning",
            Self::Reflecting => "Reflecting",
            Self::Survival => "Survival",
            Self::ShuttingDown => "ShuttingDown",
        }
    }
}

/// State of a cognitive domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DomainState {
    /// Not initialized
    Uninitialized = 0,
    /// Initializing
    Starting      = 1,
    /// Running normally
    Active        = 2,
    /// Temporarily paused
    Paused        = 3,
    /// Degraded but functional
    Degraded      = 4,
    /// Failed and needs recovery
    Failed        = 5,
    /// Stopped
    Stopped       = 6,
}

impl DomainState {
    /// Check if domain is operational
    #[inline(always)]
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Active | Self::Degraded)
    }
}

// ============================================================================
// COGNITIVE DOMAIN TRAIT
// ============================================================================

/// Trait implemented by all cognitive domains
pub trait CognitiveDomain: Send + Sync {
    /// Get domain identifier
    fn id(&self) -> DomainId;

    /// Get domain name
    fn name(&self) -> &'static str;

    /// Initialize the domain
    fn initialize(&mut self) -> Result<(), CognitionError>;

    /// Process one cognitive cycle
    fn process_cycle(&mut self, context: &mut CycleContext) -> Result<CycleResult, CognitionError>;

    /// Handle incoming message from another domain
    fn handle_message(&mut self, msg: &DomainMessage) -> Result<(), CognitionError>;

    /// Get domain health (0.0-1.0)
    fn health(&self) -> f32;

    /// Get domain state
    fn state(&self) -> DomainState;

    /// Pause the domain
    fn pause(&mut self);

    /// Resume the domain
    fn resume(&mut self);

    /// Shutdown the domain
    fn shutdown(&mut self) -> Result<(), CognitionError>;

    /// Get domain metrics
    fn metrics(&self) -> DomainMetrics;
}

/// Context passed to each domain during a cognitive cycle
#[derive(Debug)]
#[repr(align(64))]
pub struct CycleContext {
    /// Cycle number
    pub cycle_id: u64,
    /// Timestamp at cycle start
    pub timestamp: Timestamp,
    /// Current cognitive mode
    pub mode: CognitiveMode,
    /// Time budget for this cycle (nanoseconds)
    pub time_budget_ns: u64,
    /// Messages from other domains
    pub inbox: Vec<DomainMessage>,
    /// Messages to send to other domains
    pub outbox: Vec<DomainMessage>,
    /// Shared blackboard for inter-domain data
    pub blackboard: BlackboardRef,
}

/// Result of a cognitive cycle
#[derive(Debug)]
pub struct CycleResult {
    /// Domain that produced this result
    pub domain_id: DomainId,
    /// Duration of the cycle (nanoseconds)
    pub duration_ns: u64,
    /// Number of items processed
    pub items_processed: u64,
    /// Any errors or warnings
    pub issues: Vec<CycleIssue>,
    /// Suggested mode transition
    pub mode_suggestion: Option<CognitiveMode>,
}

/// Issue during a cognitive cycle
#[derive(Debug, Clone)]
pub struct CycleIssue {
    /// Severity (0-10)
    pub severity: u8,
    /// Description
    pub description: String,
    /// Suggested action
    pub suggestion: Option<String>,
}

// ============================================================================
// INTER-DOMAIN MESSAGING
// ============================================================================

/// Message between cognitive domains
#[derive(Debug, Clone)]
pub struct DomainMessage {
    /// Unique message ID
    pub id: u64,
    /// Source domain
    pub source: DomainId,
    /// Destination domain (None = broadcast)
    pub destination: Option<DomainId>,
    /// Message type
    pub kind: MessageKind,
    /// Message priority
    pub priority: MessagePriority,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Payload
    pub payload: MessagePayload,
}

/// Kind of inter-domain message
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// Data transfer
    DataTransfer,
    /// Query requesting data
    QueryRequestingData,
    /// Response to a query
    ResponseToA,
    /// Command to perform action
    CommandToPerform,
    /// Status update
    StatusUpdate,
    /// Error notification
    ErrorNotification,
    /// Insight/observation
    Insightobservation,
}

/// Priority of inter-domain message
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum MessagePriority {
    /// Lowest priority, processed when idle
    Background = 0,
    /// Normal priority
    Normal     = 1,
    /// High priority, processed soon
    High       = 2,
    /// Critical, processed immediately
    Critical   = 3,
}

/// Payload of a domain message
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// No payload
    NoPayload,
    /// Raw bytes
    Bytes(Vec<u8>),
    /// Signals from SENSE
    Signals(Vec<SignalData>),
    /// Patterns from UNDERSTAND
    Patterns(Vec<PatternData>),
    /// Causal chains from REASON
    CausalChains(Vec<CausalChainData>),
    /// Options from DECIDE
    Options(Vec<OptionData>),
    /// Effects from ACT
    Effects(Vec<EffectData>),
    /// Memories from LTM
    Memories(Vec<MemoryData>),
    /// Insights from REFLECT
    Insights(Vec<InsightData>),
    /// Learnings from LEARN
    Learnings(Vec<LearningData>),
}

/// Signal data from SENSE domain
#[derive(Debug, Clone)]
pub struct SignalData {
    pub id: u64,
    pub kind: u32,
    pub value: f64,
    pub timestamp: Timestamp,
}

/// Pattern data from UNDERSTAND domain
#[derive(Debug, Clone)]
pub struct PatternData {
    pub id: u64,
    pub pattern_type: u32,
    pub confidence: f32,
    pub context: Vec<u8>,
}

/// Causal chain data from REASON domain
#[derive(Debug, Clone)]
pub struct CausalChainData {
    pub id: u64,
    pub cause_id: u64,
    pub effect_id: u64,
    pub strength: f32,
}

/// Option data from DECIDE domain
#[derive(Debug, Clone)]
pub struct OptionData {
    pub id: u64,
    pub action_type: u32,
    pub score: f32,
    pub confidence: f32,
}

/// Effect data from ACT domain
#[derive(Debug, Clone)]
pub struct EffectData {
    pub id: u64,
    pub action_id: u64,
    pub success: bool,
    pub duration_ns: u64,
}

/// Memory data from LTM domain
#[derive(Debug, Clone)]
pub struct MemoryData {
    pub id: u64,
    pub memory_type: u32,
    pub relevance: f32,
    pub content: Vec<u8>,
}

/// Insight data from REFLECT domain
#[derive(Debug, Clone)]
pub struct InsightData {
    pub id: u64,
    pub domain_id: DomainId,
    pub insight_type: u32,
    pub severity: u8,
    pub description: String,
}

/// Learning data from LEARN domain
#[derive(Debug, Clone)]
pub struct LearningData {
    pub id: u64,
    pub learning_type: u32,
    pub source_domain: DomainId,
    pub improvement: f32,
}

// ============================================================================
// BLACKBOARD (SHARED MEMORY)
// ============================================================================

/// Reference to shared blackboard
pub type BlackboardRef = *mut Blackboard;

/// Shared blackboard for inter-domain communication
pub struct Blackboard {
    /// Current signals from SENSE
    pub signals: Vec<SignalData>,
    /// Current patterns from UNDERSTAND
    pub patterns: Vec<PatternData>,
    /// Current causal hypotheses from REASON
    pub causal_chains: Vec<CausalChainData>,
    /// Current decision options from DECIDE
    pub options: Vec<OptionData>,
    /// Recent effects from ACT
    pub effects: Vec<EffectData>,
    /// Relevant memories from LTM
    pub memories: Vec<MemoryData>,
    /// Current insights from REFLECT
    pub insights: Vec<InsightData>,
    /// Recent learnings from LEARN
    pub learnings: Vec<LearningData>,
    /// Global attention focus
    pub attention_focus: Option<AttentionFocus>,
    /// System health summary
    pub system_health: SystemHealth,
}

impl Blackboard {
    /// Create a new empty blackboard
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
            patterns: Vec::new(),
            causal_chains: Vec::new(),
            options: Vec::new(),
            effects: Vec::new(),
            memories: Vec::new(),
            insights: Vec::new(),
            learnings: Vec::new(),
            attention_focus: None,
            system_health: SystemHealth::default(),
        }
    }

    /// Clear all data for new cycle
    #[inline]
    pub fn clear(&mut self) {
        self.signals.clear();
        self.patterns.clear();
        self.causal_chains.clear();
        self.options.clear();
        self.effects.clear();
        self.memories.clear();
        self.insights.clear();
        self.learnings.clear();
    }
}

impl Default for Blackboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Current focus of attention
#[derive(Debug, Clone)]
pub struct AttentionFocus {
    /// What we're focusing on
    pub target: AttentionTarget,
    /// Priority (0-10)
    pub priority: u8,
    /// How long we've been focused (cycles)
    pub duration: u64,
    /// Reason for focus
    pub reason: String,
}

/// Target of attention
#[derive(Debug, Clone)]
pub enum AttentionTarget {
    /// Focus on a specific component
    Component(u64),
    /// Focus on a specific process
    Process(u64),
    /// Focus on a subsystem
    Subsystem(String),
    /// Focus on a pattern
    Pattern(u64),
    /// Focus on an anomaly
    Anomaly(u64),
}

/// System health summary
#[derive(Debug, Clone, Default)]
pub struct SystemHealth {
    /// Overall health (0.0-1.0)
    pub overall: f32,
    /// CPU health
    pub cpu: f32,
    /// Memory health
    pub memory: f32,
    /// I/O health
    pub io: f32,
    /// Network health
    pub network: f32,
    /// Scheduler health
    pub scheduler: f32,
}

// ============================================================================
// DOMAIN METRICS
// ============================================================================

/// Metrics for a cognitive domain
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct DomainMetrics {
    /// Total cycles processed
    pub total_cycles: u64,
    /// Successful cycles
    pub successful_cycles: u64,
    /// Failed cycles
    pub failed_cycles: u64,
    /// Average cycle duration (nanoseconds)
    pub avg_cycle_duration_ns: u64,
    /// Maximum cycle duration (nanoseconds)
    pub max_cycle_duration_ns: u64,
    /// Items processed per cycle (average)
    pub avg_items_per_cycle: f64,
    /// Current queue depth
    pub queue_depth: u64,
    /// Memory usage (bytes)
    pub memory_usage: u64,
}

// ============================================================================
// ERRORS
// ============================================================================

/// Cognition errors
#[derive(Debug, Clone)]
pub enum CognitionError {
    /// Domain not initialized
    DomainNotInitialized,
    /// Domain already initialized
    DomainAlreadyInitialized,
    /// Domain is stopped
    DomainIsStopped,
    /// Timeout exceeded
    TimeoutExceeded,
    /// Resource exhausted
    ResourceExhausted,
    /// Invalid state transition
    InvalidStateTransition,
    /// Communication error
    CommunicationError(String),
    /// Processing error
    ProcessingError(String),
    /// Internal error
    Internal(String),
}

// ============================================================================
// COGNITIVE ORCHESTRATOR
// ============================================================================

/// Main cognitive orchestrator
pub struct CognitiveOrchestrator {
    /// Current mode
    mode: CognitiveMode,
    /// Cycle counter
    cycle_count: AtomicU64,
    /// Registered domains
    domains: Vec<Box<dyn CognitiveDomain>>,
    /// Shared blackboard
    blackboard: Box<Blackboard>,
    /// Message router
    message_queue: Vec<DomainMessage>,
    /// Is running
    running: AtomicBool,
    /// Configuration
    config: OrchestratorConfig,
    /// Statistics
    stats: OrchestratorStats,
}

/// Configuration for the orchestrator
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Target cycle time (nanoseconds)
    pub target_cycle_ns: u64,
    /// Maximum domains
    pub max_domains: usize,
    /// Maximum messages per cycle
    pub max_messages_per_cycle: usize,
    /// Enable parallel processing
    pub parallel_processing: bool,
    /// Health check interval (cycles)
    pub health_check_interval: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            target_cycle_ns: 1_000_000, // 1ms
            max_domains: 16,
            max_messages_per_cycle: 1000,
            parallel_processing: false,
            health_check_interval: 100,
        }
    }
}

/// Statistics for the orchestrator
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct OrchestratorStats {
    /// Total cycles run
    pub total_cycles: u64,
    /// Total messages routed
    pub total_messages: u64,
    /// Average cycle time (nanoseconds)
    pub avg_cycle_time_ns: u64,
    /// Maximum cycle time (nanoseconds)
    pub max_cycle_time_ns: u64,
    /// Mode transitions
    pub mode_transitions: u64,
    /// Health checks performed
    pub health_checks: u64,
}

impl CognitiveOrchestrator {
    /// Create a new orchestrator
    pub fn new(config: OrchestratorConfig) -> Self {
        Self {
            mode: CognitiveMode::Booting,
            cycle_count: AtomicU64::new(0),
            domains: Vec::new(),
            blackboard: Box::new(Blackboard::new()),
            message_queue: Vec::new(),
            running: AtomicBool::new(false),
            config,
            stats: OrchestratorStats::default(),
        }
    }

    /// Register a cognitive domain
    #[inline]
    pub fn register_domain(
        &mut self,
        domain: Box<dyn CognitiveDomain>,
    ) -> Result<(), CognitionError> {
        if self.domains.len() >= self.config.max_domains {
            return Err(CognitionError::ResourceExhausted);
        }
        self.domains.push(domain);
        Ok(())
    }

    /// Get current mode
    #[inline(always)]
    pub fn mode(&self) -> CognitiveMode {
        self.mode
    }

    /// Set cognitive mode
    #[inline]
    pub fn set_mode(&mut self, mode: CognitiveMode) {
        if self.mode != mode {
            self.mode = mode;
            self.stats.mode_transitions += 1;
        }
    }

    /// Get cycle count
    #[inline(always)]
    pub fn cycle_count(&self) -> u64 {
        self.cycle_count.load(Ordering::Relaxed)
    }

    /// Get statistics
    #[inline(always)]
    pub fn stats(&self) -> &OrchestratorStats {
        &self.stats
    }

    /// Get blackboard reference
    #[inline(always)]
    pub fn blackboard(&self) -> &Blackboard {
        &self.blackboard
    }

    /// Get mutable blackboard reference
    #[inline(always)]
    pub fn blackboard_mut(&mut self) -> &mut Blackboard {
        &mut self.blackboard
    }

    /// Initialize all domains
    #[inline]
    pub fn initialize(&mut self) -> Result<(), CognitionError> {
        for domain in &mut self.domains {
            domain.initialize()?;
        }
        self.mode = CognitiveMode::Normal;
        self.running.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Run one cognitive cycle
    pub fn run_cycle(&mut self) -> Result<Vec<CycleResult>, CognitionError> {
        let cycle_id = self.cycle_count.fetch_add(1, Ordering::Relaxed);
        let cycle_start = Timestamp::now();

        let mut results = Vec::new();

        // Create context for this cycle
        let blackboard_ptr = &mut *self.blackboard as BlackboardRef;

        for domain in &mut self.domains {
            // Skip non-operational domains
            if !domain.state().is_operational() {
                continue;
            }

            // Deliver messages to domain
            let inbox: Vec<_> = self
                .message_queue
                .iter()
                .filter(|m| m.destination.is_none() || m.destination == Some(domain.id()))
                .cloned()
                .collect();

            let mut context = CycleContext {
                cycle_id,
                timestamp: cycle_start,
                mode: self.mode,
                time_budget_ns: self.config.target_cycle_ns / self.domains.len() as u64,
                inbox,
                outbox: Vec::new(),
                blackboard: blackboard_ptr,
            };

            // Process cycle
            match domain.process_cycle(&mut context) {
                Ok(result) => {
                    // Collect outgoing messages
                    self.message_queue.extend(context.outbox);
                    results.push(result);
                },
                Err(e) => {
                    // Log error but continue
                    results.push(CycleResult {
                        domain_id: domain.id(),
                        duration_ns: 0,
                        items_processed: 0,
                        issues: vec![CycleIssue {
                            severity: 8,
                            description: alloc::format!("Domain error: {:?}", e),
                            suggestion: Some("Check domain health".into()),
                        }],
                        mode_suggestion: None,
                    });
                },
            }
        }

        // Clear processed messages
        self.message_queue.retain(|m| {
            m.timestamp.elapsed_ns() < 1_000_000_000 // Keep for 1 second
        });

        // Update stats
        self.stats.total_cycles += 1;
        let cycle_time = cycle_start.elapsed_ns();
        self.stats.avg_cycle_time_ns =
            (self.stats.avg_cycle_time_ns * (self.stats.total_cycles - 1) + cycle_time)
                / self.stats.total_cycles;
        if cycle_time > self.stats.max_cycle_time_ns {
            self.stats.max_cycle_time_ns = cycle_time;
        }

        // Check health periodically
        if cycle_id % self.config.health_check_interval == 0 {
            self.check_health();
        }

        Ok(results)
    }

    /// Check health of all domains
    fn check_health(&mut self) {
        self.stats.health_checks += 1;

        let mut total_health = 0.0;
        let mut count = 0;

        for domain in &self.domains {
            total_health += domain.health();
            count += 1;
        }

        if count > 0 {
            self.blackboard.system_health.overall = total_health / count as f32;
        }
    }

    /// Shutdown all domains
    #[inline]
    pub fn shutdown(&mut self) -> Result<(), CognitionError> {
        self.mode = CognitiveMode::ShuttingDown;
        self.running.store(false, Ordering::SeqCst);

        for domain in &mut self.domains {
            let _ = domain.shutdown();
        }

        Ok(())
    }

    /// Get domain count
    #[inline(always)]
    pub fn domain_count(&self) -> usize {
        self.domains.len()
    }

    /// Check if running
    #[inline(always)]
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub use attention::*;
pub use bandwidth::*;
pub use blackboard::*;
pub use bus::*;
pub use context::*;
pub use coordinator::*;
pub use cycle::*;
pub use fusion::*;
pub use health::*;
pub use insight::*;
pub use integration::*;
pub use latency::*;
pub use lifecycle::*;
pub use load::*;
pub use message::*;
pub use metrics::*;
pub use mode::*;
pub use monitor::*;
pub use oracle::*;
pub use output::*;
pub use pipeline::*;
pub use priority::*;
pub use protocol::*;
pub use quality::*;
pub use queue::*;
pub use router::*;
pub use scheduler::*;
pub use session::*;
pub use state::*;
pub use stream::*;
pub use sync::*;
pub use telemetry::*;
pub use throttle::*;
pub use timing::*;
pub use trace::*;
pub use transfer::*;
pub use transition::*;
pub use trigger::*;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cognitive_mode() {
        assert!(CognitiveMode::Normal.is_full_cognition());
        assert!(CognitiveMode::Survival.is_degraded());
        assert!(!CognitiveMode::Normal.is_degraded());
    }

    #[test]
    fn test_domain_state() {
        assert!(DomainState::Active.is_operational());
        assert!(DomainState::Degraded.is_operational());
        assert!(!DomainState::Failed.is_operational());
    }

    #[test]
    fn test_blackboard() {
        let mut bb = Blackboard::new();
        assert!(bb.signals.is_empty());

        bb.signals.push(SignalData {
            id: 1,
            kind: 0,
            value: 42.0,
            timestamp: Timestamp::now(),
        });
        assert_eq!(bb.signals.len(), 1);

        bb.clear();
        assert!(bb.signals.is_empty());
    }

    #[test]
    fn test_orchestrator() {
        let config = OrchestratorConfig::default();
        let orch = CognitiveOrchestrator::new(config);

        assert_eq!(orch.mode(), CognitiveMode::Booting);
        assert_eq!(orch.domain_count(), 0);
        assert!(!orch.is_running());
    }

    #[test]
    fn test_message_priority() {
        assert!(MessagePriority::Critical > MessagePriority::High);
        assert!(MessagePriority::High > MessagePriority::Normal);
        assert!(MessagePriority::Normal > MessagePriority::Background);
    }
}
