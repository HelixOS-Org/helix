//! Quarantine entry

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;

use super::level::{QuarantineLevel, QuarantineReason};
use crate::core::{ComponentId, NexusTimestamp};

// ============================================================================
// QUARANTINE ENTRY
// ============================================================================

/// An entry for a quarantined component
#[derive(Debug, Clone)]
pub struct QuarantineEntry {
    /// Component ID
    pub component: ComponentId,
    /// Quarantine level
    pub level: QuarantineLevel,
    /// Reason for quarantine
    pub reason: QuarantineReason,
    /// Timestamp of quarantine start
    pub started: NexusTimestamp,
    /// Timestamp of scheduled release (if any)
    pub release_at: Option<NexusTimestamp>,
    /// Number of release attempts
    pub release_attempts: u32,
    /// Last release attempt timestamp
    pub last_release_attempt: Option<NexusTimestamp>,
    /// Is auto-release enabled?
    pub auto_release: bool,
    /// Health threshold for release
    pub release_threshold: f32,
    /// Dependencies that were also quarantined
    pub cascade_targets: Vec<ComponentId>,
}

impl QuarantineEntry {
    /// Create a new entry
    pub fn new(component: ComponentId, reason: QuarantineReason) -> Self {
        let level = reason.recommended_level();
        Self {
            component,
            level,
            reason,
            started: NexusTimestamp::now(),
            release_at: None,
            release_attempts: 0,
            last_release_attempt: None,
            auto_release: true,
            release_threshold: 0.7,
            cascade_targets: Vec::new(),
        }
    }

    /// Set quarantine level
    #[inline(always)]
    pub fn with_level(mut self, level: QuarantineLevel) -> Self {
        self.level = level;
        self
    }

    /// Set release time
    #[inline(always)]
    pub fn with_release_at(mut self, timestamp: NexusTimestamp) -> Self {
        self.release_at = Some(timestamp);
        self
    }

    /// Set release after duration
    #[inline]
    pub fn with_release_after(mut self, duration_cycles: u64) -> Self {
        let release_time =
            NexusTimestamp::from_ticks(NexusTimestamp::now().ticks() + duration_cycles);
        self.release_at = Some(release_time);
        self
    }

    /// Disable auto-release
    #[inline(always)]
    pub fn without_auto_release(mut self) -> Self {
        self.auto_release = false;
        self
    }

    /// Get duration in quarantine
    #[inline(always)]
    pub fn duration(&self) -> u64 {
        NexusTimestamp::now().duration_since(self.started)
    }

    /// Is release scheduled?
    #[inline]
    pub fn is_release_scheduled(&self) -> bool {
        if let Some(release_at) = self.release_at {
            release_at.ticks() > NexusTimestamp::now().ticks()
        } else {
            false
        }
    }

    /// Should release now?
    #[inline]
    pub fn should_release(&self) -> bool {
        if let Some(release_at) = self.release_at {
            NexusTimestamp::now().ticks() >= release_at.ticks()
        } else {
            false
        }
    }

    /// Record release attempt
    #[inline(always)]
    pub fn record_release_attempt(&mut self) {
        self.release_attempts += 1;
        self.last_release_attempt = Some(NexusTimestamp::now());
    }

    /// Escalate quarantine level
    #[inline]
    pub fn escalate(&mut self) {
        self.level = match self.level {
            QuarantineLevel::Monitored => QuarantineLevel::Degraded,
            QuarantineLevel::Degraded => QuarantineLevel::Restricted,
            QuarantineLevel::Restricted => QuarantineLevel::Isolated,
            QuarantineLevel::Isolated => QuarantineLevel::Suspended,
            QuarantineLevel::Suspended => QuarantineLevel::Suspended,
        };
    }

    /// De-escalate quarantine level
    #[inline]
    pub fn deescalate(&mut self) {
        self.level = match self.level {
            QuarantineLevel::Suspended => QuarantineLevel::Isolated,
            QuarantineLevel::Isolated => QuarantineLevel::Restricted,
            QuarantineLevel::Restricted => QuarantineLevel::Degraded,
            QuarantineLevel::Degraded => QuarantineLevel::Monitored,
            QuarantineLevel::Monitored => QuarantineLevel::Monitored,
        };
    }
}
