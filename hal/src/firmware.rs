//! # Firmware Interface Abstraction
//!
//! This module defines traits for firmware interaction (ACPI, Device Tree, etc.)

use alloc::vec::Vec;

use crate::{HalResult, PhysAddr};

/// Firmware interface abstraction
pub trait FirmwareInterface: Send + Sync {
    /// Get the firmware type
    fn firmware_type(&self) -> FirmwareType;

    /// Get the firmware version string
    fn firmware_version(&self) -> Option<&str>;

    /// Get the memory map from firmware
    fn memory_map(&self) -> Vec<crate::mmu::MemoryRegion>;

    /// Get ACPI tables (if available)
    fn acpi_rsdp(&self) -> Option<PhysAddr>;

    /// Get Device Tree blob (if available)
    fn device_tree_blob(&self) -> Option<&[u8]>;

    /// Get command line arguments
    fn command_line(&self) -> Option<&str>;

    /// Get the framebuffer info (if available)
    fn framebuffer(&self) -> Option<FramebufferInfo>;

    /// Get the boot time (if available)
    fn boot_time(&self) -> Option<BootTime>;

    /// Request system reboot through firmware
    fn request_reboot(&self) -> HalResult<()>;

    /// Request system shutdown through firmware
    fn request_shutdown(&self) -> HalResult<()>;

    /// Get EFI runtime services (if available)
    fn efi_runtime_services(&self) -> Option<PhysAddr>;
}

/// Firmware type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirmwareType {
    /// Legacy BIOS
    Bios,
    /// UEFI
    Uefi,
    /// Device Tree based (ARM, RISC-V)
    DeviceTree,
    /// OpenFirmware
    OpenFirmware,
    /// Unknown/Other
    Unknown,
}

/// Framebuffer information
#[derive(Debug, Clone, Copy)]
pub struct FramebufferInfo {
    /// Physical address of the framebuffer
    pub address: PhysAddr,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pitch (bytes per row)
    pub pitch: u32,
    /// Bits per pixel
    pub bpp: u8,
    /// Pixel format
    pub format: PixelFormat,
}

/// Pixel format for framebuffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// RGB (red, green, blue)
    Rgb,
    /// BGR (blue, green, red)
    Bgr,
    /// RGBA (with alpha channel)
    Rgba,
    /// BGRA (with alpha channel)
    Bgra,
    /// Unknown format
    Unknown,
}

/// Boot time information
#[derive(Debug, Clone, Copy)]
pub struct BootTime {
    /// Year (e.g., 2025)
    pub year: u16,
    /// Month (1-12)
    pub month: u8,
    /// Day (1-31)
    pub day: u8,
    /// Hour (0-23)
    pub hour: u8,
    /// Minute (0-59)
    pub minute: u8,
    /// Second (0-59)
    pub second: u8,
}

/// SMBIOS entry point
#[derive(Debug, Clone)]
pub struct SmbiosInfo {
    /// Entry point address
    pub entry_point: PhysAddr,
    /// SMBIOS version
    pub version: (u8, u8),
    /// Table address
    pub table_address: PhysAddr,
    /// Table length
    pub table_length: u32,
}

/// CPU information from firmware
#[derive(Debug, Clone)]
pub struct FirmwareCpuInfo {
    /// CPU ID
    pub id: u32,
    /// ACPI processor ID (if applicable)
    pub acpi_id: Option<u32>,
    /// Is this the bootstrap processor?
    pub is_bsp: bool,
    /// Is this CPU enabled?
    pub is_enabled: bool,
}
