//! # MAGMA Hardware Abstraction Layer
//!
//! This crate provides hardware-level abstractions for interacting with
//! NVIDIA GPUs, including PCI enumeration, BAR mapping, MMIO operations,
//! and interrupt handling.
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────┐
//! │                         magma-hal                             │
//! │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │
//! │  │   PCI    │  │   BAR    │  │   MMIO   │  │     IRQ      │  │
//! │  │ Discovery│  │ Mapping  │  │   Ops    │  │   Handler    │  │
//! │  └──────────┘  └──────────┘  └──────────┘  └──────────────┘  │
//! │                        │                                      │
//! │  ┌────────────────────────────────────────────────────────┐  │
//! │  │                    Platform Traits                     │  │
//! │  │  (x86_64::Platform, aarch64::Platform, riscv64::...)   │  │
//! │  └────────────────────────────────────────────────────────┘  │
//! └───────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Platform Independence
//!
//! The HAL is designed to be platform-independent through the `Platform`
//! trait. Each architecture implements this trait to provide access to
//! hardware primitives.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]

#[cfg(feature = "alloc")]
extern crate alloc;

#[cfg(feature = "std")]
extern crate std;

pub mod bar;
pub mod iommu;
pub mod irq;
pub mod mmio;
pub mod pci;
pub mod platform;

// Re-exports
pub use bar::{BarInfo, BarRegion, BarType};
pub use mmio::{MmioRegion, MmioSlice};
pub use pci::{PciDevice, PciDeviceInfo};
pub use platform::Platform;
