//! # Boot Information Structures
//!
//! Architecture-agnostic boot information passed from bootloaders.
//! Supports Limine, Multiboot2, UEFI, and direct boot protocols.
//!
//! ## Boot Info Layout
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         BOOT INFO STRUCTURE                              │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Header                                                                  │
//! │  ├── Magic (0x48454C49585F4254 = "HELIX_BT")                           │
//! │  ├── Version                                                            │
//! │  ├── Size                                                               │
//! │  └── Protocol (Limine/Multiboot2/UEFI/Direct)                          │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Memory Information                                                      │
//! │  ├── Memory Map Entries                                                 │
//! │  ├── Total Physical Memory                                              │
//! │  ├── HHDM Offset                                                        │
//! │  └── Kernel Physical/Virtual Addresses                                  │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Framebuffer (Optional)                                                  │
//! │  ├── Address, Width, Height, Pitch, BPP                                 │
//! │  └── Pixel Format                                                       │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Firmware Tables                                                         │
//! │  ├── ACPI RSDP                                                          │
//! │  ├── SMBIOS Entry                                                       │
//! │  └── Device Tree (ARM/RISC-V)                                           │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Modules (Optional)                                                      │
//! │  └── List of loaded modules                                             │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  SMP Information                                                         │
//! │  ├── CPU Count                                                          │
//! │  ├── BSP ID                                                             │
//! │  └── Per-CPU Info Array                                                 │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::error::{BootError, BootResult};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Boot info magic: "HELIX_BT" in ASCII
pub const BOOT_INFO_MAGIC: u64 = 0x54425F58494C4548;

/// Current boot info version
pub const BOOT_INFO_VERSION: u32 = 1;

/// Maximum memory map entries
pub const MAX_MEMORY_MAP_ENTRIES: usize = 256;

/// Maximum modules
pub const MAX_MODULES: usize = 64;

/// Maximum CPUs
pub const MAX_CPUS: usize = 256;

/// Maximum command line length
pub const MAX_CMDLINE_LEN: usize = 4096;

// =============================================================================
// BOOT PROTOCOL
// =============================================================================

/// Boot protocol identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum BootProtocol {
    /// Unknown protocol
    Unknown    = 0,
    /// Limine boot protocol
    Limine     = 1,
    /// Multiboot2 protocol
    Multiboot2 = 2,
    /// UEFI direct boot
    Uefi       = 3,
    /// Direct boot (custom)
    Direct     = 4,
    /// Linux boot protocol
    LinuxBoot  = 5,
    /// Device Tree boot (ARM/RISC-V)
    DeviceTree = 6,
}

// =============================================================================
// BOOT INFO HEADER
// =============================================================================

/// Boot information header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BootInfoHeader {
    /// Magic value (BOOT_INFO_MAGIC)
    pub magic: u64,
    /// Structure version
    pub version: u32,
    /// Total structure size
    pub size: u32,
    /// Boot protocol used
    pub protocol: BootProtocol,
    /// Flags
    pub flags: BootInfoFlags,
    /// Checksum (optional)
    pub checksum: u32,
    /// Reserved for future use
    pub reserved: u32,
}

impl BootInfoHeader {
    /// Validate the header
    pub fn validate(&self) -> BootResult<()> {
        if self.magic != BOOT_INFO_MAGIC {
            return Err(BootError::BootInfoMagicMismatch);
        }
        if self.version > BOOT_INFO_VERSION {
            return Err(BootError::BootInfoVersionMismatch);
        }
        Ok(())
    }
}

use bitflags::bitflags;

bitflags! {
    /// Boot info flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct BootInfoFlags: u32 {
        /// Memory map present
        const MEMORY_MAP = 1 << 0;
        /// Framebuffer present
        const FRAMEBUFFER = 1 << 1;
        /// ACPI present
        const ACPI = 1 << 2;
        /// SMBIOS present
        const SMBIOS = 1 << 3;
        /// Device tree present
        const DEVICE_TREE = 1 << 4;
        /// Modules present
        const MODULES = 1 << 5;
        /// SMP info present
        const SMP = 1 << 6;
        /// Command line present
        const CMDLINE = 1 << 7;
        /// UEFI runtime available
        const UEFI_RUNTIME = 1 << 8;
        /// Secure boot active
        const SECURE_BOOT = 1 << 9;
        /// Higher half direct map valid
        const HHDM = 1 << 10;
        /// Kernel already mapped
        const KERNEL_MAPPED = 1 << 11;
        /// Page tables provided
        const PAGE_TABLES = 1 << 12;
    }
}

// =============================================================================
// MAIN BOOT INFO STRUCTURE
// =============================================================================

/// Main boot information structure
///
/// This is the primary structure passed from bootloader to kernel.
/// It provides a unified interface regardless of the actual boot protocol.
#[derive(Debug)]
#[repr(C)]
pub struct BootInfo {
    /// Header
    pub header: BootInfoHeader,

    /// Memory information
    pub memory: MemoryInfo,

    /// Framebuffer information (optional)
    pub framebuffer: Option<FramebufferInfo>,

    /// ACPI information (optional)
    pub acpi: Option<AcpiInfo>,

    /// SMBIOS information (optional)
    pub smbios: Option<SmbiosInfo>,

    /// Device tree (ARM/RISC-V)
    pub device_tree: Option<DeviceTreeInfo>,

    /// Loaded modules
    pub modules: ModuleList,

    /// SMP information
    pub smp: SmpInfo,

    /// Command line
    pub cmdline: CommandLine,

    /// UEFI information
    pub uefi: Option<UefiInfo>,

    /// Architecture-specific data
    pub arch_info: ArchBootInfo,
}

impl BootInfo {
    /// Validate the boot info structure
    pub fn validate(&self) -> BootResult<()> {
        self.header.validate()?;

        // Validate memory info
        if self.header.flags.contains(BootInfoFlags::MEMORY_MAP) {
            if self.memory.map_entries == 0 {
                return Err(BootError::EmptyMemoryMap);
            }
        }

        Ok(())
    }

    /// Check if a specific feature is available
    pub fn has_feature(&self, flag: BootInfoFlags) -> bool {
        self.header.flags.contains(flag)
    }

    /// Get HHDM offset
    pub fn hhdm_offset(&self) -> u64 {
        self.memory.hhdm_offset
    }

    /// Convert physical address to virtual via HHDM
    pub fn phys_to_virt(&self, phys: u64) -> u64 {
        phys + self.memory.hhdm_offset
    }

    /// Convert virtual address to physical via HHDM
    pub fn virt_to_phys(&self, virt: u64) -> u64 {
        virt - self.memory.hhdm_offset
    }
}

// =============================================================================
// MEMORY INFORMATION
// =============================================================================

/// Memory information
#[derive(Debug)]
#[repr(C)]
pub struct MemoryInfo {
    /// Memory map entry count
    pub map_entries: usize,

    /// Memory map (pointer to array)
    pub memory_map: *const MemoryMapEntry,

    /// Total physical memory (bytes)
    pub total_memory: u64,

    /// Usable memory (bytes)
    pub usable_memory: u64,

    /// Higher Half Direct Map offset
    pub hhdm_offset: u64,

    /// Kernel physical start
    pub kernel_phys_start: u64,

    /// Kernel physical end
    pub kernel_phys_end: u64,

    /// Kernel virtual base
    pub kernel_virt_base: u64,

    /// Page table root (if provided by bootloader)
    pub page_table_root: u64,

    /// Stack top (if provided by bootloader)
    pub stack_top: u64,

    /// Stack size
    pub stack_size: u64,
}

impl MemoryInfo {
    /// Get memory map as slice
    ///
    /// # Safety
    /// The memory_map pointer must be valid
    pub unsafe fn memory_map(&self) -> &[MemoryMapEntry] {
        if self.memory_map.is_null() || self.map_entries == 0 {
            &[]
        } else {
            core::slice::from_raw_parts(self.memory_map, self.map_entries)
        }
    }

    /// Iterate over usable memory regions
    ///
    /// # Safety
    ///
    /// The caller must ensure memory map information is valid.
    pub unsafe fn usable_regions(&self) -> impl Iterator<Item = &MemoryMapEntry> {
        self.memory_map()
            .iter()
            .filter(|e| e.memory_type == MemoryType::Usable)
    }
}

/// Memory map entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryMapEntry {
    /// Physical base address
    pub base: u64,
    /// Region length in bytes
    pub length: u64,
    /// Memory type
    pub memory_type: MemoryType,
    /// Attributes
    pub attributes: MemoryAttributes,
}

impl MemoryMapEntry {
    /// Get end address (exclusive)
    pub fn end(&self) -> u64 {
        self.base + self.length
    }

    /// Check if this region contains an address
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.end()
    }

    /// Check if usable for kernel allocation
    pub fn is_usable(&self) -> bool {
        self.memory_type == MemoryType::Usable
    }
}

/// Memory type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryType {
    /// Usable RAM
    Usable           = 0,
    /// Reserved by firmware
    Reserved         = 1,
    /// ACPI reclaimable
    AcpiReclaimable  = 2,
    /// ACPI NVS
    AcpiNvs          = 3,
    /// Bad memory
    BadMemory        = 4,
    /// Bootloader reclaimable
    BootloaderReclaimable = 5,
    /// Kernel and modules
    KernelAndModules = 6,
    /// Framebuffer
    Framebuffer      = 7,
    /// EFI runtime services
    EfiRuntime       = 8,
    /// EFI boot services
    EfiBoot          = 9,
    /// Unknown type
    Unknown          = 255,
}

impl MemoryType {
    /// Convert from UEFI memory type
    pub fn from_uefi(uefi_type: u32) -> Self {
        match uefi_type {
            7 => Self::Usable, // EfiConventionalMemory
            0 | 5 | 6 | 8 | 9 | 10 | 12 | 13 => Self::Reserved,
            3 => Self::EfiBoot, // EfiBootServicesCode/Data
            4 => Self::EfiBoot,
            11 => Self::AcpiReclaimable,
            10 => Self::AcpiNvs,
            1 | 2 => Self::EfiRuntime, // EfiRuntimeServicesCode/Data
            _ => Self::Unknown,
        }
    }

    /// Convert from Multiboot2 memory type
    pub fn from_multiboot2(mb_type: u32) -> Self {
        match mb_type {
            1 => Self::Usable,
            2 => Self::Reserved,
            3 => Self::AcpiReclaimable,
            4 => Self::AcpiNvs,
            5 => Self::BadMemory,
            _ => Self::Unknown,
        }
    }

    /// Convert from Limine memory type
    pub fn from_limine(limine_type: u64) -> Self {
        match limine_type {
            0 => Self::Usable,
            1 => Self::Reserved,
            2 => Self::AcpiReclaimable,
            3 => Self::AcpiNvs,
            4 => Self::BadMemory,
            5 => Self::BootloaderReclaimable,
            6 => Self::KernelAndModules,
            7 => Self::Framebuffer,
            _ => Self::Unknown,
        }
    }
}

bitflags! {
    /// Memory attributes
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryAttributes: u32 {
        /// Uncacheable
        const UC = 1 << 0;
        /// Write-combining
        const WC = 1 << 1;
        /// Write-through
        const WT = 1 << 2;
        /// Write-back
        const WB = 1 << 3;
        /// Uncacheable exported
        const UCE = 1 << 4;
        /// Write-protected
        const WP = 1 << 5;
        /// Read-protected
        const RP = 1 << 6;
        /// Execute-protected
        const XP = 1 << 7;
        /// Non-volatile
        const NV = 1 << 8;
        /// More reliable
        const MORE_RELIABLE = 1 << 9;
        /// Read-only
        const RO = 1 << 10;
        /// Specific purpose
        const SP = 1 << 11;
        /// Crypto capable
        const CRYPTO = 1 << 12;
        /// Runtime accessible
        const RUNTIME = 1 << 15;
    }
}

// =============================================================================
// FRAMEBUFFER INFORMATION
// =============================================================================

/// Framebuffer information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FramebufferInfo {
    /// Framebuffer physical address
    pub address: u64,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pitch (bytes per scanline)
    pub pitch: u32,
    /// Bits per pixel
    pub bpp: u16,
    /// Pixel format
    pub format: PixelFormat,
    /// Red mask position
    pub red_shift: u8,
    /// Red mask size
    pub red_size: u8,
    /// Green mask position
    pub green_shift: u8,
    /// Green mask size
    pub green_size: u8,
    /// Blue mask position
    pub blue_shift: u8,
    /// Blue mask size
    pub blue_size: u8,
    /// Alpha mask position (if present)
    pub alpha_shift: u8,
    /// Alpha mask size (if present)
    pub alpha_size: u8,
}

impl FramebufferInfo {
    /// Get framebuffer size in bytes
    pub fn size(&self) -> usize {
        self.pitch as usize * self.height as usize
    }

    /// Calculate pixel offset
    pub fn pixel_offset(&self, x: u32, y: u32) -> usize {
        (y as usize * self.pitch as usize) + (x as usize * (self.bpp as usize / 8))
    }

    /// Create RGB color for this format
    pub fn rgb(&self, r: u8, g: u8, b: u8) -> u32 {
        let r = (r as u32) << self.red_shift;
        let g = (g as u32) << self.green_shift;
        let b = (b as u32) << self.blue_shift;
        r | g | b
    }

    /// Create RGBA color for this format
    pub fn rgba(&self, r: u8, g: u8, b: u8, a: u8) -> u32 {
        let r = (r as u32) << self.red_shift;
        let g = (g as u32) << self.green_shift;
        let b = (b as u32) << self.blue_shift;
        let a = (a as u32) << self.alpha_shift;
        r | g | b | a
    }
}

/// Pixel format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PixelFormat {
    /// RGB (red-green-blue)
    Rgb     = 0,
    /// BGR (blue-green-red)
    Bgr     = 1,
    /// RGBX (RGB with padding)
    Rgbx    = 2,
    /// BGRX (BGR with padding)
    Bgrx    = 3,
    /// Unknown format
    Unknown = 255,
}

// =============================================================================
// ACPI INFORMATION
// =============================================================================

/// ACPI information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AcpiInfo {
    /// RSDP (Root System Description Pointer) address
    pub rsdp_address: u64,
    /// RSDP revision
    pub rsdp_revision: u8,
    /// RSDT address (if revision 0)
    pub rsdt_address: u32,
    /// XSDT address (if revision >= 2)
    pub xsdt_address: u64,
}

impl AcpiInfo {
    /// Check if ACPI 2.0+ is available
    pub fn is_acpi2(&self) -> bool {
        self.rsdp_revision >= 2
    }

    /// Get the root table address
    pub fn root_table(&self) -> u64 {
        if self.is_acpi2() && self.xsdt_address != 0 {
            self.xsdt_address
        } else {
            self.rsdt_address as u64
        }
    }
}

// =============================================================================
// SMBIOS INFORMATION
// =============================================================================

/// SMBIOS information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SmbiosInfo {
    /// Entry point address (32-bit or 64-bit)
    pub entry_point: u64,
    /// Entry point type
    pub entry_type: SmbiosEntryType,
    /// SMBIOS version (major)
    pub version_major: u8,
    /// SMBIOS version (minor)
    pub version_minor: u8,
    /// Structure table address
    pub table_address: u64,
    /// Structure table length
    pub table_length: u32,
}

/// SMBIOS entry point type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SmbiosEntryType {
    /// 32-bit entry point (SMBIOS 2.x)
    Entry32 = 0,
    /// 64-bit entry point (SMBIOS 3.x)
    Entry64 = 1,
}

// =============================================================================
// DEVICE TREE INFORMATION
// =============================================================================

/// Device tree information (ARM/RISC-V)
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DeviceTreeInfo {
    /// DTB (Device Tree Blob) address
    pub dtb_address: u64,
    /// DTB size
    pub dtb_size: u32,
}

// =============================================================================
// MODULE INFORMATION
// =============================================================================

/// Module list
#[derive(Debug)]
#[repr(C)]
pub struct ModuleList {
    /// Number of modules
    pub count: usize,
    /// Module entries
    pub modules: *const ModuleEntry,
}

impl ModuleList {
    /// Get modules as slice
    ///
    /// # Safety
    /// The modules pointer must be valid
    pub unsafe fn as_slice(&self) -> &[ModuleEntry] {
        if self.modules.is_null() || self.count == 0 {
            &[]
        } else {
            core::slice::from_raw_parts(self.modules, self.count)
        }
    }
}

/// Module entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ModuleEntry {
    /// Module physical address
    pub address: u64,
    /// Module size
    pub size: u64,
    /// Module path/name (null-terminated)
    pub path: [u8; 256],
    /// Module command line (null-terminated)
    pub cmdline: [u8; 256],
}

impl ModuleEntry {
    /// Get module path as string
    pub fn path_str(&self) -> &str {
        let len = self
            .path
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.path.len());
        core::str::from_utf8(&self.path[..len]).unwrap_or("")
    }

    /// Get module command line as string
    pub fn cmdline_str(&self) -> &str {
        let len = self
            .cmdline
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.cmdline.len());
        core::str::from_utf8(&self.cmdline[..len]).unwrap_or("")
    }
}

// =============================================================================
// SMP INFORMATION
// =============================================================================

/// SMP information
#[derive(Debug)]
#[repr(C)]
pub struct SmpInfo {
    /// Total CPU count
    pub cpu_count: u32,
    /// BSP (Boot Strap Processor) ID
    pub bsp_id: u32,
    /// Per-CPU information
    pub cpus: *const CpuInfo,
}

impl SmpInfo {
    /// Get CPU info as slice
    ///
    /// # Safety
    /// The cpus pointer must be valid
    pub unsafe fn as_slice(&self) -> &[CpuInfo] {
        if self.cpus.is_null() || self.cpu_count == 0 {
            &[]
        } else {
            core::slice::from_raw_parts(self.cpus, self.cpu_count as usize)
        }
    }
}

/// Per-CPU information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpuInfo {
    /// CPU ID (APIC ID on x86, MPIDR on ARM, Hart ID on RISC-V)
    pub cpu_id: u32,
    /// ACPI processor UID
    pub acpi_uid: u32,
    /// Local APIC ID (x86 specific)
    pub lapic_id: u32,
    /// Is this the BSP?
    pub is_bsp: bool,
    /// Is this CPU online?
    pub online: bool,
    /// CPU's entry point (for AP startup)
    pub entry_point: u64,
    /// CPU's argument (for AP startup)
    pub argument: u64,
    /// Stack pointer for this CPU
    pub stack_top: u64,
    /// Goto address (Limine protocol)
    pub goto_address: *mut u64,
}

// =============================================================================
// COMMAND LINE
// =============================================================================

/// Command line information
#[derive(Debug)]
#[repr(C)]
pub struct CommandLine {
    /// Command line string (null-terminated)
    pub cmdline: [u8; MAX_CMDLINE_LEN],
    /// Command line length
    pub length: usize,
}

impl CommandLine {
    /// Get command line as string
    pub fn as_str(&self) -> &str {
        let len = self.cmdline[..self.length]
            .iter()
            .position(|&c| c == 0)
            .unwrap_or(self.length);
        core::str::from_utf8(&self.cmdline[..len]).unwrap_or("")
    }

    /// Check if a parameter is present
    pub fn has_param(&self, param: &str) -> bool {
        self.as_str().split_whitespace().any(|p| p == param)
    }

    /// Get value of a parameter (format: param=value)
    pub fn get_param(&self, param: &str) -> Option<&str> {
        let prefix = param;
        self.as_str().split_whitespace().find_map(|p| {
            if p.starts_with(prefix) && p.len() > prefix.len() {
                let rest = &p[prefix.len()..];
                if rest.starts_with('=') {
                    Some(&rest[1..])
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

// =============================================================================
// UEFI INFORMATION
// =============================================================================

/// UEFI specific information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct UefiInfo {
    /// EFI System Table address
    pub system_table: u64,
    /// EFI memory map
    pub memory_map: u64,
    /// EFI memory map size
    pub memory_map_size: u64,
    /// EFI memory descriptor size
    pub descriptor_size: u64,
    /// EFI memory descriptor version
    pub descriptor_version: u32,
    /// Runtime services available
    pub runtime_services: bool,
    /// Secure boot enabled
    pub secure_boot: bool,
}

// =============================================================================
// ARCHITECTURE-SPECIFIC BOOT INFO
// =============================================================================

/// Architecture-specific boot information
#[derive(Debug)]
#[repr(C)]
pub enum ArchBootInfo {
    /// x86_64 specific
    X86_64(X86BootInfoArch),
    /// AArch64 specific
    AArch64(AArch64BootInfoArch),
    /// RISC-V 64 specific
    RiscV64(RiscV64BootInfoArch),
    /// No architecture-specific info
    None,
}

/// x86_64 boot info
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct X86BootInfoArch {
    /// PML4 (or PML5) address
    pub pml_address: u64,
    /// 5-level paging enabled
    pub la57_enabled: bool,
    /// GDT address
    pub gdt_address: u64,
    /// GDT entries
    pub gdt_entries: u16,
    /// VGA text mode base (if available)
    pub vga_text_base: u64,
}

/// AArch64 boot info
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct AArch64BootInfoArch {
    /// Current exception level
    pub current_el: u8,
    /// TTBR0_EL1
    pub ttbr0: u64,
    /// TTBR1_EL1
    pub ttbr1: u64,
    /// TCR_EL1
    pub tcr: u64,
    /// MAIR_EL1
    pub mair: u64,
    /// PSCI version
    pub psci_version: u32,
    /// GIC version
    pub gic_version: u8,
    /// GIC distributor base
    pub gicd_base: u64,
    /// GIC CPU interface / redistributor base
    pub gicc_base: u64,
}

/// RISC-V 64 boot info
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct RiscV64BootInfoArch {
    /// Hart ID
    pub hart_id: u64,
    /// SATP value
    pub satp: u64,
    /// PLIC base
    pub plic_base: u64,
    /// CLINT base
    pub clint_base: u64,
    /// SBI spec version
    pub sbi_spec_version: u32,
    /// SBI implementation ID
    pub sbi_impl_id: u64,
    /// Timer frequency (from DTB/SBI)
    pub timebase_frequency: u64,
}

// =============================================================================
// BOOT INFO BUILDER
// =============================================================================

/// Builder for creating boot info structures
pub struct BootInfoBuilder {
    info: BootInfo,
    memory_map_buffer: [MemoryMapEntry; MAX_MEMORY_MAP_ENTRIES],
    memory_map_count: usize,
}

impl BootInfoBuilder {
    /// Create a new builder
    pub fn new(protocol: BootProtocol) -> Self {
        Self {
            info: BootInfo {
                header: BootInfoHeader {
                    magic: BOOT_INFO_MAGIC,
                    version: BOOT_INFO_VERSION,
                    size: core::mem::size_of::<BootInfo>() as u32,
                    protocol,
                    flags: BootInfoFlags::empty(),
                    checksum: 0,
                    reserved: 0,
                },
                memory: MemoryInfo {
                    map_entries: 0,
                    memory_map: core::ptr::null(),
                    total_memory: 0,
                    usable_memory: 0,
                    hhdm_offset: 0,
                    kernel_phys_start: 0,
                    kernel_phys_end: 0,
                    kernel_virt_base: 0,
                    page_table_root: 0,
                    stack_top: 0,
                    stack_size: 0,
                },
                framebuffer: None,
                acpi: None,
                smbios: None,
                device_tree: None,
                modules: ModuleList {
                    count: 0,
                    modules: core::ptr::null(),
                },
                smp: SmpInfo {
                    cpu_count: 1,
                    bsp_id: 0,
                    cpus: core::ptr::null(),
                },
                cmdline: CommandLine {
                    cmdline: [0; MAX_CMDLINE_LEN],
                    length: 0,
                },
                uefi: None,
                arch_info: ArchBootInfo::None,
            },
            memory_map_buffer: [MemoryMapEntry {
                base: 0,
                length: 0,
                memory_type: MemoryType::Unknown,
                attributes: MemoryAttributes::empty(),
            }; MAX_MEMORY_MAP_ENTRIES],
            memory_map_count: 0,
        }
    }

    /// Add a memory map entry
    pub fn add_memory_region(
        &mut self,
        base: u64,
        length: u64,
        memory_type: MemoryType,
    ) -> &mut Self {
        if self.memory_map_count < MAX_MEMORY_MAP_ENTRIES {
            self.memory_map_buffer[self.memory_map_count] = MemoryMapEntry {
                base,
                length,
                memory_type,
                attributes: MemoryAttributes::empty(),
            };
            self.memory_map_count += 1;
            self.info.header.flags.insert(BootInfoFlags::MEMORY_MAP);
        }
        self
    }

    /// Set HHDM offset
    pub fn set_hhdm(&mut self, offset: u64) -> &mut Self {
        self.info.memory.hhdm_offset = offset;
        self.info.header.flags.insert(BootInfoFlags::HHDM);
        self
    }

    /// Set framebuffer
    pub fn set_framebuffer(&mut self, fb: FramebufferInfo) -> &mut Self {
        self.info.framebuffer = Some(fb);
        self.info.header.flags.insert(BootInfoFlags::FRAMEBUFFER);
        self
    }

    /// Set ACPI info
    pub fn set_acpi(&mut self, acpi: AcpiInfo) -> &mut Self {
        self.info.acpi = Some(acpi);
        self.info.header.flags.insert(BootInfoFlags::ACPI);
        self
    }

    /// Set SMBIOS info
    pub fn set_smbios(&mut self, smbios: SmbiosInfo) -> &mut Self {
        self.info.smbios = Some(smbios);
        self.info.header.flags.insert(BootInfoFlags::SMBIOS);
        self
    }

    /// Set device tree
    pub fn set_device_tree(&mut self, dt: DeviceTreeInfo) -> &mut Self {
        self.info.device_tree = Some(dt);
        self.info.header.flags.insert(BootInfoFlags::DEVICE_TREE);
        self
    }

    /// Set command line
    pub fn set_cmdline(&mut self, cmdline: &str) -> &mut Self {
        let bytes = cmdline.as_bytes();
        let len = bytes.len().min(MAX_CMDLINE_LEN - 1);
        self.info.cmdline.cmdline[..len].copy_from_slice(&bytes[..len]);
        self.info.cmdline.cmdline[len] = 0;
        self.info.cmdline.length = len;
        self.info.header.flags.insert(BootInfoFlags::CMDLINE);
        self
    }

    /// Build the boot info (returns pointer to internal buffer)
    pub fn build(&mut self) -> &BootInfo {
        // Update memory map pointer
        self.info.memory.map_entries = self.memory_map_count;
        self.info.memory.memory_map = self.memory_map_buffer.as_ptr();

        // Calculate totals
        let mut total = 0u64;
        let mut usable = 0u64;
        for i in 0..self.memory_map_count {
            total += self.memory_map_buffer[i].length;
            if self.memory_map_buffer[i].memory_type == MemoryType::Usable {
                usable += self.memory_map_buffer[i].length;
            }
        }
        self.info.memory.total_memory = total;
        self.info.memory.usable_memory = usable;

        &self.info
    }
}
