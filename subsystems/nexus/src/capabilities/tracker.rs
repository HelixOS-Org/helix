//! Capability Tracker
//!
//! Tracking capability events and process capabilities.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{Capability, CapabilitySet, Pid, ProcessCaps};

/// Capability event
#[derive(Debug, Clone)]
pub struct CapabilityEvent {
    /// Timestamp
    pub timestamp: u64,
    /// Process ID
    pub pid: Pid,
    /// Event type
    pub event_type: CapEventType,
    /// Capability
    pub capability: Option<Capability>,
    /// Old set
    pub old_set: Option<CapabilitySet>,
    /// New set
    pub new_set: Option<CapabilitySet>,
    /// Success
    pub success: bool,
}

/// Capability event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapEventType {
    /// Capability raised
    Raised,
    /// Capability dropped
    Dropped,
    /// Capability check
    Check,
    /// Capability denied
    Denied,
    /// Set modified
    SetModified,
    /// Bounds dropped
    BoundsDropped,
    /// Exec transition
    ExecTransition,
    /// Setuid transition
    SetuidTransition,
}

impl CapEventType {
    /// Get event name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Raised => "raised",
            Self::Dropped => "dropped",
            Self::Check => "check",
            Self::Denied => "denied",
            Self::SetModified => "set_modified",
            Self::BoundsDropped => "bounds_dropped",
            Self::ExecTransition => "exec_transition",
            Self::SetuidTransition => "setuid_transition",
        }
    }
}

/// Capability tracker
pub struct CapabilityTracker {
    /// Events
    events: Vec<CapabilityEvent>,
    /// Max events
    max_events: usize,
    /// Process caps
    pub(crate) process_caps: BTreeMap<Pid, ProcessCaps>,
    /// Total events
    total_events: AtomicU64,
    /// Denial count
    denial_count: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl CapabilityTracker {
    /// Create new tracker
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::new(),
            max_events,
            process_caps: BTreeMap::new(),
            total_events: AtomicU64::new(0),
            denial_count: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    /// Record event
    pub fn record(&mut self, event: CapabilityEvent) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        self.total_events.fetch_add(1, Ordering::Relaxed);

        if matches!(event.event_type, CapEventType::Denied) {
            self.denial_count.fetch_add(1, Ordering::Relaxed);
        }

        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
    }

    /// Register process
    pub fn register_process(&mut self, caps: ProcessCaps) {
        self.process_caps.insert(caps.pid, caps);
    }

    /// Unregister process
    pub fn unregister_process(&mut self, pid: Pid) -> Option<ProcessCaps> {
        self.process_caps.remove(&pid)
    }

    /// Get process caps
    pub fn get_process(&self, pid: Pid) -> Option<&ProcessCaps> {
        self.process_caps.get(&pid)
    }

    /// Get recent events
    pub fn recent_events(&self, count: usize) -> &[CapabilityEvent] {
        let start = self.events.len().saturating_sub(count);
        &self.events[start..]
    }

    /// Get denial count
    pub fn denial_count(&self) -> u64 {
        self.denial_count.load(Ordering::Relaxed)
    }

    /// Get total events
    pub fn total_events(&self) -> u64 {
        self.total_events.load(Ordering::Relaxed)
    }

    /// Enable/disable
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

impl Default for CapabilityTracker {
    fn default() -> Self {
        Self::new(10000)
    }
}
