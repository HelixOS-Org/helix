//! Boot Modules
//!
//! Structures for passing loaded modules (initrd, kernel modules, etc.) to the kernel.

use crate::error::{Error, Result};
use crate::raw::types::{PhysicalAddress, VirtualAddress};

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// MODULE TYPE
// =============================================================================

/// Module type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ModuleType {
    /// Unknown module type
    Unknown      = 0,
    /// Initial ramdisk (initrd/initramfs)
    Initrd       = 1,
    /// Kernel module (.ko)
    KernelModule = 2,
    /// Device tree blob
    DeviceTree   = 3,
    /// ACPI table
    AcpiTable    = 4,
    /// Microcode update
    Microcode    = 5,
    /// Configuration file
    Config       = 6,
    /// Firmware image
    Firmware     = 7,
    /// ELF binary
    Elf          = 8,
    /// Symbol table
    SymbolTable  = 9,
    /// Debug information
    DebugInfo    = 10,
    /// Splash screen image
    SplashScreen = 11,
    /// Custom module type
    Custom       = 0xFFFF,
}

impl ModuleType {
    /// Detect from filename
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    pub fn from_filename(name: &str) -> Self {
        // Case-sensitivity is intentional: we've already lowercased the input
        let name_lower = name.to_lowercase();

        if name_lower.contains("initrd") || name_lower.contains("initramfs") {
            ModuleType::Initrd
        } else if name_lower.ends_with(".ko")
            || name_lower.ends_with(".ko.xz")
            || name_lower.ends_with(".ko.zst")
        {
            ModuleType::KernelModule
        } else if name_lower.ends_with(".dtb") || name_lower.ends_with(".dts") {
            ModuleType::DeviceTree
        } else if name_lower.contains("acpi") && name_lower.ends_with(".aml") {
            ModuleType::AcpiTable
        } else if name_lower.contains("microcode") || name_lower.contains("ucode") {
            ModuleType::Microcode
        } else if name_lower.ends_with(".conf")
            || name_lower.ends_with(".cfg")
            || name_lower.ends_with(".toml")
        {
            ModuleType::Config
        } else if name_lower.ends_with(".elf") {
            ModuleType::Elf
        } else if name_lower.ends_with(".sym") || name_lower.ends_with(".map") {
            ModuleType::SymbolTable
        } else if name_lower.ends_with(".debug") || name_lower.ends_with(".dwarf") {
            ModuleType::DebugInfo
        } else if name_lower.ends_with(".bmp")
            || name_lower.ends_with(".tga")
            || name_lower.ends_with(".png")
        {
            ModuleType::SplashScreen
        } else {
            ModuleType::Unknown
        }
    }

    /// Detect from magic bytes
    pub fn from_magic(data: &[u8]) -> Self {
        if data.len() < 4 {
            return ModuleType::Unknown;
        }

        // Check common magic bytes
        match &data[0..4] {
            // ELF magic
            [0x7F, b'E', b'L', b'F'] => ModuleType::Elf,
            // Gzip (compressed initrd)
            [0x1F, 0x8B, ..] => ModuleType::Initrd,
            // XZ
            [0xFD, b'7', b'z', b'X'] => ModuleType::Initrd,
            // Zstd
            [0x28, 0xB5, 0x2F, 0xFD] => ModuleType::Initrd,
            // CPIO (uncompressed initrd)
            [b'0', b'7', b'0', b'7'] => ModuleType::Initrd,
            // FDT magic (device tree)
            [0xD0, 0x0D, 0xFE, 0xED] => ModuleType::DeviceTree,
            // ACPI DSDT/SSDT
            [b'D' | b'S', b'S', b'D', b'T'] => ModuleType::AcpiTable,
            _ => ModuleType::Unknown,
        }
    }

    /// Get type name
    pub fn name(&self) -> &'static str {
        match self {
            ModuleType::Unknown => "Unknown",
            ModuleType::Initrd => "Initial Ramdisk",
            ModuleType::KernelModule => "Kernel Module",
            ModuleType::DeviceTree => "Device Tree",
            ModuleType::AcpiTable => "ACPI Table",
            ModuleType::Microcode => "Microcode",
            ModuleType::Config => "Configuration",
            ModuleType::Firmware => "Firmware",
            ModuleType::Elf => "ELF Binary",
            ModuleType::SymbolTable => "Symbol Table",
            ModuleType::DebugInfo => "Debug Info",
            ModuleType::SplashScreen => "Splash Screen",
            ModuleType::Custom => "Custom",
        }
    }
}

// =============================================================================
// MODULE FLAGS
// =============================================================================

/// Module flags
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ModuleFlags(pub u32);

impl ModuleFlags {
    /// Module is required (boot fails without it)
    pub const REQUIRED: Self = Self(1 << 0);
    /// Module is compressed
    pub const COMPRESSED: Self = Self(1 << 1);
    /// Module is encrypted
    pub const ENCRYPTED: Self = Self(1 << 2);
    /// Module has been verified
    pub const VERIFIED: Self = Self(1 << 3);
    /// Module should be loaded at specific address
    pub const FIXED_ADDRESS: Self = Self(1 << 4);
    /// Module is executable
    pub const EXECUTABLE: Self = Self(1 << 5);
    /// Module is writable
    pub const WRITABLE: Self = Self(1 << 6);
    /// Module contains relocations
    pub const RELOCATABLE: Self = Self(1 << 7);
    /// Module is page-aligned
    pub const PAGE_ALIGNED: Self = Self(1 << 8);
    /// Module is early-load (before memory subsystem)
    pub const EARLY_LOAD: Self = Self(1 << 9);

    /// Empty flags
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Check if flag is set
    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    /// Set flag
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    /// Clear flag
    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }

    /// Combine flags
    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

// =============================================================================
// MODULE INFO
// =============================================================================

/// Module information
#[derive(Debug, Clone)]
pub struct ModuleInfo {
    /// Module name
    pub name: String,
    /// Physical address where module is loaded
    pub physical_address: PhysicalAddress,
    /// Virtual address (if mapped)
    pub virtual_address: VirtualAddress,
    /// Size in bytes
    pub size: u64,
    /// Module type
    pub module_type: ModuleType,
    /// Module flags
    pub flags: ModuleFlags,
    /// Command line / arguments for this module
    pub cmdline: String,
    /// Hash of module content (SHA-256)
    pub hash: [u8; 32],
    /// Load order
    pub load_order: u32,
}

impl ModuleInfo {
    /// Create new module info
    pub fn new(name: String, physical_address: PhysicalAddress, size: u64) -> Self {
        Self {
            name,
            physical_address,
            virtual_address: VirtualAddress(0),
            size,
            module_type: ModuleType::Unknown,
            flags: ModuleFlags::empty(),
            cmdline: String::new(),
            hash: [0; 32],
            load_order: 0,
        }
    }

    /// Get end address
    pub fn end(&self) -> PhysicalAddress {
        self.physical_address + self.size
    }

    /// Check if address is within module
    pub fn contains(&self, addr: PhysicalAddress) -> bool {
        addr >= self.physical_address && addr < self.end()
    }

    /// Get page count
    pub fn page_count(&self) -> u64 {
        (self.size + 4095) / 4096
    }

    /// Check if verified
    pub fn is_verified(&self) -> bool {
        self.flags.contains(ModuleFlags::VERIFIED)
    }

    /// Check if compressed
    pub fn is_compressed(&self) -> bool {
        self.flags.contains(ModuleFlags::COMPRESSED)
    }

    /// Check if required
    pub fn is_required(&self) -> bool {
        self.flags.contains(ModuleFlags::REQUIRED)
    }
}

impl Default for ModuleInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            physical_address: PhysicalAddress(0),
            virtual_address: VirtualAddress(0),
            size: 0,
            module_type: ModuleType::Unknown,
            flags: ModuleFlags::empty(),
            cmdline: String::new(),
            hash: [0; 32],
            load_order: 0,
        }
    }
}

// =============================================================================
// C-COMPATIBLE MODULE INFO
// =============================================================================

/// C-compatible module info for serialization
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct ModuleInfoRaw {
    /// Physical address
    pub physical_address: u64,
    /// Virtual address
    pub virtual_address: u64,
    /// Size in bytes
    pub size: u64,
    /// Module type
    pub module_type: u32,
    /// Flags
    pub flags: u32,
    /// Name offset in string table
    pub name_offset: u32,
    /// Name length
    pub name_length: u32,
    /// Command line offset in string table
    pub cmdline_offset: u32,
    /// Command line length
    pub cmdline_length: u32,
    /// Load order
    pub load_order: u32,
    /// Reserved
    pub reserved: u32,
    /// Hash (SHA-256)
    pub hash: [u8; 32],
}

impl ModuleInfoRaw {
    /// Structure size
    pub const SIZE: usize = core::mem::size_of::<Self>();
}

// =============================================================================
// MODULE LIST
// =============================================================================

/// List of loaded modules
#[derive(Debug, Clone, Default)]
pub struct ModuleList {
    /// Modules
    modules: Vec<ModuleInfo>,
    /// Next load order
    next_order: u32,
}

impl ModuleList {
    /// Create new module list
    pub fn new() -> Self {
        Self {
            modules: Vec::new(),
            next_order: 0,
        }
    }

    /// Add module
    pub fn add(&mut self, mut module: ModuleInfo) {
        module.load_order = self.next_order;
        self.next_order += 1;
        self.modules.push(module);
    }

    /// Add module with automatic type detection
    pub fn add_auto(
        &mut self,
        name: String,
        addr: PhysicalAddress,
        size: u64,
        data: &[u8],
    ) -> &ModuleInfo {
        let mut module = ModuleInfo::new(name.clone(), addr, size);

        // Detect type from filename first, then magic
        module.module_type = ModuleType::from_filename(&name);
        if module.module_type == ModuleType::Unknown && !data.is_empty() {
            module.module_type = ModuleType::from_magic(data);
        }

        module.load_order = self.next_order;
        self.next_order += 1;
        self.modules.push(module);

        self.modules.last().unwrap()
    }

    /// Get module count
    pub fn len(&self) -> usize {
        self.modules.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    /// Get module by index
    pub fn get(&self, index: usize) -> Option<&ModuleInfo> {
        self.modules.get(index)
    }

    /// Get module by index (mutable)
    pub fn get_mut(&mut self, index: usize) -> Option<&mut ModuleInfo> {
        self.modules.get_mut(index)
    }

    /// Find module by name
    pub fn find(&self, name: &str) -> Option<&ModuleInfo> {
        self.modules.iter().find(|m| m.name == name)
    }

    /// Find module by name (mutable)
    pub fn find_mut(&mut self, name: &str) -> Option<&mut ModuleInfo> {
        self.modules.iter_mut().find(|m| m.name == name)
    }

    /// Find modules by type
    pub fn find_by_type(&self, module_type: ModuleType) -> impl Iterator<Item = &ModuleInfo> {
        self.modules
            .iter()
            .filter(move |m| m.module_type == module_type)
    }

    /// Find initrd
    pub fn find_initrd(&self) -> Option<&ModuleInfo> {
        self.find_by_type(ModuleType::Initrd).next()
    }

    /// Find device tree
    pub fn find_device_tree(&self) -> Option<&ModuleInfo> {
        self.find_by_type(ModuleType::DeviceTree).next()
    }

    /// Iterate all modules
    pub fn iter(&self) -> impl Iterator<Item = &ModuleInfo> {
        self.modules.iter()
    }

    /// Iterate all modules (mutable)
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut ModuleInfo> {
        self.modules.iter_mut()
    }

    /// Get total size of all modules
    pub fn total_size(&self) -> u64 {
        self.modules.iter().map(|m| m.size).sum()
    }

    /// Get required modules
    pub fn required_modules(&self) -> impl Iterator<Item = &ModuleInfo> {
        self.modules.iter().filter(|m| m.is_required())
    }

    /// Check if all required modules are loaded
    pub fn all_required_loaded(&self) -> bool {
        self.required_modules().all(|m| m.physical_address.0 != 0)
    }

    /// Sort by load order
    pub fn sort_by_load_order(&mut self) {
        self.modules.sort_by_key(|m| m.load_order);
    }

    /// Sort by type (initrd first, then kernel modules, etc.)
    pub fn sort_by_priority(&mut self) {
        self.modules.sort_by_key(|m| {
            match m.module_type {
                ModuleType::Microcode => 0, // Earliest
                ModuleType::DeviceTree => 1,
                ModuleType::AcpiTable => 2,
                ModuleType::Initrd => 3,
                ModuleType::KernelModule => 4,
                ModuleType::Config => 5,
                _ => 100,
            }
        });
    }
}

// =============================================================================
// MODULE LOADER
// =============================================================================

/// Module loader configuration
#[derive(Debug, Clone)]
pub struct ModuleLoaderConfig {
    /// Base address for loading modules
    pub base_address: PhysicalAddress,
    /// Alignment for modules
    pub alignment: u64,
    /// Verify module hashes
    pub verify_hashes: bool,
    /// Decompress compressed modules
    pub decompress: bool,
    /// Maximum module size
    pub max_module_size: u64,
    /// Maximum total modules size
    pub max_total_size: u64,
}

impl Default for ModuleLoaderConfig {
    fn default() -> Self {
        Self {
            base_address: PhysicalAddress(0x1000_0000), // 256 MB
            alignment: 4096,
            verify_hashes: true,
            decompress: true,
            max_module_size: 1024 * 1024 * 1024,    // 1 GB
            max_total_size: 4 * 1024 * 1024 * 1024, // 4 GB
        }
    }
}

/// Module loader
pub struct ModuleLoader {
    /// Configuration
    config: ModuleLoaderConfig,
    /// Loaded modules
    modules: ModuleList,
    /// Current load address
    current_address: PhysicalAddress,
    /// Total loaded size
    total_loaded: u64,
}

impl ModuleLoader {
    /// Create new module loader
    pub fn new(config: ModuleLoaderConfig) -> Self {
        let current_address = config.base_address;
        Self {
            config,
            modules: ModuleList::new(),
            current_address,
            total_loaded: 0,
        }
    }

    /// Load module from buffer
    pub fn load(&mut self, name: String, data: &[u8]) -> Result<&ModuleInfo> {
        let size = data.len() as u64;

        // Check size limits
        if size > self.config.max_module_size {
            return Err(Error::OutOfResources);
        }

        if self.total_loaded + size > self.config.max_total_size {
            return Err(Error::OutOfResources);
        }

        // Align address
        let aligned_addr = PhysicalAddress(
            (self.current_address.0 + self.config.alignment - 1) & !(self.config.alignment - 1),
        );

        // Create module info
        let module = self.modules.add_auto(name, aligned_addr, size, data);

        // Update state
        self.current_address = PhysicalAddress(aligned_addr.0 + size);
        self.total_loaded += size;

        Ok(module)
    }

    /// Get loaded modules
    pub fn modules(&self) -> &ModuleList {
        &self.modules
    }

    /// Get loaded modules (mutable)
    pub fn modules_mut(&mut self) -> &mut ModuleList {
        &mut self.modules
    }

    /// Get total loaded size
    pub fn total_loaded(&self) -> u64 {
        self.total_loaded
    }

    /// Get remaining capacity
    pub fn remaining_capacity(&self) -> u64 {
        self.config.max_total_size.saturating_sub(self.total_loaded)
    }
}

// =============================================================================
// MODULE BUILDER
// =============================================================================

/// Module builder for fluent API
pub struct ModuleBuilder {
    module: ModuleInfo,
}

impl ModuleBuilder {
    /// Create new builder
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            module: ModuleInfo {
                name: name.into(),
                ..Default::default()
            },
        }
    }

    /// Set physical address
    pub fn physical_address(mut self, addr: PhysicalAddress) -> Self {
        self.module.physical_address = addr;
        self
    }

    /// Set virtual address
    pub fn virtual_address(mut self, addr: VirtualAddress) -> Self {
        self.module.virtual_address = addr;
        self
    }

    /// Set size
    pub fn size(mut self, size: u64) -> Self {
        self.module.size = size;
        self
    }

    /// Set module type
    pub fn module_type(mut self, module_type: ModuleType) -> Self {
        self.module.module_type = module_type;
        self
    }

    /// Set flags
    pub fn flags(mut self, flags: ModuleFlags) -> Self {
        self.module.flags = flags;
        self
    }

    /// Add flag
    pub fn add_flag(mut self, flag: ModuleFlags) -> Self {
        self.module.flags.insert(flag);
        self
    }

    /// Set command line
    pub fn cmdline(mut self, cmdline: impl Into<String>) -> Self {
        self.module.cmdline = cmdline.into();
        self
    }

    /// Set hash
    pub fn hash(mut self, hash: [u8; 32]) -> Self {
        self.module.hash = hash;
        self
    }

    /// Mark as required
    pub fn required(self) -> Self {
        self.add_flag(ModuleFlags::REQUIRED)
    }

    /// Mark as compressed
    pub fn compressed(self) -> Self {
        self.add_flag(ModuleFlags::COMPRESSED)
    }

    /// Mark as verified
    pub fn verified(self) -> Self {
        self.add_flag(ModuleFlags::VERIFIED)
    }

    /// Build module
    pub fn build(self) -> ModuleInfo {
        self.module
    }
}

// =============================================================================
// INITRD PARSING
// =============================================================================

/// CPIO archive format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpioFormat {
    /// Old binary format
    OldBinary,
    /// Old portable ASCII
    OldAscii,
    /// New ASCII (SVR4)
    NewAscii,
    /// New ASCII with CRC
    NewAsciiCrc,
}

/// CPIO entry header
#[derive(Debug, Clone)]
pub struct CpioEntry {
    /// Filename
    pub name: String,
    /// File size
    pub size: u64,
    /// File mode
    pub mode: u32,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Number of links
    pub nlink: u32,
    /// Modification time
    pub mtime: u64,
    /// Device ID
    pub devmajor: u32,
    /// Device minor
    pub devminor: u32,
    /// Inode number
    pub ino: u64,
    /// Data offset in archive
    pub data_offset: u64,
}

impl CpioEntry {
    /// Check if directory
    pub fn is_dir(&self) -> bool {
        (self.mode & 0o170_000) == 0o040_000
    }

    /// Check if regular file
    pub fn is_file(&self) -> bool {
        (self.mode & 0o170_000) == 0o100_000
    }

    /// Check if symlink
    pub fn is_symlink(&self) -> bool {
        (self.mode & 0o170_000) == 0o120_000
    }

    /// Get permissions
    pub fn permissions(&self) -> u32 {
        self.mode & 0o777
    }
}

/// CPIO archive reader
pub struct CpioReader<'a> {
    data: &'a [u8],
    offset: usize,
    format: CpioFormat,
}

impl<'a> CpioReader<'a> {
    /// Create new reader
    pub fn new(data: &'a [u8]) -> Result<Self> {
        let format = Self::detect_format(data)?;
        Ok(Self {
            data,
            offset: 0,
            format,
        })
    }

    /// Detect CPIO format
    fn detect_format(data: &[u8]) -> Result<CpioFormat> {
        if data.len() < 6 {
            return Err(Error::InvalidParameter);
        }

        // Check magic
        if &data[0..6] == b"070701" {
            Ok(CpioFormat::NewAscii)
        } else if &data[0..6] == b"070702" {
            Ok(CpioFormat::NewAsciiCrc)
        } else if &data[0..6] == b"070707" {
            Ok(CpioFormat::OldAscii)
        } else if data[0..2] == [0xC7, 0x71] {
            Ok(CpioFormat::OldBinary)
        } else {
            Err(Error::InvalidParameter)
        }
    }

    /// Get format
    pub fn format(&self) -> CpioFormat {
        self.format
    }

    /// Read next entry (new ASCII format)
    pub fn next_entry(&mut self) -> Result<Option<CpioEntry>> {
        if self.offset >= self.data.len() {
            return Ok(None);
        }

        match self.format {
            CpioFormat::NewAscii | CpioFormat::NewAsciiCrc => self.read_new_ascii_entry(),
            _ => Err(Error::Unsupported),
        }
    }

    /// Read new ASCII format entry
    fn read_new_ascii_entry(&mut self) -> Result<Option<CpioEntry>> {
        const HEADER_SIZE: usize = 110;

        if self.offset + HEADER_SIZE > self.data.len() {
            return Ok(None);
        }

        let header = &self.data[self.offset..self.offset + HEADER_SIZE];

        // Verify magic
        if &header[0..6] != b"070701" && &header[0..6] != b"070702" {
            return Err(Error::InvalidParameter);
        }

        // Parse hex fields
        let parse_hex = |s: &[u8]| -> u64 {
            let s = core::str::from_utf8(s).unwrap_or("0");
            u64::from_str_radix(s, 16).unwrap_or(0)
        };

        let ino = parse_hex(&header[6..14]);
        let mode = parse_hex(&header[14..22]) as u32;
        let uid = parse_hex(&header[22..30]) as u32;
        let gid = parse_hex(&header[30..38]) as u32;
        let nlink = parse_hex(&header[38..46]) as u32;
        let mtime = parse_hex(&header[46..54]);
        let filesize = parse_hex(&header[54..62]);
        let devmajor = parse_hex(&header[62..70]) as u32;
        let devminor = parse_hex(&header[70..78]) as u32;
        let _rdevmajor = parse_hex(&header[78..86]);
        let _rdevminor = parse_hex(&header[86..94]);
        let namesize = parse_hex(&header[94..102]) as usize;
        let _checksum = parse_hex(&header[102..110]);

        // Read filename
        let name_start = self.offset + HEADER_SIZE;
        let name_end = name_start + namesize - 1; // Exclude null terminator

        if name_end > self.data.len() {
            return Err(Error::InvalidParameter);
        }

        let name_str = core::str::from_utf8(&self.data[name_start..name_end]).unwrap_or("");
        let name = alloc::string::String::from(name_str);

        // Check for trailer
        if name == "TRAILER!!!" {
            return Ok(None);
        }

        // Calculate data offset (4-byte aligned after header + name)
        let header_end = name_start + namesize;
        let data_offset = (header_end + 3) & !3;

        let entry = CpioEntry {
            name,
            size: filesize,
            mode,
            uid,
            gid,
            nlink,
            mtime,
            devmajor,
            devminor,
            ino,
            data_offset: data_offset as u64,
        };

        // Advance offset (4-byte aligned after data)
        self.offset = ((data_offset + filesize as usize) + 3) & !3;

        Ok(Some(entry))
    }

    /// Get entry data
    pub fn entry_data(&self, entry: &CpioEntry) -> &'a [u8] {
        let start = entry.data_offset as usize;
        let end = start + entry.size as usize;

        if end <= self.data.len() {
            &self.data[start..end]
        } else {
            &[]
        }
    }

    /// Reset reader
    pub fn reset(&mut self) {
        self.offset = 0;
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_module_type_detection() {
        assert_eq!(ModuleType::from_filename("initrd.img"), ModuleType::Initrd);
        assert_eq!(
            ModuleType::from_filename("test.ko"),
            ModuleType::KernelModule
        );
        assert_eq!(
            ModuleType::from_filename("board.dtb"),
            ModuleType::DeviceTree
        );
        assert_eq!(ModuleType::from_filename("random.bin"), ModuleType::Unknown);
    }

    #[test]
    fn test_module_magic() {
        assert_eq!(
            ModuleType::from_magic(&[0x7F, b'E', b'L', b'F']),
            ModuleType::Elf
        );
        assert_eq!(
            ModuleType::from_magic(&[0x1F, 0x8B, 0x08, 0x00]),
            ModuleType::Initrd
        );
    }

    #[test]
    fn test_module_list() {
        let mut list = ModuleList::new();

        list.add(ModuleInfo::new(String::from("test"), 0x1000, 0x100));
        assert_eq!(list.len(), 1);

        let found = list.find("test");
        assert!(found.is_some());
        assert_eq!(found.unwrap().physical_address, 0x1000);
    }

    #[test]
    fn test_module_builder() {
        let module = ModuleBuilder::new("initrd.img")
            .physical_address(0x10000000)
            .size(0x1000000)
            .module_type(ModuleType::Initrd)
            .required()
            .verified()
            .build();

        assert_eq!(module.name, "initrd.img");
        assert!(module.is_required());
        assert!(module.is_verified());
    }
}
