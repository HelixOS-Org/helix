//! Audit Log
//!
//! Audit log buffer and search functionality.

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{AuditEvent, AuditEventId};

/// Audit log buffer
pub struct AuditLog {
    /// Events
    events: Vec<AuditEvent>,
    /// Max events
    max_events: usize,
    /// Total events logged
    total_logged: AtomicU64,
    /// Events dropped
    events_dropped: AtomicU64,
    /// Next event ID
    next_id: AtomicU64,
    /// Enabled
    enabled: AtomicBool,
}

impl AuditLog {
    /// Create new audit log
    pub fn new(max_events: usize) -> Self {
        Self {
            events: Vec::with_capacity(max_events),
            max_events,
            total_logged: AtomicU64::new(0),
            events_dropped: AtomicU64::new(0),
            next_id: AtomicU64::new(1),
            enabled: AtomicBool::new(true),
        }
    }

    /// Allocate event ID
    pub fn allocate_id(&self) -> AuditEventId {
        AuditEventId::new(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Log event
    pub fn log(&mut self, event: AuditEvent) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        self.total_logged.fetch_add(1, Ordering::Relaxed);

        if self.events.len() >= self.max_events {
            self.events.remove(0);
            self.events_dropped.fetch_add(1, Ordering::Relaxed);
        }

        self.events.push(event);
    }

    /// Get recent events
    pub fn recent(&self, count: usize) -> &[AuditEvent] {
        let start = self.events.len().saturating_sub(count);
        &self.events[start..]
    }

    /// Get all events
    pub fn all(&self) -> &[AuditEvent] {
        &self.events
    }

    /// Search events
    pub fn search<F>(&self, predicate: F) -> Vec<&AuditEvent>
    where
        F: Fn(&AuditEvent) -> bool,
    {
        self.events.iter().filter(|e| predicate(e)).collect()
    }

    /// Get event by ID
    pub fn get(&self, id: AuditEventId) -> Option<&AuditEvent> {
        self.events.iter().find(|e| e.id == id)
    }

    /// Get total logged
    pub fn total_logged(&self) -> u64 {
        self.total_logged.load(Ordering::Relaxed)
    }

    /// Get events dropped
    pub fn events_dropped(&self) -> u64 {
        self.events_dropped.load(Ordering::Relaxed)
    }

    /// Enable/disable
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Clear log
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /// Event count
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.max_events
    }
}
