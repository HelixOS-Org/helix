//! # Helix Userspace Subsystem
//!
//! The revolutionary userspace subsystem providing:
//! - ELF64 binary loading and execution
//! - Interactive shell with built-in commands
//! - Userspace runtime and process management
//! - Syscall interface layer
//!
//! ## Key Innovation
//!
//! Unlike traditional userspace implementations, Helix userspace is:
//! - **Hot-reloadable**: Shell commands can be updated live
//! - **Intent-driven**: Commands express intent, DIS optimizes execution
//! - **Self-healing**: Crashed processes auto-restart
//! - **Modular**: Each component can be swapped independently

#![no_std]
#![feature(allocator_api)]
#![feature(slice_pattern)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

extern crate alloc;

pub mod elf;
pub mod environment;
pub mod program;
pub mod runtime;
pub mod shell;
pub mod syscalls;

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// Re-exports
pub use elf::{ElfError, ElfHeader, ElfLoader, ProgramHeader};
pub use environment::{EnvVar, Environment};
pub use program::{Program, ProgramInfo};
pub use runtime::{ProcessHandle, Runtime, RuntimeConfig};
pub use shell::{CommandResult, Shell, ShellCommand};
pub use syscalls::{Syscall, SyscallResult, SyscallTable};

/// Userspace subsystem result type
pub type UserResult<T> = Result<T, UserError>;

/// Userspace subsystem errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UserError {
    /// ELF parsing error
    ElfError(ElfError),
    /// Invalid program
    InvalidProgram,
    /// Program not found
    ProgramNotFound,
    /// Permission denied
    PermissionDenied,
    /// Out of memory
    OutOfMemory,
    /// Invalid argument
    InvalidArgument,
    /// Shell error
    ShellError(String),
    /// Runtime error
    RuntimeError(String),
    /// Syscall error
    SyscallError(i32),
    /// IO error
    IoError,
    /// Not implemented
    NotImplemented,
}

impl From<ElfError> for UserError {
    fn from(e: ElfError) -> Self {
        UserError::ElfError(e)
    }
}

/// Userspace version
pub const VERSION: &str = "0.1.0";

/// Userspace build info
pub const BUILD_INFO: &str = concat!(
    "Helix Userspace v",
    env!("CARGO_PKG_VERSION"),
    " - Revolutionary OS Layer"
);

/// Statistics for userspace subsystem
#[derive(Debug, Default)]
pub struct UserspaceStats {
    /// Number of programs loaded
    pub programs_loaded: AtomicU64,
    /// Number of processes spawned
    pub processes_spawned: AtomicU64,
    /// Number of syscalls made
    pub syscalls_made: AtomicU64,
    /// Number of shell commands executed
    pub commands_executed: AtomicU64,
    /// Shell active
    pub shell_active: AtomicBool,
}

impl UserspaceStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            programs_loaded: AtomicU64::new(0),
            processes_spawned: AtomicU64::new(0),
            syscalls_made: AtomicU64::new(0),
            commands_executed: AtomicU64::new(0),
            shell_active: AtomicBool::new(false),
        }
    }

    /// Increment programs loaded
    pub fn program_loaded(&self) {
        self.programs_loaded.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment processes spawned
    pub fn process_spawned(&self) {
        self.processes_spawned.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment syscalls made
    pub fn syscall_made(&self) {
        self.syscalls_made.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment commands executed
    pub fn command_executed(&self) {
        self.commands_executed.fetch_add(1, Ordering::Relaxed);
    }
}

/// Global userspace statistics
pub static STATS: UserspaceStats = UserspaceStats::new();

/// Initialize the userspace subsystem
pub fn init() -> UserResult<()> {
    // Initialize ELF loader
    elf::init()?;

    // Initialize runtime
    runtime::init()?;

    // Initialize syscall table
    syscalls::init()?;

    Ok(())
}

/// Userspace capabilities
#[derive(Debug, Clone, Copy)]
pub struct UserspaceCapabilities {
    /// Can load ELF binaries
    pub can_load_elf: bool,
    /// Can spawn processes
    pub can_spawn: bool,
    /// Has shell access
    pub has_shell: bool,
    /// Can make syscalls
    pub can_syscall: bool,
    /// Has network access
    pub has_network: bool,
    /// Has filesystem access
    pub has_filesystem: bool,
}

impl Default for UserspaceCapabilities {
    fn default() -> Self {
        Self {
            can_load_elf: true,
            can_spawn: true,
            has_shell: true,
            can_syscall: true,
            has_network: false,    // Not yet implemented
            has_filesystem: false, // Not yet implemented
        }
    }
}

/// Run the shell (convenience function)
pub fn run_shell() -> UserResult<()> {
    let shell = Shell::new();
    shell.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_userspace_init() {
        // Stats should start at 0
        assert_eq!(STATS.programs_loaded.load(Ordering::Relaxed), 0);
        assert_eq!(STATS.processes_spawned.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn test_capabilities() {
        let caps = UserspaceCapabilities::default();
        assert!(caps.can_load_elf);
        assert!(caps.has_shell);
    }
}
