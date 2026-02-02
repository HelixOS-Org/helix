//! # Helix Execution Subsystem
//!
//! The execution subsystem manages:
//! - Thread creation and management
//! - Process abstraction
//! - Scheduler framework
//! - Context switching
//! - Execution domains
//!
//! ## Key Principle
//!
//! This subsystem provides FRAMEWORKS, not implementations.
//! The actual scheduler, for example, is a module that can be swapped.

#![no_std]
#![feature(negative_impls)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

pub mod context;
pub mod domains;
pub mod process;
pub mod scheduler;
pub mod thread;

use core::sync::atomic::{AtomicU64, Ordering};

/// Unique identifier for threads
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadId(u64);

impl ThreadId {
    /// Create a new thread ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the kernel idle thread ID
    pub const fn idle() -> Self {
        Self(0)
    }

    /// Get the raw ID value
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(u64);

impl ProcessId {
    /// Create a new process ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the kernel process ID
    pub const fn kernel() -> Self {
        Self(0)
    }

    /// Get the raw ID value
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution result type
pub type ExecResult<T> = Result<T, ExecError>;

/// Execution errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecError {
    /// Thread not found
    ThreadNotFound,
    /// Process not found
    ProcessNotFound,
    /// Invalid state
    InvalidState,
    /// Already exists
    AlreadyExists,
    /// Out of resources
    OutOfResources,
    /// Permission denied
    PermissionDenied,
    /// Invalid argument
    InvalidArgument,
    /// Internal error
    Internal,
}
