//! PE/COFF Loader
//!
//! Complete PE32+ executable loader for UEFI-style images.
//! Supports Windows portable executable format used by UEFI applications.

use crate::error::{Error, Result};
use crate::loader::{
    ImageFlags, ImageFormat, ImageSection, LoadedImage, MachineType, SectionFlags,
};
use crate::raw::types::VirtualAddress;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// PE CONSTANTS
// =============================================================================

/// DOS MZ magic
pub const DOS_MAGIC: [u8; 2] = [b'M', b'Z'];

/// PE signature
pub const PE_SIGNATURE: [u8; 4] = [b'P', b'E', 0, 0];

/// PE32 magic
pub const PE32_MAGIC: u16 = 0x10B;

/// PE32+ magic
pub const PE32PLUS_MAGIC: u16 = 0x20B;

/// Machine types
pub mod machine {
    pub const IMAGE_FILE_MACHINE_UNKNOWN: u16 = 0x0;
    pub const IMAGE_FILE_MACHINE_I386: u16 = 0x014C;
    pub const IMAGE_FILE_MACHINE_AMD64: u16 = 0x8664;
    pub const IMAGE_FILE_MACHINE_ARM: u16 = 0x01C0;
    pub const IMAGE_FILE_MACHINE_ARM64: u16 = 0xAA64;
    pub const IMAGE_FILE_MACHINE_RISCV64: u16 = 0x5064;
}

/// Characteristics
pub mod characteristics {
    pub const IMAGE_FILE_RELOCS_STRIPPED: u16 = 0x0001;
    pub const IMAGE_FILE_EXECUTABLE_IMAGE: u16 = 0x0002;
    pub const IMAGE_FILE_LINE_NUMS_STRIPPED: u16 = 0x0004;
    pub const IMAGE_FILE_LOCAL_SYMS_STRIPPED: u16 = 0x0008;
    pub const IMAGE_FILE_LARGE_ADDRESS_AWARE: u16 = 0x0020;
    pub const IMAGE_FILE_32BIT_MACHINE: u16 = 0x0100;
    pub const IMAGE_FILE_DEBUG_STRIPPED: u16 = 0x0200;
    pub const IMAGE_FILE_REMOVABLE_RUN_FROM_SWAP: u16 = 0x0400;
    pub const IMAGE_FILE_NET_RUN_FROM_SWAP: u16 = 0x0800;
    pub const IMAGE_FILE_SYSTEM: u16 = 0x1000;
    pub const IMAGE_FILE_DLL: u16 = 0x2000;
    pub const IMAGE_FILE_UP_SYSTEM_ONLY: u16 = 0x4000;
}

/// DLL characteristics
pub mod dll_characteristics {
    pub const IMAGE_DLLCHARACTERISTICS_HIGH_ENTROPY_VA: u16 = 0x0020;
    pub const IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE: u16 = 0x0040;
    pub const IMAGE_DLLCHARACTERISTICS_FORCE_INTEGRITY: u16 = 0x0080;
    pub const IMAGE_DLLCHARACTERISTICS_NX_COMPAT: u16 = 0x0100;
    pub const IMAGE_DLLCHARACTERISTICS_NO_ISOLATION: u16 = 0x0200;
    pub const IMAGE_DLLCHARACTERISTICS_NO_SEH: u16 = 0x0400;
    pub const IMAGE_DLLCHARACTERISTICS_NO_BIND: u16 = 0x0800;
    pub const IMAGE_DLLCHARACTERISTICS_APPCONTAINER: u16 = 0x1000;
    pub const IMAGE_DLLCHARACTERISTICS_WDM_DRIVER: u16 = 0x2000;
    pub const IMAGE_DLLCHARACTERISTICS_GUARD_CF: u16 = 0x4000;
    pub const IMAGE_DLLCHARACTERISTICS_TERMINAL_SERVER_AWARE: u16 = 0x8000;
}

/// Section characteristics
pub mod section_characteristics {
    /// Section should not be padded to next boundary
    pub const IMAGE_SCN_TYPE_NO_PAD: u32 = 0x0000_0008;
    /// Section contains executable code
    pub const IMAGE_SCN_CNT_CODE: u32 = 0x0000_0020;
    /// Section contains initialized data
    pub const IMAGE_SCN_CNT_INITIALIZED_DATA: u32 = 0x0000_0040;
    /// Section contains uninitialized data
    pub const IMAGE_SCN_CNT_UNINITIALIZED_DATA: u32 = 0x0000_0080;
    /// Section contains comments or info
    pub const IMAGE_SCN_LNK_INFO: u32 = 0x0000_0200;
    /// Section will not become part of the image
    pub const IMAGE_SCN_LNK_REMOVE: u32 = 0x0000_0800;
    /// Section contains COMDAT data
    pub const IMAGE_SCN_LNK_COMDAT: u32 = 0x0000_1000;
    /// Section contains data referenced through GP
    pub const IMAGE_SCN_GPREL: u32 = 0x0000_8000;
    /// 1-byte alignment
    pub const IMAGE_SCN_ALIGN_1BYTES: u32 = 0x0010_0000;
    /// 2-byte alignment
    pub const IMAGE_SCN_ALIGN_2BYTES: u32 = 0x0020_0000;
    /// 4-byte alignment
    pub const IMAGE_SCN_ALIGN_4BYTES: u32 = 0x0030_0000;
    /// 8-byte alignment
    pub const IMAGE_SCN_ALIGN_8BYTES: u32 = 0x0040_0000;
    /// 16-byte alignment
    pub const IMAGE_SCN_ALIGN_16BYTES: u32 = 0x0050_0000;
    /// 32-byte alignment
    pub const IMAGE_SCN_ALIGN_32BYTES: u32 = 0x0060_0000;
    /// 64-byte alignment
    pub const IMAGE_SCN_ALIGN_64BYTES: u32 = 0x0070_0000;
    /// 128-byte alignment
    pub const IMAGE_SCN_ALIGN_128BYTES: u32 = 0x0080_0000;
    /// 256-byte alignment
    pub const IMAGE_SCN_ALIGN_256BYTES: u32 = 0x0090_0000;
    /// 512-byte alignment
    pub const IMAGE_SCN_ALIGN_512BYTES: u32 = 0x00A0_0000;
    /// 1024-byte alignment
    pub const IMAGE_SCN_ALIGN_1024BYTES: u32 = 0x00B0_0000;
    /// 2048-byte alignment
    pub const IMAGE_SCN_ALIGN_2048BYTES: u32 = 0x00C0_0000;
    /// 4096-byte alignment
    pub const IMAGE_SCN_ALIGN_4096BYTES: u32 = 0x00D0_0000;
    /// 8192-byte alignment
    pub const IMAGE_SCN_ALIGN_8192BYTES: u32 = 0x00E0_0000;
    /// Section contains extended relocations
    pub const IMAGE_SCN_LNK_NRELOC_OVFL: u32 = 0x0100_0000;
    /// Section can be discarded as needed
    pub const IMAGE_SCN_MEM_DISCARDABLE: u32 = 0x0200_0000;
    /// Section cannot be cached
    pub const IMAGE_SCN_MEM_NOT_CACHED: u32 = 0x0400_0000;
    /// Section cannot be paged
    pub const IMAGE_SCN_MEM_NOT_PAGED: u32 = 0x0800_0000;
    /// Section can be shared in memory
    pub const IMAGE_SCN_MEM_SHARED: u32 = 0x1000_0000;
    /// Section is executable
    pub const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
    /// Section is readable
    pub const IMAGE_SCN_MEM_READ: u32 = 0x4000_0000;
    /// Section is writable
    pub const IMAGE_SCN_MEM_WRITE: u32 = 0x8000_0000;
}

/// Data directory indices
pub mod directory {
    pub const IMAGE_DIRECTORY_ENTRY_EXPORT: usize = 0;
    pub const IMAGE_DIRECTORY_ENTRY_IMPORT: usize = 1;
    pub const IMAGE_DIRECTORY_ENTRY_RESOURCE: usize = 2;
    pub const IMAGE_DIRECTORY_ENTRY_EXCEPTION: usize = 3;
    pub const IMAGE_DIRECTORY_ENTRY_SECURITY: usize = 4;
    pub const IMAGE_DIRECTORY_ENTRY_BASERELOC: usize = 5;
    pub const IMAGE_DIRECTORY_ENTRY_DEBUG: usize = 6;
    pub const IMAGE_DIRECTORY_ENTRY_ARCHITECTURE: usize = 7;
    pub const IMAGE_DIRECTORY_ENTRY_GLOBALPTR: usize = 8;
    pub const IMAGE_DIRECTORY_ENTRY_TLS: usize = 9;
    pub const IMAGE_DIRECTORY_ENTRY_LOAD_CONFIG: usize = 10;
    pub const IMAGE_DIRECTORY_ENTRY_BOUND_IMPORT: usize = 11;
    pub const IMAGE_DIRECTORY_ENTRY_IAT: usize = 12;
    pub const IMAGE_DIRECTORY_ENTRY_DELAY_IMPORT: usize = 13;
    pub const IMAGE_DIRECTORY_ENTRY_COM_DESCRIPTOR: usize = 14;
}

/// Base relocation types
pub mod reloc_type {
    pub const IMAGE_REL_BASED_ABSOLUTE: u16 = 0;
    pub const IMAGE_REL_BASED_HIGH: u16 = 1;
    pub const IMAGE_REL_BASED_LOW: u16 = 2;
    pub const IMAGE_REL_BASED_HIGHLOW: u16 = 3;
    pub const IMAGE_REL_BASED_HIGHADJ: u16 = 4;
    pub const IMAGE_REL_BASED_MIPS_JMPADDR: u16 = 5;
    pub const IMAGE_REL_BASED_ARM_MOV32: u16 = 5;
    pub const IMAGE_REL_BASED_THUMB_MOV32: u16 = 7;
    pub const IMAGE_REL_BASED_MIPS_JMPADDR16: u16 = 9;
    pub const IMAGE_REL_BASED_DIR64: u16 = 10;
}

/// Subsystem types
pub mod subsystem {
    pub const IMAGE_SUBSYSTEM_UNKNOWN: u16 = 0;
    pub const IMAGE_SUBSYSTEM_NATIVE: u16 = 1;
    pub const IMAGE_SUBSYSTEM_WINDOWS_GUI: u16 = 2;
    pub const IMAGE_SUBSYSTEM_WINDOWS_CUI: u16 = 3;
    pub const IMAGE_SUBSYSTEM_OS2_CUI: u16 = 5;
    pub const IMAGE_SUBSYSTEM_POSIX_CUI: u16 = 7;
    pub const IMAGE_SUBSYSTEM_NATIVE_WINDOWS: u16 = 8;
    pub const IMAGE_SUBSYSTEM_WINDOWS_CE_GUI: u16 = 9;
    pub const IMAGE_SUBSYSTEM_EFI_APPLICATION: u16 = 10;
    pub const IMAGE_SUBSYSTEM_EFI_BOOT_SERVICE_DRIVER: u16 = 11;
    pub const IMAGE_SUBSYSTEM_EFI_RUNTIME_DRIVER: u16 = 12;
    pub const IMAGE_SUBSYSTEM_EFI_ROM: u16 = 13;
    pub const IMAGE_SUBSYSTEM_XBOX: u16 = 14;
    pub const IMAGE_SUBSYSTEM_WINDOWS_BOOT_APPLICATION: u16 = 16;
}

// =============================================================================
// PE STRUCTURES
// =============================================================================

/// DOS header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct DosHeader {
    /// Magic number (MZ)
    pub e_magic: [u8; 2],
    /// Bytes on last page of file
    pub e_cblp: u16,
    /// Pages in file
    pub e_cp: u16,
    /// Relocations
    pub e_crlc: u16,
    /// Size of header in paragraphs
    pub e_cparhdr: u16,
    /// Minimum extra paragraphs needed
    pub e_minalloc: u16,
    /// Maximum extra paragraphs needed
    pub e_maxalloc: u16,
    /// Initial SS value
    pub e_ss: u16,
    /// Initial SP value
    pub e_sp: u16,
    /// Checksum
    pub e_csum: u16,
    /// Initial IP value
    pub e_ip: u16,
    /// Initial CS value
    pub e_cs: u16,
    /// File address of relocation table
    pub e_lfarlc: u16,
    /// Overlay number
    pub e_ovno: u16,
    /// Reserved words
    pub e_res: [u16; 4],
    /// OEM identifier
    pub e_oemid: u16,
    /// OEM information
    pub e_oeminfo: u16,
    /// Reserved words
    pub e_res2: [u16; 10],
    /// File address of new exe header
    pub e_lfanew: i32,
}

impl DosHeader {
    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < core::mem::size_of::<Self>() {
            return Err(Error::InvalidData);
        }

        let header: Self = unsafe { *(data.as_ptr().cast::<Self>()) };

        if header.e_magic != DOS_MAGIC {
            return Err(Error::InvalidMagic);
        }

        Ok(header)
    }

    /// Get PE header offset
    #[must_use]
    pub fn pe_offset(&self) -> usize {
        usize::try_from(self.e_lfanew).unwrap_or(0)
    }
}

/// COFF file header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct CoffHeader {
    /// Machine type
    pub machine: u16,
    /// Number of sections
    pub number_of_sections: u16,
    /// Time date stamp
    pub time_date_stamp: u32,
    /// Pointer to symbol table
    pub pointer_to_symbol_table: u32,
    /// Number of symbols
    pub number_of_symbols: u32,
    /// Size of optional header
    pub size_of_optional_header: u16,
    /// Characteristics
    pub characteristics: u16,
}

impl CoffHeader {
    /// Parse from bytes
    pub fn parse(data: &[u8], offset: usize) -> Result<Self> {
        // Check PE signature first
        if data.len() < offset + 4 {
            return Err(Error::InvalidData);
        }

        if data[offset..offset + 4] != PE_SIGNATURE {
            return Err(Error::InvalidMagic);
        }

        let header_offset = offset + 4;
        if data.len() < header_offset + core::mem::size_of::<Self>() {
            return Err(Error::InvalidData);
        }

        Ok(unsafe { *(data[header_offset..].as_ptr().cast::<Self>()) })
    }

    /// Get machine type
    #[must_use]
    pub const fn machine_type(&self) -> MachineType {
        match self.machine {
            machine::IMAGE_FILE_MACHINE_I386 => MachineType::X86,
            machine::IMAGE_FILE_MACHINE_AMD64 => MachineType::X86_64,
            machine::IMAGE_FILE_MACHINE_ARM => MachineType::Arm,
            machine::IMAGE_FILE_MACHINE_ARM64 => MachineType::Aarch64,
            machine::IMAGE_FILE_MACHINE_RISCV64 => MachineType::RiscV64,
            _ => MachineType::Unknown,
        }
    }

    /// Check if executable
    #[must_use]
    pub const fn is_executable(&self) -> bool {
        (self.characteristics & characteristics::IMAGE_FILE_EXECUTABLE_IMAGE) != 0
    }

    /// Check if DLL
    #[must_use]
    pub const fn is_dll(&self) -> bool {
        (self.characteristics & characteristics::IMAGE_FILE_DLL) != 0
    }

    /// Check if relocations stripped
    #[must_use]
    pub const fn relocs_stripped(&self) -> bool {
        (self.characteristics & characteristics::IMAGE_FILE_RELOCS_STRIPPED) != 0
    }
}

/// Data directory entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct DataDirectory {
    /// Virtual address
    pub virtual_address: u32,
    /// Size
    pub size: u32,
}

/// Optional header (PE32+)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct OptionalHeader64 {
    /// Magic number
    pub magic: u16,
    /// Major linker version
    pub major_linker_version: u8,
    /// Minor linker version
    pub minor_linker_version: u8,
    /// Size of code
    pub size_of_code: u32,
    /// Size of initialized data
    pub size_of_initialized_data: u32,
    /// Size of uninitialized data
    pub size_of_uninitialized_data: u32,
    /// Address of entry point
    pub address_of_entry_point: u32,
    /// Base of code
    pub base_of_code: u32,
    /// Image base
    pub image_base: u64,
    /// Section alignment
    pub section_alignment: u32,
    /// File alignment
    pub file_alignment: u32,
    /// Major operating system version
    pub major_operating_system_version: u16,
    /// Minor operating system version
    pub minor_operating_system_version: u16,
    /// Major image version
    pub major_image_version: u16,
    /// Minor image version
    pub minor_image_version: u16,
    /// Major subsystem version
    pub major_subsystem_version: u16,
    /// Minor subsystem version
    pub minor_subsystem_version: u16,
    /// Win32 version value
    pub win32_version_value: u32,
    /// Size of image
    pub size_of_image: u32,
    /// Size of headers
    pub size_of_headers: u32,
    /// Checksum
    pub check_sum: u32,
    /// Subsystem
    pub subsystem: u16,
    /// DLL characteristics
    pub dll_characteristics: u16,
    /// Size of stack reserve
    pub size_of_stack_reserve: u64,
    /// Size of stack commit
    pub size_of_stack_commit: u64,
    /// Size of heap reserve
    pub size_of_heap_reserve: u64,
    /// Size of heap commit
    pub size_of_heap_commit: u64,
    /// Loader flags
    pub loader_flags: u32,
    /// Number of RVA and sizes
    pub number_of_rva_and_sizes: u32,
    /// Data directories
    pub data_directories: [DataDirectory; 16],
}

impl OptionalHeader64 {
    /// Parse from bytes
    pub fn parse(data: &[u8], offset: usize) -> Result<Self> {
        if data.len() < offset + core::mem::size_of::<Self>() {
            return Err(Error::InvalidData);
        }

        let header: Self = unsafe { *(data[offset..].as_ptr().cast::<Self>()) };

        if header.magic != PE32PLUS_MAGIC {
            return Err(Error::UnsupportedFormat);
        }

        Ok(header)
    }

    /// Check if UEFI application
    #[must_use]
    pub const fn is_uefi(&self) -> bool {
        matches!(
            self.subsystem,
            subsystem::IMAGE_SUBSYSTEM_EFI_APPLICATION
                | subsystem::IMAGE_SUBSYSTEM_EFI_BOOT_SERVICE_DRIVER
                | subsystem::IMAGE_SUBSYSTEM_EFI_RUNTIME_DRIVER
                | subsystem::IMAGE_SUBSYSTEM_EFI_ROM
        )
    }

    /// Check if NX compatible
    #[must_use]
    pub const fn is_nx_compat(&self) -> bool {
        (self.dll_characteristics & dll_characteristics::IMAGE_DLLCHARACTERISTICS_NX_COMPAT) != 0
    }

    /// Check if dynamic base (ASLR)
    #[must_use]
    pub const fn is_dynamic_base(&self) -> bool {
        (self.dll_characteristics & dll_characteristics::IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE) != 0
    }

    /// Get data directory
    #[must_use]
    pub const fn data_directory(&self, index: usize) -> Option<&DataDirectory> {
        if index < self.number_of_rva_and_sizes as usize && index < 16 {
            Some(&self.data_directories[index])
        } else {
            None
        }
    }
}

/// Section header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct PeSectionHeader {
    /// Name
    pub name: [u8; 8],
    /// Virtual size
    pub virtual_size: u32,
    /// Virtual address
    pub virtual_address: u32,
    /// Size of raw data
    pub size_of_raw_data: u32,
    /// Pointer to raw data
    pub pointer_to_raw_data: u32,
    /// Pointer to relocations
    pub pointer_to_relocations: u32,
    /// Pointer to line numbers
    pub pointer_to_linenumbers: u32,
    /// Number of relocations
    pub number_of_relocations: u16,
    /// Number of line numbers
    pub number_of_linenumbers: u16,
    /// Characteristics
    pub characteristics: u32,
}

impl PeSectionHeader {
    /// Get section name as string
    #[must_use]
    pub fn name_str(&self) -> String {
        let end = self.name.iter().position(|&b| b == 0).unwrap_or(8);
        String::from_utf8_lossy(&self.name[..end]).into_owned()
    }

    /// Check if contains code
    #[must_use]
    pub const fn is_code(&self) -> bool {
        (self.characteristics & section_characteristics::IMAGE_SCN_CNT_CODE) != 0
    }

    /// Check if initialized data
    #[must_use]
    pub const fn is_initialized_data(&self) -> bool {
        (self.characteristics & section_characteristics::IMAGE_SCN_CNT_INITIALIZED_DATA) != 0
    }

    /// Check if uninitialized data
    #[must_use]
    pub const fn is_uninitialized_data(&self) -> bool {
        (self.characteristics & section_characteristics::IMAGE_SCN_CNT_UNINITIALIZED_DATA) != 0
    }

    /// Check if readable
    #[must_use]
    pub const fn is_readable(&self) -> bool {
        (self.characteristics & section_characteristics::IMAGE_SCN_MEM_READ) != 0
    }

    /// Check if writable
    #[must_use]
    pub const fn is_writable(&self) -> bool {
        (self.characteristics & section_characteristics::IMAGE_SCN_MEM_WRITE) != 0
    }

    /// Check if executable
    #[must_use]
    pub const fn is_executable(&self) -> bool {
        (self.characteristics & section_characteristics::IMAGE_SCN_MEM_EXECUTE) != 0
    }

    /// Check if discardable
    #[must_use]
    pub const fn is_discardable(&self) -> bool {
        (self.characteristics & section_characteristics::IMAGE_SCN_MEM_DISCARDABLE) != 0
    }

    /// Get alignment
    #[must_use]
    pub const fn alignment(&self) -> u32 {
        let align_mask = self.characteristics & 0x00F0_0000;
        if align_mask == 0 {
            return 16; // Default alignment
        }

        let shift = (align_mask >> 20) - 1;
        1 << shift
    }

    /// Convert to section flags
    #[must_use]
    pub const fn to_section_flags(&self) -> SectionFlags {
        SectionFlags {
            readable: self.is_readable(),
            writable: self.is_writable(),
            executable: self.is_executable(),
            allocated: true,
            code: self.is_code(),
            data: self.is_initialized_data(),
            bss: self.is_uninitialized_data(),
        }
    }
}

/// Base relocation block
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct BaseRelocationBlock {
    /// Page RVA
    pub virtual_address: u32,
    /// Block size
    pub size_of_block: u32,
}

/// Export directory
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ExportDirectory {
    /// Characteristics
    pub characteristics: u32,
    /// Time date stamp
    pub time_date_stamp: u32,
    /// Major version
    pub major_version: u16,
    /// Minor version
    pub minor_version: u16,
    /// Name RVA
    pub name: u32,
    /// Base ordinal
    pub base: u32,
    /// Number of functions
    pub number_of_functions: u32,
    /// Number of names
    pub number_of_names: u32,
    /// Address of functions
    pub address_of_functions: u32,
    /// Address of names
    pub address_of_names: u32,
    /// Address of name ordinals
    pub address_of_name_ordinals: u32,
}

/// Import directory
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ImportDirectory {
    /// Import lookup table RVA
    pub import_lookup_table_rva: u32,
    /// Time date stamp
    pub time_date_stamp: u32,
    /// Forwarder chain
    pub forwarder_chain: u32,
    /// Name RVA
    pub name_rva: u32,
    /// Import address table RVA
    pub import_address_table_rva: u32,
}

// =============================================================================
// PE LOADER
// =============================================================================

/// PE file loader
pub struct PeLoader {
    /// DOS header
    dos_header: Option<DosHeader>,
    /// COFF header
    coff_header: Option<CoffHeader>,
    /// Optional header
    optional_header: Option<OptionalHeader64>,
    /// Section headers
    section_headers: Vec<PeSectionHeader>,
    /// Exports
    exports: Vec<PeExport>,
    /// Imports
    imports: Vec<PeImport>,
    /// Relocations
    relocations: Vec<PeRelocation>,
    /// Loaded image
    image: Option<LoadedImage>,
    /// Raw data
    data: Vec<u8>,
}

impl PeLoader {
    /// Create new PE loader
    pub fn new() -> Self {
        Self {
            dos_header: None,
            coff_header: None,
            optional_header: None,
            section_headers: Vec::new(),
            exports: Vec::new(),
            imports: Vec::new(),
            relocations: Vec::new(),
            image: None,
            data: Vec::new(),
        }
    }

    /// Load PE from buffer
    pub fn load(&mut self, data: &[u8]) -> Result<LoadedImage> {
        self.data = data.to_vec();

        // Parse DOS header
        let dos = DosHeader::parse(data)?;
        self.dos_header = Some(dos);

        // Parse COFF header
        let pe_offset = dos.pe_offset();
        let coff = CoffHeader::parse(data, pe_offset)?;
        self.coff_header = Some(coff);

        // Parse optional header
        let opt_offset = pe_offset + 4 + core::mem::size_of::<CoffHeader>();
        let optional = OptionalHeader64::parse(data, opt_offset)?;
        self.optional_header = Some(optional);

        // Parse section headers
        self.parse_sections(data, &coff, opt_offset)?;

        // Parse exports
        self.parse_exports(data)?;

        // Parse imports
        self.parse_imports(data)?;

        // Parse relocations
        self.parse_relocations(data)?;

        // Build loaded image
        let image = self.build_image()?;
        self.image = Some(image.clone());

        Ok(image)
    }

    /// Parse section headers
    fn parse_sections(&mut self, data: &[u8], coff: &CoffHeader, opt_offset: usize) -> Result<()> {
        self.section_headers.clear();

        let sections_offset = opt_offset + usize::from(coff.size_of_optional_header);
        let section_size = core::mem::size_of::<PeSectionHeader>();

        for i in 0..usize::from(coff.number_of_sections) {
            let offset = sections_offset + i * section_size;

            if data.len() < offset + section_size {
                return Err(Error::InvalidData);
            }

            let section: PeSectionHeader =
                unsafe { *(data[offset..].as_ptr().cast::<PeSectionHeader>()) };

            self.section_headers.push(section);
        }

        Ok(())
    }

    /// Parse exports
    fn parse_exports(&mut self, data: &[u8]) -> Result<()> {
        self.exports.clear();

        let Some(optional) = &self.optional_header else {
            return Ok(());
        };

        let Some(export_dir) = optional.data_directory(directory::IMAGE_DIRECTORY_ENTRY_EXPORT)
        else {
            return Ok(());
        };

        if export_dir.virtual_address == 0 || export_dir.size == 0 {
            return Ok(());
        }

        let export_rva = u64::from(export_dir.virtual_address);
        let export_offset = self.rva_to_offset(export_rva)?;

        if data.len() < export_offset + core::mem::size_of::<ExportDirectory>() {
            return Ok(());
        }

        let export_table: ExportDirectory =
            unsafe { *(data[export_offset..].as_ptr().cast::<ExportDirectory>()) };

        // Parse export names
        let names_offset = self.rva_to_offset(u64::from(export_table.address_of_names))?;
        let ordinals_offset =
            self.rva_to_offset(u64::from(export_table.address_of_name_ordinals))?;
        let functions_offset = self.rva_to_offset(u64::from(export_table.address_of_functions))?;

        for i in 0..export_table.number_of_names as usize {
            // Get name RVA
            let name_rva_offset = names_offset + i * 4;
            if data.len() < name_rva_offset + 4 {
                break;
            }
            let name_rva = u32::from_le_bytes([
                data[name_rva_offset],
                data[name_rva_offset + 1],
                data[name_rva_offset + 2],
                data[name_rva_offset + 3],
            ]);

            // Get name
            let name_offset = self.rva_to_offset(u64::from(name_rva))?;
            let name = self.read_string(data, name_offset);

            // Get ordinal
            let ordinal_offset = ordinals_offset + i * 2;
            if data.len() < ordinal_offset + 2 {
                break;
            }
            let ordinal = u16::from_le_bytes([data[ordinal_offset], data[ordinal_offset + 1]]);

            // Get function address
            let func_offset = functions_offset + (ordinal as usize) * 4;
            if data.len() < func_offset + 4 {
                break;
            }
            let func_rva = u32::from_le_bytes([
                data[func_offset],
                data[func_offset + 1],
                data[func_offset + 2],
                data[func_offset + 3],
            ]);

            self.exports.push(PeExport {
                name,
                ordinal: ordinal + u16::try_from(export_table.base).unwrap_or(0),
                address: u64::from(func_rva),
            });
        }

        Ok(())
    }

    /// Parse imports
    fn parse_imports(&mut self, data: &[u8]) -> Result<()> {
        self.imports.clear();

        let Some(optional) = &self.optional_header else {
            return Ok(());
        };

        let Some(import_dir) = optional.data_directory(directory::IMAGE_DIRECTORY_ENTRY_IMPORT)
        else {
            return Ok(());
        };

        if import_dir.virtual_address == 0 || import_dir.size == 0 {
            return Ok(());
        }

        let mut import_offset = self.rva_to_offset(u64::from(import_dir.virtual_address))?;
        let import_size = core::mem::size_of::<ImportDirectory>();

        loop {
            if data.len() < import_offset + import_size {
                break;
            }

            let import: ImportDirectory =
                unsafe { *(data[import_offset..].as_ptr().cast::<ImportDirectory>()) };

            // Check for terminator
            if import.import_lookup_table_rva == 0 && import.name_rva == 0 {
                break;
            }

            // Get DLL name
            let name_offset = self.rva_to_offset(u64::from(import.name_rva))?;
            let dll_name = self.read_string(data, name_offset);

            self.imports.push(PeImport {
                dll_name,
                functions: Vec::new(), // Would parse IAT here
            });

            import_offset += import_size;
        }

        Ok(())
    }

    /// Parse base relocations
    fn parse_relocations(&mut self, data: &[u8]) -> Result<()> {
        self.relocations.clear();

        let Some(optional) = &self.optional_header else {
            return Ok(());
        };

        let Some(reloc_dir) = optional.data_directory(directory::IMAGE_DIRECTORY_ENTRY_BASERELOC)
        else {
            return Ok(());
        };

        if reloc_dir.virtual_address == 0 || reloc_dir.size == 0 {
            return Ok(());
        }

        let mut offset = self.rva_to_offset(u64::from(reloc_dir.virtual_address))?;
        let end = offset + reloc_dir.size as usize;

        while offset < end && offset + 8 <= data.len() {
            let block: BaseRelocationBlock =
                unsafe { *(data[offset..].as_ptr().cast::<BaseRelocationBlock>()) };

            if block.size_of_block == 0 {
                break;
            }

            let page_rva = block.virtual_address as u64;
            let num_entries = (block.size_of_block as usize - 8) / 2;

            for i in 0..num_entries {
                let entry_offset = offset + 8 + i * 2;
                if entry_offset + 2 > data.len() {
                    break;
                }

                let entry = u16::from_le_bytes([data[entry_offset], data[entry_offset + 1]]);
                let reloc_type = entry >> 12;
                let reloc_offset = entry & 0xFFF;

                if reloc_type != reloc_type::IMAGE_REL_BASED_ABSOLUTE {
                    self.relocations.push(PeRelocation {
                        address: page_rva + u64::from(reloc_offset),
                        reloc_type,
                    });
                }
            }

            offset += block.size_of_block as usize;
        }

        Ok(())
    }

    /// Convert RVA to file offset
    fn rva_to_offset(&self, rva: u64) -> Result<usize> {
        for section in &self.section_headers {
            let section_start = u64::from(section.virtual_address);
            let section_end = section_start + u64::from(section.virtual_size);

            if rva >= section_start && rva < section_end {
                let offset = rva - section_start;
                return Ok(
                    usize::try_from(u64::from(section.pointer_to_raw_data) + offset).unwrap_or(0),
                );
            }
        }

        // If not in any section, assume it's in headers
        if rva
            < self
                .optional_header
                .as_ref()
                .map_or(0x1000, |h| u64::from(h.size_of_headers))
        {
            return Ok(usize::try_from(rva).unwrap_or(0));
        }

        Err(Error::InvalidAddress)
    }

    /// Read null-terminated string
    fn read_string(&self, data: &[u8], offset: usize) -> String {
        if offset >= data.len() {
            return String::new();
        }

        let end = data[offset..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| offset + p)
            .unwrap_or(data.len());

        String::from_utf8_lossy(&data[offset..end]).into_owned()
    }

    /// Build loaded image
    fn build_image(&self) -> Result<LoadedImage> {
        let coff = self.coff_header.as_ref().ok_or(Error::NotLoaded)?;
        let optional = self.optional_header.as_ref().ok_or(Error::NotLoaded)?;

        // Build sections
        let mut sections = Vec::new();

        for shdr in &self.section_headers {
            sections.push(ImageSection {
                name: shdr.name_str(),
                virtual_address: VirtualAddress(
                    optional.image_base + u64::from(shdr.virtual_address),
                ),
                size: u64::from(shdr.virtual_size),
                file_offset: u64::from(shdr.pointer_to_raw_data),
                file_size: u64::from(shdr.size_of_raw_data),
                alignment: u64::from(shdr.alignment()),
                flags: shdr.to_section_flags(),
            });
        }

        // Find BSS
        let bss_section = self
            .section_headers
            .iter()
            .find(|s| s.is_uninitialized_data());

        let (bss_start, bss_size) = bss_section.map_or((None, 0), |s| {
            (
                Some(VirtualAddress(
                    optional.image_base + u64::from(s.virtual_address),
                )),
                u64::from(s.virtual_size),
            )
        });

        // Build flags
        let flags = ImageFlags {
            pie: optional.is_dynamic_base(),
            nx_stack: optional.is_nx_compat(),
            relocatable: !self.relocations.is_empty(),
            has_symbols: false,
            stripped: true,
        };

        Ok(LoadedImage {
            format: ImageFormat::Pe32Plus,
            entry_point: VirtualAddress(
                optional.image_base + u64::from(optional.address_of_entry_point),
            ),
            load_address: VirtualAddress(optional.image_base),
            image_size: u64::from(optional.size_of_image),
            sections,
            stack_top: None,
            bss_start,
            bss_size,
            name: String::new(),
            machine: coff.machine_type(),
            flags,
        })
    }

    /// Get DOS header
    pub fn dos_header(&self) -> Option<&DosHeader> {
        self.dos_header.as_ref()
    }

    /// Get COFF header
    pub fn coff_header(&self) -> Option<&CoffHeader> {
        self.coff_header.as_ref()
    }

    /// Get optional header
    pub fn optional_header(&self) -> Option<&OptionalHeader64> {
        self.optional_header.as_ref()
    }

    /// Get section headers
    pub fn section_headers(&self) -> &[PeSectionHeader] {
        &self.section_headers
    }

    /// Get exports
    pub fn exports(&self) -> &[PeExport] {
        &self.exports
    }

    /// Get imports
    pub fn imports(&self) -> &[PeImport] {
        &self.imports
    }

    /// Get relocations
    pub fn relocations(&self) -> &[PeRelocation] {
        &self.relocations
    }

    /// Find export by name
    pub fn find_export(&self, name: &str) -> Option<&PeExport> {
        self.exports.iter().find(|e| e.name == name)
    }

    /// Get loaded image
    pub fn image(&self) -> Option<&LoadedImage> {
        self.image.as_ref()
    }
}

impl Default for PeLoader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// PE EXPORT
// =============================================================================

/// PE export entry
#[derive(Debug, Clone)]
pub struct PeExport {
    /// Export name
    pub name: String,
    /// Ordinal
    pub ordinal: u16,
    /// RVA
    pub address: u64,
}

// =============================================================================
// PE IMPORT
// =============================================================================

/// PE import entry
#[derive(Debug, Clone)]
pub struct PeImport {
    /// DLL name
    pub dll_name: String,
    /// Imported functions
    pub functions: Vec<String>,
}

// =============================================================================
// PE RELOCATION
// =============================================================================

/// PE relocation entry
#[derive(Debug, Clone)]
pub struct PeRelocation {
    /// Address (RVA)
    pub address: u64,
    /// Relocation type
    pub reloc_type: u16,
}

impl PeRelocation {
    /// Get type name
    pub fn type_name(&self) -> &'static str {
        match self.reloc_type {
            reloc_type::IMAGE_REL_BASED_ABSOLUTE => "ABSOLUTE",
            reloc_type::IMAGE_REL_BASED_HIGH => "HIGH",
            reloc_type::IMAGE_REL_BASED_LOW => "LOW",
            reloc_type::IMAGE_REL_BASED_HIGHLOW => "HIGHLOW",
            reloc_type::IMAGE_REL_BASED_HIGHADJ => "HIGHADJ",
            reloc_type::IMAGE_REL_BASED_DIR64 => "DIR64",
            _ => "UNKNOWN",
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
    fn test_dos_magic() {
        assert_eq!(DOS_MAGIC, [b'M', b'Z']);
    }

    #[test]
    fn test_pe_signature() {
        assert_eq!(PE_SIGNATURE, [b'P', b'E', 0, 0]);
    }

    #[test]
    fn test_machine_types() {
        let coff = CoffHeader {
            machine: machine::IMAGE_FILE_MACHINE_AMD64,
            number_of_sections: 0,
            time_date_stamp: 0,
            pointer_to_symbol_table: 0,
            number_of_symbols: 0,
            size_of_optional_header: 0,
            characteristics: 0,
        };

        assert_eq!(coff.machine_type(), MachineType::X86_64);
    }

    #[test]
    fn test_section_flags() {
        let section = PeSectionHeader {
            name: [b'.', b't', b'e', b'x', b't', 0, 0, 0],
            virtual_size: 0x1000,
            virtual_address: 0x1000,
            size_of_raw_data: 0x1000,
            pointer_to_raw_data: 0x400,
            pointer_to_relocations: 0,
            pointer_to_linenumbers: 0,
            number_of_relocations: 0,
            number_of_linenumbers: 0,
            characteristics: section_characteristics::IMAGE_SCN_CNT_CODE
                | section_characteristics::IMAGE_SCN_MEM_EXECUTE
                | section_characteristics::IMAGE_SCN_MEM_READ,
        };

        assert!(section.is_code());
        assert!(section.is_executable());
        assert!(section.is_readable());
        assert!(!section.is_writable());
    }
}
