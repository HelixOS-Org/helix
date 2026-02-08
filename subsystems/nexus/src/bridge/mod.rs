//! # Intelligent Syscall Bridge — Year 4 SYMBIOSIS (Q1 2029)
//!
//! Revolutionary kernel-userland bridge that transforms syscalls from dumb
//! request-response into an intelligent, predictive, and cooperative channel.
//!
//! ## Key Innovations
//!
//! - **Syscall Prediction**: Anticipate what the app needs before it asks
//! - **Automatic Batching**: Merge similar syscalls for throughput gains
//! - **Context-Aware Optimization**: Adapt execution path to app type
//! - **Async Intelligent I/O**: Non-blocking syscalls with smart scheduling
//! - **Application Profiling**: Continuous learning from app behavior
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                     INTELLIGENT SYSCALL BRIDGE                       │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │                                                                      │
//! │   Userland ──▶ Interceptor ──▶ Predictor ──▶ Batcher ──▶ Executor  │
//! │                     │              │             │            │      │
//! │                     ▼              ▼             ▼            ▼      │
//! │                  Profile       Prefetch       Merge       Optimize  │
//! │                                                                      │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Submodules
//!
//! - `syscall`: Core intelligent syscall interceptor and router
//! - `predict`: Syscall sequence prediction engine
//! - `batch`: Automatic syscall batching and merging
//! - `async_io`: Async intelligent I/O with smart scheduling
//! - `profile`: Application profiling and behavior learning

#![allow(dead_code)]

extern crate alloc;

pub mod async_io;
pub mod batch;
pub mod cache;
pub mod coalesce;
pub mod compat;
pub mod context;
pub mod fallback;
pub mod history;
pub mod intent;
pub mod intercept;
pub mod metrics;
pub mod optimizer;
pub mod pattern;
pub mod pipeline;
pub mod predict;
pub mod prefetch;
pub mod profile;
pub mod queue;
pub mod routing;
pub mod security;
pub mod syscall;
pub mod throttle;
pub mod trace;
pub mod transform;
pub mod validate;

// Year 4 Expansion — Round 3 bridge modules
pub mod audit;
pub mod compress;
pub mod dedup;
pub mod emulate;
pub mod ratelimit;
pub mod replay;
pub mod sandbox;
pub mod telemetry;
pub mod version;

// Year 4 Expansion — Round 4 bridge modules
pub mod accounting;
pub mod admission;
pub mod circuit;
pub mod dependency;
pub mod escalation;
pub mod instrument;
pub mod marshal;
pub mod retry;
pub mod snapshot;

// Year 4 Expansion — Round 5 bridge modules
pub mod bulkhead;
pub mod canary;
pub mod correlation;
pub mod debounce;
pub mod fence;
pub mod gateway;
pub mod health;
pub mod isolation;
pub mod priority;

// Year 4 Expansion — Round 6 bridge modules
pub mod backoff;
pub mod capability;
pub mod checkpoint;
pub mod compress_engine;
pub mod dispatch;
pub mod envelope;
pub mod flow;
pub mod namespace;
pub mod quota;
pub mod sched_bridge;

// Year 4 Expansion — Round 7 bridge modules
pub mod affinity;
pub mod credential;
pub mod deferred;
pub mod error_recovery;
pub mod fd_table;
pub mod futex;
pub mod mem_proxy;
pub mod net_proxy;
pub mod signal_proxy;
pub mod vfs_proxy;

// Year 4 Expansion — Round 8 bridge modules
pub mod abi_translator;
pub mod async_syscall;
pub mod copy_engine;
pub mod event_bridge;
pub mod ipc_proxy;
pub mod seccomp;
pub mod syscall_table;
pub mod timer_bridge;
pub mod user_context;
pub mod wait_queue;

// Year 4 Expansion — Round 9 bridge modules
pub mod cap_mgr;
pub mod cred_mgr;
pub mod epoll_bridge;
pub mod io_uring_bridge;
pub mod mmap_mgr;
pub mod ns_proxy;
pub mod proc_lifecycle;
pub mod ptrace_bridge;
pub mod rlimit_bridge;
pub mod socket_bridge;
pub mod syscall_profiler;

// Year 4 Expansion — Round 10 bridge modules
pub mod block_bridge;
pub mod cgroup_bridge;
pub mod clock_bridge;
pub mod dma_fence;
pub mod irq_bridge;
pub mod module_bridge;
pub mod netlink_proxy;
pub mod power_bridge;
pub mod procfs_bridge;
pub mod sysfs_proxy;
pub mod wq_proxy;

// Year 4 Expansion — Round 11 bridge modules
pub mod aio_bridge;
pub mod kcov_bridge;
pub mod mount_bridge;
pub mod msg_bridge;
pub mod pipe_bridge;
pub mod poll_bridge;
pub mod sem_bridge;
pub mod shm_bridge;
pub mod swap_bridge;
pub mod tty_bridge;
pub mod xattr_bridge;

// Year 4 Expansion — Round 12 bridge modules
pub mod addr_space;
pub mod bpf_bridge;
pub mod dev_proxy;
pub mod hugetlb_bridge;
pub mod inotify_bridge;
pub mod kexec_bridge;
pub mod landlock_bridge;
pub mod perf_bridge;
pub mod sched_ext;
pub mod userfault_bridge;

// Year 4 Expansion — Round 13 bridge modules
pub mod acpi_bridge;
pub mod crypto_bridge;
pub mod debug_bridge;
pub mod eventfd_bridge;
pub mod flock_bridge;
pub mod keyring_bridge;
pub mod memfd_bridge;
pub mod pidfd_bridge;
pub mod rseq_bridge;
pub mod signalfd_bridge;
pub mod timerfd_bridge;

// Year 4 Expansion — Round 14 bridge modules
pub mod fanotify_bridge;
pub mod kcmp_bridge;
pub mod mnt_ns_bridge;
pub mod netlink_bridge;
pub mod perf_hw_bridge;
pub mod quota_bridge;
pub mod splice_bridge;
pub mod sysctl_bridge;

// Year 4 Expansion — Round 15 bridge modules
pub mod binfmt_bridge;
pub mod cap_bridge;
pub mod dnotify_bridge;
pub mod posix_timer_bridge;
pub mod userfaultfd_bridge;

// Round 16
pub mod cred_bridge;
pub mod io_prio_bridge;
pub mod mqueue_bridge;
pub mod pkey_bridge;

// Round 17
pub mod acct_bridge;
pub mod audit_bridge;
pub mod clone_bridge;
pub mod namespace_bridge;
pub mod syslog_bridge;
// Round 18
pub mod keyctl_bridge;
pub mod membarrier_bridge;
pub mod seccomp_bridge;
pub mod tls_bridge;
// Round 19
pub mod ioprio_bridge;
pub mod mq_bridge;
pub mod prctl_bridge;
// Round 20
// Round 21
pub mod sendfile_bridge;
// Round 22
pub mod cpu_affinity_bridge;
pub mod cpuset_bridge;
pub mod numa_bridge;
pub mod taskstats_bridge;
// Round 23
pub mod copy_file_range_bridge;
pub mod fallocate_bridge;
pub mod kqueue_bridge;
pub mod readahead_bridge;
pub mod select_bridge;
// Round 24
pub mod bio_bridge;
pub mod blkdev_bridge;
pub mod dentry_bridge;
pub mod devmapper_bridge;
pub mod inode_bridge;
pub mod iosched_bridge;
pub mod raid_bridge;
pub mod superblock_bridge;
pub mod vfs_bridge;
// Round 25 — Security/crypto bridge modules
pub mod apparmor_bridge;
pub mod capability_bridge;
pub mod integrity_bridge;
pub mod lsm_bridge;
pub mod random_bridge;
pub mod selinux_bridge;
// Round 26 — IPC/signals bridge modules
pub mod msgqueue_bridge;
pub mod semaphore_bridge;
pub mod sigaction_bridge;
pub mod signal_bridge;
pub mod sigprocmask_bridge;
pub mod sigqueue_bridge;

// Round 27 — Networking/socket bridge modules
pub mod accept_bridge;
pub mod bind_bridge;
pub mod listen_bridge;
pub mod tcp_bridge;
pub mod udp_bridge;
pub mod unix_bridge;

// Round 28 — Filesystem/VFS bridge modules
pub mod fsync_bridge;
pub mod readdir_bridge;
pub mod statfs_bridge;
pub mod truncate_bridge;

// Round 29 — Process/thread management bridge modules
pub mod affinity_bridge;
pub mod exec_bridge;
pub mod exit_bridge;
pub mod fork_bridge;
pub mod priority_bridge;
pub mod pthread_bridge;
pub mod tid_bridge;
pub mod wait_bridge;

// Round 30 — Memory management bridge modules
pub mod brk_bridge;
pub mod hugepage_bridge;
pub mod madvise_bridge;
pub mod mlock_bridge;
pub mod mmap_bridge;
pub mod mprotect_bridge;
pub mod mremap_bridge;
pub mod msync_bridge;
pub mod munmap_bridge;
pub mod shmem_bridge;
pub mod vma_bridge;

// Re-exports from Round 4 modules
// Re-exports from Round 8 modules
pub use abi_translator::{
    AbiArgType, AbiVersion as AbiTranslatorVersion, BridgeAbiTranslator, BridgeAbiTranslatorStats,
    FieldDescriptor, RegisterMapping, StructTranslation, SyscallTranslation,
};
pub use accounting::{
    AccountingResource, BridgeAccountingEngine, BridgeAccountingStats, CostModel, CostUnit,
    ProcessAccount, ResourceCounter, SyscallCost,
};
// Re-exports from Round 17 bridge modules
pub use acct_bridge::{AcctBridgeStats, AcctEntry, AcctRecordType, BridgeAcct};
// Re-exports from Round 13 bridge modules
pub use acpi_bridge::{
    AcpiBridgeStats, AcpiDevice, AcpiDeviceState, AcpiEvent, AcpiEventType, AcpiTable,
    AcpiTableType, BridgeAcpi, GpeInfo, SleepState as AcpiSleepState,
};
// Re-exports from Round 12 modules
pub use addr_space::{
    AddrProt, AddrSpaceStats, AddrTranslation, AslrPolicy, BridgeAddrSpace, ProcessAddrSpace,
    RegionType, VmaRegion,
};
pub use admission::{
    AdmissionConfig, AdmissionDecision, AdmissionPriority, BridgeAdmissionController,
    BridgeAdmissionStats, LoadLevel, ProcessCredit, QueuedRequest,
};
// Re-exports from Round 7 modules
pub use affinity::{
    AffinityChangeReason, AffinityClass, BridgeAffinityStats, BridgeAffinityTracker,
    ProcessAffinityProfile, SyscallCpuAffinity,
};
// Re-exports from Round 11 modules
pub use aio_bridge::{
    AioBridgeStats, AioContext, AioEvent, AioIocb, AioOp, AioState, BridgeAioBridge,
};
// Re-exports from Round 15 bridge modules
    AioContext, AioCtxState, AioV2BridgeStats, AioV2CompletionMode, AioV2Op, BridgeAioV2, Iocb,
};
// Re-exports from Round 18 bridge modules
    AioV3BridgeStats, AioV3Completion, AioV3OpType, AioV3Ring, AioV3Submission, BridgeAioV3,
};
// Re-exports from Round 21 bridge modules
    AioV4BridgeStats, AioV4Cqe, AioV4CqeState, AioV4FixedBuf, AioV4Op, AioV4Ring, AioV4Sqe,
    BridgeAioV4,
};
// Round 23 re-exports
    AioV5BridgeStats, AioV5Cb, AioV5Context, AioV5OpType, AioV5Priority, AioV5Ring, AioV5State,
    BridgeAioV5,
};
pub use async_io::{AsyncCompletion, AsyncIoEngine, AsyncIoRequest, AsyncPriority, AsyncStatus};
pub use async_syscall::{
    AsyncContext, AsyncSyscallFlag, AsyncSyscallState, BridgeAsyncSyscall, BridgeAsyncSyscallStats,
    CqEntry, InFlightSyscall, Ring as AsyncRing, SqEntry,
};
// Re-exports from Round 3 modules
pub use audit::{
    AlertCondition, AuditEvent, AuditEventType, AuditManagerStats, AuditRule, AuditSeverity,
    BridgeAuditManager,
};
pub use audit_bridge::{
    AuditBridgeStats, AuditField, AuditMsgType, AuditRecord, AuditRule as AuditBridgeRule,
    BridgeAudit,
};
// Re-exports from Round 6 modules
pub use backoff::{
    BackoffConfig, BackoffState, BackoffStrategy, BackoffTracker, BridgeBackoffManager,
    BridgeBackoffStats, ErrorClass, ProcessBackoff,
};
pub use batch::{BatchDecision, BatchEntry, BatchGroup, BatchOptimizer, BatchStats};
pub use binfmt_bridge::{
    BinaryImage, BinfmtBridgeStats, BinfmtLoadState, BinfmtMiscRule, BinfmtType, BridgeBinfmt,
    ElfInfo, ProgHeader,
};
// Re-exports from Round 19 bridge modules
    BinfmtV2BridgeStats, BinfmtV2Entry, BinfmtV2Type, BridgeBinfmtV2, LoadInfoV2,
};
pub use bio_bridge::{BioBridgeOp, BioBridgeRecord, BioBridgeResult, BioBridgeStats, BridgeBio};
pub use blkdev_bridge::{
    BlkdevBridgeOp, BlkdevBridgeRecord, BlkdevBridgeResult, BlkdevBridgeStats, BridgeBlkdev,
};
// Re-exports from Round 10 modules
pub use block_bridge::{
    BioOp, BioState, BlockBridgeStats, BlockDevice, BlockReq, BridgeBlockBridge,
    IoSched as BridgeIoSched, Partition,
};
pub use bpf_bridge::{
    BpfBridgeStats, BpfInsn, BpfMap, BpfMapType, BpfProgType, BpfProgram, BridgeBpf, VerifyConfig,
    VerifyResult,
};
    BpfV2BridgeStats, BpfV2Map, BpfV2MapType, BpfV2ProgType, BpfV2Program, BridgeBpfV2,
};
// Re-exports from Round 5 modules
pub use bulkhead::{
    BridgeBulkheadManager, BridgeBulkheadStats, Bulkhead, BulkheadClass, BulkheadState,
    OverflowPolicy,
};
pub use cache::{CacheKey, Cacheability, CachedResult, SyscallCache, SyscallCacheConfig};
pub use canary::{
    BridgeCanaryManager, BridgeCanaryStats, CanaryDeployment, CanaryMetric, CanaryState,
    ComparisonResult, MetricSamples as CanaryMetricSamples,
};
pub use cap_bridge::{BridgeCap, CapBitmask, CapBridgeStats, CapSetType, Capability, ProcessCaps};
// Re-exports from Round 9 modules
pub use cap_mgr::{
    BridgeCapMgr, BridgeCapType, CapAuditAction, CapAuditEntry, CapScope, CapSet, CapState,
    CapToken,
};
pub use capability::{
    BridgeCapabilityManager, BridgeCapabilityStats, CapabilityState, CapabilityToken,
    CapabilityType, ProcessCapabilityTable, ProtectedResource,
};
pub use cgroup_bridge::{
    BridgeCgroupBridge, CgroupBridgeStats, CgroupController, CgroupEvent, CgroupMigration,
    CgroupNode, CgroupOp, CgroupVersion, ControllerLimit,
};
// Re-exports from Round 14 bridge modules
    BridgeCgroupV2, CgroupV2BridgeStats, CgroupV2Controller, CgroupV2Event, CgroupV2EventType,
    CgroupV2Node, CpuWeightConfig, DistributionModel, IoConfig, MemoryLimits, MemoryProtection,
};
// Re-exports from Round 22 bridge modules
    BridgeCgroupV4, CgroupV4BridgeStats, CgroupV4Controller, CgroupV4ControllerState,
    CgroupV4DelegateScope, CgroupV4Group, CgroupV4Pressure, CgroupV4PressureKind,
};
pub use checkpoint::{
    BridgeCheckpointManager, BridgeCheckpointStats, Checkpoint, CheckpointState, CheckpointTrigger,
    RestorePlan, StateComponent, StateFragment,
};
pub use circuit::{
    BridgeCircuitBreakerManager, BridgeFailureType, CircuitBreaker, CircuitBreakerConfig,
    CircuitBreakerStats, CircuitState, FailureEvent, FailureWindow,
};
pub use clock_bridge::{
    BridgeClockBridge, ClockBridgeStats, ClockEventDevice, ClockEventMode, ClockFlag, ClockRating,
    ClockSource, TimekeepingState,
};
pub use clone_bridge::{BridgeClone, CloneBridgeStats, CloneFlag as CloneBridgeFlag, CloneRequest};
// Re-exports from expanded modules (Round 2)
pub use coalesce::{
    CoalesceCategory, CoalesceEngine, CoalesceState, CoalesceStats, CoalescedBatch, PendingSyscall,
    WindowConfig,
};
pub use compat::{AbiVersion, ArgRewriter, CompatConfig, CompatLayer, CompatProfile, MappingTable};
pub use compress::{
    BridgeCompressionManager, CompressedBlock, CompressionLevel, CompressionMethod,
    CompressionStats, DeltaCompressor, RleCompressor, ZeroPageDedup,
};
pub use compress_engine::{
    BridgeCompressionEngine, BridgeCompressionStats, CompressionAlgorithm, CompressionDictionary,
    CompressionLevel as CompressEngineLevel, CompressionResult, DeltaEncoder, DeltaStats,
    DictionaryEntry, RleEncoder, RleStats, SyscallCompressionProfile,
};
pub use context::{
    Capability, CapabilitySet, ContextManager, LimitCheck, NamespaceContext, NamespaceType,
    ProcessContext, RLimit, ResourceLimits, SchedClass, SecurityLabel, ThreadContext,
};
pub use copy_engine::{
    BridgeCopyEngine, BridgeCopyEngineStats, CopyCompletion, CopyDirection, CopyMethod,
    CopyRequest, IoVec, PinnedPages,
};
pub use copy_file_range_bridge::{
    BridgeCopyFileRange, CopyRangeBridgeStats, CopyRangeMode, CopyRangeOp, CopyRangeState,
};
pub use correlation::{
    BridgeCorrelationEngine, BridgeCorrelationStats, CoOccurrenceMatrix, CorrelationLink,
    CorrelationRule, CorrelationStrength, SyscallCorrelationType, SyscallEvent, TemporalWindow,
};
pub use cpu_affinity_bridge::{
    AffinityMask, AffinityMigrationType, AffinityScope, BridgeCpuAffinity, CpuAffinityBridgeStats,
    CpuTopologyNode, ProcessAffinity,
};
pub use cpuset_bridge::{
    BridgeCpuset, CpusetBridgeStats, CpusetDistribution, CpusetGroup, CpusetMask, CpusetMemPolicy,
    CpusetPartition,
};
// Re-exports from Round 16 bridge modules
pub use cred_bridge::{
    BridgeCred, CapSet as CredCapSet, CredBridgeStats, CredChangeEvent, CredType, ProcessCred,
};
pub use cred_mgr::{
    BridgeCredMgr, BridgeCredMgrStats, CredChangeEvent, CredChangeType, CredentialSet, ProcessCreds,
};
    BridgeCredV2, CapabilitySetV2, CredV2BridgeStats, CredV2Type, ProcessCredV2,
};
pub use credential::{
    AuthzDecision, BridgeCredentialProxy, BridgeCredentialStats, CredCacheEntry, CredentialSet,
    CredentialType, EscalationEvent, PrivilegeLevel,
};
pub use crypto_bridge::{
    AlgStats, BridgeCrypto, CryptoAlg, CryptoAlgType, CryptoBridgeStats, CryptoOp, CryptoPriority,
    CryptoRequest, CryptoResult,
};
pub use debounce::{
    BridgeDebounceManager, BridgeDebounceStats, DebounceEntry, DebounceResult, DebounceStrategy,
    ProcessDebounce,
};
pub use debug_bridge::{
    BpState, BreakpointType, BridgeDebug, DebugBreakpoint, DebugBridgeStats, DebugEvent,
    DebugFacility, DynDbgEntry, FtraceFunction, KprobeEntry,
};
pub use dedup::{
    BridgeDedupManager, CachedResult as DedupCachedResult, DedupSafety, DedupStats,
    RedundancyPattern, SyscallDedupPolicy, SyscallSignature,
};
pub use deferred::{
    BridgeDeferredEngine, BridgeDeferredStats, DeferralReason, DeferredCompletion, DeferredState,
    DeferredSyscall,
};
pub use dentry_bridge::{
    BridgeDentry, DentryBridgeOp, DentryBridgeRecord, DentryBridgeResult, DentryBridgeStats,
};
pub use dependency::{
    BridgeDependencyTracker, DependencyEdge, DependencyGraph, DependencyNode, DependencyStrength,
    DependencyTrackerStats, DependencyType,
};
pub use dev_proxy::{
    BridgeDevProxy, DevClass, DevDescriptor, DevNumber, DevPermission, DevPowerState,
    DevProxyStats, DevResource, HotplugAction, HotplugEvent, IoOp, IoRequest,
};
pub use devmapper_bridge::{
    BridgeDevMapper, DmBridgeOp, DmBridgeRecord, DmBridgeResult, DmBridgeStats,
};
pub use dispatch::{
    BridgeDispatchOptimizer, BridgeDispatchStats, DispatchDecision, DispatchPrediction,
    DispatchTable, HandlerEntry, HandlerState, HandlerType,
    SyscallPredictor as DispatchSyscallPredictor,
};
pub use dma_fence::{
    BridgeDmaFence, DmaFence as BridgeDmaFenceEntry, DmaFenceStats, FenceContext, FenceState,
    FenceType, FenceWaitReq, SyncFile,
};
pub use dnotify_bridge::{
    BridgeDnotify, DnotifyBridgeStats, DnotifyEvent, DnotifyMask, DnotifyWatch,
};
pub use emulate::{
    BridgeEmulationManager, EmulationAccuracy, EmulationContext, EmulationStats, EmulationTarget,
    ErrnoMapping, TranslationEntry, TranslationTable,
};
pub use envelope::{
    ArgDescriptor as EnvelopeArgDescriptor, ArgDirection, ArgType as EnvelopeArgType,
    BridgeEnvelopeManager, BridgeEnvelopeStats, CallerContext, EnvelopeState, EnvelopeVersion,
    SyscallEnvelope,
};
pub use epoll_bridge::{
    BridgeEpollBridge, BridgeEpollStats, EpollEntry, EpollEvents, EpollInstance, EpollTrigger,
    WakeupSource, WakeupSourceType,
};
    BridgeEpollV2, EpollV2BridgeStats, EpollV2Flags, EpollV2Instance, EpollV2Item, EpollV2Op,
};
// Re-exports from Round 20 bridge modules
    BridgeEpollV3, EpollV3BridgeStats, EpollV3Event, EpollV3Instance, EpollV3Interest,
    EpollV3Trigger,
};
    BridgeEpollV4, EpollV4BridgeStats, EpollV4Event, EpollV4Instance, EpollV4Interest, EpollV4Mode,
};
pub use error_recovery::{
    BridgeErrorRecovery, BridgeErrorRecoveryStats, ErrorPattern, RecoveryStrategy, SyscallError,
    SyscallErrorCategory,
};
pub use escalation::{
    BasePriority, BridgeEscalationManager, EscalationPolicy, EscalationReason, EscalationState,
    EscalationStats, TrackedSyscall,
};
pub use event_bridge::{
    BridgeEventBridge, BridgeEventBridgeStats, EventInstance, EventInterest,
    EventKind as BridgeEventKind, EventSourceType, ReadyEvent, TriggerMode,
};
pub use eventfd_bridge::{
    BridgeEventfd, EventfdBridgeStats, EventfdFlags, EventfdInstance, EventfdOp,
};
    BridgeEventfdV2, EventfdV2BridgeStats, EventfdV2Flag, EventfdV2Instance, EventfdV2State,
};
pub use fallback::{
    EmulationEntry, EmulationRegistry, ErrorCategory, FallbackAlertType, FallbackEngine,
    FallbackResult, FallbackStrategy, RetryConfig, SyscallFallbackChain,
};
pub use fallocate_bridge::{
    BridgeFallocate, FallocateBridgeStats, FallocateMode, FallocateOp, FallocateState,
    FileSpaceTracker,
};
pub use fanotify_bridge::{
    BridgeFanotify, FanEventMask, FanInitFlags, FanMark, FanMarkType, FanPermResponse,
    FanotifyBridgeStats, FanotifyEvent, FanotifyGroup,
};
    BridgeFanotifyV2, FanMarkType as FanV2MarkType, FanotifyV2BridgeStats, FanotifyV2Event,
    FanotifyV2Group, FanotifyV2Mark, FanotifyV2Mask,
};
    BridgeFanotifyV3, FanEventV3, FanGroupV3, FanResponseV3, FanotifyV3BridgeStats, FanotifyV3Mask,
};
pub use fd_table::{
    BridgeFdTableProxy, BridgeFdTableStats, FdEntry, FdFlags, FdType, ProcessFdTable,
};
pub use fence::{
    BridgeFenceManager, BridgeFenceStats, FenceChain, FencePoint, FencePool, FenceScope,
    FenceState, FenceType,
};
pub use flock_bridge::{
    BridgeFlock, FileLock, FlockBridgeStats, FlockType, InodeLockState, LockMechanism,
    LockOpResult, LockOpType, LockState,
};
    BridgeFlockV2, DeadlockEdge, FlockV2BridgeStats, FlockV2State, FlockV2Type, PosixLock,
};
    BridgeFlockV3, DeadlockDetector, FileLockV3, FlockV3BridgeStats, FlockV3Type, LockScopeV3,
};
    BridgeFlockV4, FlockV4BridgeRecord, FlockV4BridgeStats, FlockV4Op, FlockV4Result,
};
pub use flow::{
    AdmissionWindow, BridgeFlowController, BridgeFlowStats, CongestionSignal, CreditBucket,
    FlowPriority, FlowState, ProcessFlow,
};
pub use futex::{
    BridgeFutexProxy, BridgeFutexStats, FutexContention, FutexDeadlockHint, FutexEntry, FutexOp,
};
    BridgeFutexV2, FutexBucket, FutexOp, FutexV2Stats, FutexWaiter, PiChainEntry, WaiterPriority,
    WaitvEntry,
};
    BridgeFutexV3, FutexV3Instance, FutexV3Key, FutexV3Op, FutexV3Stats, FutexV3Waiter,
    FutexV3WaiterState,
};
pub use gateway::{
    ApiVersion as GatewayApiVersion, BridgeGatewayManager, BridgeGatewayStats, CallerProfile,
    FeatureFlag, GatewayRateLimiter, GatewayState,
};
pub use health::{
    BridgeComponent, BridgeHealthMonitor, BridgeHealthStats, ComponentHealth, ComponentStatus,
    HealingAction, HealingTrigger, Heartbeat,
};
pub use history::{HistoryManager, HistoryQuery, QueryResult, RecordRingBuffer, SyscallRecord};
pub use hugetlb_bridge::{
    BridgeHugetlb, HugePagePool, HugePageSize, HugetlbCgroupLimit, HugetlbStats, NumaHugeBinding,
    ProcessReservation, ReservationState,
};
pub use inode_bridge::{
    BridgeInode, InodeBridgeOp, InodeBridgeRecord, InodeBridgeResult, InodeBridgeStats,
};
pub use inotify_bridge::{
    BridgeInotify, CoalesceConfig as InotifyCoalesceConfig, InotifyBridgeStats, InotifyEvent,
    InotifyInstance, InotifyMask, WatchDescriptor,
};
    BridgeInotifyV2, InotifyV2BridgeStats, InotifyV2Event, InotifyV2Instance, InotifyV2Mask,
    InotifyV2Watch,
};
    BridgeInotifyV3, InotifyV3BridgeStats, InotifyV3Event, InotifyV3Instance, InotifyV3Mask,
    InotifyV3Watch,
};
pub use instrument::{
    BridgeInstrumentationEngine, EventFilter, FilterField, FilterOp, InstrumentationEvent,
    InstrumentationProbe, InstrumentationStats, PerfCounter, PerfCounterType, ProbeState,
    ProbeType,
};
pub use intent::{IntentAnalyzer, IntentConfidence, IntentPattern, IntentType};
pub use intercept::{
    FilterCondition, FilterProgram, InterceptAction, InterceptEngine, InterceptHook,
    InterceptPoint, InterceptVerdict, SyscallArgs,
};
pub use io_prio_bridge::{
    BridgeIoPrio, IoPrioBridgeStats, IoPrioEvent, IoPriority, IoSchedClass, ProcessIoPrio,
};
pub use io_uring_bridge::{
    BridgeCqe, BridgeIoUringBridge, BridgeIoUringStats, BridgeSqe, FixedBuffer, IoUringInstance,
    IoUringOp, OpStats as IoUringOpStats, RegisteredFdTable, SqeFlags,
};
    BridgeIoUringV2, IoUringV2BridgeStats, IoUringV2BufGroup, IoUringV2CqeFlag, IoUringV2Op,
    IoUringV2RegisteredFd, IoUringV2Ring,
};
    BridgeIoUringV3, CqeEntry, IoUringV3BridgeStats, IoUringV3Feature, IoUringV3Op, IoUringV3Ring,
    RegisteredBufPool, SqeEntry,
};
pub use ioprio_bridge::{BridgeIoprio, IoprioBridgeStats, IoprioClass, IoprioEntry, IoprioWho};
pub use iosched_bridge::{
    BridgeIoSched as BridgeIoSchedV2, IoSchedBridgeOp, IoSchedBridgePrio, IoSchedBridgeRecord,
    IoSchedBridgeResult, IoSchedBridgeStats,
};
pub use ipc_proxy::{
    BridgeIpcProxy, BridgeIpcProxyStats, IpcBatch, IpcChannel, IpcChannelState, IpcChannelType,
    IpcMessage, IpcRoute,
};
pub use irq_bridge::{
    BridgeIrqBridge, CpuIrqStats, IrqBridgeStats, IrqDesc, IrqDomain, IrqReturn, IrqStateFlag,
    IrqTrigger, IrqType,
};
pub use isolation::{
    ArgComparison, ArgFilter, AuditEntry as IsolationAuditEntry, AuditLog as IsolationAuditLog,
    BridgeIsolationManager, BridgeIsolationStats, FilterAction, FilterChain, FilterMatch,
    FilterRule,
};
pub use kcmp_bridge::{
    BridgeKcmp, KcmpBridgeStats, KcmpRequest, KcmpResponse, KcmpResult, KcmpType, ProcessResources,
    ResourceIdentity,
};
    BridgeKcmpV2, KcmpV2BridgeStats, KcmpV2Record, KcmpV2Result, KcmpV2Type, ResourceId,
};
pub use kcov_bridge::{
    BridgeKcovBridge, CmpRecord, CmpType, CovEntry, CoverageDatabase, KcovBridgeStats, KcovMode,
    TaskCovBuffer,
};
pub use kexec_bridge::{
    BridgeKexec, CrashReserveRegion, ImageState, KexecBridgeStats, KexecImage, KexecSegment,
    KexecType, PurgatoryState, ShutdownNotifier,
};
pub use keyctl_bridge::{
    BridgeKeyctl, KernelKey as KeyctlKernelKey, KeyPerm as KeyctlKeyPerm,
    KeyState as KeyctlKeyState, KeyType as KeyctlKeyType, KeyctlBridgeStats,
};
pub use keyring_bridge::{
    BridgeKeyring, KernelKey, KeyOp, KeyPerm, KeyState, KeyType, Keyring, KeyringBridgeStats,
};
    BridgeKeyringV2, KeyEntryV2, KeyPermV2, KeyStateV2, KeyTypeV2, KeyringV2, KeyringV2BridgeStats,
};
pub use kqueue_bridge::{
    BridgeKqueue, Kevent, KqueueBridgeStats, KqueueFilter, KqueueFlag, KqueueInstance, VnodeEvent,
};
pub use landlock_bridge::{
    BridgeLandlock, FsAccessRights, FsPathRule, LandlockAbi, LandlockBridgeStats, LandlockDomain,
    NetAccessRights, NetPortRule, Ruleset as LandlockRuleset,
};
    BridgeLandlockV2, LandlockFsFlags, LandlockV2BridgeStats, LandlockV2Rule, LandlockV2RuleType,
    LandlockV2Ruleset,
};
    BridgeLandlockV3, LandlockV3BridgeStats, LandlockV3FsAccess, LandlockV3NetAccess,
    LandlockV3Rule, LandlockV3RuleType, LandlockV3Ruleset,
};
    BridgeLandlockV4, LandlockV4BridgeStats, LandlockV4FsAccess, LandlockV4FsRule,
    LandlockV4NetAccess, LandlockV4NetRule, LandlockV4RuleType, LandlockV4Ruleset,
};
pub use marshal::{
    ArgDescriptor, ArgType as MarshalArgType, BridgeMarshalEngine, MarshalStats, MarshalledValue,
    PointerValidator, SyscallSignature as MarshalSignature,
    ValidationError as MarshalValidationError, ValidationResult as MarshalValidationResult,
};
pub use mem_proxy::{
    BridgeMemoryProxy, BridgeMemoryProxyStats, MadvHint, MemOp, MemOpRecord, ProcessMemProxy,
    ThpRecommendation, VmaProxyEntry,
};
pub use membarrier_bridge::{
    BridgeMembarrier, MembarrierBridgeStats, MembarrierCmd, MembarrierInvocation,
    MembarrierRegistration,
};
pub use memfd_bridge::{
    BridgeMemfd, MemfdBridgeStats, MemfdFlags, MemfdInstance, MemfdOp, SealFlags,
};
    BridgeMemfdV2, MemfdSeals, MemfdV2BridgeStats, MemfdV2Flags, MemfdV2Instance,
};
    BridgeMemfdV3, MemfdV3Backing, MemfdV3BridgeStats, MemfdV3Flag, MemfdV3Instance, MemfdV3Seal,
};
pub use metrics::{
    ErrorTracker, LatencyHistogram, MetricsRegistry, ProcessSyscallMetrics, SyscallTypeMetrics,
    ThroughputTracker,
};
pub use mmap_mgr::{
    BridgeMmapMgr, BridgeMmapMgrStats, MmapEvent, MmapEventType, ProcessAddrSpace, Vma, VmaPerms,
    VmaType,
};
    BridgeMmapV2, MmapV2Flag, MmapV2Prot, MmapV2Region, MmapV2Space, MmapV2Stats, MmapV2Type,
};
pub use mnt_ns_bridge::{
    BridgeMntNs, FsType as MntNsFsType, MntNsBridgeStats, MountEventType as MntNsEventType,
    MountFlags as MntNsFlags, MountNamespace as MntNsNamespace, MountPoint as MntNsPoint,
    MountPropagation as MntNsPropagation,
};
pub use module_bridge::{
    BridgeModuleBridge, ModuleBridgeStats, ModuleDesc, ModuleEvent, ModuleEventKind, ModuleLoadReq,
    ModuleParam, ModuleState, ModuleSymbol, ModuleTaint, ParamType,
};
pub use mount_bridge::{
    BridgeMountBridge, FsType, MountBridgeStats, MountEvent, MountEventType, MountFlags,
    MountNamespace, MountPoint, MountPropagation,
};
    BridgeMountV2, MountV2BridgeStats, MountV2Entry, MountV2Flag, MountV2FsContext, MountV2FsType,
    MountV2IdMap, MountV2Propagation,
};
    BridgeMountV3, MountV3BridgeOp, MountV3BridgeRecord, MountV3BridgeStats, MountV3Result,
};
pub use mq_bridge::{
    BridgeMq, MqBridgeStats, MqDescriptor, MqMessage as MqBridgeMessage, MqPriority,
};
pub use mqueue_bridge::{BridgeMqueue, MessageQueue, MqAttr, MqMessage, MqueueBridgeStats};
pub use msg_bridge::{BridgeMsgBridge, MsgBridgeStats, MsgEntry, MsgPerm, MsgQueue, MsgQueueState};
pub use namespace::{
    BridgeNamespaceManager, BridgeNamespaceStats, Namespace,
    NamespaceState as BridgeNamespaceState, NamespaceType as BridgeNamespaceType,
    ProcessNamespaceSet, TranslationRule, TranslationType,
};
pub use namespace_bridge::{
    BridgeNamespace as BridgeNsV2, Namespace as NsBridgeEntry, NamespaceBridgeStats,
    NsType as NsBridgeType, ProcessNsSet as NsBridgeProcessNsSet,
};
    BridgeNamespaceV2, NsV2BridgeStats, NsV2IdMapping, NsV2Instance, NsV2Ownership, NsV2State,
    NsV2Type,
};
pub use net_proxy::{
    BridgeNetProxy, BridgeNetProxyStats, BridgeSocketState, BridgeSocketType, CoalesceOpportunity,
    NetSyscallType, SocketEntry,
};
pub use netlink_bridge::{
    BridgeNetlink, GenlFamily, NetlinkBridgeStats, NlMcastGroup, NlMsgFlags, NlMsgHeader,
    NlMsgType as NlBridgeMsgType, NlProto, NlSocket as NlBridgeSocket,
};
pub use netlink_proxy::{
    BridgeNetlinkProxy, GenlFamily, GenlOp, NetlinkProxyStats, NlFamily, NlMessage, NlMsgFlags,
    NlMsgType, NlSocket,
};
    BridgeNetlinkV2, NetlinkV2BridgeStats, NlV2Message, NlV2MsgType, NlV2Protocol, NlV2Socket,
};
    BridgeNetlinkV3, NetlinkV3BridgeStats, NetlinkV3Proto, NetlinkV3Socket, NlMsgTypeV3,
};
pub use ns_proxy::{
    BridgeNsProxy, IdMapping, NamespaceDesc, NsRefType, NsReference, NsType, PidMapping,
    ProcessNsSet,
};
pub use numa_bridge::{
    BridgeNuma, NumaBridgeStats, NumaHintType, NumaMigrateMode, NumaNode, NumaPolicy,
    NumaProcessState,
};
pub use optimizer::{
    AdaptiveTuner, ContentionDetector, GlobalOptimizer, OptimizationBenefit,
    OptimizationOpportunity, OptimizationType, TunableParam,
};
pub use pattern::{NgramAnalyzer, PatternKind, PatternMatch, PatternMatcher, PatternTemplate};
pub use perf_bridge::{
    BridgePerf, CpuPmuState, HwEvent, PerfBridgeStats, PerfEvent, PerfEventAttr, PerfEventType,
    PerfSample, SampleType, SwEvent,
};
pub use perf_hw_bridge::{
    BridgePerfHw, CounterState as PmuCounterState, EventScope, PerfCounter as PmuPerfCounter,
    PerfEventGroup, PerfHwBridgeStats, PerfSample, PmuEventType, SamplingMode,
};
    BridgePerfV2, PerfV2Attr, PerfV2BridgeStats, PerfV2Event, PerfV2EventType, PerfV2SampleType,
};
    BridgePerfV3, PerfV3BridgeStats, PerfV3Counter, PerfV3EventType, PerfV3HwEvent,
};
pub use pidfd_bridge::{
    BridgePidfd, PidfdBridgeStats, PidfdFlags, PidfdInstance, PidfdOp, ProcessPidfdState,
};
    BridgePidfdV2, PidfdSignalV2, PidfdV2BridgeStats, PidfdV2Entry, PidfdV2Flag, PidfdV2State,
};
    BridgePidfdV3, PidfdV3BridgeStats, PidfdV3Flag, PidfdV3Instance, PidfdV3ProcState,
    PidfdV3WaitResult,
};
pub use pipe_bridge::{
    BridgePipeBridge, PipeBridgeStats, PipeBuffer, PipeInstance, PipeState, SpliceRecord,
};
pub use pipeline::{PipelineConfig, PipelineStage, StageDecision, SyscallPipeline};
pub use pkey_bridge::{BridgePkey, Pkey, PkeyAccess, PkeyBridgeStats, PkruState, ProcessPkeys};
    BridgePkeyV2, PkeyV2Access, PkeyV2BridgeStats, PkeyV2Entry, PkruState as PkruStateV2,
};
pub use poll_bridge::{
    BridgePollBridge, FdPollStats, PollBridgeStats, PollEvents, PollFdEntry, PollRequest,
    PollVariant,
};
pub use posix_timer_bridge::{
    BridgePosixTimer, PosixClockId, PosixTimer, PosixTimerBridgeStats, TimerNotify,
};
pub use power_bridge::{
    BridgePowerBridge, DevicePower, DomainState, PowerBridgeStats, PowerDomain, RuntimePmState,
    SleepState, WakeupSource,
};
pub use prctl_bridge::{BridgePrctl, PrctlBridgeStats, PrctlOption, ProcessPrctlState};
    BridgePrctlV2, PrctlV2BridgeStats, PrctlV2Op, PrctlV2ProcessState, PrctlV2Securebits,
    PrctlV2SpecCtrl,
};
pub use predict::{
    PredictedSyscall, SyscallConfidence, SyscallPattern, SyscallPredictor, SyscallSequence,
};
pub use prefetch::{
    FileReadAhead, PrefetchConfig, PrefetchManager, PrefetchPriority, PrefetchRequest, PrefetchType,
};
pub use priority::{
    BoostReason, BridgePriorityEngine, BridgePriorityStats, PriorityQueue, PriorityRequest,
    StarvationDetector, SyscallPriority,
};
pub use proc_lifecycle::{
    BridgeProcLifecycle, CloneFlagBridge, ExitReason, ProcEntry, ProcLifecycleState, ProcTreeNode,
};
pub use procfs_bridge::{
    BridgeProcfsBridge, ProcAccessResult, ProcEntry, ProcEntryType, ProcNamespace,
    ProcessProcState, ProcfsBridgeStats,
};
pub use profile::{AppBehavior, AppClass, AppProfile, AppProfiler, ResourceUsagePattern};
pub use ptrace_bridge::{
    Breakpoint, BridgePtraceBridge, BridgePtraceStats, CompareOp, PtraceEvent, PtraceRequest,
    RegisterSnapshot, Tracee, TraceeState, WatchType, Watchpoint,
};
    BridgePtraceV2, PtraceEvent as PtraceV2Event, PtraceRequest as PtraceV2Request, PtraceStop,
    PtraceV2BridgeStats, RegisterSet as PtraceRegisterSet, Tracee as PtraceV2Tracee,
    TraceeState as PtraceV2TraceeState,
};
    BridgePtraceV3, PtraceV3BridgeStats, PtraceV3Event, PtraceV3Request, PtraceV3Session,
};
    BridgePtraceV4, PtraceV4BridgeStats, PtraceV4Regs, PtraceV4Request, PtraceV4Stop,
    PtraceV4Tracee,
};
    BridgePtraceV5, PtraceV5BridgeStats, PtraceV5HwBreakpoint, PtraceV5HwType, PtraceV5Request,
    PtraceV5StopKind, PtraceV5TraceeState,
};
pub use queue::{
    BackpressureConfig, BackpressureState, DrainagePolicy, QueueEntry, QueueManager, QueuePriority,
    SyscallQueue,
};
pub use quota::{
    BridgeQuotaEnforcer, BridgeQuotaStats, GroupQuota, ProcessQuota, QuotaAction, QuotaDefinition,
    QuotaResource, QuotaState, WindowedUsage,
};
pub use quota_bridge::{
    BridgeQuota, DiskQuota, FsQuotaState, QuotaBridgeStats, QuotaEnforcement,
    QuotaResource as DiskQuotaResource, QuotaState as DiskQuotaState, QuotaType, QuotaViolation,
};
    BridgeQuotaV2, QuotaV2BridgeStats, QuotaV2Limits, QuotaV2Type, QuotaV2Usage,
};
pub use raid_bridge::{
    BridgeRaid, RaidBridgeOp, RaidBridgeRecord, RaidBridgeResult, RaidBridgeStats,
};
pub use ratelimit::{
    BridgeRateLimiter, RateLimitDecision, RateLimitPolicy, RateLimitScope, RateLimiterStats,
    SlidingWindowCounter as RateSlidingWindow, TokenBucket as RateTokenBucket,
};
pub use readahead_bridge::{
    BridgeReadahead, ReadaheadBridgeStats, ReadaheadContext, ReadaheadPattern, ReadaheadState,
};
pub use replay::{
    BridgeReplayManager, RecordedSyscall, RecordingFilter, RecordingSession, ReplayDivergence,
    ReplayManagerStats, ReplaySession, ReplayState, SyscallArg,
    SyscallResult as ReplaySyscallResult,
};
pub use retry::{
    BridgeRetryEngine, BridgeRetryStats, RetryBudget, RetryOutcome, RetryPolicy, RetryState,
    RetryStrategy, RetryableCategory,
};
pub use rlimit_bridge::{
    BridgeRlimitBridge, BridgeRlimitStats, LimitChangeAudit, LimitCheckResult, ProcessRlimits,
    RLIM_INFINITY, ResourceType, Rlimit,
};
    BridgeRlimitV2, ProcessRlimitsV2, RlimitV2BridgeStats, RlimitV2Check, RlimitV2Pair,
    RlimitV2Resource,
};
    BridgeRlimitV3, RlimitV3BridgeStats, RlimitV3Enforcement, RlimitV3ProcessLimits,
    RlimitV3Resource, RlimitV3Value,
};
pub use routing::{
    CachedRoute, FallbackChain, FallbackHandler, RouteCache, RouteConditions, RouteEntry,
    RoutePath, RouteReason, RouteStats, RoutingEngine,
};
pub use rseq_bridge::{
    BridgeRseq, CpuRseqState, CriticalSection as RseqCriticalSection, RseqAbort, RseqAbortReason,
    RseqBridgeStats, RseqFlags, RseqRegistration,
};
    BridgeRseqV2, RseqV2BridgeStats, RseqV2CriticalSection, RseqV2Flags, RseqV2ThreadState,
};
pub use sandbox::{
    ArgFilter, ArgOp, BridgeSandboxManager, FilterAction, SandboxInstance, SandboxManagerStats,
    SandboxProfile, SandboxRule, SandboxStrictness, SandboxViolation,
};
pub use sched_bridge::{
    BlockedSyscall, BridgeSchedBridge, BridgeSchedStats, ClassLatencyTracker, PreemptionRegion,
    PriorityInheritance, SchedHint, SyscallClassifier, SyscallSchedClass,
};
pub use sched_ext::{
    BridgeSchedExt, CpuScxState, DispatchFlags, DispatchQueue, SchedExtBridgeStats, SchedExtOp,
    ScxSchedulerInfo, ScxTaskState,
};
    BridgeSchedV2, ProcessSchedV2, SchedV2Attr, SchedV2BridgeStats, SchedV2Policy,
};
    BridgeSchedV3, SchedV3BridgeStats, SchedV3DeadlineParams, SchedV3EevdfState, SchedV3ExtOps,
    SchedV3Flag, SchedV3Policy, SchedV3TaskState,
};
pub use seccomp::{
    ArgCmp, ArgCondition, BridgeSeccompEngine, BridgeSeccompStats, SeccompAction,
    SeccompAuditEntry, SeccompFilter, SeccompRule,
};
pub use seccomp_bridge::{
    BridgeSeccomp, ProcessSeccomp, SeccompBridgeAction, SeccompBridgeFilter, SeccompBridgeStats,
    SeccompInsn as SeccompBridgeInsn,
};
    BridgeSeccompV2, ProcessSeccompV2, SeccompV2Action, SeccompV2BridgeStats, SeccompV2Cmp,
    SeccompV2Filter, SeccompV2Rule,
};
    BridgeSeccompV3, SeccompV3Action, SeccompV3Arch, SeccompV3ArgCheck, SeccompV3BridgeStats,
    SeccompV3CacheResult, SeccompV3CmpOp, SeccompV3Filter, SeccompV3Rule,
};
pub use security::{SecurityAction, SecurityEngine, SecurityRule};
pub use select_bridge::{BridgeSelect, SelectBridgeStats, SelectCall, SelectFdSet, SelectMask};
pub use sem_bridge::{
    BridgeSemBridge, SemBridgeStats, SemOp, SemPerm, SemUndo, SemWaiter, Semaphore, SemaphoreSet,
};
pub use sendfile_bridge::{
    BridgeSendfile, SendfileBridgeStats, SendfileMode, SendfilePipeBuf, SendfileSrcType,
    SendfileState, SendfileTransfer,
};
    BridgeSendfileV2, SendfileV2BridgeStats, SendfileV2Mode, SendfileV2Op, SendfileV2State,
};
pub use shm_bridge::{
    BridgeShmBridge, ShmAttach, ShmBridgeStats, ShmOp, ShmPerm, ShmSegment, ShmState,
};
pub use signal_proxy::{
    BridgeSignalProxy, BridgeSignalProxyStats, DeliveryState, ProcessSignalState, SignalCategory,
    SignalEntry,
};
pub use signalfd_bridge::{
    BridgeSignalfd, PendingSignal, SignalSet, SignalfdBridgeStats, SignalfdEvent, SignalfdFlags,
    SignalfdInstance, SignalfdOp,
};
    BridgeSignalfdV2, SignalfdV2BridgeStats, SignalfdV2Info, SignalfdV2Instance, SignalfdV2Signal,
};
pub use snapshot::{
    BridgeSnapshot, BridgeSnapshotManager, BridgeSnapshotStats, FdSnapshot, MemoryRegionSnapshot,
    ProcessSnapshot, RegisterState, SnapshotDiff, SnapshotScope, SnapshotState,
};
pub use socket_bridge::{
    BridgeSocket, BridgeSocketBridge, BridgeSocketStats, ConnInfo, SockAddr, SockBufStats,
    SockOptions, SocketDomain, SocketState, SocketType,
};
    BridgeSocketV2, SocketV2BridgeStats, SocketV2Family, SocketV2Instance, SocketV2Options,
    SocketV2State, SocketV2Type,
};
pub use splice_bridge::{
    BridgeSplice, EndpointType, PipeBuffer as SplicePipeBuffer, SpliceBridgeStats, SpliceFlags,
    SpliceOp, SpliceTransfer,
};
    BridgeSpliceV2, PipeV2Buffer, SpliceV2BridgeStats, SpliceV2Flags, SpliceV2Op, SpliceV2Transfer,
};
    BridgeSpliceV3, PipeV3Buffer, SpliceV3BridgeStats, SpliceV3Flag, SpliceV3Op, SpliceV3Transfer,
};
    BridgeSpliceV4, SpliceV4BridgeStats, SpliceV4Flag, SpliceV4Op, SpliceV4PageRef, SpliceV4Pipe,
    SpliceV4Transfer,
};
    BridgeSpliceV5, SpliceV5BridgeStats, SpliceV5Flag, SpliceV5Op, SpliceV5Page, SpliceV5Pipe,
    SpliceV5PipeState,
};
pub use superblock_bridge::{
    BridgeSuperblock, SbBridgeOp, SbBridgeRecord, SbBridgeResult, SbBridgeStats,
};
pub use swap_bridge::{
    BridgeSwapBridge, ProcessSwapInfo, SwapArea, SwapAreaState, SwapAreaType, SwapBridgeStats,
    SwapCluster,
};
pub use syscall::{
    OptimizationHint, SyscallContext, SyscallId, SyscallInterceptor, SyscallMetrics, SyscallResult,
    SyscallRouter, SyscallType,
};
pub use syscall_profiler::{
    BridgeSyscallProfilerV2, ErrnoTracker, LatencyBucketBridge, LatencyHistogram, SyscallPair,
    SyscallProfileV2,
};
pub use syscall_table::{
    BridgeSyscallTable, BridgeSyscallTableStats, HotPatch, SyscallCategory,
    SyscallEntry as SyscallTableEntry, SyscallRange, SyscallTableFlag,
};
pub use sysctl_bridge::{
    BridgeSysctl, SysctlBridgeStats, SysctlChangeEvent, SysctlNs, SysctlParam, SysctlPerm,
    SysctlTable, SysctlValueType,
};
pub use sysfs_proxy::{
    AttrType, BridgeSysfsProxy, KObject, SysfsAttr, SysfsProxyStats, SysfsSubsystem, Uevent,
    UeventAction, UeventFilter,
};
pub use syslog_bridge::{
    BridgeSyslog, SyslogBridgeStats, SyslogFacility, SyslogMessage, SyslogRingBuffer,
    SyslogSeverity,
};
    BridgeSyslogV2, SyslogV2BridgeStats, SyslogV2Entry, SyslogV2Facility, SyslogV2Level,
    SyslogV2Ring,
};
pub use taskstats_bridge::{
    BridgeTaskstats, TaskstatsBridgeStats, TaskstatsCmd, TaskstatsCpuAccounting, TaskstatsEntry,
    TaskstatsIoAccounting, TaskstatsMemAccounting, TaskstatsVersion,
};
pub use telemetry::{
    BridgeTelemetryManager, MetricType, MetricValue, SpanStatus, TelemetryCounter, TelemetryGauge,
    TelemetryHistogram, TelemetrySpan, TelemetryStats,
};
pub use throttle::{
    ProcessThrottleConfig, SlidingWindow, SyscallThrottleConfig, ThrottleDecision, ThrottleEngine,
    ThrottleReason, ThrottleStats, TokenBucket,
};
pub use timer_bridge::{
    BridgeTimerBridge, BridgeTimerBridgeStats, ClockSource as BridgeClockSource, CoalesceGroup,
    TimerEntry, TimerState as BridgeTimerState, TimerType, WheelLevel,
};
pub use timerfd_bridge::{
    BridgeTimerfd, TimerClockType, TimerSpec, TimerState as TfdTimerState, TimerfdBridgeStats,
    TimerfdEvent, TimerfdFlags, TimerfdInstance, TimerfdOp,
};
    BridgeTimerfdV2, TimerfdV2BridgeStats, TimerfdV2Clock, TimerfdV2Flags, TimerfdV2Instance,
    TimerfdV2Spec,
};
    BridgeTimerfdV3, TimerV3Clock, TimerV3Entry, TimerV3State, TimerfdV3BridgeStats,
};
    BridgeTimerfdV4, TimerfdV4BridgeStats, TimerfdV4Clock, TimerfdV4Flag, TimerfdV4Instance,
    TimerfdV4State,
};
pub use tls_bridge::{
    BridgeTls, TlsBridgeStats, TlsCipher, TlsConnection, TlsDirection, TlsVersion,
};
pub use trace::{
    BridgeTraceManager, BridgeTraceSession, LatencyHistogram as TraceLatencyHistogram,
    SessionState as TraceSessionState, SyscallTraceSummary, TraceEvent, TraceEventType,
    TraceFilter, TraceRingBuffer,
};
pub use transform::{TransformEngine, TransformRule, TransformType, TransformedSyscall};
pub use tty_bridge::{
    BridgeTtyBridge, LineDiscipline, PtyPair, TermiosAttrs, TtyBridgeStats, TtyDevice, TtyType,
    WinSize,
};
pub use user_context::{
    BridgeUserContext, BridgeUserContextStats, FpuState, GpRegs, RegisterSet, ThreadUserContext,
    TlsDescriptor, UserStack,
};
pub use userfault_bridge::{
    BridgeUserfault, FaultType, RegisterMode, RegisteredRange, ResolveOp, ResolveRequest,
    UffdEventType, UffdFeatures, UffdInstance, UffdMsg, UserfaultBridgeStats,
};
    BridgeUserfaultV2, UffdV2BridgeStats as UserfaultV2BridgeStats,
    UffdV2Event as UserfaultV2Event, UffdV2FaultType, UffdV2Feature as UserfaultV2Feature,
    UffdV2Instance as UserfaultV2Instance, UffdV2Registration,
};
pub use userfaultfd_bridge::{
    BridgeUserfaultfd, UffdBridgeStats, UffdEvent, UffdFaultType, UffdFeatures, UffdInstance,
    UffdRange, UffdRegMode,
};
    BridgeUserfaultfdV2, UffdV2BridgeStats, UffdV2Event, UffdV2Feature, UffdV2Instance, UffdV2Type,
};
pub use validate::{
    ArgRule, ArgType, SyscallValidationSpec, ValidationContext, ValidationEngine, ValidationError,
    ValidationFinding, ValidationReport, ValidationResult, ValidationStats,
};
pub use version::{
    ApiVersion, BridgeVersionManager, CompatShim, FeatureInfo, FeatureStatus, ShimType,
    SyscallDefinition, SyscallFeature, VersioningStats,
};
pub use vfs_bridge::{BridgeVfs, BridgeVfsCall, BridgeVfsResult, VfsBridgeRecord, VfsBridgeStats};
pub use vfs_proxy::{
    BridgeVfsProxy, BridgeVfsProxyStats, CacheResult, DentryCacheEntry, ProcessVfsProfile,
    StatCacheEntry, VfsOp,
};
pub use wait_queue::{
    BridgeWaitQueueMgr, BridgeWaitQueueStats, WaitEntry, WaitQueue, WaitQueueType, WaitState,
};
pub use wq_proxy::{
    BridgeWqProxy, WorkItem, WorkPriority as WqWorkPriority, WorkState as WqWorkState, Workqueue,
    WqFlag, WqProxyStats,
};
pub use xattr_bridge::{
    BridgeXattrBridge, InodeXattrs, XattrBridgeStats, XattrEntry, XattrNamespace, XattrOp,
    XattrOpRecord, XattrSetFlag,
};
    BridgeXattrV2, InodeXattrs as InodeXattrsV2, XattrEntry as XattrV2Entry, XattrFlags,
    XattrNamespace as XattrV2Namespace, XattrOp as XattrV2Op, XattrV2BridgeStats,
};
    BridgeXattrV3, InodeXattrsV3, XattrV3BridgeStats, XattrV3Entry, XattrV3Namespace,
};
// Re-exports from Round 25 — Security/crypto bridge
pub use apparmor_bridge::{
    AppArmorBridgeStats, AppArmorMode, AppArmorOp, AppArmorRecord, AppArmorResult,
    BridgeAppArmor,
};
    AuditV2BridgeStats, AuditV2MsgType, AuditV2Op, AuditV2Record, AuditV2Result, BridgeAuditV2,
};
pub use capability_bridge::{
    BridgeCap as BridgeCapPosix, BridgeCapability, CapBridgeStats as CapBridgeStatsV3, CapOp,
    CapRecord, CapResult,
};
    BridgeCryptoV2, CryptoV2AlgType, CryptoV2BridgeStats, CryptoV2Op, CryptoV2Record,
    CryptoV2Result,
};
pub use integrity_bridge::{
    BridgeIntegrity, IntegrityBridgeStats, IntegrityOp, IntegrityRecord, IntegrityResult,
};
    BridgeKeyringV3, KeyV3BridgeStats, KeyV3Op, KeyV3Record, KeyV3Result, KeyV3Type,
};
    BridgeLandlockV5, LandlockV5Access, LandlockV5BridgeStats, LandlockV5Op, LandlockV5Record,
    LandlockV5Result,
};
pub use lsm_bridge::{
    BridgeLsm, LsmBridgeStats, LsmDecision, LsmHookCategory, LsmHookRecord,
};
pub use random_bridge::{
    BridgeRandom, RandomBridgeStats, RandomOp, RandomRecord, RandomResult, RandomSource,
};
    BridgeSeccompV4, SeccompV4Action, SeccompV4BridgeStats, SeccompV4Op, SeccompV4Record,
};
pub use selinux_bridge::{
    AvcEntry, BridgeSelinux, SelinuxBridgeStats, SelinuxOp, SelinuxRecord, SelinuxResult,
};
// Re-exports from Round 26 — IPC/signals bridge
    BridgeEventfdV3, EventfdV3BridgeStats, EventfdV3Flag, EventfdV3Op, EventfdV3Record,
};
    BridgeMqueueV2, MqueueV2BridgeStats, MqueueV2Op, MqueueV2Record, MqueueV2Result,
};
pub use msgqueue_bridge::{
    BridgeMsgqueue, MsgctlCmd, MsgqueueBridgeStats, MsgqueueOp, MsgqueueRecord, MsgqueueResult,
};
    BridgePipeV2, PipeV2BridgeStats, PipeV2Flag, PipeV2Op, PipeV2Record,
};
pub use semaphore_bridge::{
    BridgeSemaphore, SemaphoreBridgeStats, SemaphoreOp, SemaphoreRecord, SemaphoreResult,
    SemctlCmd,
};
    BridgeShmV2, ShmV2BridgeStats, ShmV2Flag, ShmV2Op, ShmV2Record, ShmV2Result,
};
pub use sigaction_bridge::{
    BridgeSigaction, SigactionBridgeStats, SigactionFlag, SigactionHandler, SigactionOp,
    SigactionRecord,
};
pub use signal_bridge::{
    BridgeSignal, BridgeSignalMgr, SignalBridgeRecord, SignalBridgeStats, SignalMethod,
    SignalResult,
};
    BridgeSignalfdV3, SignalfdV3BridgeStats, SignalfdV3Op, SignalfdV3Record,
};
pub use sigprocmask_bridge::{
    BridgeSigprocmask, SigprocmaskBridgeStats, SigprocmaskHow, SigprocmaskRecord,
};
pub use sigqueue_bridge::{BridgeSigqueue, SigqueueBridgeStats, SigqueueRecord, SigqueueResult};

// Round 27 re-exports — Networking/socket bridge
pub use accept_bridge::{AcceptBridgeEvent, AcceptBridgeRecord, AcceptBridgeStats, BridgeAccept};
pub use bind_bridge::{BindBridgeRecord, BindBridgeStats, BindFamily, BridgeBind};
pub use listen_bridge::{BridgeListen, ListenBridgeEvent, ListenBridgeRecord, ListenBridgeStats};
pub use tcp_bridge::{BridgeTcp, TcpBridgeRecord, TcpBridgeState, TcpBridgeStats};
pub use udp_bridge::{BridgeUdp, UdpBridgeEvent, UdpBridgeRecord, UdpBridgeStats};
pub use unix_bridge::{BridgeUnix, UnixBridgeEvent, UnixBridgeRecord, UnixBridgeStats};

// Round 28 re-exports — Filesystem/VFS bridge
pub use fsync_bridge::{BridgeFsync, FsyncBridgeEvent, FsyncBridgeRecord, FsyncBridgeStats};
pub use readdir_bridge::{BridgeReaddir, ReaddirBridgeEvent, ReaddirBridgeRecord, ReaddirBridgeStats};
pub use statfs_bridge::{BridgeStatfs, StatfsBridgeEvent, StatfsBridgeRecord, StatfsBridgeStats};
pub use truncate_bridge::{BridgeTruncate, TruncateBridgeEvent, TruncateBridgeRecord, TruncateBridgeStats};

// Round 29 re-exports
pub use affinity_bridge::{BridgeAffinityEntry, BridgeAffinityManager, BridgeAffinityScope, BridgeAffinityStats};
pub use exec_bridge::{BridgeExecFormat, BridgeExecManager, BridgeExecResult, BridgeExecStats};
pub use exit_bridge::{BridgeExitManager, BridgeExitReason, BridgeExitRecord, BridgeExitStats};
pub use fork_bridge::{BridgeForkEntry, BridgeForkManager, BridgeForkStats, BridgeForkType};
pub use priority_bridge::{BridgePriorityEntry, BridgePriorityManager, BridgePriorityPolicy, BridgePriorityStats};
pub use pthread_bridge::{BridgePthreadEntry, BridgePthreadManager, BridgePthreadState, BridgePthreadStats};
pub use tid_bridge::{BridgeTidEntry, BridgeTidManager, BridgeTidPolicy, BridgeTidStats};
pub use wait_bridge::{BridgeExitStatus, BridgeWaitManager, BridgeWaitStats, BridgeWaitTarget};

// Re-exports from Round 30 — Memory management
pub use brk_bridge::{BridgeBrkManager, BridgeBrkOp, BridgeBrkState, BridgeBrkStats};
pub use hugepage_bridge::{BridgeHugepageAlloc, BridgeHugepageManager, BridgeHugepageSize, BridgeHugepageStats};
pub use madvise_bridge::{BridgeMadviseAdvice, BridgeMadviseManager, BridgeMadviseRecord, BridgeMadviseStats};
pub use mlock_bridge::{BridgeMlockManager, BridgeMlockOp, BridgeMlockRegion, BridgeMlockStats};
pub use mmap_bridge::{BridgeMmapFlag, BridgeMmapManager, BridgeMmapProt, BridgeMmapRegion, BridgeMmapStats};
pub use mprotect_bridge::{BridgeMprotectManager, BridgeMprotectPerm, BridgeMprotectRecord, BridgeMprotectStats};
pub use mremap_bridge::{BridgeMremapFlag, BridgeMremapManager, BridgeMremapRecord, BridgeMremapStats};
pub use msync_bridge::{BridgeMsyncFlag, BridgeMsyncManager, BridgeMsyncRecord, BridgeMsyncStats};
pub use munmap_bridge::{BridgeMunmapManager, BridgeMunmapResult, BridgeMunmapStats};
pub use shmem_bridge::{BridgeShmemManager, BridgeShmemRegion, BridgeShmemStats, BridgeShmemType};
pub use vma_bridge::{BridgeVmaEntry, BridgeVmaFlags, BridgeVmaManager, BridgeVmaStats, BridgeVmaType};

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_type_classification() {
        assert!(SyscallType::Read.is_io());
        assert!(SyscallType::Write.is_io());
        assert!(SyscallType::Mmap.is_memory());
        assert!(SyscallType::Fork.is_process());
        assert!(SyscallType::Socket.is_network());
    }

    #[test]
    fn test_syscall_prediction_basic() {
        let mut predictor = SyscallPredictor::new(64, 3);

        // Feed a pattern: read -> read -> read
        predictor.observe(SyscallType::Read);
        predictor.observe(SyscallType::Read);
        predictor.observe(SyscallType::Read);

        let prediction = predictor.predict_next();
        assert!(prediction.is_some());
        let pred = prediction.unwrap();
        assert_eq!(pred.syscall_type, SyscallType::Read);
        assert!(pred.confidence.value() > 0.5);
    }

    #[test]
    fn test_batch_optimizer() {
        let mut optimizer = BatchOptimizer::new(10, 1000);

        let e1 = BatchEntry::new(SyscallId(1), SyscallType::Read, 4096);
        let e2 = BatchEntry::new(SyscallId(2), SyscallType::Read, 4096);
        let e3 = BatchEntry::new(SyscallId(3), SyscallType::Read, 4096);

        optimizer.submit(e1);
        optimizer.submit(e2);
        optimizer.submit(e3);

        let groups = optimizer.flush();
        // Three reads should be batchable
        assert!(!groups.is_empty());
    }

    #[test]
    fn test_app_profiler() {
        let mut profiler = AppProfiler::new(100);

        // Simulate a sequential reader
        for _ in 0..50 {
            profiler.record_syscall(SyscallType::Read, 100);
        }

        let profile = profiler.build_profile();
        assert_eq!(profile.dominant_class, AppClass::IoIntensive);
    }

    #[test]
    fn test_async_io_engine() {
        let mut engine = AsyncIoEngine::new(256);

        let req = AsyncIoRequest::new(SyscallId(1), SyscallType::Read, 8192, AsyncPriority::Normal);
        let ticket = engine.submit(req);

        assert_eq!(engine.status(ticket), AsyncStatus::Queued);
        assert_eq!(engine.pending_count(), 1);
    }
}
