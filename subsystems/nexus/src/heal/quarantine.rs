//! Component quarantine management
//!
//! This module provides quarantine functionality for isolating
//! failing components to prevent cascade failures.

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::{ComponentId, NexusTimestamp};

/// Quarantined component
#[derive(Debug, Clone)]
pub struct QuarantinedComponent {
    /// Component ID
    pub component: ComponentId,
    /// Reason for quarantine
    pub reason: String,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Number of failed healing attempts
    pub failed_attempts: u32,
    /// Last error
    pub last_error: Option<String>,
    /// Scheduled release (if any)
    pub release_at: Option<NexusTimestamp>,
}

/// Quarantine manager
pub struct QuarantineManager {
    /// Quarantined components
    quarantined: BTreeMap<u64, QuarantinedComponent>,
    /// Maximum quarantine duration (cycles)
    max_duration: u64,
}

impl QuarantineManager {
    /// Create a new quarantine manager
    pub fn new(max_duration: u64) -> Self {
        Self {
            quarantined: BTreeMap::new(),
            max_duration,
        }
    }

    /// Quarantine a component
    pub fn quarantine(&mut self, component: ComponentId, reason: impl Into<String>) {
        let entry = QuarantinedComponent {
            component,
            reason: reason.into(),
            timestamp: NexusTimestamp::now(),
            failed_attempts: 0,
            last_error: None,
            release_at: None,
        };
        self.quarantined.insert(component.raw(), entry);
    }

    /// Quarantine with scheduled release
    pub fn quarantine_with_release(
        &mut self,
        component: ComponentId,
        reason: impl Into<String>,
        release_at: NexusTimestamp,
    ) {
        let mut entry = QuarantinedComponent {
            component,
            reason: reason.into(),
            timestamp: NexusTimestamp::now(),
            failed_attempts: 0,
            last_error: None,
            release_at: Some(release_at),
        };
        self.quarantined.insert(component.raw(), entry);
    }

    /// Check if a component is quarantined
    pub fn is_quarantined(&self, component: ComponentId) -> bool {
        self.quarantined.contains_key(&component.raw())
    }

    /// Get quarantine entry
    pub fn get(&self, component: ComponentId) -> Option<&QuarantinedComponent> {
        self.quarantined.get(&component.raw())
    }

    /// Get quarantine entry mutably
    pub fn get_mut(&mut self, component: ComponentId) -> Option<&mut QuarantinedComponent> {
        self.quarantined.get_mut(&component.raw())
    }

    /// Release a component from quarantine
    pub fn release(&mut self, component: ComponentId) -> Option<QuarantinedComponent> {
        self.quarantined.remove(&component.raw())
    }

    /// Record a failed healing attempt
    pub fn record_failure(&mut self, component: ComponentId, error: impl Into<String>) {
        if let Some(entry) = self.quarantined.get_mut(&component.raw()) {
            entry.failed_attempts += 1;
            entry.last_error = Some(error.into());
        }
    }

    /// Get all quarantined components
    pub fn all_quarantined(&self) -> Vec<&QuarantinedComponent> {
        self.quarantined.values().collect()
    }

    /// Count quarantined components
    pub fn count(&self) -> usize {
        self.quarantined.len()
    }

    /// Check for expired quarantines
    pub fn check_expired(&mut self) -> Vec<ComponentId> {
        let now = NexusTimestamp::now();
        let mut expired = Vec::new();

        for (_, entry) in &self.quarantined {
            if let Some(release_at) = entry.release_at {
                if now.ticks() >= release_at.ticks() {
                    expired.push(entry.component);
                }
            } else {
                let duration = now.duration_since(entry.timestamp);
                if duration >= self.max_duration {
                    expired.push(entry.component);
                }
            }
        }

        for comp in &expired {
            self.quarantined.remove(&comp.raw());
        }

        expired
    }

    /// Set maximum quarantine duration
    pub fn set_max_duration(&mut self, duration: u64) {
        self.max_duration = duration;
    }
}
