//! # AArch64 Core Framework
//!
//! This module provides fundamental CPU primitives for AArch64:
//! - General purpose registers
//! - System registers
//! - CPU feature detection
//! - Cache maintenance
//! - Memory barriers
//! - FPU/NEON state management

pub mod barriers;
pub mod cache;
pub mod features;
pub mod fpu;
pub mod registers;
pub mod system_regs;

pub use barriers::{dmb, dsb, isb, MemoryBarrier};
pub use cache::{cache_clean, cache_clean_invalidate, cache_invalidate, CacheOp};
pub use features::{ArmFeature, CpuFeatures};
pub use fpu::{restore_fpu_state, save_fpu_state, FpuState};
pub use registers::Registers;
pub use system_regs::{Mair, Sctlr, SystemRegs, Tcr};
