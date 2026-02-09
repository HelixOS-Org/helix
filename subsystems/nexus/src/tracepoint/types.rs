//! Tracepoint Core Types
//!
//! Fundamental types for tracepoint management.

/// Tracepoint identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TracepointId(pub u64);

impl TracepointId {
    /// Create a new tracepoint ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Probe identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProbeId(pub u64);

impl ProbeId {
    /// Create a new probe ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Event identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u64);

impl EventId {
    /// Create a new event ID
    #[inline(always)]
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID value
    #[inline(always)]
    pub const fn raw(&self) -> u64 {
        self.0
    }
}

/// Tracepoint subsystem
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TracepointSubsystem {
    /// Scheduler subsystem
    Sched,
    /// Block I/O subsystem
    Block,
    /// Network subsystem
    Net,
    /// Memory management
    Mm,
    /// Filesystem
    Fs,
    /// IPC subsystem
    Ipc,
    /// Interrupt handling
    Irq,
    /// Timer subsystem
    Timer,
    /// Power management
    Power,
    /// Workqueue
    Workqueue,
    /// RCU subsystem
    Rcu,
    /// Signal handling
    Signal,
    /// Module loading
    Module,
    /// KVM/virtualization
    Kvm,
    /// Custom/user-defined
    Custom,
}

impl TracepointSubsystem {
    /// Get subsystem name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Sched => "sched",
            Self::Block => "block",
            Self::Net => "net",
            Self::Mm => "mm",
            Self::Fs => "fs",
            Self::Ipc => "ipc",
            Self::Irq => "irq",
            Self::Timer => "timer",
            Self::Power => "power",
            Self::Workqueue => "workqueue",
            Self::Rcu => "rcu",
            Self::Signal => "signal",
            Self::Module => "module",
            Self::Kvm => "kvm",
            Self::Custom => "custom",
        }
    }

    /// All subsystems
    pub fn all() -> &'static [TracepointSubsystem] {
        &[
            Self::Sched,
            Self::Block,
            Self::Net,
            Self::Mm,
            Self::Fs,
            Self::Ipc,
            Self::Irq,
            Self::Timer,
            Self::Power,
            Self::Workqueue,
            Self::Rcu,
            Self::Signal,
            Self::Module,
            Self::Kvm,
            Self::Custom,
        ]
    }
}

/// Tracepoint state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TracepointState {
    /// Registered but disabled
    Disabled,
    /// Active and collecting
    Enabled,
    /// Paused temporarily
    Paused,
    /// Error state
    Error,
}

/// Event format field types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// Unsigned 8-bit integer
    U8,
    /// Unsigned 16-bit integer
    U16,
    /// Unsigned 32-bit integer
    U32,
    /// Unsigned 64-bit integer
    U64,
    /// Signed 8-bit integer
    S8,
    /// Signed 16-bit integer
    S16,
    /// Signed 32-bit integer
    S32,
    /// Signed 64-bit integer
    S64,
    /// Pointer
    Pointer,
    /// String
    String,
    /// Fixed-size array
    Array,
    /// Dynamic array
    DynArray,
}
