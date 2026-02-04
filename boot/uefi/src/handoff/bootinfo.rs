//! Boot Information Structure
//!
//! The main boot information structure passed to the kernel.

use crate::handoff::{FramebufferInfo, MemoryMap, ModuleInfo};
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// BOOT INFO
// =============================================================================

/// Boot information structure
///
/// This structure contains all information the kernel needs to know
/// about the system configuration at boot time.
#[derive(Debug, Clone)]
pub struct BootInfo {
    /// Magic number for validation
    pub magic: u32,

    /// Boot info version
    pub version: u32,

    /// Bootloader name
    pub bootloader_name: String,

    /// Bootloader version
    pub bootloader_version: String,

    /// Kernel command line
    pub command_line: String,

    /// Memory map
    pub memory_map: MemoryMap,

    /// Framebuffer information
    pub framebuffer: Option<FramebufferInfo>,

    /// RSDP address (ACPI root)
    pub rsdp_address: Option<PhysicalAddress>,

    /// SMBIOS entry point address
    pub smbios_address: Option<PhysicalAddress>,

    /// EFI system table address
    pub efi_system_table: Option<PhysicalAddress>,

    /// EFI memory map address
    pub efi_memory_map: Option<PhysicalAddress>,

    /// EFI memory map size
    pub efi_memory_map_size: u64,

    /// EFI memory descriptor size
    pub efi_memory_descriptor_size: u64,

    /// Loaded modules
    pub modules: Vec<ModuleInfo>,

    /// Kernel physical address
    pub kernel_physical_address: Option<PhysicalAddress>,

    /// Kernel virtual address
    pub kernel_virtual_address: Option<VirtualAddress>,

    /// Kernel size
    pub kernel_size: u64,

    // =========================================================================
    // KASLR / RELOCATION FIELDS
    // =========================================================================
    /// KASLR slide offset (load_address - link_address)
    /// Zero if KASLR is disabled or kernel loaded at link address
    pub kernel_slide: i64,

    /// Address where kernel was linked (from linker script)
    pub kernel_link_address: u64,

    /// Whether KASLR was applied during this boot
    pub kaslr_enabled: bool,

    /// Entropy quality used for KASLR (0=none, 1=weak, 2=moderate, 3=strong, 4=crypto)
    pub kaslr_entropy_quality: u8,

    /// Number of relocation entries applied
    pub relocation_count: u64,

    // =========================================================================
    // END KASLR / RELOCATION FIELDS
    // =========================================================================
    /// Physical memory offset (for identity mapping)
    pub physical_memory_offset: Option<u64>,

    /// Recursive page table index
    pub recursive_index: Option<u16>,

    /// TLS template
    pub tls_template: Option<TlsTemplate>,

    /// Boot timestamp (nanoseconds since boot)
    pub boot_timestamp: u64,

    /// CPU count (detected at boot)
    pub cpu_count: u32,

    /// BSP (Bootstrap Processor) APIC ID
    pub bsp_apic_id: u32,

    /// Device tree blob address (for ARM/RISC-V)
    pub dtb_address: Option<PhysicalAddress>,

    /// Device tree blob size
    pub dtb_size: u64,
}

impl BootInfo {
    /// Create new boot info
    pub fn new() -> Self {
        Self {
            magic: super::BOOT_INFO_MAGIC,
            version: super::BOOT_INFO_VERSION,
            bootloader_name: String::new(),
            bootloader_version: String::new(),
            command_line: String::new(),
            memory_map: MemoryMap::new(),
            framebuffer: None,
            rsdp_address: None,
            smbios_address: None,
            efi_system_table: None,
            efi_memory_map: None,
            efi_memory_map_size: 0,
            efi_memory_descriptor_size: 0,
            modules: Vec::new(),
            kernel_physical_address: None,
            kernel_virtual_address: None,
            kernel_size: 0,
            // KASLR / Relocation defaults
            kernel_slide: 0,
            kernel_link_address: 0,
            kaslr_enabled: false,
            kaslr_entropy_quality: 0,
            relocation_count: 0,
            // End KASLR fields
            physical_memory_offset: None,
            recursive_index: None,
            tls_template: None,
            boot_timestamp: 0,
            cpu_count: 1,
            bsp_apic_id: 0,
            dtb_address: None,
            dtb_size: 0,
        }
    }

    /// Validate boot info
    pub fn validate(&self) -> bool {
        self.magic == super::BOOT_INFO_MAGIC && self.version == super::BOOT_INFO_VERSION
    }

    /// Get total usable memory
    pub fn total_usable_memory(&self) -> u64 {
        self.memory_map.total_usable()
    }

    /// Get highest physical address
    pub fn highest_physical_address(&self) -> u64 {
        self.memory_map.highest_address()
    }

    /// Check if framebuffer is available
    pub fn has_framebuffer(&self) -> bool {
        self.framebuffer.is_some()
    }

    /// Check if ACPI is available
    pub fn has_acpi(&self) -> bool {
        self.rsdp_address.is_some()
    }

    /// Check if running on EFI
    pub fn is_efi(&self) -> bool {
        self.efi_system_table.is_some()
    }

    // =========================================================================
    // KASLR / RELOCATION METHODS
    // =========================================================================

    /// Check if KASLR was active during boot
    pub fn is_kaslr_active(&self) -> bool {
        self.kaslr_enabled && self.kernel_slide != 0
    }

    /// Get the KASLR slide offset
    pub fn get_kernel_slide(&self) -> i64 {
        self.kernel_slide
    }

    /// Translate a linked address to its actual runtime address
    ///
    /// This is useful for converting addresses from debug info or symbols
    /// to their actual runtime locations.
    pub fn translate_address(&self, linked_addr: u64) -> u64 {
        (linked_addr as i128 + self.kernel_slide as i128) as u64
    }

    /// Get KASLR entropy quality description
    pub fn kaslr_entropy_description(&self) -> &'static str {
        match self.kaslr_entropy_quality {
            0 => "None (KASLR disabled)",
            1 => "Weak (TSC-based)",
            2 => "Moderate (Firmware RNG)",
            3 => "Strong (RDRAND)",
            4 => "Cryptographic (RDSEED)",
            _ => "Unknown",
        }
    }

    /// Set KASLR information
    pub fn set_kaslr_info(
        &mut self,
        slide: i64,
        link_addr: u64,
        entropy_quality: u8,
        reloc_count: u64,
    ) {
        self.kernel_slide = slide;
        self.kernel_link_address = link_addr;
        self.kaslr_enabled = slide != 0;
        self.kaslr_entropy_quality = entropy_quality;
        self.relocation_count = reloc_count;
    }

    // =========================================================================
    // END KASLR / RELOCATION METHODS
    // =========================================================================

    /// Get module count
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// Find module by name
    pub fn find_module(&self, name: &str) -> Option<&ModuleInfo> {
        self.modules.iter().find(|m| m.name == name)
    }

    /// Parse command line option
    pub fn get_cmdline_option(&self, key: &str) -> Option<&str> {
        for part in self.command_line.split_whitespace() {
            if let Some(value) = part.strip_prefix(key) {
                if value.starts_with('=') {
                    return Some(&value[1..]);
                } else if value.is_empty() {
                    return Some("");
                }
            }
        }
        None
    }

    /// Check if command line option is present
    pub fn has_cmdline_option(&self, key: &str) -> bool {
        self.command_line
            .split_whitespace()
            .any(|part| part == key || part.starts_with(&alloc::format!("{}=", key)))
    }
}

impl Default for BootInfo {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TLS TEMPLATE
// =============================================================================

/// TLS (Thread Local Storage) template
#[derive(Debug, Clone, Copy)]
pub struct TlsTemplate {
    /// Start address of template
    pub start_addr: VirtualAddress,
    /// Size of template in file
    pub file_size: u64,
    /// Size of template in memory (includes BSS)
    pub mem_size: u64,
}

impl TlsTemplate {
    /// Get BSS size
    pub fn bss_size(&self) -> u64 {
        if self.mem_size > self.file_size {
            self.mem_size - self.file_size
        } else {
            0
        }
    }
}

// =============================================================================
// BOOT FLAGS
// =============================================================================

/// Boot flags
#[derive(Debug, Clone, Copy, Default)]
pub struct BootFlags {
    /// Debug mode enabled
    pub debug: bool,
    /// Verbose output
    pub verbose: bool,
    /// Single user mode
    pub single_user: bool,
    /// No graphics mode
    pub no_graphics: bool,
    /// Safe mode
    pub safe_mode: bool,
    /// Rescue mode
    pub rescue: bool,
    /// Emergency mode
    pub emergency: bool,
}

impl BootFlags {
    /// Parse from command line
    pub fn from_cmdline(cmdline: &str) -> Self {
        let mut flags = Self::default();

        for opt in cmdline.split_whitespace() {
            match opt {
                "debug" => flags.debug = true,
                "verbose" | "-v" => flags.verbose = true,
                "single" | "1" | "S" => flags.single_user = true,
                "nographics" | "text" => flags.no_graphics = true,
                "safe" => flags.safe_mode = true,
                "rescue" => flags.rescue = true,
                "emergency" => flags.emergency = true,
                _ => {},
            }
        }

        flags
    }
}

// =============================================================================
// BOOT INFO HEADER (C ABI)
// =============================================================================

/// C-compatible boot info header
///
/// This structure can be used for direct memory layout matching
/// with C kernel code.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BootInfoHeader {
    /// Magic number
    pub magic: u32,
    /// Version
    pub version: u32,
    /// Total size of boot info
    pub total_size: u32,
    /// Reserved
    pub reserved: u32,
    /// Flags
    pub flags: u64,
    /// Memory map offset
    pub memory_map_offset: u32,
    /// Memory map entries
    pub memory_map_entries: u32,
    /// Framebuffer offset
    pub framebuffer_offset: u32,
    /// Modules offset
    pub modules_offset: u32,
    /// Modules count
    pub modules_count: u32,
    /// Command line offset
    pub cmdline_offset: u32,
    /// Command line length
    pub cmdline_length: u32,
    /// RSDP address
    pub rsdp_address: u64,
    /// SMBIOS address
    pub smbios_address: u64,
    /// EFI system table
    pub efi_system_table: u64,
    /// Kernel physical address
    pub kernel_physical: u64,
    /// Kernel virtual address
    pub kernel_virtual: u64,
    /// Kernel size
    pub kernel_size: u64,
    /// Physical memory offset
    pub physical_offset: u64,
}

impl BootInfoHeader {
    /// Header size
    pub const SIZE: usize = core::mem::size_of::<Self>();

    /// Create from BootInfo
    pub fn from_boot_info(info: &BootInfo) -> Self {
        let mut flags = 0u64;
        if info.framebuffer.is_some() {
            flags |= 1 << 0;
        }
        if info.rsdp_address.is_some() {
            flags |= 1 << 1;
        }
        if info.smbios_address.is_some() {
            flags |= 1 << 2;
        }
        if info.efi_system_table.is_some() {
            flags |= 1 << 3;
        }

        Self {
            magic: info.magic,
            version: info.version,
            total_size: 0, // Set during serialization
            reserved: 0,
            flags,
            memory_map_offset: 0,
            memory_map_entries: info.memory_map.entries.len() as u32,
            framebuffer_offset: 0,
            modules_offset: 0,
            modules_count: info.modules.len() as u32,
            cmdline_offset: 0,
            cmdline_length: info.command_line.len() as u32,
            rsdp_address: info.rsdp_address.map(|a| a.0).unwrap_or(0),
            smbios_address: info.smbios_address.map(|a| a.0).unwrap_or(0),
            efi_system_table: info.efi_system_table.map(|a| a.0).unwrap_or(0),
            kernel_physical: info.kernel_physical_address.map(|a| a.0).unwrap_or(0),
            kernel_virtual: info.kernel_virtual_address.map(|a| a.0).unwrap_or(0),
            kernel_size: info.kernel_size,
            physical_offset: info.physical_memory_offset.unwrap_or(0),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_boot_info_new() {
        let info = BootInfo::new();
        assert!(info.validate());
        assert_eq!(info.module_count(), 0);
    }

    #[test]
    fn test_cmdline_parsing() {
        let mut info = BootInfo::new();
        info.command_line = String::from("root=/dev/sda1 debug verbose init=/bin/init");

        assert_eq!(info.get_cmdline_option("root"), Some("/dev/sda1"));
        assert_eq!(info.get_cmdline_option("init"), Some("/bin/init"));
        assert!(info.has_cmdline_option("debug"));
        assert!(info.has_cmdline_option("verbose"));
    }

    #[test]
    fn test_boot_flags() {
        let flags = BootFlags::from_cmdline("debug verbose rescue");
        assert!(flags.debug);
        assert!(flags.verbose);
        assert!(flags.rescue);
        assert!(!flags.safe_mode);
    }

    #[test]
    fn test_tls_template() {
        let tls = TlsTemplate {
            start_addr: 0x1000,
            file_size: 0x100,
            mem_size: 0x200,
        };

        assert_eq!(tls.bss_size(), 0x100);
    }
}
