//! # Helix Kernel Core
//!
//! The kernel core is the minimal orchestration layer that coordinates
//! all subsystems and modules. It is designed to be as small as possible
//! while providing the essential coordination services.
//!
//! ## Philosophy
//!
//! The core kernel is **policy-free**. It provides mechanisms, not policies.
//! All policy decisions are delegated to modules that can be replaced.
//!
//! ## Components
//!
//! - **Orchestrator**: Main coordination point
//! - **Lifecycle Manager**: Boot, shutdown, suspend/resume
//! - **Capability Broker**: Distribute and validate capabilities
//! - **Resource Broker**: Coordinate resource allocation
//! - **Syscall Gateway**: Entry point for system calls
//! - **Interrupt Router**: Dispatch interrupts to handlers
//!
//! ## Trusted Computing Base
//!
//! The core is part of the TCB and must be minimal and auditable.

#![no_std]
#![feature(negative_impls)]
#![feature(never_type)]
#![feature(allocator_api)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

pub mod debug;
pub mod hotreload;
pub mod interrupts;
pub mod ipc;
pub mod orchestrator;
pub mod selfheal;
pub mod syscall;

use core::sync::atomic::{AtomicU32, Ordering};

/// Kernel version information
pub const KERNEL_VERSION: KernelVersion = KernelVersion {
    major: 0,
    minor: 1,
    patch: 0,
    suffix: "alpha",
};

/// Kernel version structure
#[derive(Debug, Clone, Copy)]
pub struct KernelVersion {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
    /// Patch version
    pub patch: u16,
    /// Version suffix (e.g., "alpha", "beta", "rc1")
    pub suffix: &'static str,
}

/// Kernel state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum KernelState {
    /// Kernel is in early boot phase
    EarlyBoot    = 0,
    /// Kernel is initializing subsystems
    Initializing = 1,
    /// Kernel is running normally
    Running      = 2,
    /// Kernel is shutting down
    ShuttingDown = 3,
    /// Kernel is in panic state
    Panic        = 4,
    /// Kernel is suspended
    Suspended    = 5,
}

impl From<u32> for KernelState {
    fn from(value: u32) -> Self {
        match value {
            0 => KernelState::EarlyBoot,
            1 => KernelState::Initializing,
            2 => KernelState::Running,
            3 => KernelState::ShuttingDown,
            4 => KernelState::Panic,
            5 => KernelState::Suspended,
            _ => KernelState::Panic,
        }
    }
}

static KERNEL_STATE: AtomicU32 = AtomicU32::new(KernelState::EarlyBoot as u32);

/// Get the current kernel state
pub fn kernel_state() -> KernelState {
    KernelState::from(KERNEL_STATE.load(Ordering::SeqCst))
}

/// Set the kernel state
///
/// # Safety
/// This should only be called by the lifecycle manager.
pub(crate) fn set_kernel_state(state: KernelState) {
    KERNEL_STATE.store(state as u32, Ordering::SeqCst);
}

/// Result type for kernel operations
pub type KernelResult<T> = Result<T, KernelError>;

/// Kernel error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    /// Operation not permitted
    NotPermitted,
    /// Resource not found
    NotFound,
    /// Resource already exists
    AlreadyExists,
    /// Invalid argument
    InvalidArgument,
    /// Operation would block
    WouldBlock,
    /// Resource is busy
    Busy,
    /// Out of memory
    OutOfMemory,
    /// Operation timed out
    Timeout,
    /// Operation was interrupted
    Interrupted,
    /// I/O error
    IoError,
    /// Not implemented
    NotImplemented,
    /// Internal error
    Internal,
    /// Capability error
    CapabilityError(CapabilityError),
    /// Subsystem error
    SubsystemError(&'static str),
}

/// Capability-related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityError {
    /// Capability not found
    NotFound,
    /// Capability expired
    Expired,
    /// Insufficient rights
    InsufficientRights,
    /// Capability revoked
    Revoked,
    /// Invalid capability
    Invalid,
}

/// Core trait that all kernel components must implement
pub trait KernelComponent: Send + Sync {
    /// Get the component name
    fn name(&self) -> &'static str;

    /// Get the component version
    fn version(&self) -> &'static str;

    /// Initialize the component
    fn init(&mut self) -> KernelResult<()>;

    /// Shutdown the component
    fn shutdown(&mut self) -> KernelResult<()>;

    /// Check if the component is healthy
    fn health_check(&self) -> bool {
        true
    }

    /// Get component statistics
    fn stats(&self) -> Option<ComponentStats> {
        None
    }
}

/// Statistics for a kernel component
#[derive(Debug, Clone, Default)]
pub struct ComponentStats {
    /// Number of operations performed
    pub operations: u64,
    /// Number of errors
    pub errors: u64,
    /// Current load (0-100)
    pub load: u8,
    /// Memory usage in bytes
    pub memory_usage: usize,
}

/// Event types for kernel-wide events
#[derive(Debug, Clone)]
pub enum KernelEvent {
    /// A module was loaded
    ModuleLoaded { name: alloc::string::String },
    /// A module was unloaded
    ModuleUnloaded { name: alloc::string::String },
    /// System state changed
    StateChanged { old: KernelState, new: KernelState },
    /// Memory pressure detected
    MemoryPressure { level: MemoryPressureLevel },
    /// CPU hotplug event
    CpuHotplug { cpu: usize, online: bool },
    /// Timer tick
    TimerTick { timestamp: u64 },
    /// Custom event
    Custom { id: u64, data: alloc::vec::Vec<u8> },
}

/// Memory pressure levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    /// Normal memory usage
    Normal,
    /// Low memory warning
    Low,
    /// Critical memory shortage
    Critical,
    /// Out of memory
    OutOfMemory,
}

/// Kernel event listener trait
pub trait KernelEventListener: Send + Sync {
    /// Handle a kernel event
    fn on_event(&self, event: &KernelEvent);

    /// Get the events this listener is interested in
    fn subscribed_events(&self) -> &[KernelEventType];
}

/// Types of kernel events (for subscription)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelEventType {
    /// Module events
    Module,
    /// State change events
    StateChange,
    /// Memory events
    Memory,
    /// CPU events
    Cpu,
    /// Timer events
    Timer,
    /// All custom events
    Custom,
    /// All events
    All,
}
