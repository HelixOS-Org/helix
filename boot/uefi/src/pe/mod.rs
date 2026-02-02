//! PE/COFF Parser
//!
//! Comprehensive PE32+ parser for Windows kernel and UEFI application loading.

use core::fmt;

// =============================================================================
// DOS HEADER
// =============================================================================

/// DOS magic bytes
pub const DOS_MAGIC: u16 = 0x5A4D; // "MZ"

/// DOS header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DosHeader {
    /// Magic number (MZ)
    pub e_magic: u16,
    /// Bytes on last page
    pub e_cblp: u16,
    /// Pages in file
    pub e_cp: u16,
    /// Relocations
    pub e_crlc: u16,
    /// Size of header in paragraphs
    pub e_cparhdr: u16,
    /// Minimum extra paragraphs
    pub e_minalloc: u16,
    /// Maximum extra paragraphs
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
    /// File offset of relocation table
    pub e_lfarlc: u16,
    /// Overlay number
    pub e_ovno: u16,
    /// Reserved
    pub e_res: [u16; 4],
    /// OEM identifier
    pub e_oemid: u16,
    /// OEM info
    pub e_oeminfo: u16,
    /// Reserved
    pub e_res2: [u16; 10],
    /// File offset of PE header
    pub e_lfanew: i32,
}

impl DosHeader {
    /// Size of DOS header
    pub const SIZE: usize = 64;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let magic = u16::from_le_bytes([bytes[0], bytes[1]]);
        if magic != DOS_MAGIC {
            return None;
        }

        Some(Self {
            e_magic: magic,
            e_cblp: u16::from_le_bytes([bytes[2], bytes[3]]),
            e_cp: u16::from_le_bytes([bytes[4], bytes[5]]),
            e_crlc: u16::from_le_bytes([bytes[6], bytes[7]]),
            e_cparhdr: u16::from_le_bytes([bytes[8], bytes[9]]),
            e_minalloc: u16::from_le_bytes([bytes[10], bytes[11]]),
            e_maxalloc: u16::from_le_bytes([bytes[12], bytes[13]]),
            e_ss: u16::from_le_bytes([bytes[14], bytes[15]]),
            e_sp: u16::from_le_bytes([bytes[16], bytes[17]]),
            e_csum: u16::from_le_bytes([bytes[18], bytes[19]]),
            e_ip: u16::from_le_bytes([bytes[20], bytes[21]]),
            e_cs: u16::from_le_bytes([bytes[22], bytes[23]]),
            e_lfarlc: u16::from_le_bytes([bytes[24], bytes[25]]),
            e_ovno: u16::from_le_bytes([bytes[26], bytes[27]]),
            e_res: [
                u16::from_le_bytes([bytes[28], bytes[29]]),
                u16::from_le_bytes([bytes[30], bytes[31]]),
                u16::from_le_bytes([bytes[32], bytes[33]]),
                u16::from_le_bytes([bytes[34], bytes[35]]),
            ],
            e_oemid: u16::from_le_bytes([bytes[36], bytes[37]]),
            e_oeminfo: u16::from_le_bytes([bytes[38], bytes[39]]),
            e_res2: [
                u16::from_le_bytes([bytes[40], bytes[41]]),
                u16::from_le_bytes([bytes[42], bytes[43]]),
                u16::from_le_bytes([bytes[44], bytes[45]]),
                u16::from_le_bytes([bytes[46], bytes[47]]),
                u16::from_le_bytes([bytes[48], bytes[49]]),
                u16::from_le_bytes([bytes[50], bytes[51]]),
                u16::from_le_bytes([bytes[52], bytes[53]]),
                u16::from_le_bytes([bytes[54], bytes[55]]),
                u16::from_le_bytes([bytes[56], bytes[57]]),
                u16::from_le_bytes([bytes[58], bytes[59]]),
            ],
            e_lfanew: i32::from_le_bytes(bytes[60..64].try_into().ok()?),
        })
    }

    /// Is valid DOS header
    pub fn is_valid(&self) -> bool {
        self.e_magic == DOS_MAGIC && self.e_lfanew > 0
    }
}

// =============================================================================
// PE SIGNATURE AND COFF HEADER
// =============================================================================

/// PE signature
pub const PE_SIGNATURE: u32 = 0x00004550; // "PE\0\0"

/// COFF file header
#[repr(C, packed)]
#[derive(Clone, Copy)]
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
    /// Size
    pub const SIZE: usize = 20;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            machine: u16::from_le_bytes([bytes[0], bytes[1]]),
            number_of_sections: u16::from_le_bytes([bytes[2], bytes[3]]),
            time_date_stamp: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            pointer_to_symbol_table: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
            number_of_symbols: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
            size_of_optional_header: u16::from_le_bytes([bytes[16], bytes[17]]),
            characteristics: u16::from_le_bytes([bytes[18], bytes[19]]),
        })
    }
}

/// Machine types
pub mod machine {
    pub const UNKNOWN: u16 = 0x0;
    pub const I386: u16 = 0x14c;
    pub const AMD64: u16 = 0x8664;
    pub const ARM: u16 = 0x1c0;
    pub const ARM64: u16 = 0xaa64;
    pub const RISCV32: u16 = 0x5032;
    pub const RISCV64: u16 = 0x5064;
    pub const EBC: u16 = 0xebc; // EFI byte code
}

/// Characteristics
pub mod characteristics {
    pub const RELOCS_STRIPPED: u16 = 0x0001;
    pub const EXECUTABLE_IMAGE: u16 = 0x0002;
    pub const LINE_NUMS_STRIPPED: u16 = 0x0004;
    pub const LOCAL_SYMS_STRIPPED: u16 = 0x0008;
    pub const AGGRESSIVE_WS_TRIM: u16 = 0x0010;
    pub const LARGE_ADDRESS_AWARE: u16 = 0x0020;
    pub const BYTES_REVERSED_LO: u16 = 0x0080;
    pub const MACHINE_32BIT: u16 = 0x0100;
    pub const DEBUG_STRIPPED: u16 = 0x0200;
    pub const REMOVABLE_RUN_FROM_SWAP: u16 = 0x0400;
    pub const NET_RUN_FROM_SWAP: u16 = 0x0800;
    pub const SYSTEM: u16 = 0x1000;
    pub const DLL: u16 = 0x2000;
    pub const UP_SYSTEM_ONLY: u16 = 0x4000;
    pub const BYTES_REVERSED_HI: u16 = 0x8000;
}

// =============================================================================
// OPTIONAL HEADER
// =============================================================================

/// Optional header magic
pub mod optional_magic {
    pub const PE32: u16 = 0x10b;
    pub const PE32_PLUS: u16 = 0x20b;
    pub const ROM: u16 = 0x107;
}

/// Subsystem
pub mod subsystem {
    pub const UNKNOWN: u16 = 0;
    pub const NATIVE: u16 = 1;
    pub const WINDOWS_GUI: u16 = 2;
    pub const WINDOWS_CUI: u16 = 3;
    pub const OS2_CUI: u16 = 5;
    pub const POSIX_CUI: u16 = 7;
    pub const NATIVE_WINDOWS: u16 = 8;
    pub const WINDOWS_CE_GUI: u16 = 9;
    pub const EFI_APPLICATION: u16 = 10;
    pub const EFI_BOOT_SERVICE_DRIVER: u16 = 11;
    pub const EFI_RUNTIME_DRIVER: u16 = 12;
    pub const EFI_ROM: u16 = 13;
    pub const XBOX: u16 = 14;
    pub const WINDOWS_BOOT_APPLICATION: u16 = 16;
}

/// DLL characteristics
pub mod dll_characteristics {
    pub const HIGH_ENTROPY_VA: u16 = 0x0020;
    pub const DYNAMIC_BASE: u16 = 0x0040;
    pub const FORCE_INTEGRITY: u16 = 0x0080;
    pub const NX_COMPAT: u16 = 0x0100;
    pub const NO_ISOLATION: u16 = 0x0200;
    pub const NO_SEH: u16 = 0x0400;
    pub const NO_BIND: u16 = 0x0800;
    pub const APPCONTAINER: u16 = 0x1000;
    pub const WDM_DRIVER: u16 = 0x2000;
    pub const GUARD_CF: u16 = 0x4000;
    pub const TERMINAL_SERVER_AWARE: u16 = 0x8000;
}

/// PE32+ optional header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct OptionalHeader64 {
    /// Magic
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
    /// Major OS version
    pub major_operating_system_version: u16,
    /// Minor OS version
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
    /// Number of data directories
    pub number_of_rva_and_sizes: u32,
}

impl OptionalHeader64 {
    /// Size (without data directories)
    pub const SIZE: usize = 112;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            magic: u16::from_le_bytes([bytes[0], bytes[1]]),
            major_linker_version: bytes[2],
            minor_linker_version: bytes[3],
            size_of_code: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            size_of_initialized_data: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
            size_of_uninitialized_data: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
            address_of_entry_point: u32::from_le_bytes(bytes[16..20].try_into().ok()?),
            base_of_code: u32::from_le_bytes(bytes[20..24].try_into().ok()?),
            image_base: u64::from_le_bytes(bytes[24..32].try_into().ok()?),
            section_alignment: u32::from_le_bytes(bytes[32..36].try_into().ok()?),
            file_alignment: u32::from_le_bytes(bytes[36..40].try_into().ok()?),
            major_operating_system_version: u16::from_le_bytes([bytes[40], bytes[41]]),
            minor_operating_system_version: u16::from_le_bytes([bytes[42], bytes[43]]),
            major_image_version: u16::from_le_bytes([bytes[44], bytes[45]]),
            minor_image_version: u16::from_le_bytes([bytes[46], bytes[47]]),
            major_subsystem_version: u16::from_le_bytes([bytes[48], bytes[49]]),
            minor_subsystem_version: u16::from_le_bytes([bytes[50], bytes[51]]),
            win32_version_value: u32::from_le_bytes(bytes[52..56].try_into().ok()?),
            size_of_image: u32::from_le_bytes(bytes[56..60].try_into().ok()?),
            size_of_headers: u32::from_le_bytes(bytes[60..64].try_into().ok()?),
            check_sum: u32::from_le_bytes(bytes[64..68].try_into().ok()?),
            subsystem: u16::from_le_bytes([bytes[68], bytes[69]]),
            dll_characteristics: u16::from_le_bytes([bytes[70], bytes[71]]),
            size_of_stack_reserve: u64::from_le_bytes(bytes[72..80].try_into().ok()?),
            size_of_stack_commit: u64::from_le_bytes(bytes[80..88].try_into().ok()?),
            size_of_heap_reserve: u64::from_le_bytes(bytes[88..96].try_into().ok()?),
            size_of_heap_commit: u64::from_le_bytes(bytes[96..104].try_into().ok()?),
            loader_flags: u32::from_le_bytes(bytes[104..108].try_into().ok()?),
            number_of_rva_and_sizes: u32::from_le_bytes(bytes[108..112].try_into().ok()?),
        })
    }

    /// Is PE32+
    pub fn is_pe32_plus(&self) -> bool {
        self.magic == optional_magic::PE32_PLUS
    }

    /// Is EFI application
    pub fn is_efi_application(&self) -> bool {
        self.subsystem == subsystem::EFI_APPLICATION
    }

    /// Is EFI boot service driver
    pub fn is_efi_boot_driver(&self) -> bool {
        self.subsystem == subsystem::EFI_BOOT_SERVICE_DRIVER
    }

    /// Is EFI runtime driver
    pub fn is_efi_runtime_driver(&self) -> bool {
        self.subsystem == subsystem::EFI_RUNTIME_DRIVER
    }

    /// Is any EFI type
    pub fn is_efi(&self) -> bool {
        matches!(
            self.subsystem,
            subsystem::EFI_APPLICATION
                | subsystem::EFI_BOOT_SERVICE_DRIVER
                | subsystem::EFI_RUNTIME_DRIVER
                | subsystem::EFI_ROM
        )
    }

    /// Has relocations
    pub fn is_dynamic_base(&self) -> bool {
        self.dll_characteristics & dll_characteristics::DYNAMIC_BASE != 0
    }
}

// =============================================================================
// DATA DIRECTORIES
// =============================================================================

/// Data directory entry
#[repr(C, packed)]
#[derive(Clone, Copy, Default)]
pub struct DataDirectory {
    /// Virtual address
    pub virtual_address: u32,
    /// Size
    pub size: u32,
}

impl DataDirectory {
    /// Size
    pub const SIZE: usize = 8;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            virtual_address: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            size: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
        })
    }

    /// Is present
    pub fn is_present(&self) -> bool {
        self.virtual_address != 0 && self.size != 0
    }
}

/// Data directory indices
pub mod data_directory_index {
    pub const EXPORT: usize = 0;
    pub const IMPORT: usize = 1;
    pub const RESOURCE: usize = 2;
    pub const EXCEPTION: usize = 3;
    pub const CERTIFICATE: usize = 4;
    pub const BASE_RELOC: usize = 5;
    pub const DEBUG: usize = 6;
    pub const ARCHITECTURE: usize = 7;
    pub const GLOBAL_PTR: usize = 8;
    pub const TLS: usize = 9;
    pub const LOAD_CONFIG: usize = 10;
    pub const BOUND_IMPORT: usize = 11;
    pub const IAT: usize = 12;
    pub const DELAY_IMPORT: usize = 13;
    pub const CLR_RUNTIME: usize = 14;
    pub const RESERVED: usize = 15;
}

// =============================================================================
// SECTIONS
// =============================================================================

/// Section header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct SectionHeader {
    /// Section name
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

impl SectionHeader {
    /// Size
    pub const SIZE: usize = 40;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            name: bytes[0..8].try_into().ok()?,
            virtual_size: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
            virtual_address: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
            size_of_raw_data: u32::from_le_bytes(bytes[16..20].try_into().ok()?),
            pointer_to_raw_data: u32::from_le_bytes(bytes[20..24].try_into().ok()?),
            pointer_to_relocations: u32::from_le_bytes(bytes[24..28].try_into().ok()?),
            pointer_to_linenumbers: u32::from_le_bytes(bytes[28..32].try_into().ok()?),
            number_of_relocations: u16::from_le_bytes([bytes[32], bytes[33]]),
            number_of_linenumbers: u16::from_le_bytes([bytes[34], bytes[35]]),
            characteristics: u32::from_le_bytes(bytes[36..40].try_into().ok()?),
        })
    }

    /// Get section name as string
    pub fn name_str(&self) -> &str {
        // Find null terminator or end
        let len = self.name.iter().position(|&c| c == 0).unwrap_or(8);
        core::str::from_utf8(&self.name[..len]).unwrap_or("")
    }

    /// Is code section
    pub fn is_code(&self) -> bool {
        self.characteristics & section_characteristics::CNT_CODE != 0
    }

    /// Is data section
    pub fn is_data(&self) -> bool {
        self.characteristics
            & (section_characteristics::CNT_INITIALIZED_DATA
                | section_characteristics::CNT_UNINITIALIZED_DATA)
            != 0
    }

    /// Is readable
    pub fn is_readable(&self) -> bool {
        self.characteristics & section_characteristics::MEM_READ != 0
    }

    /// Is writable
    pub fn is_writable(&self) -> bool {
        self.characteristics & section_characteristics::MEM_WRITE != 0
    }

    /// Is executable
    pub fn is_executable(&self) -> bool {
        self.characteristics & section_characteristics::MEM_EXECUTE != 0
    }

    /// Is discardable
    pub fn is_discardable(&self) -> bool {
        self.characteristics & section_characteristics::MEM_DISCARDABLE != 0
    }

    /// Contains virtual address
    pub fn contains_rva(&self, rva: u32) -> bool {
        rva >= self.virtual_address && rva < self.virtual_address + self.virtual_size
    }

    /// RVA to file offset
    pub fn rva_to_offset(&self, rva: u32) -> Option<u32> {
        if self.contains_rva(rva) {
            Some(rva - self.virtual_address + self.pointer_to_raw_data)
        } else {
            None
        }
    }
}

/// Section characteristics
pub mod section_characteristics {
    pub const TYPE_NO_PAD: u32 = 0x00000008;
    pub const CNT_CODE: u32 = 0x00000020;
    pub const CNT_INITIALIZED_DATA: u32 = 0x00000040;
    pub const CNT_UNINITIALIZED_DATA: u32 = 0x00000080;
    pub const LNK_OTHER: u32 = 0x00000100;
    pub const LNK_INFO: u32 = 0x00000200;
    pub const LNK_REMOVE: u32 = 0x00000800;
    pub const LNK_COMDAT: u32 = 0x00001000;
    pub const GPREL: u32 = 0x00008000;
    pub const MEM_16BIT: u32 = 0x00020000;
    pub const MEM_LOCKED: u32 = 0x00040000;
    pub const MEM_PRELOAD: u32 = 0x00080000;
    pub const ALIGN_1BYTES: u32 = 0x00100000;
    pub const ALIGN_2BYTES: u32 = 0x00200000;
    pub const ALIGN_4BYTES: u32 = 0x00300000;
    pub const ALIGN_8BYTES: u32 = 0x00400000;
    pub const ALIGN_16BYTES: u32 = 0x00500000;
    pub const ALIGN_32BYTES: u32 = 0x00600000;
    pub const ALIGN_64BYTES: u32 = 0x00700000;
    pub const ALIGN_128BYTES: u32 = 0x00800000;
    pub const ALIGN_256BYTES: u32 = 0x00900000;
    pub const ALIGN_512BYTES: u32 = 0x00A00000;
    pub const ALIGN_1024BYTES: u32 = 0x00B00000;
    pub const ALIGN_2048BYTES: u32 = 0x00C00000;
    pub const ALIGN_4096BYTES: u32 = 0x00D00000;
    pub const ALIGN_8192BYTES: u32 = 0x00E00000;
    pub const LNK_NRELOC_OVFL: u32 = 0x01000000;
    pub const MEM_DISCARDABLE: u32 = 0x02000000;
    pub const MEM_NOT_CACHED: u32 = 0x04000000;
    pub const MEM_NOT_PAGED: u32 = 0x08000000;
    pub const MEM_SHARED: u32 = 0x10000000;
    pub const MEM_EXECUTE: u32 = 0x20000000;
    pub const MEM_READ: u32 = 0x40000000;
    pub const MEM_WRITE: u32 = 0x80000000;
}

// =============================================================================
// RELOCATIONS
// =============================================================================

/// Base relocation block
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct BaseRelocationBlock {
    /// Page RVA
    pub virtual_address: u32,
    /// Block size
    pub size_of_block: u32,
}

impl BaseRelocationBlock {
    /// Size
    pub const SIZE: usize = 8;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            virtual_address: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            size_of_block: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
        })
    }

    /// Number of entries
    pub fn entry_count(&self) -> usize {
        (self.size_of_block as usize - Self::SIZE) / 2
    }
}

/// Relocation types
pub mod reloc_type {
    pub const ABSOLUTE: u16 = 0;
    pub const HIGH: u16 = 1;
    pub const LOW: u16 = 2;
    pub const HIGHLOW: u16 = 3;
    pub const HIGHADJ: u16 = 4;
    pub const DIR64: u16 = 10;
}

/// Base relocation entry
#[derive(Clone, Copy)]
pub struct BaseRelocationEntry {
    /// Offset within page
    pub offset: u16,
    /// Relocation type
    pub reloc_type: u16,
}

impl BaseRelocationEntry {
    /// Parse from u16
    pub fn from_u16(value: u16) -> Self {
        Self {
            offset: value & 0x0FFF,
            reloc_type: value >> 12,
        }
    }

    /// Is valid (not padding)
    pub fn is_valid(&self) -> bool {
        self.reloc_type != reloc_type::ABSOLUTE
    }
}

// =============================================================================
// EXPORTS
// =============================================================================

/// Export directory
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ExportDirectory {
    /// Export flags
    pub characteristics: u32,
    /// Time date stamp
    pub time_date_stamp: u32,
    /// Major version
    pub major_version: u16,
    /// Minor version
    pub minor_version: u16,
    /// Name RVA
    pub name: u32,
    /// Ordinal base
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

impl ExportDirectory {
    /// Size
    pub const SIZE: usize = 40;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            characteristics: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            time_date_stamp: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            major_version: u16::from_le_bytes([bytes[8], bytes[9]]),
            minor_version: u16::from_le_bytes([bytes[10], bytes[11]]),
            name: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
            base: u32::from_le_bytes(bytes[16..20].try_into().ok()?),
            number_of_functions: u32::from_le_bytes(bytes[20..24].try_into().ok()?),
            number_of_names: u32::from_le_bytes(bytes[24..28].try_into().ok()?),
            address_of_functions: u32::from_le_bytes(bytes[28..32].try_into().ok()?),
            address_of_names: u32::from_le_bytes(bytes[32..36].try_into().ok()?),
            address_of_name_ordinals: u32::from_le_bytes(bytes[36..40].try_into().ok()?),
        })
    }
}

// =============================================================================
// IMPORTS
// =============================================================================

/// Import descriptor
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct ImportDescriptor {
    /// Import lookup table RVA (or characteristics)
    pub original_first_thunk: u32,
    /// Time date stamp
    pub time_date_stamp: u32,
    /// Forwarder chain
    pub forwarder_chain: u32,
    /// Module name RVA
    pub name: u32,
    /// Import address table RVA
    pub first_thunk: u32,
}

impl ImportDescriptor {
    /// Size
    pub const SIZE: usize = 20;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            original_first_thunk: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            time_date_stamp: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            forwarder_chain: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
            name: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
            first_thunk: u32::from_le_bytes(bytes[16..20].try_into().ok()?),
        })
    }

    /// Is null terminator
    pub fn is_null(&self) -> bool {
        self.original_first_thunk == 0 && self.name == 0 && self.first_thunk == 0
    }
}

// =============================================================================
// PE FILE
// =============================================================================

/// Maximum number of data directories
pub const MAX_DATA_DIRECTORIES: usize = 16;

/// Maximum number of sections
pub const MAX_SECTIONS: usize = 96;

/// Parsed PE file
pub struct PeFile<'a> {
    /// Raw data
    data: &'a [u8],
    /// DOS header
    dos_header: DosHeader,
    /// COFF header
    coff_header: CoffHeader,
    /// Optional header
    optional_header: OptionalHeader64,
    /// Data directories
    data_directories: [DataDirectory; MAX_DATA_DIRECTORIES],
    /// Section headers
    sections: [Option<SectionHeader>; MAX_SECTIONS],
    /// Number of sections
    section_count: usize,
}

impl<'a> PeFile<'a> {
    /// Parse PE file
    pub fn parse(data: &'a [u8]) -> Result<Self, PeError> {
        // Parse DOS header
        let dos_header = DosHeader::from_bytes(data).ok_or(PeError::InvalidDosHeader)?;

        if !dos_header.is_valid() {
            return Err(PeError::InvalidDosHeader);
        }

        // Get PE header offset
        let pe_offset = dos_header.e_lfanew as usize;

        if pe_offset + 4 > data.len() {
            return Err(PeError::InvalidPeSignature);
        }

        // Check PE signature
        let signature = u32::from_le_bytes(
            data[pe_offset..pe_offset + 4]
                .try_into()
                .map_err(|_| PeError::InvalidPeSignature)?,
        );

        if signature != PE_SIGNATURE {
            return Err(PeError::InvalidPeSignature);
        }

        // Parse COFF header
        let coff_offset = pe_offset + 4;
        let coff_header =
            CoffHeader::from_bytes(&data[coff_offset..]).ok_or(PeError::InvalidCoffHeader)?;

        // Parse optional header
        let opt_offset = coff_offset + CoffHeader::SIZE;
        let optional_header = OptionalHeader64::from_bytes(&data[opt_offset..])
            .ok_or(PeError::InvalidOptionalHeader)?;

        if !optional_header.is_pe32_plus() {
            return Err(PeError::Not64Bit);
        }

        // Parse data directories
        let mut data_directories = [DataDirectory::default(); MAX_DATA_DIRECTORIES];
        let dir_offset = opt_offset + OptionalHeader64::SIZE;
        let dir_count =
            (optional_header.number_of_rva_and_sizes as usize).min(MAX_DATA_DIRECTORIES);

        for i in 0..dir_count {
            let offset = dir_offset + i * DataDirectory::SIZE;
            if offset + DataDirectory::SIZE <= data.len() {
                if let Some(dir) = DataDirectory::from_bytes(&data[offset..]) {
                    data_directories[i] = dir;
                }
            }
        }

        // Parse section headers
        let section_offset = opt_offset + coff_header.size_of_optional_header as usize;
        let section_count = (coff_header.number_of_sections as usize).min(MAX_SECTIONS);
        let mut sections = [None; MAX_SECTIONS];

        for i in 0..section_count {
            let offset = section_offset + i * SectionHeader::SIZE;
            if offset + SectionHeader::SIZE <= data.len() {
                sections[i] = SectionHeader::from_bytes(&data[offset..]);
            }
        }

        Ok(Self {
            data,
            dos_header,
            coff_header,
            optional_header,
            data_directories,
            sections,
            section_count,
        })
    }

    /// Get DOS header
    pub fn dos_header(&self) -> &DosHeader {
        &self.dos_header
    }

    /// Get COFF header
    pub fn coff_header(&self) -> &CoffHeader {
        &self.coff_header
    }

    /// Get optional header
    pub fn optional_header(&self) -> &OptionalHeader64 {
        &self.optional_header
    }

    /// Get entry point RVA
    pub fn entry_point(&self) -> u32 {
        self.optional_header.address_of_entry_point
    }

    /// Get image base
    pub fn image_base(&self) -> u64 {
        self.optional_header.image_base
    }

    /// Get image size
    pub fn image_size(&self) -> u32 {
        self.optional_header.size_of_image
    }

    /// Get section count
    pub fn section_count(&self) -> usize {
        self.section_count
    }

    /// Get section by index
    pub fn section(&self, index: usize) -> Option<&SectionHeader> {
        self.sections.get(index)?.as_ref()
    }

    /// Iterate sections
    pub fn sections(&self) -> impl Iterator<Item = &SectionHeader> {
        self.sections.iter().filter_map(|s| s.as_ref())
    }

    /// Find section by name
    pub fn find_section(&self, name: &str) -> Option<&SectionHeader> {
        self.sections().find(|s| s.name_str() == name)
    }

    /// Find section containing RVA
    pub fn section_for_rva(&self, rva: u32) -> Option<&SectionHeader> {
        self.sections().find(|s| s.contains_rva(rva))
    }

    /// Convert RVA to file offset
    pub fn rva_to_offset(&self, rva: u32) -> Option<u32> {
        self.section_for_rva(rva)?.rva_to_offset(rva)
    }

    /// Get data at RVA
    pub fn data_at_rva(&self, rva: u32, size: usize) -> Option<&[u8]> {
        let offset = self.rva_to_offset(rva)? as usize;
        let end = offset + size;

        if end > self.data.len() {
            return None;
        }

        Some(&self.data[offset..end])
    }

    /// Get data directory
    pub fn data_directory(&self, index: usize) -> Option<&DataDirectory> {
        self.data_directories.get(index)
    }

    /// Get export directory
    pub fn export_directory(&self) -> Option<ExportDirectory> {
        let dir = self.data_directory(data_directory_index::EXPORT)?;
        if !dir.is_present() {
            return None;
        }

        let data = self.data_at_rva(dir.virtual_address, ExportDirectory::SIZE)?;
        ExportDirectory::from_bytes(data)
    }

    /// Get section data
    pub fn section_data(&self, section: &SectionHeader) -> Option<&[u8]> {
        let start = section.pointer_to_raw_data as usize;
        let end = start + section.size_of_raw_data as usize;

        if end > self.data.len() {
            return None;
        }

        Some(&self.data[start..end])
    }

    /// Is EFI application
    pub fn is_efi(&self) -> bool {
        self.optional_header.is_efi()
    }

    /// Is x86_64
    pub fn is_x86_64(&self) -> bool {
        self.coff_header.machine == machine::AMD64
    }

    /// Is ARM64
    pub fn is_arm64(&self) -> bool {
        self.coff_header.machine == machine::ARM64
    }

    /// Get string at RVA
    pub fn string_at_rva(&self, rva: u32) -> Option<&str> {
        let offset = self.rva_to_offset(rva)? as usize;

        // Find null terminator
        let mut end = offset;
        while end < self.data.len() && self.data[end] != 0 {
            end += 1;
        }

        core::str::from_utf8(&self.data[offset..end]).ok()
    }
}

impl fmt::Debug for PeFile<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let machine = self.coff_header.machine;
        f.debug_struct("PeFile")
            .field("machine", &machine)
            .field("sections", &self.section_count)
            .field("entry_point", &format_args!("0x{:x}", self.entry_point()))
            .field("image_base", &format_args!("0x{:x}", self.image_base()))
            .field("image_size", &format_args!("0x{:x}", self.image_size()))
            .finish()
    }
}

// =============================================================================
// PE LOADER
// =============================================================================

/// PE loader
pub struct PeLoader;

impl PeLoader {
    /// Load PE into memory
    pub fn load(pe: &PeFile, memory: &mut [u8], base: u64) -> Result<LoadedPe, PeError> {
        let image_size = pe.image_size() as usize;

        if memory.len() < image_size {
            return Err(PeError::BufferTooSmall);
        }

        // Clear memory
        for byte in memory[..image_size].iter_mut() {
            *byte = 0;
        }

        // Copy headers
        let header_size = pe.optional_header.size_of_headers as usize;
        if header_size > pe.data.len() || header_size > memory.len() {
            return Err(PeError::InvalidHeaders);
        }
        memory[..header_size].copy_from_slice(&pe.data[..header_size]);

        // Copy sections
        for section in pe.sections() {
            if section.size_of_raw_data == 0 {
                continue;
            }

            let src_start = section.pointer_to_raw_data as usize;
            let src_end = src_start + section.size_of_raw_data as usize;
            let dst_start = section.virtual_address as usize;
            let dst_end = dst_start + section.size_of_raw_data as usize;

            if src_end > pe.data.len() || dst_end > memory.len() {
                return Err(PeError::InvalidSection);
            }

            memory[dst_start..dst_end].copy_from_slice(&pe.data[src_start..src_end]);
        }

        Ok(LoadedPe {
            entry_point: base + pe.entry_point() as u64,
            base_address: base,
            size: image_size,
        })
    }

    /// Apply relocations
    pub fn relocate(pe: &PeFile, memory: &mut [u8], load_base: u64) -> Result<(), PeError> {
        let reloc_dir = pe.data_directory(data_directory_index::BASE_RELOC);

        let reloc_dir = match reloc_dir {
            Some(dir) if dir.is_present() => dir,
            _ => return Ok(()), // No relocations needed
        };

        let delta = load_base.wrapping_sub(pe.image_base());

        if delta == 0 {
            return Ok(()); // Loaded at preferred address
        }

        let mut offset = 0u32;

        while offset < reloc_dir.size {
            let block_rva = reloc_dir.virtual_address + offset;
            let block_data = pe
                .data_at_rva(block_rva, BaseRelocationBlock::SIZE)
                .ok_or(PeError::InvalidRelocation)?;

            let block =
                BaseRelocationBlock::from_bytes(block_data).ok_or(PeError::InvalidRelocation)?;

            if block.size_of_block == 0 {
                break;
            }

            // Process entries
            for i in 0..block.entry_count() {
                let entry_offset = block_rva + BaseRelocationBlock::SIZE as u32 + (i * 2) as u32;
                let entry_data = pe
                    .data_at_rva(entry_offset, 2)
                    .ok_or(PeError::InvalidRelocation)?;

                let entry_value = u16::from_le_bytes([entry_data[0], entry_data[1]]);
                let entry = BaseRelocationEntry::from_u16(entry_value);

                if !entry.is_valid() {
                    continue;
                }

                let reloc_offset = (block.virtual_address + entry.offset as u32) as usize;

                match entry.reloc_type {
                    reloc_type::DIR64 => {
                        if reloc_offset + 8 > memory.len() {
                            return Err(PeError::InvalidRelocation);
                        }

                        let value = u64::from_le_bytes(
                            memory[reloc_offset..reloc_offset + 8].try_into().unwrap(),
                        );
                        let new_value = value.wrapping_add(delta);
                        memory[reloc_offset..reloc_offset + 8]
                            .copy_from_slice(&new_value.to_le_bytes());
                    },
                    reloc_type::HIGHLOW => {
                        if reloc_offset + 4 > memory.len() {
                            return Err(PeError::InvalidRelocation);
                        }

                        let value = u32::from_le_bytes(
                            memory[reloc_offset..reloc_offset + 4].try_into().unwrap(),
                        );
                        let new_value = value.wrapping_add(delta as u32);
                        memory[reloc_offset..reloc_offset + 4]
                            .copy_from_slice(&new_value.to_le_bytes());
                    },
                    _ => {}, // Ignore other types
                }
            }

            offset += block.size_of_block;
        }

        Ok(())
    }
}

/// Loaded PE info
#[derive(Debug, Clone)]
pub struct LoadedPe {
    /// Entry point address
    pub entry_point: u64,
    /// Base load address
    pub base_address: u64,
    /// Image size
    pub size: usize,
}

// =============================================================================
// PE ERROR
// =============================================================================

/// PE error
#[derive(Debug, Clone)]
pub enum PeError {
    /// Invalid DOS header
    InvalidDosHeader,
    /// Invalid PE signature
    InvalidPeSignature,
    /// Invalid COFF header
    InvalidCoffHeader,
    /// Invalid optional header
    InvalidOptionalHeader,
    /// Not 64-bit
    Not64Bit,
    /// Invalid headers
    InvalidHeaders,
    /// Invalid section
    InvalidSection,
    /// Invalid relocation
    InvalidRelocation,
    /// Buffer too small
    BufferTooSmall,
}

impl fmt::Display for PeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDosHeader => write!(f, "invalid DOS header"),
            Self::InvalidPeSignature => write!(f, "invalid PE signature"),
            Self::InvalidCoffHeader => write!(f, "invalid COFF header"),
            Self::InvalidOptionalHeader => write!(f, "invalid optional header"),
            Self::Not64Bit => write!(f, "not a 64-bit PE file"),
            Self::InvalidHeaders => write!(f, "invalid headers"),
            Self::InvalidSection => write!(f, "invalid section"),
            Self::InvalidRelocation => write!(f, "invalid relocation"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dos_magic() {
        assert_eq!(DOS_MAGIC, 0x5A4D);
    }

    #[test]
    fn test_pe_signature() {
        assert_eq!(PE_SIGNATURE, 0x00004550);
    }

    #[test]
    fn test_relocation_entry() {
        let entry = BaseRelocationEntry::from_u16(0xA123);
        assert_eq!(entry.offset, 0x123);
        assert_eq!(entry.reloc_type, 0xA);
    }

    #[test]
    fn test_section_characteristics() {
        let section = SectionHeader {
            name: [b'.', b't', b'e', b'x', b't', 0, 0, 0],
            virtual_size: 0x1000,
            virtual_address: 0x1000,
            size_of_raw_data: 0x1000,
            pointer_to_raw_data: 0x200,
            pointer_to_relocations: 0,
            pointer_to_linenumbers: 0,
            number_of_relocations: 0,
            number_of_linenumbers: 0,
            characteristics: section_characteristics::CNT_CODE
                | section_characteristics::MEM_READ
                | section_characteristics::MEM_EXECUTE,
        };

        assert!(section.is_code());
        assert!(section.is_readable());
        assert!(section.is_executable());
        assert!(!section.is_writable());
        assert_eq!(section.name_str(), ".text");
    }
}
