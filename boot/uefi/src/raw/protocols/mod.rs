//! Raw UEFI Protocol Definitions
//!
//! This module contains the raw FFI definitions for UEFI protocols.
//! These are low-level structures matching the UEFI specification exactly.

#![allow(clippy::unreadable_literal)]

pub mod block;
pub mod device_path;
pub mod file;
pub mod gop;
pub mod loaded_image;
pub mod pci;
pub mod rng;
pub mod serial;

// Re-export commonly used protocols
pub use block::*;
pub use device_path::*;
pub use file::*;
pub use gop::*;
pub use loaded_image::*;
pub use pci::*;
pub use rng::*;
pub use serial::*;
