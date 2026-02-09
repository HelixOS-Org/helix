//! # NEXUS - Next-generation EXecutive Unified System
//!
//! ## The First Truly Intelligent Kernel Framework
//!
//! NEXUS represents the culmination of GENESIS (Year 1) of the Helix OS AI Roadmap.
//! It transforms the kernel from a simple executor into a conscious, self-aware system
//! capable of:
//!
//! - **Predicting** failures 30 seconds before they occur
//! - **Self-healing** without human intervention
//! - **Optimizing** across all architectures (x86_64, AArch64, RISC-V)
//! - **Learning** from every boot, crash, and interaction
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                           NEXUS GENESIS                                      │
//! ├─────────────────────────────────────────────────────────────────────────────┤
//! │                                                                              │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
//! │  │   TESTING   │  │  PREDICTION │  │   HEALING   │  │ PERFORMANCE │        │
//! │  │             │  │             │  │             │  │             │        │
//! │  │  • Fuzzing  │  │  • Crash    │  │  • Micro-   │  │  • SIMD     │        │
//! │  │  • Chaos    │  │  • Degrade  │  │    Rollback │  │  • Lock-free│        │
//! │  │  • Proof    │  │  • Forecast │  │  • Reconstr │  │  • Per-arch │        │
//! │  │  • Bench    │  │  • Anomaly  │  │  • Quarant  │  │  • Zero-copy│        │
//! │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘        │
//! │         │                │                │                │               │
//! │         └────────────────┴────────────────┴────────────────┘               │
//! │                                   │                                         │
//! │                          ┌────────▼────────┐                               │
//! │                          │  OBSERVABILITY  │                               │
//! │                          │                 │                               │
//! │                          │  • Tracing      │                               │
//! │                          │  • Causal Graph │                               │
//! │                          │  • Replay       │                               │
//! │                          │  • Debug        │                               │
//! │                          └─────────────────┘                               │
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use helix_nexus::{Nexus, NexusConfig};
//!
//! // Initialize NEXUS with default configuration
//! let config = NexusConfig::default();
//! Nexus::init(config).expect("Failed to initialize NEXUS");
//!
//! // Get the global instance
//! let nexus = Nexus::get();
//!
//! // Enable predictive monitoring
//! nexus.prediction().enable();
//!
//! // Start the self-healing engine
//! nexus.healing().start();
//!
//! // Process events
//! nexus.tick();
//! ```
//!
//! ## Features
//!
//! - `full` - All GENESIS features (default)
//! - `minimal` - Core prediction and healing only
//! - `q1_complete` - Testing, fuzzing, benchmarking, chaos, proof
//! - `q2_complete` - Prediction, degradation, canary, forecast, anomaly
//! - `q3_complete` - Healing, microrollback, reconstruct, quarantine, substitute
//! - `q4_complete` - Optimization, SIMD accelerators for all architectures
//! - `observability` - Tracing, causal graphs, replay, debug

#![no_std]
#![cfg_attr(feature = "nightly", feature(const_trait_impl))]
#![deny(unsafe_op_in_unsafe_fn)]
#![allow(missing_docs)]
#![warn(clippy::all)]
#![allow(dead_code)]

extern crate alloc;

// ============================================================================
// FAST PRIMITIVES — O(1) data structures for nanosecond hot paths
// ============================================================================

/// Ultra-high-performance primitives: RingBuffer, FastEma, FlatMap, FastHasher.
/// All operations target nanosecond latency with zero heap allocation.
pub mod fast;

// ============================================================================
// Q1 2026: HARDENING & TESTING
// ============================================================================

/// Testing framework for kernel-level validation
pub mod testing;

/// Kernel-level fuzzing with deterministic replay
pub mod fuzz;

/// Micro and macro benchmarking framework
pub mod bench;

/// Chaos engineering for kernel resilience testing
pub mod chaos;

/// Formal verification and proof-carrying code
pub mod proof;

// ============================================================================
// Q2 2026: PREDICTIVE INTELLIGENCE
// ============================================================================

/// Crash prediction engine (30s lookahead)
pub mod predict;

/// Degradation detection and analysis
pub mod degrade;

/// Canary invariants for early corruption detection
pub mod canary;

/// Resource forecasting engine
pub mod forecast;

/// Advanced anomaly detection
pub mod anomaly;

// ============================================================================
// Q3 2026: SELF-HEALING ENGINE
// ============================================================================

/// Core self-healing engine
pub mod heal;

/// Micro-rollback for granular recovery
pub mod microrollback;

/// State reconstruction from checkpoints
pub mod reconstruct;

/// Subsystem quarantine and isolation
pub mod quarantine;

/// Hot module substitution
pub mod substitute;

// ============================================================================
// Q4 2026: MULTI-ARCH PERFORMANCE
// ============================================================================

/// Cross-platform optimizations
pub mod optimize;

/// Architecture-specific accelerators
pub mod accel;

// ============================================================================
// AI & INTELLIGENCE MODULES
// ============================================================================

/// Machine learning primitives for kernel intelligence
pub mod ml;

/// AI-powered scheduler intelligence
pub mod scheduler;

/// Memory pattern analysis and prediction
pub mod memory;

/// Security anomaly detection and intrusion detection
pub mod security;

/// Power management intelligence
pub mod power;

/// Kernel telemetry system
pub mod telemetry;

/// I/O intelligence and optimization
pub mod io;

/// Network intelligence and traffic analysis
pub mod network;

/// Process intelligence and behavior analysis
pub mod process;

/// Cache intelligence and optimization
pub mod cache;

/// Interrupt intelligence and optimization
pub mod interrupt;

/// Filesystem intelligence and optimization
pub mod filesystem;

/// Driver intelligence and health monitoring
pub mod driver;

/// Virtualization and container intelligence
pub mod virtualization;

/// NUMA topology intelligence and optimization
pub mod numa;

/// Synchronization primitive analysis and optimization
pub mod sync;

/// Timer and time management intelligence
pub mod timer;

// ============================================================================
// HARDWARE SUBSYSTEM INTELLIGENCE
// ============================================================================

/// IOMMU and DMA remapping intelligence
pub mod iommu;

/// PCI bus device enumeration and management
pub mod pci;

/// USB subsystem device detection and analysis
pub mod usb;

/// Block device I/O scheduling intelligence
pub mod block;

/// Network interface management and traffic analysis
pub mod net;

/// Thermal zone management and cooling control
pub mod thermal;

/// Performance monitoring unit and hardware counters
pub mod perf;

/// Function tracing and latency analysis
pub mod ftrace;

// ============================================================================
// OBSERVABILITY
// ============================================================================

/// Ultra-low overhead tracing
pub mod trace;

/// Causal graph construction and analysis
pub mod causal;

/// Deterministic execution replay
pub mod replay;

/// AI-powered kernel debugger
pub mod debug;

/// Central intelligence coordinator - orchestrates all NEXUS modules
pub mod orchestrator;

// ============================================================================
// YEAR 2 - COGNITION: ADVANCED INTELLIGENCE
// ============================================================================

/// Code understanding engine - parses and comprehends kernel code
pub mod understand;

/// Causal reasoning engine - root cause analysis and explanation
pub mod reason;

/// Long-term memory - episodic, semantic, and procedural memory
pub mod ltm;

/// Advanced learning algorithms - reinforcement, online, meta, transfer, curriculum
/// Also includes: feedback loops, hypothesis testing, safe learning, regression detection
/// (Unified from learn/ and learning/ modules)
pub mod learning;

/// Goal-directed planning - hierarchical, temporal, reactive planning
pub mod planning;

/// Behavior systems - trees, state machines, utility AI, subsumption
pub mod behavior;

/// Semantic processing - embeddings, similarity, concepts, knowledge bases
pub mod semantic;

/// Neural network inference - tensors, layers, activations, networks
pub mod neural;

// ============================================================================
// YEAR 3 - EVOLUTION: SELF-EVOLUTION & GENETIC ALGORITHMS
// ============================================================================

// DANGER: The following modules can modify the kernel at runtime.
// They are exposed through the sandbox module with safety controls.
// Direct usage should be avoided in production.

/// Code generation engine - synthesize, verify, and optimize kernel code
/// WARNING: Can generate and execute arbitrary code
pub mod codegen;

/// Genetic algorithm engine - evolutionary optimization
/// WARNING: Can evolve system parameters
pub mod genetic;

/// Self-modification engine - runtime kernel evolution
/// WARNING: Can modify running kernel code
pub mod selfmod;

/// Sandbox for dangerous Year 3 modules with safety controls
/// Use this instead of direct access to codegen/genetic/selfmod
pub mod sandbox;

/// Distributed evolution - federated learning across nodes
pub mod distributed;

/// Quantum-inspired optimization - QAOA, quantum annealing
pub mod quantum;

/// Neural Architecture Search - automatic model design
pub mod nas;

/// Symbolic AI integration - logic programming, unification
pub mod symbolic;

/// Evolutionary game theory - Nash equilibrium, mechanism design
pub mod game_theory;

/// Morphogenetic kernel - self-organizing biological structures
pub mod morpho;

/// Zero-shot learning - handle novel situations without training
pub mod zeroshot;

/// Metacognitive controller - kernel self-awareness
pub mod metacog;

/// Formal verification engine - SAT/SMT, model checking
pub mod formal;

/// Emergent swarm intelligence - ACO, PSO, flocking
pub mod swarm;

// ============================================================================
// YEAR 4 - SYMBIOSIS: KERNEL-APPLICATION COOPERATION
// ============================================================================

/// Intelligent syscall layer - interception, prediction, batching, async I/O
pub mod bridge;

/// Application understanding engine - profiling, classification, adaptation
pub mod apps;

/// Cooperation protocol - bidirectional hints, contracts, negotiation
pub mod coop;

/// Holistic optimization - system-wide balancing, policies, prediction
pub mod holistic;

// ============================================================================
// COGNITIVE ARCHITECTURE (7 DOMAINS)
// ============================================================================

/// Typed identifiers, temporal types, and domain envelopes
pub mod types;

/// Core traits for cognitive domains
pub mod traits;

/// Message bus with unidirectional flow validation
pub mod bus;

/// Sense domain - perception layer with probes and signal collection
pub mod sense;

/// Decide domain - decision making with policy engine and conflict resolution
pub mod decide;

/// Act domain - execution layer with effectors and transaction management
pub mod act;

/// Reflect domain - metacognition, introspection, calibration, evolution
pub mod reflect;

// ============================================================================
// CORE INFRASTRUCTURE
// ============================================================================

/// Core types and traits
pub mod core;

/// Math utilities for no_std
pub mod math;

/// Event system for NEXUS
pub mod event;

/// Configuration management
pub mod config;

/// Error types
pub mod error;

/// Statistics and metrics
pub mod stats;

/// Integration layer
pub mod integration;

// ============================================================================
// RE-EXPORTS
// ============================================================================

pub use crate::accel::{AcceleratorRegistry, CryptoAccel, VectorOps};
pub use crate::anomaly::{Anomaly, AnomalyDetector};
// Apps (4.2) - Application Understanding Engine
pub use crate::apps::{
    AdaptationEngine, AppFingerprint, BehaviorSignature, OptimizationEngine, ProcessProfile,
    WorkloadCategory, WorkloadPredictor,
};
// TODO: bench module is empty - uncomment when types are implemented
// pub use crate::bench::{Benchmark, BenchmarkResult, BenchmarkSuite};
// Block Device Intelligence Re-exports
pub use crate::block::{
    BlockAnalysis, BlockDevice, BlockDeviceId, BlockDeviceState, BlockDeviceType,
    BlockIntelligence, BlockManager, DiskStats, IoRequest as BlockIoRequest, IoRequestType,
    IoScheduler as BlockIoScheduler, Major, Minor, Partition, PartitionType, QueueDepth,
    RequestQueue, WorkloadAnalysis as BlockWorkloadAnalysis, WorkloadType as BlockWorkloadType,
};
pub use crate::bridge::profile::{AppClass, AppProfile, AppProfiler};
// Year 4 - SYMBIOSIS Re-exports
// Bridge (4.1) - Intelligent Syscall Layer
pub use crate::bridge::{
    AsyncIoEngine, AsyncPriority, BatchOptimizer, SyscallContext, SyscallInterceptor,
    SyscallPredictor, SyscallRouter, SyscallType,
};
// Cache Intelligence Re-exports
pub use crate::cache::{
    AccessPattern as CacheAccessPattern, AccessPatternTracker, CacheEntry, CacheId,
    CacheIntelligence, CacheKey, CacheLevel, CacheLineState, CacheManager, CacheStats, CacheWarmer,
    EvictionOptimizer, EvictionPolicy, InclusionPolicy, MultiLevelCache,
};
pub use crate::canary::{Canary, CanaryMonitor, Invariant};
pub use crate::causal::{CausalEdge, CausalGraph, CausalNode};
pub use crate::chaos::{ChaosEngine, FaultConfig};
pub use crate::config::NexusConfig;
pub use crate::coop::hints::{
    AppHint, AppHintType, KernelAdvisory, KernelAdvisoryType, PressureLevel,
};
// Coop (4.3) - Cooperation Protocol
pub use crate::coop::{
    Contract, ContractState, CoopFeedback, CoopMessage, CoopSession, FeedbackCollector,
    FeedbackType, HintBus, NegotiationEngine, ProtocolVersion,
};
pub use crate::core::{Nexus, NexusLevel, NexusState};
pub use crate::debug::{BugPattern, Debugger, Diagnosis};
// Q2 Re-exports
// TODO: degrade module is empty - uncomment when types are implemented
// pub use crate::degrade::{DegradationDetector, DegradationType};
// Driver Intelligence Re-exports
pub use crate::driver::{
    CompatibilityAnalyzer, CompatibilityIssue, DeviceClass, DriverConflict, DriverFaultPredictor,
    DriverHealthMonitor, DriverId, DriverInfo, DriverIntelligence, DriverMetrics,
    DriverResourceTracker, DriverState, FaultPrediction, FaultType, HealthEvent, HealthEventType,
    HealthLevel, ResourceLimits, ResourceViolation,
};
pub use crate::error::{NexusError, NexusResult};
pub use crate::event::{NexusEvent, NexusEventKind};
// Filesystem Intelligence Re-exports
pub use crate::filesystem::{
    AccessMode, CachedFileInfo, DefragPriority, DirectoryAnalyzer, DirectoryInfo,
    FileAccessTracker, FileMeta, FileType, FilesystemIntelligence, FragmentationAnalyzer,
    FragmentationScore, FsOptimalSettings, FsWorkloadClassifier, FsWorkloadType, Inode,
    IoPatternType, PageCacheAnalyzer,
};
pub use crate::forecast::{ForecastResult, Forecaster, TimeSeries};
// Ftrace Intelligence Re-exports
pub use crate::ftrace::{
    CallGraph, CallGraphNode, FtraceAction, FtraceAnalysis, FtraceIntelligence, FtraceIssue,
    FtraceIssueType, FtraceManager, FtraceRecommendation, FuncAddr, FunctionInfo, HotFunction,
    LatencyIssue, LatencyRecord, LatencyStats, LatencyType, Pid, TraceBuffer, TraceEntry,
    TraceEntryType, TraceId, TracerOptions, TracerType,
};
pub use crate::fuzz::{FuzzInput, Fuzzer, MutationStrategy};
pub use crate::heal::{HealingEngine, HealingResult, HealingStrategy};
pub use crate::holistic::policy::{PolicyAction, PolicyCondition};
// Holistic (4.4) - System-Wide Optimization
pub use crate::holistic::{
    OptimizationGoal, Orchestrator, PolicyEngine, PolicyRule, ResourceBalancer, SystemPredictor,
    SystemSnapshot,
};
// Integration Re-exports
// TODO: integration module is empty - uncomment when types are implemented
// pub use crate::integration::{HealthProbe, HealthStatus, NexusRuntime};
// Interrupt Intelligence Re-exports
pub use crate::interrupt::{
    AffinityChange, AffinityOptimizer, CoalescingOptimizer, CoalescingSettings, CpuId,
    DeliveryMode, InterruptIntelligence, InterruptPattern, InterruptPatternDetector,
    InterruptPriority, InterruptRecord, InterruptType, Irq, IrqStats, StormDetector, StormEvent,
    StormInfo,
};
// I/O Intelligence Re-exports
pub use crate::io::{
    DeviceInfo, DeviceType, IoIntelligence, IoOpType, IoPriority, IoRequest, IoScheduler,
    IoSchedulerStats, LatencyPredictor, PrefetchConfig, PrefetchEngine, SchedulingAlgorithm,
};
// IOMMU Intelligence Re-exports
pub use crate::iommu::{
    DeviceId as IommuDeviceId, DmaDirection, DmaMapping, DmaMappingTracker, DomainId, DomainType,
    FaultTracker, FaultType as IommuFaultType, IommuAnalysis, IommuCapabilities, IommuDomain,
    IommuFault, IommuId, IommuIntelligence, IommuIssue, IommuManager, IommuState, IommuType,
    IommuUnit,
};
// TODO: Re-enable when types are implemented
// pub use crate::memory::{
//     AccessPattern, AllocationIntelligence, HotPageTracker, MemoryIntelligence, NumaAnalyzer,
//     PatternDetector, PrefetchPredictor,
// };
// Q3 Re-exports
pub use crate::microrollback::{MicroRollbackEngine, RollbackPoint};
// AI & Intelligence Re-exports
pub use crate::ml::{DecisionTree, KMeans, RandomForest, SGDClassifier, TinyNN};
// Network Interface Intelligence Re-exports
// TODO: Re-enable when types are implemented
// pub use crate::net::{
//     DriverFeature, DriverFeatures, Duplex, IfIndex, InterfaceState, InterfaceStats, InterfaceType,
//     Ipv4Address, Ipv6Address, LinkSpeed, LinkState, MacAddress, NetworkAnalysis,
//     NetworkIntelligence, NetworkInterface, NetworkManager, QdiscType, QueueStats, RingStats,
//     TrafficAnalysis, TrafficPattern,
// };
// Network Intelligence Re-exports
pub use crate::network::{
    BandwidthPredictor, ConnectionPredictor, Direction, FlowId, FlowStats, NetworkAnomaly,
    NetworkAnomalyDetector, NetworkAnomalyType, NetworkIntelligence, Protocol, QosClass, QosEngine,
    TrafficAnalyzer,
};
// NUMA Intelligence Re-exports
pub use crate::numa::{
    AffinityInfo, AffinityManager, AffinityViolation, BandwidthMonitor, Distance,
    LatencyPredictor as NumaLatencyPredictor, MigrationCostAnalyzer, NodeId, NumaIntelligence,
    NumaNode, NumaStats, NumaTopology, Placement, PlacementOptimizer, PlacementRecommendation,
};
// Q4 Re-exports
pub use crate::optimize::{Architecture, OptimizationLevel, Optimizer};
// Orchestrator Intelligence Re-exports
pub use crate::orchestrator::{
    Decision, DecisionAction, DecisionId, DecisionStatus, DecisionType, DecisionUrgency,
    GenesisSummary, HealthLevel as OrchestratorHealthLevel, OrchestratorAction,
    OrchestratorAnalysis, OrchestratorEvent, OrchestratorEventType, OrchestratorIntelligence,
    OrchestratorIssue, OrchestratorIssueType, OrchestratorManager, OrchestratorRecommendation,
    PolicyType, SubsystemId, SubsystemMetrics, SubsystemPriority, SubsystemState, SubsystemStatus,
    SystemPolicy,
};
// PCI Intelligence Re-exports
pub use crate::pci::{
    Bar, BarFlags, BarType, ClassCode, ExtCapability, ExtCapabilityId, PciAnalysis, PciBus,
    PciCapability, PciDevice, PciDeviceId, PciDeviceType, PciIntelligence, PciManager, PcieLink,
    PcieLinkSpeed, PcieLinkWidth, PowerState as PciPowerState, ProductId, VendorId,
};
// Performance Monitoring Intelligence Re-exports
// TODO: Re-enable when types are implemented
// pub use crate::perf::{
//     BranchMissRate, CacheEvent, CacheLevel, CacheMissRate, CacheOp, CacheResult, EventConfig,
//     EventId, EventState, EventType, HardwareEvent, Ipc, PerfAnalysis, PerfEvent, PerfIntelligence,
//     PerfManager, PerfMetrics, Pmu, PmuCapabilities, PmuId, PmuType, Sample, SampleType,
//     SoftwareEvent, WorkloadAnalysis as PerfWorkloadAnalysis, WorkloadCharacter,
// };
pub use crate::power::{
    CState, CStateSelector, EnergyProfiler, PState, PStateGovernor, PowerIntelligence, PowerMode,
};
// Re-export key types from submodules
pub use crate::predict::{CrashPrediction, PredictionConfidence, PredictionEngine};
// Process Intelligence Re-exports
// TODO: Re-enable when types are implemented
// pub use crate::process::{
//     BehaviorEvent, BehaviorEventType, CpuProfile, KillRecommendation, LifecycleEvent,
//     LifecycleManager, PriorityOptimizer, ProcessAnomaly, ProcessAnomalyType,
//     ProcessBehaviorAnalyzer, ProcessId, ProcessIntelligence, ProcessMetrics, ProcessProfile,
//     ProcessState, ProcessType, ResourcePrediction, ResourcePredictor,
// };
pub use crate::proof::{Property, PropertyType, VerificationResult, Verifier};
pub use crate::quarantine::{QuarantineLevel, QuarantineSystem};
pub use crate::reconstruct::{StateReconstructor, StateSnapshot};
// Observability Re-exports
pub use crate::replay::{RecordingSession, ReplayEngine, ReplayState};
pub use crate::scheduler::{
    AffinityPredictor, LoadPredictor, PriorityLearner, SchedulerIntelligence, WorkloadClassifier,
    WorkloadType,
};
pub use crate::security::{
    BehavioralProfile, IntrusionDetectionSystem, MemorySecurityMonitor, NetworkSecurityMonitor,
    SyscallMonitor, Threat, ThreatSeverity, ThreatType,
};
// TODO: Re-enable when types are implemented
// pub use crate::stats::NexusStats;
// pub use crate::substitute::{ModuleInfo, ModuleSlot, SubstitutionManager};
// Sync Intelligence Re-exports
pub use crate::sync::{
    AcquireMode, ContentionAnalyzer, ContentionEvent, ContentionStats, DeadlockDetector,
    DeadlockInfo, LockId, LockInfo, LockOrderOptimizer, LockState, LockType, LongSpin, NearMiss,
    OrderViolation, RwLockOptimizer, RwLockStats, RwPattern, RwRecommendation, SpinStats,
    SpinlockAnalyzer, SyncIntelligence, ThreadId, WaitTimeModel, WaitTimePredictor,
};
pub use crate::telemetry::{
    AlertRule, Counter, DataPoint, Gauge, TelemetryHistogram, TelemetryRegistry,
    TimeSeries as TelemetryTimeSeries,
};
// Q1 Re-exports
pub use crate::testing::{TestCase, TestResult, TestRunner, TestSuite};
// Thermal Intelligence Re-exports
pub use crate::thermal::{
    CoolingDevice, CoolingDeviceId, CoolingDeviceType, FanInfo, FanMode, Temperature,
    ThermalAction, ThermalAnalysis, ThermalEvent, ThermalEventType, ThermalGovernor,
    ThermalIntelligence, ThermalIssue, ThermalIssueType, ThermalManager, ThermalRecommendation,
    ThermalZone, ThermalZoneId, ThermalZoneMode, ThermalZoneType, TripPoint, TripPointType,
};
// Timer Intelligence Re-exports
// TODO: Re-enable when types are implemented
// pub use crate::timer::{
//     CoalescedGroup, CoalescingStats, DeadlinePredictor, HrtimerInfo, HrtimerManager, HrtimerMode,
//     JitterAnalyzer, JitterStats, PatternType, PowerAwareScheduler, SchedulingDecision,
//     TimerCoalescer, TimerId, TimerInfo, TimerIntelligence, TimerPattern, TimerPriority, TimerState,
//     TimerType, TimerWheel,
// };
pub use crate::trace::Tracer;
// USB Intelligence Re-exports
pub use crate::usb::{
    EndpointDirection, HubPort, HubPortState, TransferStatus, TransferType, UsbAnalysis, UsbBus,
    UsbBusType, UsbClass, UsbConfiguration, UsbDevice, UsbDeviceId, UsbDeviceState, UsbEndpoint,
    UsbHub, UsbIntelligence, UsbInterface, UsbManager, UsbProductId, UsbSpeed, UsbTransfer,
    UsbVendorId,
};
// Virtualization Intelligence Re-exports
pub use crate::virtualization::{
    CgroupStats, ContainerInfo, ContainerIntelligence, EscapeAttempt, GuestOs, IsolationAnalyzer,
    IsolationLevel, IsolationViolation, MigrationOptimizer, MigrationRecommendation, NamespaceInfo,
    NodeResources, SchedulingPolicy, VirtId, VirtMetrics, VirtResourceScheduler, VirtType,
    VirtualizationIntelligence, VmExitStats, VmExitType, VmInfo, VmIntelligence, WorkloadInfo,
    WorkloadPriority, WorkloadState,
};

// ============================================================================
// VERSION INFO
// ============================================================================

/// NEXUS version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// NEXUS build timestamp
pub const BUILD_TIMESTAMP: &str = "2029-01-29";

/// NEXUS codename
pub const CODENAME: &str = "SYMBIOSIS";

/// NEXUS year
pub const YEAR: u32 = 4;

// ============================================================================
// ARCHITECTURE DETECTION
// ============================================================================

/// Current architecture
#[cfg(target_arch = "x86_64")]
pub const ARCH: &str = "x86_64";

#[cfg(target_arch = "aarch64")]
pub const ARCH: &str = "aarch64";

#[cfg(target_arch = "riscv64")]
pub const ARCH: &str = "riscv64";

#[cfg(not(any(
    target_arch = "x86_64",
    target_arch = "aarch64",
    target_arch = "riscv64"
)))]
pub const ARCH: &str = "unknown";
