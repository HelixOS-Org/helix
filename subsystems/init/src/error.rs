//! # Error Handling and Rollback
//!
//! This module provides comprehensive error handling for the initialization
//! framework including error types, rollback chains, and recovery mechanisms.
//!
//! ## Error Propagation Diagram
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        ERROR PROPAGATION FLOW                            │
//! │                                                                          │
//! │  ╔═══════════════╗                                                       │
//! │  ║   Subsystem   ║──▶ InitError ──┬──▶ Log ──▶ Continue                 │
//! │  ║     Fails     ║               │                                       │
//! │  ╚═══════════════╝               ├──▶ Retry ──┬──▶ Success ──▶ Continue │
//! │                                   │           └──▶ Fail ──┐              │
//! │                                   │                       │              │
//! │                                   └──▶ Rollback ◀─────────┘              │
//! │                                           │                              │
//! │                                           ▼                              │
//! │  ┌─────────────────────────────────────────────────────────────────┐    │
//! │  │                    ROLLBACK CHAIN                                │    │
//! │  │                                                                  │    │
//! │  │  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐      │    │
//! │  │  │ Subsys  │◀───│ Subsys  │◀───│ Subsys  │◀───│ Failed  │      │    │
//! │  │  │    A    │    │    B    │    │    C    │    │ Subsys  │      │    │
//! │  │  │(rollbak)│    │(rollbak)│    │(rollbak)│    │         │      │    │
//! │  │  └─────────┘    └─────────┘    └─────────┘    └─────────┘      │    │
//! │  │       │              │              │                          │    │
//! │  │       ▼              ▼              ▼                          │    │
//! │  │   cleanup()     cleanup()      cleanup()                       │    │
//! │  │                                                                  │    │
//! │  └─────────────────────────────────────────────────────────────────┘    │
//! │                                                                          │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Error Categories
//!
//! | Category | Severity | Retry | Rollback | Example |
//! |----------|----------|-------|----------|---------|
//! | Timeout | Medium | Yes | Maybe | Hardware not ready |
//! | Resource | High | No | Yes | OOM during init |
//! | Dependency | High | No | Yes | Required subsystem failed |
//! | Hardware | Critical | No | Partial | Device not found |
//! | Config | Medium | No | No | Invalid configuration |
//! | Internal | Critical | No | Yes | Bug in init code |

use core::fmt;
use core::sync::atomic::{AtomicU32, Ordering};

use crate::phase::InitPhase;
use crate::subsystem::SubsystemId;

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// ERROR KIND
// =============================================================================

/// Classification of initialization errors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum ErrorKind {
    // -------------------------------------------------------------------------
    // General Errors (0-99)
    // -------------------------------------------------------------------------
    /// Unknown or unclassified error
    Unknown              = 0,

    /// Operation timed out
    Timeout              = 1,

    /// Invalid state for operation
    InvalidState         = 2,

    /// Invalid argument provided
    InvalidArgument      = 3,

    /// Operation not supported
    NotSupported         = 4,

    /// Permission denied
    PermissionDenied     = 5,

    /// Already exists
    AlreadyExists        = 6,

    /// Not found
    NotFound             = 7,

    /// Would block (non-blocking context)
    WouldBlock           = 8,

    /// Interrupted by signal/event
    Interrupted          = 9,

    // -------------------------------------------------------------------------
    // Resource Errors (100-199)
    // -------------------------------------------------------------------------
    /// Out of memory
    OutOfMemory          = 100,

    /// Resource exhausted
    ResourceExhausted    = 101,

    /// Resource busy
    ResourceBusy         = 102,

    /// Resource locked
    ResourceLocked       = 103,

    /// Resource unavailable
    ResourceUnavailable  = 104,

    /// Allocation failed
    AllocationFailed     = 105,

    /// Memory map failed
    MappingFailed        = 106,

    // -------------------------------------------------------------------------
    // Subsystem Errors (200-299)
    // -------------------------------------------------------------------------
    /// Subsystem initialization failed
    SubsystemFailed      = 200,

    /// Subsystem not found
    SubsystemNotFound    = 201,

    /// Subsystem already initialized
    SubsystemAlreadyInitialized = 202,

    /// Subsystem not initialized
    SubsystemNotInitialized = 203,

    /// Subsystem shutdown failed
    SubsystemShutdownFailed = 204,

    /// Subsystem registration failed
    SubsystemRegistrationFailed = 205,

    /// Subsystem validation failed
    SubsystemValidationFailed = 206,

    // -------------------------------------------------------------------------
    // Dependency Errors (300-399)
    // -------------------------------------------------------------------------
    /// Dependency not satisfied
    DependencyNotSatisfied = 300,

    /// Circular dependency detected
    CircularDependency   = 301,

    /// Dependency version mismatch
    DependencyVersionMismatch = 302,

    /// Dependency failed
    DependencyFailed     = 303,

    /// Dependency timeout
    DependencyTimeout    = 304,

    /// Required dependency missing
    MissingDependency    = 305,

    /// Optional dependency unavailable
    OptionalDependencyUnavailable = 306,

    // -------------------------------------------------------------------------
    // Phase Errors (400-499)
    // -------------------------------------------------------------------------
    /// Wrong phase for operation
    WrongPhase           = 400,

    /// Phase transition failed
    PhaseTransitionFailed = 401,

    /// Phase barrier timeout
    PhaseBarrierTimeout  = 402,

    /// Phase already complete
    PhaseAlreadyComplete = 403,

    /// Phase not started
    PhaseNotStarted      = 404,

    /// Phase requirements not met
    PhaseRequirementsNotMet = 405,

    // -------------------------------------------------------------------------
    // Hardware Errors (500-599)
    // -------------------------------------------------------------------------
    /// Hardware not found
    HardwareNotFound     = 500,

    /// Hardware initialization failed
    HardwareInitFailed   = 501,

    /// Hardware not responding
    HardwareNotResponding = 502,

    /// Hardware error
    HardwareError        = 503,

    /// Device driver error
    DriverError          = 504,

    /// Firmware error
    FirmwareError        = 505,

    /// Bus error
    BusError             = 506,

    // -------------------------------------------------------------------------
    // Configuration Errors (600-699)
    // -------------------------------------------------------------------------
    /// Invalid configuration
    InvalidConfig        = 600,

    /// Missing configuration
    MissingConfig        = 601,

    /// Configuration parse error
    ConfigParseError     = 602,

    /// Configuration validation error
    ConfigValidationError = 603,

    // -------------------------------------------------------------------------
    // Rollback Errors (700-799)
    // -------------------------------------------------------------------------
    /// Rollback failed
    RollbackFailed       = 700,

    /// Rollback not possible
    RollbackNotPossible  = 701,

    /// Partial rollback
    PartialRollback      = 702,

    /// Rollback timeout
    RollbackTimeout      = 703,

    // -------------------------------------------------------------------------
    // Internal Errors (800-899)
    // -------------------------------------------------------------------------
    /// Internal error (bug)
    InternalError        = 800,

    /// Assertion failed
    AssertionFailed      = 801,

    /// Invariant violation
    InvariantViolation   = 802,

    /// Stack overflow
    StackOverflow        = 803,

    /// Data corruption
    DataCorruption       = 804,
}

impl ErrorKind {
    /// Get the error category name
    pub const fn category(&self) -> &'static str {
        match *self as u32 {
            0..=99 => "General",
            100..=199 => "Resource",
            200..=299 => "Subsystem",
            300..=399 => "Dependency",
            400..=499 => "Phase",
            500..=599 => "Hardware",
            600..=699 => "Configuration",
            700..=799 => "Rollback",
            800..=899 => "Internal",
            _ => "Unknown",
        }
    }

    /// Check if error is recoverable
    pub const fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ErrorKind::Timeout
                | ErrorKind::WouldBlock
                | ErrorKind::ResourceBusy
                | ErrorKind::ResourceLocked
                | ErrorKind::Interrupted
                | ErrorKind::HardwareNotResponding
                | ErrorKind::OptionalDependencyUnavailable
        )
    }

    /// Check if error should trigger rollback
    pub const fn should_rollback(&self) -> bool {
        matches!(
            self,
            ErrorKind::OutOfMemory
                | ErrorKind::ResourceExhausted
                | ErrorKind::AllocationFailed
                | ErrorKind::MappingFailed
                | ErrorKind::SubsystemFailed
                | ErrorKind::DependencyNotSatisfied
                | ErrorKind::DependencyFailed
                | ErrorKind::MissingDependency
                | ErrorKind::HardwareInitFailed
                | ErrorKind::InternalError
                | ErrorKind::DataCorruption
        )
    }

    /// Check if error is critical (kernel cannot continue)
    pub const fn is_critical(&self) -> bool {
        matches!(
            self,
            ErrorKind::OutOfMemory
                | ErrorKind::InternalError
                | ErrorKind::DataCorruption
                | ErrorKind::StackOverflow
                | ErrorKind::InvariantViolation
        )
    }

    /// Get suggested retry count
    pub const fn retry_count(&self) -> u32 {
        match self {
            ErrorKind::Timeout => 3,
            ErrorKind::WouldBlock => 5,
            ErrorKind::ResourceBusy => 3,
            ErrorKind::HardwareNotResponding => 2,
            ErrorKind::Interrupted => 5,
            _ => 0,
        }
    }

    /// Get suggested retry delay in microseconds
    pub const fn retry_delay_us(&self) -> u64 {
        match self {
            ErrorKind::Timeout => 100_000,               // 100ms
            ErrorKind::WouldBlock => 1_000,              // 1ms
            ErrorKind::ResourceBusy => 10_000,           // 10ms
            ErrorKind::HardwareNotResponding => 500_000, // 500ms
            _ => 0,
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

// =============================================================================
// INIT ERROR
// =============================================================================

/// Comprehensive initialization error
#[derive(Debug)]
pub struct InitError {
    /// Error classification
    kind: ErrorKind,

    /// Human-readable message
    message: &'static str,

    /// Optional detailed message (heap allocated)
    details: Option<String>,

    /// Subsystem that caused the error
    subsystem: Option<SubsystemId>,

    /// Phase where error occurred
    phase: Option<InitPhase>,

    /// Source error (chained)
    source: Option<Box<InitError>>,

    /// Backtrace (if available)
    #[cfg(feature = "backtrace")]
    backtrace: Option<alloc::vec::Vec<usize>>,

    /// Retry count (if retried)
    retry_count: u32,

    /// Error code (for external reference)
    error_code: u32,
}

impl InitError {
    /// Create new error with kind and message
    pub const fn new(kind: ErrorKind, message: &'static str) -> Self {
        Self {
            kind,
            message,
            details: None,
            subsystem: None,
            phase: None,
            source: None,
            #[cfg(feature = "backtrace")]
            backtrace: None,
            retry_count: 0,
            error_code: 0,
        }
    }

    /// Create error from kind with default message
    pub fn from_kind(kind: ErrorKind) -> Self {
        Self::new(kind, kind_to_message(kind))
    }

    /// Add details to error
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Add subsystem to error
    pub fn with_subsystem(mut self, subsystem: SubsystemId) -> Self {
        self.subsystem = Some(subsystem);
        self
    }

    /// Add phase to error
    pub fn with_phase(mut self, phase: InitPhase) -> Self {
        self.phase = Some(phase);
        self
    }

    /// Add source error
    pub fn with_source(mut self, source: InitError) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Set retry count
    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    /// Set error code
    pub fn with_error_code(mut self, code: u32) -> Self {
        self.error_code = code;
        self
    }

    /// Get error kind
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// Get message
    pub fn message(&self) -> &str {
        self.message
    }

    /// Get details
    pub fn details(&self) -> Option<&str> {
        self.details.as_deref()
    }

    /// Get subsystem
    pub fn subsystem(&self) -> Option<SubsystemId> {
        self.subsystem
    }

    /// Get phase
    pub fn phase(&self) -> Option<InitPhase> {
        self.phase
    }

    /// Get source error
    pub fn source(&self) -> Option<&InitError> {
        self.source.as_deref()
    }

    /// Get retry count
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Get error code
    pub fn error_code(&self) -> u32 {
        self.error_code
    }

    /// Check if recoverable
    pub fn is_recoverable(&self) -> bool {
        self.kind.is_recoverable()
    }

    /// Check if should rollback
    pub fn should_rollback(&self) -> bool {
        self.kind.should_rollback()
    }

    /// Check if critical
    pub fn is_critical(&self) -> bool {
        self.kind.is_critical()
    }

    /// Get full error chain
    pub fn chain(&self) -> ErrorChain<'_> {
        ErrorChain {
            current: Some(self),
        }
    }
}

impl fmt::Display for InitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.kind.category(), self.message)?;

        if let Some(ref details) = self.details {
            write!(f, ": {}", details)?;
        }

        if let Some(subsystem) = self.subsystem {
            write!(f, " (subsystem: {})", subsystem.0)?;
        }

        if let Some(phase) = self.phase {
            write!(f, " (phase: {})", phase)?;
        }

        Ok(())
    }
}

/// Iterator over error chain
pub struct ErrorChain<'a> {
    current: Option<&'a InitError>,
}

impl<'a> Iterator for ErrorChain<'a> {
    type Item = &'a InitError;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        self.current = current.source.as_deref();
        Some(current)
    }
}

/// Get default message for error kind
const fn kind_to_message(kind: ErrorKind) -> &'static str {
    match kind {
        ErrorKind::Unknown => "Unknown error",
        ErrorKind::Timeout => "Operation timed out",
        ErrorKind::InvalidState => "Invalid state",
        ErrorKind::InvalidArgument => "Invalid argument",
        ErrorKind::NotSupported => "Not supported",
        ErrorKind::PermissionDenied => "Permission denied",
        ErrorKind::AlreadyExists => "Already exists",
        ErrorKind::NotFound => "Not found",
        ErrorKind::WouldBlock => "Would block",
        ErrorKind::Interrupted => "Interrupted",
        ErrorKind::OutOfMemory => "Out of memory",
        ErrorKind::ResourceExhausted => "Resource exhausted",
        ErrorKind::ResourceBusy => "Resource busy",
        ErrorKind::ResourceLocked => "Resource locked",
        ErrorKind::ResourceUnavailable => "Resource unavailable",
        ErrorKind::AllocationFailed => "Allocation failed",
        ErrorKind::MappingFailed => "Memory mapping failed",
        ErrorKind::SubsystemFailed => "Subsystem failed",
        ErrorKind::SubsystemNotFound => "Subsystem not found",
        ErrorKind::SubsystemAlreadyInitialized => "Subsystem already initialized",
        ErrorKind::SubsystemNotInitialized => "Subsystem not initialized",
        ErrorKind::SubsystemShutdownFailed => "Subsystem shutdown failed",
        ErrorKind::SubsystemRegistrationFailed => "Subsystem registration failed",
        ErrorKind::SubsystemValidationFailed => "Subsystem validation failed",
        ErrorKind::DependencyNotSatisfied => "Dependency not satisfied",
        ErrorKind::CircularDependency => "Circular dependency",
        ErrorKind::DependencyVersionMismatch => "Dependency version mismatch",
        ErrorKind::DependencyFailed => "Dependency failed",
        ErrorKind::DependencyTimeout => "Dependency timeout",
        ErrorKind::MissingDependency => "Missing dependency",
        ErrorKind::OptionalDependencyUnavailable => "Optional dependency unavailable",
        ErrorKind::WrongPhase => "Wrong phase",
        ErrorKind::PhaseTransitionFailed => "Phase transition failed",
        ErrorKind::PhaseBarrierTimeout => "Phase barrier timeout",
        ErrorKind::PhaseAlreadyComplete => "Phase already complete",
        ErrorKind::PhaseNotStarted => "Phase not started",
        ErrorKind::PhaseRequirementsNotMet => "Phase requirements not met",
        ErrorKind::HardwareNotFound => "Hardware not found",
        ErrorKind::HardwareInitFailed => "Hardware initialization failed",
        ErrorKind::HardwareNotResponding => "Hardware not responding",
        ErrorKind::HardwareError => "Hardware error",
        ErrorKind::DriverError => "Driver error",
        ErrorKind::FirmwareError => "Firmware error",
        ErrorKind::BusError => "Bus error",
        ErrorKind::InvalidConfig => "Invalid configuration",
        ErrorKind::MissingConfig => "Missing configuration",
        ErrorKind::ConfigParseError => "Configuration parse error",
        ErrorKind::ConfigValidationError => "Configuration validation error",
        ErrorKind::RollbackFailed => "Rollback failed",
        ErrorKind::RollbackNotPossible => "Rollback not possible",
        ErrorKind::PartialRollback => "Partial rollback",
        ErrorKind::RollbackTimeout => "Rollback timeout",
        ErrorKind::InternalError => "Internal error",
        ErrorKind::AssertionFailed => "Assertion failed",
        ErrorKind::InvariantViolation => "Invariant violation",
        ErrorKind::StackOverflow => "Stack overflow",
        ErrorKind::DataCorruption => "Data corruption",
    }
}

// =============================================================================
// RESULT TYPE
// =============================================================================

/// Result type for initialization operations
pub type InitResult<T> = Result<T, InitError>;

// =============================================================================
// ROLLBACK CHAIN
// =============================================================================

/// Rollback action trait
pub trait RollbackAction: Send + Sync {
    /// Execute the rollback action
    fn execute(&self) -> InitResult<()>;

    /// Get description of the action
    fn description(&self) -> &str;

    /// Check if action is critical (must succeed)
    fn is_critical(&self) -> bool {
        false
    }
}

/// A chain of rollback actions to execute on failure
pub struct RollbackChain {
    /// Stack of rollback actions (LIFO order)
    actions: Vec<RollbackEntry>,

    /// Whether chain is active
    active: bool,

    /// Statistics
    executed: AtomicU32,
    succeeded: AtomicU32,
    failed: AtomicU32,
}

/// Entry in the rollback chain
struct RollbackEntry {
    /// Subsystem ID (if applicable)
    subsystem: Option<SubsystemId>,

    /// Phase (if applicable)
    phase: Option<InitPhase>,

    /// The rollback action
    action: Box<dyn RollbackAction>,

    /// Priority (higher = earlier in rollback)
    priority: i32,
}

impl RollbackChain {
    /// Create new rollback chain
    pub fn new() -> Self {
        Self {
            actions: Vec::new(),
            active: true,
            executed: AtomicU32::new(0),
            succeeded: AtomicU32::new(0),
            failed: AtomicU32::new(0),
        }
    }

    /// Push a rollback action onto the chain
    pub fn push<A: RollbackAction + 'static>(&mut self, action: A) {
        if self.active {
            self.actions.push(RollbackEntry {
                subsystem: None,
                phase: None,
                action: Box::new(action),
                priority: 0,
            });
        }
    }

    /// Push action with subsystem context
    pub fn push_for_subsystem<A: RollbackAction + 'static>(
        &mut self,
        subsystem: SubsystemId,
        action: A,
    ) {
        if self.active {
            self.actions.push(RollbackEntry {
                subsystem: Some(subsystem),
                phase: None,
                action: Box::new(action),
                priority: 0,
            });
        }
    }

    /// Push action with phase and priority
    pub fn push_with_priority<A: RollbackAction + 'static>(
        &mut self,
        phase: InitPhase,
        priority: i32,
        action: A,
    ) {
        if self.active {
            self.actions.push(RollbackEntry {
                subsystem: None,
                phase: Some(phase),
                action: Box::new(action),
                priority,
            });
        }
    }

    /// Execute all rollback actions
    pub fn execute(&mut self) -> InitResult<()> {
        let mut last_error: Option<InitError> = None;

        // Sort by priority (descending) and then by insertion order (descending = LIFO)
        self.actions.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Execute in LIFO order
        while let Some(entry) = self.actions.pop() {
            self.executed.fetch_add(1, Ordering::Relaxed);

            match entry.action.execute() {
                Ok(()) => {
                    self.succeeded.fetch_add(1, Ordering::Relaxed);
                },
                Err(e) => {
                    self.failed.fetch_add(1, Ordering::Relaxed);

                    if entry.action.is_critical() {
                        return Err(InitError::new(
                            ErrorKind::RollbackFailed,
                            "Critical rollback action failed",
                        )
                        .with_source(e));
                    }

                    last_error = Some(e);
                },
            }
        }

        // Return last non-critical error if any
        if self.failed.load(Ordering::Relaxed) > 0 {
            Err(last_error.unwrap_or_else(|| InitError::from_kind(ErrorKind::PartialRollback)))
        } else {
            Ok(())
        }
    }

    /// Execute rollback for a specific subsystem only
    pub fn execute_for_subsystem(&mut self, subsystem: SubsystemId) -> InitResult<()> {
        let (matching, remaining): (Vec<_>, Vec<_>) = self
            .actions
            .drain(..)
            .partition(|e| e.subsystem == Some(subsystem));

        self.actions = remaining;

        for entry in matching.into_iter().rev() {
            entry.action.execute()?;
        }

        Ok(())
    }

    /// Execute rollback for a specific phase
    pub fn execute_for_phase(&mut self, phase: InitPhase) -> InitResult<()> {
        let (matching, remaining): (Vec<_>, Vec<_>) =
            self.actions.drain(..).partition(|e| e.phase == Some(phase));

        self.actions = remaining;

        let mut sorted: Vec<_> = matching.into_iter().collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));

        for entry in sorted {
            entry.action.execute()?;
        }

        Ok(())
    }

    /// Clear the chain without executing
    pub fn clear(&mut self) {
        self.actions.clear();
    }

    /// Disarm the chain (prevent new additions)
    pub fn disarm(&mut self) {
        self.active = false;
    }

    /// Rearm the chain
    pub fn rearm(&mut self) {
        self.active = true;
    }

    /// Get number of pending actions
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Get statistics
    pub fn stats(&self) -> RollbackStats {
        RollbackStats {
            executed: self.executed.load(Ordering::Relaxed),
            succeeded: self.succeeded.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            pending: self.actions.len() as u32,
        }
    }
}

impl Default for RollbackChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Rollback statistics
#[derive(Debug, Clone, Copy)]
pub struct RollbackStats {
    /// Actions executed
    pub executed: u32,
    /// Successful actions
    pub succeeded: u32,
    /// Failed actions
    pub failed: u32,
    /// Pending actions
    pub pending: u32,
}

// =============================================================================
// SIMPLE ROLLBACK ACTIONS
// =============================================================================

/// No-op rollback action
pub struct NoOpRollback;

impl RollbackAction for NoOpRollback {
    fn execute(&self) -> InitResult<()> {
        Ok(())
    }

    fn description(&self) -> &str {
        "No-op"
    }
}

/// Function-based rollback action
pub struct FnRollback<F: Fn() -> InitResult<()> + Send + Sync> {
    func: F,
    desc: &'static str,
    critical: bool,
}

impl<F: Fn() -> InitResult<()> + Send + Sync> FnRollback<F> {
    /// Create new function rollback
    pub fn new(func: F, desc: &'static str) -> Self {
        Self {
            func,
            desc,
            critical: false,
        }
    }

    /// Mark as critical
    pub fn critical(mut self) -> Self {
        self.critical = true;
        self
    }
}

impl<F: Fn() -> InitResult<()> + Send + Sync> RollbackAction for FnRollback<F> {
    fn execute(&self) -> InitResult<()> {
        (self.func)()
    }

    fn description(&self) -> &str {
        self.desc
    }

    fn is_critical(&self) -> bool {
        self.critical
    }
}

// =============================================================================
// ERROR HANDLER
// =============================================================================

/// Error handling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorPolicy {
    /// Panic on error
    Panic,
    /// Log and continue
    LogAndContinue,
    /// Retry with backoff
    Retry,
    /// Rollback and abort
    Rollback,
    /// Custom handler
    Custom,
}

/// Error handler callback type
pub type ErrorHandlerFn = fn(&InitError) -> ErrorPolicy;

/// Global error handler
pub struct ErrorHandler {
    /// Default policy
    default_policy: ErrorPolicy,

    /// Custom handler
    handler: Option<ErrorHandlerFn>,

    /// Maximum retries for Retry policy
    max_retries: u32,

    /// Base retry delay in microseconds
    retry_delay_us: u64,

    /// Whether to collect errors
    collect_errors: bool,

    /// Collected errors
    errors: Vec<InitError>,
}

impl ErrorHandler {
    /// Create new error handler with default policy
    pub fn new(policy: ErrorPolicy) -> Self {
        Self {
            default_policy: policy,
            handler: None,
            max_retries: 3,
            retry_delay_us: 10_000,
            collect_errors: false,
            errors: Vec::new(),
        }
    }

    /// Set custom handler
    pub fn with_handler(mut self, handler: ErrorHandlerFn) -> Self {
        self.handler = Some(handler);
        self
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, max_retries: u32, delay_us: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_delay_us = delay_us;
        self
    }

    /// Enable error collection
    pub fn collect(mut self) -> Self {
        self.collect_errors = true;
        self
    }

    /// Handle an error
    pub fn handle(&mut self, error: InitError) -> ErrorPolicy {
        if self.collect_errors {
            self.errors.push(InitError::new(error.kind, error.message));
        }

        // Check custom handler first
        if let Some(handler) = self.handler {
            let policy = handler(&error);
            if policy != ErrorPolicy::Custom {
                return policy;
            }
        }

        // Use error kind's suggestion or default policy
        if error.is_critical() {
            ErrorPolicy::Panic
        } else if error.is_recoverable() && self.default_policy == ErrorPolicy::Retry {
            ErrorPolicy::Retry
        } else {
            self.default_policy
        }
    }

    /// Get collected errors
    pub fn errors(&self) -> &[InitError] {
        &self.errors
    }

    /// Clear collected errors
    pub fn clear_errors(&mut self) {
        self.errors.clear();
    }

    /// Get max retries
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Get retry delay
    pub fn retry_delay_us(&self) -> u64 {
        self.retry_delay_us
    }
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new(ErrorPolicy::Rollback)
    }
}

// =============================================================================
// ERROR MACROS
// =============================================================================

/// Create an InitError with current location info
#[macro_export]
macro_rules! init_error {
    ($kind:expr, $msg:literal) => {
        $crate::error::InitError::new($kind, $msg)
    };
    ($kind:expr, $msg:literal, $($arg:tt)*) => {
        $crate::error::InitError::new($kind, $msg)
            .with_details(alloc::format!($($arg)*))
    };
}

/// Return early with an error
#[macro_export]
macro_rules! init_bail {
    ($kind:expr, $msg:literal) => {
        return Err($crate::init_error!($kind, $msg))
    };
    ($kind:expr, $msg:literal, $($arg:tt)*) => {
        return Err($crate::init_error!($kind, $msg, $($arg)*))
    };
}

/// Ensure a condition is true, otherwise return error
#[macro_export]
macro_rules! init_ensure {
    ($cond:expr, $kind:expr, $msg:literal) => {
        if !$cond {
            $crate::init_bail!($kind, $msg);
        }
    };
    ($cond:expr, $kind:expr, $msg:literal, $($arg:tt)*) => {
        if !$cond {
            $crate::init_bail!($kind, $msg, $($arg)*);
        }
    };
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_kind_categories() {
        assert_eq!(ErrorKind::Timeout.category(), "General");
        assert_eq!(ErrorKind::OutOfMemory.category(), "Resource");
        assert_eq!(ErrorKind::SubsystemFailed.category(), "Subsystem");
        assert_eq!(ErrorKind::CircularDependency.category(), "Dependency");
        assert_eq!(ErrorKind::WrongPhase.category(), "Phase");
        assert_eq!(ErrorKind::HardwareNotFound.category(), "Hardware");
        assert_eq!(ErrorKind::InvalidConfig.category(), "Configuration");
        assert_eq!(ErrorKind::RollbackFailed.category(), "Rollback");
        assert_eq!(ErrorKind::InternalError.category(), "Internal");
    }

    #[test]
    fn test_error_kind_properties() {
        assert!(ErrorKind::Timeout.is_recoverable());
        assert!(!ErrorKind::InternalError.is_recoverable());

        assert!(ErrorKind::OutOfMemory.should_rollback());
        assert!(!ErrorKind::Timeout.should_rollback());

        assert!(ErrorKind::InternalError.is_critical());
        assert!(!ErrorKind::Timeout.is_critical());
    }

    #[test]
    fn test_init_error_creation() {
        let err = InitError::new(ErrorKind::Timeout, "Test timeout")
            .with_phase(InitPhase::Core)
            .with_retry_count(2);

        assert_eq!(err.kind(), ErrorKind::Timeout);
        assert_eq!(err.message(), "Test timeout");
        assert_eq!(err.phase(), Some(InitPhase::Core));
        assert_eq!(err.retry_count(), 2);
    }

    #[test]
    fn test_error_chain() {
        let inner = InitError::new(ErrorKind::HardwareError, "Hardware failed");
        let outer =
            InitError::new(ErrorKind::SubsystemFailed, "Driver init failed").with_source(inner);

        let chain: Vec<_> = outer.chain().collect();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].kind(), ErrorKind::SubsystemFailed);
        assert_eq!(chain[1].kind(), ErrorKind::HardwareError);
    }

    #[test]
    fn test_rollback_chain() {
        use core::sync::atomic::{AtomicBool, Ordering};
        static EXECUTED: AtomicBool = AtomicBool::new(false);

        struct TestRollback;
        impl RollbackAction for TestRollback {
            fn execute(&self) -> InitResult<()> {
                EXECUTED.store(true, Ordering::SeqCst);
                Ok(())
            }
            fn description(&self) -> &str {
                "Test rollback"
            }
        }

        let mut chain = RollbackChain::new();
        chain.push(TestRollback);

        assert_eq!(chain.len(), 1);
        assert!(chain.execute().is_ok());
        assert!(EXECUTED.load(Ordering::SeqCst));
        assert!(chain.is_empty());
    }

    #[test]
    fn test_error_handler() {
        let mut handler = ErrorHandler::new(ErrorPolicy::LogAndContinue);

        let err = InitError::new(ErrorKind::Timeout, "Test");
        let policy = handler.handle(err);

        // Timeout is recoverable, but policy is LogAndContinue (not Retry)
        assert_eq!(policy, ErrorPolicy::LogAndContinue);

        let critical_err = InitError::new(ErrorKind::InternalError, "Critical");
        let policy = handler.handle(critical_err);

        // Critical errors always panic
        assert_eq!(policy, ErrorPolicy::Panic);
    }
}
