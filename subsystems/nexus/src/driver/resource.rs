//! Driver resource tracking.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use super::types::DriverId;
use crate::core::NexusTimestamp;

// ============================================================================
// RESOURCE TRACKER
// ============================================================================

/// Tracks driver resource usage
pub struct DriverResourceTracker {
    /// Resource snapshots
    snapshots: BTreeMap<DriverId, Vec<ResourceSnapshot>>,
    /// Max snapshots
    max_snapshots: usize,
    /// Resource limits
    limits: BTreeMap<DriverId, ResourceLimits>,
    /// Violation events
    violations: Vec<ResourceViolation>,
}

/// Resource snapshot
#[derive(Debug, Clone, Copy)]
struct ResourceSnapshot {
    /// Timestamp
    #[allow(dead_code)]
    timestamp: u64,
    /// Memory bytes
    memory_bytes: u64,
    /// CPU percentage
    cpu_percent: f64,
    /// DMA buffers
    #[allow(dead_code)]
    dma_buffers: u32,
    /// Interrupt claims
    #[allow(dead_code)]
    interrupt_claims: u32,
}

/// Resource limits
#[derive(Debug, Clone, Copy)]
pub struct ResourceLimits {
    /// Maximum memory
    pub max_memory: u64,
    /// Maximum CPU
    pub max_cpu_percent: f64,
    /// Maximum DMA buffers
    pub max_dma_buffers: u32,
    /// Maximum interrupt claims
    pub max_interrupts: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 256 * 1024 * 1024, // 256MB
            max_cpu_percent: 20.0,
            max_dma_buffers: 64,
            max_interrupts: 8,
        }
    }
}

/// Resource violation
#[derive(Debug, Clone)]
pub struct ResourceViolation {
    /// Driver ID
    pub driver_id: DriverId,
    /// Violation type
    pub violation_type: ResourceViolationType,
    /// Current value
    pub current_value: f64,
    /// Limit value
    pub limit_value: f64,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Resource violation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceViolationType {
    /// Memory limit exceeded
    MemoryExceeded,
    /// CPU limit exceeded
    CpuExceeded,
    /// DMA limit exceeded
    DmaExceeded,
    /// Interrupt limit exceeded
    InterruptExceeded,
}

impl DriverResourceTracker {
    /// Create new tracker
    pub fn new() -> Self {
        Self {
            snapshots: BTreeMap::new(),
            max_snapshots: 1000,
            limits: BTreeMap::new(),
            violations: Vec::new(),
        }
    }

    /// Set limits for driver
    pub fn set_limits(&mut self, driver_id: DriverId, limits: ResourceLimits) {
        self.limits.insert(driver_id, limits);
    }

    /// Record resource usage
    pub fn record(
        &mut self,
        driver_id: DriverId,
        memory_bytes: u64,
        cpu_percent: f64,
        dma_buffers: u32,
        interrupt_claims: u32,
    ) -> Vec<ResourceViolation> {
        let snapshot = ResourceSnapshot {
            timestamp: NexusTimestamp::now().raw(),
            memory_bytes,
            cpu_percent,
            dma_buffers,
            interrupt_claims,
        };

        let snapshots = self.snapshots.entry(driver_id).or_default();
        snapshots.push(snapshot);
        if snapshots.len() > self.max_snapshots {
            snapshots.remove(0);
        }

        // Check violations
        let limits = self.limits.get(&driver_id).copied().unwrap_or_default();
        let mut new_violations = Vec::new();

        if memory_bytes > limits.max_memory {
            new_violations.push(self.record_violation(
                driver_id,
                ResourceViolationType::MemoryExceeded,
                memory_bytes as f64,
                limits.max_memory as f64,
            ));
        }

        if cpu_percent > limits.max_cpu_percent {
            new_violations.push(self.record_violation(
                driver_id,
                ResourceViolationType::CpuExceeded,
                cpu_percent,
                limits.max_cpu_percent,
            ));
        }

        if dma_buffers > limits.max_dma_buffers {
            new_violations.push(self.record_violation(
                driver_id,
                ResourceViolationType::DmaExceeded,
                dma_buffers as f64,
                limits.max_dma_buffers as f64,
            ));
        }

        if interrupt_claims > limits.max_interrupts {
            new_violations.push(self.record_violation(
                driver_id,
                ResourceViolationType::InterruptExceeded,
                interrupt_claims as f64,
                limits.max_interrupts as f64,
            ));
        }

        new_violations
    }

    /// Record violation
    fn record_violation(
        &mut self,
        driver_id: DriverId,
        violation_type: ResourceViolationType,
        current: f64,
        limit: f64,
    ) -> ResourceViolation {
        let violation = ResourceViolation {
            driver_id,
            violation_type,
            current_value: current,
            limit_value: limit,
            timestamp: NexusTimestamp::now(),
        };

        self.violations.push(violation.clone());
        violation
    }

    /// Get average resource usage
    pub fn average_usage(&self, driver_id: DriverId) -> Option<(f64, f64)> {
        let snapshots = self.snapshots.get(&driver_id)?;
        if snapshots.is_empty() {
            return None;
        }

        let avg_memory =
            snapshots.iter().map(|s| s.memory_bytes as f64).sum::<f64>() / snapshots.len() as f64;
        let avg_cpu = snapshots.iter().map(|s| s.cpu_percent).sum::<f64>() / snapshots.len() as f64;

        Some((avg_memory, avg_cpu))
    }

    /// Get violations for driver
    pub fn get_violations(&self, driver_id: DriverId) -> Vec<&ResourceViolation> {
        self.violations
            .iter()
            .filter(|v| v.driver_id == driver_id)
            .collect()
    }

    /// Get all violations
    pub fn all_violations(&self) -> &[ResourceViolation] {
        &self.violations
    }
}

impl Default for DriverResourceTracker {
    fn default() -> Self {
        Self::new()
    }
}
