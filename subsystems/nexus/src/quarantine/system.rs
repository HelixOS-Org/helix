//! Quarantine system implementation

#![allow(dead_code)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::entry::QuarantineEntry;
use super::level::{QuarantineLevel, QuarantineReason};
use crate::core::{ComponentId, NexusTimestamp};

// ============================================================================
// QUARANTINE HISTORY ENTRY
// ============================================================================

/// Entry in quarantine history
#[derive(Debug, Clone)]
pub struct QuarantineHistoryEntry {
    /// Component
    pub component: ComponentId,
    /// Start time
    pub started: NexusTimestamp,
    /// End time
    pub ended: NexusTimestamp,
    /// Final level
    pub level: QuarantineLevel,
    /// Reason
    pub reason: String,
    /// Was released successfully
    pub success: bool,
}

// ============================================================================
// QUARANTINE STATS
// ============================================================================

/// Quarantine statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct QuarantineStats {
    /// Total currently quarantined
    pub total_quarantined: usize,
    /// Monitored count
    pub monitored: u32,
    /// Degraded count
    pub degraded: u32,
    /// Restricted count
    pub restricted: u32,
    /// Isolated count
    pub isolated: u32,
    /// Suspended count
    pub suspended: u32,
    /// Total quarantines ever
    pub total_quarantines: u64,
    /// Total releases ever
    pub total_releases: u64,
}

// ============================================================================
// QUARANTINE SYSTEM
// ============================================================================

/// The quarantine system
pub struct QuarantineSystem {
    /// Quarantined components
    entries: BTreeMap<u64, QuarantineEntry>,
    /// Quarantine history
    history: VecDeque<QuarantineHistoryEntry>,
    /// Maximum history entries
    max_history: usize,
    /// Default release timeout (cycles)
    default_timeout: u64,
    /// Component dependencies
    dependencies: BTreeMap<u64, Vec<ComponentId>>,
    /// Total quarantines
    total_quarantines: AtomicU64,
    /// Total releases
    total_releases: AtomicU64,
}

impl QuarantineSystem {
    /// Create a new quarantine system
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            history: VecDeque::new(),
            max_history: 1000,
            default_timeout: 60 * 1_000_000_000, // 60 seconds
            dependencies: BTreeMap::new(),
            total_quarantines: AtomicU64::new(0),
            total_releases: AtomicU64::new(0),
        }
    }

    /// Set component dependencies
    #[inline(always)]
    pub fn set_dependencies(&mut self, component: ComponentId, deps: Vec<ComponentId>) {
        self.dependencies.insert(component.raw(), deps);
    }

    /// Quarantine a component
    #[inline]
    pub fn quarantine(&mut self, entry: QuarantineEntry) {
        let component_id = entry.component.raw();
        self.entries.insert(component_id, entry);
        self.total_quarantines.fetch_add(1, Ordering::Relaxed);
    }

    /// Quarantine a component with cascade
    pub fn quarantine_with_cascade(&mut self, mut entry: QuarantineEntry) {
        let component = entry.component;

        // Find components that depend on this one
        let dependents: Vec<ComponentId> = self
            .dependencies
            .iter()
            .filter(|(_, deps)| deps.contains(&component))
            .map(|(id, _)| ComponentId::new(*id))
            .collect();

        // Cascade quarantine
        for dep in &dependents {
            if !self.is_quarantined(*dep) {
                let cascade_entry =
                    QuarantineEntry::new(*dep, QuarantineReason::DependencyCascade {
                        source: component,
                    });
                self.quarantine(cascade_entry);
            }
        }

        entry.cascade_targets = dependents;
        self.quarantine(entry);
    }

    /// Check if a component is quarantined
    #[inline(always)]
    pub fn is_quarantined(&self, component: ComponentId) -> bool {
        self.entries.contains_key(&component.raw())
    }

    /// Get quarantine entry for a component
    #[inline(always)]
    pub fn get_entry(&self, component: ComponentId) -> Option<&QuarantineEntry> {
        self.entries.get(&component.raw())
    }

    /// Get mutable quarantine entry
    #[inline(always)]
    pub fn get_entry_mut(&mut self, component: ComponentId) -> Option<&mut QuarantineEntry> {
        self.entries.get_mut(&component.raw())
    }

    /// Get quarantine level for a component
    #[inline(always)]
    pub fn get_level(&self, component: ComponentId) -> Option<QuarantineLevel> {
        self.entries.get(&component.raw()).map(|e| e.level)
    }

    /// Release a component from quarantine
    pub fn release(&mut self, component: ComponentId) -> Option<QuarantineEntry> {
        if let Some(entry) = self.entries.remove(&component.raw()) {
            // Add to history
            let history_entry = QuarantineHistoryEntry {
                component,
                started: entry.started,
                ended: NexusTimestamp::now(),
                level: entry.level,
                reason: entry.reason.description(),
                success: true,
            };

            if self.history.len() >= self.max_history {
                self.history.pop_front();
            }
            self.history.push_back(history_entry);

            self.total_releases.fetch_add(1, Ordering::Relaxed);

            // Also release cascade targets
            for target in &entry.cascade_targets {
                self.release(*target);
            }

            Some(entry)
        } else {
            None
        }
    }

    /// Escalate quarantine for a component
    #[inline]
    pub fn escalate(&mut self, component: ComponentId) {
        if let Some(entry) = self.entries.get_mut(&component.raw()) {
            entry.escalate();
        }
    }

    /// De-escalate quarantine for a component
    pub fn deescalate(&mut self, component: ComponentId) {
        if let Some(entry) = self.entries.get_mut(&component.raw()) {
            entry.deescalate();

            // Release if back to monitored
            if entry.level == QuarantineLevel::Monitored {
                // Schedule release
                entry.release_at = Some(NexusTimestamp::from_ticks(
                    NexusTimestamp::now().ticks() + 5_000_000_000, // 5 seconds
                ));
            }
        }
    }

    /// Check for components to release
    pub fn check_releases(&mut self) -> Vec<ComponentId> {
        let mut to_release = Vec::new();

        for (id, entry) in &self.entries {
            if entry.should_release() {
                to_release.push(ComponentId::new(*id));
            }
        }

        for component in &to_release {
            self.release(*component);
        }

        to_release
    }

    /// Try to release with health check
    pub fn try_release(
        &mut self,
        component: ComponentId,
        current_health: f32,
    ) -> Result<bool, &str> {
        let entry = self
            .entries
            .get_mut(&component.raw())
            .ok_or("Component not quarantined")?;

        entry.record_release_attempt();

        if current_health >= entry.release_threshold {
            self.release(component);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get all quarantined components
    #[inline(always)]
    pub fn quarantined(&self) -> Vec<&QuarantineEntry> {
        self.entries.values().collect()
    }

    /// Get components at specific level
    #[inline(always)]
    pub fn at_level(&self, level: QuarantineLevel) -> Vec<&QuarantineEntry> {
        self.entries.values().filter(|e| e.level == level).collect()
    }

    /// Get quarantine history
    #[inline(always)]
    pub fn history(&self) -> &[QuarantineHistoryEntry] {
        &self.history
    }

    /// Get statistics
    pub fn stats(&self) -> QuarantineStats {
        let mut level_counts = [0u32; 5];

        for entry in self.entries.values() {
            level_counts[entry.level as usize] += 1;
        }

        QuarantineStats {
            total_quarantined: self.entries.len(),
            monitored: level_counts[0],
            degraded: level_counts[1],
            restricted: level_counts[2],
            isolated: level_counts[3],
            suspended: level_counts[4],
            total_quarantines: self.total_quarantines.load(Ordering::Relaxed),
            total_releases: self.total_releases.load(Ordering::Relaxed),
        }
    }

    /// Clear all quarantines (emergency)
    pub fn clear_all(&mut self) {
        let entries = core::mem::take(&mut self.entries);
        for (id, entry) in entries {
            let history_entry = QuarantineHistoryEntry {
                component: ComponentId::new(id),
                started: entry.started,
                ended: NexusTimestamp::now(),
                level: entry.level,
                reason: entry.reason.description(),
                success: false, // Forced release
            };
            self.history.push_back(history_entry);
        }
    }
}

impl Default for QuarantineSystem {
    fn default() -> Self {
        Self::new()
    }
}
