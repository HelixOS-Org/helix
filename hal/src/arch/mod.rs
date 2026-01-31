//! # Architecture-Specific HAL Modules
//!
//! This module re-exports the appropriate HAL implementation
//! based on the target architecture.

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

// Re-export the current architecture's HAL
#[cfg(target_arch = "x86_64")]
pub use x86_64 as current;

#[cfg(target_arch = "aarch64")]
pub use aarch64 as current;

// Future architectures
// #[cfg(target_arch = "riscv64")]
// pub mod riscv64;
