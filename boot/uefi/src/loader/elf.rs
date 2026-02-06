//! ELF Loader
//!
//! Complete ELF64 executable loader with full support for all standard
//! features including program headers, sections, relocations, and symbols.

use crate::error::{Error, Result};
use crate::loader::{
    ImageFlags, ImageFormat, ImageSection, LoadedImage, MachineType, SectionFlags,
};
use crate::raw::types::VirtualAddress;

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

// =============================================================================
// ELF CONSTANTS
// =============================================================================

/// ELF magic number
pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

/// ELF classes.
pub mod class {
    /// 32-bit ELF class.
    pub const ELFCLASS32: u8 = 1;
    /// 64-bit ELF class.
    pub const ELFCLASS64: u8 = 2;
}

/// ELF data encoding.
pub mod data {
    /// Little endian data encoding.
    pub const ELFDATA2LSB: u8 = 1;
    /// Big endian data encoding.
    pub const ELFDATA2MSB: u8 = 2;
}

/// ELF OS/ABI identifiers.
pub mod osabi {
    /// No specific OS/ABI.
    pub const ELFOSABI_NONE: u8 = 0;
    /// Linux OS/ABI.
    pub const ELFOSABI_LINUX: u8 = 3;
    /// FreeBSD OS/ABI.
    pub const ELFOSABI_FREEBSD: u8 = 9;
    /// Standalone/embedded OS/ABI.
    pub const ELFOSABI_STANDALONE: u8 = 255;
}

/// ELF object file types.
pub mod elf_type {
    /// No file type.
    pub const ET_NONE: u16 = 0;
    /// Relocatable file.
    pub const ET_REL: u16 = 1;
    /// Executable file.
    pub const ET_EXEC: u16 = 2;
    /// Shared object file.
    pub const ET_DYN: u16 = 3;
    /// Core file.
    pub const ET_CORE: u16 = 4;
}

/// ELF machine types.
pub mod machine {
    /// No machine type.
    pub const EM_NONE: u16 = 0;
    /// Intel 80386.
    pub const EM_386: u16 = 3;
    /// ARM.
    pub const EM_ARM: u16 = 40;
    /// AMD x86-64 architecture.
    pub const EM_X86_64: u16 = 62;
    /// ARM 64-bit architecture (`AArch64`).
    pub const EM_AARCH64: u16 = 183;
    /// RISC-V.
    pub const EM_RISCV: u16 = 243;
}

/// Program header types.
pub mod pt {
    /// Null entry (unused).
    pub const PT_NULL: u32 = 0;
    /// Loadable segment.
    pub const PT_LOAD: u32 = 1;
    /// Dynamic linking information.
    pub const PT_DYNAMIC: u32 = 2;
    /// Interpreter path.
    pub const PT_INTERP: u32 = 3;
    /// Auxiliary information.
    pub const PT_NOTE: u32 = 4;
    /// Reserved.
    pub const PT_SHLIB: u32 = 5;
    /// Program header table.
    pub const PT_PHDR: u32 = 6;
    /// Thread-local storage.
    pub const PT_TLS: u32 = 7;
    /// GNU exception handling frame.
    pub const PT_GNU_EH_FRAME: u32 = 0x6474_e550;
    /// GNU stack permissions.
    pub const PT_GNU_STACK: u32 = 0x6474_e551;
    /// GNU read-only after relocation.
    pub const PT_GNU_RELRO: u32 = 0x6474_e552;
}

/// Program header flags.
pub mod pf {
    /// Executable permission.
    pub const PF_X: u32 = 1;
    /// Write permission.
    pub const PF_W: u32 = 2;
    /// Read permission.
    pub const PF_R: u32 = 4;
}

/// Section header types.
pub mod sht {
    /// Null entry (unused).
    pub const SHT_NULL: u32 = 0;
    /// Program-defined data.
    pub const SHT_PROGBITS: u32 = 1;
    /// Symbol table.
    pub const SHT_SYMTAB: u32 = 2;
    /// String table.
    pub const SHT_STRTAB: u32 = 3;
    /// Relocation entries with addends.
    pub const SHT_RELA: u32 = 4;
    /// Symbol hash table.
    pub const SHT_HASH: u32 = 5;
    /// Dynamic linking info.
    pub const SHT_DYNAMIC: u32 = 6;
    /// Notes.
    pub const SHT_NOTE: u32 = 7;
    /// BSS (uninitialized data).
    pub const SHT_NOBITS: u32 = 8;
    /// Relocation entries without addends.
    pub const SHT_REL: u32 = 9;
    /// Reserved.
    pub const SHT_SHLIB: u32 = 10;
    /// Dynamic symbol table.
    pub const SHT_DYNSYM: u32 = 11;
    /// Initialization function array.
    pub const SHT_INIT_ARRAY: u32 = 14;
    /// Finalization function array.
    pub const SHT_FINI_ARRAY: u32 = 15;
    /// Pre-initialization function array.
    pub const SHT_PREINIT_ARRAY: u32 = 16;
    /// Section group.
    pub const SHT_GROUP: u32 = 17;
    /// Extended section indices.
    pub const SHT_SYMTAB_SHNDX: u32 = 18;
}

/// Section header flags.
pub mod shf {
    /// Writable data.
    pub const SHF_WRITE: u64 = 1;
    /// Allocated in memory.
    pub const SHF_ALLOC: u64 = 2;
    /// Executable code.
    pub const SHF_EXECINSTR: u64 = 4;
    /// Mergeable section.
    pub const SHF_MERGE: u64 = 0x10;
    /// Contains null-terminated strings.
    pub const SHF_STRINGS: u64 = 0x20;
    /// `sh_info` contains section index.
    pub const SHF_INFO_LINK: u64 = 0x40;
    /// Preserve section ordering.
    pub const SHF_LINK_ORDER: u64 = 0x80;
    /// Thread-local storage.
    pub const SHF_TLS: u64 = 0x400;
}

/// Symbol binding types.
pub mod stb {
    /// Local symbol.
    pub const STB_LOCAL: u8 = 0;
    /// Global symbol.
    pub const STB_GLOBAL: u8 = 1;
    /// Weak symbol.
    pub const STB_WEAK: u8 = 2;
}

/// Symbol types.
pub mod stt {
    /// No type.
    pub const STT_NOTYPE: u8 = 0;
    /// Data object.
    pub const STT_OBJECT: u8 = 1;
    /// Function.
    pub const STT_FUNC: u8 = 2;
    /// Section.
    pub const STT_SECTION: u8 = 3;
    /// Source file.
    pub const STT_FILE: u8 = 4;
    /// Common block.
    pub const STT_COMMON: u8 = 5;
    /// Thread-local storage.
    pub const STT_TLS: u8 = 6;
}

/// Relocation types for `x86_64`.
pub mod r_x86_64 {
    /// No relocation.
    pub const R_X86_64_NONE: u32 = 0;
    /// Direct 64-bit.
    pub const R_X86_64_64: u32 = 1;
    /// PC-relative 32-bit.
    pub const R_X86_64_PC32: u32 = 2;
    /// 32-bit GOT entry.
    pub const R_X86_64_GOT32: u32 = 3;
    /// 32-bit PLT address.
    pub const R_X86_64_PLT32: u32 = 4;
    /// Copy symbol at runtime.
    pub const R_X86_64_COPY: u32 = 5;
    /// Create GOT entry.
    pub const R_X86_64_GLOB_DAT: u32 = 6;
    /// Create PLT entry.
    pub const R_X86_64_JUMP_SLOT: u32 = 7;
    /// Adjust by program base.
    pub const R_X86_64_RELATIVE: u32 = 8;
    /// 32-bit signed GOT-relative offset.
    pub const R_X86_64_GOTPCREL: u32 = 9;
    /// Direct 32-bit zero-extended.
    pub const R_X86_64_32: u32 = 10;
    /// Direct 32-bit sign-extended.
    pub const R_X86_64_32S: u32 = 11;
    /// Direct 16-bit.
    pub const R_X86_64_16: u32 = 12;
    /// 16-bit PC-relative.
    pub const R_X86_64_PC16: u32 = 13;
    /// Direct 8-bit.
    pub const R_X86_64_8: u32 = 14;
    /// 8-bit PC-relative.
    pub const R_X86_64_PC8: u32 = 15;
    /// ID of module containing symbol.
    pub const R_X86_64_DTPMOD64: u32 = 16;
    /// Offset in TLS block.
    pub const R_X86_64_DTPOFF64: u32 = 17;
    /// Offset in initial TLS block.
    pub const R_X86_64_TPOFF64: u32 = 18;
    /// 32-bit PC-relative offset to GD GOT entry.
    pub const R_X86_64_TLSGD: u32 = 19;
    /// 32-bit PC-relative offset to LD GOT entry.
    pub const R_X86_64_TLSLD: u32 = 20;
    /// Offset in TLS block (32-bit).
    pub const R_X86_64_DTPOFF32: u32 = 21;
    /// 32-bit PC-relative offset to IE GOT entry.
    pub const R_X86_64_GOTTPOFF: u32 = 22;
    /// Offset in initial TLS block (32-bit).
    pub const R_X86_64_TPOFF32: u32 = 23;
    /// 64-bit PC-relative.
    pub const R_X86_64_PC64: u32 = 24;
    /// 64-bit offset to GOT.
    pub const R_X86_64_GOTOFF64: u32 = 25;
    /// 32-bit signed PC-relative offset to GOT.
    pub const R_X86_64_GOTPC32: u32 = 26;
    /// 32-bit symbol size.
    pub const R_X86_64_SIZE32: u32 = 32;
    /// 64-bit symbol size.
    pub const R_X86_64_SIZE64: u32 = 33;
    /// 32-bit PC-relative offset to TLS descriptor in GOT.
    pub const R_X86_64_GOTPC32_TLSDESC: u32 = 34;
    /// Relaxable call through TLS descriptor.
    pub const R_X86_64_TLSDESC_CALL: u32 = 35;
    /// TLS descriptor.
    pub const R_X86_64_TLSDESC: u32 = 36;
    /// Indirect relative.
    pub const R_X86_64_IRELATIVE: u32 = 37;
    /// 64-bit adjust by program base.
    pub const R_X86_64_RELATIVE64: u32 = 38;
}

// =============================================================================
// ELF STRUCTURES
// =============================================================================

/// ELF64 header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    /// Magic number and info
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
    /// Parse from bytes
    pub fn parse(data: &[u8]) -> Result<Self> {
        if data.len() < core::mem::size_of::<Self>() {
            return Err(Error::InvalidData);
        }

        // Check magic
        if data[0..4] != ELF_MAGIC {
            return Err(Error::InvalidMagic);
        }

        // Check class (must be 64-bit)
        if data[4] != class::ELFCLASS64 {
            return Err(Error::UnsupportedFormat);
        }

        // Check endianness (only little-endian supported)
        if data[5] != data::ELFDATA2LSB {
            return Err(Error::UnsupportedFormat);
        }

        // SAFETY: We've validated the data length and magic bytes above.
        // Using read_unaligned because byte slice may not be aligned.
        Ok(unsafe { core::ptr::read_unaligned(data.as_ptr().cast::<Self>()) })
    }

    /// Validate header
    pub fn validate(&self) -> Result<()> {
        // Check magic
        if self.e_ident[0..4] != ELF_MAGIC {
            return Err(Error::InvalidMagic);
        }

        // Check class
        if self.e_ident[4] != class::ELFCLASS64 {
            return Err(Error::UnsupportedFormat);
        }

        // Check type
        if self.e_type != elf_type::ET_EXEC && self.e_type != elf_type::ET_DYN {
            return Err(Error::UnsupportedFormat);
        }

        // Check machine
        match self.e_machine {
            machine::EM_X86_64 | machine::EM_AARCH64 | machine::EM_RISCV => {},
            _ => return Err(Error::UnsupportedArchitecture),
        }

        Ok(())
    }

    /// Get machine type
    #[must_use]
    pub const fn machine_type(&self) -> MachineType {
        match self.e_machine {
            machine::EM_386 => MachineType::X86,
            machine::EM_X86_64 => MachineType::X86_64,
            machine::EM_ARM => MachineType::Arm,
            machine::EM_AARCH64 => MachineType::Aarch64,
            machine::EM_RISCV => MachineType::RiscV64,
            _ => MachineType::Unknown,
        }
    }

    /// Check if position independent
    #[must_use]
    pub const fn is_pie(&self) -> bool {
        self.e_type == elf_type::ET_DYN
    }
}

/// ELF64 program header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64ProgramHeader {
    /// Segment type
    pub p_type: u32,
    /// Segment flags
    pub p_flags: u32,
    /// Segment offset in file
    pub p_offset: u64,
    /// Virtual address
    pub p_vaddr: u64,
    /// Physical address
    pub p_paddr: u64,
    /// Segment size in file
    pub p_filesz: u64,
    /// Segment size in memory
    pub p_memsz: u64,
    /// Alignment
    pub p_align: u64,
}

impl Elf64ProgramHeader {
    /// Parse from bytes at offset
    pub fn parse(data: &[u8], offset: usize) -> Result<Self> {
        if data.len() < offset + core::mem::size_of::<Self>() {
            return Err(Error::InvalidData);
        }

        // SAFETY: We've validated the data length above.
        // Using read_unaligned because byte slice may not be aligned.
        Ok(unsafe { core::ptr::read_unaligned(data[offset..].as_ptr().cast::<Self>()) })
    }

    /// Check if loadable
    #[must_use]
    pub const fn is_load(&self) -> bool {
        self.p_type == pt::PT_LOAD
    }

    /// Check if executable
    #[must_use]
    pub const fn is_executable(&self) -> bool {
        (self.p_flags & pf::PF_X) != 0
    }

    /// Check if writable
    #[must_use]
    pub const fn is_writable(&self) -> bool {
        (self.p_flags & pf::PF_W) != 0
    }

    /// Check if readable
    #[must_use]
    pub const fn is_readable(&self) -> bool {
        (self.p_flags & pf::PF_R) != 0
    }

    /// Get BSS size (memsz - filesz)
    #[must_use]
    pub const fn bss_size(&self) -> u64 {
        if self.p_memsz > self.p_filesz {
            self.p_memsz - self.p_filesz
        } else {
            0
        }
    }
}

/// ELF64 section header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64SectionHeader {
    /// Section name (offset into string table)
    pub sh_name: u32,
    /// Section type
    pub sh_type: u32,
    /// Section flags
    pub sh_flags: u64,
    /// Virtual address
    pub sh_addr: u64,
    /// Offset in file
    pub sh_offset: u64,
    /// Section size
    pub sh_size: u64,
    /// Link to another section
    pub sh_link: u32,
    /// Additional info
    pub sh_info: u32,
    /// Alignment
    pub sh_addralign: u64,
    /// Entry size (for tables)
    pub sh_entsize: u64,
}

impl Elf64SectionHeader {
    /// Parse from bytes at offset
    pub fn parse(data: &[u8], offset: usize) -> Result<Self> {
        if data.len() < offset + core::mem::size_of::<Self>() {
            return Err(Error::InvalidData);
        }

        // SAFETY: We've validated the data length above.
        // Using read_unaligned because byte slice may not be aligned.
        Ok(unsafe { core::ptr::read_unaligned(data[offset..].as_ptr().cast::<Self>()) })
    }

    /// Check if allocated
    #[must_use]
    pub const fn is_allocated(&self) -> bool {
        (self.sh_flags & shf::SHF_ALLOC) != 0
    }

    /// Check if writable
    #[must_use]
    pub const fn is_writable(&self) -> bool {
        (self.sh_flags & shf::SHF_WRITE) != 0
    }

    /// Check if executable
    #[must_use]
    pub const fn is_executable(&self) -> bool {
        (self.sh_flags & shf::SHF_EXECINSTR) != 0
    }

    /// Check if BSS
    #[must_use]
    pub const fn is_bss(&self) -> bool {
        self.sh_type == sht::SHT_NOBITS
    }

    /// Convert to section flags
    #[must_use]
    pub const fn to_section_flags(&self) -> SectionFlags {
        SectionFlags {
            readable: true,
            writable: self.is_writable(),
            executable: self.is_executable(),
            allocated: self.is_allocated(),
            code: self.is_executable(),
            data: self.is_writable() && !self.is_bss(),
            bss: self.is_bss(),
        }
    }
}

/// ELF64 symbol
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Symbol {
    /// Symbol name (offset into string table)
    pub st_name: u32,
    /// Symbol info (type and binding)
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

impl Elf64Symbol {
    /// Get symbol binding
    #[must_use]
    pub const fn binding(&self) -> u8 {
        self.st_info >> 4
    }

    /// Get symbol type
    #[must_use]
    pub const fn symbol_type(&self) -> u8 {
        self.st_info & 0xF
    }

    /// Check if function
    #[must_use]
    pub const fn is_function(&self) -> bool {
        self.symbol_type() == stt::STT_FUNC
    }

    /// Check if object
    #[must_use]
    pub const fn is_object(&self) -> bool {
        self.symbol_type() == stt::STT_OBJECT
    }

    /// Check if global
    #[must_use]
    pub const fn is_global(&self) -> bool {
        self.binding() == stb::STB_GLOBAL
    }

    /// Check if local
    #[must_use]
    pub const fn is_local(&self) -> bool {
        self.binding() == stb::STB_LOCAL
    }

    /// Check if weak
    #[must_use]
    pub const fn is_weak(&self) -> bool {
        self.binding() == stb::STB_WEAK
    }
}

/// ELF64 relocation with addend
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Rela {
    /// Relocation offset
    pub r_offset: u64,
    /// Relocation info (type and symbol)
    pub r_info: u64,
    /// Addend
    pub r_addend: i64,
}

impl Elf64Rela {
    /// Get relocation type
    #[must_use]
    pub const fn reloc_type(&self) -> u32 {
        (self.r_info & 0xFFFF_FFFF) as u32
    }

    /// Get symbol index
    #[must_use]
    pub const fn symbol_index(&self) -> u32 {
        (self.r_info >> 32) as u32
    }
}

/// ELF64 relocation without addend
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Rel {
    /// Relocation offset
    pub r_offset: u64,
    /// Relocation info (type and symbol)
    pub r_info: u64,
}

impl Elf64Rel {
    /// Get relocation type
    #[must_use]
    pub const fn reloc_type(&self) -> u32 {
        (self.r_info & 0xFFFF_FFFF) as u32
    }

    /// Get symbol index
    #[must_use]
    pub const fn symbol_index(&self) -> u32 {
        (self.r_info >> 32) as u32
    }
}

/// ELF64 dynamic entry
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Dyn {
    /// Dynamic tag
    pub d_tag: i64,
    /// Value
    pub d_val: u64,
}

/// Dynamic section tags.
pub mod dt {
    /// End of dynamic section.
    pub const DT_NULL: i64 = 0;
    /// Name of needed library.
    pub const DT_NEEDED: i64 = 1;
    /// Size of PLT relocs.
    pub const DT_PLTRELSZ: i64 = 2;
    /// Processor-defined value (GOT/PLT).
    pub const DT_PLTGOT: i64 = 3;
    /// Address of symbol hash table.
    pub const DT_HASH: i64 = 4;
    /// Address of string table.
    pub const DT_STRTAB: i64 = 5;
    /// Address of symbol table.
    pub const DT_SYMTAB: i64 = 6;
    /// Address of RELA relocs.
    pub const DT_RELA: i64 = 7;
    /// Total size of RELA relocs.
    pub const DT_RELASZ: i64 = 8;
    /// Size of one RELA reloc.
    pub const DT_RELAENT: i64 = 9;
    /// Size of string table.
    pub const DT_STRSZ: i64 = 10;
    /// Size of one symbol table entry.
    pub const DT_SYMENT: i64 = 11;
    /// Address of init function.
    pub const DT_INIT: i64 = 12;
    /// Address of fini function.
    pub const DT_FINI: i64 = 13;
    /// Name of shared object.
    pub const DT_SONAME: i64 = 14;
    /// Library search path (deprecated).
    pub const DT_RPATH: i64 = 15;
    /// Start symbol search here.
    pub const DT_SYMBOLIC: i64 = 16;
    /// Address of REL relocs.
    pub const DT_REL: i64 = 17;
    /// Total size of REL relocs.
    pub const DT_RELSZ: i64 = 18;
    /// Size of one REL reloc.
    pub const DT_RELENT: i64 = 19;
    /// Type of reloc in PLT.
    pub const DT_PLTREL: i64 = 20;
    /// For debugging (unspecified).
    pub const DT_DEBUG: i64 = 21;
    /// Reloc might modify text segment.
    pub const DT_TEXTREL: i64 = 22;
    /// Address of PLT relocs.
    pub const DT_JMPREL: i64 = 23;
    /// Process relocations of object.
    pub const DT_BIND_NOW: i64 = 24;
    /// Array with addresses of init functions.
    pub const DT_INIT_ARRAY: i64 = 25;
    /// Array with addresses of fini functions.
    pub const DT_FINI_ARRAY: i64 = 26;
    /// Size in bytes of `DT_INIT_ARRAY`.
    pub const DT_INIT_ARRAYSZ: i64 = 27;
    /// Size in bytes of `DT_FINI_ARRAY`.
    pub const DT_FINI_ARRAYSZ: i64 = 28;
    /// Library search path.
    pub const DT_RUNPATH: i64 = 29;
    /// Flags for the object.
    pub const DT_FLAGS: i64 = 30;
    /// Array with addresses of pre-init functions.
    pub const DT_PREINIT_ARRAY: i64 = 32;
    /// Size in bytes of `DT_PREINIT_ARRAY`.
    pub const DT_PREINIT_ARRAYSZ: i64 = 33;
    /// GNU-style hash table.
    pub const DT_GNU_HASH: i64 = 0x6fff_fef5;
    /// Number of RELA relocs.
    pub const DT_RELACOUNT: i64 = 0x6fff_fff9;
    /// Number of REL relocs.
    pub const DT_RELCOUNT: i64 = 0x6fff_fffa;
    /// State flags (GNU extension).
    pub const DT_FLAGS_1: i64 = 0x6fff_fffb;
}

// =============================================================================
// ELF LOADER
// =============================================================================

/// ELF file loader
pub struct ElfLoader {
    /// Parsed header
    header: Option<Elf64Header>,
    /// Program headers
    program_headers: Vec<Elf64ProgramHeader>,
    /// Section headers
    section_headers: Vec<Elf64SectionHeader>,
    /// Symbols
    symbols: Vec<ElfSymbol>,
    /// Relocations
    relocations: Vec<ElfRelocation>,
    /// String table
    string_table: Vec<u8>,
    /// Section string table
    section_strings: Vec<u8>,
    /// Dynamic entries
    dynamic: Vec<Elf64Dyn>,
    /// Loaded image
    image: Option<LoadedImage>,
    /// Image data
    data: Vec<u8>,
}

impl ElfLoader {
    /// Create new ELF loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            header: None,
            program_headers: Vec::new(),
            section_headers: Vec::new(),
            symbols: Vec::new(),
            relocations: Vec::new(),
            string_table: Vec::new(),
            section_strings: Vec::new(),
            dynamic: Vec::new(),
            image: None,
            data: Vec::new(),
        }
    }

    /// Load ELF from buffer
    pub fn load(&mut self, data: &[u8]) -> Result<LoadedImage> {
        // Store data
        self.data = data.to_vec();

        // Parse header
        let header = Elf64Header::parse(data)?;
        header.validate()?;
        self.header = Some(header);

        // Parse program headers
        self.parse_program_headers(data, &header)?;

        // Parse section headers
        self.parse_section_headers(data, &header)?;

        // Load section string table
        self.load_section_strings(data, &header);

        // Parse symbols
        self.parse_symbols(data)?;

        // Parse relocations
        self.parse_relocations(data);

        // Parse dynamic section
        self.parse_dynamic(data);

        // Build loaded image
        let image = self.build_image()?;
        self.image = Some(image.clone());

        Ok(image)
    }

    /// Parse program headers
    fn parse_program_headers(&mut self, data: &[u8], header: &Elf64Header) -> Result<()> {
        self.program_headers.clear();

        let offset = usize::try_from(header.e_phoff).map_err(|_| Error::InvalidData)?;
        let size = usize::from(header.e_phentsize);
        let count = usize::from(header.e_phnum);

        for i in 0..count {
            let phdr = Elf64ProgramHeader::parse(data, offset + i * size)?;
            self.program_headers.push(phdr);
        }

        Ok(())
    }

    /// Parse section headers
    fn parse_section_headers(&mut self, data: &[u8], header: &Elf64Header) -> Result<()> {
        self.section_headers.clear();

        if header.e_shoff == 0 {
            return Ok(());
        }

        let offset = usize::try_from(header.e_shoff).map_err(|_| Error::InvalidData)?;
        let size = usize::from(header.e_shentsize);
        let count = usize::from(header.e_shnum);

        for i in 0..count {
            let shdr = Elf64SectionHeader::parse(data, offset + i * size)?;
            self.section_headers.push(shdr);
        }

        Ok(())
    }

    /// Load section string table
    fn load_section_strings(&mut self, data: &[u8], header: &Elf64Header) {
        let shstrndx = usize::from(header.e_shstrndx);
        if shstrndx == 0 || shstrndx >= self.section_headers.len() {
            return;
        }

        let shdr = &self.section_headers[shstrndx];
        let Some(start) = usize::try_from(shdr.sh_offset).ok() else {
            return;
        };
        let Some(size) = usize::try_from(shdr.sh_size).ok() else {
            return;
        };
        let end = start.saturating_add(size);

        if end <= data.len() {
            self.section_strings = data[start..end].to_vec();
        }
    }

    /// Get section name from string table
    fn section_name(&self, offset: u32) -> String {
        let start = offset as usize; // u32 to usize is always safe
        if start >= self.section_strings.len() {
            return String::new();
        }

        let end = self.section_strings[start..]
            .iter()
            .position(|&b| b == 0)
            .map_or(self.section_strings.len(), |p| start + p);

        String::from_utf8_lossy(&self.section_strings[start..end]).into_owned()
    }

    /// Parse symbols
    fn parse_symbols(&mut self, data: &[u8]) -> Result<()> {
        self.symbols.clear();

        // Find symbol table section
        let Some(symtab) = self
            .section_headers
            .iter()
            .find(|s| s.sh_type == sht::SHT_SYMTAB)
        else {
            return Ok(());
        };

        // Find associated string table
        let strtab_idx = symtab.sh_link as usize; // u32 to usize is always safe
        if strtab_idx >= self.section_headers.len() {
            return Ok(());
        }

        let strtab = &self.section_headers[strtab_idx];
        let strtab_start = usize::try_from(strtab.sh_offset).map_err(|_| Error::InvalidData)?;
        let strtab_size = usize::try_from(strtab.sh_size).map_err(|_| Error::InvalidData)?;
        let strtab_end = strtab_start.saturating_add(strtab_size);

        if strtab_end > data.len() {
            return Err(Error::InvalidData);
        }

        self.string_table = data[strtab_start..strtab_end].to_vec();

        // Parse symbols
        let start = usize::try_from(symtab.sh_offset).map_err(|_| Error::InvalidData)?;
        let entry_size = usize::try_from(symtab.sh_entsize).map_err(|_| Error::InvalidData)?;
        let symtab_size = usize::try_from(symtab.sh_size).map_err(|_| Error::InvalidData)?;
        let count = symtab_size / entry_size;

        for i in 0..count {
            let offset = start + i * entry_size;
            if offset + core::mem::size_of::<Elf64Symbol>() > data.len() {
                break;
            }

            // SAFETY: We've validated the data length above. Using read_unaligned
            // because byte slice may not be aligned.
            let sym: Elf64Symbol =
                unsafe { core::ptr::read_unaligned(data[offset..].as_ptr().cast::<Elf64Symbol>()) };

            let name = self.get_string(sym.st_name);

            self.symbols.push(ElfSymbol {
                name,
                value: sym.st_value,
                size: sym.st_size,
                info: sym.st_info,
                other: sym.st_other,
                section_index: sym.st_shndx,
            });
        }

        Ok(())
    }

    /// Get string from string table
    fn get_string(&self, offset: u32) -> String {
        let start = offset as usize; // u32 to usize is always safe
        if start >= self.string_table.len() {
            return String::new();
        }

        let end = self.string_table[start..]
            .iter()
            .position(|&b| b == 0)
            .map_or(self.string_table.len(), |p| start + p);

        String::from_utf8_lossy(&self.string_table[start..end]).into_owned()
    }

    /// Parse relocations
    fn parse_relocations(&mut self, data: &[u8]) {
        self.relocations.clear();

        for shdr in &self.section_headers {
            if shdr.sh_type != sht::SHT_RELA && shdr.sh_type != sht::SHT_REL {
                continue;
            }

            let Some(start) = usize::try_from(shdr.sh_offset).ok() else {
                continue;
            };
            let Some(entry_size) = usize::try_from(shdr.sh_entsize).ok().filter(|&s| s > 0) else {
                continue;
            };
            let Some(shdr_size) = usize::try_from(shdr.sh_size).ok() else {
                continue;
            };
            let count = shdr_size / entry_size;
            let with_addend = shdr.sh_type == sht::SHT_RELA;

            for i in 0..count {
                let offset = start + i * entry_size;

                if with_addend {
                    if offset + core::mem::size_of::<Elf64Rela>() > data.len() {
                        break;
                    }

                    // SAFETY: We've validated the data length above. Using read_unaligned
                    // because byte slice may not be aligned.
                    let rela: Elf64Rela = unsafe {
                        core::ptr::read_unaligned(data[offset..].as_ptr().cast::<Elf64Rela>())
                    };

                    self.relocations.push(ElfRelocation {
                        offset: rela.r_offset,
                        reloc_type: rela.reloc_type(),
                        symbol_index: rela.symbol_index(),
                        addend: rela.r_addend,
                    });
                } else {
                    if offset + core::mem::size_of::<Elf64Rel>() > data.len() {
                        break;
                    }

                    // SAFETY: We've validated the data length above. Using read_unaligned
                    // because byte slice may not be aligned.
                    let rel: Elf64Rel = unsafe {
                        core::ptr::read_unaligned(data[offset..].as_ptr().cast::<Elf64Rel>())
                    };

                    self.relocations.push(ElfRelocation {
                        offset: rel.r_offset,
                        reloc_type: rel.reloc_type(),
                        symbol_index: rel.symbol_index(),
                        addend: 0,
                    });
                }
            }
        }
    }

    /// Parse dynamic section
    fn parse_dynamic(&mut self, data: &[u8]) {
        self.dynamic.clear();

        // Find PT_DYNAMIC segment
        let Some(dyn_seg) = self
            .program_headers
            .iter()
            .find(|p| p.p_type == pt::PT_DYNAMIC)
        else {
            return;
        };

        let Some(start) = usize::try_from(dyn_seg.p_offset).ok() else {
            return;
        };
        let entry_size = core::mem::size_of::<Elf64Dyn>();
        let Some(filesz) = usize::try_from(dyn_seg.p_filesz).ok() else {
            return;
        };
        let count = filesz / entry_size;

        for i in 0..count {
            let offset = start + i * entry_size;
            if offset + entry_size > data.len() {
                break;
            }

            // SAFETY: We've validated the data length above. Using read_unaligned
            // because byte slice may not be aligned.
            let dyn_entry: Elf64Dyn =
                unsafe { core::ptr::read_unaligned(data[offset..].as_ptr().cast::<Elf64Dyn>()) };

            if dyn_entry.d_tag == dt::DT_NULL {
                break;
            }

            self.dynamic.push(dyn_entry);
        }
    }

    /// Build loaded image from parsed ELF
    fn build_image(&self) -> Result<LoadedImage> {
        let header = self.header.as_ref().ok_or(Error::NotLoaded)?;

        // Calculate memory layout
        let mut min_addr = u64::MAX;
        let mut max_addr = 0u64;

        for phdr in &self.program_headers {
            if !phdr.is_load() {
                continue;
            }

            if phdr.p_vaddr < min_addr {
                min_addr = phdr.p_vaddr;
            }

            let end = phdr.p_vaddr + phdr.p_memsz;
            if end > max_addr {
                max_addr = end;
            }
        }

        // Build sections from section headers
        let mut sections = Vec::new();

        for shdr in &self.section_headers {
            if !shdr.is_allocated() {
                continue;
            }

            let name = self.section_name(shdr.sh_name);

            sections.push(ImageSection {
                name,
                virtual_address: VirtualAddress(shdr.sh_addr),
                size: shdr.sh_size,
                file_offset: shdr.sh_offset,
                file_size: if shdr.is_bss() { 0 } else { shdr.sh_size },
                alignment: shdr.sh_addralign,
                flags: shdr.to_section_flags(),
            });
        }

        // Check for NX stack
        let nx_stack = self
            .program_headers
            .iter()
            .any(|p| p.p_type == pt::PT_GNU_STACK && !p.is_executable());

        // Build image flags
        let flags = ImageFlags {
            pie: header.is_pie(),
            nx_stack,
            relocatable: !self.relocations.is_empty(),
            has_symbols: !self.symbols.is_empty(),
            stripped: self.symbols.is_empty(),
        };

        // Find BSS
        let bss_section = self
            .section_headers
            .iter()
            .find(|s| s.is_bss() && s.is_allocated());

        let (bss_start, bss_size) =
            bss_section.map_or((None, 0), |s| (Some(VirtualAddress(s.sh_addr)), s.sh_size));

        Ok(LoadedImage {
            format: ImageFormat::Elf64,
            entry_point: VirtualAddress(header.e_entry),
            load_address: VirtualAddress(min_addr),
            image_size: max_addr - min_addr,
            sections,
            stack_top: None,
            bss_start,
            bss_size,
            name: String::new(),
            machine: header.machine_type(),
            flags,
        })
    }

    /// Get header
    #[must_use]
    pub const fn header(&self) -> Option<&Elf64Header> {
        self.header.as_ref()
    }

    /// Get program headers
    #[must_use]
    pub fn program_headers(&self) -> &[Elf64ProgramHeader] {
        &self.program_headers
    }

    /// Get section headers
    #[must_use]
    pub fn section_headers(&self) -> &[Elf64SectionHeader] {
        &self.section_headers
    }

    /// Get symbols
    #[must_use]
    pub fn symbols(&self) -> &[ElfSymbol] {
        &self.symbols
    }

    /// Find symbol by name
    pub fn find_symbol(&self, name: &str) -> Option<&ElfSymbol> {
        self.symbols.iter().find(|s| s.name == name)
    }

    /// Get relocations
    pub fn relocations(&self) -> &[ElfRelocation] {
        &self.relocations
    }

    /// Get dynamic entries
    #[must_use]
    pub fn dynamic(&self) -> &[Elf64Dyn] {
        &self.dynamic
    }

    /// Get loaded image
    #[must_use]
    pub const fn image(&self) -> Option<&LoadedImage> {
        self.image.as_ref()
    }

    /// Get loadable segments
    #[must_use]
    pub fn loadable_segments(&self) -> Vec<&Elf64ProgramHeader> {
        self.program_headers
            .iter()
            .filter(|h| h.is_load())
            .collect()
    }

    /// Get raw data
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl Default for ElfLoader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// ELF SYMBOL
// =============================================================================

/// Parsed ELF symbol
#[derive(Debug, Clone)]
pub struct ElfSymbol {
    /// Symbol name
    pub name: String,
    /// Symbol value
    pub value: u64,
    /// Symbol size
    pub size: u64,
    /// Symbol info
    pub info: u8,
    /// Symbol visibility
    pub other: u8,
    /// Section index
    pub section_index: u16,
}

impl ElfSymbol {
    /// Get binding
    #[must_use]
    pub const fn binding(&self) -> u8 {
        self.info >> 4
    }

    /// Get type
    #[must_use]
    pub const fn symbol_type(&self) -> u8 {
        self.info & 0xF
    }

    /// Check if function
    #[must_use]
    pub const fn is_function(&self) -> bool {
        self.symbol_type() == stt::STT_FUNC
    }

    /// Check if global
    #[must_use]
    pub const fn is_global(&self) -> bool {
        self.binding() == stb::STB_GLOBAL
    }
}

// =============================================================================
// ELF RELOCATION
// =============================================================================

/// Parsed ELF relocation
#[derive(Debug, Clone)]
pub struct ElfRelocation {
    /// Offset
    pub offset: u64,
    /// Relocation type
    pub reloc_type: u32,
    /// Symbol index
    pub symbol_index: u32,
    /// Addend
    pub addend: i64,
}

impl ElfRelocation {
    /// Get relocation type name for `x86_64`
    #[must_use]
    pub const fn type_name(&self) -> &'static str {
        match self.reloc_type {
            r_x86_64::R_X86_64_NONE => "R_X86_64_NONE",
            r_x86_64::R_X86_64_64 => "R_X86_64_64",
            r_x86_64::R_X86_64_PC32 => "R_X86_64_PC32",
            r_x86_64::R_X86_64_GOT32 => "R_X86_64_GOT32",
            r_x86_64::R_X86_64_PLT32 => "R_X86_64_PLT32",
            r_x86_64::R_X86_64_COPY => "R_X86_64_COPY",
            r_x86_64::R_X86_64_GLOB_DAT => "R_X86_64_GLOB_DAT",
            r_x86_64::R_X86_64_JUMP_SLOT => "R_X86_64_JUMP_SLOT",
            r_x86_64::R_X86_64_RELATIVE => "R_X86_64_RELATIVE",
            r_x86_64::R_X86_64_GOTPCREL => "R_X86_64_GOTPCREL",
            r_x86_64::R_X86_64_32 => "R_X86_64_32",
            r_x86_64::R_X86_64_32S => "R_X86_64_32S",
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
    fn test_elf_magic() {
        assert_eq!(ELF_MAGIC, [0x7F, b'E', b'L', b'F']);
    }

    #[test]
    fn test_elf_header_parse() {
        // Minimal valid ELF64 header
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(&ELF_MAGIC);
        data[4] = class::ELFCLASS64;
        data[5] = data::ELFDATA2LSB;
        data[6] = 1; // Version

        let header = Elf64Header::parse(&data);
        assert!(header.is_ok());
    }

    #[test]
    fn test_program_header_flags() {
        let phdr = Elf64ProgramHeader {
            p_type: pt::PT_LOAD,
            p_flags: pf::PF_R | pf::PF_X,
            p_offset: 0,
            p_vaddr: 0x1000,
            p_paddr: 0x1000,
            p_filesz: 0x1000,
            p_memsz: 0x2000,
            p_align: 0x1000,
        };

        assert!(phdr.is_load());
        assert!(phdr.is_readable());
        assert!(phdr.is_executable());
        assert!(!phdr.is_writable());
        assert_eq!(phdr.bss_size(), 0x1000);
    }

    #[test]
    fn test_section_flags() {
        let shdr = Elf64SectionHeader {
            sh_name: 0,
            sh_type: sht::SHT_PROGBITS,
            sh_flags: shf::SHF_ALLOC | shf::SHF_EXECINSTR,
            sh_addr: 0,
            sh_offset: 0,
            sh_size: 0,
            sh_link: 0,
            sh_info: 0,
            sh_addralign: 0,
            sh_entsize: 0,
        };

        assert!(shdr.is_allocated());
        assert!(shdr.is_executable());
        assert!(!shdr.is_writable());
        assert!(!shdr.is_bss());
    }
}
