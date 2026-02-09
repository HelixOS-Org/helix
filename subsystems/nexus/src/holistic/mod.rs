//! # Holistic Optimization — System-Wide Optimization Engine
//!
//! Milestone 4.4: Integrates information from all subsystems to perform
//! global optimizations that no single component could achieve alone.
//! Combines syscall intelligence (bridge), application understanding (apps),
//! and cooperation feedback (coop) into unified optimization decisions.

extern crate alloc;

pub mod balance;
pub mod global;
pub mod orchestrate;
pub mod policy;
pub mod predict;

// Year 4 Expansion — Holistic subsystem deep modules
pub mod analyzer;
pub mod capacity;
pub mod emergency;
pub mod fairness;
pub mod io;
pub mod memory;
pub mod power;
pub mod scheduler;
pub mod sla;
pub mod thermal;
pub mod topology;

// Year 4 Expansion — Round 2 holistic modules
pub mod anomaly_holistic;
pub mod latency;
pub mod network_holistic;
pub mod profiler;
pub mod qos;
pub mod resource_pool;
pub mod scaling;
pub mod workload;

// Year 4 Expansion — Round 3 holistic modules
pub mod correlation;
pub mod forecast;
pub mod governance;
pub mod health;
pub mod migration_holistic;
pub mod optimization;
pub mod pressure;
pub mod telemetry_holistic;

// Year 4 Expansion — Round 4 holistic modules
pub mod accounting;
pub mod adaptive;
pub mod congestion;
pub mod energy_holistic;
pub mod event_holistic;
pub mod feedback_holistic;
pub mod fragmentation;
pub mod isolation;

// Year 4 Expansion — Round 5 holistic modules
pub mod budget_holistic;
pub mod capacity_holistic;
pub mod dedup_holistic;
pub mod diagnostic;
pub mod placement;
pub mod predictor;
pub mod reclaim;
pub mod scheduling;

// Year 4 Expansion — Round 6 holistic modules
pub mod cache_manager;
pub mod compaction;
pub mod entropy;
pub mod io_scheduler;
pub mod memory_tiering;
pub mod numa_balancer;
pub mod power_governor;
pub mod sched_domain;

// Year 4 Expansion — Round 7 holistic modules
pub mod bandwidth_mgr;
pub mod cfs_tuner;
pub mod cpu_topology;
pub mod deadline_mgr;
pub mod hotplug;
pub mod interrupt_balance;
pub mod load_balance;
pub mod memory_compress;
pub mod sched_group;

// Year 4 Expansion — Round 8 holistic modules
pub mod cgroup_mgr;
pub mod cpu_freq;
pub mod io_priority;
pub mod irq_balance;
pub mod lock_contention;
pub mod numa_place;
pub mod page_cache;
pub mod slab_optimizer;
pub mod swap_mgr;
pub mod tlb_mgr;
pub mod watchdog_mgr;

// Year 4 Expansion — Round 9 holistic modules
pub mod cpu_idle;
pub mod dma_mgr;
pub mod hugepage_mgr;
pub mod kthread_pool;
pub mod memcg_mgr;
pub mod oom_killer;
pub mod perf_counter;
pub mod rcu_tracker;
pub mod vmstat_tracker;
pub mod writeback_ctrl;
pub mod zram_mgr;

// Year 4 Expansion — Round 10 holistic modules
pub mod cred_mgr;
pub mod fpu_context;
pub mod futex_tracker;
pub mod ksm_dedup;
pub mod mem_watermark;
pub mod mmap_advisor;
pub mod readahead_tuner;
pub mod signal_dispatch;
pub mod sysctl_tuner;
pub mod task_affinity;
pub mod tick_mgr;
pub mod workqueue_mgr;

// Year 4 Expansion — Round 11 holistic modules
pub mod dirty_tracker;
pub mod iommu_mgr;
pub mod kcalloc_pool;
pub mod migrate_pages;
pub mod mprotect_mgr;
pub mod pgtable_mgr;
pub mod preempt_ctrl;
pub mod rss_tracker;
pub mod sched_latency;
pub mod softirq_mgr;
pub mod timer_wheel;

// Year 4 Expansion — Round 12 holistic modules
pub mod blk_throttle;
pub mod cgroup_orchestrator;
pub mod cpuset_ctrl;
pub mod memory_compact;
pub mod net_classifier;
pub mod oom_reaper;
pub mod page_alloc;
pub mod perf_events;
pub mod psi_monitor;
pub mod sched_domains;
pub mod thermal_zone;

// Round 13
pub mod affinity_mgr;
pub mod clock_source;
pub mod dma_engine;
pub mod firmware_mgr;
pub mod hotplug_mgr;
pub mod irq_domain;
pub mod numa_policy;
pub mod power_domain;
pub mod rcu_tree;
pub mod wq_scheduler;

// Round 14
pub mod balloon_driver;
pub mod cache_partition;
pub mod dma_fence;
pub mod freq_scaling;
pub mod msi_controller;
pub mod page_table;
pub mod pci_enum;
pub mod slab_alloc;

// Round 15
pub mod acpi_mgr;
pub mod buddy_alloc;
pub mod cfs_sched;
pub mod ftrace_mgr;
pub mod irq_affinity;
pub mod kprobe_mgr;
pub mod rcu_sync;
pub mod vma_mgr;

// Round 16
pub mod cpufreq_gov;
pub mod devfreq_mgr;
pub mod dma_pool;
pub mod hwmon_mgr;
pub mod iommu_alloc;
pub mod msi_mgr;
pub mod numa_mgr;
pub mod power_mgr;
pub mod thermal_mgr;
// Round 17
pub mod blk_mq;
pub mod cfs_bandwidth;
pub mod cgroup_mem;
pub mod futex_mgr;
pub mod io_sched;
pub mod ksm_mgr;
pub mod memcg_oom;
pub mod workqueue;
// Round 18
pub mod balloon_drv;
pub mod cgroup_cpu;
pub mod dirty_writeback;
pub mod ebpf_verifier;
pub mod huge_page_alloc;
pub mod irq_thread;
pub mod kprobes;
pub mod percpu_alloc;
pub mod softirq;
pub mod vmalloc;
// Round 19
pub mod freelist;
pub mod iommu;
pub mod ksm;
pub mod numa_balance;
pub mod page_reclaim;
pub mod zswap;
// Round 20
pub mod cgroup_io;
pub mod memcg_reclaim;
pub mod mempolicy;
// Round 21
pub mod bio_layer;
pub mod block_dev;
pub mod btrfs_cow;
pub mod dentry_cache;
pub mod ext4_journal;
pub mod f2fs_gc;
pub mod inode_cache;
pub mod nfs_client;
pub mod tmpfs_mgr;
pub mod xfs_log;
// Round 22
pub mod arp_cache;
pub mod dns_cache;
pub mod ip_routing;
pub mod net_device;
pub mod net_ns;
pub mod netfilter;
pub mod qdisc;
pub mod socket_mgr;
pub mod tcp_stack;
pub mod udp_mgr;
pub mod xdp_mgr;
// Round 23
pub mod vfs_holistic;
pub mod mount_holistic;
pub mod dentry_holistic;
pub mod inode_holistic;
pub mod superblock_holistic;
pub mod file_lock_holistic;
pub mod bio_holistic;
pub mod blkdev_holistic;
pub mod ioscheduler_holistic;
pub mod raid_holistic;
pub mod dm_holistic;
// Round 24
pub mod acl_holistic;
pub mod chmod_holistic;
pub mod chown_holistic;
pub mod extent_holistic;
pub mod journal_holistic;
pub mod pagecache_holistic;
pub mod quota_holistic;
pub mod readahead_holistic;
pub mod stat_holistic;
pub mod writeback_holistic;
pub mod xattr_holistic;
// Round 25 — Security holistic analysis modules
pub mod audit_holistic;
pub mod capability_holistic;
pub mod credential_holistic;
pub mod crypto_holistic;
pub mod integrity_holistic;
pub mod keyring_holistic;
pub mod lsm_holistic;
pub mod mac_holistic;
pub mod namespace_holistic;
pub mod sandboxing_holistic;
pub mod seccomp_holistic;

// Round 26 — IPC/signal holistic analysis
pub mod eventfd_holistic;
pub mod futex_holistic;
pub mod ipc_holistic;
pub mod mqueue_holistic;
pub mod msgqueue_holistic;
pub mod pipe_holistic;
pub mod semaphore_holistic;
pub mod shm_holistic;
pub mod sigaction_holistic;
pub mod signal_holistic;
pub mod timerfd_holistic;

// Round 27 — Networking/socket holistic analysis
pub mod bandwidth_holistic;
pub mod congestion_holistic;
pub mod connection_holistic;
pub mod epoll_holistic;
pub mod latency_holistic;
pub mod netstack_holistic;
pub mod routing_holistic;
pub mod socket_holistic;
pub mod tcp_holistic;
pub mod udp_holistic;

// Round 28 — Filesystem/VFS holistic analysis modules
pub mod flock_holistic;
pub mod page_cache_holistic;

// Round 29 — Process/thread holistic modules
pub mod clone_holistic;
pub mod exec_holistic;
pub mod exit_holistic;
pub mod fork_holistic;
pub mod nice_holistic;
pub mod pgid_holistic;
pub mod pid_holistic;
pub mod prctl_holistic;
pub mod session_holistic;
pub mod thread_holistic;
pub mod wait_holistic;

// Re-exports from Round 4 modules
pub use accounting::{
    AccountableResource, AccountingPeriod, EntityType, HolisticAccountingEngine,
    HolisticAccountingStats, ResourceLedger, UsageEntry,
};
pub use adaptive::{
    ControlLoop, ControlLoopState, ControlMode, ControlVariable, HolisticAdaptiveEngine,
    HolisticAdaptiveStats, PidController,
};
// Re-exports from expanded modules
pub use analyzer::{
    BottleneckSeverity, BottleneckType, MetricCorrelation, MetricSample, MetricTimeSeries,
    SystemAnalyzer, SystemHealth, SystemMetricType,
};
// Re-exports from Round 2 modules
pub use anomaly_holistic::{
    AnomalySource, CascadeEvent, HolisticAnomaly, HolisticAnomalyManager, HolisticAnomalySeverity,
    HolisticAnomalyStats, HolisticAnomalyType, MetricCorrelation as HolisticMetricCorrelation,
    MetricTracker,
};
// Re-exports from Round 6 modules
    DriftDetector, HolisticAnomaly as AnomalyV2Entry, HolisticAnomalySeverity as AnomalyV2Severity,
    HolisticAnomalyType as AnomalyV2Type, HolisticAnomalyV2, HolisticAnomalyV2Stats,
    MetricAnomalyTracker,
};
pub use balance::*;
// Re-exports from Round 7 modules
pub use bandwidth_mgr::{
    BwReservation, BwResource, BwShare, CongestionLevel as BwCongestionLevel, DeviceBandwidth,
    HolisticBandwidthMgr, HolisticBandwidthMgrStats, TokenBucket,
};
// Re-exports from Round 5 modules
pub use budget_holistic::{
    BudgetPeriod, BudgetState, BudgetedResource, HolisticBudgetEngine, HolisticBudgetStats,
    ResourceBudget, TenantBudget,
};
pub use cache_manager::{
    CachePartitionEntry, HolisticCacheManager, HolisticCacheManagerStats, PageClassifier, PageTemp,
    PartitionScheme, PrefetchHint,
};
pub use capacity::{
    CapacityPlanner, CapacityResource, PlannerConfig, ResourceCapacity, ScalingDirection,
    ScalingRecommendation, Scenario, ScenarioResult, UsageSample,
};
pub use capacity_holistic::{
    CapacityDataPoint, CapacityHealth, CapacityResource as HolisticCapacityResource,
    CapacityScenario, CapacityTimeSeries, CapacityTrend, HolisticCapacityEngine,
    HolisticCapacityStats, SizingRecommendation,
};
pub use cfs_tuner::{
    CfsParameters, CfsTunable, CpuCfsStats, HolisticCfsTuner, HolisticCfsTunerStats,
    LatencyHistogram, TuningDirection, TuningRecommendation,
};
// Round 8 re-exports
pub use cgroup_mgr::{
    CgroupController, CgroupNode, CpuCgroupLimits, HolisticCgroupMgr, HolisticCgroupStats,
    IoCgroupLimits, MemCgroupLimits, PsiInfo, PsiLevel,
};
pub use compaction::{
    CompactAction, CompactZone, CompactionUrgency, HolisticCompactionEngine,
    HolisticCompactionStats, HugePagePool, PageOrder, ZoneCompactState,
};
pub use congestion::{
    BackpressureAction, CongestionLevel, CongestionResource, CongestionWindow, CwndState,
    HolisticCongestionEngine, HolisticCongestionStats, ResourceCongestion,
};
// Re-exports from Round 3 modules
pub use correlation::{
    CorrelationCluster, CorrelationMetricSource, CorrelationResult, CorrelationSeries,
    CorrelationType, HolisticCorrelationEngine, HolisticCorrelationStats,
};
pub use cpu_freq::{
    CpuFreqGovernor, CpuFreqState, EppHint, FreqDomain, FreqTransitionReason, HolisticCpuFreqGov,
    HolisticCpuFreqStats, TurboBudget,
};
// Re-exports from Round 9 modules
pub use cpu_idle::{
    CStateLevel, CpuIdleState, CpuIdleStats, HolisticCpuIdle, IdleGovernor, PackageCState,
    WakeSource,
};
pub use cpu_topology::{
    CacheDomain, CacheLevel as TopologyCacheLevel, CoreType, HolisticCpuTopology,
    HolisticCpuTopologyStats, LogicalCpu as TopologyLogicalCpu,
    NumaDistance as TopologyNumaDistance, Package, PlacementHint, TopologyPlacement,
};
pub use deadline_mgr::{
    AdmissionResult as DeadlineAdmission, DeadlineClass, DeadlineParams, DeadlineTaskState,
    HolisticDeadlineMgr, HolisticDeadlineMgrStats, MissSeverity, SlackInfo,
};
pub use dedup_holistic::{
    DedupScanner, DedupState as HolisticDedupState, HolisticDedupEngine, HolisticDedupStats,
    MergeGroup, PageFingerprint, ScanPriority, ScanStats,
};
pub use diagnostic::{
    DiagnosisConfidence, DiagnosisReport, FaultNode, FaultNodeType, FaultTree,
    HolisticDiagnosticEngine, HolisticDiagnosticStats, RootCause, Symptom, SymptomCategory,
    SymptomSeverity,
};
pub use dma_mgr::{
    DmaBuffer, DmaCoherency, DmaDirection, DmaStats as HolisticDmaStats, DmaTransfer, DmaZone,
    DmaZoneState, HolisticDmaMgr, IommuMapping, SgEntry, SgList,
};
pub use emergency::{
    DegradationAction, EmergencyAction, EmergencyActionType, EmergencyEvent, EmergencyLevel,
    EmergencyManager, EmergencyTrigger, RecoveryProcedure, RecoveryState, ServicePriority,
    ServiceRecord, ServiceState, WatchdogEntry,
};
pub use energy_holistic::{
    BatteryState as HolisticBatteryState, EnergyBudget, EnergyProfile, HolisticEnergyEngine,
    HolisticEnergyStats, PowerDomain as HolisticPowerDomain, PowerSource,
    PowerState as HolisticPowerState,
};
pub use entropy::{
    EntropyPool, EntropySource, HolisticEntropyStats, HolisticEntropyTracker, PoolHealth,
    SourceTracker,
};
pub use event_holistic::{
    AggregationWindow, CorrelatedGroup, EventPattern, EventSeverity, EventSource,
    HolisticEventEngine, HolisticEventStats, PatternCondition, SystemEvent,
};
pub use fairness::{
    FairnessEngine, FairnessMetrics, FairnessReport, FairnessResource, ProcessFairness,
    ResourceShare, StarvationConfig,
};
    EntityAllocation, EnvyPair, FairnessResource as FairnessV2Resource, FairnessViolation,
    FairnessViolationSeverity, HolisticFairnessV2, HolisticFairnessV2Stats, MaxMinResult,
};
pub use feedback_holistic::{
    CascadeController, ControllerType, FeedbackControllerState, FeedbackLoop, FeedbackVariable,
    GainSchedule, HolisticFeedbackEngine, HolisticFeedbackStats,
};
pub use forecast::{
    CapacityPlan, CapacityRisk, ForecastMetric, ForecastResult, ForecastScenario, ForecastSeries,
    HolisticForecastEngine, HolisticForecastHorizon, HolisticForecastStats, TrendDirection,
};
pub use fragmentation::{
    BuddyOrderStats, FragSeverity, FragType, HolisticFragmentationEngine,
    HolisticFragmentationStats, HugePageAvailability, MemoryZone as FragMemoryZone, SlabCacheInfo,
    ZoneFragStats,
};
pub use global::*;
pub use governance::{
    ComplianceRecord, EnforcementMode, GovernancePolicy, GovernancePolicyState, GovernanceScope,
    GovernanceViolation, GovernedResource, HolisticGovernanceEngine, HolisticGovernanceStats,
    ResourceLimit, ViolationSeverity,
};
pub use health::{
    HealthCause, HealthDimension, HealthIssue, HealthLevel, HealthReport, HolisticHealthEngine,
    HolisticHealthStats, VitalSign,
};
pub use hotplug::{
    CpuHotplugState, HolisticHotplugMgr, HolisticHotplugStats, HotplugAction, HotplugEvent,
    HotplugPolicy, HotplugResource, MemorySection,
};
pub use hugepage_mgr::{
    CompactionRequest, HolisticHugepageMgr, HugeAllocRecord, HugePageSize, HugePageStats,
    NodeHugePool, ThpDefrag, ThpMode, ThpStats,
};
pub use interrupt_balance::{
    BalanceStrategy as IrqBalanceStrategy, CoalesceSuggestion, CpuIrqLoad, HolisticIrqBalance,
    HolisticIrqBalanceStats, InterruptType, IrqRebalance, IrqStats,
};
pub use io::{
    DeviceStats, DeviceType, HolisticIoManager, IoMergeEngine, IoProcessPriority, IoRequest,
    IoSchedClass, MergedRequest, ReadAheadConfig, WritebackPolicy,
};
pub use io_priority::{
    DeviceSaturation, HolisticIoPriority, HolisticIoPriorityStats, IoOpType, IoPriorityClass,
    IoRequest as HolisticIoRequest, ProcessIoWeight, ReadaheadState,
};
pub use io_scheduler::{
    DeviceQueue, HolisticIoScheduler, HolisticIoSchedulerStats, IoDeviceType, IoDirection,
    IoPriorityClass, IoRequest as IoSchedRequest, IoSchedAlgo,
};
pub use irq_balance::{
    CoalesceStrategy, CpuIrqLoad, HolisticIrqBalance, HolisticIrqBalanceStats, IrqDeliveryMode,
    IrqDescriptor, IrqMigration, IrqMigrationReason, IrqType, MsiXAssignment,
};
pub use isolation::{
    HolisticIsolationEngine, HolisticIsolationStats, InterferenceDetector, InterferenceEvent,
    InterferenceType, IsolationDomain, IsolationDomainType, IsolationStrength, ResourcePartition,
};
pub use kthread_pool::{
    HolisticKthreadPool, KThreadPoolStats, KWorkItem, KWorkPriority, KWorker, KWorkerState,
    KWorkqueue,
};
pub use latency::{
    HolisticLatencyAnalyzer, LatencyAnalyzerStats, LatencyBudget, LatencyComponent,
    LatencyPercentiles, LatencySpan, RequestTrace,
};
pub use load_balance::{
    BalanceDomainLevel, BalanceGroup, CpuLoadState, HolisticLoadBalance, HolisticLoadBalanceStats,
    LoadDimension, LoadVector, LoadWeights, MigrationRecommendation,
    MigrationType as LbMigrationType,
};
pub use lock_contention::{
    ContentionHotspot, ContentionLevel, HolisticLockContention, HolisticLockContentionStats,
    LockDepGraph, LockInstance, LockOrderEdgeHolistic, PriorityInversionHolistic, SystemLockType,
};
pub use memcg_mgr::{
    HolisticMemcgMgr, Memcg, MemcgCounters, MemcgEvent, MemcgEventKind, MemcgLimit, MemcgOomPolicy,
    MemcgPressure, MemcgReclaimInfo, MemcgStats,
};
pub use memory::{
    HolisticMemoryManager, MemoryPressure, MemoryZone, OomPolicy, OomScore, ReclaimAction,
    ReclaimPolicy, ReclaimTarget, ZoneStats,
};
pub use memory_compress::{
    AdmissionDecision as CompressAdmission, CompressAlgorithm, CompressPool, CompressPoolType,
    CompressedPage, HolisticMemCompress, HolisticMemCompressStats,
};
pub use memory_tiering::{
    HolisticMemoryTiering, HolisticMemoryTieringStats, MemoryTier, PageHotness, PageTierInfo,
    TierInfo, TierMigrationDecision, TierMigrationDir,
};
pub use migration_holistic::{
    HolisticMigrationEngine, HolisticMigrationReason, HolisticMigrationState,
    HolisticMigrationStats, MigrationCostBenefit, MigrationPriority, MigrationRequest,
    MigrationTarget, ProcessMigrationHistory,
};
pub use network_holistic::{
    BandwidthAllocation, BandwidthClass, CongestionDetector, CongestionState, FlowDirection,
    HolisticInterfaceType, HolisticNetworkAnalyzer, HolisticNetworkStats, HolisticProtocol,
    InterfaceProfile, NetworkFlow,
};
pub use numa_balancer::{
    HolisticNumaBalancer, HolisticNumaBalancerStats, NumaDistanceMatrix, NumaMigReason,
    NumaNode as NumaBalancerNode, NumaNodeState, NumaPageMigration,
};
pub use numa_place::{
    AccessLocality, HolisticNumaPlace, HolisticNumaPlaceStats, MigrationCandidate,
    NumaDistanceEntry, NumaMigrationDir, NumaNodeMemState, ProcessNumaProfile,
};
pub use oom_killer::{
    CgroupOomState, HolisticOomKiller, KillStage, OomCandidate, OomKillRecord, OomPolicy, OomStats,
    OomTrigger,
};
pub use optimization::{
    ConstraintDef, ConstraintType, HolisticOptimizationEngine, HolisticOptimizationStats,
    ObjectiveDef, OptSolution, OptimizationDirection, OptimizationObjective, ParetoFront,
};
pub use orchestrate::*;
pub use page_cache::{
    CacheEvictionPolicy, DeviceWritebackState, GlobalCacheStats, HolisticPageCache,
    InodeCacheState, ProcessCacheUsage, WritebackMode, WritebackParams,
};
pub use perf_counter::{
    CounterGroup, CounterMode, DerivedMetric, HolisticPerfCounter, HwEvent,
    PerfCounter as HolisticPerfCounterEntry, PerfCounterStats, PerfSample, PmuState,
};
pub use placement::{
    HolisticPlacementEngine, HolisticPlacementStats, InterferenceLevel, InterferenceModel,
    InterferencePair, PlacementCandidate as HolisticPlacementCandidate, PlacementConstraint,
    PlacementDimension, PlacementRequest, PlacementResult,
};
pub use policy::*;
pub use power::{
    BatteryInfo, BatteryState, CState, PState, PowerBudget, PowerDomain, PowerEstimate,
    PowerManager, PowerProfile,
};
pub use power_governor::{
    CState as GovCState, CpuFreqState, FreqAction, HolisticPowerGovernor,
    HolisticPowerGovernorStats, PowerDomain as GovPowerDomain, PowerPolicy,
};
pub use predict::*;
pub use predictor::{
    CorrelationTracker, HolisticPredictorEngine, HolisticPredictorStats,
    MetricCorrelation as PredictorMetricCorrelation, MetricSample as PredictorMetricSample,
    MetricWindow, Prediction, PredictionMethod, PredictionTarget, SloDefinition, SloPrediction,
    SloState,
};
pub use pressure::{
    HolisticPressureEngine, HolisticPressureStats, PressureCategory, PressureEvent,
    PressureResource, PressureSeverity, PressureWindow, ResourcePressure, ThrottleRecommendation,
};
pub use profiler::{
    HolisticProfiler, HolisticProfilerStats, Hotspot, LockContention, OptimizationHint,
    OptimizationHintType, ProfileDomain, ProfileGranularity, ProfileSample, ProfileSession,
    ProfileSessionState, SampleSource,
};
pub use qos::{
    AdmissionState, HolisticQosManager, HolisticQosStats, QosAdmissionResult, QosClass,
    QosEnforcementMode, QosPolicy, QosResource, ResourceGuarantee,
};
    HolisticQosV2, HolisticQosV2Stats, QosClassV2, QosGroupV2, QosResourceV2, QosSloType, QosSloV2,
    ResourceAllocation, SloViolation,
};
pub use rcu_tracker::{
    GracePeriod, GracePeriodState, HolisticRcuTracker, PerCpuRcuState, RcuCallback, RcuFlavor,
    RcuStall as RcuStallEntry, RcuStats as HolisticRcuStats,
};
pub use reclaim::{
    HolisticReclaimEngine, HolisticReclaimStats, OomCandidate, OomKiller, ProcessReclaimable,
    ReclaimAction as HolisticReclaimAction, ReclaimSource as HolisticReclaimSource, ReclaimUrgency,
    ReclaimZoneType, ZoneState,
};
    AgeHistogramBucket, CgroupMemPressure, HolisticReclaimV2, HolisticReclaimV2Stats,
    PageGeneration, ReclaimEvent, ReclaimSource, ReclaimUrgency, WorkingSetEstimatorV2,
    ZoneWatermarks,
};
pub use resource_pool::{
    FragmentationMetrics, HolisticResourcePoolManager, PartitionMode, PoolPartition,
    PoolResourceType, PoolState, ResourcePool, ResourcePoolStats,
};
pub use scaling::{
    DemandForecast, DemandPredictor, HolisticScalingManager, HolisticScalingStats, ScalingDecision,
    ScalingDimension, ScalingDirection as HolisticScalingDirection, ScalingMode, ScalingPolicy,
    ScalingReason, ScalingThreshold,
};
pub use sched_domain::{
    DomainBalanceState, HolisticSchedDomain, HolisticSchedDomainStats, MigrationUrgency,
    SchedCpuGroup, SchedDomain as SchedDomainEntry, SchedDomainLevel, SchedMigrationSuggestion,
};
pub use sched_group::{
    BandwidthParams, BandwidthRuntime, GroupSchedPolicy, HolisticSchedGroup,
    HolisticSchedGroupStats, SchedGroup as SchedGroupEntry,
};
pub use scheduler::{
    CoreLoad, GlobalSchedClass, HolisticScheduler, LoadBalancer, NumaAffinity, ProcessSchedParams,
    SchedDecision, SchedReason, SchedRecord,
};
pub use scheduling::{
    CpuState, HolisticLoadBalancer, HolisticSchedulingEngine, HolisticSchedulingStats,
    HolisticTaskClass, LoadImbalance, PlacementReason, SchedDomain, SchedTask,
};
pub use sla::{
    ErrorBudget, MetricEvaluation, SlaDefinition, SlaEvaluation, SlaMetricType, SlaStatus,
    SlaTarget, SlaTier, SlaViolation, SystemSlaManager, ViolationCause,
};
pub use slab_optimizer::{
    HolisticSlabOptimizer, HolisticSlabStats, MergeCandidate, ShrinkUrgency, SlabAllocator,
    SlabCache, SlabPage,
};
pub use swap_mgr::{
    CompressedSwapState, HolisticSwapMgr, HolisticSwapMgrStats, ProcessSwapUsage,
    SwapAllocStrategy, SwapDevice, SwapDeviceType,
};
pub use telemetry_holistic::{
    AggregationMethod, HolisticTelemetryEngine, HolisticTelemetryStats, MetricPoint, MetricSeries,
    RetentionPolicy, TelemetryMetricType, TelemetrySource,
};
pub use thermal::{
    CoolingDevice, CoolingType, ThermalEvent, ThermalManager, ThermalZone, ThermalZoneType,
    ThrottleLevel, TripPoint, TripType,
};
    CoolingAction, CoolingDevice as CoolingDeviceV2, CoolingType as CoolingTypeV2,
    HolisticThermalV2, HolisticThermalV2Stats, ThermalBudget, ThermalZone as ThermalZoneV2,
    ThermalZoneType as ThermalZoneTypeV2, TripPointType,
};
pub use tlb_mgr::{
    CpuTlbState, HolisticTlbMgr, HolisticTlbMgrStats, HugePageCandidate, PcidSlot, ShootdownBatch,
    ShootdownRequest, TlbFlushReason, TlbPageSize,
};
pub use topology::{
    CacheDescriptor, CacheLevel, CpuPackage, InterconnectType, LogicalCpu, NumaDistance, NumaNode,
    NumaPolicy, PhysicalCore, Proximity, ProximityLevel, TopologyDevice, TopologyManager,
    TopologySummary,
};
pub use vmstat_tracker::{
    HolisticVmstatTracker, PageState as VmPageState, ReclaimCounters,
    SwapCounters as VmSwapCounters, VmRateSample, VmStatSummary, VmZone, ZoneCounters, ZoneDesc,
};
pub use watchdog_mgr::{
    CpuWatchdog, HolisticWatchdogMgr, HolisticWatchdogMgrStats, HungTaskEntry, LockupType,
    RcuStallRecord, WatchdogEvent, WatchdogRecoveryAction, WatchdogState as HolisticWatchdogState,
};
pub use workload::{
    HolisticWorkloadAnalyzer, HolisticWorkloadStats, LoadPattern, PhaseDetector, ResourceSnapshot,
    WorkloadClass, WorkloadFingerprint, WorkloadMix, WorkloadPhase,
};
pub use writeback_ctrl::{
    BdiState, DirtyLimits, HolisticWritebackCtrl, InodeWbState, ThrottleInfo, WritebackReason,
    WritebackState, WritebackStats, WritebackWork,
};
pub use zram_mgr::{
    CompAlgoStats, HolisticZramMgr, ZramCompAlgo, ZramDevice, ZramStats, ZramWritebackReason,
    ZramWritebackRecord,
};

// Round 10 re-exports
pub use cred_mgr::{
    CapSet, Capability, CredEvent, CredEventType, CredManagerStats, HolisticCredManager,
    ProcessCred,
};
pub use fpu_context::{
    CpuFpuCaps, FpuContextStats, FpuException, FpuFeature, FpuStrategy, HolisticFpuContext,
    MxcsrFlags, TaskFpuContext,
};
pub use futex_tracker::{
    FutexBucket, FutexOp, FutexTrackerStats, FutexWaiter, HolisticFutexTracker, PiChain,
    RequeueOp, WaiterState as FutexWaiterState,
};
pub use ksm_dedup::{
    HolisticKsmDedup, KsmDedupStats, KsmPage, KsmPageState, KsmScanConfig,
    PageFingerprint as KsmFingerprint, ProcessKsmInfo, StableTreeNode,
};
pub use mem_watermark::{
    HolisticMemWatermark, MemZone, OrderWatermark, WatermarkLevel, WatermarkStats,
    ZoneType as WatermarkZoneType, ZoneWatermarks as WatermarkThresholds,
};
pub use mmap_advisor::{
    AddressGap, HolisticMmapAdvisor, MadviseHint, MmapAdvisorStats, ProcessAddressSpace, Vma,
    VmaPerms, VmaType,
};
pub use readahead_tuner::{
    AccessPattern as ReadaheadAccessPattern, FileReadahead, HolisticReadaheadTuner,
    InterleavedStream, ReadaheadState as ReadaheadTunerState, ReadaheadStats,
};
pub use signal_dispatch::{
    HolisticSignalDispatch, ProcessSignalState, QueuedSignal, SigAction, SigMask, SignalClass,
    SignalDispatchStats, SignalDisposition,
};
pub use sysctl_tuner::{
    ChangeReason, HolisticSysctlTuner, ParamBounds, ParamCategory, ParamChange, ParamType,
    SysctlParam, SysctlTunerStats, TuningRecommendation as SysctlRecommendation,
    WorkloadProfile as SysctlWorkloadProfile,
};
pub use task_affinity::{
    CacheDomain as AffinityCacheDomain, CpuSet, HolisticTaskAffinity, MemBindPolicy,
    MigrationCost, NodeSet, TaskAffinity, TaskAffinityStats,
};
pub use tick_mgr::{
    BroadcastState, CpuTickState, HolisticTickMgr, TickMgrStats, TickMode,
    TimerEntry as TickTimerEntry, TimerState as TickTimerState, TimerType as TickTimerType,
};
pub use workqueue_mgr::{
    HolisticWorkqueueMgr, WorkItem as WqWorkItem, WorkItemState as WqWorkItemState,
    WorkerPool as WqWorkerPool, WorkqueueStats, WqDescriptor, WqType,
};

// ── Round 11 re-exports ──
pub use dirty_tracker::{
    DirtyLimits, DirtyPage, DirtyState, DirtyTrackerStats, HolisticDirtyTracker,
    ProcessDirtyState, WritebackBatch, WritebackPriority,
};
pub use iommu_mgr::{
    DmaFault, HolisticIommuMgr, IommuDomain, IommuDomainType, IommuMgrStats, IotlbCache,
    IotlbEntry, IovaRegion,
};
pub use kcalloc_pool::{
    AllocState as KcAllocState, CpuCache as KcCpuCache, EmergencyReserve, HolisticKcallocPool,
    KcallocPoolStats, LeakSuspect, ObjectHeader, SizeClass as KcSizeClass,
    SlabPage as KcSlabPage,
};
pub use migrate_pages::{
    HolisticMigratePages, MemoryTier as MigMemoryTier, MigratePageStats,
    MigrationReason as MigPageReason, MigrationRequest as MigPageRequest,
    NodeMigrationState, PageHotness as MigPageHotness, TrackedPage,
};
pub use mprotect_mgr::{
    AccessType, HolisticMprotectMgr, MprotectMgrStats, ProcessProtState, ProtFlags,
    ProtRegion, ProtViolation, ProtectionKey, StackGuard, WxPolicy,
};
pub use pgtable_mgr::{
    HolisticPgtableMgr, PageTablePage, PcidEntry, PgtableMgrStats, ProcessPageTable, PtLevel,
    PteFlags, ThpCandidate,
};
pub use preempt_ctrl::{
    CpuPreemptState, CriticalSection, DisableReason, HolisticPreemptCtrl,
    LatencyBudget as PreemptLatencyBudget, PreemptCtrlStats, PreemptHotspot, PreemptModel,
    PreemptDisableEntry,
};
pub use rss_tracker::{
    HolisticRssTracker, ProcessRss, RssComponent, RssLimitType, RssTrackerStats,
    SystemRssSummary,
};
pub use sched_latency::{
    ContextSwitchInfo, CpuSchedState, HolisticSchedLatency, LatencyCategory,
    LatencySample, SchedEventType, SchedLatencyStats, TaskLatencyState,
};
pub use softirq_mgr::{
    BurstDetector, CpuSoftirqState, HolisticSoftirqMgr, SoftIrqType, SoftirqMgrStats,
    SoftirqState, SoftirqVectorStats,
};
pub use timer_wheel::{
    CoalesceGroup, CpuTimerState, HolisticTimerWheel, TimerEntry as WheelTimerEntry,
    TimerState as WheelTimerState, TimerType as WheelTimerType, TimerWheelStats,
    WheelLevel,
};

// ── Round 12 re-exports ──
pub use blk_throttle::{
    BlkThrottleStats, BwLimit, CgroupIoStat, HolisticBlkThrottle, IoDirection,
    LatencyTarget, ThrottleDevice, ThrottleEvent, ThrottlePolicy, ThrottleReason,
};
pub use cgroup_orchestrator::{
    CgroupController as OrchCgroupController, CgroupLimits, CgroupNode as OrchCgroupNode,
    CgroupOrchStats, CgroupUsage, CgroupVersion, HolisticCgroupOrch, OrchAction,
};
pub use cpuset_ctrl::{
    CpuDistPolicy, Cpuset, CpusetMigration, CpusetPartition, CpusetStats,
    CpusetViolation, HolisticCpusetCtrl, MemPlacePolicy, ViolationType,
};
pub use memory_compact::{
    CompactEvent, CompactMode, CompactResult, CompactStats, HolisticMemoryCompact,
    MigrationScanner, PageMobility, ZoneCompactState,
};
pub use net_classifier::{
    ClassStats, ClassifyMatch, ClassifyRule, FlowEntry, HolisticNetClassifier,
    NetClassifierStats, Protocol as NetProtocol, TrafficClass,
};
pub use oom_reaper::{
    CgroupOomState as ReaperCgroupOomState, HolisticOomReaper, MemPressureLevel,
    OomKillRecord as ReaperOomKillRecord, OomPolicy as ReaperOomPolicy,
    OomReason, OomReaperStats, OomTaskInfo, VictimState,
};
pub use page_alloc::{
    AllocRequest, BuddyOrderStats, GfpFlags, HolisticPageAlloc, PageAllocStats,
    PageZoneType, WatermarkLevel as AllocWatermarkLevel, ZoneAllocState, ZoneWatermarks,
};
pub use perf_events::{
    CpuPerfState, HolisticPerfEvents, HwEventId, PerfEvent, PerfEventConfig,
    PerfEventState, PerfEventType, PerfEventsStats, PerfSample, PmuDesc, PmuType,
    SwEventId,
};
pub use psi_monitor::{
    CgroupPsi, HolisticPsiMonitor, PsiAlert, PsiMonitorStats, PsiReading,
    PsiResource, PsiTrigger, PsiType, PsiWindow,
};
pub use sched_domains::{
    BalanceDecision, BalanceReason, DomainFlags, DomainLevel, GroupType as SdGroupType,
    HolisticSchedDomains, SchedDomain as SdSchedDomain, SchedDomainsStats,
    SchedGroup as SdSchedGroup,
};
pub use thermal_zone::{
    CoolingDevice as TzCoolingDevice, CoolingType as TzCoolingType,
    HolisticThermalZone, ThermalEvent as TzThermalEvent,
    ThermalEventType, ThermalGovernor, ThermalStats, ThermalZone as TzThermalZoneEntry,
    ThermalZoneKind, TripPoint as TzTripPoint, TripType as TzTripType,
};

// Round 13 re-exports
pub use affinity_mgr::{
    AffinityBinding, AffinityMgrStats, AffinityPolicy, AffinityScope,
    CpuMask, HolisticAffinityMgr, MigrationEvent, MigrationReason, NodeMask,
};
pub use clock_source::{
    ClockFlags, ClockQuality, ClockSource as ClockSrcDesc,
    ClockSourceStats, ClockSourceType, ClockState as ClockSrcState,
    ClockWatchdog, HolisticClockSource,
};
    CacheInfo as TopoV2CacheInfo, CacheLevel as TopoV2CacheLevel,
    CacheType as TopoV2CacheType, CpuFeatures, CpuTopoV2Stats,
    HolisticCpuTopoV2, LogicalCpu as TopoV2LogicalCpu,
    MicroArch, TopologyDistances,
};
pub use dma_engine::{
    DmaChannel, DmaChannelState, DmaDirection as DmaEngDirection,
    DmaEngineStats, DmaPriority, DmaRegion, DmaTransfer as DmaEngTransfer,
    DmaTransferType, HolisticDmaEngine, SgEntry,
};
pub use firmware_mgr::{
    FirmwareImage, FirmwareMgrStats, FirmwareSecLevel, FirmwareState,
    FirmwareType, FirmwareUpdateReq, FirmwareVersion, HolisticFirmwareMgr,
};
pub use hotplug_mgr::{
    HotplugAction as HpMgrAction, HotplugMgrStats,
    HotplugNotifier, HotplugOperation, HotplugResource as HpMgrResource,
    HotplugState as HpMgrState, CpuHotplugState as HpMgrCpuState,
    HolisticHotplugMgr as HolisticHpMgr, NotifierPriority,
};
pub use irq_domain::{
    HolisticIrqDomain, IrqDelivery, IrqDesc,
    IrqDomain, IrqDomainStats, IrqState as IrqDomainState,
    IrqTrigger, IrqType as IrqDomainType,
};
pub use numa_policy::{
    HolisticNumaPolicy, MigrationMode, NodeMemInfo, NodeState as NumaNodePolicyState,
    NumaBalanceEvent, NumaDistance as NumaPolicyDistance,
    NumaPolicy as NumaPolicyType, NumaPolicyStats, ProcessNumaBinding,
};
pub use power_domain::{
    HolisticPowerDomain as HolisticPwrDomain, PowerConstraint,
    PowerDevice, PowerDomain as PwrDomain, PowerDomainStats,
    PowerDomainType, PowerGovernor as PwrDomainGovernor,
    PowerState as PwrDomainState,
};
pub use rcu_tree::{
    GpState, GracePeriodInfo as RcuTreeGpInfo,
    HolisticRcuTree, RcuCpuData, RcuNodeState,
    RcuTreeFlavor, RcuTreeNode, RcuTreeStats,
};
pub use wq_scheduler::{
    HolisticWqScheduler, WqSchedulerStats, WqType as WqSchedType,
    WqWorker, WorkFlags, WorkItem as WqSchedWorkItem,
    WorkItemState as WqSchedItemState, WorkPriority as WqSchedPriority,
    Workqueue as WqSchedQueue,
};

// Round 14 re-exports
pub use balloon_driver::{
    BalloonDriverStats, BalloonInstance, BalloonPageType, BalloonState,
    HolisticBalloonDriver, InflationSource,
};
pub use cache_partition::{
    CacheLevel as CatCacheLevel, CacheMonitorData, CachePartitionStats,
    CdpConfig, ClosEntry, HolisticCachePartition, PartitionType,
};
    CState, CStateLatency, CpuIdleState, CpuIdleV2Stats,
    HolisticCpuIdleV2, IdleGovernor,
};
pub use dma_fence::{
    DmaFence as DmaFenceEntry, DmaFenceStats, FenceState, FenceType,
    HolisticDmaFence, SyncFile, TimelineFence,
};
pub use freq_scaling::{
    EnergyPref, FreqDomain as DvfsDomain, FreqGovernor as DvfsGovernor,
    FreqScalingStats, HolisticFreqScaling, ScalingState,
};
    HolisticIommuV2, IoMapping, IommuDomain as IommuV2Domain,
    IommuDomainType as IommuV2DomainType, IommuFault, IommuFaultType,
    IommuType, IommuV2Stats, IoptLevel,
};
pub use msi_controller::{
    HolisticMsiController, MsiControllerStats, MsiDeliveryMode,
    MsiDevice, MsiIrqDomain, MsiType, MsiVector,
};
pub use page_table::{
    AddressSpace, HolisticPageTable, PageFlags, PageTableStats,
    PtEntry, PtLevel, TlbFlushType,
};
pub use pci_enum::{
    Bdf, HolisticPciEnum, PciBar, PciBus, PciCapability, PciClass,
    PciDevice, PciEnumStats, PciHeaderType,
};
pub use slab_alloc::{
    CacheFlags, HolisticSlabAlloc, Slab, SlabAllocStats,
    SlabCache, SlabState,
};
    HolisticTimerWheelV2, TimerEntry as TwV2TimerEntry,
    TimerState as TwV2TimerState, TimerType as TwV2TimerType,
    TimerWheelV2Stats, WheelLevel,
};

// Round 15 re-exports
pub use acpi_mgr::{
    AcpiMgrStats, AcpiPowerState, AcpiTableHeader, AcpiTableType,
    HolisticAcpiMgr, MadtEntry, MadtEntryType, SratMemAffinity,
};
pub use buddy_alloc::{
    BuddyAllocStats, BuddyState, BuddyZone, FreeBlock,
    HolisticBuddyAlloc, MAX_ORDER,
};
pub use cfs_sched::{
    CfsEntity, CfsRunQueue, CfsSchedStats, CfsState,
    HolisticCfsSched,
};
pub use ftrace_mgr::{
    FtraceMgrStats, HolisticFtraceMgr, TraceBuffer, TraceEvent,
    TraceEventType, TraceFilter,
};
    HolisticHugePageV2, HugePagePool as HugePagePoolV2,
    HugePageSize as HugePageSizeV2, HugePageV2Stats, ThpCollapseEvent, ThpPolicy,
};
pub use irq_affinity::{
    CpuIrqLoad as IrqAffinityCpuLoad, HolisticIrqAffinity,
    IrqAffinityStats, IrqBalanceMode, IrqDesc as IrqAffinityDesc,
    IrqSourceType,
};
pub use kprobe_mgr::{
    HolisticKprobeMgr, KprobeEntry, KprobeMgrStats,
    ProbeHit, ProbeState, ProbeType,
};
    HolisticOomKillerV2, MemPressure, OomCandidate as OomV2Candidate,
    OomKillEvent, OomKillerV2Stats, OomPolicy as OomV2Policy,
};
    HolisticPerfEventsV2, HwEventType, PerfCounter as PerfV2Counter,
    PerfEventGroup, PerfEventsV2Stats, PerfSampleV2, SwEventType,
};
pub use rcu_sync::{
    GpState as RcuGpState, GracePeriod as RcuGracePeriod,
    HolisticRcuSync, RcuCallback as RcuSyncCallback, RcuCpuData,
    RcuFlavor as RcuSyncFlavor, RcuSyncStats,
};
pub use vma_mgr::{
    HolisticVmaMgr, ProcessMm, Vma as VmaEntry, VmaFlags,
    VmaMgrStats, VmaType as VmaMgrType,
};

// Round 16 re-exports
pub use cpufreq_gov::{
    CpuFreqGovStats, CpuFreqState as CpuFreqGovState,
    FreqTransition, GovernorType as CpuFreqGovType,
    HolisticCpuFreqGov as HolisticCpuFreqGovV2,
};
pub use devfreq_mgr::{
    DevFreqGovernor, DevFreqMgrStats, DevFreqProfile,
    DevPowerState, HolisticDevFreqMgr,
};
pub use dma_pool::{
    DmaBuffer as DmaPoolBuffer, DmaBufferState as DmaPoolBufferState,
    DmaPool, DmaPoolStats, HolisticDmaPool,
};
pub use hwmon_mgr::{
    HolisticHwmonMgr, HwmonMgrStats, HwmonSensor,
    HwmonSensorType, SensorAlarm,
};
pub use iommu_alloc::{
    HolisticIommuAlloc, IommuAllocDomain, IommuAllocStats,
    IommuMapType, IovaRegion as IommuIovaRegion,
};
pub use msi_mgr::{
    DeviceMsi, HolisticMsiMgr, MsiEntry as MsiAllocEntry,
    MsiMgrStats, MsiType as MsiAllocType,
};
pub use numa_mgr::{
    HolisticNumaMgr, NumaDistance as NumaMgrDistance,
    NumaMgrStats, NumaNode as NumaMgrNode,
    NumaPolicy as NumaMgrPolicy,
};
    HolisticPciEnumV2, PciBar as PciEnumBar, PciBarType,
    PciClassV2, PciDeviceV2, PciEnumV2Stats,
};
pub use power_mgr::{
    DevicePowerEntry, DevicePowerState, HolisticPowerMgr,
    PowerDomain as PwrMgrDomain, PowerMgrStats, SystemPowerState,
};
pub use thermal_mgr::{
    CoolingDevice as ThermalCoolingDev, HolisticThermalMgr,
    ThermalMgrStats, ThermalTrip as ThermalMgrTrip,
    ThermalTripType as ThermalMgrTripType,
    ThermalZone as ThermalMgrZone,
};
    HolisticWatchdogMgrV2, WatchdogV2, WatchdogV2MgrStats,
    WatchdogV2State, WatchdogV2Type,
};
// Round 17 re-exports
pub use blk_mq::{
    BlkIoOp, BlkMqStats, BlkRequest, HolisticBlkMq, HwQueue,
};
pub use cfs_bandwidth::{
    CfsBandwidthStats, CfsBwGroup, CfsBwState, HolisticCfsBandwidth,
};
pub use cgroup_mem::{
    CgroupMemState, CgroupMemStats, HolisticCgroupMem, MemLimitType,
};
pub use futex_mgr::{
    FutexBucket as FutexMgrBucket, FutexMgrStats,
    FutexOp as FutexMgrOp, FutexWaiter as FutexMgrWaiter,
    HolisticFutexMgr,
};
pub use io_sched::{
    HolisticIoSched, IoPrioClass as IoSchedPrioClass,
    IoRequest as IoSchedV2Request, IoSchedPolicy, IoSchedStats,
    SchedQueue,
};
pub use ksm_mgr::{
    HolisticKsmMgr, KsmMgrStats,
    KsmPage as KsmMgrPage, KsmPageState as KsmMgrPageState,
    StableNode,
};
pub use memcg_oom::{
    CgroupOomState, HolisticMemcgOom, MemcgOomStats,
    OomAction, OomEvent, OomVictim,
};
    CachedPageV2, HolisticPageCacheV2, PageCacheStateV2,
    PageCacheV2Stats, PageTreeNode,
};
    GpStateV2, GpTrackerV2, HolisticRcuTreeV2,
    RcuNodeLevel, RcuTreeNode as RcuTreeNodeV2, RcuTreeV2Stats,
};
    HolisticSwapMgrV2, SwapAreaType, SwapAreaV2,
    SwapEntryV2, SwapMgrV2Stats, SwapPriority,
};
pub use workqueue::{
    HolisticWorkqueue, WqPriority,
    WorkItem as WqWorkItemV2, WorkItemState as WqWorkItemStateV2,
    WorkerPool as WqWorkerPoolV2,
    WorkqueueStats as WorkqueueV2Stats,
};
// Round 18 re-exports
pub use balloon_drv::{
    BalloonAction, BalloonDrvStats, BalloonInstance as BalloonDrvInstance,
    BalloonPageRange, BalloonState as BalloonDrvState, HolisticBalloonDrv,
};
pub use cgroup_cpu::{
    CgroupCpuPolicy, CgroupCpuStats, CpuCgroup,
    HolisticCgroupCpu,
};
pub use dirty_writeback::{
    BdiWriteback, DirtyPageInfo, DirtyWritebackStats,
    HolisticDirtyWriteback, WritebackState as DirtyWritebackState,
};
pub use ebpf_verifier::{
    EbpfVerifierStats, HolisticEbpfVerifier, ProgramVerification,
    RegState, VerifierInsn, VerifyResult,
};
pub use huge_page_alloc::{
    HolisticHugePageAlloc, HugePageAllocPool, HugePageAllocSize,
    HugePageAllocStats, HugePageEntry,
};
pub use irq_thread::{
    HolisticIrqThread, IrqActionType, IrqThread,
    IrqThreadState, IrqThreadStats,
};
pub use kprobes::{
    HolisticKprobes, KernelProbe, KprobesStats,
    ProbeState as KprobeState, ProbeType as KprobeType,
};
pub use percpu_alloc::{
    HolisticPercpuAlloc, PercpuAlloc, PercpuAllocStats,
    PercpuChunk, PercpuChunkState,
};
    HolisticSlabAllocV2, SlabAllocV2Stats, SlabV2Cache,
    SlabV2Page, SlabV2State,
};
pub use softirq::{
    HolisticSoftirq, SoftirqEntry, SoftirqStats,
    SoftirqVec,
};
pub use vmalloc::{
    HolisticVmalloc, VmallocArea, VmallocAreaType,
    VmallocHole, VmallocStats,
};
// Round 19 re-exports
    BuddyAllocV2Stats, BuddyV2Block, BuddyV2Order,
    BuddyV2Zone, HolisticBuddyAllocV2,
};
    CfsBwV2Group, CfsBwV2State, CfsBwV2Stats,
    HolisticCfsBandwidthV2,
};
    CompactionV2Mode, CompactionV2Result, CompactionV2Stats,
    CompactionV2Zone, HolisticCompactionV2,
};
    DmaPoolV2, DmaPoolV2Stats, DmaV2AllocState,
    DmaV2Entry, HolisticDmaPoolV2,
};
pub use freelist::{
    FreePageEntry, Freelist, FreelistStats, FreelistType,
    HolisticFreelist,
};
pub use iommu::{
    HolisticIommu, IommuDevice, IommuDomain as IommuHolisticDomain,
    IommuDomainType as IommuHolisticDomainType,
    IommuMapping as IommuHolisticMapping, IommuStats,
};
pub use ksm::{
    HolisticKsm, KsmPage as KsmHolisticPage,
    KsmPageState as KsmHolisticPageState, KsmScanInfo, KsmStats,
};
pub use numa_balance::{
    HolisticNumaBalance, NumaBalanceStats, NumaBalanceTask,
    NumaFaultType, NumaNodeInfo,
};
    HolisticOomReaperV2, OomReaperV2Stats, OomV2ProcessInfo,
    OomV2Reason, OomV2Victim,
};
pub use page_reclaim::{
    HolisticPageReclaim, LruListType, PageReclaimStats,
    ReclaimScanType, ReclaimZone,
};
pub use zswap::{
    HolisticZswap, ZswapCompressor, ZswapEntry,
    ZswapPool, ZswapStats,
};
// Round 20 re-exports
pub use cgroup_io::{
    CgroupIoAccounting, CgroupIoDeviceId, CgroupIoDeviceLimit,
    CgroupIoInstance, CgroupIoPolicy, CgroupIoStats,
    HolisticCgroupIo, IoDirection as CgroupIoDirection,
};
    CmaRegion, CmaRegionState, HolisticHugePageV3, HugePageV3Size,
    HugePageV3Stats, MigrationCandidate, NumaHugePagePool,
};
pub use memcg_reclaim::{
    HolisticMemcgReclaim, MemcgLruScan, MemcgLruType,
    MemcgReclaimCtx, MemcgReclaimStats, MemcgReclaimUrgency,
    MemcgScanResult,
};
pub use mempolicy::{
    HolisticMempolicy, MempolicyFlag, MempolicyInstance,
    MempolicyMode, MempolicyScope, MempolicyStats,
    NumaNodemask, WeightedInterleave,
};
    HolisticPercpuAllocV2, PercpuAllocV2Stats, PercpuV2Chunk,
    PercpuV2ChunkState, PercpuV2Group, PercpuV2Strategy,
};
    HolisticRcuTreeV3, RcuTreeV3Stats, RcuV3Callback,
    RcuV3CpuData, RcuV3GpState, RcuV3Node, RcuV3NodeRole,
};
    HolisticSlabAllocV3, MagazineState, SlabV3Cache,
    SlabV3Depot, SlabV3Magazine, SlabV3SizeClass, SlabV3Stats,
};
    HolisticSwapMgrV3, SwapMgrV3Stats, SwapV3Area,
    SwapV3Cluster, SwapV3Compressor, SwapV3DeviceType,
    SwapV3SlotState, ZswapEntry as ZswapV3Entry,
};
    HolisticTlbMgrV2, TlbMgrV2Stats, TlbV2CpuData,
    TlbV2CpuState, TlbV2EntryType, TlbV2Scope,
    TlbV2ShootdownBatch,
};
    HolisticWorkqueueV2, WorkqueueV2Stats, WqV2Flag,
    WqV2Instance, WqV2Priority, WqV2Type,
    WqV2WorkItem, WqV2WorkState, WqV2WorkerPool,
};
    HolisticWritebackCtrlV2, WbV2Bandwidth, WbV2BdiEntry,
    WbV2CgroupCtx, WbV2State, WbV2ThrottleZone,
    WritebackCtrlV2Stats,
};
// Round 21 re-exports
pub use bio_layer::{
    BioDeviceQueue, BioFlag, BioLayerStats,
    BioOp, BioRequest, BioState, HolisticBioLayer,
};
pub use block_dev::{
    BlockDevEntry, BlockDevGeometry, BlockDevPartition,
    BlockDevScheduler, BlockDevState, BlockDevStats,
    BlockDevType, HolisticBlockDev,
};
pub use btrfs_cow::{
    BtrfsCowExtent, BtrfsCowExtentState, BtrfsCowExtentType,
    BtrfsCowSpaceInfo, BtrfsCowStats, BtrfsSnapshot,
    HolisticBtrfsCow,
};
pub use dentry_cache::{
    DentryCacheEntry, DentryCacheStats, DentryState,
    DentryType, HolisticDentryCache,
};
pub use ext4_journal::{
    Ext4JournalStats, HolisticExt4Journal, JournalMode,
    JournalSpace, JournalTransaction, JournalTxState,
};
pub use f2fs_gc::{
    F2fsGcMode, F2fsGcRound, F2fsGcStats,
    F2fsSegment, F2fsSegmentState, F2fsSegmentType,
    F2fsVictimPolicy, HolisticF2fsGc,
};
pub use inode_cache::{
    HolisticInodeCache, InodeCacheEntry,
    InodeCacheState as InodeCacheV2State, InodeCacheStats,
    InodeCacheType, InodeSuperBlockPartition,
};
pub use nfs_client::{
    HolisticNfsClient, NfsClientState, NfsClientStats,
    NfsDelegation, NfsDelegationType, NfsMountInstance,
    NfsVersion,
};
    FolioOrder, GenerationStats, HolisticPageCacheV3,
    PageCacheV3Folio, PageCacheV3Stats,
    PageGeneration as PageCacheV3Generation,
};
pub use tmpfs_mgr::{
    HolisticTmpfsMgr, TmpfsHugePolicy, TmpfsMgrStats,
    TmpfsMountInstance, TmpfsMountState,
};
pub use xfs_log::{
    HolisticXfsLog, XfsLogItem, XfsLogItemState,
    XfsLogItemType, XfsLogReservation, XfsLogStats,
};
// Round 22 re-exports
pub use arp_cache::{
    ArpCacheStats, ArpEntry, ArpOpType, HardwareType,
    HolisticArpCache, MacAddress, NudState,
};
pub use dns_cache::{
    DnsCacheEntry, DnsCacheStats, DnsCacheState,
    DnsRcode, DnsRecordType, DnsServerState,
    HolisticDnsCache,
};
pub use ip_routing::{
    HolisticIpRouting, IpRoutingStats, PolicyRule as IpPolicyRule,
    RouteEntry, RouteNextHop, RouteProto, RouteScope, RouteType,
};
pub use net_device::{
    HolisticNetDevice, NetDevQueue, NetDevice, NetDevState,
    NetDevType, NetDeviceStats, OffloadFeature,
};
pub use net_ns::{
    HolisticNetNs, NetNamespace, NetNsCap, NetNsState,
    NetNsStats, VethPair, VethState,
};
pub use netfilter::{
    ConntrackEntry, ConntrackState, HolisticNetfilter,
    NatType, NetfilterStats, NfChain, NfHook, NfMatch,
    NfMatchType, NfRule, NfTableType, NfVerdict,
};
pub use qdisc::{
    FqCodelFlow, HolisticQdisc, HtbClass, QdiscState,
    QdiscStats, QdiscType, TcClassState, TokenBucket,
};
pub use socket_mgr::{
    HolisticSocketMgr, ManagedSocket, SocketBuffer,
    SocketDomain, SocketMgrState, SocketMgrStats,
    SocketShutMode, SocketType as HolisticSocketType,
};
pub use tcp_stack::{
    HolisticTcpStack, TcpCongestionAlgo, TcpConnection,
    TcpCwndState, TcpStackStats, TcpState, TcpTimerKind,
};
pub use udp_mgr::{
    HolisticUdpMgr, UdpChecksumMode, UdpDatagram,
    UdpMgrStats, UdpMulticastMode, UdpSocket, UdpSocketState,
};
pub use xdp_mgr::{
    HolisticXdpMgr, XdpAction, XdpAttachMode, XdpMap,
    XdpMapType, XdpMgrStats, XdpProgState, XdpProgram,
};
// Round 23 re-exports
pub use vfs_holistic::{
    HolisticVfs, HolisticVfsStats, PathWalkState,
    VfsFsType, VfsOpRecord, VfsOpType,
};
pub use mount_holistic::{
    HolisticMount, HolisticMountStats, MountFlag,
    MountPoint, MountPropagation, MountType,
};
pub use dentry_holistic::{
    DentryCacheEntry as DentryV2CacheEntry, DentryFlag,
    DentryLru, DentryState as DentryV2State,
    HolisticDentry, HolisticDentryStats,
};
pub use inode_holistic::{
    HolisticInode, HolisticInodeMgr, HolisticInodeStats,
    InodeAllocator, InodeState, InodeType,
};
pub use superblock_holistic::{
    HolisticSbStats, HolisticSuperblock, HolisticSuperblockMgr,
    SbFeature, SuperblockState,
};
pub use file_lock_holistic::{
    DeadlockDetector, FileLockRequest, FileLockState,
    FileLockType, HolisticFileLock, HolisticFileLockStats,
};
pub use bio_holistic::{
    BioOp as BioV2Op, BioRequest as BioV2Request,
    BioState as BioV2State, HolisticBio,
    HolisticBioStats,
};
pub use blkdev_holistic::{
    BlkPartition, BlkdevInstance, BlkdevState,
    BlkdevType, HolisticBlkdev, HolisticBlkdevStats,
};
pub use ioscheduler_holistic::{
    BfqBudget, HolisticIoScheduler as HolisticIoSchedV2,
    HolisticIoSchedStats, IoReqType, IoSchedQueue,
    IoSchedType, IoPrioClass as IoPrioV2Class,
};
pub use raid_holistic::{
    HolisticRaid, HolisticRaidStats, RaidArray,
    RaidDisk, RaidDiskState, RaidLevel, RaidState,
};
pub use dm_holistic::{
    DmDevice, DmDevState, DmTarget, DmTargetType,
    DmThinPoolStatus, HolisticDm, HolisticDmStats,
};
pub use acl_holistic::{
    AclEntryType, AclPerm, HolisticAcl, HolisticAclEntry, HolisticAclStats, InodeAcl,
};
pub use chmod_holistic::{
    ChmodChangeRecord, ChmodRiskLevel, HolisticChmod, HolisticChmodStats,
};
pub use chown_holistic::{
    ChownChangeRecord, ChownChangeType, HolisticChown, HolisticChownStats, PrivilegeDirection,
};
pub use extent_holistic::{
    ExtentRecord, ExtentState, FragmentationAnalysis, HolisticExtent, HolisticExtentStats,
};
pub use journal_holistic::{
    HolisticJournal, HolisticJournalStats, JournalOp, JournalState,
    JournalTransaction as JournalTxn,
};
pub use pagecache_holistic::{
    FileCacheProfile, HolisticPageCache as HolisticPageCacheV4, HolisticPageCacheStats,
    PageCacheList, PageCacheOp,
};
pub use quota_holistic::{
    HolisticQuota, HolisticQuotaEntry, HolisticQuotaStats, HolisticQuotaType, QuotaState,
};
pub use readahead_holistic::{
    HolisticRaWindow, HolisticReadahead, HolisticReadaheadStats,
    ReadaheadPattern, ReadaheadState as HolisticRaState,
};
pub use stat_holistic::{
    HolisticStat, HolisticStatCall, HolisticStatFileType, HolisticStatStats, StatCallPattern,
};
pub use writeback_holistic::{
    DeviceWriteback, HolisticWbState, HolisticWriteback, HolisticWritebackStats,
    WritebackReason as WritebackReasonV2,
};
pub use xattr_holistic::{
    HolisticXattr, HolisticXattrNs, HolisticXattrStats, XattrUsagePattern,
};
// Re-exports from Round 25 — Security holistic analysis
pub use audit_holistic::{
    AuditHolisticFinding, AuditHolisticMetric, AuditHolisticStats, HolisticAudit,
};
pub use capability_holistic::{
    CapHolisticFinding, CapHolisticMetric, CapHolisticStats, HolisticCapability,
};
pub use credential_holistic::{
    CredHolisticFinding, CredHolisticMetric, CredHolisticStats, HolisticCredential,
};
pub use crypto_holistic::{
    CryptoHolisticFinding, CryptoHolisticMetric, CryptoHolisticStats, HolisticCrypto,
};
pub use integrity_holistic::{
    HolisticIntegrity, IntegrityHolisticFinding, IntegrityHolisticMetric, IntegrityHolisticStats,
};
pub use keyring_holistic::{
    HolisticKeyring, KeyringHolisticFinding, KeyringHolisticMetric, KeyringHolisticStats,
};
pub use lsm_holistic::{HolisticLsm, LsmFinding, LsmHolisticMetric, LsmHolisticStats};
pub use mac_holistic::{
    HolisticMac, MacHolisticFinding, MacHolisticMetric, MacHolisticStats,
};
pub use namespace_holistic::{
    HolisticNamespace, NsHolisticFinding, NsHolisticMetric, NsHolisticStats, NsHolisticType,
};
pub use sandboxing_holistic::{
    HolisticSandboxing, SandboxHolisticFinding, SandboxHolisticMetric, SandboxHolisticStats,
};
pub use seccomp_holistic::{
    HolisticSeccomp, SeccompFinding, SeccompHolisticMetric, SeccompHolisticStats,
};

// Round 26 re-exports — IPC/signal holistic analysis
pub use eventfd_holistic::{EventfdHolisticRecord, EventfdHolisticStats, EventfdPattern, HolisticEventfd};
pub use futex_holistic::{FutexHolisticRecord, FutexHolisticStats, FutexQueueHealth, HolisticFutex};
pub use ipc_holistic::{HolisticIpc, IpcHolisticRecord, IpcHolisticStats, IpcMechanism};
pub use mqueue_holistic::{HolisticMqueue, MqueueHolisticRecord, MqueueHolisticStats, MqueueLatencyBand};
pub use msgqueue_holistic::{HolisticMsgqueue, MsgqueueDepthState, MsgqueueHolisticRecord, MsgqueueHolisticStats};
pub use pipe_holistic::{HolisticPipe, PipeHealth, PipeHolisticRecord, PipeHolisticStats};
pub use semaphore_holistic::{HolisticSemaphore, SemContentionLevel, SemHolisticRecord, SemHolisticStats};
pub use shm_holistic::{HolisticShm, ShmHolisticRecord, ShmHolisticStats, ShmUtilization};
pub use sigaction_holistic::{HolisticSigaction, SigactionHolisticRecord, SigactionHolisticStats, SigactionPattern};
pub use signal_holistic::{HolisticSignal, SignalHolisticRecord, SignalHolisticStats, SignalPattern};
pub use timerfd_holistic::{HolisticTimerfd, TimerPrecision, TimerfdHolisticRecord, TimerfdHolisticStats};

// Round 27 re-exports — Networking/socket holistic analysis
pub use bandwidth_holistic::{BandwidthGrade, BandwidthHolisticRecord, BandwidthHolisticStats, HolisticBandwidth};
pub use congestion_holistic::{CongestionHolisticRecord, CongestionHolisticStats, CongestionPattern, HolisticCongestion};
pub use connection_holistic::{ConnHealth, ConnectionHolisticRecord, ConnectionHolisticStats, HolisticConnection};
pub use epoll_holistic::{EpollHolisticRecord, EpollHolisticStats, EpollScalability, HolisticEpoll};
pub use latency_holistic::{HolisticLatency, LatencyBucket, LatencyHolisticRecord, LatencyHolisticStats};
pub use netstack_holistic::{HolisticNetstack, NetstackHolisticRecord, NetstackHolisticStats, NetstackLayer};
pub use routing_holistic::{HolisticRouting, RoutingEfficiency, RoutingHolisticRecord, RoutingHolisticStats};
pub use socket_holistic::{HolisticSocket, SocketHolisticRecord, SocketHolisticStats, SocketLifecycle};
pub use tcp_holistic::{HolisticTcp, TcpHealth, TcpHolisticRecord, TcpHolisticStats};
pub use udp_holistic::{HolisticUdp, UdpHolisticRecord, UdpHolisticStats, UdpQuality};

// Round 28 re-exports
pub use flock_holistic::{HolisticFlockHealth, HolisticFlockManager, HolisticFlockMetric, HolisticFlockStats};
pub use page_cache_holistic::{HolisticPageCacheAnalysisStats, HolisticPageCacheAnalyzer, HolisticPageCacheHealth, HolisticPageCacheMetric};

// Re-exports from Round 29 — Process/thread holistic
pub use clone_holistic::{HolisticCloneManager, HolisticClonePattern, HolisticCloneRecord, HolisticCloneStats};
pub use exec_holistic::{HolisticExecManager, HolisticExecPattern, HolisticExecRecord, HolisticExecStats};
pub use exit_holistic::{HolisticExitManager, HolisticExitPattern, HolisticExitRecord, HolisticExitStats};
pub use fork_holistic::{HolisticForkEntry, HolisticForkManager, HolisticForkPattern, HolisticForkStats};
pub use nice_holistic::{HolisticNiceEntry, HolisticNiceFairness, HolisticNiceManager, HolisticNiceStats};
pub use pgid_holistic::{HolisticPgidEntry, HolisticPgidHealth, HolisticPgidManager, HolisticPgidStats};
pub use pid_holistic::{HolisticPidEntry, HolisticPidHealth, HolisticPidManager, HolisticPidStats};
pub use prctl_holistic::{HolisticPrctlEntry, HolisticPrctlManager, HolisticPrctlPosture, HolisticPrctlStats};
pub use session_holistic::{HolisticSessionEntry, HolisticSessionHealth, HolisticSessionManager, HolisticSessionStats};
pub use thread_holistic::{HolisticThreadEntry, HolisticThreadManager, HolisticThreadPattern, HolisticThreadStats};
pub use wait_holistic::{HolisticWaitEntry, HolisticWaitManager, HolisticWaitPattern, HolisticWaitStats};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_snapshot() {
        let snap = SystemSnapshot::new(4, 8 * 1024 * 1024 * 1024);
        assert_eq!(snap.cpu_cores, 4);
        assert!(snap.total_memory > 0);
    }

    #[test]
    fn test_optimization_goal() {
        let goal = OptimizationGoal::Throughput;
        assert_ne!(goal, OptimizationGoal::Latency);
    }

    #[test]
    fn test_policy_rule() {
        let rule = PolicyRule::new(
            PolicyCondition::CpuAbove(0.9),
            PolicyAction::ThrottleLowPriority,
            5,
        );
        assert_eq!(rule.priority, 5);
    }

    #[test]
    fn test_orchestrator() {
        let orch = Orchestrator::new();
        assert_eq!(orch.pending_actions(), 0);
    }

    #[test]
    fn test_resource_balancer() {
        let balancer = ResourceBalancer::new(4, 8 * 1024 * 1024 * 1024);
        let (cpu, mem) = balancer.available_resources();
        assert!(cpu > 0.0);
        assert!(mem > 0);
    }
}
pub mod anomaly;
pub mod cpu_topo;
pub mod hugepage;
// R30 — Memory Management
pub mod mmap_holistic;
pub mod shmem_holistic;
pub mod mprotect_holistic;
pub mod mremap_holistic;
pub mod msync_holistic;
pub mod munmap_holistic;
pub mod vma_holistic;
pub mod page_fault_holistic;
pub mod oom_holistic;
pub mod swap_holistic;
pub mod mlock_holistic;
// Year 5 Expansion — Holistic Future Prediction: The Master Prediction Engine
pub mod future;

// Year 5 Expansion — Holistic Consciousness: The Crown Module
pub mod conscious;

// Year 5 Expansion — Holistic Research: The Master Research Engine
pub mod research;

// Year 5 Expansion — Holistic Transcendence: The Superintelligent Kernel APEX
pub mod transcend;
