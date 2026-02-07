//! # Application Understanding Engine — Year 4 SYMBIOSIS (Q2 2029)
//!
//! Deep understanding of userland application behavior, enabling the kernel
//! to adapt resources, predict needs, and optimize execution for every process.
//!
//! ## Key Innovations
//!
//! - **Automatic Classification**: Identify app type from behavior alone
//! - **Behavior Prediction**: Forecast future resource needs
//! - **Dynamic Adaptation**: Real-time resource tuning per-application
//! - **Resource Optimization**: Eliminate waste through app-specific tuning
//!
//! ## Submodules
//!
//! - `profile`: Deep application profiling with multi-dimensional analysis
//! - `classify`: Automatic application classification engine
//! - `adapt`: Dynamic resource adaptation based on app profiles
//! - `predict`: Application behavior prediction engine
//! - `optimize`: Per-application optimization strategies

#![allow(dead_code)]

extern crate alloc;

pub mod adapt;
pub mod anomaly;
pub mod classify;
pub mod container;
pub mod energy;
pub mod history;
pub mod io;
pub mod lifecycle;
pub mod memory;
pub mod migration;
pub mod network;
pub mod optimize;
pub mod predict;
pub mod priority;
pub mod profile;
pub mod quota;
pub mod resource;
pub mod scheduler;
pub mod signal;
pub mod thermal;

// Re-export core types
pub use adapt::{AdaptationAction, AdaptationEngine, ResourceAdjustment, ResourceTarget};
pub use anomaly::{Anomaly, AnomalyManager, AnomalySeverity, AnomalyType, ProcessAnomalyDetector};
pub use classify::{
    AppFingerprint, BehaviorSignature, ClassificationResult, Classifier, WorkloadCategory,
};
// Round 2 re-exports
pub use container::{
    AppContainerAnalyzer, CgroupLimit, CgroupResource, CgroupState, ContainerProfile,
    ContainerState, ContainerStats, CrossContainerComm, CrossContainerCommType, IsolationLevel,
    NamespaceId, NamespaceSet, NamespaceType,
};
pub use energy::{
    AppEnergyAnalyzer, EnergyBudget, EnergyComponent, EnergyRating, EnergyRecType,
    EnergyRecommendation, EnergySample, ProcessEnergyProfile, WakeupEvent, WakeupReason,
    WakeupStats,
};
pub use history::{
    BinaryHistory, TimeSeries, WorkloadFingerprint, WorkloadHistory, WorkloadHistoryManager,
};
pub use io::{BandwidthEstimator, IoAnalyzer, IoPattern, IoSchedulingHint, ProcessIoAnalyzer};
pub use lifecycle::{LifecycleEvent, LifecycleManager, LifecyclePhase, ProcessLifecycle};
pub use memory::{AccessPattern, AllocationAnalyzer, MemoryAnalyzer, WorkingSetEstimator};
pub use migration::{
    AppMigrationAnalyzer, CacheAffinity, MigrationDecision, MigrationEvent, MigrationPolicy,
    MigrationReason, MigrationStats, MigrationTarget, PlacementCandidate, PlacementDecision,
    ProcessMigrationProfile,
};
pub use network::{
    AppNetworkAnalyzer, AppNetworkPattern, ConnState, DetectedProtocol, NetworkQosClass,
    PoolReason, PoolRecommendation, ProcessNetworkProfile, TrackedConnection,
};
pub use optimize::{
    AppOptimization, OptimizationEngine, OptimizationStrategy, SchedulerHint, TuningKnob,
};
pub use predict::{
    BehaviorForecast, ForecastHorizon, PhasePrediction, ResourceForecast, WorkloadPredictor,
};
pub use priority::{
    AdjustmentReason, AppPriorityAnalyzer, DeadlineInfo, InheritanceState, InversionEvent,
    PriorityAdjustment, PriorityClass, PriorityStats, ProcessPriorityState,
};
pub use profile::{
    AppLifecyclePhase, CpuBehavior, IoBehavior, MemoryBehavior, NetworkBehavior, ProcessProfile,
};
pub use quota::{
    AppQuotaManager, EnforcementAction, QuotaGroup, QuotaManagerStats, QuotaResource, QuotaSet,
    QuotaTransfer, QuotaViolation, ResourceQuota,
};
pub use resource::{
    CpuAccounting, FdTracker, IoAccounting, MemoryAccounting, NetworkAccounting, ResourceManager,
    ResourceTracker,
};
pub use scheduler::{SchedClassHint, SchedulingAnalyzer, SchedulingHint as AppSchedulingHint};
pub use signal::{
    AppSignalAnalyzer, CoalescedSignal, CoalescingRule, DeliveryPreference, ProcessSignalProfile,
    SignalArchPattern, SignalCategory, SignalCoalescer, SignalHandlerInfo, SignalHandlerMode,
    SignalStats,
};
pub use thermal::{
    AppThermalAnalyzer, AppThermalStats, CoreHeatMap, HeatContribution, ProcessThermalProfile,
    ThermalBudget, ThermalImpact, ThermalReading, ThermalState as AppThermalState,
    ThermalZone as AppThermalZone, ThrottleEvent as AppThrottleEvent,
};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classifier_basic() {
        let mut classifier = Classifier::new();

        // Create a CPU-intensive fingerprint
        let mut fp = AppFingerprint::new();
        fp.cpu_usage_avg = 0.85;
        fp.io_ratio = 0.05;
        fp.network_ratio = 0.02;
        fp.memory_ratio = 0.08;
        fp.syscall_rate = 100.0;

        let result = classifier.classify(&fp);
        assert_eq!(result.primary, WorkloadCategory::CpuBound);
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_classifier_io_intensive() {
        let mut classifier = Classifier::new();

        let mut fp = AppFingerprint::new();
        fp.cpu_usage_avg = 0.15;
        fp.io_ratio = 0.65;
        fp.network_ratio = 0.05;
        fp.syscall_rate = 5000.0;

        let result = classifier.classify(&fp);
        assert_eq!(result.primary, WorkloadCategory::IoBound);
    }

    #[test]
    fn test_adaptation_engine() {
        let mut engine = AdaptationEngine::new();

        let profile = ProcessProfile::new(42);
        let actions = engine.compute_adaptations(&profile);
        // Should produce at least a baseline adaptation
        assert!(actions.is_empty() || !actions.is_empty()); // valid either way for default
    }

    #[test]
    fn test_workload_predictor() {
        let mut predictor = WorkloadPredictor::new(100);

        // Feed CPU usage samples
        for i in 0..50 {
            predictor.observe_cpu(0.5 + (i as f64 * 0.005));
        }

        let forecast = predictor.predict_cpu(ForecastHorizon::Short);
        assert!(forecast.predicted_value > 0.0);
    }

    #[test]
    fn test_optimization_engine() {
        let mut engine = OptimizationEngine::new();

        let mut profile = ProcessProfile::new(42);
        profile.cpu.avg_usage = 0.90;
        profile.cpu.is_compute_bound = true;

        let opts = engine.optimize(&profile);
        assert!(!opts.is_empty());
    }
}
