//! # Unified Boot Information
//!
//! This module provides a unified, high-level abstraction over all Limine
//! boot information. It collects all responses into a single, type-safe
//! structure that can be passed throughout the kernel.
//!
//! ## Features
//!
//! - Zero-cost abstraction over raw Limine responses
//! - Lifetime-bound references preventing use-after-free
//! - Builder pattern for flexible initialization
//! - Integration with the `BootProtocol` trait from the boot abstraction layer
//!
//! ## Example
//!
//! ```rust,no_run
//! use helix_limine::boot_info::BootInfo;
//! use helix_limine::requests::*;
//!
//! // Declare all requests
//! #[used]
//! #[link_section = ".limine_requests"]
//! static MEMMAP: MemoryMapRequest = MemoryMapRequest::new();
//!
//! #[used]
//! #[link_section = ".limine_requests"]
//! static HHDM: HhdmRequest = HhdmRequest::new();
//!
//! fn kernel_main() {
//!     let boot_info = BootInfo::new()
//!         .with_memory_map(&MEMMAP)
//!         .with_hhdm(&HHDM)
//!         .build()
//!         .expect("Failed to collect boot info");
//!
//!     println!(
//!         "Total memory: {} MB",
//!         boot_info.total_memory() / (1024 * 1024)
//!     );
//! }
//! ```

use core::fmt;

use crate::requests::*;

/// Unified boot information structure
///
/// This structure collects all available boot information from Limine
/// into a single, easy-to-use interface.
pub struct BootInfo<'a> {
    /// Bootloader information
    bootloader: Option<&'a BootloaderInfoResponse>,
    /// Memory map
    memory_map: Option<&'a MemoryMapResponse>,
    /// HHDM offset
    hhdm: Option<&'a HhdmResponse>,
    /// Paging mode
    paging_mode: Option<&'a PagingModeResponse>,
    /// Kernel file
    kernel_file: Option<&'a KernelFileResponse>,
    /// Kernel addresses
    kernel_address: Option<&'a KernelAddressResponse>,
    /// Boot modules
    modules: Option<&'a ModuleResponse>,
    /// SMP information
    smp: Option<&'a SmpResponse>,
    /// Framebuffer(s)
    framebuffer: Option<&'a FramebufferResponse>,
    /// RSDP (ACPI)
    rsdp: Option<&'a RsdpResponse>,
    /// SMBIOS
    smbios: Option<&'a SmbiosResponse>,
    /// EFI system table
    efi_system_table: Option<&'a EfiSystemTableResponse>,
    /// EFI memory map
    efi_memmap: Option<&'a EfiMemmapResponse>,
    /// DTB (Device Tree)
    dtb: Option<&'a DtbResponse>,
    /// Boot time
    boot_time: Option<&'a BootTimeResponse>,
}

impl<'a> BootInfo<'a> {
    /// Create a new boot info builder
    pub fn builder() -> BootInfoBuilder<'a> {
        BootInfoBuilder::new()
    }

    /// Create a new empty boot info (use builder instead)
    pub const fn empty() -> Self {
        Self {
            bootloader: None,
            memory_map: None,
            hhdm: None,
            paging_mode: None,
            kernel_file: None,
            kernel_address: None,
            modules: None,
            smp: None,
            framebuffer: None,
            rsdp: None,
            smbios: None,
            efi_system_table: None,
            efi_memmap: None,
            dtb: None,
            boot_time: None,
        }
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get bootloader information
    pub fn bootloader(&self) -> Option<&'a BootloaderInfoResponse> {
        self.bootloader
    }

    /// Get the bootloader name
    pub fn bootloader_name(&self) -> &str {
        self.bootloader
            .map(BootloaderInfoResponse::name)
            .unwrap_or("Unknown")
    }

    /// Get the bootloader version
    pub fn bootloader_version(&self) -> &str {
        self.bootloader
            .map(BootloaderInfoResponse::version)
            .unwrap_or("Unknown")
    }

    /// Get memory map
    pub fn memory_map(&self) -> Option<&'a MemoryMapResponse> {
        self.memory_map
    }

    /// Iterate over memory regions
    pub fn memory_regions(&self) -> impl Iterator<Item = MemoryEntry> + 'a {
        self.memory_map.into_iter().flat_map(|m| m.entries())
    }

    /// Get usable memory regions
    pub fn usable_memory(&self) -> impl Iterator<Item = MemoryEntry> + 'a {
        self.memory_regions().filter(MemoryEntry::is_usable)
    }

    /// Get total physical memory
    pub fn total_memory(&self) -> u64 {
        self.memory_map
            .map(MemoryMapResponse::total_memory)
            .unwrap_or(0)
    }

    /// Get total usable memory
    pub fn usable_memory_size(&self) -> u64 {
        self.memory_map
            .map(MemoryMapResponse::total_usable_memory)
            .unwrap_or(0)
    }

    /// Get HHDM response
    pub fn hhdm(&self) -> Option<&'a HhdmResponse> {
        self.hhdm
    }

    /// Get HHDM offset
    pub fn hhdm_offset(&self) -> u64 {
        self.hhdm.map_or(0, HhdmResponse::offset)
    }

    /// Convert physical address to virtual using HHDM
    pub fn phys_to_virt(&self, phys: u64) -> u64 {
        self.hhdm.map_or(phys, |h| h.phys_to_virt(phys))
    }

    /// Convert virtual address to physical using HHDM
    pub fn virt_to_phys(&self, virt: u64) -> Option<u64> {
        self.hhdm.and_then(|h| h.virt_to_phys(virt))
    }

    /// Get paging mode
    pub fn paging_mode(&self) -> Option<&'a PagingModeResponse> {
        self.paging_mode
    }

    /// Check if using 5-level paging
    pub fn is_five_level_paging(&self) -> bool {
        self.paging_mode
            .map_or(false, PagingModeResponse::is_five_level)
    }

    /// Get kernel file
    pub fn kernel_file(&self) -> Option<&'a KernelFileResponse> {
        self.kernel_file
    }

    /// Get kernel address
    pub fn kernel_address(&self) -> Option<&'a KernelAddressResponse> {
        self.kernel_address
    }

    /// Get kernel physical base
    pub fn kernel_phys_base(&self) -> u64 {
        self.kernel_address
            .map_or(0, KernelAddressResponse::physical_base)
    }

    /// Get kernel virtual base
    pub fn kernel_virt_base(&self) -> u64 {
        self.kernel_address
            .map_or(0, KernelAddressResponse::virtual_base)
    }

    /// Get modules
    pub fn modules(&self) -> Option<&'a ModuleResponse> {
        self.modules
    }

    /// Get number of boot modules
    pub fn module_count(&self) -> usize {
        self.modules.map_or(0, ModuleResponse::module_count)
    }

    /// Get SMP information
    pub fn smp(&self) -> Option<&'a SmpResponse> {
        self.smp
    }

    /// Get number of CPUs
    pub fn cpu_count(&self) -> usize {
        self.smp.map_or(1, SmpResponse::cpu_count)
    }

    /// Get BSP LAPIC ID
    pub fn bsp_lapic_id(&self) -> u64 {
        self.smp.map_or(0, SmpResponse::bsp_lapic_id)
    }

    /// Get framebuffer response
    pub fn framebuffer(&self) -> Option<&'a FramebufferResponse> {
        self.framebuffer
    }

    /// Get primary framebuffer
    pub fn primary_framebuffer(&self) -> Option<Framebuffer<'a>> {
        self.framebuffer.and_then(FramebufferResponse::primary)
    }

    /// Check if graphical output is available
    pub fn has_framebuffer(&self) -> bool {
        self.framebuffer.map_or(false, |f| f.count() > 0)
    }

    /// Get RSDP
    pub fn rsdp(&self) -> Option<&'a RsdpResponse> {
        self.rsdp
    }

    /// Get ACPI RSDP address
    pub fn acpi_rsdp_address(&self) -> Option<*const u8> {
        self.rsdp.map(RsdpResponse::address)
    }

    /// Check if ACPI is available
    pub fn has_acpi(&self) -> bool {
        self.rsdp.is_some()
    }

    /// Get SMBIOS
    pub fn smbios(&self) -> Option<&'a SmbiosResponse> {
        self.smbios
    }

    /// Get EFI system table
    pub fn efi_system_table(&self) -> Option<&'a EfiSystemTableResponse> {
        self.efi_system_table
    }

    /// Check if booted via UEFI
    pub fn is_uefi(&self) -> bool {
        self.efi_system_table.is_some()
    }

    /// Get EFI memory map
    pub fn efi_memmap(&self) -> Option<&'a EfiMemmapResponse> {
        self.efi_memmap
    }

    /// Get DTB
    pub fn dtb(&self) -> Option<&'a DtbResponse> {
        self.dtb
    }

    /// Check if Device Tree is available
    pub fn has_device_tree(&self) -> bool {
        self.dtb.map_or(false, DtbResponse::is_valid)
    }

    /// Get boot time
    pub fn boot_time(&self) -> Option<&'a BootTimeResponse> {
        self.boot_time
    }

    /// Get boot timestamp
    pub fn boot_timestamp(&self) -> Option<i64> {
        self.boot_time.map(BootTimeResponse::timestamp)
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Check if essential boot information is available
    pub fn is_valid(&self) -> bool {
        self.memory_map.is_some() && self.hhdm.is_some()
    }

    /// Get a list of missing essential components
    pub fn missing_essentials(&self) -> MissingComponents {
        MissingComponents {
            memory_map: self.memory_map.is_none(),
            hhdm: self.hhdm.is_none(),
            kernel_address: self.kernel_address.is_none(),
        }
    }

    // =========================================================================
    // Architecture-specific helpers
    // =========================================================================

    /// Get the architecture type
    pub fn architecture(&self) -> Architecture {
        #[cfg(target_arch = "x86_64")]
        {
            Architecture::X86_64
        }

        #[cfg(target_arch = "aarch64")]
        {
            Architecture::AArch64
        }

        #[cfg(target_arch = "riscv64")]
        {
            Architecture::RiscV64
        }

        #[cfg(not(any(
            target_arch = "x86_64",
            target_arch = "aarch64",
            target_arch = "riscv64"
        )))]
        {
            Architecture::Unknown
        }
    }

    /// Get firmware type
    pub fn firmware(&self) -> FirmwareType {
        if self.efi_system_table.is_some() {
            FirmwareType::Uefi
        } else if self.dtb.is_some() {
            FirmwareType::DeviceTree
        } else {
            FirmwareType::Legacy
        }
    }
}

impl fmt::Debug for BootInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BootInfo")
            .field("bootloader", &self.bootloader_name())
            .field("total_memory_mb", &(self.total_memory() / (1024 * 1024)))
            .field(
                "usable_memory_mb",
                &(self.usable_memory_size() / (1024 * 1024)),
            )
            .field("cpu_count", &self.cpu_count())
            .field("has_framebuffer", &self.has_framebuffer())
            .field("has_acpi", &self.has_acpi())
            .field("is_uefi", &self.is_uefi())
            .field("architecture", &self.architecture())
            .finish()
    }
}

/// Boot info builder
pub struct BootInfoBuilder<'a> {
    info: BootInfo<'a>,
}

impl<'a> BootInfoBuilder<'a> {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            info: BootInfo::empty(),
        }
    }

    /// Add bootloader info from request
    #[must_use]
    pub fn with_bootloader(mut self, request: &'a BootloaderInfoRequest) -> Self {
        self.info.bootloader = request.response();
        self
    }

    /// Add memory map from request
    #[must_use]
    pub fn with_memory_map(mut self, request: &'a MemoryMapRequest) -> Self {
        self.info.memory_map = request.response();
        self
    }

    /// Add HHDM from request
    #[must_use]
    pub fn with_hhdm(mut self, request: &'a HhdmRequest) -> Self {
        self.info.hhdm = request.response();
        self
    }

    /// Add paging mode from request
    #[must_use]
    pub fn with_paging_mode(mut self, request: &'a PagingModeRequest) -> Self {
        self.info.paging_mode = request.response();
        self
    }

    /// Add kernel file from request
    #[must_use]
    pub fn with_kernel_file(mut self, request: &'a KernelFileRequest) -> Self {
        self.info.kernel_file = request.response();
        self
    }

    /// Add kernel address from request
    #[must_use]
    pub fn with_kernel_address(mut self, request: &'a KernelAddressRequest) -> Self {
        self.info.kernel_address = request.response();
        self
    }

    /// Add modules from request
    #[must_use]
    pub fn with_modules(mut self, request: &'a ModuleRequest) -> Self {
        self.info.modules = request.response();
        self
    }

    /// Add SMP from request
    #[must_use]
    pub fn with_smp(mut self, request: &'a SmpRequest) -> Self {
        self.info.smp = request.response();
        self
    }

    /// Add framebuffer from request
    #[must_use]
    pub fn with_framebuffer(mut self, request: &'a FramebufferRequest) -> Self {
        self.info.framebuffer = request.response();
        self
    }

    /// Add RSDP from request
    #[must_use]
    pub fn with_rsdp(mut self, request: &'a RsdpRequest) -> Self {
        self.info.rsdp = request.response();
        self
    }

    /// Add SMBIOS from request
    #[must_use]
    pub fn with_smbios(mut self, request: &'a SmbiosRequest) -> Self {
        self.info.smbios = request.response();
        self
    }

    /// Add EFI system table from request
    #[must_use]
    pub fn with_efi_system_table(mut self, request: &'a EfiSystemTableRequest) -> Self {
        self.info.efi_system_table = request.response();
        self
    }

    /// Add EFI memory map from request
    #[must_use]
    pub fn with_efi_memmap(mut self, request: &'a EfiMemmapRequest) -> Self {
        self.info.efi_memmap = request.response();
        self
    }

    /// Add DTB from request
    #[must_use]
    pub fn with_dtb(mut self, request: &'a DtbRequest) -> Self {
        self.info.dtb = request.response();
        self
    }

    /// Add boot time from request
    #[must_use]
    pub fn with_boot_time(mut self, request: &'a BootTimeRequest) -> Self {
        self.info.boot_time = request.response();
        self
    }

    /// Build the boot info
    pub fn build(self) -> Result<BootInfo<'a>, BootInfoError> {
        if self.info.memory_map.is_none() {
            return Err(BootInfoError::MissingMemoryMap);
        }
        if self.info.hhdm.is_none() {
            return Err(BootInfoError::MissingHhdm);
        }
        Ok(self.info)
    }

    /// Build without validation (allows missing components)
    pub fn build_unchecked(self) -> BootInfo<'a> {
        self.info
    }
}

impl Default for BootInfoBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// Boot info creation error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootInfoError {
    /// Memory map is required but not provided
    MissingMemoryMap,
    /// HHDM is required but not provided
    MissingHhdm,
    /// Kernel address is required but not provided
    MissingKernelAddress,
    /// Invalid memory map
    InvalidMemoryMap,
}

impl fmt::Display for BootInfoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingMemoryMap => write!(f, "Memory map not available"),
            Self::MissingHhdm => write!(f, "HHDM not available"),
            Self::MissingKernelAddress => write!(f, "Kernel address not available"),
            Self::InvalidMemoryMap => write!(f, "Invalid memory map"),
        }
    }
}

/// Missing essential components
#[derive(Debug, Clone, Copy)]
pub struct MissingComponents {
    /// Memory map is missing
    pub memory_map: bool,
    /// HHDM is missing
    pub hhdm: bool,
    /// Kernel address is missing
    pub kernel_address: bool,
}

impl MissingComponents {
    /// Check if any essential component is missing
    pub fn any_missing(&self) -> bool {
        self.memory_map || self.hhdm || self.kernel_address
    }
}

/// Architecture type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    /// `x86-64` / `AMD64`
    X86_64,
    /// `AArch64` / `ARM64`
    AArch64,
    /// `RISC-V` 64-bit
    RiscV64,
    /// Unknown architecture
    Unknown,
}

/// Firmware type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareType {
    /// UEFI firmware
    Uefi,
    /// Device Tree based (FDT)
    DeviceTree,
    /// Legacy BIOS
    Legacy,
}

// =============================================================================
// BootProtocol trait implementation
// =============================================================================

/// Trait for boot protocol abstraction
///
/// This allows uniform access to boot information regardless of
/// the underlying boot protocol (Limine, Multiboot2, etc.)
pub trait BootProtocol {
    /// Get total usable memory in bytes
    fn usable_memory(&self) -> u64;

    /// Get the HHDM offset
    fn hhdm_offset(&self) -> u64;

    /// Get the number of CPUs
    fn cpu_count(&self) -> usize;

    /// Check if booted via UEFI
    fn is_uefi(&self) -> bool;

    /// Get the boot command line
    fn command_line(&self) -> &str;

    /// Get the bootloader name
    fn bootloader_name(&self) -> &str;
}

impl BootProtocol for BootInfo<'_> {
    fn usable_memory(&self) -> u64 {
        self.usable_memory_size()
    }

    fn hhdm_offset(&self) -> u64 {
        BootInfo::hhdm_offset(self)
    }

    fn cpu_count(&self) -> usize {
        BootInfo::cpu_count(self)
    }

    fn is_uefi(&self) -> bool {
        BootInfo::is_uefi(self)
    }

    fn command_line(&self) -> &str {
        if let Some(kf) = self.kernel_file {
            if let Some(file) = kf.file() {
                return file.cmdline_static();
            }
        }
        ""
    }

    fn bootloader_name(&self) -> &str {
        BootInfo::bootloader_name(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_creation() {
        let builder = BootInfoBuilder::new();
        let info = builder.build_unchecked();
        assert!(!info.is_valid());
    }

    #[test]
    fn test_missing_components() {
        let info = BootInfo::empty();
        let missing = info.missing_essentials();
        assert!(missing.any_missing());
        assert!(missing.memory_map);
        assert!(missing.hhdm);
    }
}
