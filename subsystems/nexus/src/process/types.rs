//! Process Core Types
//!
//! Fundamental types for process intelligence.

/// Process identifier
pub type ProcessId = u64;

/// Thread identifier
pub type ThreadId = u64;

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Running on CPU
    Running,
    /// Ready to run
    Ready,
    /// Blocked on I/O
    BlockedIo,
    /// Blocked on mutex/lock
    BlockedSync,
    /// Sleeping
    Sleeping,
    /// Stopped
    Stopped,
    /// Zombie
    Zombie,
}

/// Process type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessType {
    /// Interactive process (UI, shell)
    Interactive,
    /// Batch processing
    Batch,
    /// Real-time process
    RealTime,
    /// Background service
    Daemon,
    /// System process
    System,
    /// Unknown
    Unknown,
}

/// CPU usage profile
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuProfile {
    /// CPU-intensive
    CpuBound,
    /// I/O-intensive
    IoBound,
    /// Memory-intensive
    MemoryBound,
    /// Balanced
    Balanced,
    /// Idle
    Idle,
}
