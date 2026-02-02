//! Virtualization Intelligence Module
//!
//! AI-powered virtualization and container management for the NEXUS subsystem.
//!
//! # Architecture
//!
//! The virtualization module provides:
//! - **Types**: Core types (VirtType, IsolationLevel, WorkloadState)
//! - **Workload**: Workload information and lifecycle
//! - **Metrics**: Resource usage metrics and time series
//! - **VM**: Virtual machine intelligence
//! - **Container**: Container-specific intelligence
//! - **Migration**: Smart workload migration optimization
//! - **Scheduler**: Intelligent resource scheduling
//! - **Isolation**: Security boundary analysis
//! - **Intelligence**: Central coordinator
//!
//! # Usage
//!
//! ```rust,ignore
//! use nexus::virtualization::{
//!     VirtualizationIntelligence, VmInfo, ContainerInfo,
//!     VirtType, WorkloadPriority,
//! };
//!
//! let mut intel = VirtualizationIntelligence::new();
//!
//! // Register a VM
//! let vm = VmInfo { ... };
//! intel.register_vm(vm);
//!
//! // Register a container
//! let container = ContainerInfo { ... };
//! intel.register_container(container);
//!
//! // Update metrics
//! intel.update_metrics(workload_id, metrics);
//! ```

extern crate alloc;

// Submodules
pub mod container;
pub mod intelligence;
pub mod isolation;
pub mod metrics;
pub mod migration;
pub mod scheduler;
pub mod types;
pub mod vm;
pub mod workload;

// Re-export all public types
pub use container::{
    CgroupStats, ContainerInfo, ContainerIntelligence, MountInfo, NamespaceInfo, NetworkMode,
};
pub use intelligence::VirtualizationIntelligence;
pub use isolation::{
    EscapeAttempt, EscapeType, IsolationAnalyzer, IsolationViolation, SecurityBoundary,
    ViolationSeverity, ViolationType,
};
pub use metrics::{MetricsSeries, VirtMetrics};
pub use migration::{
    MigrationOptimizer, MigrationReason, MigrationRecommendation, MigrationRecord, NodeResources,
};
pub use scheduler::{
    AllocationEvent, AllocationEventType, CpuPolicy, IoPolicy, MemoryPolicy, ResourceReservation,
    SchedulingPolicy, VirtResourceScheduler,
};
pub use types::{IsolationLevel, VirtId, VirtType, WorkloadPriority, WorkloadState};
pub use vm::{
    DiskFormat, DiskInfo, GuestOs, IoIntensity, NetIntensity, VmExitStats, VmExitType, VmInfo,
    VmIntelligence, VmProfile,
};
pub use workload::WorkloadInfo;
