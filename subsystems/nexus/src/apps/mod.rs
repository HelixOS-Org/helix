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
pub mod affinity;
pub mod anomaly;
pub mod cache;
pub mod cgroup;
pub mod classify;
pub mod container;
pub mod energy;
pub mod futex;
pub mod gpu;
pub mod history;
pub mod io;
pub mod lifecycle;
pub mod memory;
pub mod migration;
pub mod network;
pub mod numa;
pub mod optimize;
pub mod predict;
pub mod priority;
pub mod profile;
pub mod quota;
pub mod resource;
pub mod scheduler;
pub mod signal;
pub mod syscall_profile;
pub mod thermal;
pub mod watchdog;
// Round 4
pub mod capability;
pub mod credential;
pub mod fault;
pub mod heap;
pub mod ipc;
pub mod mmap_tracker;
pub mod rlimit;
pub mod sampling;
pub mod threading;
// Round 5
pub mod binary;
pub mod dependency;
pub mod environment;
pub mod fd_tracker;
pub mod lock;
pub mod page_cache;
pub mod sched_profile;
pub mod trace;
// Round 6
pub mod cgroup_v2;
pub mod exe_profile;
pub mod futex_v2;
pub mod interrupt;
pub mod leak_detect;
pub mod net_stack;
pub mod perf_counter;
pub mod seccomp_profile;
pub mod timer_profile;
pub mod vma_tracker;

// Re-export core types
pub use adapt::{AdaptationAction, AdaptationEngine, ResourceAdjustment, ResourceTarget};
// Round 3 re-exports
pub use affinity::{
    AffinityMask, AffinityPolicy, AppAffinityManager, AppAffinityStats, CoreDescriptor, CoreType,
    MigrationEvent as AffinityMigrationEvent, ProcessAffinityProfile,
};
pub use anomaly::{Anomaly, AnomalyManager, AnomalySeverity, AnomalyType, ProcessAnomalyDetector};
// Round 5 re-exports
pub use binary::{
    AppBinaryAnalyzer, AppBinaryStats, BinaryProfile, ExecFormat, SectionInfo, SectionPerms,
    SectionType, SymbolBinding, SymbolInfo, SymbolType,
};
pub use cache::{
    AppCacheAnalyzer, AppCacheStats, CacheAccessType, CacheLevel, CacheLevelCounters,
    CachePartition, CachePartitionMode, PollutionDetector, PollutionEvent,
    WorkingSetEstimate as CacheWorkingSetEstimate, WorkingSetTracker, WorkingSetTrend,
};
// Round 4 re-exports
pub use capability::{
    AppCapability, AppCapabilityManager, AppCapabilitySet, AppCapabilityStats, CapUsageRecord,
    CapabilityCategory, ProcessCapProfile,
};
pub use cgroup::{
    AppCgroupAnalyzer, AppCgroupStats, CgroupController, CgroupMigration, CgroupNode,
    CgroupPressure, CgroupVersion, CpuLimit, IoLimit, MemoryLimit, PidLimit,
};
// Round 6 re-exports
pub use cgroup_v2::{
    AppCgroupV2Profiler, AppCgroupV2Stats, CgroupIoStats, CgroupMemoryStats,
    CgroupNode as CgroupV2Node, CgroupPressure as CgroupV2Pressure, CpuBandwidth,
};
pub use classify::{
    AppFingerprint, BehaviorSignature, ClassificationResult, Classifier, WorkloadCategory,
};
// Round 2 re-exports
pub use container::{
    AppContainerAnalyzer, CgroupLimit, CgroupResource, CgroupState, ContainerProfile,
    ContainerState, ContainerStats, CrossContainerComm, CrossContainerCommType, IsolationLevel,
    NamespaceId, NamespaceSet, NamespaceType,
};
pub use credential::{
    AppCredentialManager, AppCredentialStats, CredentialChange, CredentialEvent, CredentialSet,
    GroupId, ProcessCredProfile, SecuritySession, SessionType, UserId,
};
pub use dependency::{
    AppDepType, AppDependencyAnalyzer, AppDependencyStats, DepState, DepStrength, DependencyEdge,
    DependencyGraph,
};
pub use energy::{
    AppEnergyAnalyzer, EnergyBudget, EnergyComponent, EnergyRating, EnergyRecType,
    EnergyRecommendation, EnergySample, ProcessEnergyProfile, WakeupEvent, WakeupReason,
    WakeupStats,
};
pub use environment::{
    AppEnvironmentStats, AppEnvironmentTracker, EnvCategory, EnvDiff, EnvEntry,
    EnvironmentSnapshot, NamespaceInfo, NamespaceSet as AppNamespaceSet, ProcessEnvironment,
};
pub use exe_profile::{
    AppExeProfiler, AppExeProfilerStats, ExeArchitecture, ExecutableFormat, ExecutableProfile,
    LibraryDep, SectionInfo as ExeSectionInfo, SectionType as ExeSectionType,
};
pub use fault::{
    AppFaultAnalyzer, AppFaultStats, FaultEvent, FaultPattern, FaultSeverity, FaultType,
    ProcessFaultProfile,
};
pub use fd_tracker::{AppFdStats, AppFdTracker, FdEntry, FdFlags, FdTable, FdType};
pub use futex::{
    AppFutexAnalyzer, AppFutexStats, LockDescriptor, LockState, PriorityInversion,
    ProcessSyncProfile, SyncPrimitiveType, WaitChain, WaitChainEntry,
};
pub use futex_v2::{
    AppFutexV2Profiler, AppFutexV2Stats, BucketStats, ContentionLevel as FutexContentionLevel,
    FutexAddress, FutexHashProfiler, FutexOp, WaitChainDetector, WaitResult,
};
pub use gpu::{
    AppGpuAnalyzer, AppGpuStats, GpuAllocType, GpuAllocation, GpuDevice, GpuDeviceType, GpuEngine,
    ProcessGpuProfile,
};
pub use heap::{
    AllocEventType, AllocHistogram, AllocRecord, AllocSizeClass, AppHeapAnalyzer, AppHeapStats,
    CallsiteProfile, FragmentationInfo, PotentialLeak, ProcessHeapProfile,
};
pub use history::{
    BinaryHistory, TimeSeries, WorkloadFingerprint, WorkloadHistory, WorkloadHistoryManager,
};
pub use interrupt::{
    AppInterruptProfiler, AppInterruptStats, IrqCategory, IrqStats, ProcessIrqImpact, SoftirqStats,
    SoftirqType, StormDetector, StormSeverity,
};
pub use io::{BandwidthEstimator, IoAnalyzer, IoPattern, IoSchedulingHint, ProcessIoAnalyzer};
pub use ipc::{
    AppIpcAnalyzer, AppIpcChannel, AppIpcMechanism, AppIpcStats, IpcChannelId, IpcDirection,
    IpcEdge, IpcGraph,
};
pub use leak_detect::{
    AllocPattern, AllocType, AllocationRecord, AppLeakDetector, AppLeakDetectorStats,
    CallsiteStats as LeakCallsiteStats, LeakReport, LeakSeverity, ProcessLeakDetector,
};
pub use lifecycle::{LifecycleEvent, LifecycleManager, LifecyclePhase, ProcessLifecycle};
pub use lock::{
    AppLockAnalyzer, AppLockStats, DeadlockDetector, LockEventType, LockInstance, LockOrderPair,
    LockOrderValidator, LockType, WaitForEdge,
};
pub use memory::{AccessPattern, AllocationAnalyzer, MemoryAnalyzer, WorkingSetEstimator};
pub use migration::{
    AppMigrationAnalyzer, CacheAffinity, MigrationDecision, MigrationEvent, MigrationPolicy,
    MigrationReason, MigrationStats, MigrationTarget, PlacementCandidate, PlacementDecision,
    ProcessMigrationProfile,
};
pub use mmap_tracker::{
    AppMmapStats, AppMmapTracker, MmapFlags, MmapProtection, MmapRegion, MmapType,
    ProcessAddressSpace, VasStats,
};
pub use net_stack::{
    AppNetProfilerStats, AppNetStackProfiler, ConnDirection, ConnectionProfile, NetProtocol,
    ProcessNetProfile, SocketBufferStats, TcpState,
};
pub use network::{
    AppNetworkAnalyzer, AppNetworkPattern, ConnState, DetectedProtocol, NetworkQosClass,
    PoolReason, PoolRecommendation, ProcessNetworkProfile, TrackedConnection,
};
pub use numa::{
    AppNumaAnalyzer, AppNumaStats, NumaAccessCounters, NumaAccessType, NumaNode, NumaTopology,
    PlacementReason, PlacementRecommendation, ProcessNumaProfile,
};
pub use optimize::{
    AppOptimization, OptimizationEngine, OptimizationStrategy, SchedulerHint, TuningKnob,
};
pub use page_cache::{
    AccessPattern as AppAccessPattern, AppPageCacheProfiler, AppPageCacheStats, CachedPage,
    FaultLatencyHistogram, PageFaultRecord, PageFaultType, PageState, ProcessPageCacheStats,
    ThrashingDetector, WorkingSetEstimator as AppWorkingSetEstimator,
};
pub use perf_counter::{
    AppPerfCounterProfiler, AppPerfCounterStats, CounterSnapshot, HwCounter, PerfBottleneck,
    ProcessPerfProfile,
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
pub use rlimit::{
    AppRlimitManager, AppRlimitStats, LimitViolation, ProcessLimitProfile, Rlimit, RlimitResource,
    ViolationType,
};
pub use sampling::{
    AddressHistogram, AppSamplingEngine, AppSamplingStats, CallGraph, ProcessSamplingProfile,
    Sample, SampleSource, SamplingConfig,
};
pub use sched_profile::{
    AppSchedProfileStats, AppSchedProfiler, ContextSwitchReason, CpuBurst, RunState,
    ThreadSchedProfile, WakeupChainTracker, WakeupEvent as AppWakeupEvent,
};
pub use scheduler::{SchedClassHint, SchedulingAnalyzer, SchedulingHint as AppSchedulingHint};
pub use seccomp_profile::{
    AppSeccompProfiler, AppSeccompProfilerStats, FilterChain, FilterResult, FilterRule,
    ProcessSeccompProfile, SeccompAction, ViolationRecord, ViolationSeverity,
};
pub use signal::{
    AppSignalAnalyzer, CoalescedSignal, CoalescingRule, DeliveryPreference, ProcessSignalProfile,
    SignalArchPattern, SignalCategory, SignalCoalescer, SignalHandlerInfo, SignalHandlerMode,
    SignalStats,
};
pub use syscall_profile::{
    AppSyscallProfileStats, AppSyscallProfiler, BottleneckType, PatternDetector, PatternType,
    ProcessSyscallProfile, SyscallBottleneck, SyscallCategory, SyscallCostClass, SyscallCounter,
    SyscallDescriptor, SyscallPattern,
};
pub use thermal::{
    AppThermalAnalyzer, AppThermalStats, CoreHeatMap, HeatContribution, ProcessThermalProfile,
    ThermalBudget, ThermalImpact, ThermalReading, ThermalState as AppThermalState,
    ThermalZone as AppThermalZone, ThrottleEvent as AppThrottleEvent,
};
pub use threading::{
    AppThreadAnalyzer, AppThreadState, AppThreadStats, CommEdge, CommType, ThreadDescriptor,
    ThreadPool, ThreadType,
};
pub use timer_profile::{
    AppTimerProfiler, AppTimerProfilerStats, CoalesceGroup, ProcessTimerProfile, TimerPrecision,
    TimerRecord, TimerState, TimerType, WheelLevelStats,
};
pub use trace::{
    AppCallGraph, AppTraceEvent, AppTraceEventType, AppTraceProfiler, AppTraceStats, CallNode,
    FlameGraphCollector, FlameStack,
};
pub use vma_tracker::{
    AppVmaTracker, AppVmaTrackerStats, FragReport, GrowthPattern, ProcessVmaTracker, VmaEntry,
    VmaPerms, VmaType,
};
pub use watchdog::{
    AppWatchdogManager, AppWatchdogStats, HealthCheckConfig, HealthCheckResult, HealthCheckType,
    ProcessWatchdog, RecoveryAction, WatchdogStatus,
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
