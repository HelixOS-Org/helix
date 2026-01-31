//! # Subsystem Trait and Types
//!
//! This module defines the core `Subsystem` trait that all kernel subsystems
//! must implement, along with supporting types for identification, state
//! management, and lifecycle control.
//!
//! ## Subsystem Lifecycle
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                         SUBSYSTEM LIFECYCLE                                  │
//! │                                                                              │
//! │  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
//! │  │Registered│───▶│Validating│───▶│  Ready   │───▶│Initializ.│───▶│  Active  │
//! │  └──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
//! │       │              │               │               │               │
//! │       ▼              ▼               ▼               ▼               ▼
//! │  validate()     check_deps()    wait_deps()    init()          run()
//! │                                                                      │
//! │                                                                      ▼
//! │  ┌──────────┐    ┌──────────┐    ┌──────────┐                   ┌──────────┐
//! │  │ Removed  │◀───│ Cleaned  │◀───│ Stopped  │◀──────────────────│Suspending│
//! │  └──────────┘    └──────────┘    └──────────┘                   └──────────┘
//! │       ▲              ▲               ▲                               │
//! │       │              │               │                               ▼
//! │  unregister()   cleanup()       shutdown()                      suspend()
//! │                                                                      │
//! │                                      ┌──────────┐                    │
//! │                                      │ Suspended │◀──────────────────┘
//! │                                      └──────────┘
//! │                                           │
//! │                                           ▼
//! │                                      resume() ───▶ Active
//! │                                                                              │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Subsystem Properties
//!
//! | Property | Description |
//! |----------|-------------|
//! | ID | Unique identifier (compile-time or runtime) |
//! | Name | Human-readable name |
//! | Phase | Which init phase this subsystem belongs to |
//! | Priority | Order within phase (higher = earlier) |
//! | Dependencies | Other subsystems required before this one |
//! | Provides | Capabilities this subsystem provides |

use core::cmp::Ordering as CmpOrdering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::sync::atomic::{AtomicU32, Ordering};

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};

extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// SUBSYSTEM ID
// =============================================================================

/// Unique identifier for a subsystem
///
/// IDs can be generated at compile time from the subsystem name or assigned
/// at runtime for dynamic subsystems.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct SubsystemId(pub u64);

impl SubsystemId {
    /// Create ID from name at compile time
    pub const fn from_name(name: &str) -> Self {
        Self(const_fnv1a_hash(name.as_bytes()))
    }

    /// Create ID from components
    pub const fn from_parts(phase: u8, category: u8, instance: u16, unique: u32) -> Self {
        let id = ((phase as u64) << 56)
            | ((category as u64) << 48)
            | ((instance as u64) << 32)
            | (unique as u64);
        Self(id)
    }

    /// Get raw value
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Invalid/null ID
    pub const INVALID: Self = Self(0);

    /// Check if valid
    pub const fn is_valid(&self) -> bool {
        self.0 != 0
    }
}

impl fmt::Debug for SubsystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SubsystemId(0x{:016x})", self.0)
    }
}

impl fmt::Display for SubsystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:016x}", self.0)
    }
}

impl Default for SubsystemId {
    fn default() -> Self {
        Self::INVALID
    }
}

/// Compile-time FNV-1a hash
const fn const_fnv1a_hash(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    let mut hash = FNV_OFFSET;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

// =============================================================================
// SUBSYSTEM STATE
// =============================================================================

/// Current state of a subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum SubsystemState {
    /// Subsystem is registered but not validated
    Registered   = 0,

    /// Subsystem is being validated
    Validating   = 1,

    /// Subsystem passed validation, waiting for dependencies
    Ready        = 2,

    /// Subsystem is being initialized
    Initializing = 3,

    /// Subsystem is fully active
    Active       = 4,

    /// Subsystem is being suspended
    Suspending   = 5,

    /// Subsystem is suspended
    Suspended    = 6,

    /// Subsystem is resuming
    Resuming     = 7,

    /// Subsystem is shutting down
    ShuttingDown = 8,

    /// Subsystem has stopped
    Stopped      = 9,

    /// Subsystem is being cleaned up
    Cleaning     = 10,

    /// Subsystem has been cleaned up
    Cleaned      = 11,

    /// Subsystem failed initialization
    Failed       = 12,

    /// Subsystem was removed from registry
    Removed      = 13,
}

impl SubsystemState {
    /// Check if subsystem can be initialized
    pub fn can_initialize(&self) -> bool {
        matches!(self, SubsystemState::Ready)
    }

    /// Check if subsystem is operational
    pub fn is_operational(&self) -> bool {
        matches!(self, SubsystemState::Active | SubsystemState::Suspended)
    }

    /// Check if subsystem has completed (success or failure)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            SubsystemState::Active
                | SubsystemState::Failed
                | SubsystemState::Removed
                | SubsystemState::Cleaned
        )
    }

    /// Check if subsystem can be shut down
    pub fn can_shutdown(&self) -> bool {
        matches!(
            self,
            SubsystemState::Active | SubsystemState::Suspended | SubsystemState::Failed
        )
    }

    /// Get state name
    pub const fn name(&self) -> &'static str {
        match self {
            SubsystemState::Registered => "Registered",
            SubsystemState::Validating => "Validating",
            SubsystemState::Ready => "Ready",
            SubsystemState::Initializing => "Initializing",
            SubsystemState::Active => "Active",
            SubsystemState::Suspending => "Suspending",
            SubsystemState::Suspended => "Suspended",
            SubsystemState::Resuming => "Resuming",
            SubsystemState::ShuttingDown => "ShuttingDown",
            SubsystemState::Stopped => "Stopped",
            SubsystemState::Cleaning => "Cleaning",
            SubsystemState::Cleaned => "Cleaned",
            SubsystemState::Failed => "Failed",
            SubsystemState::Removed => "Removed",
        }
    }
}

impl fmt::Display for SubsystemState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Atomic subsystem state for thread-safe access
pub struct AtomicSubsystemState(AtomicU32);

impl AtomicSubsystemState {
    /// Create new atomic state
    pub const fn new(state: SubsystemState) -> Self {
        Self(AtomicU32::new(state as u32))
    }

    /// Load current state
    pub fn load(&self) -> SubsystemState {
        let val = self.0.load(Ordering::SeqCst);
        // Safety: SubsystemState repr(u32) and we only store valid variants
        unsafe { core::mem::transmute(val) }
    }

    /// Store new state
    pub fn store(&self, state: SubsystemState) {
        self.0.store(state as u32, Ordering::SeqCst);
    }

    /// Compare and exchange
    pub fn compare_exchange(
        &self,
        current: SubsystemState,
        new: SubsystemState,
    ) -> Result<SubsystemState, SubsystemState> {
        self.0
            .compare_exchange(
                current as u32,
                new as u32,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .map(|v| unsafe { core::mem::transmute(v) })
            .map_err(|v| unsafe { core::mem::transmute(v) })
    }

    /// Transition state if allowed
    pub fn transition(&self, from: SubsystemState, to: SubsystemState) -> bool {
        self.compare_exchange(from, to).is_ok()
    }
}

impl Default for AtomicSubsystemState {
    fn default() -> Self {
        Self::new(SubsystemState::Registered)
    }
}

// =============================================================================
// DEPENDENCY SPECIFICATION
// =============================================================================

/// How a dependency should be resolved
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    /// Hard requirement - must be initialized before this subsystem
    Required,

    /// Optional - use if available, but don't fail if not
    Optional,

    /// Weak - only order constraint, no actual dependency
    Weak,

    /// Conflict - cannot coexist with this subsystem
    Conflict,
}

/// Dependency on another subsystem
#[derive(Debug, Clone)]
pub struct Dependency {
    /// ID of the dependency
    pub id: SubsystemId,

    /// Name of the dependency (for diagnostics)
    pub name: &'static str,

    /// Kind of dependency
    pub kind: DependencyKind,

    /// Minimum version (if applicable)
    pub min_version: Option<(u16, u16, u16)>,

    /// Maximum version (if applicable)
    pub max_version: Option<(u16, u16, u16)>,
}

impl Dependency {
    /// Create required dependency
    pub const fn required(name: &'static str) -> Self {
        Self {
            id: SubsystemId::from_name(name),
            name,
            kind: DependencyKind::Required,
            min_version: None,
            max_version: None,
        }
    }

    /// Create optional dependency
    pub const fn optional(name: &'static str) -> Self {
        Self {
            id: SubsystemId::from_name(name),
            name,
            kind: DependencyKind::Optional,
            min_version: None,
            max_version: None,
        }
    }

    /// Create weak dependency
    pub const fn weak(name: &'static str) -> Self {
        Self {
            id: SubsystemId::from_name(name),
            name,
            kind: DependencyKind::Weak,
            min_version: None,
            max_version: None,
        }
    }

    /// Create conflict
    pub const fn conflict(name: &'static str) -> Self {
        Self {
            id: SubsystemId::from_name(name),
            name,
            kind: DependencyKind::Conflict,
            min_version: None,
            max_version: None,
        }
    }

    /// Set version range
    pub const fn with_version(mut self, min: (u16, u16, u16), max: (u16, u16, u16)) -> Self {
        self.min_version = Some(min);
        self.max_version = Some(max);
        self
    }
}

// =============================================================================
// SUBSYSTEM INFO
// =============================================================================

/// Static information about a subsystem
#[derive(Debug, Clone)]
pub struct SubsystemInfo {
    /// Unique identifier
    pub id: SubsystemId,

    /// Human-readable name
    pub name: &'static str,

    /// Description
    pub description: &'static str,

    /// Version (major, minor, patch)
    pub version: (u16, u16, u16),

    /// Initialization phase
    pub phase: InitPhase,

    /// Priority within phase (higher = earlier)
    pub priority: i32,

    /// Dependencies on other subsystems
    pub dependencies: &'static [Dependency],

    /// Capabilities this subsystem provides
    pub provides: PhaseCapabilities,

    /// Capabilities this subsystem requires
    pub requires: PhaseCapabilities,

    /// Whether subsystem is essential (cannot fail)
    pub essential: bool,

    /// Whether subsystem supports hot-reload
    pub hot_reloadable: bool,

    /// Whether subsystem can be suspended
    pub suspendable: bool,

    /// Estimated initialization time in microseconds
    pub estimated_init_us: u64,

    /// Maximum allowed initialization time in microseconds
    pub timeout_us: u64,

    /// Author/maintainer
    pub author: &'static str,

    /// License
    pub license: &'static str,
}

impl SubsystemInfo {
    /// Create minimal info
    pub const fn new(name: &'static str, phase: InitPhase) -> Self {
        Self {
            id: SubsystemId::from_name(name),
            name,
            description: "",
            version: (0, 1, 0),
            phase,
            priority: 0,
            dependencies: &[],
            provides: PhaseCapabilities::empty(),
            requires: PhaseCapabilities::empty(),
            essential: false,
            hot_reloadable: false,
            suspendable: true,
            estimated_init_us: 1000,
            timeout_us: 10_000_000, // 10 seconds
            author: "",
            license: "",
        }
    }

    /// Builder: set description
    pub const fn with_description(mut self, desc: &'static str) -> Self {
        self.description = desc;
        self
    }

    /// Builder: set version
    pub const fn with_version(mut self, major: u16, minor: u16, patch: u16) -> Self {
        self.version = (major, minor, patch);
        self
    }

    /// Builder: set priority
    pub const fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Builder: set dependencies
    pub const fn with_dependencies(mut self, deps: &'static [Dependency]) -> Self {
        self.dependencies = deps;
        self
    }

    /// Builder: set provides
    pub const fn provides(mut self, caps: PhaseCapabilities) -> Self {
        self.provides = caps;
        self
    }

    /// Builder: set requires
    pub const fn requires(mut self, caps: PhaseCapabilities) -> Self {
        self.requires = caps;
        self
    }

    /// Builder: mark as essential
    pub const fn essential(mut self) -> Self {
        self.essential = true;
        self
    }

    /// Builder: mark as hot-reloadable
    pub const fn hot_reloadable(mut self) -> Self {
        self.hot_reloadable = true;
        self
    }

    /// Builder: set timeout
    pub const fn with_timeout(mut self, timeout_us: u64) -> Self {
        self.timeout_us = timeout_us;
        self
    }

    /// Check if version satisfies constraint
    pub fn version_satisfies(&self, min: (u16, u16, u16), max: (u16, u16, u16)) -> bool {
        self.version >= min && self.version <= max
    }

    /// Get version string
    pub fn version_string(&self) -> String {
        alloc::format!("{}.{}.{}", self.version.0, self.version.1, self.version.2)
    }
}

impl PartialEq for SubsystemInfo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for SubsystemInfo {}

impl Hash for SubsystemInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for SubsystemInfo {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for SubsystemInfo {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        // First by phase, then by priority (reversed for higher = earlier)
        self.phase
            .cmp(&other.phase)
            .then_with(|| other.priority.cmp(&self.priority))
    }
}

// =============================================================================
// SUBSYSTEM TRAIT
// =============================================================================

/// Core trait that all subsystems must implement
///
/// This trait defines the lifecycle methods for a kernel subsystem.
/// Subsystems are the building blocks of the kernel, each responsible
/// for a specific piece of functionality.
pub trait Subsystem: Send + Sync {
    // -------------------------------------------------------------------------
    // Required Methods
    // -------------------------------------------------------------------------

    /// Get subsystem information
    fn info(&self) -> &SubsystemInfo;

    /// Initialize the subsystem
    ///
    /// Called during the init phase when all dependencies are satisfied.
    /// The context provides access to kernel services available at this phase.
    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()>;

    // -------------------------------------------------------------------------
    // Optional Lifecycle Methods
    // -------------------------------------------------------------------------

    /// Validate subsystem configuration
    ///
    /// Called before initialization to check that the subsystem can be
    /// initialized with the current configuration.
    fn validate(&self, _ctx: &InitContext) -> InitResult<()> {
        Ok(())
    }

    /// Called after all subsystems in the phase are initialized
    fn post_phase_init(&mut self, _ctx: &mut InitContext) -> InitResult<()> {
        Ok(())
    }

    /// Suspend the subsystem
    ///
    /// Called when the system is suspending or for hot-reload preparation.
    fn suspend(&mut self, _ctx: &mut InitContext) -> InitResult<()> {
        if !self.info().suspendable {
            return Err(InitError::new(
                ErrorKind::NotSupported,
                "Subsystem does not support suspend",
            ));
        }
        Ok(())
    }

    /// Resume the subsystem
    fn resume(&mut self, _ctx: &mut InitContext) -> InitResult<()> {
        Ok(())
    }

    /// Shutdown the subsystem
    ///
    /// Called in reverse order during kernel shutdown.
    fn shutdown(&mut self, _ctx: &mut InitContext) -> InitResult<()> {
        Ok(())
    }

    /// Clean up resources
    ///
    /// Called after shutdown to release all resources.
    fn cleanup(&mut self, _ctx: &mut InitContext) -> InitResult<()> {
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Health and Diagnostics
    // -------------------------------------------------------------------------

    /// Check subsystem health
    fn health_check(&self) -> InitResult<HealthStatus> {
        Ok(HealthStatus::Healthy)
    }

    /// Get subsystem statistics
    fn stats(&self) -> SubsystemStats {
        SubsystemStats::default()
    }

    /// Perform self-test
    fn self_test(&self) -> InitResult<()> {
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Hot Reload
    // -------------------------------------------------------------------------

    /// Prepare for hot reload
    ///
    /// Save any state that needs to be preserved across reload.
    #[cfg(feature = "hot_reload")]
    fn prepare_hot_reload(&mut self, _ctx: &mut InitContext) -> InitResult<HotReloadState> {
        if !self.info().hot_reloadable {
            return Err(InitError::new(
                ErrorKind::NotSupported,
                "Subsystem does not support hot reload",
            ));
        }
        Ok(HotReloadState::new())
    }

    /// Complete hot reload
    ///
    /// Restore state after reload.
    #[cfg(feature = "hot_reload")]
    fn complete_hot_reload(
        &mut self,
        _ctx: &mut InitContext,
        _state: HotReloadState,
    ) -> InitResult<()> {
        Ok(())
    }
}

// =============================================================================
// HEALTH STATUS
// =============================================================================

/// Health status of a subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Subsystem is healthy
    Healthy,

    /// Subsystem has warnings but is functional
    Degraded,

    /// Subsystem is unhealthy
    Unhealthy,

    /// Health status unknown
    Unknown,
}

impl HealthStatus {
    /// Check if operational (Healthy or Degraded)
    pub fn is_operational(&self) -> bool {
        matches!(self, HealthStatus::Healthy | HealthStatus::Degraded)
    }
}

// =============================================================================
// SUBSYSTEM STATS
// =============================================================================

/// Runtime statistics for a subsystem
#[derive(Debug, Clone, Default)]
pub struct SubsystemStats {
    /// Initialization time in microseconds
    pub init_time_us: u64,

    /// Total operations performed
    pub operations: u64,

    /// Failed operations
    pub failures: u64,

    /// Memory usage in bytes
    pub memory_bytes: u64,

    /// CPU time in microseconds
    pub cpu_time_us: u64,

    /// Last health check timestamp
    pub last_health_check: u64,

    /// Custom metrics
    pub custom: [(u64, u64); 8],
}

impl SubsystemStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            init_time_us: 0,
            operations: 0,
            failures: 0,
            memory_bytes: 0,
            cpu_time_us: 0,
            last_health_check: 0,
            custom: [(0, 0); 8],
        }
    }

    /// Set custom metric
    pub fn set_custom(&mut self, index: usize, key: u64, value: u64) {
        if index < 8 {
            self.custom[index] = (key, value);
        }
    }

    /// Get failure rate
    pub fn failure_rate(&self) -> f64 {
        if self.operations == 0 {
            0.0
        } else {
            (self.failures as f64) / (self.operations as f64)
        }
    }
}

// =============================================================================
// HOT RELOAD STATE
// =============================================================================

/// State preserved during hot reload
#[cfg(feature = "hot_reload")]
#[derive(Debug)]
pub struct HotReloadState {
    /// Serialized state data
    data: Vec<u8>,

    /// State version for compatibility
    version: u32,

    /// Checksum for validation
    checksum: u64,
}

#[cfg(feature = "hot_reload")]
impl HotReloadState {
    /// Create empty state
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            version: 1,
            checksum: 0,
        }
    }

    /// Create state with data
    pub fn with_data(data: Vec<u8>) -> Self {
        let checksum = crate::subsystem::compute_checksum(&data);
        Self {
            data,
            version: 1,
            checksum,
        }
    }

    /// Get data
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Validate checksum
    pub fn validate(&self) -> bool {
        compute_checksum(&self.data) == self.checksum
    }
}

#[cfg(feature = "hot_reload")]
impl Default for HotReloadState {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple checksum computation
#[cfg(feature = "hot_reload")]
fn compute_checksum(data: &[u8]) -> u64 {
    let mut hash = 0u64;
    for (i, &byte) in data.iter().enumerate() {
        hash = hash.wrapping_add((byte as u64).wrapping_mul(i as u64 + 1));
    }
    hash
}

// =============================================================================
// SUBSYSTEM WRAPPER
// =============================================================================

/// Wrapper around a subsystem with runtime state
pub struct SubsystemWrapper {
    /// The subsystem implementation
    inner: Box<dyn Subsystem>,

    /// Current state
    state: AtomicSubsystemState,

    /// Initialization timestamp
    init_start: u64,

    /// Completion timestamp
    init_end: u64,

    /// Error (if failed)
    error: Option<InitError>,
}

impl SubsystemWrapper {
    /// Create new wrapper
    pub fn new(subsystem: Box<dyn Subsystem>) -> Self {
        Self {
            inner: subsystem,
            state: AtomicSubsystemState::new(SubsystemState::Registered),
            init_start: 0,
            init_end: 0,
            error: None,
        }
    }

    /// Get subsystem info
    pub fn info(&self) -> &SubsystemInfo {
        self.inner.info()
    }

    /// Get subsystem ID
    pub fn id(&self) -> SubsystemId {
        self.inner.info().id
    }

    /// Get current state
    pub fn state(&self) -> SubsystemState {
        self.state.load()
    }

    /// Get inner subsystem
    pub fn inner(&self) -> &dyn Subsystem {
        &*self.inner
    }

    /// Get mutable inner subsystem
    pub fn inner_mut(&mut self) -> &mut dyn Subsystem {
        &mut *self.inner
    }

    /// Try to initialize
    pub fn try_init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        // Transition from Ready to Initializing
        if !self
            .state
            .transition(SubsystemState::Ready, SubsystemState::Initializing)
        {
            let current = self.state.load();
            return Err(
                InitError::new(ErrorKind::InvalidState, "Subsystem not in Ready state")
                    .with_details(alloc::format!("Current state: {:?}", current)),
            );
        }

        self.init_start = crate::get_timestamp();

        // Run initialization
        match self.inner.init(ctx) {
            Ok(()) => {
                self.init_end = crate::get_timestamp();
                self.state.store(SubsystemState::Active);
                Ok(())
            },
            Err(e) => {
                self.init_end = crate::get_timestamp();
                self.state.store(SubsystemState::Failed);
                self.error = Some(InitError::new(e.kind(), e.message()));
                Err(e)
            },
        }
    }

    /// Try to shutdown
    pub fn try_shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        if !self.state.load().can_shutdown() {
            return Err(InitError::new(
                ErrorKind::InvalidState,
                "Subsystem cannot be shut down in current state",
            ));
        }

        self.state.store(SubsystemState::ShuttingDown);

        match self.inner.shutdown(ctx) {
            Ok(()) => {
                self.state.store(SubsystemState::Stopped);
                Ok(())
            },
            Err(e) => {
                self.state.store(SubsystemState::Failed);
                Err(e)
            },
        }
    }

    /// Get initialization duration in microseconds
    pub fn init_duration_us(&self) -> u64 {
        if self.init_end > self.init_start {
            self.init_end - self.init_start
        } else {
            0
        }
    }

    /// Get error if failed
    pub fn error(&self) -> Option<&InitError> {
        self.error.as_ref()
    }
}

// =============================================================================
// SUBSYSTEM ID CONSTANTS
// =============================================================================

/// Well-known subsystem IDs for core kernel components
pub mod well_known {
    use super::SubsystemId;

    // Boot phase
    pub const FIRMWARE: SubsystemId = SubsystemId::from_name("firmware");
    pub const BOOT_INFO: SubsystemId = SubsystemId::from_name("boot_info");
    pub const EARLY_CONSOLE: SubsystemId = SubsystemId::from_name("early_console");

    // Early phase
    pub const PMM: SubsystemId = SubsystemId::from_name("pmm");
    pub const VMM: SubsystemId = SubsystemId::from_name("vmm");
    pub const HEAP: SubsystemId = SubsystemId::from_name("heap");
    pub const CPU: SubsystemId = SubsystemId::from_name("cpu");
    pub const INTERRUPTS: SubsystemId = SubsystemId::from_name("interrupts");

    // Core phase
    pub const SCHEDULER: SubsystemId = SubsystemId::from_name("scheduler");
    pub const IPC: SubsystemId = SubsystemId::from_name("ipc");
    pub const TIMERS: SubsystemId = SubsystemId::from_name("timers");
    pub const SMP: SubsystemId = SubsystemId::from_name("smp");

    // Late phase
    pub const DRIVERS: SubsystemId = SubsystemId::from_name("drivers");
    pub const VFS: SubsystemId = SubsystemId::from_name("vfs");
    pub const NETWORK: SubsystemId = SubsystemId::from_name("network");
    pub const SECURITY: SubsystemId = SubsystemId::from_name("security");

    // Runtime phase
    pub const USERLAND: SubsystemId = SubsystemId::from_name("userland");
    pub const SERVICES: SubsystemId = SubsystemId::from_name("services");
    pub const DEBUG: SubsystemId = SubsystemId::from_name("debug");
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subsystem_id() {
        let id1 = SubsystemId::from_name("test");
        let id2 = SubsystemId::from_name("test");
        let id3 = SubsystemId::from_name("other");

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        assert!(id1.is_valid());
        assert!(!SubsystemId::INVALID.is_valid());
    }

    #[test]
    fn test_subsystem_state_transitions() {
        let state = AtomicSubsystemState::new(SubsystemState::Registered);

        assert_eq!(state.load(), SubsystemState::Registered);

        // Valid transition
        assert!(state.transition(SubsystemState::Registered, SubsystemState::Validating));
        assert_eq!(state.load(), SubsystemState::Validating);

        // Invalid transition (wrong current state)
        assert!(!state.transition(SubsystemState::Registered, SubsystemState::Ready));
        assert_eq!(state.load(), SubsystemState::Validating);
    }

    #[test]
    fn test_dependency() {
        let dep = Dependency::required("memory").with_version((1, 0, 0), (2, 0, 0));

        assert_eq!(dep.kind, DependencyKind::Required);
        assert_eq!(dep.min_version, Some((1, 0, 0)));
    }

    #[test]
    fn test_subsystem_info_ordering() {
        let info1 = SubsystemInfo::new("a", InitPhase::Boot).with_priority(10);
        let info2 = SubsystemInfo::new("b", InitPhase::Boot).with_priority(5);
        let info3 = SubsystemInfo::new("c", InitPhase::Early).with_priority(100);

        // Same phase: higher priority comes first
        assert!(info1 < info2);

        // Different phases: earlier phase comes first
        assert!(info1 < info3);
        assert!(info2 < info3);
    }

    #[test]
    fn test_well_known_ids() {
        assert!(well_known::PMM.is_valid());
        assert!(well_known::SCHEDULER.is_valid());
        assert_ne!(well_known::PMM, well_known::VMM);
    }
}
