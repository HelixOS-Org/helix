//! Firmware Domain
//!
//! ACPI, UEFI, SMBIOS and firmware update management.

extern crate alloc;

pub mod types;
pub mod acpi;
pub mod uefi;
pub mod smbios;
pub mod update;
pub mod intelligence;

// Re-export all types
pub use types::*;
pub use acpi::*;
pub use uefi::*;
pub use smbios::*;
pub use update::*;
pub use intelligence::*;
