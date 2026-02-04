//! # Raw Limine Protocol Structures
//!
//! This module contains the raw C-compatible structures that match the
//! Limine protocol specification exactly. These are FFI structures used
//! for direct communication with the bootloader.
//!
//! ## Safety
//!
//! All structures in this module are `#[repr(C)]` to ensure ABI compatibility.
//! They should only be accessed through the safe abstractions in higher layers.

#![allow(clippy::doc_markdown)]

use core::ffi::c_char;
use core::ptr;

// =============================================================================
// Base Request/Response Structures
// =============================================================================

/// Raw Limine request header
///
/// All Limine requests start with this structure.
#[repr(C)]
pub struct RawLimineRequest {
    /// Request identifier (4 x u64)
    pub id: [u64; 4],
    /// Protocol revision
    pub revision: u64,
    /// Pointer to response (filled by bootloader)
    pub response: *const (),
}

impl RawLimineRequest {
    /// Create a new request with the given ID
    pub const fn new(id: [u64; 4]) -> Self {
        Self {
            id,
            revision: 0,
            response: ptr::null(),
        }
    }

    /// Create a new request with specific revision
    pub const fn with_revision(id: [u64; 4], revision: u64) -> Self {
        Self {
            id,
            revision,
            response: ptr::null(),
        }
    }
}

// Safety: Request structure is read-only after initialization
unsafe impl Sync for RawLimineRequest {}

// =============================================================================
// Bootloader Info
// =============================================================================

/// Raw bootloader info response
#[repr(C)]
pub struct RawBootloaderInfoResponse {
    /// Response revision
    pub revision: u64,
    /// Bootloader name (null-terminated)
    pub name: *const c_char,
    /// Bootloader version (null-terminated)
    pub version: *const c_char,
}

// =============================================================================
// Stack Size
// =============================================================================

/// Raw stack size request
#[repr(C)]
pub struct RawStackSizeRequest {
    /// Base request
    pub base: RawLimineRequest,
    /// Requested stack size
    pub stack_size: u64,
}

/// Raw stack size response
#[repr(C)]
pub struct RawStackSizeResponse {
    /// Response revision
    pub revision: u64,
}

// =============================================================================
// HHDM (Higher Half Direct Map)
// =============================================================================

/// Raw HHDM response
#[repr(C)]
pub struct RawHhdmResponse {
    /// Response revision
    pub revision: u64,
    /// HHDM offset (virtual address where physical 0 is mapped)
    pub offset: u64,
}

// =============================================================================
// Framebuffer
// =============================================================================

/// Raw framebuffer response
#[repr(C)]
pub struct RawFramebufferResponse {
    /// Response revision
    pub revision: u64,
    /// Number of framebuffers
    pub framebuffer_count: u64,
    /// Pointer to array of framebuffer pointers
    pub framebuffers: *const *const RawFramebuffer,
}

/// Raw framebuffer structure
#[repr(C)]
pub struct RawFramebuffer {
    /// Physical address of the framebuffer
    pub address: *mut u8,
    /// Width in pixels
    pub width: u64,
    /// Height in pixels
    pub height: u64,
    /// Pitch (bytes per row)
    pub pitch: u64,
    /// Bits per pixel
    pub bpp: u16,
    /// Memory model (1 = RGB)
    pub memory_model: u8,
    /// Red mask size
    pub red_mask_size: u8,
    /// Red mask shift
    pub red_mask_shift: u8,
    /// Green mask size
    pub green_mask_size: u8,
    /// Green mask shift
    pub green_mask_shift: u8,
    /// Blue mask size
    pub blue_mask_size: u8,
    /// Blue mask shift
    pub blue_mask_shift: u8,
    /// Unused bytes for alignment
    pub unused: [u8; 7],
    /// EDID size
    pub edid_size: u64,
    /// EDID data pointer
    pub edid: *const u8,
    /// Number of video modes (revision 1+)
    pub mode_count: u64,
    /// Video modes pointer (revision 1+)
    pub modes: *const *const RawVideoMode,
}

/// Raw video mode structure
#[repr(C)]
pub struct RawVideoMode {
    /// Mode pitch
    pub pitch: u64,
    /// Mode width
    pub width: u64,
    /// Mode height
    pub height: u64,
    /// Mode bits per pixel
    pub bpp: u16,
    /// Mode memory model
    pub memory_model: u8,
    /// Red mask size
    pub red_mask_size: u8,
    /// Red mask shift
    pub red_mask_shift: u8,
    /// Green mask size
    pub green_mask_size: u8,
    /// Green mask shift
    pub green_mask_shift: u8,
    /// Blue mask size
    pub blue_mask_size: u8,
    /// Blue mask shift
    pub blue_mask_shift: u8,
}

// =============================================================================
// Paging Mode
// =============================================================================

/// Raw paging mode request
#[repr(C)]
pub struct RawPagingModeRequest {
    /// Base request
    pub base: RawLimineRequest,
    /// Requested paging mode
    pub mode: u64,
    /// Minimum paging mode
    pub min_mode: u64,
    /// Maximum paging mode
    pub max_mode: u64,
}

/// Raw paging mode response
#[repr(C)]
pub struct RawPagingModeResponse {
    /// Response revision
    pub revision: u64,
    /// Actual paging mode used
    pub mode: u64,
}

// =============================================================================
// SMP (Symmetric Multi-Processing)
// =============================================================================

/// Raw SMP request
#[repr(C)]
pub struct RawSmpRequest {
    /// Base request
    pub base: RawLimineRequest,
    /// SMP flags
    pub flags: u64,
}

/// Raw SMP response
#[repr(C)]
pub struct RawSmpResponse {
    /// Response revision
    pub revision: u64,
    /// SMP flags (x2apic enabled, etc.)
    pub flags: u64,
    /// BSP LAPIC ID
    pub bsp_lapic_id: u64,
    /// Number of CPUs
    pub cpu_count: u64,
    /// Pointer to array of CPU info pointers
    pub cpus: *const *const RawSmpInfo,
}

/// Raw SMP CPU info (x86_64)
#[repr(C)]
pub struct RawSmpInfo {
    /// Processor ID (ACPI)
    pub processor_id: u32,
    /// LAPIC ID
    pub lapic_id: u32,
    /// Reserved
    pub reserved: u64,
    /// Goto address (write to start CPU)
    pub goto_address: core::sync::atomic::AtomicU64,
    /// Extra argument passed to CPU
    pub extra_argument: u64,
}

/// Raw SMP CPU info (AArch64)
#[repr(C)]
pub struct RawSmpInfoAarch64 {
    /// Processor ID
    pub processor_id: u32,
    /// GIC CPU interface number
    pub gic_iface_no: u32,
    /// MPIDR
    pub mpidr: u64,
    /// Reserved
    pub reserved: u64,
    /// Goto address
    pub goto_address: core::sync::atomic::AtomicU64,
    /// Extra argument
    pub extra_argument: u64,
}

/// Raw SMP CPU info (RISC-V)
#[repr(C)]
pub struct RawSmpInfoRiscv {
    /// Processor ID
    pub processor_id: u64,
    /// Hart ID
    pub hartid: u64,
    /// Reserved
    pub reserved: u64,
    /// Goto address
    pub goto_address: core::sync::atomic::AtomicU64,
    /// Extra argument
    pub extra_argument: u64,
}

// =============================================================================
// Memory Map
// =============================================================================

/// Raw memory map response
#[repr(C)]
pub struct RawMemmapResponse {
    /// Response revision
    pub revision: u64,
    /// Number of entries
    pub entry_count: u64,
    /// Pointer to array of entry pointers
    pub entries: *const *const RawMemmapEntry,
}

/// Raw memory map entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RawMemmapEntry {
    /// Base address
    pub base: u64,
    /// Length in bytes
    pub length: u64,
    /// Memory type
    pub entry_type: u64,
}

// =============================================================================
// Entry Point
// =============================================================================

/// Entry point function type
pub type EntryPointFn = extern "C" fn() -> !;

/// Raw entry point request
#[repr(C)]
pub struct RawEntryPointRequest {
    /// Base request
    pub base: RawLimineRequest,
    /// Custom entry point function
    pub entry: EntryPointFn,
}

/// Raw entry point response
#[repr(C)]
pub struct RawEntryPointResponse {
    /// Response revision
    pub revision: u64,
}

// =============================================================================
// Kernel/Module File
// =============================================================================

/// Raw file response (used for kernel and modules)
#[repr(C)]
pub struct RawKernelFileResponse {
    /// Response revision
    pub revision: u64,
    /// Kernel file
    pub kernel_file: *const RawFile,
}

/// Raw module response
#[repr(C)]
pub struct RawModuleResponse {
    /// Response revision
    pub revision: u64,
    /// Number of modules
    pub module_count: u64,
    /// Pointer to array of module pointers
    pub modules: *const *const RawFile,
}

/// Raw file structure
#[repr(C)]
pub struct RawFile {
    /// Response revision
    pub revision: u64,
    /// File address in memory
    pub address: *const u8,
    /// File size
    pub size: u64,
    /// File path (null-terminated)
    pub path: *const c_char,
    /// Command line / string (null-terminated)
    pub cmdline: *const c_char,
    /// Media type
    pub media_type: u32,
    /// Unused
    pub unused: u32,
    /// TFTP IP (if media_type == TFTP)
    pub tftp_ip: u32,
    /// TFTP port (if media_type == TFTP)
    pub tftp_port: u32,
    /// Partition index
    pub partition_index: u32,
    /// MBR disk ID
    pub mbr_disk_id: u32,
    /// GPT disk UUID
    pub gpt_disk_uuid: RawUuid,
    /// GPT partition UUID
    pub gpt_part_uuid: RawUuid,
    /// Partition UUID
    pub part_uuid: RawUuid,
}

/// Raw UUID structure
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawUuid {
    /// UUID bytes
    pub bytes: [u8; 16],
}

impl RawUuid {
    /// Create a null UUID
    pub const fn null() -> Self {
        Self { bytes: [0; 16] }
    }

    /// Check if UUID is null
    pub const fn is_null(&self) -> bool {
        let mut i = 0;
        while i < 16 {
            if self.bytes[i] != 0 {
                return false;
            }
            i += 1;
        }
        true
    }
}

// =============================================================================
// RSDP (ACPI)
// =============================================================================

/// Raw RSDP response
#[repr(C)]
pub struct RawRsdpResponse {
    /// Response revision
    pub revision: u64,
    /// RSDP address
    pub address: *const u8,
}

// =============================================================================
// SMBIOS
// =============================================================================

/// Raw SMBIOS response
#[repr(C)]
pub struct RawSmbiosResponse {
    /// Response revision
    pub revision: u64,
    /// 32-bit SMBIOS entry point
    pub entry_32: *const u8,
    /// 64-bit SMBIOS entry point
    pub entry_64: *const u8,
}

// =============================================================================
// EFI
// =============================================================================

/// Raw EFI system table response
#[repr(C)]
pub struct RawEfiSystemTableResponse {
    /// Response revision
    pub revision: u64,
    /// EFI system table address
    pub address: *const u8,
}

/// Raw EFI memory map response
#[repr(C)]
pub struct RawEfiMemmapResponse {
    /// Response revision
    pub revision: u64,
    /// Memory map address
    pub memmap: *const u8,
    /// Memory map size
    pub memmap_size: u64,
    /// Descriptor size
    pub desc_size: u64,
    /// Descriptor version
    pub desc_version: u64,
}

// =============================================================================
// Boot Time
// =============================================================================

/// Raw boot time response
#[repr(C)]
pub struct RawBootTimeResponse {
    /// Response revision
    pub revision: u64,
    /// Boot time (UNIX timestamp)
    pub boot_time: i64,
}

// =============================================================================
// Kernel Address
// =============================================================================

/// Raw kernel address response
#[repr(C)]
pub struct RawKernelAddressResponse {
    /// Response revision
    pub revision: u64,
    /// Physical base address
    pub physical_base: u64,
    /// Virtual base address
    pub virtual_base: u64,
}

// =============================================================================
// DTB (Device Tree Blob)
// =============================================================================

/// Raw DTB response
#[repr(C)]
pub struct RawDtbResponse {
    /// Response revision
    pub revision: u64,
    /// DTB address
    pub dtb: *const u8,
}

// =============================================================================
// Size Assertions
// =============================================================================

const _: () = {
    // Ensure structures have expected sizes and alignments
    assert!(core::mem::size_of::<RawLimineRequest>() == 48);
    assert!(core::mem::align_of::<RawLimineRequest>() == 8);

    assert!(core::mem::size_of::<RawMemmapEntry>() == 24);
    assert!(core::mem::size_of::<RawUuid>() == 16);
};
