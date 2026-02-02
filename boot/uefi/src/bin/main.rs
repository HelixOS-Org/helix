//! Helix OS UEFI Bootloader
//!
//! Main entry point for UEFI boot.
//!
//! This bootloader provides a complete UEFI boot experience for Helix OS,
//! supporting both x86_64 and AArch64 architectures.

#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]
#![allow(unused_imports)]

extern crate alloc;

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use helix_uefi::arch::{Architecture, CpuFeatures, MemoryModel, PlatformInit};
use helix_uefi::error::{Error, Result};
use helix_uefi::handoff::bootinfo::{BootInfo, BootInfoHeader, TlsTemplate};
use helix_uefi::handoff::framebuffer::{FramebufferInfo, PixelFormat};
use helix_uefi::handoff::memory_map::{
    MemoryMap as HandoffMemoryMap, MemoryType as HandoffMemoryType,
};
use helix_uefi::handoff::modules::{ModuleInfo, ModuleList, ModuleType};
use helix_uefi::handoff::rsdp::{AcpiTableFinder, RsdpInfo};
use helix_uefi::loader::elf::ElfLoader;
use helix_uefi::loader::image::KernelImage;
use helix_uefi::memory::allocator::UefiAllocator;
use helix_uefi::memory::map::{MemoryDescriptor, MemoryMap};
use helix_uefi::protocols::file::{File, FileAttribute, FileMode, FileSystem};
use helix_uefi::protocols::graphics::GraphicsOutput;
use helix_uefi::raw::protocol::*;
use helix_uefi::raw::system::*;
use helix_uefi::raw::types::*;
use helix_uefi::services::boot::BootServices;
use helix_uefi::services::console::Console;
use helix_uefi::services::runtime::RuntimeServices;
use helix_uefi::tables::acpi::AcpiTables;
use helix_uefi::tables::config::ConfigurationTable;
use helix_uefi::tables::smbios::SmbiosTables;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Bootloader version
pub const VERSION_MAJOR: u16 = 1;
pub const VERSION_MINOR: u16 = 0;
pub const VERSION_PATCH: u16 = 0;

/// Bootloader name
pub const BOOTLOADER_NAME: &str = "Helix UEFI Boot";

/// Default kernel path
pub const DEFAULT_KERNEL_PATH: &str = "\\EFI\\helix\\kernel";

/// Default initrd path
pub const DEFAULT_INITRD_PATH: &str = "\\EFI\\helix\\initrd";

/// Default config path
pub const DEFAULT_CONFIG_PATH: &str = "\\EFI\\helix\\boot.cfg";

/// Magic number for boot info
pub const BOOT_INFO_MAGIC: u64 = 0x48454C4958424F4F; // "HELIXBOO"

/// Minimum required memory (64 MiB)
pub const MIN_MEMORY_REQUIRED: u64 = 64 * 1024 * 1024;

/// Kernel load address (higher half)
pub const KERNEL_LOAD_VIRT: u64 = 0xFFFF_FFFF_8000_0000;

/// Physical memory map base
pub const PHYS_MAP_BASE: u64 = 0xFFFF_8000_0000_0000;

/// Stack size (256 KiB)
pub const KERNEL_STACK_SIZE: u64 = 256 * 1024;

// =============================================================================
// GLOBAL STATE
// =============================================================================

/// System table pointer
static mut SYSTEM_TABLE: Option<*mut EfiSystemTable> = None;

/// Boot services exited flag
static BOOT_SERVICES_EXITED: AtomicBool = AtomicBool::new(false);

/// Boot timestamp
static BOOT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

// =============================================================================
// ENTRY POINT
// =============================================================================

/// UEFI entry point
#[no_mangle]
pub extern "efiapi" fn efi_main(
    image_handle: EfiHandle,
    system_table: *mut EfiSystemTable,
) -> EfiStatus {
    // Store system table
    unsafe {
        SYSTEM_TABLE = Some(system_table);
    }

    // Record boot timestamp
    #[cfg(target_arch = "x86_64")]
    {
        let tsc = unsafe { core::arch::x86_64::_rdtsc() };
        BOOT_TIMESTAMP.store(tsc, Ordering::Relaxed);
    }

    // Initialize and run boot process
    match boot_main(image_handle, system_table) {
        Ok(()) => EFI_SUCCESS,
        Err(e) => {
            // Try to print error if console is available
            unsafe {
                if let Some(st) = SYSTEM_TABLE {
                    let st = &*st;
                    if !st.con_out.is_null() {
                        // Print error (simplified)
                        let _ = print_error(st, e);
                    }
                }
            }
            e.to_status()
        },
    }
}

/// Main boot logic
fn boot_main(image_handle: EfiHandle, system_table: *mut EfiSystemTable) -> Result<()> {
    let st = unsafe { &*system_table };

    // Verify system table
    if st.hdr.signature != EFI_SYSTEM_TABLE_SIGNATURE {
        return Err(Error::InvalidParameter);
    }

    // Get boot services
    if st.boot_services.is_null() {
        return Err(Error::Unsupported);
    }
    let _bs = unsafe { &*st.boot_services };

    // Clear screen and print banner
    print_banner(st)?;

    // Early hardware initialization
    early_init()?;

    // Load configuration
    let config = load_config(image_handle, st)?;

    // Detect CPU features
    let cpu_features = detect_cpu_features();
    print_cpu_info(st, &cpu_features)?;

    // Initialize graphics
    let framebuffer = init_graphics(st)?;

    // Find ACPI tables
    let acpi_info = find_acpi_tables(st)?;
    print_acpi_info(st, &acpi_info)?;

    // Find SMBIOS tables
    let smbios_addr = find_smbios_tables(st);

    // Load kernel
    let kernel = load_kernel(image_handle, st, &config)?;
    print_kernel_info(st, &kernel)?;

    // Load modules (initrd, etc.)
    let modules = load_modules(image_handle, st, &config)?;

    // Get memory map and exit boot services
    let (memory_map, runtime_services) = exit_boot_services(image_handle, st)?;

    // Set up page tables
    let page_tables = setup_paging(&kernel, &memory_map, &framebuffer)?;

    // Allocate kernel stack
    let kernel_stack = allocate_kernel_stack(&memory_map)?;

    // Build boot info structure
    let boot_info = build_boot_info(
        &framebuffer,
        &memory_map,
        &acpi_info,
        smbios_addr,
        &kernel,
        &modules,
        kernel_stack,
        &config,
    )?;

    // Hand off to kernel
    handoff_to_kernel(&kernel, &boot_info, kernel_stack, page_tables)?;

    // Should never reach here
    unreachable!()
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Early hardware initialization
fn early_init() -> Result<()> {
    #[cfg(target_arch = "x86_64")]
    {
        helix_uefi::arch::x86_64::early_init()?;
    }

    #[cfg(target_arch = "aarch64")]
    {
        helix_uefi::arch::aarch64::early_init();
    }

    Ok(())
}

/// Detect CPU features
fn detect_cpu_features() -> CpuFeatures {
    #[cfg(target_arch = "x86_64")]
    {
        helix_uefi::arch::x86_64::cpu::detect_features()
    }

    #[cfg(target_arch = "aarch64")]
    {
        CpuFeatures::default() // TODO: Implement for aarch64
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        CpuFeatures::default()
    }
}

// =============================================================================
// CONFIGURATION
// =============================================================================

/// Boot configuration
#[derive(Debug, Clone)]
pub struct BootConfig {
    /// Kernel path
    pub kernel_path: [u8; 256],
    pub kernel_path_len: usize,
    /// Initrd path
    pub initrd_path: [u8; 256],
    pub initrd_path_len: usize,
    /// Command line
    pub cmdline: [u8; 1024],
    pub cmdline_len: usize,
    /// Verbose boot
    pub verbose: bool,
    /// Debug mode
    pub debug: bool,
    /// Timeout in seconds
    pub timeout: u32,
}

impl Default for BootConfig {
    fn default() -> Self {
        let mut config = Self {
            kernel_path: [0; 256],
            kernel_path_len: 0,
            initrd_path: [0; 256],
            initrd_path_len: 0,
            cmdline: [0; 1024],
            cmdline_len: 0,
            verbose: false,
            debug: false,
            timeout: 3,
        };

        // Set default paths
        let kernel = DEFAULT_KERNEL_PATH.as_bytes();
        config.kernel_path[..kernel.len()].copy_from_slice(kernel);
        config.kernel_path_len = kernel.len();

        let initrd = DEFAULT_INITRD_PATH.as_bytes();
        config.initrd_path[..initrd.len()].copy_from_slice(initrd);
        config.initrd_path_len = initrd.len();

        config
    }
}

/// Load boot configuration
fn load_config(_image_handle: EfiHandle, _st: &EfiSystemTable) -> Result<BootConfig> {
    // Try to load config file, fall back to defaults
    // TODO: Implement config file parsing
    Ok(BootConfig::default())
}

// =============================================================================
// GRAPHICS
// =============================================================================

/// Initialize graphics and get framebuffer info
fn init_graphics(st: &EfiSystemTable) -> Result<FramebufferInfo> {
    let bs = unsafe { &*st.boot_services };

    // Locate GOP
    let mut gop: *mut EfiGraphicsOutputProtocol = core::ptr::null_mut();
    let guid = EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID;

    let status = unsafe {
        (bs.locate_protocol)(
            &guid as *const _,
            core::ptr::null_mut(),
            &mut gop as *mut _ as *mut *mut core::ffi::c_void,
        )
    };

    if status != EFI_SUCCESS || gop.is_null() {
        // No GOP, return empty framebuffer
        return Ok(FramebufferInfo {
            address: 0,
            size: 0,
            width: 0,
            height: 0,
            stride: 0,
            format: PixelFormat::Rgb32,
            bitmask: None,
            bpp: 0,
        });
    }

    let gop = unsafe { &*gop };
    let mode_info = unsafe { &*(*gop.mode).info };

    let format = match mode_info.pixel_format {
        0 => PixelFormat::Rgb32, // PixelRedGreenBlueReserved8BitPerColor
        1 => PixelFormat::Bgr32, // PixelBlueGreenRedReserved8BitPerColor
        2 => PixelFormat::Bitmask,
        _ => PixelFormat::Rgb32,
    };

    let fb = unsafe { (*gop.mode).frame_buffer_base };
    let fb_size = unsafe { (*gop.mode).frame_buffer_size };

    Ok(FramebufferInfo {
        address: fb,
        size: fb_size as u64,
        width: mode_info.horizontal_resolution,
        height: mode_info.vertical_resolution,
        stride: mode_info.pixels_per_scan_line * 4,
        format,
        bitmask: None,
        bpp: 32,
    })
}

// =============================================================================
// ACPI
// =============================================================================

/// ACPI information
#[derive(Debug, Clone)]
pub struct AcpiInfo {
    /// RSDP address
    pub rsdp_addr: PhysicalAddress,
    /// RSDP version (1 or 2)
    pub version: u8,
    /// XSDT address (v2 only)
    pub xsdt_addr: Option<PhysicalAddress>,
    /// RSDT address
    pub rsdt_addr: PhysicalAddress,
}

/// Find ACPI tables
fn find_acpi_tables(st: &EfiSystemTable) -> Result<AcpiInfo> {
    // ACPI 2.0 GUID
    let acpi20_guid = EfiGuid {
        data1: 0x8868e871,
        data2: 0xe4f1,
        data3: 0x11d3,
        data4: [0xbc, 0x22, 0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
    };

    // ACPI 1.0 GUID
    let acpi10_guid = EfiGuid {
        data1: 0xeb9d2d30,
        data2: 0x2d88,
        data3: 0x11d3,
        data4: [0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
    };

    // Search configuration tables
    let config_count = st.number_of_table_entries;
    let config_tables = st.configuration_table;

    let mut rsdp_addr: Option<PhysicalAddress> = None;
    let mut is_v2 = false;

    for i in 0..config_count {
        let entry = unsafe { &*config_tables.add(i) };

        if entry.vendor_guid == acpi20_guid {
            rsdp_addr = Some(entry.PhysicalAddress(vendor_table as u64));
            is_v2 = true;
            break;
        }

        if entry.vendor_guid == acpi10_guid && rsdp_addr.is_none() {
            rsdp_addr = Some(entry.PhysicalAddress(vendor_table as u64));
        }
    }

    let rsdp = rsdp_addr.ok_or(Error::NotFound)?;

    // Parse RSDP to get RSDT/XSDT
    let rsdp_ptr = rsdp as *const u8;
    let revision = unsafe { *rsdp_ptr.add(15) };

    let rsdt_addr = unsafe {
        let ptr = rsdp_ptr.add(16) as *const u32;
        *PhysicalAddress(ptr as u64)
    };

    let xsdt_addr = if revision >= 2 {
        unsafe {
            let ptr = rsdp_ptr.add(24) as *const u64;
            Some(*ptr)
        }
    } else {
        None
    };

    Ok(AcpiInfo {
        rsdp_addr: rsdp,
        version: if is_v2 { 2 } else { 1 },
        xsdt_addr,
        rsdt_addr,
    })
}

// =============================================================================
// SMBIOS
// =============================================================================

/// Find SMBIOS tables
fn find_smbios_tables(st: &EfiSystemTable) -> Option<PhysicalAddress> {
    // SMBIOS 3.0 GUID
    let smbios3_guid = EfiGuid {
        data1: 0xf2fd1544,
        data2: 0x9794,
        data3: 0x4a2c,
        data4: [0x99, 0x2e, 0xe5, 0xbb, 0xcf, 0x20, 0xe3, 0x94],
    };

    // SMBIOS GUID
    let smbios_guid = EfiGuid {
        data1: 0xeb9d2d31,
        data2: 0x2d88,
        data3: 0x11d3,
        data4: [0x9a, 0x16, 0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
    };

    let config_count = st.number_of_table_entries;
    let config_tables = st.configuration_table;

    for i in 0..config_count {
        let entry = unsafe { &*config_tables.add(i) };

        if entry.vendor_guid == smbios3_guid || entry.vendor_guid == smbios_guid {
            return Some(entry.PhysicalAddress(vendor_table as u64));
        }
    }

    None
}

// =============================================================================
// KERNEL LOADING
// =============================================================================

/// Loaded kernel information
#[derive(Debug)]
pub struct LoadedKernel {
    /// Physical load address
    pub phys_base: PhysicalAddress,
    /// Virtual base address
    pub virt_base: VirtualAddress,
    /// Size in bytes
    pub size: u64,
    /// Entry point
    pub entry_point: VirtualAddress,
    /// TLS template
    pub tls_template: Option<TlsTemplate>,
}

/// Load kernel from filesystem
fn load_kernel(
    image_handle: EfiHandle,
    st: &EfiSystemTable,
    config: &BootConfig,
) -> Result<LoadedKernel> {
    let bs = unsafe { &*st.boot_services };

    // Get loaded image protocol to find our device
    let loaded_image_guid = EFI_LOADED_IMAGE_PROTOCOL_GUID;
    let mut loaded_image: *mut EfiLoadedImageProtocol = core::ptr::null_mut();

    let status = unsafe {
        (bs.open_protocol)(
            image_handle,
            &loaded_image_guid as *const _,
            &mut loaded_image as *mut _ as *mut *mut core::ffi::c_void,
            image_handle,
            core::ptr::null_mut(),
            EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL,
        )
    };

    if status != EFI_SUCCESS {
        return Err(Error::from_status(status));
    }

    let loaded_image = unsafe { &*loaded_image };
    let device_handle = loaded_image.device_handle;

    // Get file system protocol
    let fs_guid = EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_GUID;
    let mut fs_protocol: *mut EfiSimpleFileSystemProtocol = core::ptr::null_mut();

    let status = unsafe {
        (bs.open_protocol)(
            device_handle,
            &fs_guid as *const _,
            &mut fs_protocol as *mut _ as *mut core::ffi::c_void,
            image_handle,
            core::ptr::null_mut(),
            EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL,
        )
    };

    if status != EFI_SUCCESS {
        return Err(Error::from_status(status));
    }

    let fs = unsafe { &*fs_protocol };

    // Open volume
    let mut root: *mut EfiFileProtocol = core::ptr::null_mut();
    let status = unsafe { (fs.open_volume)(fs_protocol, &mut root) };

    if status != EFI_SUCCESS {
        return Err(Error::from_status(status));
    }

    // Convert path to UTF-16
    let path = &config.kernel_path[..config.kernel_path_len];
    let mut path16 = [0u16; 256];
    for (i, &b) in path.iter().enumerate() {
        path16[i] = b as u16;
    }

    // Open kernel file
    let mut kernel_file: *mut EfiFileProtocol = core::ptr::null_mut();
    let root = unsafe { &*root };

    let status = unsafe {
        (root.open)(
            root as *const _ as *mut _,
            &mut kernel_file,
            path16.as_ptr(),
            EFI_FILE_MODE_READ,
            0,
        )
    };

    if status != EFI_SUCCESS {
        return Err(Error::NotFound);
    }

    let kernel_file_ref = unsafe { &*kernel_file };

    // Get file size
    let file_info_guid = EfiGuid {
        data1: 0x09576e92,
        data2: 0x6d3f,
        data3: 0x11d2,
        data4: [0x8e, 0x39, 0x00, 0xa0, 0xc9, 0x69, 0x72, 0x3b],
    };

    let mut info_size: usize = 0;
    let mut info_buffer = [0u8; 256];

    let _status = unsafe {
        (kernel_file_ref.get_info)(
            kernel_file,
            &file_info_guid,
            &mut info_size,
            info_buffer.as_mut_ptr() as *mut core::ffi::c_void,
        )
    };

    // For now, use a fixed size or get from file
    // This is simplified - full implementation would properly query file size
    let file_size = 16 * 1024 * 1024; // Assume max 16 MiB kernel

    // Allocate memory for kernel
    let mut kernel_buffer: *mut core::ffi::c_void = core::ptr::null_mut();
    let pages = (file_size + 4095) / 4096;

    let status = unsafe {
        (bs.allocate_pages)(
            0, // AllocateAnyPages
            2, // EfiLoaderData
            pages,
            &mut kernel_buffer as *mut _ as *mut PhysicalAddress,
        )
    };

    if status != EFI_SUCCESS {
        return Err(Error::OutOfResources);
    }

    // Read kernel file
    let mut bytes_read = file_size as usize;
    let status = unsafe { (kernel_file_ref.read)(kernel_file, &mut bytes_read, kernel_buffer) };

    if status != EFI_SUCCESS {
        return Err(Error::from_status(status));
    }

    // Parse ELF
    let kernel_data =
        unsafe { core::slice::from_raw_parts(kernel_buffer as *const u8, bytes_read) };

    // Verify ELF magic
    if bytes_read < 4 || &kernel_data[0..4] != b"\x7fELF" {
        return Err(Error::InvalidParameter);
    }

    // Parse ELF header (simplified)
    let entry_point = if kernel_data[4] == 2 {
        // 64-bit ELF
        u64::from_le_bytes([
            kernel_data[24],
            kernel_data[25],
            kernel_data[26],
            kernel_data[27],
            kernel_data[28],
            kernel_data[29],
            kernel_data[30],
            kernel_data[31],
        ])
    } else {
        return Err(Error::Unsupported);
    };

    Ok(LoadedKernel {
        phys_base: PhysicalAddress(kernel_buffer as u64),
        virt_base: KERNEL_LOAD_VIRT,
        size: bytes_read as u64,
        entry_point,
        tls_template: None,
    })
}

// =============================================================================
// MODULE LOADING
// =============================================================================

/// Loaded modules
#[derive(Debug)]
pub struct LoadedModules {
    /// Initrd physical address
    pub initrd_addr: Option<PhysicalAddress>,
    /// Initrd size
    pub initrd_size: u64,
    /// Module count
    pub count: usize,
}

/// Load additional modules
fn load_modules(
    _image_handle: EfiHandle,
    _st: &EfiSystemTable,
    _config: &BootConfig,
) -> Result<LoadedModules> {
    // TODO: Load initrd and other modules
    Ok(LoadedModules {
        initrd_addr: None,
        initrd_size: 0,
        count: 0,
    })
}

// =============================================================================
// MEMORY MAP
// =============================================================================

/// Exit boot services and get final memory map
fn exit_boot_services(
    image_handle: EfiHandle,
    st: &EfiSystemTable,
) -> Result<(Vec<MemoryDescriptor>, *mut EfiRuntimeServices)> {
    let bs = unsafe { &*st.boot_services };
    let runtime = st.runtime_services;

    // Get memory map size
    let mut map_size: usize = 0;
    let mut map_key: usize = 0;
    let mut desc_size: usize = 0;
    let mut desc_version: u32 = 0;

    let _status = unsafe {
        (bs.get_memory_map)(
            &mut map_size,
            core::ptr::null_mut(),
            &mut map_key,
            &mut desc_size,
            &mut desc_version,
        )
    };

    // Add extra space
    map_size += desc_size * 8;

    // Allocate buffer
    let mut buffer: *mut core::ffi::c_void = core::ptr::null_mut();
    let pages = (map_size + 4095) / 4096;

    let status = unsafe {
        (bs.allocate_pages)(
            0,
            2, // EfiLoaderData
            pages,
            &mut buffer as *mut _ as *mut PhysicalAddress,
        )
    };

    if status != EFI_SUCCESS {
        return Err(Error::OutOfResources);
    }

    // Get memory map
    let status = unsafe {
        (bs.get_memory_map)(
            &mut map_size,
            buffer as *mut EfiMemoryDescriptor,
            &mut map_key,
            &mut desc_size,
            &mut desc_version,
        )
    };

    if status != EFI_SUCCESS {
        return Err(Error::from_status(status));
    }

    // Exit boot services
    let status = unsafe { (bs.exit_boot_services)(image_handle, map_key) };

    if status != EFI_SUCCESS {
        // Retry with fresh memory map
        let status = unsafe {
            (bs.get_memory_map)(
                &mut map_size,
                buffer as *mut EfiMemoryDescriptor,
                &mut map_key,
                &mut desc_size,
                &mut desc_version,
            )
        };

        if status != EFI_SUCCESS {
            return Err(Error::from_status(status));
        }

        let status = unsafe { (bs.exit_boot_services)(image_handle, map_key) };

        if status != EFI_SUCCESS {
            return Err(Error::from_status(status));
        }
    }

    BOOT_SERVICES_EXITED.store(true, Ordering::Release);

    // Parse memory map into our format
    let entry_count = map_size / desc_size;
    let mut descriptors = Vec::with_capacity(entry_count);

    for i in 0..entry_count {
        let desc_ptr =
            unsafe { (buffer as *const u8).add(i * desc_size) as *const EfiMemoryDescriptor };
        let desc = unsafe { &*desc_ptr };

        descriptors.push(MemoryDescriptor {
            memory_type: desc.memory_type,
            physical_start: desc.physical_start,
            virtual_start: desc.virtual_start,
            number_of_pages: desc.number_of_pages,
            attribute: desc.attribute,
        });
    }

    Ok((descriptors, runtime))
}

// Memory descriptor for our use
#[derive(Debug, Clone)]
pub struct MemoryDescriptor {
    pub memory_type: u32,
    pub physical_start: PhysicalAddress,
    pub virtual_start: VirtualAddress,
    pub number_of_pages: u64,
    pub attribute: u64,
}

// Minimal Vec implementation for no_std
extern crate alloc;
use alloc::vec::Vec;

// =============================================================================
// PAGING
// =============================================================================

/// Page table setup result
#[derive(Debug)]
pub struct PageTableSetup {
    /// Root page table physical address
    pub root: PhysicalAddress,
}

/// Set up page tables for kernel
fn setup_paging(
    _kernel: &LoadedKernel,
    _memory_map: &[MemoryDescriptor],
    _framebuffer: &FramebufferInfo,
) -> Result<PageTableSetup> {
    // TODO: Build page tables for kernel
    // - Identity map for transition
    // - Higher half mapping for kernel
    // - Physical memory map
    // - Framebuffer mapping

    Ok(PageTableSetup { root: 0 })
}

// =============================================================================
// STACK
// =============================================================================

/// Allocate kernel stack
fn allocate_kernel_stack(_memory_map: &[MemoryDescriptor]) -> Result<VirtualAddress> {
    // TODO: Allocate stack from available memory
    Ok(0)
}

// =============================================================================
// BOOT INFO
// =============================================================================

/// Build boot info structure
fn build_boot_info(
    framebuffer: &FramebufferInfo,
    _memory_map: &[MemoryDescriptor],
    acpi: &AcpiInfo,
    smbios: Option<PhysicalAddress>,
    kernel: &LoadedKernel,
    _modules: &LoadedModules,
    stack_top: VirtualAddress,
    config: &BootConfig,
) -> Result<BootInfo> {
    let boot_timestamp = BOOT_TIMESTAMP.load(Ordering::Relaxed);

    let mut boot_info = BootInfo {
        magic: BOOT_INFO_MAGIC,
        version: (VERSION_MAJOR as u32) << 16 | VERSION_MINOR as u32,
        bootloader_name: [0; 64],
        bootloader_version: [0; 32],
        command_line: [0; 1024],
        memory_map_addr: 0,
        memory_map_size: 0,
        memory_map_entry_size: 0,
        framebuffer: framebuffer.clone(),
        rsdp_addr: acpi.rsdp_addr,
        smbios_addr: smbios.unwrap_or(0),
        efi_system_table: unsafe { SYSTEM_TABLE.unwrap_or(core::ptr::null_mut()) as u64 },
        efi_runtime_services: 0,
        module_count: 0,
        modules_addr: 0,
        kernel_phys_base: kernel.phys_base,
        kernel_virt_base: kernel.virt_base,
        kernel_size: kernel.size,
        kernel_entry: kernel.entry_point,
        tls_template: kernel.tls_template,
        boot_timestamp,
        cpu_count: 1,
        bsp_apic_id: 0,
        dtb_addr: None,
    };

    // Copy bootloader name
    let name = BOOTLOADER_NAME.as_bytes();
    boot_info.bootloader_name[..name.len()].copy_from_slice(name);

    // Copy command line
    let cmdline = &config.cmdline[..config.cmdline_len];
    boot_info.command_line[..cmdline.len()].copy_from_slice(cmdline);

    Ok(boot_info)
}

// =============================================================================
// HANDOFF
// =============================================================================

/// Hand off to kernel
fn handoff_to_kernel(
    kernel: &LoadedKernel,
    boot_info: &BootInfo,
    stack_top: VirtualAddress,
    _page_tables: PageTableSetup,
) -> Result<()> {
    // Entry point function type
    type KernelEntry = extern "sysv64" fn(*const BootInfo) -> !;

    let entry: KernelEntry = unsafe { core::mem::transmute(kernel.entry_point) };

    // Disable interrupts
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }

    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("msr DAIFSet, #3", options(nomem, nostack));
    }

    // TODO: Switch to kernel page tables
    // TODO: Switch to kernel stack

    // Jump to kernel
    entry(boot_info as *const BootInfo)
}

// =============================================================================
// OUTPUT
// =============================================================================

/// Print banner
fn print_banner(st: &EfiSystemTable) -> Result<()> {
    let con_out = unsafe { &*st.con_out };

    // Clear screen
    let _ = unsafe { (con_out.clear_screen)(st.con_out) };

    // Print banner
    let banner: &[u16] = &[
        'H' as u16,
        'e' as u16,
        'l' as u16,
        'i' as u16,
        'x' as u16,
        ' ' as u16,
        'U' as u16,
        'E' as u16,
        'F' as u16,
        'I' as u16,
        ' ' as u16,
        'B' as u16,
        'o' as u16,
        'o' as u16,
        't' as u16,
        'l' as u16,
        'o' as u16,
        'a' as u16,
        'd' as u16,
        'e' as u16,
        'r' as u16,
        '\r' as u16,
        '\n' as u16,
        0,
    ];

    let _ = unsafe { (con_out.output_string)(st.con_out, banner.as_ptr()) };

    Ok(())
}

/// Print CPU information
fn print_cpu_info(_st: &EfiSystemTable, _features: &CpuFeatures) -> Result<()> {
    // TODO: Print CPU features
    Ok(())
}

/// Print ACPI information
fn print_acpi_info(_st: &EfiSystemTable, _acpi: &AcpiInfo) -> Result<()> {
    // TODO: Print ACPI info
    Ok(())
}

/// Print kernel information
fn print_kernel_info(_st: &EfiSystemTable, _kernel: &LoadedKernel) -> Result<()> {
    // TODO: Print kernel info
    Ok(())
}

/// Print error
unsafe fn print_error(st: &EfiSystemTable, error: Error) -> Result<()> {
    let con_out = &*st.con_out;

    let msg: &[u16] = &[
        'E' as u16,
        'r' as u16,
        'r' as u16,
        'o' as u16,
        'r' as u16,
        ':' as u16,
        ' ' as u16,
        '0' as u16 + (error as u16 / 10),
        '0' as u16 + (error as u16 % 10),
        '\r' as u16,
        '\n' as u16,
        0,
    ];

    let _ = (con_out.output_string)(st.con_out, msg.as_ptr());

    Ok(())
}

// =============================================================================
// PANIC HANDLER
// =============================================================================

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Try to print panic message if possible
    unsafe {
        if let Some(st) = SYSTEM_TABLE {
            if !BOOT_SERVICES_EXITED.load(Ordering::Acquire) {
                let st = &*st;
                if !st.con_out.is_null() {
                    let con_out = &*st.con_out;

                    let msg: &[u16] = &[
                        'P' as u16,
                        'A' as u16,
                        'N' as u16,
                        'I' as u16,
                        'C' as u16,
                        '!' as u16,
                        '\r' as u16,
                        '\n' as u16,
                        0,
                    ];

                    let _ = (con_out.output_string)(st.con_out, msg.as_ptr());
                }
            }
        }
    }

    // Halt
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack, noreturn));
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}

// =============================================================================
// ALLOCATOR
// =============================================================================

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

struct DummyAllocator;

unsafe impl core::alloc::GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // In a real implementation, this would use UEFI's allocate_pool
        core::ptr::null_mut()
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // In a real implementation, this would use UEFI's free_pool
    }
}

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("cli; hlt", options(nomem, nostack, noreturn));
        }
    }
}
