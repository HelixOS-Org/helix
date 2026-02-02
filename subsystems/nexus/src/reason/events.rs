//! Causal Events
//!
//! This module provides event types and structures for causal reasoning.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::CausalEventId;

/// Causal event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalEventType {
    /// System call
    Syscall,
    /// Interrupt
    Interrupt,
    /// Page fault
    PageFault,
    /// Context switch
    ContextSwitch,
    /// Memory allocation
    MemoryAlloc,
    /// Memory free
    MemoryFree,
    /// Lock acquire
    LockAcquire,
    /// Lock release
    LockRelease,
    /// Signal sent
    SignalSent,
    /// Signal received
    SignalReceived,
    /// Process created
    ProcessCreated,
    /// Process terminated
    ProcessTerminated,
    /// File opened
    FileOpened,
    /// File closed
    FileClosed,
    /// Network packet sent
    PacketSent,
    /// Network packet received
    PacketReceived,
    /// Timer fired
    TimerFired,
    /// Error occurred
    ErrorOccurred,
    /// Warning issued
    WarningIssued,
    /// State changed
    StateChanged,
    /// Configuration changed
    ConfigChanged,
    /// Resource exhausted
    ResourceExhausted,
    /// Threshold exceeded
    ThresholdExceeded,
    /// Anomaly detected
    AnomalyDetected,
    /// Recovery attempted
    RecoveryAttempted,
    /// Recovery succeeded
    RecoverySucceeded,
    /// Recovery failed
    RecoveryFailed,
    /// Custom event
    Custom,
}

impl CausalEventType {
    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Syscall => "syscall",
            Self::Interrupt => "interrupt",
            Self::PageFault => "page_fault",
            Self::ContextSwitch => "context_switch",
            Self::MemoryAlloc => "memory_alloc",
            Self::MemoryFree => "memory_free",
            Self::LockAcquire => "lock_acquire",
            Self::LockRelease => "lock_release",
            Self::SignalSent => "signal_sent",
            Self::SignalReceived => "signal_received",
            Self::ProcessCreated => "process_created",
            Self::ProcessTerminated => "process_terminated",
            Self::FileOpened => "file_opened",
            Self::FileClosed => "file_closed",
            Self::PacketSent => "packet_sent",
            Self::PacketReceived => "packet_received",
            Self::TimerFired => "timer_fired",
            Self::ErrorOccurred => "error_occurred",
            Self::WarningIssued => "warning_issued",
            Self::StateChanged => "state_changed",
            Self::ConfigChanged => "config_changed",
            Self::ResourceExhausted => "resource_exhausted",
            Self::ThresholdExceeded => "threshold_exceeded",
            Self::AnomalyDetected => "anomaly_detected",
            Self::RecoveryAttempted => "recovery_attempted",
            Self::RecoverySucceeded => "recovery_succeeded",
            Self::RecoveryFailed => "recovery_failed",
            Self::Custom => "custom",
        }
    }

    /// Is error type
    pub fn is_error(&self) -> bool {
        matches!(
            self,
            Self::ErrorOccurred | Self::RecoveryFailed | Self::ResourceExhausted
        )
    }

    /// Is state change
    pub fn is_state_change(&self) -> bool {
        matches!(
            self,
            Self::StateChanged
                | Self::ConfigChanged
                | Self::ProcessCreated
                | Self::ProcessTerminated
                | Self::LockAcquire
                | Self::LockRelease
        )
    }

    /// Is warning or error
    pub fn is_warning_or_error(&self) -> bool {
        matches!(
            self,
            Self::WarningIssued
                | Self::ErrorOccurred
                | Self::RecoveryFailed
                | Self::ResourceExhausted
                | Self::AnomalyDetected
                | Self::ThresholdExceeded
        )
    }
}

/// Event severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    /// Debug level
    Debug    = 0,
    /// Info level
    Info     = 1,
    /// Warning level
    Warning  = 2,
    /// Error level
    Error    = 3,
    /// Critical level
    Critical = 4,
    /// Fatal level
    Fatal    = 5,
}

impl EventSeverity {
    /// Get severity name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Critical => "critical",
            Self::Fatal => "fatal",
        }
    }

    /// Is severe (error or above)
    pub fn is_severe(&self) -> bool {
        *self >= Self::Error
    }
}

/// Causal event
#[derive(Debug, Clone)]
pub struct CausalEvent {
    /// Event ID
    pub id: CausalEventId,
    /// Event type
    pub event_type: CausalEventType,
    /// Timestamp (nanoseconds since boot)
    pub timestamp: u64,
    /// Severity
    pub severity: EventSeverity,
    /// Process ID (if applicable)
    pub pid: Option<u32>,
    /// Thread ID (if applicable)
    pub tid: Option<u32>,
    /// CPU ID
    pub cpu: Option<u32>,
    /// Description
    pub description: String,
    /// Properties
    pub properties: BTreeMap<String, String>,
    /// Stack trace (if available)
    pub stack_trace: Option<Vec<u64>>,
}

impl CausalEvent {
    /// Create new event
    pub fn new(id: CausalEventId, event_type: CausalEventType, timestamp: u64) -> Self {
        Self {
            id,
            event_type,
            timestamp,
            severity: EventSeverity::Info,
            pid: None,
            tid: None,
            cpu: None,
            description: String::new(),
            properties: BTreeMap::new(),
            stack_trace: None,
        }
    }

    /// With severity
    pub fn with_severity(mut self, severity: EventSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// With description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    /// With property
    pub fn with_property(mut self, key: &str, value: &str) -> Self {
        self.properties
            .insert(String::from(key), String::from(value));
        self
    }

    /// With process
    pub fn with_process(mut self, pid: u32, tid: u32) -> Self {
        self.pid = Some(pid);
        self.tid = Some(tid);
        self
    }

    /// With CPU
    pub fn with_cpu(mut self, cpu: u32) -> Self {
        self.cpu = Some(cpu);
        self
    }

    /// With stack trace
    pub fn with_stack_trace(mut self, trace: Vec<u64>) -> Self {
        self.stack_trace = Some(trace);
        self
    }
}
