//! Synchronization Core Types
//!
//! Fundamental types for synchronization primitives.

/// Lock identifier
pub type LockId = u64;

/// Thread/Task identifier
pub type ThreadId = u64;

/// Lock type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// Mutex
    Mutex,
    /// Spinlock
    Spinlock,
    /// Read-write lock
    RwLock,
    /// Semaphore
    Semaphore,
    /// Condition variable
    CondVar,
    /// Barrier
    Barrier,
    /// Seqlock
    Seqlock,
    /// RCU
    Rcu,
}

impl LockType {
    /// Is blocking lock?
    pub fn is_blocking(&self) -> bool {
        matches!(
            self,
            Self::Mutex | Self::RwLock | Self::Semaphore | Self::CondVar | Self::Barrier
        )
    }

    /// Is spinning lock?
    pub fn is_spinning(&self) -> bool {
        matches!(self, Self::Spinlock | Self::Seqlock)
    }

    /// Typical overhead (nanoseconds)
    pub fn typical_overhead_ns(&self) -> u64 {
        match self {
            Self::Spinlock => 50,
            Self::Mutex => 500,
            Self::RwLock => 600,
            Self::Semaphore => 800,
            Self::CondVar => 1000,
            Self::Barrier => 2000,
            Self::Seqlock => 30,
            Self::Rcu => 20,
        }
    }
}

/// Lock state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockState {
    /// Free
    Free,
    /// Held exclusively
    HeldExclusive,
    /// Held shared (read)
    HeldShared(u32),
    /// Waiting
    Waiting,
}

/// Lock acquisition mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireMode {
    /// Exclusive (write)
    Exclusive,
    /// Shared (read)
    Shared,
    /// Try acquire (non-blocking)
    Try,
    /// Timed acquire
    Timed(u64),
}
