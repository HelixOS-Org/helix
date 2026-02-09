//! Active fault representation

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};

use super::config::FaultConfig;
use super::types::FaultType;
use crate::core::NexusTimestamp;

// ============================================================================
// FAULT
// ============================================================================

/// An active fault
#[derive(Debug, Clone)]
pub struct Fault {
    /// Unique fault ID
    pub id: u64,
    /// Configuration
    pub config: FaultConfig,
    /// Start timestamp
    pub started: NexusTimestamp,
    /// End timestamp (if ended)
    pub ended: Option<NexusTimestamp>,
    /// Number of occurrences
    pub occurrences: u32,
    /// Is fault currently active
    pub active: bool,
}

impl Fault {
    /// Create a new fault
    pub fn new(config: FaultConfig) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            config,
            started: NexusTimestamp::now(),
            ended: None,
            occurrences: 0,
            active: true,
        }
    }

    /// Record an occurrence
    #[inline]
    pub fn record_occurrence(&mut self) {
        self.occurrences += 1;

        // Check max occurrences
        if let Some(max) = self.config.max_occurrences {
            if self.occurrences >= max {
                self.stop();
            }
        }
    }

    /// Stop the fault
    #[inline(always)]
    pub fn stop(&mut self) {
        self.active = false;
        self.ended = Some(NexusTimestamp::now());
    }

    /// Check if fault should trigger
    pub fn should_trigger(&self) -> bool {
        if !self.active || !self.config.enabled {
            return false;
        }

        // Check duration
        if let Some(duration) = self.config.duration_cycles {
            if NexusTimestamp::now().duration_since(self.started) > duration {
                return false;
            }
        }

        // Check max occurrences
        if let Some(max) = self.config.max_occurrences {
            if self.occurrences >= max {
                return false;
            }
        }

        // Check probability
        true // Simplified - real implementation would use RNG
    }

    /// Get fault type
    #[inline(always)]
    pub fn fault_type(&self) -> FaultType {
        self.config.fault_type
    }
}
