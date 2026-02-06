//! NEXUS Core — Central Infrastructure
//!
//! This module provides the core infrastructure for NEXUS:
//! - The main Nexus struct and lifecycle management
//! - Clock and timing utilities
//! - Identity and versioning
//!
//! # Lifecycle
//!
//! ```text
//! ┌────────────┐     ┌────────────┐     ┌────────────┐
//! │   INIT     │────►│   READY    │────►│  RUNNING   │
//! └────────────┘     └────────────┘     └─────┬──────┘
//!                                             │
//!                    ┌────────────┐     ┌─────▼──────┐
//!                    │  STOPPED   │◄────│ SHUTTING   │
//!                    └────────────┘     │   DOWN     │
//!                                       └────────────┘
//! ```

#![allow(dead_code)]

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::bus::{Domain, MessageBus};
use crate::types::*;

// ============================================================================
// NEXUS IDENTITY
// ============================================================================

/// NEXUS version information
#[derive(Debug, Clone)]
pub struct NexusVersion {
    /// Semantic version
    pub version: Version,
    /// Codename
    pub codename: &'static str,
    /// Build timestamp
    pub build_timestamp: &'static str,
    /// Git commit hash
    pub git_hash: Option<&'static str>,
}

impl NexusVersion {
    /// Current version
    pub const CURRENT: Self = Self {
        version: Version::new(2, 0, 0),
        codename: "COGNITION",
        build_timestamp: "2026-01-30",
        git_hash: None,
    };
}

impl core::fmt::Display for NexusVersion {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "NEXUS {} \"{}\" ({})",
            self.version, self.codename, self.build_timestamp
        )
    }
}

/// NEXUS identity
#[derive(Debug, Clone)]
pub struct NexusIdentity {
    /// Instance ID
    pub id: NexusId,
    /// Boot ID (unique per boot)
    pub boot_id: u64,
    /// Version
    pub version: NexusVersion,
    /// Start time
    pub start_time: Timestamp,
}

impl NexusIdentity {
    /// Create new identity
    pub fn new(boot_id: u64) -> Self {
        Self {
            id: NexusId::generate(),
            boot_id,
            version: NexusVersion::CURRENT,
            start_time: Timestamp::now(),
        }
    }

    /// Uptime
    pub fn uptime(&self, now: Timestamp) -> Duration {
        now.elapsed_since(self.start_time)
    }
}

// ============================================================================
// CLOCK
// ============================================================================

/// NEXUS internal clock
pub struct NexusClock {
    /// Current tick
    tick: AtomicU64,
    /// Current time (nanoseconds)
    time_ns: AtomicU64,
    /// Tick duration (nanoseconds)
    tick_duration_ns: u64,
    /// Is running
    running: AtomicBool,
}

impl NexusClock {
    /// Create new clock
    pub fn new(tick_duration: Duration) -> Self {
        Self {
            tick: AtomicU64::new(0),
            time_ns: AtomicU64::new(0),
            tick_duration_ns: tick_duration.as_nanos(),
            running: AtomicBool::new(false),
        }
    }

    /// Default tick rate (10ms)
    pub const DEFAULT_TICK_DURATION: Duration = Duration::from_millis(10);

    /// Start the clock
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    /// Stop the clock
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    /// Advance clock by one tick
    pub fn tick(&self) -> u64 {
        let tick = self.tick.fetch_add(1, Ordering::SeqCst) + 1;
        self.time_ns
            .fetch_add(self.tick_duration_ns, Ordering::SeqCst);
        tick
    }

    /// Set time (for synchronization)
    pub fn set_time(&self, time_ns: u64) {
        self.time_ns.store(time_ns, Ordering::SeqCst);
    }

    /// Get current tick
    pub fn current_tick(&self) -> u64 {
        self.tick.load(Ordering::Acquire)
    }

    /// Get current time
    pub fn now(&self) -> Timestamp {
        Timestamp::new(self.time_ns.load(Ordering::Acquire))
    }

    /// Get tick duration
    pub fn tick_duration(&self) -> Duration {
        Duration::from_nanos(self.tick_duration_ns)
    }
}

impl Default for NexusClock {
    fn default() -> Self {
        Self::new(Self::DEFAULT_TICK_DURATION)
    }
}

// ============================================================================
// NEXUS STATE
// ============================================================================

/// NEXUS state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NexusState {
    /// Not yet initialized
    Uninitialized,
    /// Initializing
    Initializing,
    /// Ready to run
    Ready,
    /// Running
    Running,
    /// Paused
    Paused,
    /// Shutting down
    ShuttingDown,
    /// Stopped
    Stopped,
    /// Failed
    Failed,
}

impl NexusState {
    /// Get state name
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Uninitialized => "uninitialized",
            Self::Initializing => "initializing",
            Self::Ready => "ready",
            Self::Running => "running",
            Self::Paused => "paused",
            Self::ShuttingDown => "shutting_down",
            Self::Stopped => "stopped",
            Self::Failed => "failed",
        }
    }

    /// Is operational
    pub const fn is_operational(&self) -> bool {
        matches!(self, Self::Ready | Self::Running | Self::Paused)
    }

    /// Can transition to
    pub fn can_transition_to(&self, target: &Self) -> bool {
        matches!(
            (self, target),
            (Self::Uninitialized, Self::Initializing)
                | (Self::Initializing, Self::Ready)
                | (Self::Initializing, Self::Failed)
                | (Self::Ready, Self::Running)
                | (Self::Ready, Self::ShuttingDown)
                | (Self::Running, Self::Paused)
                | (Self::Running, Self::ShuttingDown)
                | (Self::Running, Self::Failed)
                | (Self::Paused, Self::Running)
                | (Self::Paused, Self::ShuttingDown)
                | (Self::ShuttingDown, Self::Stopped)
        )
    }
}

// ============================================================================
// NEXUS CONFIGURATION
// ============================================================================

/// NEXUS configuration
#[derive(Debug, Clone)]
pub struct NexusConfig {
    /// Tick duration
    pub tick_duration: Duration,
    /// Enable perception domain
    pub enable_sense: bool,
    /// Enable comprehension domain
    pub enable_understand: bool,
    /// Enable reasoning domain
    pub enable_reason: bool,
    /// Enable decision domain
    pub enable_decide: bool,
    /// Enable execution domain
    pub enable_act: bool,
    /// Enable memory domain
    pub enable_memory: bool,
    /// Enable reflection domain
    pub enable_reflect: bool,
    /// Maximum message bus queue size
    pub max_bus_queue: usize,
    /// Enable safety constraints
    pub enable_safety: bool,
    /// Enable telemetry
    pub enable_telemetry: bool,
    /// Log level
    pub log_level: LogLevel,
}

impl Default for NexusConfig {
    fn default() -> Self {
        Self {
            tick_duration: NexusClock::DEFAULT_TICK_DURATION,
            enable_sense: true,
            enable_understand: true,
            enable_reason: true,
            enable_decide: true,
            enable_act: true,
            enable_memory: true,
            enable_reflect: true,
            max_bus_queue: 10000,
            enable_safety: true,
            enable_telemetry: true,
            log_level: LogLevel::Info,
        }
    }
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

// ============================================================================
// DOMAIN MANAGER
// ============================================================================

/// Status of each domain
#[derive(Debug, Clone)]
pub struct DomainStatus {
    /// Domain
    pub domain: Domain,
    /// Is enabled
    pub enabled: bool,
    /// Is healthy
    pub healthy: bool,
    /// Health score (0-100)
    pub health_score: u8,
    /// Last tick
    pub last_tick: u64,
    /// Message backlog
    pub message_backlog: usize,
}

/// Domain manager
pub struct DomainManager {
    /// Domain statuses
    statuses: [DomainStatus; 7],
}

impl DomainManager {
    /// Create new domain manager
    pub fn new(config: &NexusConfig) -> Self {
        Self {
            statuses: [
                DomainStatus {
                    domain: Domain::Sense,
                    enabled: config.enable_sense,
                    healthy: true,
                    health_score: 100,
                    last_tick: 0,
                    message_backlog: 0,
                },
                DomainStatus {
                    domain: Domain::Understand,
                    enabled: config.enable_understand,
                    healthy: true,
                    health_score: 100,
                    last_tick: 0,
                    message_backlog: 0,
                },
                DomainStatus {
                    domain: Domain::Reason,
                    enabled: config.enable_reason,
                    healthy: true,
                    health_score: 100,
                    last_tick: 0,
                    message_backlog: 0,
                },
                DomainStatus {
                    domain: Domain::Decide,
                    enabled: config.enable_decide,
                    healthy: true,
                    health_score: 100,
                    last_tick: 0,
                    message_backlog: 0,
                },
                DomainStatus {
                    domain: Domain::Act,
                    enabled: config.enable_act,
                    healthy: true,
                    health_score: 100,
                    last_tick: 0,
                    message_backlog: 0,
                },
                DomainStatus {
                    domain: Domain::Memory,
                    enabled: config.enable_memory,
                    healthy: true,
                    health_score: 100,
                    last_tick: 0,
                    message_backlog: 0,
                },
                DomainStatus {
                    domain: Domain::Reflect,
                    enabled: config.enable_reflect,
                    healthy: true,
                    health_score: 100,
                    last_tick: 0,
                    message_backlog: 0,
                },
            ],
        }
    }

    /// Get domain status
    pub fn status(&self, domain: Domain) -> Option<&DomainStatus> {
        self.statuses.iter().find(|s| s.domain == domain)
    }

    /// Get mutable domain status
    pub fn status_mut(&mut self, domain: Domain) -> Option<&mut DomainStatus> {
        self.statuses.iter_mut().find(|s| s.domain == domain)
    }

    /// Get all statuses
    pub fn all_statuses(&self) -> &[DomainStatus] {
        &self.statuses
    }

    /// Get enabled domains
    pub fn enabled_domains(&self) -> Vec<Domain> {
        self.statuses
            .iter()
            .filter(|s| s.enabled)
            .map(|s| s.domain)
            .collect()
    }

    /// Get healthy domains
    pub fn healthy_domains(&self) -> Vec<Domain> {
        self.statuses
            .iter()
            .filter(|s| s.enabled && s.healthy)
            .map(|s| s.domain)
            .collect()
    }

    /// Overall health score
    pub fn overall_health(&self) -> u8 {
        let enabled: Vec<_> = self.statuses.iter().filter(|s| s.enabled).collect();
        if enabled.is_empty() {
            return 0;
        }
        let sum: u32 = enabled.iter().map(|s| s.health_score as u32).sum();
        (sum / enabled.len() as u32) as u8
    }

    /// Update domain tick
    pub fn record_tick(&mut self, domain: Domain, tick: u64) {
        if let Some(status) = self.status_mut(domain) {
            status.last_tick = tick;
        }
    }

    /// Update domain health
    pub fn update_health(&mut self, domain: Domain, healthy: bool, score: u8) {
        if let Some(status) = self.status_mut(domain) {
            status.healthy = healthy;
            status.health_score = score;
        }
    }
}

// ============================================================================
// NEXUS MAIN STRUCT
// ============================================================================

/// The main NEXUS cognitive kernel
pub struct Nexus {
    /// Identity
    identity: NexusIdentity,
    /// Configuration
    config: NexusConfig,
    /// State
    state: NexusState,
    /// Clock
    clock: NexusClock,
    /// Message bus
    bus: MessageBus,
    /// Domain manager
    domains: DomainManager,
    /// Total ticks processed
    total_ticks: AtomicU64,
    /// Total messages processed
    total_messages: AtomicU64,
}

impl Nexus {
    /// Create new NEXUS instance
    pub fn new(boot_id: u64, config: NexusConfig) -> Self {
        let clock = NexusClock::new(config.tick_duration);
        let bus = MessageBus::new();
        let domains = DomainManager::new(&config);

        Self {
            identity: NexusIdentity::new(boot_id),
            config,
            state: NexusState::Uninitialized,
            clock,
            bus,
            domains,
            total_ticks: AtomicU64::new(0),
            total_messages: AtomicU64::new(0),
        }
    }

    /// Get identity
    pub fn identity(&self) -> &NexusIdentity {
        &self.identity
    }

    /// Get configuration
    pub fn config(&self) -> &NexusConfig {
        &self.config
    }

    /// Get current state
    pub fn state(&self) -> NexusState {
        self.state
    }

    /// Get clock
    pub fn clock(&self) -> &NexusClock {
        &self.clock
    }

    /// Get message bus
    pub fn bus(&self) -> &MessageBus {
        &self.bus
    }

    /// Get mutable message bus
    pub fn bus_mut(&mut self) -> &mut MessageBus {
        &mut self.bus
    }

    /// Get domain manager
    pub fn domains(&self) -> &DomainManager {
        &self.domains
    }

    /// Get mutable domain manager
    pub fn domains_mut(&mut self) -> &mut DomainManager {
        &mut self.domains
    }

    /// Initialize NEXUS
    pub fn init(&mut self) -> NexusResult<()> {
        if !self.state.can_transition_to(&NexusState::Initializing) {
            return Err(NexusError::new(
                ErrorCode::InvalidState,
                alloc::format!("Cannot initialize from state: {}", self.state.name()),
            ));
        }

        self.state = NexusState::Initializing;

        // Initialize bus
        self.bus.start();

        // Initialize clock
        self.clock.start();

        // TODO: Initialize each domain

        self.state = NexusState::Ready;
        Ok(())
    }

    /// Start NEXUS
    pub fn start(&mut self) -> NexusResult<()> {
        if !self.state.can_transition_to(&NexusState::Running) {
            return Err(NexusError::new(
                ErrorCode::InvalidState,
                alloc::format!("Cannot start from state: {}", self.state.name()),
            ));
        }

        self.state = NexusState::Running;
        Ok(())
    }

    /// Pause NEXUS
    pub fn pause(&mut self) -> NexusResult<()> {
        if !self.state.can_transition_to(&NexusState::Paused) {
            return Err(NexusError::new(
                ErrorCode::InvalidState,
                alloc::format!("Cannot pause from state: {}", self.state.name()),
            ));
        }

        self.state = NexusState::Paused;
        Ok(())
    }

    /// Resume NEXUS
    pub fn resume(&mut self) -> NexusResult<()> {
        if self.state != NexusState::Paused {
            return Err(NexusError::new(
                ErrorCode::InvalidState,
                "Can only resume from paused state",
            ));
        }

        self.state = NexusState::Running;
        Ok(())
    }

    /// Shutdown NEXUS
    pub fn shutdown(&mut self) -> NexusResult<()> {
        if !self.state.can_transition_to(&NexusState::ShuttingDown) {
            return Err(NexusError::new(
                ErrorCode::InvalidState,
                alloc::format!("Cannot shutdown from state: {}", self.state.name()),
            ));
        }

        self.state = NexusState::ShuttingDown;

        // Stop clock
        self.clock.stop();

        // Stop bus
        self.bus.stop();

        // TODO: Shutdown each domain

        self.state = NexusState::Stopped;
        Ok(())
    }

    /// Process one tick
    pub fn tick(&mut self) -> NexusResult<TickResult> {
        if self.state != NexusState::Running {
            return Ok(TickResult::skipped());
        }

        let tick_num = self.clock.tick();
        let now = self.clock.now();

        let mut result = TickResult::new(tick_num, now);

        // Process each enabled domain in order
        let enabled = self.domains.enabled_domains();

        for domain in &enabled {
            // Record tick
            self.domains.record_tick(*domain, tick_num);

            // Process messages for this domain
            let messages = self.bus.receive_all(*domain);
            result.messages_processed += messages.len() as u64;

            // TODO: Actually process messages with domain handlers
        }

        self.total_ticks.fetch_add(1, Ordering::Relaxed);
        self.total_messages
            .fetch_add(result.messages_processed, Ordering::Relaxed);

        Ok(result)
    }

    /// Get overall status
    pub fn status(&self) -> NexusStatus {
        NexusStatus {
            identity: self.identity.clone(),
            state: self.state,
            uptime: self.identity.uptime(self.clock.now()),
            current_tick: self.clock.current_tick(),
            total_ticks: self.total_ticks.load(Ordering::Relaxed),
            total_messages: self.total_messages.load(Ordering::Relaxed),
            domain_health: self.domains.overall_health(),
            bus_stats: self.bus.stats(),
        }
    }
}

/// Result of a single tick
#[derive(Debug, Clone)]
pub struct TickResult {
    /// Tick number
    pub tick: u64,
    /// Timestamp
    pub timestamp: Timestamp,
    /// Was tick processed
    pub processed: bool,
    /// Messages processed
    pub messages_processed: u64,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl TickResult {
    /// Create new result
    pub fn new(tick: u64, timestamp: Timestamp) -> Self {
        Self {
            tick,
            timestamp,
            processed: true,
            messages_processed: 0,
            errors: Vec::new(),
        }
    }

    /// Create skipped result
    pub fn skipped() -> Self {
        Self {
            tick: 0,
            timestamp: Timestamp::ZERO,
            processed: false,
            messages_processed: 0,
            errors: Vec::new(),
        }
    }
}

/// Overall NEXUS status
#[derive(Debug, Clone)]
pub struct NexusStatus {
    /// Identity
    pub identity: NexusIdentity,
    /// Current state
    pub state: NexusState,
    /// Uptime
    pub uptime: Duration,
    /// Current tick
    pub current_tick: u64,
    /// Total ticks
    pub total_ticks: u64,
    /// Total messages
    pub total_messages: u64,
    /// Domain health (0-100)
    pub domain_health: u8,
    /// Bus stats
    pub bus_stats: crate::bus::BusStats,
}

// ============================================================================
// GLOBAL INSTANCE (optional pattern)
// ============================================================================

use spin::RwLock;

/// Global NEXUS instance (thread-safe singleton)
static NEXUS: spin::Once<RwLock<Nexus>> = spin::Once::new();

/// Initialize global NEXUS instance
///
/// # Safety
/// Must only be called once during kernel initialization
pub unsafe fn init_global(boot_id: u64, config: NexusConfig) -> NexusResult<()> {
    if NEXUS.get().is_some() {
        return Err(NexusError::new(
            ErrorCode::AlreadyInitialized,
            "NEXUS already initialized",
        ));
    }

    let mut nexus = Nexus::new(boot_id, config);
    nexus.init()?;

    NEXUS.call_once(|| RwLock::new(nexus));
    Ok(())
}

/// Get reference to global NEXUS instance (read access)
///
/// # Safety
/// Must only be called after init_global
pub fn get_global() -> Option<spin::RwLockReadGuard<'static, Nexus>> {
    NEXUS.get().map(|n| n.read())
}

/// Get mutable reference to global NEXUS instance (write access)
///
/// # Safety
/// Must ensure single-threaded access or proper synchronization
pub fn get_global_mut() -> Option<spin::RwLockWriteGuard<'static, Nexus>> {
    NEXUS.get().map(|n| n.write())
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nexus_lifecycle() {
        let config = NexusConfig::default();
        let mut nexus = Nexus::new(1, config);

        assert_eq!(nexus.state(), NexusState::Uninitialized);

        nexus.init().unwrap();
        assert_eq!(nexus.state(), NexusState::Ready);

        nexus.start().unwrap();
        assert_eq!(nexus.state(), NexusState::Running);

        nexus.pause().unwrap();
        assert_eq!(nexus.state(), NexusState::Paused);

        nexus.resume().unwrap();
        assert_eq!(nexus.state(), NexusState::Running);

        nexus.shutdown().unwrap();
        assert_eq!(nexus.state(), NexusState::Stopped);
    }

    #[test]
    fn test_clock() {
        let clock = NexusClock::new(Duration::from_millis(10));
        clock.start();

        assert_eq!(clock.current_tick(), 0);

        clock.tick();
        assert_eq!(clock.current_tick(), 1);

        clock.tick();
        assert_eq!(clock.current_tick(), 2);
    }

    #[test]
    fn test_domain_manager() {
        let config = NexusConfig::default();
        let manager = DomainManager::new(&config);

        assert_eq!(manager.enabled_domains().len(), 7);
        assert_eq!(manager.overall_health(), 100);
    }

    #[test]
    fn test_state_transitions() {
        assert!(NexusState::Uninitialized.can_transition_to(&NexusState::Initializing));
        assert!(NexusState::Initializing.can_transition_to(&NexusState::Ready));
        assert!(NexusState::Ready.can_transition_to(&NexusState::Running));
        assert!(!NexusState::Stopped.can_transition_to(&NexusState::Running));
    }
}
