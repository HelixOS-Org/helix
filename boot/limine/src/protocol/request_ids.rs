//! # Request Identifier Constants
//!
//! This module contains the unique identifiers for each Limine request type.
//! Each request type has a unique 4-element u64 array identifier.

// These are protocol-defined magic values that must be exact
#![allow(clippy::unreadable_literal)]

use super::magic::make_request_id;

// =============================================================================
// Request IDs
// =============================================================================

/// Bootloader info request ID
pub const BOOTLOADER_INFO_REQUEST_ID: [u64; 4] =
    make_request_id(0xf55038d8e2a1202f, 0x279426fcf5f59740);

/// Stack size request ID
pub const STACK_SIZE_REQUEST_ID: [u64; 4] = make_request_id(0x224ef0460a8e8926, 0xe1cb0fc25f46ea3d);

/// Higher Half Direct Map request ID
pub const HHDM_REQUEST_ID: [u64; 4] = make_request_id(0x48dcf1cb8ad2b852, 0x63984e959a98244b);

/// Framebuffer request ID
pub const FRAMEBUFFER_REQUEST_ID: [u64; 4] =
    make_request_id(0x9d5827dcd881dd75, 0xa3148604f6fab11b);

/// Terminal (legacy) request ID
pub const TERMINAL_REQUEST_ID: [u64; 4] = make_request_id(0xc8ac59310c2b0844, 0xa68d0c7265d38878);

/// Paging mode request ID
pub const PAGING_MODE_REQUEST_ID: [u64; 4] =
    make_request_id(0x95c1a0edab0944cb, 0xa4e5cb3842f7488a);

/// SMP (Symmetric Multi-Processing) request ID
pub const SMP_REQUEST_ID: [u64; 4] = make_request_id(0x95a67b819a1b857e, 0xa0b61b723b6a73e0);

/// Memory map request ID
pub const MEMMAP_REQUEST_ID: [u64; 4] = make_request_id(0x67cf3d9d378a806f, 0xe304acdfc50c3c62);

/// Entry point request ID
pub const ENTRY_POINT_REQUEST_ID: [u64; 4] =
    make_request_id(0x13d86c035a1cd3e1, 0x2b0caa89d8f3026a);

/// Kernel file request ID
pub const KERNEL_FILE_REQUEST_ID: [u64; 4] =
    make_request_id(0xad97e90e83f1ed67, 0x31eb5d1c5ff23b69);

/// Module request ID
pub const MODULE_REQUEST_ID: [u64; 4] = make_request_id(0x3e7e279702be32af, 0xca1c4f3bd1280cee);

/// ACPI RSDP request ID
pub const RSDP_REQUEST_ID: [u64; 4] = make_request_id(0xc5e77b6b397e7b43, 0x27637845accdcf3c);

/// SMBIOS request ID
pub const SMBIOS_REQUEST_ID: [u64; 4] = make_request_id(0x9e9046f11e095391, 0xaa4a520fefbde5ee);

/// EFI System Table request ID
pub const EFI_SYSTEM_TABLE_REQUEST_ID: [u64; 4] =
    make_request_id(0x5ceba5163eaaf6d6, 0x0a6981610cf65fcc);

/// EFI Memory Map request ID
pub const EFI_MEMMAP_REQUEST_ID: [u64; 4] = make_request_id(0x7df62a431d6872d5, 0xa4fcdfb3e57306c8);

/// Boot time request ID
pub const BOOT_TIME_REQUEST_ID: [u64; 4] = make_request_id(0x502746e184c088aa, 0xfbc5ec83e6327893);

/// Kernel address request ID
pub const KERNEL_ADDRESS_REQUEST_ID: [u64; 4] =
    make_request_id(0x71ba76863cc55f63, 0xb2644a48c516a487);

/// Device Tree Blob request ID
pub const DTB_REQUEST_ID: [u64; 4] = make_request_id(0xb40ddb48fb54bac7, 0x545081493f81ffb7);

// =============================================================================
// Short Aliases for Convenience
// =============================================================================

/// Alias for bootloader info request ID
pub const BOOTLOADER_INFO_ID: [u64; 4] = BOOTLOADER_INFO_REQUEST_ID;
/// Alias for memory map request ID
pub const MEMMAP_ID: [u64; 4] = MEMMAP_REQUEST_ID;
/// Alias for HHDM request ID
pub const HHDM_ID: [u64; 4] = HHDM_REQUEST_ID;
/// Alias for paging mode request ID
pub const PAGING_MODE_ID: [u64; 4] = PAGING_MODE_REQUEST_ID;
/// Alias for kernel file request ID
pub const KERNEL_FILE_ID: [u64; 4] = KERNEL_FILE_REQUEST_ID;
/// Alias for kernel address request ID
pub const KERNEL_ADDRESS_ID: [u64; 4] = KERNEL_ADDRESS_REQUEST_ID;
/// Alias for module request ID
pub const MODULE_ID: [u64; 4] = MODULE_REQUEST_ID;
/// Alias for SMP request ID
pub const SMP_ID: [u64; 4] = SMP_REQUEST_ID;
/// Alias for framebuffer request ID
pub const FRAMEBUFFER_ID: [u64; 4] = FRAMEBUFFER_REQUEST_ID;
/// Alias for RSDP request ID
pub const RSDP_ID: [u64; 4] = RSDP_REQUEST_ID;
/// Alias for SMBIOS request ID
pub const SMBIOS_ID: [u64; 4] = SMBIOS_REQUEST_ID;
/// Alias for EFI system table request ID
pub const EFI_SYSTEM_TABLE_ID: [u64; 4] = EFI_SYSTEM_TABLE_REQUEST_ID;
/// Alias for EFI memory map request ID
pub const EFI_MEMMAP_ID: [u64; 4] = EFI_MEMMAP_REQUEST_ID;
/// Alias for DTB request ID
pub const DTB_ID: [u64; 4] = DTB_REQUEST_ID;
/// Alias for entry point request ID
pub const ENTRY_POINT_ID: [u64; 4] = ENTRY_POINT_REQUEST_ID;
/// Alias for stack size request ID
pub const STACK_SIZE_ID: [u64; 4] = STACK_SIZE_REQUEST_ID;
/// Alias for boot time request ID
pub const BOOT_TIME_ID: [u64; 4] = BOOT_TIME_REQUEST_ID;

// =============================================================================
// Request ID Enumeration
// =============================================================================

/// Enumeration of all known request types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum RequestType {
    /// Bootloader information
    BootloaderInfo = 0,
    /// Stack size configuration
    StackSize      = 1,
    /// Higher Half Direct Map
    Hhdm           = 2,
    /// Framebuffer
    Framebuffer    = 3,
    /// Terminal (legacy)
    Terminal       = 4,
    /// Paging mode
    PagingMode     = 5,
    /// Symmetric Multi-Processing
    Smp            = 6,
    /// Memory map
    MemoryMap      = 7,
    /// Custom entry point
    EntryPoint     = 8,
    /// Kernel file access
    KernelFile     = 9,
    /// Boot modules
    Module         = 10,
    /// ACPI RSDP
    Rsdp           = 11,
    /// SMBIOS tables
    Smbios         = 12,
    /// EFI System Table
    EfiSystemTable = 13,
    /// EFI Memory Map
    EfiMemoryMap   = 14,
    /// Boot time
    BootTime       = 15,
    /// Kernel load address
    KernelAddress  = 16,
    /// Device Tree Blob
    Dtb            = 17,
}

impl RequestType {
    /// Get the request ID for this request type
    pub const fn id(&self) -> [u64; 4] {
        match self {
            Self::BootloaderInfo => BOOTLOADER_INFO_REQUEST_ID,
            Self::StackSize => STACK_SIZE_REQUEST_ID,
            Self::Hhdm => HHDM_REQUEST_ID,
            Self::Framebuffer => FRAMEBUFFER_REQUEST_ID,
            Self::Terminal => TERMINAL_REQUEST_ID,
            Self::PagingMode => PAGING_MODE_REQUEST_ID,
            Self::Smp => SMP_REQUEST_ID,
            Self::MemoryMap => MEMMAP_REQUEST_ID,
            Self::EntryPoint => ENTRY_POINT_REQUEST_ID,
            Self::KernelFile => KERNEL_FILE_REQUEST_ID,
            Self::Module => MODULE_REQUEST_ID,
            Self::Rsdp => RSDP_REQUEST_ID,
            Self::Smbios => SMBIOS_REQUEST_ID,
            Self::EfiSystemTable => EFI_SYSTEM_TABLE_REQUEST_ID,
            Self::EfiMemoryMap => EFI_MEMMAP_REQUEST_ID,
            Self::BootTime => BOOT_TIME_REQUEST_ID,
            Self::KernelAddress => KERNEL_ADDRESS_REQUEST_ID,
            Self::Dtb => DTB_REQUEST_ID,
        }
    }

    /// Try to identify a request type from its ID
    pub fn from_id(id: &[u64; 4]) -> Option<Self> {
        match *id {
            BOOTLOADER_INFO_REQUEST_ID => Some(Self::BootloaderInfo),
            STACK_SIZE_REQUEST_ID => Some(Self::StackSize),
            HHDM_REQUEST_ID => Some(Self::Hhdm),
            FRAMEBUFFER_REQUEST_ID => Some(Self::Framebuffer),
            TERMINAL_REQUEST_ID => Some(Self::Terminal),
            PAGING_MODE_REQUEST_ID => Some(Self::PagingMode),
            SMP_REQUEST_ID => Some(Self::Smp),
            MEMMAP_REQUEST_ID => Some(Self::MemoryMap),
            ENTRY_POINT_REQUEST_ID => Some(Self::EntryPoint),
            KERNEL_FILE_REQUEST_ID => Some(Self::KernelFile),
            MODULE_REQUEST_ID => Some(Self::Module),
            RSDP_REQUEST_ID => Some(Self::Rsdp),
            SMBIOS_REQUEST_ID => Some(Self::Smbios),
            EFI_SYSTEM_TABLE_REQUEST_ID => Some(Self::EfiSystemTable),
            EFI_MEMMAP_REQUEST_ID => Some(Self::EfiMemoryMap),
            BOOT_TIME_REQUEST_ID => Some(Self::BootTime),
            KERNEL_ADDRESS_REQUEST_ID => Some(Self::KernelAddress),
            DTB_REQUEST_ID => Some(Self::Dtb),
            _ => None,
        }
    }
}
