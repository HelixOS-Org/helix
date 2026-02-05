//! # Helix UEFI Boot Platform
//!
//! Revolutionary UEFI boot implementation in pure Rust, designed to become
//! the reference implementation for modern, secure, Rust-based operating
//! system bootstrapping.
//!
//! ## Architecture
//!
//! The crate is organized in layers:
//!
//! - **Layer 0 (raw)**: Raw UEFI bindings matching the UEFI specification
//! - **Layer 1 (services)**: Safe wrappers around Boot/Runtime services
//! - **Layer 2 (protocols)**: High-level protocol abstractions
//! - **Layer 3 (tables)**: Hardware table parsing (ACPI, SMBIOS, DTB)
//! - **Layer 4 (security)**: Secure Boot, Measured Boot, signatures
//! - **Layer 5 (handoff)**: Kernel handoff and transition
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! #![no_std]
//! #![no_main]
//!
//! use helix_uefi::prelude::*;
//!
//! #[entry]
//! fn efi_main(image: Handle, st: &SystemTable) -> Status {
//!     let env = UefiEnv::init(image, st)?;
//!     env.console().println("Helix UEFI Boot");
//!
//!     // Load and boot kernel
//!     let kernel = env.load_kernel("\\EFI\\HELIX\\KERNEL")?;
//!     env.boot_kernel(kernel)
//! }
//! ```
//!
//! ## Features
//!
//! - `x86_64` - x86-64 architecture support (default)
//! - `aarch64` - `AArch64` architecture support
//! - `gop` - Graphics Output Protocol
//! - `filesystem` - File system protocols
//! - `acpi` - ACPI table parsing
//! - `smbios` - SMBIOS table parsing
//! - `security` - Security features (Secure Boot, Measured Boot)
//! - `smp` - Multi-processor support
//! - `full` - All features enabled

#![no_std]
#![allow(unsafe_op_in_unsafe_fn)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![feature(alloc_error_handler)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

// =============================================================================
// MODULES
// =============================================================================

/// Raw UEFI bindings (Layer 0)
///
/// Direct mappings to UEFI specification structures and functions.
/// All types here are `#[repr(C)]` and match the UEFI ABI exactly.
pub mod raw;

/// Safe service wrappers (Layer 1)
///
/// Safe Rust wrappers around UEFI Boot Services and Runtime Services.
pub mod services;

/// Protocol abstractions (Layer 2)
///
/// High-level, safe abstractions for UEFI protocols.
pub mod protocols;

/// Hardware table parsing (Layer 3)
///
/// Parsers for ACPI, SMBIOS, and Device Tree tables.
pub mod tables;

/// Security features (Layer 4)
///
/// Secure Boot, Measured Boot, and signature verification.
#[cfg(feature = "security")]
pub mod security;

/// Memory management
///
/// Safe abstractions for UEFI memory management.
pub mod memory;

/// Kernel relocation support
///
/// PIE kernel relocation and KASLR integration.
pub mod relocation;

/// Kernel loading
///
/// ELF/PE loaders and kernel preparation.
pub mod loader;

// =============================================================================
// FORMAT PARSING MODULES
// =============================================================================

/// ELF binary parsing
///
/// Complete ELF64/ELF32 format parser for kernel and module loading.
/// Supports relocations, sections, symbols, and dynamic linking info.
pub mod elf;

/// PE/COFF binary parsing
///
/// Complete PE/COFF format parser for UEFI executables and drivers.
/// Supports relocations, imports, exports, and optional headers.
pub mod pe;

// =============================================================================
// FIRMWARE TABLE MODULES
// =============================================================================

/// ACPI table parsing
///
/// Comprehensive ACPI table discovery and parsing.
/// Supports RSDP, RSDT, XSDT, FADT, MADT, and many more tables.
pub mod acpi;

/// SMBIOS table parsing
///
/// System Management BIOS information tables.
/// Provides access to system, baseboard, processor, and memory information.
pub mod smbios;

// =============================================================================
// DEBUG AND DIAGNOSTICS
// =============================================================================

/// Debug output and logging
///
/// Serial port, debug port, and logging infrastructure.
/// Supports multiple output channels and debug levels.
pub mod debug;

/// System diagnostics
///
/// CPU diagnostics, memory testing, and boot progress tracking.
/// Includes comprehensive self-test capabilities.
pub mod diag;

// =============================================================================
// CORE UTILITY MODULES
// =============================================================================

/// Memory allocators
///
/// Page frame allocators, pool allocators, and heap management.
/// Includes bitmap, bump, stack, and pool allocators.
pub mod mem_alloc;

/// Time and timer services
///
/// RTC access, TSC reading, durations, and timer management.
/// Architecture-aware with x86_64 and AArch64 support.
pub mod time;

/// String utilities
///
/// UCS-2 string handling, formatting, and conversion.
/// Full UTF-8 to UCS-2 conversion support.
pub mod string;

/// GUID utilities
///
/// GUID handling, parsing, and well-known GUIDs.
/// Includes 50+ standard UEFI/Windows/Linux GUIDs.
pub mod guid;

/// Event and synchronization
///
/// UEFI event system and synchronization primitives.
/// Includes mutexes, semaphores, barriers, and event groups.
pub mod event;

/// UEFI variable services
///
/// Variable storage, access, and manipulation.
/// Supports all standard UEFI variable operations.
pub mod variable;

/// PCI/PCIe support
///
/// PCI configuration space access and device enumeration.
/// Supports legacy PCI and PCIe ECAM access.
pub mod pci;

/// Compression utilities
///
/// Compression and decompression for boot payloads.
/// Supports RLE, LZ77, Huffman, GZIP, and more.
pub mod compress;

// =============================================================================
// DEVICE AND PROTOCOL MODULES
// =============================================================================

/// USB protocol support
///
/// Complete USB device enumeration, HID, mass storage, and hub support.
/// Includes USB 1.0/2.0/3.x protocol stacks and xHCI controller support.
pub mod usb;

/// TPM (Trusted Platform Module) support
///
/// TPM 1.2 and TPM 2.0 support for measured boot and attestation.
/// Includes PCR operations, event logging, and NVRAM access.
pub mod tpm;

/// VirtIO device support
///
/// VirtIO 1.0/1.1/1.2 specification support for virtualized environments.
/// Includes block, network, console, and GPU device drivers.
pub mod virtio;

/// NVMe (Non-Volatile Memory Express) support
///
/// Complete NVMe 1.4 specification support for high-performance SSD access.
/// Includes admin/I/O queue management, namespace operations, and SMART data.
pub mod nvme;

/// SD/MMC card support
///
/// SD 2.0-4.0, SDHC/SDXC, eMMC 5.1 specification support.
/// Includes high-speed modes, UHS-I/II, DDR, and command queuing.
pub mod sdmmc;

/// SCSI/SAS/iSCSI support
///
/// Complete SCSI protocol support for enterprise storage access.
/// Includes SPC-5, SBC-4, SAS 4.0, and iSCSI boot support.
pub mod scsi;

/// Audio support (boot chime)
///
/// HD Audio, AC'97, and PC Speaker support for boot feedback.
/// Includes WAV playback, tone generation, and beep code patterns.
pub mod audio;

/// Input device support
///
/// Keyboard, mouse, and touch input with scancode translation.
/// Includes PS/2, USB HID, and international keyboard layouts.
pub mod input;

/// Security chain of trust
///
/// Secure Boot verification, certificate management, and TPM measured boot.
/// Includes Authenticode, PKCS#7, and image verification.
pub mod chain;

/// Boot monitoring and diagnostics
///
/// Performance profiling, health monitoring, and event logging.
/// Includes boot timing, memory tracking, and checkpoint system.
pub mod monitor;

/// Graphics memory and framebuffer
///
/// Framebuffer management, pixel formats, and rendering primitives.
/// Includes fonts, images, progress bars, and boot splash.
pub mod framebuffer;

/// ACPI and power management
///
/// Complete ACPI table parsing and power state management.
/// Includes S-states, C-states, P-states, thermal, and battery.
pub mod power;

// =============================================================================
// BOOT SERVICE MODULES
// =============================================================================

/// Boot configuration
///
/// Boot menu configuration, profile parsing, and settings.
pub mod config;

/// Console services
///
/// Advanced console input/output with color and cursor support.
pub mod console;

/// Boot menu system
///
/// Interactive boot menu with timeout, selection, and themes.
pub mod menu;

/// Network boot
///
/// PXE boot, HTTP boot, and network protocol support.
pub mod network;

/// Device path utilities
///
/// Device path parsing, construction, and manipulation.
pub mod device_path;

/// Graphics support
///
/// Framebuffer, fonts, and graphical boot splash.
pub mod graphics;

/// Filesystem support
///
/// FAT filesystem, ESP access, and file operations.
pub mod filesystem;

/// Runtime services wrapper
///
/// Runtime environment after ExitBootServices.
pub mod runtime;

// =============================================================================
// ADDITIONAL SUBSYSTEMS
// =============================================================================

/// Block device abstraction
///
/// Block device types, partition tables (MBR/GPT), and SMART data.
/// Provides unified access to storage devices.
pub mod block;

/// Network protocol stack
///
/// Complete network stack with IPv4/IPv6, TCP/UDP, DHCP, DNS.
/// Includes PXE boot, TFTP, and HTTP boot support.
pub mod netstack;

/// Terminal emulation
///
/// ANSI/VT100 escape sequences, colors, themes, and box drawing.
/// Includes EFI console color mapping and serial port support.
pub mod terminal;

/// Driver discovery and management
///
/// Device enumeration, driver binding, and device tree.
/// Includes PCI/USB/ACPI device identification.
pub mod drivers;

/// Boot animation system
///
/// Easing functions, keyframes, sprites, and particle effects.
/// Includes progress bars, spinners, and splash screens.
pub mod animation;

/// Cryptographic primitives
///
/// Hash functions, digital signatures, and encryption.
/// Includes SHA-2, RSA, ECDSA, AES, and X.509 support.
pub mod crypto;

/// Internationalization and localization
///
/// Language support, translations, and locale-aware formatting.
/// Includes keyboard layouts, date/time formats, and Unicode utilities.
pub mod locale;

/// Testing and validation framework
///
/// Hardware tests, boot verification, and diagnostic tests.
/// Includes memory, CPU, storage, network, and security testing.
pub mod testing;

/// Boot settings and configuration
///
/// Persistent boot settings, display preferences, and security options.
/// Includes network, storage, debug, and power settings.
pub mod settings;

/// Boot entries and menu system
///
/// Boot entry management, menu configuration, and boot order.
/// Includes Linux, EFI, chainload, PXE, and recovery entries.
pub mod entries;

/// Boot profiles and presets
///
/// Hardware profiles, user presets, and environment configurations.
/// Includes quick boot, safe mode, debug, and recovery presets.
pub mod profiles;

/// Boot events and notifications
///
/// Event-driven boot process management with lifecycle events.
/// Includes progress, error, device, security, and user events.
pub mod events;

/// Resource and asset management
///
/// Fonts, icons, images, themes, and string resources.
/// Includes caching, formats, and resource bundles.
pub mod resources;

/// UI layout and widget system
///
/// Layout engine, widgets, and rendering primitives.
/// Includes flex layout, text, buttons, lists, and progress bars.
pub mod layout;

/// Error handling and recovery
///
/// Comprehensive error handling, recovery strategies, and fallback mechanisms.
/// Includes error codes, beep patterns, error logs, and recovery screens.
pub mod recovery;

/// Command and action system
///
/// User commands, keyboard bindings, and action handlers.
/// Includes boot, menu, config, system, and debug commands.
pub mod commands;

/// Boot orchestration
///
/// Core boot orchestration and phase management.
/// Includes phase tracking, state machine, hooks, and kernel handoff.
pub mod orchestrator;

/// Validation and integrity
///
/// File, memory, and configuration validation.
/// Includes checksums, signatures, pre-boot checks, and hardware requirements.
pub mod validate;

/// System information and diagnostics
///
/// Hardware discovery and system reporting.
/// Includes CPU, memory, storage, firmware, graphics, and network info.
pub mod sysinfo;

/// Parsing and formatting utilities
///
/// Number parsing, size formatting, path utilities, and config parsing.
/// Includes GUID formatting, duration, and string manipulation.
pub mod parse;

/// Theme and appearance
///
/// Visual theming, color schemes, and UI styling.
/// Includes color management, fonts, component styles, and theme switching.
pub mod theme;

/// Performance monitoring
///
/// Timing, statistics, and performance tracking.
/// Includes phase timing, memory perf, I/O throughput, and benchmarks.
pub mod perf;

/// Splash screen and visual boot
///
/// Splash screen rendering and boot animations.
/// Includes progress visualization, spinners, and transitions.
pub mod splash;

/// Help and documentation
///
/// Contextual help, keyboard shortcuts, and user guidance.
/// Includes help topics, quick reference, and tips.
pub mod help;

/// FAT32 filesystem
///
/// FAT12/16/32 filesystem parsing and file access.
/// Includes BPB parsing, directory entries, LFN support, and path utilities.
pub mod fat32;

/// Boot manager core
///
/// Main boot manager logic and state machine.
/// Includes entry management, timeout, selection, and boot orchestration.
pub mod bootmgr;

/// Network boot support
///
/// PXE, HTTP/HTTPS boot, TFTP, and DHCP.
/// Includes network configuration and download management.
pub mod netboot;

/// Partition table support
///
/// GPT and MBR partition table parsing.
/// Includes GUID handling, partition type identification, and ESP detection.
pub mod partition;

// =============================================================================
// HANDOFF AND ARCHITECTURE
// =============================================================================

/// Kernel handoff (Layer 5)
///
/// Boot information structures and ExitBootServices handling.
pub mod handoff;

/// Architecture-specific code
pub mod arch;

/// Error types
pub mod error;

// =============================================================================
// GLOBAL ALLOCATOR
// =============================================================================

#[global_allocator]
static ALLOCATOR: mem_alloc::BootAllocator = mem_alloc::BootAllocator::new();

#[alloc_error_handler]
fn alloc_error(_layout: core::alloc::Layout) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

// =============================================================================
// RE-EXPORTS
// =============================================================================

pub use error::{Error, Result};
pub use handoff::BootInfo;
pub use raw::system_table::{EfiSystemTable, SystemTablePtr};
pub use raw::types::{Guid, Handle, PhysicalAddress, Status, VirtualAddress};
pub use services::boot::BootServices;
pub use services::runtime::RuntimeServices;

/// Alias for `EfiSystemTable` for convenience
pub type SystemTable = EfiSystemTable;

// =============================================================================
// PRELUDE
// =============================================================================

/// Prelude module for convenient imports
///
/// Common imports for UEFI applications.
///
/// ```rust
/// use helix_uefi::prelude::*;
/// ```
pub mod prelude {

    // ACPI tables
    pub use crate::acpi::SdtHeader;
    // Compression
    pub use crate::compress::{rle_compress, rle_decompress, CompressionType};
    // Debug utilities
    pub use crate::debug::SerialPort;
    // Diagnostics
    pub use crate::diag::{BootProgress, BootStage};
    // ELF parsing
    pub use crate::elf::{Elf64Header, Elf64ProgramHeader};
    // Event utilities
    pub use crate::event::{Event, EventType, Mutex, Semaphore, Tpl};
    // GUID utilities
    pub use crate::guid::Guid as GuidExt;
    // Allocator utilities
    pub use crate::mem_alloc::{AllocError, BitmapAllocator, PoolAllocator};
    // PCI utilities
    pub use crate::pci::{Bar, PciAddress, PciConfigHeader};
    // PE parsing
    pub use crate::pe::DosHeader;
    // Console
    #[cfg(feature = "simple_text")]
    pub use crate::protocols::console::{Console, InputKey};
    // Filesystem
    #[cfg(feature = "filesystem")]
    pub use crate::protocols::filesystem::{File, FileInfo, FileSystem};
    // Graphics
    #[cfg(feature = "gop")]
    pub use crate::protocols::graphics::{GraphicsOutput, ModeInfo, PixelFormat};
    // Protocol traits
    pub use crate::protocols::Protocol;
    // SMBIOS tables
    pub use crate::smbios::{Smbios2EntryPoint, SmbiosTable};
    // String utilities
    pub use crate::string::{CStr16, Char16, String16};
    // Time utilities
    pub use crate::time::{Duration, Instant, Time, Timer};
    // Variable utilities
    pub use crate::variable::{VariableAttributes, VariableStorage};
    // Environment
    pub use crate::UefiEnv;
    pub use crate::{
        // Entry macro
        entry,
        // Handoff
        BootInfo,

        BootServices,
        // Error handling
        Error,
        Guid,
        // Core types
        Handle,
        // Memory
        PhysicalAddress,
        Result,

        RuntimeServices,

        Status,
        SystemTablePtr,
        VirtualAddress,
    };
}

// =============================================================================
// UEFI ENVIRONMENT
// =============================================================================

use core::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

/// Global system table pointer (set by entry point)
static SYSTEM_TABLE: AtomicPtr<raw::system_table::EfiSystemTable> =
    AtomicPtr::new(core::ptr::null_mut());

/// Global image handle (set by entry point)
static IMAGE_HANDLE: AtomicPtr<core::ffi::c_void> = AtomicPtr::new(core::ptr::null_mut());

/// Boot services exited flag
static BOOT_SERVICES_EXITED: AtomicBool = AtomicBool::new(false);

/// UEFI Environment
///
/// Main interface to the UEFI firmware. Provides access to all services,
/// protocols, and tables in a safe, Rust-idiomatic way.
///
/// # Example
///
/// ```rust,no_run
/// use helix_uefi::prelude::*;
///
/// fn boot(image: Handle, st: &SystemTable) -> Result<()> {
///     let env = UefiEnv::init(image, st)?;
///
///     // Use console
///     env.console()?.println("Hello, UEFI!");
///
///     // Access boot services
///     let memory_map = env.boot_services().memory_map()?;
///
///     Ok(())
/// }
/// ```
pub struct UefiEnv {
    image_handle: Handle,
    system_table: &'static SystemTable,
}

impl UefiEnv {
    /// Initialize the UEFI environment
    ///
    /// # Safety
    ///
    /// This must be called exactly once, at the start of the UEFI application,
    /// with the handle and system table provided by the firmware.
    pub unsafe fn init(
        image_handle: Handle,
        system_table: *const raw::system_table::EfiSystemTable,
    ) -> Result<Self> {
        if system_table.is_null() {
            return Err(Error::InvalidParameter);
        }

        // Store global pointers
        SYSTEM_TABLE.store(system_table.cast_mut(), Ordering::Release);
        IMAGE_HANDLE.store(image_handle.as_ptr().cast(), Ordering::Release);

        // Validate system table
        let st = &*system_table.cast::<SystemTable>();
        if !st.validate() {
            return Err(Error::InvalidParameter);
        }

        Ok(Self {
            image_handle,
            system_table: &*system_table.cast::<SystemTable>(),
        })
    }

    /// Get the image handle
    pub const fn image_handle(&self) -> Handle {
        self.image_handle
    }

    /// Get the system table
    pub const fn system_table(&self) -> &SystemTable {
        self.system_table
    }

    /// Get Boot Services
    ///
    /// # Panics
    ///
    /// Panics if called after `exit_boot_services()`.
    pub fn boot_services(&self) -> Option<&raw::boot_services::EfiBootServices> {
        assert!(
            !BOOT_SERVICES_EXITED.load(Ordering::Acquire),
            "Boot services have been exited"
        );
        unsafe { self.system_table.boot_services() }
    }

    /// Get Runtime Services
    pub fn runtime_services(&self) -> Option<&raw::runtime_services::EfiRuntimeServices> {
        unsafe { self.system_table.runtime_services() }
    }

    /// Get console for text I/O
    #[cfg(feature = "simple_text")]
    pub fn console(&self) -> Result<protocols::console::Console<'_>> {
        protocols::console::Console::new(self.system_table)
    }

    /// Get graphics output
    #[cfg(feature = "gop")]
    pub const fn graphics(&self) -> Result<protocols::graphics::GraphicsOutput> {
        // TODO: Implement properly via boot services protocol location
        Err(Error::Unsupported)
    }

    /// Get file system access
    #[cfg(feature = "filesystem")]
    pub const fn filesystem(&self) -> Result<protocols::filesystem::FileSystem> {
        // TODO: Implement properly via boot services protocol location
        Err(Error::Unsupported)
    }

    /// Get ACPI tables
    #[cfg(feature = "acpi")]
    pub fn acpi(&self) -> Result<protocols::acpi::AcpiTables> {
        Ok(protocols::acpi::AcpiTables::new(self.image_handle))
    }

    /// Get SMBIOS tables
    #[cfg(feature = "smbios")]
    pub fn smbios(&self) -> Result<protocols::smbios::SmbiosTables> {
        Ok(protocols::smbios::SmbiosTables::new(self.image_handle))
    }

    /// Load a kernel from the file system
    #[cfg(feature = "filesystem")]
    pub fn load_kernel(&self, path: &str) -> Result<loader::LoadedImage> {
        let mut kernel_loader = loader::KernelLoader::new();
        let _ = kernel_loader.load_file(path)?;
        kernel_loader.image().cloned().ok_or(Error::NotFound)
    }

    /// Exit boot services and prepare kernel handoff
    ///
    /// After this call, Boot Services are no longer available.
    pub fn exit_boot_services(self) -> Result<handoff::BootInfo> {
        // Mark boot services as exited
        BOOT_SERVICES_EXITED.store(true, Ordering::Release);
        Ok(handoff::BootInfo::default())
    }
}

// =============================================================================
// ENTRY POINT MACRO
// =============================================================================

/// Entry point attribute for UEFI applications
///
/// This macro generates the correct entry point signature expected by
/// UEFI firmware and handles the initial setup.
///
/// # Example
///
/// ```rust,no_run
/// #![no_std]
/// #![no_main]
///
/// use helix_uefi::prelude::*;
///
/// #[entry]
/// fn efi_main(image: Handle, st: &SystemTable) -> Status {
///     // Your code here
///     Status::SUCCESS
/// }
/// ```
#[macro_export]
macro_rules! entry {
    ($entry:ident) => {
        #[no_mangle]
        pub extern "efiapi" fn efi_main(
            image_handle: $crate::Handle,
            system_table: *const $crate::raw::system_table::EfiSystemTable,
        ) -> $crate::Status {
            // Initialize environment
            let env = match unsafe { $crate::UefiEnv::init(image_handle, system_table) } {
                Ok(env) => env,
                Err(e) => return e.into(),
            };

            // Call user entry
            match $entry(env) {
                Ok(()) => $crate::Status::SUCCESS,
                Err(e) => e.into(),
            }
        }
    };
}

// =============================================================================
// PANIC HANDLER (for standalone builds)
// =============================================================================

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Try to print to console
    let st_ptr = SYSTEM_TABLE.load(Ordering::Acquire);
    if !st_ptr.is_null() {
        unsafe {
            let console = protocols::console::Console::new(st_ptr);
            let _ = console.set_color(
                protocols::console::Color::RED,
                protocols::console::Color::BLACK,
            );
            let _ = console.write("\r\n\r\n*** PANIC ***\r\n");
            if let Some(location) = info.location() {
                let _ = console.write("Location: ");
                let _ = console.write(location.file());
                let _ = console.write(":");
                // Note: Would need itoa for line number
                let _ = console.write("\r\n");
            }
        }
    }

    // Halt
    loop {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            core::arch::asm!("hlt");
        };

        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("wfi");
        };
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn test_status_values() {
        assert_eq!(Status::SUCCESS.0, 0);
        assert!(Status::SUCCESS.is_success());
    }

    #[test]
    fn test_guid_creation() {
        let guid = raw::types::Guid::new(0x1234_5678, 0x1234, 0x5678, [
            0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
        ]);
        assert_eq!(guid.data1, 0x1234_5678);
    }
}
