//! Orchestrator Core Types
//!
//! Fundamental type definitions for orchestrator system.

use alloc::string::String;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

// ============================================================================
// CORE IDENTIFIERS
// ============================================================================

/// Subsystem identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubsystemId(pub u32);

impl SubsystemId {
    /// Create new subsystem ID
    pub const fn new(id: u32) -> Self {
        Self(id)
    }
}

/// Well-known subsystem IDs
pub mod subsystems {
    use super::SubsystemId;

    /// Memory subsystem
    pub const MEMORY: SubsystemId = SubsystemId(1);
    /// Scheduler subsystem
    pub const SCHEDULER: SubsystemId = SubsystemId(2);
    /// Filesystem subsystem
    pub const FILESYSTEM: SubsystemId = SubsystemId(3);
    /// Network subsystem
    pub const NETWORK: SubsystemId = SubsystemId(4);
    /// Block devices
    pub const BLOCK: SubsystemId = SubsystemId(5);
    /// Power management
    pub const POWER: SubsystemId = SubsystemId(6);
    /// Thermal management
    pub const THERMAL: SubsystemId = SubsystemId(7);
    /// Security subsystem
    pub const SECURITY: SubsystemId = SubsystemId(8);
    /// Virtualization
    pub const VIRTUALIZATION: SubsystemId = SubsystemId(9);
    /// Drivers
    pub const DRIVERS: SubsystemId = SubsystemId(10);
    /// IPC
    pub const IPC: SubsystemId = SubsystemId(11);
    /// Interrupts
    pub const INTERRUPTS: SubsystemId = SubsystemId(12);
}

/// Decision ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DecisionId(pub u64);

impl DecisionId {
    /// Create new decision ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

/// Event ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u64);

impl EventId {
    /// Create new event ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}

// ============================================================================
// HEALTH & PRIORITY
// ============================================================================

/// Subsystem health level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthLevel {
    /// Critical - immediate attention required
    Critical = 0,
    /// Degraded - functioning but impaired
    Degraded = 1,
    /// Warning - potential issues detected
    Warning  = 2,
    /// Healthy - normal operation
    Healthy  = 3,
    /// Optimal - performing above expectations
    Optimal  = 4,
}

impl HealthLevel {
    /// Get level name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Critical => "critical",
            Self::Degraded => "degraded",
            Self::Warning => "warning",
            Self::Healthy => "healthy",
            Self::Optimal => "optimal",
        }
    }

    /// Score (0-100)
    pub fn score(&self) -> u8 {
        match self {
            Self::Critical => 0,
            Self::Degraded => 25,
            Self::Warning => 50,
            Self::Healthy => 75,
            Self::Optimal => 100,
        }
    }

    /// From score
    pub fn from_score(score: u8) -> Self {
        match score {
            0..=10 => Self::Critical,
            11..=40 => Self::Degraded,
            41..=60 => Self::Warning,
            61..=85 => Self::Healthy,
            _ => Self::Optimal,
        }
    }
}

/// Subsystem priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SubsystemPriority {
    /// Background - lowest priority
    Background = 0,
    /// Normal priority
    Normal     = 1,
    /// High priority
    High       = 2,
    /// Critical - must be handled immediately
    Critical   = 3,
    /// Emergency - system stability at risk
    Emergency  = 4,
}

impl SubsystemPriority {
    /// Get priority name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Background => "background",
            Self::Normal => "normal",
            Self::High => "high",
            Self::Critical => "critical",
            Self::Emergency => "emergency",
        }
    }
}

// ============================================================================
// SUBSYSTEM STATE & METRICS
// ============================================================================

/// Subsystem state
#[derive(Debug)]
pub struct SubsystemState {
    /// Subsystem ID
    pub id: SubsystemId,
    /// Name
    pub name: String,
    /// Health level
    pub health: HealthLevel,
    /// Health score (0-100)
    health_score: AtomicU32,
    /// Priority
    pub priority: SubsystemPriority,
    /// Enabled
    enabled: AtomicBool,
    /// Last update timestamp
    last_update: AtomicU64,
    /// Pending issues count
    pending_issues: AtomicU32,
    /// Active decisions
    active_decisions: AtomicU32,
    /// Metrics
    pub metrics: SubsystemMetrics,
}

impl SubsystemState {
    /// Create new subsystem state
    pub fn new(id: SubsystemId, name: String) -> Self {
        Self {
            id,
            name,
            health: HealthLevel::Healthy,
            health_score: AtomicU32::new(75),
            priority: SubsystemPriority::Normal,
            enabled: AtomicBool::new(true),
            last_update: AtomicU64::new(0),
            pending_issues: AtomicU32::new(0),
            active_decisions: AtomicU32::new(0),
            metrics: SubsystemMetrics::new(),
        }
    }

    /// Get health score
    pub fn health_score(&self) -> u32 {
        self.health_score.load(Ordering::Relaxed)
    }

    /// Update health score
    pub fn set_health_score(&self, score: u32) {
        self.health_score.store(score.min(100), Ordering::Relaxed);
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Enable/disable
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Last update
    pub fn last_update(&self) -> u64 {
        self.last_update.load(Ordering::Relaxed)
    }

    /// Update timestamp
    pub fn touch(&self, timestamp: u64) {
        self.last_update.store(timestamp, Ordering::Relaxed);
    }

    /// Pending issues
    pub fn pending_issues(&self) -> u32 {
        self.pending_issues.load(Ordering::Relaxed)
    }

    /// Add pending issue
    pub fn add_issue(&self) {
        self.pending_issues.fetch_add(1, Ordering::Relaxed);
    }

    /// Resolve issue
    pub fn resolve_issue(&self) {
        let _ = self
            .pending_issues
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |x| {
                if x > 0 { Some(x - 1) } else { Some(0) }
            });
    }
}

/// Subsystem metrics
#[derive(Debug, Default)]
pub struct SubsystemMetrics {
    /// Total events processed
    pub events_processed: AtomicU64,
    /// Total decisions made
    pub decisions_made: AtomicU64,
    /// Successful operations
    pub successful_ops: AtomicU64,
    /// Failed operations
    pub failed_ops: AtomicU64,
    /// Average response time (us)
    pub avg_response_us: AtomicU64,
}

impl SubsystemMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Record event
    pub fn record_event(&self) {
        self.events_processed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record decision
    pub fn record_decision(&self) {
        self.decisions_made.fetch_add(1, Ordering::Relaxed);
    }

    /// Record success
    pub fn record_success(&self) {
        self.successful_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Record failure
    pub fn record_failure(&self) {
        self.failed_ops.fetch_add(1, Ordering::Relaxed);
    }

    /// Success rate
    pub fn success_rate(&self) -> f32 {
        let success = self.successful_ops.load(Ordering::Relaxed);
        let failed = self.failed_ops.load(Ordering::Relaxed);
        let total = success + failed;
        if total == 0 {
            return 100.0;
        }
        success as f32 / total as f32 * 100.0
    }
}
