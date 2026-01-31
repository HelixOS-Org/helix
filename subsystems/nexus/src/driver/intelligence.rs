//! Central driver intelligence coordinator.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::compat::CompatibilityAnalyzer;
use super::fault::{DriverFaultPredictor, FaultPrediction, FaultType};
use super::health::DriverHealthMonitor;
use super::metrics::DriverMetrics;
use super::resource::{DriverResourceTracker, ResourceViolation};
use super::types::{DriverId, DriverInfo};

// ============================================================================
// DRIVER INTELLIGENCE
// ============================================================================

/// Central driver intelligence coordinator
pub struct DriverIntelligence {
    /// Driver registry
    drivers: BTreeMap<DriverId, DriverInfo>,
    /// Driver metrics
    metrics: BTreeMap<DriverId, DriverMetrics>,
    /// Health monitor
    health: DriverHealthMonitor,
    /// Fault predictor
    fault: DriverFaultPredictor,
    /// Resource tracker
    resource: DriverResourceTracker,
    /// Compatibility analyzer
    compat: CompatibilityAnalyzer,
    /// Total drivers loaded
    total_loaded: AtomicU64,
}

impl DriverIntelligence {
    /// Create new driver intelligence
    pub fn new() -> Self {
        Self {
            drivers: BTreeMap::new(),
            metrics: BTreeMap::new(),
            health: DriverHealthMonitor::default(),
            fault: DriverFaultPredictor::default(),
            resource: DriverResourceTracker::default(),
            compat: CompatibilityAnalyzer::default(),
            total_loaded: AtomicU64::new(0),
        }
    }

    /// Register driver
    pub fn register(&mut self, info: DriverInfo) {
        self.metrics.insert(info.id, DriverMetrics::default());
        self.drivers.insert(info.id, info);
    }

    /// Mark driver loaded
    pub fn mark_loaded(&mut self, driver_id: DriverId) {
        if let Some(info) = self.drivers.get_mut(&driver_id) {
            info.mark_loaded();
            self.total_loaded.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record operation
    pub fn record_operation(&mut self, driver_id: DriverId, success: bool, latency_ns: u64) {
        if let Some(metrics) = self.metrics.get_mut(&driver_id) {
            metrics.record_operation(success, latency_ns);
            self.health.record(driver_id, metrics);

            if !success {
                if let Some(info) = self.drivers.get_mut(&driver_id) {
                    info.mark_error();
                }
            }
        }
    }

    /// Record fault
    pub fn record_fault(&mut self, driver_id: DriverId, fault_type: FaultType, recovered: bool) {
        self.fault.record_fault(driver_id, fault_type, recovered);

        if !recovered {
            if let Some(info) = self.drivers.get_mut(&driver_id) {
                info.mark_crashed();
            }
        }
    }

    /// Record resource usage
    pub fn record_resources(
        &mut self,
        driver_id: DriverId,
        memory: u64,
        cpu: f64,
        dma: u32,
        interrupts: u32,
    ) -> Vec<ResourceViolation> {
        self.resource
            .record(driver_id, memory, cpu, dma, interrupts)
    }

    /// Get driver info
    pub fn get_driver(&self, driver_id: DriverId) -> Option<&DriverInfo> {
        self.drivers.get(&driver_id)
    }

    /// Get driver metrics
    pub fn get_metrics(&self, driver_id: DriverId) -> Option<&DriverMetrics> {
        self.metrics.get(&driver_id)
    }

    /// Get health score
    pub fn get_health(&self, driver_id: DriverId) -> f64 {
        self.health.get_score(driver_id)
    }

    /// Get fault prediction
    pub fn get_prediction(&self, driver_id: DriverId) -> Option<&FaultPrediction> {
        self.fault.get_prediction(driver_id)
    }

    /// Get unhealthy drivers
    pub fn unhealthy_drivers(&self) -> Vec<(DriverId, f64)> {
        self.health.unhealthy_drivers()
    }

    /// Get high-risk drivers
    pub fn high_risk_drivers(&self, threshold: f64) -> Vec<&FaultPrediction> {
        self.fault.high_risk_drivers(threshold)
    }

    /// Get health monitor
    pub fn health_monitor(&self) -> &DriverHealthMonitor {
        &self.health
    }

    /// Get resource tracker
    pub fn resource_tracker(&self) -> &DriverResourceTracker {
        &self.resource
    }

    /// Get mutable resource tracker
    pub fn resource_tracker_mut(&mut self) -> &mut DriverResourceTracker {
        &mut self.resource
    }

    /// Get compatibility analyzer
    pub fn compatibility(&self) -> &CompatibilityAnalyzer {
        &self.compat
    }

    /// Get mutable compatibility analyzer
    pub fn compatibility_mut(&mut self) -> &mut CompatibilityAnalyzer {
        &mut self.compat
    }

    /// Get total loaded drivers
    pub fn total_loaded(&self) -> u64 {
        self.total_loaded.load(Ordering::Relaxed)
    }
}

impl Default for DriverIntelligence {
    fn default() -> Self {
        Self::new()
    }
}
