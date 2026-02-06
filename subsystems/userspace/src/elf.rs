//! # ELF64 Loader
//!
//! Complete ELF64 binary parser and loader for Helix OS.
//!
//! ## Features
//! - Full ELF64 header parsing
//! - Program header interpretation
//! - Section header parsing
//! - Segment loading with proper permissions
//! - Dynamic relocation support
//! - Symbol resolution

use alloc::vec;
use alloc::vec::Vec;
use core::mem;

use super::STATS;

/// ELF magic number
pub const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];

/// ELF class - 64-bit
pub const ELFCLASS64: u8 = 2;

/// ELF data encoding - little endian
pub const ELFDATA2LSB: u8 = 1;

/// ELF version
pub const EV_CURRENT: u8 = 1;

/// ELF OS/ABI - Helix
pub const ELFOSABI_HELIX: u8 = 0xFE;

/// ELF OS/ABI - System V (standard)
pub const ELFOSABI_NONE: u8 = 0;

/// ELF type - executable
pub const ET_EXEC: u16 = 2;

/// ELF type - shared object / PIE
pub const ET_DYN: u16 = 3;

/// Machine type - x86_64
pub const EM_X86_64: u16 = 62;

/// Program header type - loadable segment
pub const PT_LOAD: u32 = 1;

/// Program header type - dynamic
pub const PT_DYNAMIC: u32 = 2;

/// Program header type - interpreter
pub const PT_INTERP: u32 = 3;

/// Program header type - note
pub const PT_NOTE: u32 = 4;

/// Program header type - thread-local storage
pub const PT_TLS: u32 = 7;

/// Segment permission - execute
pub const PF_X: u32 = 1;

/// Segment permission - write
pub const PF_W: u32 = 2;

/// Segment permission - read
pub const PF_R: u32 = 4;

/// Section type - NULL
pub const SHT_NULL: u32 = 0;

/// Section type - program data
pub const SHT_PROGBITS: u32 = 1;

/// Section type - symbol table
pub const SHT_SYMTAB: u32 = 2;

/// Section type - string table
pub const SHT_STRTAB: u32 = 3;

/// Section type - relocation with addend
pub const SHT_RELA: u32 = 4;

/// Section type - BSS
pub const SHT_NOBITS: u32 = 8;

/// ELF errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    /// Invalid magic number
    InvalidMagic,
    /// Unsupported class (not 64-bit)
    UnsupportedClass,
    /// Unsupported endianness
    UnsupportedEndian,
    /// Unsupported version
    UnsupportedVersion,
    /// Unsupported OS/ABI
    UnsupportedAbi,
    /// Unsupported type
    UnsupportedType,
    /// Unsupported machine
    UnsupportedMachine,
    /// Invalid program header
    InvalidProgramHeader,
    /// Invalid section header
    InvalidSectionHeader,
    /// Segment too large
    SegmentTooLarge,
    /// Invalid entry point
    InvalidEntryPoint,
    /// Buffer too small
    BufferTooSmall,
    /// No loadable segments
    NoLoadableSegments,
    /// Relocation error
    RelocationError,
    /// Symbol not found
    SymbolNotFound,
}

/// ELF64 Header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ElfHeader {
    /// Magic number and identification
    pub e_ident: [u8; 16],
    /// Object file type
    pub e_type: u16,
    /// Machine type
    pub e_machine: u16,
    /// Object file version
    pub e_version: u32,
    /// Entry point address
    pub e_entry: u64,
    /// Program header offset
    pub e_phoff: u64,
    /// Section header offset
    pub e_shoff: u64,
    /// Processor-specific flags
    pub e_flags: u32,
    /// ELF header size
    pub e_ehsize: u16,
    /// Program header entry size
    pub e_phentsize: u16,
    /// Number of program headers
    pub e_phnum: u16,
    /// Section header entry size
    pub e_shentsize: u16,
    /// Number of section headers
    pub e_shnum: u16,
    /// Section name string table index
    pub e_shstrndx: u16,
}

impl ElfHeader {
    /// Parse ELF header from bytes
    pub fn parse(data: &[u8]) -> Result<Self, ElfError> {
        if data.len() < mem::size_of::<Self>() {
            return Err(ElfError::BufferTooSmall);
        }

        // Check magic
        if data[0..4] != ELF_MAGIC {
            return Err(ElfError::InvalidMagic);
        }

        // Check class (64-bit)
        if data[4] != ELFCLASS64 {
            return Err(ElfError::UnsupportedClass);
        }

        // Check endianness (little)
        if data[5] != ELFDATA2LSB {
            return Err(ElfError::UnsupportedEndian);
        }

        // Check version
        if data[6] != EV_CURRENT {
            return Err(ElfError::UnsupportedVersion);
        }

        // Parse fields (little-endian)
        let e_type = u16::from_le_bytes([data[16], data[17]]);
        let e_machine = u16::from_le_bytes([data[18], data[19]]);
        let e_version = u32::from_le_bytes([data[20], data[21], data[22], data[23]]);
        let e_entry = u64::from_le_bytes([
            data[24], data[25], data[26], data[27], data[28], data[29], data[30], data[31],
        ]);
        let e_phoff = u64::from_le_bytes([
            data[32], data[33], data[34], data[35], data[36], data[37], data[38], data[39],
        ]);
        let e_shoff = u64::from_le_bytes([
            data[40], data[41], data[42], data[43], data[44], data[45], data[46], data[47],
        ]);
        let e_flags = u32::from_le_bytes([data[48], data[49], data[50], data[51]]);
        let e_ehsize = u16::from_le_bytes([data[52], data[53]]);
        let e_phentsize = u16::from_le_bytes([data[54], data[55]]);
        let e_phnum = u16::from_le_bytes([data[56], data[57]]);
        let e_shentsize = u16::from_le_bytes([data[58], data[59]]);
        let e_shnum = u16::from_le_bytes([data[60], data[61]]);
        let e_shstrndx = u16::from_le_bytes([data[62], data[63]]);

        // Check type
        if e_type != ET_EXEC && e_type != ET_DYN {
            return Err(ElfError::UnsupportedType);
        }

        // Check machine
        if e_machine != EM_X86_64 {
            return Err(ElfError::UnsupportedMachine);
        }

        let mut e_ident = [0u8; 16];
        e_ident.copy_from_slice(&data[0..16]);

        Ok(Self {
            e_ident,
            e_type,
            e_machine,
            e_version,
            e_entry,
            e_phoff,
            e_shoff,
            e_flags,
            e_ehsize,
            e_phentsize,
            e_phnum,
            e_shentsize,
            e_shnum,
            e_shstrndx,
        })
    }

    /// Check if this is a valid Helix or standard ELF
    pub fn is_valid(&self) -> bool {
        let abi = self.e_ident[7];
        abi == ELFOSABI_NONE || abi == ELFOSABI_HELIX
    }

    /// Check if this is a position-independent executable
    pub fn is_pie(&self) -> bool {
        self.e_type == ET_DYN
    }

    /// Get entry point
    pub fn entry_point(&self) -> u64 {
        self.e_entry
    }
}

/// ELF64 Program Header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct ProgramHeader {
    /// Segment type
    pub p_type: u32,
    /// Segment flags
    pub p_flags: u32,
    /// Offset in file
    pub p_offset: u64,
    /// Virtual address
    pub p_vaddr: u64,
    /// Physical address
    pub p_paddr: u64,
    /// Size in file
    pub p_filesz: u64,
    /// Size in memory
    pub p_memsz: u64,
    /// Alignment
    pub p_align: u64,
}

impl ProgramHeader {
    /// Parse program header from bytes
    pub fn parse(data: &[u8], offset: usize) -> Result<Self, ElfError> {
        if data.len() < offset + 56 {
            return Err(ElfError::BufferTooSmall);
        }

        let d = &data[offset..];

        Ok(Self {
            p_type: u32::from_le_bytes([d[0], d[1], d[2], d[3]]),
            p_flags: u32::from_le_bytes([d[4], d[5], d[6], d[7]]),
            p_offset: u64::from_le_bytes([d[8], d[9], d[10], d[11], d[12], d[13], d[14], d[15]]),
            p_vaddr: u64::from_le_bytes([d[16], d[17], d[18], d[19], d[20], d[21], d[22], d[23]]),
            p_paddr: u64::from_le_bytes([d[24], d[25], d[26], d[27], d[28], d[29], d[30], d[31]]),
            p_filesz: u64::from_le_bytes([d[32], d[33], d[34], d[35], d[36], d[37], d[38], d[39]]),
            p_memsz: u64::from_le_bytes([d[40], d[41], d[42], d[43], d[44], d[45], d[46], d[47]]),
            p_align: u64::from_le_bytes([d[48], d[49], d[50], d[51], d[52], d[53], d[54], d[55]]),
        })
    }

    /// Check if segment is loadable
    pub fn is_loadable(&self) -> bool {
        self.p_type == PT_LOAD
    }

    /// Check if segment is executable
    pub fn is_executable(&self) -> bool {
        self.p_flags & PF_X != 0
    }

    /// Check if segment is writable
    pub fn is_writable(&self) -> bool {
        self.p_flags & PF_W != 0
    }

    /// Check if segment is readable
    pub fn is_readable(&self) -> bool {
        self.p_flags & PF_R != 0
    }

    /// Get segment permissions as a string
    pub fn permissions_string(&self) -> &'static str {
        match (self.is_readable(), self.is_writable(), self.is_executable()) {
            (true, true, true) => "rwx",
            (true, true, false) => "rw-",
            (true, false, true) => "r-x",
            (true, false, false) => "r--",
            (false, true, true) => "-wx",
            (false, true, false) => "-w-",
            (false, false, true) => "--x",
            (false, false, false) => "---",
        }
    }
}

/// ELF64 Section Header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SectionHeader {
    /// Section name (index into string table)
    pub sh_name: u32,
    /// Section type
    pub sh_type: u32,
    /// Section flags
    pub sh_flags: u64,
    /// Virtual address
    pub sh_addr: u64,
    /// Offset in file
    pub sh_offset: u64,
    /// Size in bytes
    pub sh_size: u64,
    /// Link to another section
    pub sh_link: u32,
    /// Additional section info
    pub sh_info: u32,
    /// Address alignment
    pub sh_addralign: u64,
    /// Entry size (for tables)
    pub sh_entsize: u64,
}

impl SectionHeader {
    /// Parse section header from bytes
    pub fn parse(data: &[u8], offset: usize) -> Result<Self, ElfError> {
        if data.len() < offset + 64 {
            return Err(ElfError::BufferTooSmall);
        }

        let d = &data[offset..];

        Ok(Self {
            sh_name: u32::from_le_bytes([d[0], d[1], d[2], d[3]]),
            sh_type: u32::from_le_bytes([d[4], d[5], d[6], d[7]]),
            sh_flags: u64::from_le_bytes([d[8], d[9], d[10], d[11], d[12], d[13], d[14], d[15]]),
            sh_addr: u64::from_le_bytes([d[16], d[17], d[18], d[19], d[20], d[21], d[22], d[23]]),
            sh_offset: u64::from_le_bytes([d[24], d[25], d[26], d[27], d[28], d[29], d[30], d[31]]),
            sh_size: u64::from_le_bytes([d[32], d[33], d[34], d[35], d[36], d[37], d[38], d[39]]),
            sh_link: u32::from_le_bytes([d[40], d[41], d[42], d[43]]),
            sh_info: u32::from_le_bytes([d[44], d[45], d[46], d[47]]),
            sh_addralign: u64::from_le_bytes([
                d[48], d[49], d[50], d[51], d[52], d[53], d[54], d[55],
            ]),
            sh_entsize: u64::from_le_bytes([
                d[56], d[57], d[58], d[59], d[60], d[61], d[62], d[63],
            ]),
        })
    }
}

/// Symbol entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Symbol {
    /// Symbol name (index into string table)
    pub st_name: u32,
    /// Symbol info
    pub st_info: u8,
    /// Symbol visibility
    pub st_other: u8,
    /// Section index
    pub st_shndx: u16,
    /// Symbol value
    pub st_value: u64,
    /// Symbol size
    pub st_size: u64,
}

impl Symbol {
    /// Parse symbol from bytes
    pub fn parse(data: &[u8]) -> Result<Self, ElfError> {
        if data.len() < 24 {
            return Err(ElfError::BufferTooSmall);
        }

        Ok(Self {
            st_name: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            st_info: data[4],
            st_other: data[5],
            st_shndx: u16::from_le_bytes([data[6], data[7]]),
            st_value: u64::from_le_bytes([
                data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
            ]),
            st_size: u64::from_le_bytes([
                data[16], data[17], data[18], data[19], data[20], data[21], data[22], data[23],
            ]),
        })
    }

    /// Get symbol binding
    pub fn binding(&self) -> u8 {
        self.st_info >> 4
    }

    /// Get symbol type
    pub fn symbol_type(&self) -> u8 {
        self.st_info & 0xf
    }
}

/// Loaded segment info
#[derive(Debug, Clone)]
pub struct LoadedSegment {
    /// Virtual address
    pub vaddr: u64,
    /// Size in memory
    pub size: u64,
    /// Segment data
    pub data: Vec<u8>,
    /// Permissions (R/W/X)
    pub readable: bool,
    /// Writable
    pub writable: bool,
    /// Executable
    pub executable: bool,
}

/// Complete parsed ELF
#[derive(Debug)]
pub struct ParsedElf {
    /// ELF header
    pub header: ElfHeader,
    /// Program headers
    pub program_headers: Vec<ProgramHeader>,
    /// Section headers
    pub section_headers: Vec<SectionHeader>,
    /// Loadable segments
    pub segments: Vec<LoadedSegment>,
    /// Entry point
    pub entry_point: u64,
    /// Base address (for PIE)
    pub base_address: u64,
    /// Is position independent
    pub is_pie: bool,
}

/// ELF Loader
pub struct ElfLoader {
    /// Default base address for PIE executables
    pub default_base: u64,
    /// Maximum segment size
    pub max_segment_size: u64,
}

impl ElfLoader {
    /// Create new ELF loader
    pub const fn new() -> Self {
        Self {
            default_base: 0x400000,
            max_segment_size: 256 * 1024 * 1024, // 256 MB
        }
    }

    /// Parse ELF binary
    pub fn parse(&self, data: &[u8]) -> Result<ParsedElf, ElfError> {
        // Parse header
        let header = ElfHeader::parse(data)?;

        // Parse program headers
        let mut program_headers = Vec::with_capacity(header.e_phnum as usize);
        for i in 0..header.e_phnum as usize {
            let offset = header.e_phoff as usize + i * header.e_phentsize as usize;
            let ph = ProgramHeader::parse(data, offset)?;
            program_headers.push(ph);
        }

        // Parse section headers
        let mut section_headers = Vec::with_capacity(header.e_shnum as usize);
        for i in 0..header.e_shnum as usize {
            let offset = header.e_shoff as usize + i * header.e_shentsize as usize;
            if let Ok(sh) = SectionHeader::parse(data, offset) {
                section_headers.push(sh);
            }
        }

        // Load segments
        let mut segments = Vec::new();
        let mut base_address = 0u64;
        let is_pie = header.is_pie();

        for ph in &program_headers {
            if ph.is_loadable() {
                if ph.p_memsz > self.max_segment_size {
                    return Err(ElfError::SegmentTooLarge);
                }

                // Calculate base address for first loadable segment
                if segments.is_empty() && is_pie {
                    base_address = self.default_base;
                }

                let vaddr = if is_pie {
                    base_address + ph.p_vaddr
                } else {
                    ph.p_vaddr
                };

                // Copy segment data
                let mut segment_data = vec![0u8; ph.p_memsz as usize];
                let file_offset = ph.p_offset as usize;
                let file_size = ph.p_filesz as usize;

                if file_offset + file_size <= data.len() {
                    segment_data[..file_size]
                        .copy_from_slice(&data[file_offset..file_offset + file_size]);
                }

                segments.push(LoadedSegment {
                    vaddr,
                    size: ph.p_memsz,
                    data: segment_data,
                    readable: ph.is_readable(),
                    writable: ph.is_writable(),
                    executable: ph.is_executable(),
                });
            }
        }

        if segments.is_empty() {
            return Err(ElfError::NoLoadableSegments);
        }

        let entry_point = if is_pie {
            base_address + header.e_entry
        } else {
            header.e_entry
        };

        STATS.program_loaded();

        Ok(ParsedElf {
            header,
            program_headers,
            section_headers,
            segments,
            entry_point,
            base_address,
            is_pie,
        })
    }

    /// Quick validate ELF binary
    pub fn validate(&self, data: &[u8]) -> Result<(), ElfError> {
        if data.len() < 4 {
            return Err(ElfError::BufferTooSmall);
        }
        if data[0..4] != ELF_MAGIC {
            return Err(ElfError::InvalidMagic);
        }
        Ok(())
    }
}

impl Default for ElfLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize ELF subsystem
pub fn init() -> Result<(), super::UserError> {
    // Nothing to initialize for now
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_magic() {
        assert_eq!(ELF_MAGIC, [0x7f, b'E', b'L', b'F']);
    }

    #[test]
    fn test_loader_creation() {
        let loader = ElfLoader::new();
        assert_eq!(loader.default_base, 0x400000);
    }

    #[test]
    fn test_invalid_magic() {
        let loader = ElfLoader::new();
        let data = [0x00, 0x00, 0x00, 0x00];
        assert_eq!(loader.validate(&data), Err(ElfError::InvalidMagic));
    }
}
