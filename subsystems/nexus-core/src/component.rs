//! Component identifiers.

/// Unique identifier for a kernel component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentId(u64);

impl ComponentId {
    /// Create a new component ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get the raw ID
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Well-known component IDs
    pub const SCHEDULER: Self = Self::new(1);
    pub const MEMORY: Self = Self::new(2);
    pub const FILESYSTEM: Self = Self::new(3);
    pub const NETWORK: Self = Self::new(4);
    pub const DRIVERS: Self = Self::new(5);
    pub const IPC: Self = Self::new(6);
    pub const SECURITY: Self = Self::new(7);
    pub const INTERRUPTS: Self = Self::new(8);
    pub const TIMERS: Self = Self::new(9);
    pub const USERLAND: Self = Self::new(10);
}
