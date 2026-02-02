//! ELF Parser
//!
//! Comprehensive ELF64 parser for kernel loading.

extern crate alloc;
use alloc::format;
use core::fmt;

// =============================================================================
// ELF MAGIC AND CONSTANTS
// =============================================================================

/// ELF magic bytes
pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

/// ELF class (32 or 64 bit)
pub mod class {
    pub const NONE: u8 = 0;
    pub const ELF32: u8 = 1;
    pub const ELF64: u8 = 2;
}

/// ELF data encoding
pub mod encoding {
    pub const NONE: u8 = 0;
    pub const LSB: u8 = 1; // Little endian
    pub const MSB: u8 = 2; // Big endian
}

/// ELF OS/ABI
pub mod osabi {
    pub const SYSV: u8 = 0;
    pub const HPUX: u8 = 1;
    pub const NETBSD: u8 = 2;
    pub const LINUX: u8 = 3;
    pub const SOLARIS: u8 = 6;
    pub const AIX: u8 = 7;
    pub const IRIX: u8 = 8;
    pub const FREEBSD: u8 = 9;
    pub const TRU64: u8 = 10;
    pub const MODESTO: u8 = 11;
    pub const OPENBSD: u8 = 12;
    pub const STANDALONE: u8 = 255;
}

/// ELF type
pub mod elf_type {
    pub const NONE: u16 = 0;
    pub const REL: u16 = 1; // Relocatable
    pub const EXEC: u16 = 2; // Executable
    pub const DYN: u16 = 3; // Shared object
    pub const CORE: u16 = 4; // Core dump
}

/// Machine types
pub mod machine {
    pub const NONE: u16 = 0;
    pub const X86: u16 = 3;
    pub const ARM: u16 = 40;
    pub const X86_64: u16 = 62;
    pub const AARCH64: u16 = 183;
    pub const RISCV: u16 = 243;
}

/// Segment types
pub mod segment_type {
    pub const NULL: u32 = 0;
    pub const LOAD: u32 = 1;
    pub const DYNAMIC: u32 = 2;
    pub const INTERP: u32 = 3;
    pub const NOTE: u32 = 4;
    pub const SHLIB: u32 = 5;
    pub const PHDR: u32 = 6;
    pub const TLS: u32 = 7;
    pub const GNU_EH_FRAME: u32 = 0x6474e550;
    pub const GNU_STACK: u32 = 0x6474e551;
    pub const GNU_RELRO: u32 = 0x6474e552;
}

/// Segment flags
pub mod segment_flags {
    pub const X: u32 = 0x1; // Executable
    pub const W: u32 = 0x2; // Writable
    pub const R: u32 = 0x4; // Readable
}

/// Section types
pub mod section_type {
    pub const NULL: u32 = 0;
    pub const PROGBITS: u32 = 1;
    pub const SYMTAB: u32 = 2;
    pub const STRTAB: u32 = 3;
    pub const RELA: u32 = 4;
    pub const HASH: u32 = 5;
    pub const DYNAMIC: u32 = 6;
    pub const NOTE: u32 = 7;
    pub const NOBITS: u32 = 8;
    pub const REL: u32 = 9;
    pub const SHLIB: u32 = 10;
    pub const DYNSYM: u32 = 11;
    pub const INIT_ARRAY: u32 = 14;
    pub const FINI_ARRAY: u32 = 15;
    pub const PREINIT_ARRAY: u32 = 16;
    pub const GROUP: u32 = 17;
    pub const SYMTAB_SHNDX: u32 = 18;
}

/// Section flags
pub mod section_flags {
    pub const WRITE: u64 = 0x1;
    pub const ALLOC: u64 = 0x2;
    pub const EXECINSTR: u64 = 0x4;
    pub const MERGE: u64 = 0x10;
    pub const STRINGS: u64 = 0x20;
    pub const INFO_LINK: u64 = 0x40;
    pub const LINK_ORDER: u64 = 0x80;
    pub const OS_NONCONFORMING: u64 = 0x100;
    pub const GROUP: u64 = 0x200;
    pub const TLS: u64 = 0x400;
}

/// Symbol types
pub mod symbol_type {
    pub const NOTYPE: u8 = 0;
    pub const OBJECT: u8 = 1;
    pub const FUNC: u8 = 2;
    pub const SECTION: u8 = 3;
    pub const FILE: u8 = 4;
    pub const COMMON: u8 = 5;
    pub const TLS: u8 = 6;
}

/// Symbol bindings
pub mod symbol_binding {
    pub const LOCAL: u8 = 0;
    pub const GLOBAL: u8 = 1;
    pub const WEAK: u8 = 2;
}

/// Symbol visibility
pub mod symbol_visibility {
    pub const DEFAULT: u8 = 0;
    pub const INTERNAL: u8 = 1;
    pub const HIDDEN: u8 = 2;
    pub const PROTECTED: u8 = 3;
}

// =============================================================================
// ELF HEADER
// =============================================================================

/// ELF64 header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Elf64Header {
    /// Magic and identification
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
    /// Processor flags
    pub e_flags: u32,
    /// ELF header size
    pub e_ehsize: u16,
    /// Program header entry size
    pub e_phentsize: u16,
    /// Program header count
    pub e_phnum: u16,
    /// Section header entry size
    pub e_shentsize: u16,
    /// Section header count
    pub e_shnum: u16,
    /// Section name string table index
    pub e_shstrndx: u16,
}

impl Elf64Header {
    /// Size of header
    pub const SIZE: usize = 64;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        // Verify magic
        if &bytes[0..4] != &ELF_MAGIC {
            return None;
        }

        // Must be 64-bit
        if bytes[4] != class::ELF64 {
            return None;
        }

        Some(Self {
            e_ident: bytes[0..16].try_into().ok()?,
            e_type: u16::from_le_bytes([bytes[16], bytes[17]]),
            e_machine: u16::from_le_bytes([bytes[18], bytes[19]]),
            e_version: u32::from_le_bytes(bytes[20..24].try_into().ok()?),
            e_entry: u64::from_le_bytes(bytes[24..32].try_into().ok()?),
            e_phoff: u64::from_le_bytes(bytes[32..40].try_into().ok()?),
            e_shoff: u64::from_le_bytes(bytes[40..48].try_into().ok()?),
            e_flags: u32::from_le_bytes(bytes[48..52].try_into().ok()?),
            e_ehsize: u16::from_le_bytes([bytes[52], bytes[53]]),
            e_phentsize: u16::from_le_bytes([bytes[54], bytes[55]]),
            e_phnum: u16::from_le_bytes([bytes[56], bytes[57]]),
            e_shentsize: u16::from_le_bytes([bytes[58], bytes[59]]),
            e_shnum: u16::from_le_bytes([bytes[60], bytes[61]]),
            e_shstrndx: u16::from_le_bytes([bytes[62], bytes[63]]),
        })
    }

    /// Is little endian
    pub fn is_little_endian(&self) -> bool {
        self.e_ident[5] == encoding::LSB
    }

    /// Get ELF class
    pub fn class(&self) -> u8 {
        self.e_ident[4]
    }

    /// Get OS/ABI
    pub fn osabi(&self) -> u8 {
        self.e_ident[7]
    }

    /// Is executable
    pub fn is_executable(&self) -> bool {
        self.e_type == elf_type::EXEC
    }

    /// Is shared object
    pub fn is_shared_object(&self) -> bool {
        self.e_type == elf_type::DYN
    }

    /// Is relocatable
    pub fn is_relocatable(&self) -> bool {
        self.e_type == elf_type::REL
    }

    /// Is valid
    pub fn is_valid(&self) -> bool {
        self.e_ident[0..4] == ELF_MAGIC && self.e_ident[4] == class::ELF64 && self.e_version == 1
    }

    /// Is for x86_64
    pub fn is_x86_64(&self) -> bool {
        self.e_machine == machine::X86_64
    }

    /// Is for aarch64
    pub fn is_aarch64(&self) -> bool {
        self.e_machine == machine::AARCH64
    }
}

impl fmt::Debug for Elf64Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let e_type = self.e_type;
        let e_machine = self.e_machine;
        let e_entry = self.e_entry;
        let e_phnum = self.e_phnum;
        let e_shnum = self.e_shnum;
        f.debug_struct("Elf64Header")
            .field("type", &e_type)
            .field("machine", &e_machine)
            .field("entry", &format_args!("0x{:x}", e_entry))
            .field("phnum", &e_phnum)
            .field("shnum", &e_shnum)
            .finish()
    }
}

// =============================================================================
// PROGRAM HEADER
// =============================================================================

/// ELF64 program header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Elf64ProgramHeader {
    /// Segment type
    pub p_type: u32,
    /// Segment flags
    pub p_flags: u32,
    /// File offset
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

impl Elf64ProgramHeader {
    /// Size of program header
    pub const SIZE: usize = 56;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            p_type: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            p_flags: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            p_offset: u64::from_le_bytes(bytes[8..16].try_into().ok()?),
            p_vaddr: u64::from_le_bytes(bytes[16..24].try_into().ok()?),
            p_paddr: u64::from_le_bytes(bytes[24..32].try_into().ok()?),
            p_filesz: u64::from_le_bytes(bytes[32..40].try_into().ok()?),
            p_memsz: u64::from_le_bytes(bytes[40..48].try_into().ok()?),
            p_align: u64::from_le_bytes(bytes[48..56].try_into().ok()?),
        })
    }

    /// Is loadable segment
    pub fn is_load(&self) -> bool {
        self.p_type == segment_type::LOAD
    }

    /// Is executable
    pub fn is_executable(&self) -> bool {
        self.p_flags & segment_flags::X != 0
    }

    /// Is writable
    pub fn is_writable(&self) -> bool {
        self.p_flags & segment_flags::W != 0
    }

    /// Is readable
    pub fn is_readable(&self) -> bool {
        self.p_flags & segment_flags::R != 0
    }

    /// End virtual address
    pub fn end_vaddr(&self) -> u64 {
        self.p_vaddr + self.p_memsz
    }
}

impl fmt::Debug for Elf64ProgramHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let p_type = self.p_type;
        let p_vaddr = self.p_vaddr;
        let p_memsz = self.p_memsz;
        let p_filesz = self.p_filesz;
        let p_flags = self.p_flags;
        let flags = format!(
            "{}{}{}",
            if p_flags & 4 != 0 { "R" } else { "-" },
            if p_flags & 2 != 0 { "W" } else { "-" },
            if p_flags & 1 != 0 { "X" } else { "-" },
        );

        f.debug_struct("ProgramHeader")
            .field("type", &p_type)
            .field("flags", &flags)
            .field("vaddr", &format_args!("0x{:x}", p_vaddr))
            .field("memsz", &format_args!("0x{:x}", p_memsz))
            .field("filesz", &format_args!("0x{:x}", p_filesz))
            .finish()
    }
}

// =============================================================================
// SECTION HEADER
// =============================================================================

/// ELF64 section header
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Elf64SectionHeader {
    /// Section name (string table offset)
    pub sh_name: u32,
    /// Section type
    pub sh_type: u32,
    /// Section flags
    pub sh_flags: u64,
    /// Virtual address
    pub sh_addr: u64,
    /// File offset
    pub sh_offset: u64,
    /// Section size
    pub sh_size: u64,
    /// Link to another section
    pub sh_link: u32,
    /// Additional info
    pub sh_info: u32,
    /// Address alignment
    pub sh_addralign: u64,
    /// Entry size (if table)
    pub sh_entsize: u64,
}

impl Elf64SectionHeader {
    /// Size of section header
    pub const SIZE: usize = 64;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            sh_name: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            sh_type: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
            sh_flags: u64::from_le_bytes(bytes[8..16].try_into().ok()?),
            sh_addr: u64::from_le_bytes(bytes[16..24].try_into().ok()?),
            sh_offset: u64::from_le_bytes(bytes[24..32].try_into().ok()?),
            sh_size: u64::from_le_bytes(bytes[32..40].try_into().ok()?),
            sh_link: u32::from_le_bytes(bytes[40..44].try_into().ok()?),
            sh_info: u32::from_le_bytes(bytes[44..48].try_into().ok()?),
            sh_addralign: u64::from_le_bytes(bytes[48..56].try_into().ok()?),
            sh_entsize: u64::from_le_bytes(bytes[56..64].try_into().ok()?),
        })
    }

    /// Is progbits
    pub fn is_progbits(&self) -> bool {
        self.sh_type == section_type::PROGBITS
    }

    /// Is nobits (BSS)
    pub fn is_nobits(&self) -> bool {
        self.sh_type == section_type::NOBITS
    }

    /// Is allocatable
    pub fn is_alloc(&self) -> bool {
        self.sh_flags & section_flags::ALLOC != 0
    }

    /// Is writable
    pub fn is_writable(&self) -> bool {
        self.sh_flags & section_flags::WRITE != 0
    }

    /// Is executable
    pub fn is_executable(&self) -> bool {
        self.sh_flags & section_flags::EXECINSTR != 0
    }
}

// =============================================================================
// SYMBOL TABLE
// =============================================================================

/// ELF64 symbol
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Elf64Symbol {
    /// Symbol name (string table offset)
    pub st_name: u32,
    /// Type and binding
    pub st_info: u8,
    /// Visibility
    pub st_other: u8,
    /// Section index
    pub st_shndx: u16,
    /// Symbol value
    pub st_value: u64,
    /// Symbol size
    pub st_size: u64,
}

impl Elf64Symbol {
    /// Size of symbol entry
    pub const SIZE: usize = 24;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            st_name: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
            st_info: bytes[4],
            st_other: bytes[5],
            st_shndx: u16::from_le_bytes([bytes[6], bytes[7]]),
            st_value: u64::from_le_bytes(bytes[8..16].try_into().ok()?),
            st_size: u64::from_le_bytes(bytes[16..24].try_into().ok()?),
        })
    }

    /// Get symbol type
    pub fn symbol_type(&self) -> u8 {
        self.st_info & 0x0F
    }

    /// Get symbol binding
    pub fn binding(&self) -> u8 {
        self.st_info >> 4
    }

    /// Get visibility
    pub fn visibility(&self) -> u8 {
        self.st_other & 0x03
    }

    /// Is function
    pub fn is_function(&self) -> bool {
        self.symbol_type() == symbol_type::FUNC
    }

    /// Is object/data
    pub fn is_object(&self) -> bool {
        self.symbol_type() == symbol_type::OBJECT
    }

    /// Is global
    pub fn is_global(&self) -> bool {
        self.binding() == symbol_binding::GLOBAL
    }

    /// Is weak
    pub fn is_weak(&self) -> bool {
        self.binding() == symbol_binding::WEAK
    }

    /// Is defined (not external)
    pub fn is_defined(&self) -> bool {
        self.st_shndx != 0
    }
}

// =============================================================================
// RELOCATION
// =============================================================================

/// ELF64 relocation
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Elf64Rel {
    /// Relocation offset
    pub r_offset: u64,
    /// Relocation info (symbol + type)
    pub r_info: u64,
}

impl Elf64Rel {
    /// Size
    pub const SIZE: usize = 16;

    /// Get symbol index
    pub fn symbol(&self) -> u32 {
        (self.r_info >> 32) as u32
    }

    /// Get relocation type
    pub fn relocation_type(&self) -> u32 {
        (self.r_info & 0xFFFFFFFF) as u32
    }
}

/// ELF64 relocation with addend
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Elf64Rela {
    /// Relocation offset
    pub r_offset: u64,
    /// Relocation info
    pub r_info: u64,
    /// Addend
    pub r_addend: i64,
}

impl Elf64Rela {
    /// Size
    pub const SIZE: usize = 24;

    /// Parse from bytes
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            r_offset: u64::from_le_bytes(bytes[0..8].try_into().ok()?),
            r_info: u64::from_le_bytes(bytes[8..16].try_into().ok()?),
            r_addend: i64::from_le_bytes(bytes[16..24].try_into().ok()?),
        })
    }

    /// Get symbol index
    pub fn symbol(&self) -> u32 {
        (self.r_info >> 32) as u32
    }

    /// Get relocation type
    pub fn relocation_type(&self) -> u32 {
        (self.r_info & 0xFFFFFFFF) as u32
    }
}

/// x86_64 relocation types
pub mod reloc_x86_64 {
    pub const R_NONE: u32 = 0;
    pub const R_64: u32 = 1;
    pub const R_PC32: u32 = 2;
    pub const R_GOT32: u32 = 3;
    pub const R_PLT32: u32 = 4;
    pub const R_COPY: u32 = 5;
    pub const R_GLOB_DAT: u32 = 6;
    pub const R_JUMP_SLOT: u32 = 7;
    pub const R_RELATIVE: u32 = 8;
    pub const R_GOTPCREL: u32 = 9;
    pub const R_32: u32 = 10;
    pub const R_32S: u32 = 11;
    pub const R_16: u32 = 12;
    pub const R_PC16: u32 = 13;
    pub const R_8: u32 = 14;
    pub const R_PC8: u32 = 15;
    pub const R_PC64: u32 = 24;
    pub const R_GOTOFF64: u32 = 25;
    pub const R_GOTPC32: u32 = 26;
}

// =============================================================================
// ELF FILE
// =============================================================================

/// Parsed ELF file
pub struct ElfFile<'a> {
    /// Raw data
    data: &'a [u8],
    /// Header
    header: Elf64Header,
}

impl<'a> ElfFile<'a> {
    /// Parse ELF file from bytes
    pub fn parse(data: &'a [u8]) -> Result<Self, ElfError> {
        let header = Elf64Header::from_bytes(data).ok_or(ElfError::InvalidHeader)?;

        if !header.is_valid() {
            return Err(ElfError::InvalidHeader);
        }

        Ok(Self { data, header })
    }

    /// Get header
    pub fn header(&self) -> &Elf64Header {
        &self.header
    }

    /// Get entry point
    pub fn entry_point(&self) -> u64 {
        self.header.e_entry
    }

    /// Get program header count
    pub fn program_header_count(&self) -> usize {
        self.header.e_phnum as usize
    }

    /// Get program header
    pub fn program_header(&self, index: usize) -> Option<Elf64ProgramHeader> {
        if index >= self.program_header_count() {
            return None;
        }

        let offset = self.header.e_phoff as usize + index * self.header.e_phentsize as usize;

        if offset + Elf64ProgramHeader::SIZE > self.data.len() {
            return None;
        }

        Elf64ProgramHeader::from_bytes(&self.data[offset..])
    }

    /// Iterate program headers
    pub fn program_headers(&self) -> ProgramHeaderIter<'_> {
        ProgramHeaderIter {
            elf: self,
            index: 0,
        }
    }

    /// Get section header count
    pub fn section_header_count(&self) -> usize {
        self.header.e_shnum as usize
    }

    /// Get section header
    pub fn section_header(&self, index: usize) -> Option<Elf64SectionHeader> {
        if index >= self.section_header_count() {
            return None;
        }

        let offset = self.header.e_shoff as usize + index * self.header.e_shentsize as usize;

        if offset + Elf64SectionHeader::SIZE > self.data.len() {
            return None;
        }

        Elf64SectionHeader::from_bytes(&self.data[offset..])
    }

    /// Iterate section headers
    pub fn section_headers(&self) -> SectionHeaderIter<'_> {
        SectionHeaderIter {
            elf: self,
            index: 0,
        }
    }

    /// Get section name
    pub fn section_name(&self, section: &Elf64SectionHeader) -> Option<&str> {
        let shstrtab = self.section_header(self.header.e_shstrndx as usize)?;
        self.string_at(shstrtab.sh_offset as usize, section.sh_name as usize)
    }

    /// Get section data
    pub fn section_data(&self, section: &Elf64SectionHeader) -> Option<&[u8]> {
        let start = section.sh_offset as usize;
        let end = start + section.sh_size as usize;

        if end > self.data.len() {
            return None;
        }

        Some(&self.data[start..end])
    }

    /// Find section by name
    pub fn find_section(&self, name: &str) -> Option<Elf64SectionHeader> {
        for section in self.section_headers() {
            if let Some(section_name) = self.section_name(&section) {
                if section_name == name {
                    return Some(section);
                }
            }
        }
        None
    }

    /// Get segment data
    pub fn segment_data(&self, phdr: &Elf64ProgramHeader) -> Option<&[u8]> {
        let start = phdr.p_offset as usize;
        let end = start + phdr.p_filesz as usize;

        if end > self.data.len() {
            return None;
        }

        Some(&self.data[start..end])
    }

    /// Get loadable segments
    pub fn loadable_segments(&self) -> impl Iterator<Item = Elf64ProgramHeader> + '_ {
        self.program_headers().filter(|ph| ph.is_load())
    }

    /// Calculate total memory needed
    pub fn memory_requirements(&self) -> MemoryRequirements {
        let mut lowest_vaddr = u64::MAX;
        let mut highest_vaddr = 0u64;

        for phdr in self.loadable_segments() {
            if phdr.p_vaddr < lowest_vaddr {
                lowest_vaddr = phdr.p_vaddr;
            }
            if phdr.end_vaddr() > highest_vaddr {
                highest_vaddr = phdr.end_vaddr();
            }
        }

        if lowest_vaddr == u64::MAX {
            lowest_vaddr = 0;
        }

        MemoryRequirements {
            base_address: lowest_vaddr,
            size: highest_vaddr.saturating_sub(lowest_vaddr) as usize,
            alignment: 0x1000, // Page alignment
        }
    }

    /// Get string from string table
    fn string_at(&self, table_offset: usize, str_offset: usize) -> Option<&str> {
        let start = table_offset + str_offset;

        if start >= self.data.len() {
            return None;
        }

        // Find null terminator
        let mut end = start;
        while end < self.data.len() && self.data[end] != 0 {
            end += 1;
        }

        core::str::from_utf8(&self.data[start..end]).ok()
    }

    /// Get symbols from symtab section
    pub fn symbols(&self) -> Option<SymbolIter<'_>> {
        let symtab = self.find_section(".symtab")?;
        let strtab = self.find_section(".strtab")?;

        let symtab_data = self.section_data(&symtab)?;

        Some(SymbolIter {
            data: symtab_data,
            strtab_offset: strtab.sh_offset as usize,
            elf_data: self.data,
            index: 0,
            count: symtab.sh_size as usize / Elf64Symbol::SIZE,
        })
    }

    /// Find symbol by name
    pub fn find_symbol(&self, name: &str) -> Option<Elf64Symbol> {
        for (sym, sym_name) in self.symbols()? {
            if sym_name == name {
                return Some(sym);
            }
        }
        None
    }
}

/// Program header iterator
pub struct ProgramHeaderIter<'a> {
    elf: &'a ElfFile<'a>,
    index: usize,
}

impl<'a> Iterator for ProgramHeaderIter<'a> {
    type Item = Elf64ProgramHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.elf.program_header_count() {
            return None;
        }

        let phdr = self.elf.program_header(self.index);
        self.index += 1;
        phdr
    }
}

/// Section header iterator
pub struct SectionHeaderIter<'a> {
    elf: &'a ElfFile<'a>,
    index: usize,
}

impl<'a> Iterator for SectionHeaderIter<'a> {
    type Item = Elf64SectionHeader;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.elf.section_header_count() {
            return None;
        }

        let shdr = self.elf.section_header(self.index);
        self.index += 1;
        shdr
    }
}

/// Symbol iterator
pub struct SymbolIter<'a> {
    data: &'a [u8],
    strtab_offset: usize,
    elf_data: &'a [u8],
    index: usize,
    count: usize,
}

impl<'a> Iterator for SymbolIter<'a> {
    type Item = (Elf64Symbol, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.count {
            return None;
        }

        let offset = self.index * Elf64Symbol::SIZE;
        let symbol = Elf64Symbol::from_bytes(&self.data[offset..])?;

        // Get symbol name
        let name_start = self.strtab_offset + symbol.st_name as usize;
        let mut name_end = name_start;

        while name_end < self.elf_data.len() && self.elf_data[name_end] != 0 {
            name_end += 1;
        }

        let name = core::str::from_utf8(&self.elf_data[name_start..name_end]).unwrap_or("");

        self.index += 1;
        Some((symbol, name))
    }
}

/// Memory requirements for loading
#[derive(Debug, Clone, Copy)]
pub struct MemoryRequirements {
    /// Base virtual address
    pub base_address: u64,
    /// Total size needed
    pub size: usize,
    /// Required alignment
    pub alignment: usize,
}

// =============================================================================
// ELF LOADER
// =============================================================================

/// ELF loader
pub struct ElfLoader;

impl ElfLoader {
    /// Load ELF into memory
    pub fn load(elf: &ElfFile, memory: &mut [u8], base: u64) -> Result<LoadedElf, ElfError> {
        let reqs = elf.memory_requirements();

        if memory.len() < reqs.size {
            return Err(ElfError::BufferTooSmall);
        }

        // Clear memory
        for byte in memory.iter_mut() {
            *byte = 0;
        }

        // Load each segment
        for phdr in elf.loadable_segments() {
            let segment_offset = (phdr.p_vaddr - reqs.base_address) as usize;

            // Copy file data
            if phdr.p_filesz > 0 {
                let src = elf.segment_data(&phdr).ok_or(ElfError::InvalidSegment)?;

                let dst_end = segment_offset + src.len();
                if dst_end > memory.len() {
                    return Err(ElfError::BufferTooSmall);
                }

                memory[segment_offset..dst_end].copy_from_slice(src);
            }

            // BSS is already zeroed
        }

        Ok(LoadedElf {
            entry_point: elf.entry_point(),
            base_address: base,
            size: reqs.size,
        })
    }

    /// Relocate PIE/PIC executable
    pub fn relocate(
        elf: &ElfFile,
        memory: &mut [u8],
        load_base: u64,
        link_base: u64,
    ) -> Result<(), ElfError> {
        // Find .rela.dyn section
        if let Some(rela_section) = elf.find_section(".rela.dyn") {
            let rela_data = elf
                .section_data(&rela_section)
                .ok_or(ElfError::InvalidSection)?;

            let delta = load_base.wrapping_sub(link_base) as i64;

            for i in (0..rela_data.len()).step_by(Elf64Rela::SIZE) {
                if let Some(rela) = Elf64Rela::from_bytes(&rela_data[i..]) {
                    let rtype = rela.relocation_type();

                    // Handle R_X86_64_RELATIVE
                    if rtype == reloc_x86_64::R_RELATIVE {
                        let offset = (rela.r_offset - link_base) as usize;

                        if offset + 8 <= memory.len() {
                            let value = (rela.r_addend + delta) as u64;
                            memory[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

/// Loaded ELF info
#[derive(Debug, Clone)]
pub struct LoadedElf {
    /// Entry point address
    pub entry_point: u64,
    /// Base load address
    pub base_address: u64,
    /// Total size
    pub size: usize,
}

// =============================================================================
// ELF ERROR
// =============================================================================

/// ELF error
#[derive(Debug, Clone)]
pub enum ElfError {
    /// Invalid header
    InvalidHeader,
    /// Invalid segment
    InvalidSegment,
    /// Invalid section
    InvalidSection,
    /// Buffer too small
    BufferTooSmall,
    /// Unsupported architecture
    UnsupportedArch,
    /// Relocation error
    RelocationError,
}

impl fmt::Display for ElfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidHeader => write!(f, "invalid ELF header"),
            Self::InvalidSegment => write!(f, "invalid segment"),
            Self::InvalidSection => write!(f, "invalid section"),
            Self::BufferTooSmall => write!(f, "buffer too small"),
            Self::UnsupportedArch => write!(f, "unsupported architecture"),
            Self::RelocationError => write!(f, "relocation error"),
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
    fn test_elf_magic() {
        assert_eq!(ELF_MAGIC, [0x7F, b'E', b'L', b'F']);
    }

    #[test]
    fn test_program_header_flags() {
        let mut phdr = Elf64ProgramHeader {
            p_type: segment_type::LOAD,
            p_flags: segment_flags::R | segment_flags::X,
            p_offset: 0,
            p_vaddr: 0x1000,
            p_paddr: 0x1000,
            p_filesz: 0x100,
            p_memsz: 0x100,
            p_align: 0x1000,
        };

        assert!(phdr.is_load());
        assert!(phdr.is_readable());
        assert!(phdr.is_executable());
        assert!(!phdr.is_writable());
    }
}
