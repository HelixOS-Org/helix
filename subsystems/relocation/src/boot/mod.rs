//! # Boot Protocol Adapters
//!
//! Unified interface for different boot protocols.

use crate::{RelocResult, RelocError, PhysAddr, context::BootProtocol};

// ============================================================================
// BOOT CONTEXT
// ============================================================================

/// Boot information from bootloader
#[derive(Debug, Clone)]
pub struct BootContext {
    /// Boot protocol used
    pub protocol: BootProtocol,
    /// Kernel physical load address
    pub kernel_phys_base: PhysAddr,
    /// Kernel size in bytes
    pub kernel_size: usize,
    /// Kernel virtual link address (from linker script)
    pub kernel_virt_base: u64,
    /// Memory map (if available)
    pub memory_map: Option<MemoryMap>,
    /// Initrd/initramfs (if present)
    pub initrd: Option<MemoryRegion>,
    /// Command line (if present)
    pub cmdline: Option<&'static str>,
    /// ACPI RSDP physical address
    pub rsdp_addr: Option<PhysAddr>,
    /// Framebuffer info (if available)
    pub framebuffer: Option<FramebufferInfo>,
}

impl BootContext {
    /// Create empty boot context
    pub const fn empty(protocol: BootProtocol) -> Self {
        Self {
            protocol,
            kernel_phys_base: PhysAddr(0),
            kernel_size: 0,
            kernel_virt_base: 0,
            memory_map: None,
            initrd: None,
            cmdline: None,
            rsdp_addr: None,
            framebuffer: None,
        }
    }

    /// Calculate initial slide (phys - virt)
    pub fn initial_slide(&self) -> i64 {
        self.kernel_phys_base.0 as i64 - self.kernel_virt_base as i64
    }

    /// Check if KASLR is possible
    pub fn kaslr_possible(&self) -> bool {
        // KASLR needs memory map to find safe regions
        self.memory_map.is_some()
    }
}

// ============================================================================
// MEMORY STRUCTURES
// ============================================================================

/// Memory region
#[derive(Debug, Clone, Copy)]
pub struct MemoryRegion {
    /// Base physical address
    pub base: PhysAddr,
    /// Size in bytes
    pub size: u64,
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MemoryType {
    /// Usable RAM
    Usable = 0,
    /// Reserved (do not use)
    Reserved = 1,
    /// ACPI reclaimable
    AcpiReclaimable = 2,
    /// ACPI NVS
    AcpiNvs = 3,
    /// Bad memory
    BadMemory = 4,
    /// Bootloader reclaimable
    BootloaderReclaimable = 5,
    /// Kernel and modules
    KernelAndModules = 6,
    /// Framebuffer
    Framebuffer = 7,
}

/// Memory map entry
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapEntry {
    /// Base address
    pub base: PhysAddr,
    /// Size in bytes
    pub size: u64,
    /// Memory type
    pub kind: MemoryType,
}

/// Memory map
#[derive(Debug, Clone)]
pub struct MemoryMap {
    /// Entries (static array for no_std)
    entries: [Option<MemoryMapEntry>; 64],
    /// Number of valid entries
    count: usize,
}

impl MemoryMap {
    /// Create empty memory map
    pub const fn new() -> Self {
        Self {
            entries: [None; 64],
            count: 0,
        }
    }

    /// Add entry
    pub fn add(&mut self, entry: MemoryMapEntry) -> bool {
        if self.count < 64 {
            self.entries[self.count] = Some(entry);
            self.count += 1;
            true
        } else {
            false
        }
    }

    /// Get entries
    pub fn entries(&self) -> impl Iterator<Item = &MemoryMapEntry> {
        self.entries[..self.count]
            .iter()
            .filter_map(|e| e.as_ref())
    }

    /// Total usable memory
    pub fn total_usable(&self) -> u64 {
        self.entries()
            .filter(|e| e.kind == MemoryType::Usable)
            .map(|e| e.size)
            .sum()
    }

    /// Find largest usable region
    pub fn largest_usable(&self) -> Option<&MemoryMapEntry> {
        self.entries()
            .filter(|e| e.kind == MemoryType::Usable)
            .max_by_key(|e| e.size)
    }
}

/// Framebuffer info
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    /// Physical address
    pub address: PhysAddr,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pitch (bytes per row)
    pub pitch: u32,
    /// Bits per pixel
    pub bpp: u16,
}

// ============================================================================
// BOOT ADAPTER TRAIT
// ============================================================================

/// Boot protocol adapter
pub trait BootAdapter {
    /// Get boot protocol
    fn protocol(&self) -> BootProtocol;

    /// Parse boot info and create context
    ///
    /// # Safety
    /// Boot info pointer must be valid
    unsafe fn parse_boot_info(&self, boot_info: *const u8) -> RelocResult<BootContext>;

    /// Check if PIE relocation is supported
    fn supports_pie(&self) -> bool;

    /// Check if KASLR is supported
    fn supports_kaslr(&self) -> bool;

    /// Get recommended relocation strategy
    fn recommended_strategy(&self) -> crate::context::RelocationStrategy;
}

// ============================================================================
// UEFI ADAPTER
// ============================================================================

/// UEFI boot adapter
pub struct UefiAdapter;

impl BootAdapter for UefiAdapter {
    fn protocol(&self) -> BootProtocol {
        BootProtocol::Uefi
    }

    unsafe fn parse_boot_info(&self, _boot_info: *const u8) -> RelocResult<BootContext> {
        // UEFI: boot_info is EFI_SYSTEM_TABLE pointer
        // Full parsing would happen here
        let mut ctx = BootContext::empty(BootProtocol::Uefi);

        // UEFI provides excellent environment for PIE
        ctx.kernel_virt_base = 0; // UEFI loads at physical = virtual

        Ok(ctx)
    }

    fn supports_pie(&self) -> bool {
        true // UEFI has excellent PIE support
    }

    fn supports_kaslr(&self) -> bool {
        true
    }

    fn recommended_strategy(&self) -> crate::context::RelocationStrategy {
        crate::context::RelocationStrategy::FullPie
    }
}

// ============================================================================
// LIMINE ADAPTER
// ============================================================================

/// Limine boot adapter
pub struct LimineAdapter;

impl BootAdapter for LimineAdapter {
    fn protocol(&self) -> BootProtocol {
        BootProtocol::Limine
    }

    unsafe fn parse_boot_info(&self, _boot_info: *const u8) -> RelocResult<BootContext> {
        // Limine uses request/response pattern
        // Actual parsing would access limine_requests
        let ctx = BootContext::empty(BootProtocol::Limine);
        Ok(ctx)
    }

    fn supports_pie(&self) -> bool {
        true // Limine fully supports PIE
    }

    fn supports_kaslr(&self) -> bool {
        true // Limine even has built-in KASLR
    }

    fn recommended_strategy(&self) -> crate::context::RelocationStrategy {
        crate::context::RelocationStrategy::FullPie
    }
}

// ============================================================================
// MULTIBOOT2 ADAPTER
// ============================================================================

/// Multiboot2 boot adapter
pub struct Multiboot2Adapter;

impl BootAdapter for Multiboot2Adapter {
    fn protocol(&self) -> BootProtocol {
        BootProtocol::Multiboot2
    }

    unsafe fn parse_boot_info(&self, boot_info: *const u8) -> RelocResult<BootContext> {
        // Multiboot2 info structure parsing
        if boot_info.is_null() {
            return Err(RelocError::InvalidAddress);
        }

        let mut ctx = BootContext::empty(BootProtocol::Multiboot2);

        // Parse multiboot2 tags
        // Note: Multiboot2 loads in 32-bit mode with physical addressing
        // We typically set link address = load address for simplicity

        Ok(ctx)
    }

    fn supports_pie(&self) -> bool {
        // Multiboot2 32-bit entry point uses absolute addresses
        // PIE is difficult but possible with careful assembly
        false
    }

    fn supports_kaslr(&self) -> bool {
        // KASLR requires either:
        // 1. PIE support (not available)
        // 2. Bootloader cooperation (not in Multiboot2 spec)
        false
    }

    fn recommended_strategy(&self) -> crate::context::RelocationStrategy {
        // Static is the safe choice for Multiboot2
        crate::context::RelocationStrategy::StaticMinimal
    }
}

// ============================================================================
// ADAPTER DISPATCH
// ============================================================================

/// Get adapter for protocol
pub fn get_adapter(protocol: BootProtocol) -> &'static dyn BootAdapter {
    match protocol {
        BootProtocol::Uefi => &UefiAdapter,
        BootProtocol::Limine => &LimineAdapter,
        BootProtocol::Multiboot2 => &Multiboot2Adapter,
        BootProtocol::DirectBoot => &DirectBootAdapter,
        BootProtocol::Unknown => &DirectBootAdapter,
    }
}

/// Direct boot adapter (testing/development)
pub struct DirectBootAdapter;

impl BootAdapter for DirectBootAdapter {
    fn protocol(&self) -> BootProtocol {
        BootProtocol::DirectBoot
    }

    unsafe fn parse_boot_info(&self, _boot_info: *const u8) -> RelocResult<BootContext> {
        Ok(BootContext::empty(BootProtocol::DirectBoot))
    }

    fn supports_pie(&self) -> bool {
        true
    }

    fn supports_kaslr(&self) -> bool {
        false // No memory map in direct boot
    }

    fn recommended_strategy(&self) -> crate::context::RelocationStrategy {
        crate::context::RelocationStrategy::FullPie
    }
}

// ============================================================================
// LIMINE STRUCTURES (for reference)
// ============================================================================

/// Limine bootloader request structure
#[repr(C)]
pub struct LimineRequest {
    pub id: [u64; 4],
    pub revision: u64,
    pub response: *mut core::ffi::c_void,
}

/// Limine kernel address response
#[repr(C)]
pub struct LimineKernelAddressResponse {
    pub revision: u64,
    pub physical_base: u64,
    pub virtual_base: u64,
}

/// Limine HHDM (higher-half direct map) response
#[repr(C)]
pub struct LimineHhdmResponse {
    pub revision: u64,
    pub offset: u64,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn test_memory_map() {
        let mut map = MemoryMap::new();

        map.add(MemoryMapEntry {
            base: PhysAddr(0x100000),
            size: 0x1000000,
            kind: MemoryType::Usable,
        });

        map.add(MemoryMapEntry {
            base: PhysAddr(0x2000000),
            size: 0x8000000,
            kind: MemoryType::Usable,
        });

        assert_eq!(map.total_usable(), 0x1000000 + 0x8000000);
        assert_eq!(map.largest_usable().unwrap().size, 0x8000000);
    }

    #[test]
    fn test_adapter_recommendations() {
        assert!(UefiAdapter.supports_pie());
        assert!(LimineAdapter.supports_pie());
        assert!(!Multiboot2Adapter.supports_pie());
    }
}
