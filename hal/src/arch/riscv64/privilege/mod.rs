//! # RISC-V Privilege Level Framework
//!
//! This module provides comprehensive privilege level management for RISC-V.
//!
//! ## Submodules
//!
//! - `modes`: Privilege mode definitions and transitions
//! - `traps`: Trap handling infrastructure
//! - `vectors`: Trap vector table management
//! - `syscall`: System call (ECALL) handling

pub mod modes;
pub mod traps;
pub mod vectors;
pub mod syscall;

// Re-export commonly used items
pub use modes::{PrivilegeMode, get_current_mode};
pub use traps::{TrapFrame, TrapContext, handle_trap};
pub use vectors::{TrapVector, set_trap_vector};
pub use syscall::{SyscallArgs, syscall_handler};
