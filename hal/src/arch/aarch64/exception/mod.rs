//! # AArch64 Exception Framework
//!
//! This module provides comprehensive exception handling for AArch64,
//! including exception vectors, handlers, and EL transitions.

pub mod context;
pub mod el;
pub mod handlers;
pub mod syscall;
pub mod vectors;

// Re-exports
pub use context::{ExceptionContext, TrapFrame};
pub use el::{ExceptionLevel, current_el, in_el1, in_el2};
pub use handlers::{ExceptionHandler, ExceptionInfo, ExceptionType};
pub use syscall::{SyscallHandler, SyscallResult};
pub use vectors::{install_vectors, ExceptionVectors};
