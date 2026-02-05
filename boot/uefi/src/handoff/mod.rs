//! Kernel Handoff Module
//!
//! This module provides comprehensive boot information structures for passing
//! critical system data from the UEFI bootloader to the kernel. It includes
//! memory maps, framebuffer information, ACPI tables, and more.
//!
//! # Architecture
//!
//! The handoff module creates a unified boot information structure that
//! is format-agnostic and can be used by any kernel implementation.
//!
//! # Features
//!
//! - Complete memory map information
//! - Framebuffer and graphics mode data
//! - ACPI table pointers
//! - SMBIOS information
//! - Kernel command line
//! - Module/initrd information
//! - RSDP location
//! - EFI runtime services

#![allow(dead_code)]

pub mod bootinfo;
pub mod framebuffer;
pub mod memory_map;
pub mod modules;
pub mod rsdp;

pub use bootinfo::*;
pub use framebuffer::*;
pub use memory_map::*;
pub use modules::*;
pub use rsdp::*;

use crate::error::{Error, Result};
use crate::raw::types::*;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// HANDOFF BUILDER
// =============================================================================

/// Boot handoff builder
pub struct HandoffBuilder {
    /// Boot info being constructed
    boot_info: BootInfo,
    /// Memory map entries
    memory_entries: Vec<MemoryMapEntry>,
    /// Modules
    modules: Vec<ModuleInfo>,
    /// Command line
    command_line: String,
}

impl HandoffBuilder {
    /// Create new handoff builder
    pub fn new() -> Self {
        Self {
            boot_info: BootInfo::new(),
            memory_entries: Vec::new(),
            modules: Vec::new(),
            command_line: String::new(),
        }
    }

    /// Set bootloader name
    pub fn bootloader_name(mut self, name: &str) -> Self {
        self.boot_info.bootloader_name = String::from(name);
        self
    }

    /// Set bootloader version
    pub fn bootloader_version(mut self, version: &str) -> Self {
        self.boot_info.bootloader_version = String::from(version);
        self
    }

    /// Set command line
    pub fn command_line(mut self, cmdline: &str) -> Self {
        self.command_line = String::from(cmdline);
        self
    }

    /// Add memory region
    pub fn add_memory_region(
        mut self,
        base: PhysicalAddress,
        size: u64,
        memory_type: MemoryType,
    ) -> Self {
        self.memory_entries.push(MemoryMapEntry {
            physical_start: base,
            virtual_start: VirtualAddress(0),
            page_count: size / 0x1000,
            memory_type,
            attributes: MemoryAttributes::default(),
        });
        self
    }

    /// Add memory region with attributes
    pub fn add_memory_region_with_attrs(
        mut self,
        base: PhysicalAddress,
        size: u64,
        memory_type: MemoryType,
        attributes: MemoryAttributes,
    ) -> Self {
        self.memory_entries.push(MemoryMapEntry {
            physical_start: base,
            virtual_start: VirtualAddress(0),
            page_count: size / 0x1000,
            memory_type,
            attributes,
        });
        self
    }

    /// Set framebuffer info
    pub fn framebuffer(mut self, fb: FramebufferInfo) -> Self {
        self.boot_info.framebuffer = Some(fb);
        self
    }

    /// Set RSDP address
    pub fn rsdp(mut self, address: PhysicalAddress) -> Self {
        self.boot_info.rsdp_address = Some(address);
        self
    }

    /// Set SMBIOS address
    pub fn smbios(mut self, address: PhysicalAddress) -> Self {
        self.boot_info.smbios_address = Some(address);
        self
    }

    /// Set EFI system table
    pub fn efi_system_table(mut self, address: PhysicalAddress) -> Self {
        self.boot_info.efi_system_table = Some(address);
        self
    }

    /// Set EFI memory map address
    pub fn efi_memory_map(mut self, address: PhysicalAddress, size: u64, desc_size: u64) -> Self {
        self.boot_info.efi_memory_map = Some(address);
        self.boot_info.efi_memory_map_size = size;
        self.boot_info.efi_memory_descriptor_size = desc_size;
        self
    }

    /// Add module
    pub fn add_module(mut self, module: ModuleInfo) -> Self {
        self.modules.push(module);
        self
    }

    /// Set kernel physical address
    pub fn kernel_physical(mut self, address: PhysicalAddress) -> Self {
        self.boot_info.kernel_physical_address = Some(address);
        self
    }

    /// Set kernel virtual address
    pub fn kernel_virtual(mut self, address: VirtualAddress) -> Self {
        self.boot_info.kernel_virtual_address = Some(address);
        self
    }

    /// Set kernel size
    pub fn kernel_size(mut self, size: u64) -> Self {
        self.boot_info.kernel_size = size;
        self
    }

    /// Set physical memory offset
    pub fn physical_memory_offset(mut self, offset: u64) -> Self {
        self.boot_info.physical_memory_offset = Some(offset);
        self
    }

    /// Set recursive page table index
    pub fn recursive_index(mut self, index: u16) -> Self {
        self.boot_info.recursive_index = Some(index);
        self
    }

    /// Set TLS template
    pub fn tls_template(mut self, start: VirtualAddress, file_size: u64, mem_size: u64) -> Self {
        self.boot_info.tls_template = Some(TlsTemplate {
            start_addr: start,
            file_size,
            mem_size,
        });
        self
    }

    /// Build boot info
    pub fn build(mut self) -> BootInfo {
        // Set command line
        self.boot_info.command_line = self.command_line;

        // Build memory map
        self.boot_info.memory_map = MemoryMap {
            entries: self.memory_entries,
        };

        // Set modules
        self.boot_info.modules = self.modules;

        self.boot_info
    }
}

impl Default for HandoffBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HANDOFF SERIALIZER
// =============================================================================

/// Serialize boot info to memory
pub struct HandoffSerializer {
    /// Buffer
    buffer: Vec<u8>,
    /// Current offset
    offset: usize,
}

impl HandoffSerializer {
    /// Create new serializer
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            offset: 0,
        }
    }

    /// Create with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            offset: 0,
        }
    }

    /// Serialize boot info
    pub fn serialize(&mut self, info: &BootInfo) -> Result<&[u8]> {
        self.buffer.clear();
        self.offset = 0;

        // Write magic
        self.write_u32(BOOT_INFO_MAGIC)?;

        // Write version
        self.write_u32(BOOT_INFO_VERSION)?;

        // Write total size placeholder
        let size_offset = self.offset;
        self.write_u32(0)?;

        // Write flags
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
        self.write_u64(flags)?;

        // Write command line
        self.write_string(&info.command_line)?;

        // Write memory map
        self.write_u64(info.memory_map.entries.len() as u64)?;
        for entry in &info.memory_map.entries {
            self.write_u64(entry.physical_start.0)?;
            self.write_u64(entry.page_count * 4096)?; // size in bytes
            self.write_u32(entry.memory_type as u32)?;
            self.write_u32(0)?; // Reserved
        }

        // Write framebuffer if present
        if let Some(ref fb) = info.framebuffer {
            self.write_u64(fb.address.0)?;
            self.write_u32(fb.width)?;
            self.write_u32(fb.height)?;
            self.write_u32(fb.stride)?;
            self.write_u8(fb.bpp)?;
            self.write_u8(fb.format as u8)?;
            self.write_u16(0)?; // Reserved
        }

        // Write RSDP if present
        if let Some(addr) = info.rsdp_address {
            self.write_u64(addr.0)?;
        }

        // Write SMBIOS if present
        if let Some(addr) = info.smbios_address {
            self.write_u64(addr.0)?;
        }

        // Write EFI system table if present
        if let Some(addr) = info.efi_system_table {
            self.write_u64(addr.0)?;
        }

        // Write modules
        self.write_u64(info.modules.len() as u64)?;
        for module in &info.modules {
            self.write_u64(module.physical_address.0)?;
            self.write_u64(module.size)?;
            self.write_string(&module.name)?;
        }

        // Update total size
        let total_size = self.offset as u32;
        self.buffer[size_offset..size_offset + 4].copy_from_slice(&total_size.to_le_bytes());

        Ok(&self.buffer)
    }

    /// Write u8
    fn write_u8(&mut self, value: u8) -> Result<()> {
        self.buffer.push(value);
        self.offset += 1;
        Ok(())
    }

    /// Write u16
    fn write_u16(&mut self, value: u16) -> Result<()> {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self.offset += 2;
        Ok(())
    }

    /// Write u32
    fn write_u32(&mut self, value: u32) -> Result<()> {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self.offset += 4;
        Ok(())
    }

    /// Write u64
    fn write_u64(&mut self, value: u64) -> Result<()> {
        self.buffer.extend_from_slice(&value.to_le_bytes());
        self.offset += 8;
        Ok(())
    }

    /// Write string
    fn write_string(&mut self, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        self.write_u32(bytes.len() as u32)?;
        self.buffer.extend_from_slice(bytes);
        self.offset += bytes.len();

        // Align to 4 bytes
        let padding = (4 - (bytes.len() % 4)) % 4;
        for _ in 0..padding {
            self.buffer.push(0);
            self.offset += 1;
        }

        Ok(())
    }

    /// Get buffer
    pub fn buffer(&self) -> &[u8] {
        &self.buffer
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.offset
    }
}

impl Default for HandoffSerializer {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// HANDOFF PLACEMENT
// =============================================================================

/// Boot info placement in memory
pub struct HandoffPlacement {
    /// Physical address where boot info will be placed
    pub physical_address: PhysicalAddress,
    /// Virtual address (if mapped)
    pub virtual_address: Option<VirtualAddress>,
    /// Reserved size
    pub reserved_size: u64,
    /// Actual size
    pub actual_size: u64,
}

impl HandoffPlacement {
    /// Create new placement
    pub fn new(physical_address: PhysicalAddress, reserved_size: u64) -> Self {
        Self {
            physical_address,
            virtual_address: None,
            reserved_size,
            actual_size: 0,
        }
    }

    /// Set virtual address
    pub fn with_virtual(mut self, virtual_address: VirtualAddress) -> Self {
        self.virtual_address = Some(virtual_address);
        self
    }

    /// Check if fits
    pub fn fits(&self, size: u64) -> bool {
        size <= self.reserved_size
    }

    /// Write boot info to memory
    pub unsafe fn write(&mut self, serializer: &HandoffSerializer) -> Result<()> {
        let data = serializer.buffer();

        if data.len() as u64 > self.reserved_size {
            return Err(Error::OutOfMemory);
        }

        let ptr = self.physical_address.0 as *mut u8;
        core::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());

        self.actual_size = data.len() as u64;

        Ok(())
    }
}

// =============================================================================
// CONSTANTS
// =============================================================================

/// Boot info magic number
pub const BOOT_INFO_MAGIC: u32 = 0x4845_4C58; // "HELX"

/// Boot info version
pub const BOOT_INFO_VERSION: u32 = 1;

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_handoff_builder() {
        let info = HandoffBuilder::new()
            .bootloader_name("Helix UEFI Bootloader")
            .bootloader_version("1.0.0")
            .command_line("debug root=/dev/sda1")
            .add_memory_region(0x1000, 0x100000, MemoryType::Usable)
            .build();

        assert_eq!(info.bootloader_name, "Helix UEFI Bootloader");
        assert_eq!(info.command_line, "debug root=/dev/sda1");
        assert_eq!(info.memory_map.entries.len(), 1);
    }

    #[test]
    fn test_serializer() {
        let info = HandoffBuilder::new().command_line("test").build();

        let mut serializer = HandoffSerializer::new();
        let data = serializer.serialize(&info).unwrap();

        // Check magic
        assert_eq!(
            u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            BOOT_INFO_MAGIC
        );

        // Check version
        assert_eq!(
            u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            BOOT_INFO_VERSION
        );
    }
}
