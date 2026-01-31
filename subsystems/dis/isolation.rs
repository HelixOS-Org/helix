//! # Security Isolation - Capability-Based Access Control
//!
//! The Security Isolation module provides a comprehensive security framework
//! for the DIS scheduler, implementing capability-based access control,
//! resource isolation, and security domains.
//!
//! ## Security Model
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                          SECURITY DOMAINS                                    │
//! │                                                                              │
//! │   ┌─────────────────────────────────────────────────────────────────────┐   │
//! │   │                        KERNEL DOMAIN                                 │   │
//! │   │   • Full access to all resources                                     │   │
//! │   │   • Can modify security policies                                     │   │
//! │   │   • Manages all other domains                                        │   │
//! │   └─────────────────────────────────────────────────────────────────────┘   │
//! │                                  │                                          │
//! │                                  ▼                                          │
//! │   ┌─────────────────────────────────────────────────────────────────────┐   │
//! │   │                        SYSTEM DOMAIN                                 │   │
//! │   │   • Access to system resources                                       │   │
//! │   │   • Can manage user domains                                          │   │
//! │   │   • Limited kernel access                                            │   │
//! │   └─────────────────────────────────────────────────────────────────────┘   │
//! │                                  │                                          │
//! │                                  ▼                                          │
//! │   ┌─────────────────────────────────────────────────────────────────────┐   │
//! │   │                        USER DOMAINS                                  │   │
//! │   │                                                                      │   │
//! │   │   ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────────┐    │   │
//! │   │   │  Domain A  │  │  Domain B  │  │  Domain C  │  │  Sandbox   │    │   │
//! │   │   │            │  │            │  │            │  │            │    │   │
//! │   │   │  Tasks:    │  │  Tasks:    │  │  Tasks:    │  │  Tasks:    │    │   │
//! │   │   │  [1][2]    │  │  [3][4]    │  │  [5]       │  │  [6][7]    │    │   │
//! │   │   └────────────┘  └────────────┘  └────────────┘  └────────────┘    │   │
//! │   │                                                                      │   │
//! │   └─────────────────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Capabilities
//!
//! Each task has a set of capabilities that define what it can do:
//!
//! - **Resource Capabilities**: CPU, Memory, I/O access
//! - **System Capabilities**: IPC, Signals, Device access
//! - **Security Capabilities**: Domain management, Policy modification
//!
//! ## Isolation Levels
//!
//! 1. **None**: No isolation (for testing)
//! 2. **Light**: Basic process isolation
//! 3. **Standard**: Full memory and resource isolation
//! 4. **Strict**: Complete isolation with verified IPC only
//! 5. **Paranoid**: Maximum isolation, minimal capabilities

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use bitflags::bitflags;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::RwLock;

use super::{TaskId, Nanoseconds, DISError, DISResult};

// =============================================================================
// Security Domain
// =============================================================================

/// Unique identifier for a security domain
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomainId(u64);

impl DomainId {
    /// Create new domain ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
    
    /// Get raw ID
    pub fn id(&self) -> u64 {
        self.0
    }
    
    /// Kernel domain (always 0)
    pub const KERNEL: Self = Self(0);
    /// System domain (always 1)
    pub const SYSTEM: Self = Self(1);
}

/// Security domain definition
#[derive(Debug, Clone)]
pub struct SecurityDomain {
    /// Domain ID
    pub id: DomainId,
    /// Domain name
    pub name: String,
    /// Domain type
    pub domain_type: DomainType,
    /// Parent domain (if any)
    pub parent: Option<DomainId>,
    /// Default capabilities for tasks in this domain
    pub default_caps: CapabilitySet,
    /// Maximum capabilities allowed in this domain
    pub max_caps: CapabilitySet,
    /// Isolation level
    pub isolation: IsolationLevel,
    /// Resource limits
    pub limits: ResourceLimits,
    /// Active tasks in this domain
    pub tasks: BTreeSet<TaskId>,
    /// Child domains
    pub children: BTreeSet<DomainId>,
    /// Domain flags
    pub flags: DomainFlags,
    /// Creation timestamp
    pub created: Nanoseconds,
    /// Statistics
    pub stats: DomainStats,
}

/// Domain type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainType {
    /// Kernel domain - highest privilege
    Kernel,
    /// System domain - system services
    System,
    /// User domain - normal applications
    User,
    /// Service domain - background services
    Service,
    /// Sandbox domain - untrusted code
    Sandbox,
    /// Guest domain - virtualized environments
    Guest,
    /// Container domain - containerized applications
    Container,
}

bitflags! {
    /// Domain flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DomainFlags: u32 {
        /// Domain is active
        const ACTIVE = 1 << 0;
        /// Domain can create child domains
        const CAN_CREATE_CHILDREN = 1 << 1;
        /// Domain can delegate capabilities
        const CAN_DELEGATE = 1 << 2;
        /// Domain is privileged
        const PRIVILEGED = 1 << 3;
        /// Domain is a sandbox
        const SANDBOXED = 1 << 4;
        /// Domain is audited
        const AUDITED = 1 << 5;
        /// Domain is ephemeral (destroyed when empty)
        const EPHEMERAL = 1 << 6;
        /// Domain has network access
        const NETWORK = 1 << 7;
        /// Domain has filesystem access
        const FILESYSTEM = 1 << 8;
        /// Domain has device access
        const DEVICES = 1 << 9;
    }
}

/// Domain statistics
#[derive(Debug, Default)]
pub struct DomainStats {
    /// Total CPU time used
    pub cpu_time: Nanoseconds,
    /// Peak memory usage
    pub peak_memory: u64,
    /// Current memory usage
    pub current_memory: u64,
    /// Number of tasks created
    pub tasks_created: u64,
    /// Number of capability checks
    pub cap_checks: u64,
    /// Number of violations
    pub violations: u64,
}

impl Clone for DomainStats {
    fn clone(&self) -> Self {
        Self {
            cpu_time: self.cpu_time,
            peak_memory: self.peak_memory,
            current_memory: self.current_memory,
            tasks_created: self.tasks_created,
            cap_checks: self.cap_checks,
            violations: self.violations,
        }
    }
}

impl SecurityDomain {
    /// Create new security domain
    pub fn new(id: DomainId, name: &str, domain_type: DomainType) -> Self {
        let (default_caps, max_caps, flags, isolation) = match domain_type {
            DomainType::Kernel => (
                CapabilitySet::all(),
                CapabilitySet::all(),
                DomainFlags::ACTIVE | DomainFlags::PRIVILEGED | DomainFlags::CAN_CREATE_CHILDREN | DomainFlags::CAN_DELEGATE,
                IsolationLevel::None,
            ),
            DomainType::System => (
                CapabilitySet::system_default(),
                CapabilitySet::system_max(),
                DomainFlags::ACTIVE | DomainFlags::CAN_CREATE_CHILDREN | DomainFlags::CAN_DELEGATE | DomainFlags::NETWORK | DomainFlags::FILESYSTEM | DomainFlags::DEVICES,
                IsolationLevel::Light,
            ),
            DomainType::User => (
                CapabilitySet::user_default(),
                CapabilitySet::user_max(),
                DomainFlags::ACTIVE | DomainFlags::NETWORK | DomainFlags::FILESYSTEM,
                IsolationLevel::Standard,
            ),
            DomainType::Service => (
                CapabilitySet::service_default(),
                CapabilitySet::service_max(),
                DomainFlags::ACTIVE | DomainFlags::NETWORK,
                IsolationLevel::Standard,
            ),
            DomainType::Sandbox => (
                CapabilitySet::sandbox_default(),
                CapabilitySet::sandbox_max(),
                DomainFlags::ACTIVE | DomainFlags::SANDBOXED | DomainFlags::EPHEMERAL,
                IsolationLevel::Strict,
            ),
            DomainType::Guest => (
                CapabilitySet::guest_default(),
                CapabilitySet::guest_max(),
                DomainFlags::ACTIVE,
                IsolationLevel::Strict,
            ),
            DomainType::Container => (
                CapabilitySet::container_default(),
                CapabilitySet::container_max(),
                DomainFlags::ACTIVE | DomainFlags::EPHEMERAL,
                IsolationLevel::Strict,
            ),
        };
        
        Self {
            id,
            name: name.to_string(),
            domain_type,
            parent: None,
            default_caps,
            max_caps,
            isolation,
            limits: ResourceLimits::default_for(domain_type),
            tasks: BTreeSet::new(),
            children: BTreeSet::new(),
            flags,
            created: Nanoseconds::zero(),
            stats: DomainStats::default(),
        }
    }
    
    /// Add task to domain
    pub fn add_task(&mut self, task_id: TaskId) {
        self.tasks.insert(task_id);
        self.stats.tasks_created += 1;
    }
    
    /// Remove task from domain
    pub fn remove_task(&mut self, task_id: TaskId) -> bool {
        self.tasks.remove(&task_id)
    }
    
    /// Check if domain can grant capability
    pub fn can_grant(&self, cap: Capability) -> bool {
        self.max_caps.has(cap) && self.flags.contains(DomainFlags::CAN_DELEGATE)
    }
    
    /// Check if domain is ancestor of another
    pub fn is_ancestor_of(&self, domain: &SecurityDomain) -> bool {
        // Simple check - in real impl would traverse tree
        self.children.contains(&domain.id)
    }
}

impl Default for SecurityDomain {
    fn default() -> Self {
        Self::new(DomainId::new(0), "default", DomainType::User)
    }
}

// =============================================================================
// Capabilities
// =============================================================================

/// Individual capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u32)]
pub enum Capability {
    // Resource capabilities (0-31)
    /// Use CPU
    UseCpu = 0,
    /// Allocate memory
    AllocMemory = 1,
    /// Perform I/O
    PerformIo = 2,
    /// Access network
    Network = 3,
    /// Access filesystem
    Filesystem = 4,
    /// Access devices
    Devices = 5,
    /// Use DMA
    Dma = 6,
    /// Map physical memory
    PhysicalMemory = 7,
    
    // Scheduling capabilities (32-47)
    /// Set own priority
    SetOwnPriority = 32,
    /// Set any priority
    SetAnyPriority = 33,
    /// Bind to CPU
    CpuAffinity = 34,
    /// Real-time scheduling
    RealTime = 35,
    /// Bypass scheduler
    BypassScheduler = 36,
    /// Set nice value
    Nice = 37,
    
    // IPC capabilities (48-63)
    /// Send signals
    Signal = 48,
    /// Create shared memory
    SharedMemory = 49,
    /// Create message queue
    MessageQueue = 50,
    /// Create semaphore
    Semaphore = 51,
    /// Send IPC to any task
    IpcAny = 52,
    
    // System capabilities (64-79)
    /// Create tasks
    CreateTask = 64,
    /// Kill tasks
    KillTask = 65,
    /// Debug tasks
    DebugTask = 66,
    /// Trace tasks
    TraceTask = 67,
    /// Access /proc
    ProcAccess = 68,
    /// Mount filesystems
    Mount = 69,
    /// Load modules
    LoadModule = 70,
    
    // Security capabilities (80-95)
    /// Create domains
    CreateDomain = 80,
    /// Delete domains
    DeleteDomain = 81,
    /// Modify domain
    ModifyDomain = 82,
    /// Grant capabilities
    GrantCap = 83,
    /// Revoke capabilities
    RevokeCap = 84,
    /// Bypass security checks
    BypassSecurity = 85,
    /// Audit access
    Audit = 86,
    
    // Administrative capabilities (96-111)
    /// Shutdown system
    Shutdown = 96,
    /// Reboot system
    Reboot = 97,
    /// Configure kernel
    ConfigKernel = 98,
    /// Manage resources
    ResourceAdmin = 99,
    /// Time administration
    TimeAdmin = 100,
}

/// Set of capabilities
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CapabilitySet {
    /// Bitmap of capabilities (128 bits = 4 u32s)
    bits: [u32; 4],
}

impl CapabilitySet {
    /// Create empty set
    pub const fn empty() -> Self {
        Self { bits: [0; 4] }
    }
    
    /// Create set with all capabilities
    pub const fn all() -> Self {
        Self { bits: [u32::MAX; 4] }
    }
    
    /// Check if has capability
    pub fn has(&self, cap: Capability) -> bool {
        let cap_num = cap as u32;
        let word = (cap_num / 32) as usize;
        let bit = cap_num % 32;
        self.bits.get(word).map_or(false, |w| (w >> bit) & 1 == 1)
    }
    
    /// Add capability
    pub fn add(&mut self, cap: Capability) {
        let cap_num = cap as u32;
        let word = (cap_num / 32) as usize;
        let bit = cap_num % 32;
        if word < 4 {
            self.bits[word] |= 1 << bit;
        }
    }
    
    /// Remove capability
    pub fn remove(&mut self, cap: Capability) {
        let cap_num = cap as u32;
        let word = (cap_num / 32) as usize;
        let bit = cap_num % 32;
        if word < 4 {
            self.bits[word] &= !(1 << bit);
        }
    }
    
    /// Union with another set
    pub fn union(&self, other: &CapabilitySet) -> CapabilitySet {
        let mut result = self.clone();
        for i in 0..4 {
            result.bits[i] |= other.bits[i];
        }
        result
    }
    
    /// Intersection with another set
    pub fn intersection(&self, other: &CapabilitySet) -> CapabilitySet {
        let mut result = self.clone();
        for i in 0..4 {
            result.bits[i] &= other.bits[i];
        }
        result
    }
    
    /// Check if this is a subset of another
    pub fn is_subset_of(&self, other: &CapabilitySet) -> bool {
        for i in 0..4 {
            if self.bits[i] & !other.bits[i] != 0 {
                return false;
            }
        }
        true
    }
    
    /// Count capabilities
    pub fn count(&self) -> u32 {
        self.bits.iter().map(|w| w.count_ones()).sum()
    }
    
    /// System service default capabilities
    pub fn system_default() -> Self {
        let mut caps = Self::empty();
        caps.add(Capability::UseCpu);
        caps.add(Capability::AllocMemory);
        caps.add(Capability::PerformIo);
        caps.add(Capability::Network);
        caps.add(Capability::Filesystem);
        caps.add(Capability::Devices);
        caps.add(Capability::SetOwnPriority);
        caps.add(Capability::CpuAffinity);
        caps.add(Capability::Signal);
        caps.add(Capability::SharedMemory);
        caps.add(Capability::CreateTask);
        caps.add(Capability::Mount);
        caps
    }
    
    /// System service maximum capabilities
    pub fn system_max() -> Self {
        let mut caps = Self::system_default();
        caps.add(Capability::RealTime);
        caps.add(Capability::KillTask);
        caps.add(Capability::LoadModule);
        caps.add(Capability::CreateDomain);
        caps
    }
    
    /// User default capabilities
    pub fn user_default() -> Self {
        let mut caps = Self::empty();
        caps.add(Capability::UseCpu);
        caps.add(Capability::AllocMemory);
        caps.add(Capability::PerformIo);
        caps.add(Capability::Network);
        caps.add(Capability::Filesystem);
        caps.add(Capability::SetOwnPriority);
        caps.add(Capability::Signal);
        caps.add(Capability::SharedMemory);
        caps
    }
    
    /// User maximum capabilities
    pub fn user_max() -> Self {
        let mut caps = Self::user_default();
        caps.add(Capability::CreateTask);
        caps.add(Capability::CpuAffinity);
        caps.add(Capability::Nice);
        caps
    }
    
    /// Service default capabilities
    pub fn service_default() -> Self {
        let mut caps = Self::empty();
        caps.add(Capability::UseCpu);
        caps.add(Capability::AllocMemory);
        caps.add(Capability::PerformIo);
        caps.add(Capability::Network);
        caps.add(Capability::SetOwnPriority);
        caps.add(Capability::SharedMemory);
        caps.add(Capability::MessageQueue);
        caps
    }
    
    /// Service maximum capabilities
    pub fn service_max() -> Self {
        let mut caps = Self::service_default();
        caps.add(Capability::Filesystem);
        caps.add(Capability::CpuAffinity);
        caps
    }
    
    /// Sandbox default capabilities
    pub fn sandbox_default() -> Self {
        let mut caps = Self::empty();
        caps.add(Capability::UseCpu);
        caps.add(Capability::AllocMemory);
        caps
    }
    
    /// Sandbox maximum capabilities
    pub fn sandbox_max() -> Self {
        let mut caps = Self::sandbox_default();
        caps.add(Capability::PerformIo);
        caps
    }
    
    /// Guest default capabilities
    pub fn guest_default() -> Self {
        let mut caps = Self::empty();
        caps.add(Capability::UseCpu);
        caps.add(Capability::AllocMemory);
        caps.add(Capability::PerformIo);
        caps
    }
    
    /// Guest maximum capabilities
    pub fn guest_max() -> Self {
        let mut caps = Self::guest_default();
        caps.add(Capability::Network);
        caps
    }
    
    /// Container default capabilities
    pub fn container_default() -> Self {
        let mut caps = Self::empty();
        caps.add(Capability::UseCpu);
        caps.add(Capability::AllocMemory);
        caps.add(Capability::PerformIo);
        caps.add(Capability::Network);
        caps.add(Capability::Filesystem);
        caps.add(Capability::Signal);
        caps.add(Capability::CreateTask);
        caps
    }
    
    /// Container maximum capabilities
    pub fn container_max() -> Self {
        let mut caps = Self::container_default();
        caps.add(Capability::SetOwnPriority);
        caps.add(Capability::SharedMemory);
        caps
    }
}

// =============================================================================
// Isolation Level
// =============================================================================

/// Isolation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsolationLevel {
    /// No isolation (kernel mode)
    None = 0,
    /// Light isolation (basic process isolation)
    Light = 1,
    /// Standard isolation (full memory protection)
    Standard = 2,
    /// Strict isolation (verified IPC only)
    Strict = 3,
    /// Paranoid isolation (maximum restrictions)
    Paranoid = 4,
}

impl IsolationLevel {
    /// Check if allows direct memory sharing
    pub fn allows_shared_memory(&self) -> bool {
        matches!(self, IsolationLevel::None | IsolationLevel::Light | IsolationLevel::Standard)
    }
    
    /// Check if allows unverified IPC
    pub fn allows_unverified_ipc(&self) -> bool {
        matches!(self, IsolationLevel::None | IsolationLevel::Light)
    }
    
    /// Check if requires capability checks
    pub fn requires_cap_checks(&self) -> bool {
        !matches!(self, IsolationLevel::None)
    }
    
    /// Check if allows cross-domain communication
    pub fn allows_cross_domain(&self) -> bool {
        !matches!(self, IsolationLevel::Paranoid)
    }
}

// =============================================================================
// Resource Limits
// =============================================================================

/// Resource limits for a domain
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum CPU percentage (0-100 per core, max = cores * 100)
    pub max_cpu: u32,
    /// Maximum memory (bytes)
    pub max_memory: u64,
    /// Maximum file descriptors
    pub max_fds: u32,
    /// Maximum tasks
    pub max_tasks: u32,
    /// Maximum child domains
    pub max_children: u32,
    /// I/O bandwidth limit (bytes/sec)
    pub io_bandwidth: u64,
    /// Network bandwidth limit (bytes/sec)
    pub net_bandwidth: u64,
    /// Priority range (min, max)
    pub priority_range: (i8, i8),
    /// Real-time priority allowed
    pub realtime_allowed: bool,
    /// Maximum CPU affinity mask
    pub cpu_mask: u64,
}

impl ResourceLimits {
    /// Unlimited (for kernel)
    pub fn unlimited() -> Self {
        Self {
            max_cpu: u32::MAX,
            max_memory: u64::MAX,
            max_fds: u32::MAX,
            max_tasks: u32::MAX,
            max_children: u32::MAX,
            io_bandwidth: u64::MAX,
            net_bandwidth: u64::MAX,
            priority_range: (-128, 127),
            realtime_allowed: true,
            cpu_mask: u64::MAX,
        }
    }
    
    /// Default for domain type
    pub fn default_for(domain_type: DomainType) -> Self {
        match domain_type {
            DomainType::Kernel => Self::unlimited(),
            DomainType::System => Self {
                max_cpu: 400,  // 4 cores
                max_memory: 8 * 1024 * 1024 * 1024,  // 8 GB
                max_fds: 65536,
                max_tasks: 1024,
                max_children: 64,
                io_bandwidth: u64::MAX,
                net_bandwidth: u64::MAX,
                priority_range: (-20, 99),
                realtime_allowed: true,
                cpu_mask: u64::MAX,
            },
            DomainType::User => Self {
                max_cpu: 200,  // 2 cores
                max_memory: 4 * 1024 * 1024 * 1024,  // 4 GB
                max_fds: 1024,
                max_tasks: 256,
                max_children: 16,
                io_bandwidth: 100 * 1024 * 1024,  // 100 MB/s
                net_bandwidth: 100 * 1024 * 1024,  // 100 MB/s
                priority_range: (0, 39),
                realtime_allowed: false,
                cpu_mask: u64::MAX,
            },
            DomainType::Service => Self {
                max_cpu: 100,  // 1 core
                max_memory: 1024 * 1024 * 1024,  // 1 GB
                max_fds: 256,
                max_tasks: 64,
                max_children: 4,
                io_bandwidth: 50 * 1024 * 1024,  // 50 MB/s
                net_bandwidth: 50 * 1024 * 1024,  // 50 MB/s
                priority_range: (0, 20),
                realtime_allowed: false,
                cpu_mask: u64::MAX,
            },
            DomainType::Sandbox => Self {
                max_cpu: 50,  // 0.5 core
                max_memory: 256 * 1024 * 1024,  // 256 MB
                max_fds: 64,
                max_tasks: 16,
                max_children: 0,
                io_bandwidth: 10 * 1024 * 1024,  // 10 MB/s
                net_bandwidth: 0,  // No network
                priority_range: (0, 10),
                realtime_allowed: false,
                cpu_mask: 0x1,  // Single CPU
            },
            DomainType::Guest => Self {
                max_cpu: 200,
                max_memory: 2 * 1024 * 1024 * 1024,  // 2 GB
                max_fds: 512,
                max_tasks: 128,
                max_children: 8,
                io_bandwidth: 100 * 1024 * 1024,
                net_bandwidth: 100 * 1024 * 1024,
                priority_range: (0, 30),
                realtime_allowed: false,
                cpu_mask: 0xF,  // 4 CPUs
            },
            DomainType::Container => Self {
                max_cpu: 100,
                max_memory: 512 * 1024 * 1024,  // 512 MB
                max_fds: 256,
                max_tasks: 64,
                max_children: 2,
                io_bandwidth: 50 * 1024 * 1024,
                net_bandwidth: 50 * 1024 * 1024,
                priority_range: (0, 20),
                realtime_allowed: false,
                cpu_mask: 0x3,  // 2 CPUs
            },
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::default_for(DomainType::User)
    }
}

// =============================================================================
// Task Security Context
// =============================================================================

/// Security context for a task
#[derive(Debug, Clone)]
pub struct TaskSecurityContext {
    /// Task ID
    pub task_id: TaskId,
    /// Domain membership
    pub domain: DomainId,
    /// Effective capabilities
    pub effective_caps: CapabilitySet,
    /// Permitted capabilities (can be enabled)
    pub permitted_caps: CapabilitySet,
    /// Inheritable capabilities (passed to children)
    pub inheritable_caps: CapabilitySet,
    /// Bounding set (maximum ever)
    pub bounding_caps: CapabilitySet,
    /// Isolation level override
    pub isolation_override: Option<IsolationLevel>,
    /// Security flags
    pub flags: SecurityFlags,
    /// Audit level
    pub audit_level: AuditLevel,
    /// Creation timestamp
    pub created: Nanoseconds,
    /// Last capability check
    pub last_check: Nanoseconds,
}

bitflags! {
    /// Security flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SecurityFlags: u32 {
        /// Capabilities are locked (can't change)
        const CAPS_LOCKED = 1 << 0;
        /// Domain is locked
        const DOMAIN_LOCKED = 1 << 1;
        /// No new privileges
        const NO_NEW_PRIVS = 1 << 2;
        /// Seccomp enabled
        const SECCOMP = 1 << 3;
        /// Being traced
        const TRACED = 1 << 4;
        /// Dumpable (core dump allowed)
        const DUMPABLE = 1 << 5;
        /// Setuid task
        const SETUID = 1 << 6;
        /// Privileged operation in progress
        const PRIV_OPERATION = 1 << 7;
    }
}

/// Audit level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditLevel {
    /// No auditing
    None,
    /// Audit failures only
    Failures,
    /// Audit all security checks
    All,
    /// Verbose auditing
    Verbose,
}

impl TaskSecurityContext {
    /// Create new security context
    pub fn new(task_id: TaskId, domain_id: DomainId, caps: CapabilitySet) -> Self {
        Self {
            task_id,
            domain: domain_id,
            effective_caps: caps.clone(),
            permitted_caps: caps.clone(),
            inheritable_caps: CapabilitySet::empty(),
            bounding_caps: caps,
            isolation_override: None,
            flags: SecurityFlags::DUMPABLE,
            audit_level: AuditLevel::Failures,
            created: Nanoseconds::zero(),
            last_check: Nanoseconds::zero(),
        }
    }
    
    /// Check if has capability
    pub fn has_cap(&self, cap: Capability) -> bool {
        self.effective_caps.has(cap)
    }
    
    /// Enable capability (from permitted)
    pub fn enable_cap(&mut self, cap: Capability) -> bool {
        if self.flags.contains(SecurityFlags::CAPS_LOCKED) {
            return false;
        }
        if self.permitted_caps.has(cap) {
            self.effective_caps.add(cap);
            true
        } else {
            false
        }
    }
    
    /// Disable capability
    pub fn disable_cap(&mut self, cap: Capability) {
        self.effective_caps.remove(cap);
    }
    
    /// Drop capability permanently
    pub fn drop_cap(&mut self, cap: Capability) {
        self.effective_caps.remove(cap);
        self.permitted_caps.remove(cap);
        self.bounding_caps.remove(cap);
    }
    
    /// Lock capabilities
    pub fn lock_caps(&mut self) {
        self.flags.insert(SecurityFlags::CAPS_LOCKED);
    }
    
    /// Set no new privileges
    pub fn set_no_new_privs(&mut self) {
        self.flags.insert(SecurityFlags::NO_NEW_PRIVS);
    }
}

// =============================================================================
// Security Manager
// =============================================================================

/// The main security manager
pub struct SecurityManager {
    /// All domains
    domains: RwLock<BTreeMap<DomainId, SecurityDomain>>,
    /// Task security contexts
    contexts: RwLock<BTreeMap<TaskId, TaskSecurityContext>>,
    /// Next domain ID
    next_domain_id: AtomicU64,
    /// Default isolation level
    default_isolation: RwLock<IsolationLevel>,
    /// Security statistics
    stats: SecurityStats,
    /// Security enabled
    enabled: AtomicBool,
}

/// Security statistics
#[derive(Debug, Default)]
struct SecurityStats {
    cap_checks: AtomicU64,
    cap_grants: AtomicU64,
    cap_denials: AtomicU64,
    domain_creates: AtomicU64,
    domain_destroys: AtomicU64,
    violations: AtomicU64,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new() -> Self {
        let mut manager = Self {
            domains: RwLock::new(BTreeMap::new()),
            contexts: RwLock::new(BTreeMap::new()),
            next_domain_id: AtomicU64::new(2),  // 0=kernel, 1=system
            default_isolation: RwLock::new(IsolationLevel::Standard),
            stats: SecurityStats::default(),
            enabled: AtomicBool::new(true),
        };
        
        // Create kernel and system domains
        manager.create_builtin_domains();
        
        manager
    }
    
    /// Create built-in domains
    fn create_builtin_domains(&mut self) {
        let kernel = SecurityDomain::new(DomainId::KERNEL, "kernel", DomainType::Kernel);
        let mut system = SecurityDomain::new(DomainId::SYSTEM, "system", DomainType::System);
        system.parent = Some(DomainId::KERNEL);
        
        let mut domains = self.domains.write();
        domains.insert(DomainId::KERNEL, kernel);
        domains.insert(DomainId::SYSTEM, system);
    }
    
    /// Create new domain
    pub fn create_domain(&self, name: &str, domain_type: DomainType, parent: DomainId) -> DISResult<DomainId> {
        // Check parent exists
        let domains = self.domains.read();
        if !domains.contains_key(&parent) {
            return Err(DISError::DomainNotFound(parent.id()));
        }
        
        // Check parent can create children
        let parent_domain = &domains[&parent];
        if !parent_domain.flags.contains(DomainFlags::CAN_CREATE_CHILDREN) {
            return Err(DISError::PermissionDenied);
        }
        
        drop(domains);
        
        let id = DomainId::new(self.next_domain_id.fetch_add(1, Ordering::Relaxed));
        let mut domain = SecurityDomain::new(id, name, domain_type);
        domain.parent = Some(parent);
        
        // Capabilities can't exceed parent's max
        let domains = self.domains.read();
        let parent_domain = &domains[&parent];
        domain.max_caps = domain.max_caps.intersection(&parent_domain.max_caps);
        domain.default_caps = domain.default_caps.intersection(&domain.max_caps);
        drop(domains);
        
        self.domains.write().insert(id, domain);
        
        // Update parent's children
        if let Some(parent_domain) = self.domains.write().get_mut(&parent) {
            parent_domain.children.insert(id);
        }
        
        self.stats.domain_creates.fetch_add(1, Ordering::Relaxed);
        
        Ok(id)
    }
    
    /// Destroy domain
    pub fn destroy_domain(&self, domain_id: DomainId) -> DISResult<()> {
        if domain_id == DomainId::KERNEL || domain_id == DomainId::SYSTEM {
            return Err(DISError::PermissionDenied);
        }
        
        let mut domains = self.domains.write();
        
        if let Some(domain) = domains.get(&domain_id) {
            // Can't destroy if has tasks or children
            if !domain.tasks.is_empty() || !domain.children.is_empty() {
                return Err(DISError::DomainNotEmpty);
            }
            
            // Remove from parent
            if let Some(parent_id) = domain.parent {
                if let Some(parent) = domains.get_mut(&parent_id) {
                    parent.children.remove(&domain_id);
                }
            }
            
            domains.remove(&domain_id);
            self.stats.domain_destroys.fetch_add(1, Ordering::Relaxed);
            
            Ok(())
        } else {
            Err(DISError::DomainNotFound(domain_id.id()))
        }
    }
    
    /// Get domain
    pub fn get_domain(&self, domain_id: DomainId) -> Option<SecurityDomain> {
        self.domains.read().get(&domain_id).cloned()
    }
    
    /// Register task with security context
    pub fn register_task(&self, task_id: TaskId, domain_id: DomainId) -> DISResult<()> {
        let mut domains = self.domains.write();
        
        if let Some(domain) = domains.get_mut(&domain_id) {
            let context = TaskSecurityContext::new(
                task_id,
                domain_id,
                domain.default_caps.clone(),
            );
            
            domain.add_task(task_id);
            self.contexts.write().insert(task_id, context);
            
            Ok(())
        } else {
            Err(DISError::DomainNotFound(domain_id.id()))
        }
    }
    
    /// Unregister task
    pub fn unregister_task(&self, task_id: TaskId) -> DISResult<()> {
        if let Some(context) = self.contexts.write().remove(&task_id) {
            if let Some(domain) = self.domains.write().get_mut(&context.domain) {
                domain.remove_task(task_id);
            }
        }
        Ok(())
    }
    
    /// Get task security context
    pub fn get_context(&self, task_id: TaskId) -> Option<TaskSecurityContext> {
        self.contexts.read().get(&task_id).cloned()
    }
    
    /// Check capability
    pub fn check_cap(&self, task_id: TaskId, cap: Capability) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return true;
        }
        
        self.stats.cap_checks.fetch_add(1, Ordering::Relaxed);
        
        if let Some(context) = self.contexts.read().get(&task_id) {
            let has_cap = context.has_cap(cap);
            
            if has_cap {
                self.stats.cap_grants.fetch_add(1, Ordering::Relaxed);
            } else {
                self.stats.cap_denials.fetch_add(1, Ordering::Relaxed);
            }
            
            has_cap
        } else {
            false
        }
    }
    
    /// Grant capability to task
    pub fn grant_cap(&self, task_id: TaskId, cap: Capability) -> DISResult<()> {
        let mut contexts = self.contexts.write();
        
        if let Some(context) = contexts.get_mut(&task_id) {
            // Check domain allows this capability
            let domains = self.domains.read();
            if let Some(domain) = domains.get(&context.domain) {
                if !domain.max_caps.has(cap) {
                    return Err(DISError::CapabilityExceedsMax);
                }
            }
            
            context.permitted_caps.add(cap);
            context.effective_caps.add(cap);
            
            Ok(())
        } else {
            Err(DISError::TaskNotFound(task_id))
        }
    }
    
    /// Revoke capability from task
    pub fn revoke_cap(&self, task_id: TaskId, cap: Capability) -> DISResult<()> {
        if let Some(context) = self.contexts.write().get_mut(&task_id) {
            context.drop_cap(cap);
            Ok(())
        } else {
            Err(DISError::TaskNotFound(task_id))
        }
    }
    
    /// Get isolation level for task
    pub fn get_isolation(&self, task_id: TaskId) -> IsolationLevel {
        if let Some(context) = self.contexts.read().get(&task_id) {
            if let Some(override_level) = context.isolation_override {
                return override_level;
            }
            
            if let Some(domain) = self.domains.read().get(&context.domain) {
                return domain.isolation;
            }
        }
        
        *self.default_isolation.read()
    }
    
    /// Record security violation
    pub fn record_violation(&self, task_id: TaskId, cap: Capability) {
        self.stats.violations.fetch_add(1, Ordering::Relaxed);
        
        if let Some(context) = self.contexts.read().get(&task_id) {
            let domain_id = context.domain;
            if let Some(domain) = self.domains.write().get_mut(&domain_id) {
                domain.stats.violations += 1;
            }
        }
    }
    
    /// Get security statistics
    pub fn statistics(&self) -> SecurityStatistics {
        SecurityStatistics {
            cap_checks: self.stats.cap_checks.load(Ordering::Relaxed),
            cap_grants: self.stats.cap_grants.load(Ordering::Relaxed),
            cap_denials: self.stats.cap_denials.load(Ordering::Relaxed),
            domain_count: self.domains.read().len() as u64,
            context_count: self.contexts.read().len() as u64,
            violations: self.stats.violations.load(Ordering::Relaxed),
        }
    }
    
    /// Enable/disable security
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }
}

/// Security statistics
#[derive(Debug, Clone)]
pub struct SecurityStatistics {
    pub cap_checks: u64,
    pub cap_grants: u64,
    pub cap_denials: u64,
    pub domain_count: u64,
    pub context_count: u64,
    pub violations: u64,
}

impl Default for SecurityManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_capability_set() {
        let mut caps = CapabilitySet::empty();
        assert!(!caps.has(Capability::UseCpu));
        
        caps.add(Capability::UseCpu);
        assert!(caps.has(Capability::UseCpu));
        
        caps.remove(Capability::UseCpu);
        assert!(!caps.has(Capability::UseCpu));
    }
    
    #[test]
    fn test_domain_creation() {
        let manager = SecurityManager::new();
        
        let domain_id = manager.create_domain("test", DomainType::User, DomainId::SYSTEM).unwrap();
        
        let domain = manager.get_domain(domain_id).unwrap();
        assert_eq!(domain.name, "test");
        assert_eq!(domain.domain_type, DomainType::User);
    }
    
    #[test]
    fn test_capability_check() {
        let manager = SecurityManager::new();
        
        let domain_id = manager.create_domain("test", DomainType::User, DomainId::SYSTEM).unwrap();
        let task_id = TaskId::new(1);
        
        manager.register_task(task_id, domain_id).unwrap();
        
        // User domains have UseCpu by default
        assert!(manager.check_cap(task_id, Capability::UseCpu));
        // But not LoadModule
        assert!(!manager.check_cap(task_id, Capability::LoadModule));
    }
}
