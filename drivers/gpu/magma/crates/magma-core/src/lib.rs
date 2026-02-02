//! # MAGMA Core
//!
//! Foundational traits, types, and abstractions for the MAGMA GPU driver.
//!
//! This crate provides the type-system foundations that enable compile-time
//! safety guarantees across the entire driver stack.
//!
//! ## Design Principles
//!
//! 1. **Zero-Cost Abstractions**: All traits compile to optimal code
//! 2. **Type-State Safety**: Invalid states are unrepresentable
//! 3. **Generational Compatibility**: Traits are hardware-agnostic
//! 4. **No Unsafe Leakage**: Unsafe code is contained and audited
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      magma-core                             │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
//! │  │   Traits    │  │   Types     │  │     Error           │  │
//! │  │  (Engine,   │  │ (GpuAddr,   │  │   Handling          │  │
//! │  │   Memory)   │  │  PciAddr)   │  │                     │  │
//! │  └─────────────┘  └─────────────┘  └─────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(clippy::new_without_default)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

// =============================================================================
// MODULE EXPORTS
// =============================================================================

pub mod command;
pub mod engine;
pub mod error;
pub mod gpu;
pub mod memory;
pub mod sync;
pub mod traits;
pub mod types;

// Re-exports for convenience
pub use error::{Error, Result};
pub use traits::*;
pub use types::*;
