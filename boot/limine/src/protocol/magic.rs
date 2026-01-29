//! # Limine Protocol Magic Numbers and Constants
//!
//! This module defines all magic numbers, revision constants, and identifiers
//! used by the Limine boot protocol.

// =============================================================================
// Protocol Version Constants
// =============================================================================

/// Current Limine protocol revision
///
/// This value represents the base protocol revision that this implementation
/// targets. The bootloader will negotiate to the highest mutually supported
/// revision.
pub const LIMINE_REVISION: u64 = 2;

/// Limine protocol magic number
///
/// This magic number identifies valid Limine structures.
pub const LIMINE_MAGIC: [u64; 2] = [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b];

/// Base revision magic identifier
pub const BASE_REVISION_MAGIC: [u64; 2] = [0xf9562b2d5c95a6c8, 0x6a7b384944536bdc];

// =============================================================================
// Request Common Magic
// =============================================================================

/// Common magic prefix for all Limine requests
///
/// Every Limine request starts with this magic value, followed by the
/// request-specific identifier.
pub const REQUEST_MAGIC_COMMON: [u64; 2] = [0xc7b1dd30df4c8b88, 0x0a82e883a194f07b];

// =============================================================================
// Memory Map Entry Types
// =============================================================================

/// Memory region type: Usable
///
/// This memory is available for use by the kernel.
pub const LIMINE_MEMMAP_USABLE: u64 = 0;

/// Memory region type: Reserved
///
/// This memory is reserved and should not be used.
pub const LIMINE_MEMMAP_RESERVED: u64 = 1;

/// Memory region type: ACPI Reclaimable
///
/// Contains ACPI tables. Can be reclaimed after ACPI initialization.
pub const LIMINE_MEMMAP_ACPI_RECLAIMABLE: u64 = 2;

/// Memory region type: ACPI NVS
///
/// ACPI Non-Volatile Storage. Must be preserved.
pub const LIMINE_MEMMAP_ACPI_NVS: u64 = 3;

/// Memory region type: Bad Memory
///
/// This memory is defective and must not be used.
pub const LIMINE_MEMMAP_BAD_MEMORY: u64 = 4;

/// Memory region type: Bootloader Reclaimable
///
/// Used by the bootloader. Can be reclaimed after kernel initialization.
pub const LIMINE_MEMMAP_BOOTLOADER_RECLAIMABLE: u64 = 5;

/// Memory region type: Kernel and Modules
///
/// Contains the kernel image and boot modules.
pub const LIMINE_MEMMAP_KERNEL_AND_MODULES: u64 = 6;

/// Memory region type: Framebuffer
///
/// Contains the framebuffer memory.
pub const LIMINE_MEMMAP_FRAMEBUFFER: u64 = 7;

// =============================================================================
// Paging Mode Constants
// =============================================================================

/// Paging mode: 4-level paging (standard x86_64)
pub const LIMINE_PAGING_MODE_X86_64_4LVL: u64 = 0;

/// Paging mode: 5-level paging (LA57)
pub const LIMINE_PAGING_MODE_X86_64_5LVL: u64 = 1;

/// Paging mode: AArch64 4KB pages, 4 levels
pub const LIMINE_PAGING_MODE_AARCH64_4K_4LVL: u64 = 0;

/// Paging mode: AArch64 4KB pages, 5 levels
pub const LIMINE_PAGING_MODE_AARCH64_4K_5LVL: u64 = 1;

/// Paging mode: AArch64 16KB pages, 4 levels
pub const LIMINE_PAGING_MODE_AARCH64_16K_4LVL: u64 = 2;

/// Paging mode: AArch64 64KB pages, 3 levels
pub const LIMINE_PAGING_MODE_AARCH64_64K_3LVL: u64 = 3;

/// Paging mode: RISC-V Sv39 (3-level)
pub const LIMINE_PAGING_MODE_RISCV_SV39: u64 = 0;

/// Paging mode: RISC-V Sv48 (4-level)
pub const LIMINE_PAGING_MODE_RISCV_SV48: u64 = 1;

/// Paging mode: RISC-V Sv57 (5-level)
pub const LIMINE_PAGING_MODE_RISCV_SV57: u64 = 2;

/// Minimum paging mode constant
pub const LIMINE_PAGING_MODE_MIN: u64 = 0;

/// Maximum paging mode constant (x86_64)
pub const LIMINE_PAGING_MODE_MAX: u64 = 1;

/// Default paging mode (x86_64)
pub const LIMINE_PAGING_MODE_DEFAULT: u64 = LIMINE_PAGING_MODE_X86_64_4LVL;

// =============================================================================
// Framebuffer Constants
// =============================================================================

/// Framebuffer memory model: RGB
pub const LIMINE_FRAMEBUFFER_RGB: u64 = 1;

// =============================================================================
// SMP Flags
// =============================================================================

/// SMP flag: Enable X2APIC if available
pub const LIMINE_SMP_X2APIC: u64 = 1 << 0;

// =============================================================================
// Internal File Flags
// =============================================================================

/// Internal module flag: Required
pub const LIMINE_INTERNAL_MODULE_REQUIRED: u64 = 1 << 0;

/// Internal module flag: Compressed
pub const LIMINE_INTERNAL_MODULE_COMPRESSED: u64 = 1 << 1;

// =============================================================================
// Media Types
// =============================================================================

/// Media type: Generic
pub const LIMINE_MEDIA_TYPE_GENERIC: u32 = 0;

/// Media type: Optical
pub const LIMINE_MEDIA_TYPE_OPTICAL: u32 = 1;

/// Media type: TFTP
pub const LIMINE_MEDIA_TYPE_TFTP: u32 = 2;

// =============================================================================
// Partition Table Types
// =============================================================================

/// Partition table: None/Unknown
pub const LIMINE_PARTITION_NONE: u32 = 0;

/// Partition table: MBR
pub const LIMINE_PARTITION_MBR: u32 = 1;

/// Partition table: GPT
pub const LIMINE_PARTITION_GPT: u32 = 2;

// =============================================================================
// Utility Functions
// =============================================================================

/// Create a request ID from two u64 values
#[inline]
pub const fn make_request_id(a: u64, b: u64) -> [u64; 4] {
    [REQUEST_MAGIC_COMMON[0], REQUEST_MAGIC_COMMON[1], a, b]
}

/// Verify that a response pointer is valid (non-null)
#[inline]
pub const fn is_response_valid<T>(ptr: *const T) -> bool {
    !ptr.is_null()
}

/// Calculate HHDM virtual address from physical address
#[inline]
pub const fn phys_to_virt(phys: u64, hhdm_offset: u64) -> u64 {
    phys.wrapping_add(hhdm_offset)
}

/// Calculate physical address from HHDM virtual address
#[inline]
pub const fn virt_to_phys(virt: u64, hhdm_offset: u64) -> u64 {
    virt.wrapping_sub(hhdm_offset)
}
