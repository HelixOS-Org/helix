//! Raw UEFI bindings
//!
//! This module provides raw, unsafe bindings to UEFI structures and functions.
//! These are the lowest-level building blocks of the UEFI implementation.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                            RAW UEFI LAYER                               │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │                                                                         │
//! │  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
//! │  │     types       │  │  system_table   │  │ boot_services   │         │
//! │  │                 │  │                 │  │                 │         │
//! │  │ - Guid          │  │ - EfiSystemTable│  │ - Memory mgmt   │         │
//! │  │ - Handle        │  │ - TableHeader   │  │ - Protocol mgmt │         │
//! │  │ - Status        │  │ - Config tables │  │ - Event mgmt    │         │
//! │  │ - PhysicalAddr  │  │ - Runtime svc   │  │ - Image mgmt    │         │
//! │  │ - VirtualAddr   │  │ - Boot svc ptr  │  │ - Exit boot svc │         │
//! │  └─────────────────┘  └─────────────────┘  └─────────────────┘         │
//! │                                                                         │
//! │  ┌─────────────────┐  ┌─────────────────┐                               │
//! │  │runtime_services │  │   protocols/    │                               │
//! │  │                 │  │                 │                               │
//! │  │ - Time          │  │ - Console       │                               │
//! │  │ - Variables     │  │ - GOP           │                               │
//! │  │ - Virtual map   │  │ - FileSystem    │                               │
//! │  │ - Reset         │  │ - Block I/O     │                               │
//! │  │ - Capsule       │  │ - PCI           │                               │
//! │  └─────────────────┘  └─────────────────┘                               │
//! │                                                                         │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Safety
//!
//! All types in this module are raw FFI types. Using them directly is unsafe
//! and requires understanding the UEFI specification.

pub mod boot_services;
pub mod memory;
pub mod protocols;
pub mod runtime_services;
pub mod system_table;
pub mod types;

// Re-export main types
pub use boot_services::*;
pub use memory::*;
pub use runtime_services::*;
pub use system_table::*;
pub use types::*;
