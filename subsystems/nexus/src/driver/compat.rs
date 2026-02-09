//! Driver compatibility analysis.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::types::DriverId;
use crate::core::NexusTimestamp;

// ============================================================================
// COMPATIBILITY ANALYZER
// ============================================================================

/// Analyzes driver compatibility
pub struct CompatibilityAnalyzer {
    /// Known compatibility issues
    issues: BTreeMap<(DriverId, DriverId), CompatibilityIssue>,
    /// Conflict history
    conflicts: Vec<DriverConflict>,
    /// Hardware compatibility
    hardware_compat: BTreeMap<DriverId, Vec<String>>,
}

/// Compatibility issue
#[derive(Debug, Clone)]
pub struct CompatibilityIssue {
    /// First driver
    pub driver_a: DriverId,
    /// Second driver
    pub driver_b: DriverId,
    /// Issue type
    pub issue_type: CompatibilityIssueType,
    /// Severity
    pub severity: CompatibilitySeverity,
    /// Description
    pub description: String,
}

/// Compatibility issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityIssueType {
    /// Resource conflict
    ResourceConflict,
    /// Interrupt conflict
    InterruptConflict,
    /// Memory conflict
    MemoryConflict,
    /// DMA conflict
    DmaConflict,
    /// Order dependency
    OrderDependency,
    /// Known incompatibility
    KnownIncompatible,
}

/// Compatibility severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CompatibilitySeverity {
    /// Minor issue
    Minor       = 0,
    /// Performance impact
    Performance = 1,
    /// Functionality limited
    Limited     = 2,
    /// Critical issue
    Critical    = 3,
    /// Fatal conflict
    Fatal       = 4,
}

/// Driver conflict
#[derive(Debug, Clone)]
pub struct DriverConflict {
    /// Drivers involved
    pub drivers: Vec<DriverId>,
    /// Conflict description
    pub description: String,
    /// Timestamp
    pub timestamp: NexusTimestamp,
    /// Was resolved
    pub resolved: bool,
}

impl CompatibilityAnalyzer {
    /// Create new analyzer
    pub fn new() -> Self {
        Self {
            issues: BTreeMap::new(),
            conflicts: Vec::new(),
            hardware_compat: BTreeMap::new(),
        }
    }

    /// Record compatibility issue
    #[inline]
    pub fn record_issue(&mut self, issue: CompatibilityIssue) {
        let key = if issue.driver_a < issue.driver_b {
            (issue.driver_a, issue.driver_b)
        } else {
            (issue.driver_b, issue.driver_a)
        };

        self.issues.insert(key, issue);
    }

    /// Check compatibility
    #[inline]
    pub fn check_compatibility(
        &self,
        driver_a: DriverId,
        driver_b: DriverId,
    ) -> Option<&CompatibilityIssue> {
        let key = if driver_a < driver_b {
            (driver_a, driver_b)
        } else {
            (driver_b, driver_a)
        };

        self.issues.get(&key)
    }

    /// Record conflict
    #[inline]
    pub fn record_conflict(&mut self, drivers: Vec<DriverId>, description: &str) {
        self.conflicts.push(DriverConflict {
            drivers,
            description: String::from(description),
            timestamp: NexusTimestamp::now(),
            resolved: false,
        });
    }

    /// Resolve conflict
    #[inline]
    pub fn resolve_conflict(&mut self, index: usize) {
        if let Some(conflict) = self.conflicts.get_mut(index) {
            conflict.resolved = true;
        }
    }

    /// Set hardware compatibility
    #[inline(always)]
    pub fn set_hardware_compat(&mut self, driver_id: DriverId, device_ids: Vec<String>) {
        self.hardware_compat.insert(driver_id, device_ids);
    }

    /// Check hardware compatibility
    #[inline]
    pub fn check_hardware(&self, driver_id: DriverId, device_id: &str) -> bool {
        self.hardware_compat
            .get(&driver_id)
            .map(|ids| ids.iter().any(|id| id == device_id))
            .unwrap_or(true) // Unknown = assume compatible
    }

    /// Get all issues for driver
    #[inline]
    pub fn get_issues(&self, driver_id: DriverId) -> Vec<&CompatibilityIssue> {
        self.issues
            .values()
            .filter(|i| i.driver_a == driver_id || i.driver_b == driver_id)
            .collect()
    }

    /// Get unresolved conflicts
    #[inline(always)]
    pub fn unresolved_conflicts(&self) -> Vec<&DriverConflict> {
        self.conflicts.iter().filter(|c| !c.resolved).collect()
    }
}

impl Default for CompatibilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
