//! # Helix Multiboot2 - Revolutionary Boot Protocol Implementation
//!
//! This crate provides a **type-safe, zero-copy, compile-time validated**
//! implementation of the Multiboot2 boot protocol specification.
//!
//! ## Design Philosophy
//!
//! 1. **Unsafe Minimization**: Only ONE unsafe operation at the boundary
//! 2. **Zero-Copy**: All parsing happens in-place with borrowed references
//! 3. **Compile-Time Safety**: Header generation with static validation
//! 4. **Lifetime Enforcement**: Parsed data is lifetime-bound to boot info
//! 5. **Future-Proof**: Designed for UEFI/Limine compatibility
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use helix_multiboot2::{BootInfoError, Multiboot2Info};
//!
//! // Entry point receives raw pointer from bootloader
//! fn kernel_entry(multiboot_magic: u32, info_ptr: *const u8) {
//!     // Validate magic number
//!     if multiboot_magic != helix_multiboot2::BOOTLOADER_MAGIC {
//!         panic!(\"Invalid Multiboot2 magic!\");
//!     }
//!
//!     // Parse boot information (single unsafe boundary)
//!     let boot_info = unsafe {
//!         Multiboot2Info::from_ptr(info_ptr).expect(\"Invalid boot info\")
//!     };
//!
//!     // Safe iteration over typed tags
//!     for tag in boot_info.tags() {
//!         // Process tags safely...
//!     }
//! }
//! ```
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        HELIX MULTIBOOT2 STACK                           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Layer 3: BootInfo (Protocol-Agnostic Abstraction)                      │
//! │           └─ Unified interface for Multiboot2/Limine/UEFI               │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Layer 2: Typed Tags (Memory, Cmdline, Framebuffer, etc.)               │
//! │           └─ Strongly typed, safe accessors                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Layer 1: Multiboot2Info + TagIterator (Zero-Copy Parser)               │
//! │           └─ Lifetime-bound references, validated access                │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Layer 0: Raw Header + Validation (Compile-Time Generation)             │
//! │           └─ Static checksum, linker section placement                  │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
// Allow pedantic clippy warnings for low-level boot code
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::ptr_as_ptr)]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::cast_ptr_alignment)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::match_same_arms)]

#[cfg(feature = "std")]
extern crate std;

// =============================================================================
// Module Declarations
// =============================================================================

/// Multiboot2 header generation (compile-time)
pub mod header;

/// Boot information parsing
pub mod info;

/// Unified boot information abstraction
pub mod boot_info;

/// Validation and error handling
pub mod validate;

/// Memory map abstractions
pub mod memory;

// =============================================================================
// Re-exports
// =============================================================================

pub use boot_info::{BootInfo, BootProtocol};
pub use header::{HeaderBuilder, HeaderTag, Multiboot2Header};
pub use info::{Multiboot2Info, Tag, TagIterator};
pub use memory::{MemoryMap, MemoryRegion, MemoryRegionKind};
pub use validate::{BootInfoError, ValidationResult};

// =============================================================================
// Constants
// =============================================================================

/// Multiboot2 header magic number
///
/// This magic number must appear at the beginning of the Multiboot2 header
/// in the kernel image. It identifies the header to the bootloader.
pub const HEADER_MAGIC: u32 = 0xE85250D6;

/// Multiboot2 bootloader magic number
///
/// This value is placed in EAX by a Multiboot2-compliant bootloader
/// when transferring control to the kernel. Used to verify boot protocol.
pub const BOOTLOADER_MAGIC: u32 = 0x36D76289;

/// Architecture identifier for i386/x86_64
///
/// Although named "i386", this is also used for x86_64 kernels
/// as Multiboot2 always enters in 32-bit protected mode.
pub const ARCHITECTURE_I386: u32 = 0;

/// Architecture identifier for MIPS
pub const ARCHITECTURE_MIPS: u32 = 4;

/// Required alignment for Multiboot2 header (8 bytes)
pub const HEADER_ALIGNMENT: usize = 8;

/// Required alignment for tags within boot information (8 bytes)
pub const TAG_ALIGNMENT: usize = 8;

/// Maximum offset for Multiboot2 header from start of kernel image
pub const HEADER_MAX_OFFSET: usize = 32768;

// =============================================================================
// Tag Type Constants
// =============================================================================

/// Tag type identifiers as defined in Multiboot2 specification
pub mod tag_types {
    /// End tag (terminates tag list)
    pub const END: u32 = 0;
    /// Boot command line
    pub const CMDLINE: u32 = 1;
    /// Boot loader name
    pub const BOOTLOADER_NAME: u32 = 2;
    /// Boot modules
    pub const MODULE: u32 = 3;
    /// Basic memory information
    pub const BASIC_MEMINFO: u32 = 4;
    /// BIOS boot device
    pub const BOOT_DEVICE: u32 = 5;
    /// Memory map
    pub const MEMORY_MAP: u32 = 6;
    /// VBE information
    pub const VBE: u32 = 7;
    /// Framebuffer information
    pub const FRAMEBUFFER: u32 = 8;
    /// ELF sections
    pub const ELF_SECTIONS: u32 = 9;
    /// APM table
    pub const APM: u32 = 10;
    /// EFI 32-bit system table
    pub const EFI32_ST: u32 = 11;
    /// EFI 64-bit system table
    pub const EFI64_ST: u32 = 12;
    /// SMBIOS tables
    pub const SMBIOS: u32 = 13;
    /// ACPI old RSDP
    pub const ACPI_OLD: u32 = 14;
    /// ACPI new RSDP
    pub const ACPI_NEW: u32 = 15;
    /// Networking information
    pub const NETWORK: u32 = 16;
    /// EFI memory map
    pub const EFI_MMAP: u32 = 17;
    /// EFI boot services not terminated
    pub const EFI_BS: u32 = 18;
    /// EFI 32-bit image handle
    pub const EFI32_IH: u32 = 19;
    /// EFI 64-bit image handle
    pub const EFI64_IH: u32 = 20;
    /// Image load base physical address
    pub const LOAD_BASE_ADDR: u32 = 21;
}

// =============================================================================
// Header Tag Type Constants
// =============================================================================

/// Header tag type identifiers for Multiboot2 header
pub mod header_tag_types {
    /// End tag (terminates header tag list)
    pub const END: u16 = 0;
    /// Information request
    pub const INFORMATION_REQUEST: u16 = 1;
    /// Address tag
    pub const ADDRESS: u16 = 2;
    /// Entry address tag
    pub const ENTRY_ADDRESS: u16 = 3;
    /// Console flags
    pub const CONSOLE_FLAGS: u16 = 4;
    /// Framebuffer request
    pub const FRAMEBUFFER: u16 = 5;
    /// Module alignment
    pub const MODULE_ALIGN: u16 = 6;
    /// EFI boot services
    pub const EFI_BS: u16 = 7;
    /// EFI 32-bit entry point
    pub const ENTRY_ADDRESS_EFI32: u16 = 8;
    /// EFI 64-bit entry point
    pub const ENTRY_ADDRESS_EFI64: u16 = 9;
    /// Relocatable
    pub const RELOCATABLE: u16 = 10;
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Calculate Multiboot2 header checksum at compile time
///
/// The checksum must be computed such that:
/// `magic + architecture + header_length + checksum = 0` (mod 2^32)
///
/// # Arguments
///
/// * `magic` - The Multiboot2 magic number (should be `HEADER_MAGIC`)
/// * `architecture` - Architecture identifier (e.g., `ARCHITECTURE_I386`)
/// * `header_length` - Total length of the header in bytes
///
/// # Returns
///
/// The checksum value to include in the header
#[must_use]
pub const fn calculate_checksum(magic: u32, architecture: u32, header_length: u32) -> u32 {
    // We need: magic + arch + length + checksum ≡ 0 (mod 2^32)
    // Therefore: checksum = -(magic + arch + length) (mod 2^32)
    // In two's complement: checksum = (!sum).wrapping_add(1)
    let sum = magic.wrapping_add(architecture).wrapping_add(header_length);
    (!sum).wrapping_add(1)
}

/// Verify a Multiboot2 header checksum
///
/// # Arguments
///
/// * `magic` - The magic number from the header
/// * `architecture` - The architecture from the header
/// * `header_length` - The header length from the header
/// * `checksum` - The checksum from the header
///
/// # Returns
///
/// `true` if the checksum is valid, `false` otherwise
#[must_use]
pub const fn verify_checksum(
    magic: u32,
    architecture: u32,
    header_length: u32,
    checksum: u32,
) -> bool {
    magic
        .wrapping_add(architecture)
        .wrapping_add(header_length)
        .wrapping_add(checksum)
        == 0
}

/// Align a value up to the specified alignment
///
/// # Arguments
///
/// * `value` - The value to align
/// * `alignment` - The alignment (must be a power of 2)
///
/// # Returns
///
/// The value aligned up to the next multiple of `alignment`
#[must_use]
pub const fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

// =============================================================================
// Compile-Time Assertions
// =============================================================================

// Ensure our constants match the specification
const _: () = {
    assert!(HEADER_ALIGNMENT == 8);
    assert!(TAG_ALIGNMENT == 8);
    assert!(HEADER_MAX_OFFSET == 32768);

    // Verify checksum calculation
    let checksum = calculate_checksum(HEADER_MAGIC, ARCHITECTURE_I386, 24);
    assert!(verify_checksum(
        HEADER_MAGIC,
        ARCHITECTURE_I386,
        24,
        checksum
    ));
};
