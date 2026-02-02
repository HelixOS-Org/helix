//! # ELF Parsing and Structures
//!
//! Complete ELF64 parsing for kernel relocation.

use core::mem;

// ============================================================================
// ELF CONSTANTS
// ============================================================================

/// ELF magic bytes
pub const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

/// ELF class: 64-bit
pub const ELFCLASS64: u8 = 2;

/// ELF data: little-endian
pub const ELFDATA2LSB: u8 = 1;

/// ELF machine: x86_64
pub const EM_X86_64: u16 = 62;

/// ELF machine: AArch64
pub const EM_AARCH64: u16 = 183;

/// ELF machine: RISC-V
pub const EM_RISCV: u16 = 243;

/// ELF type: executable
pub const ET_EXEC: u16 = 2;

/// ELF type: shared object (PIE)
pub const ET_DYN: u16 = 3;

// Section types
/// Null section
pub const SHT_NULL: u32 = 0;
/// Program data
pub const SHT_PROGBITS: u32 = 1;
/// Symbol table
pub const SHT_SYMTAB: u32 = 2;
/// String table
pub const SHT_STRTAB: u32 = 3;
/// Relocation entries with addends
pub const SHT_RELA: u32 = 4;
/// Symbol hash table
pub const SHT_HASH: u32 = 5;
/// Dynamic linking info
pub const SHT_DYNAMIC: u32 = 6;
/// Note section
pub const SHT_NOTE: u32 = 7;
/// BSS
pub const SHT_NOBITS: u32 = 8;
/// Relocation entries (no addends)
pub const SHT_REL: u32 = 9;
/// Dynamic symbol table
pub const SHT_DYNSYM: u32 = 11;

// Program header types
/// Loadable segment
pub const PT_LOAD: u32 = 1;
/// Dynamic linking info
pub const PT_DYNAMIC: u32 = 2;
/// Interpreter path
pub const PT_INTERP: u32 = 3;
/// Note section
pub const PT_NOTE: u32 = 4;
/// Program header table
pub const PT_PHDR: u32 = 6;
/// Thread-local storage
pub const PT_TLS: u32 = 7;
/// GNU relro
pub const PT_GNU_RELRO: u32 = 0x6474e552;

// Dynamic tags
/// Null entry
pub const DT_NULL: i64 = 0;
/// String table address
pub const DT_STRTAB: i64 = 5;
/// Symbol table address
pub const DT_SYMTAB: i64 = 6;
/// RELA table address
pub const DT_RELA: i64 = 7;
/// RELA table size
pub const DT_RELASZ: i64 = 8;
/// RELA entry size
pub const DT_RELAENT: i64 = 9;
/// PLT relocations address
pub const DT_JMPREL: i64 = 23;
/// PLT relocations size
pub const DT_PLTRELSZ: i64 = 2;
/// RELA count (for relative relocs only)
pub const DT_RELACOUNT: i64 = 0x6ffffff9;

// ============================================================================
// ELF STRUCTURES
// ============================================================================

/// ELF64 file header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Header {
    /// Magic number and other info
    pub e_ident: [u8; 16],
    /// Object file type
    pub e_type: u16,
    /// Architecture
    pub e_machine: u16,
    /// Object file version
    pub e_version: u32,
    /// Entry point virtual address
    pub e_entry: u64,
    /// Program header table file offset
    pub e_phoff: u64,
    /// Section header table file offset
    pub e_shoff: u64,
    /// Processor-specific flags
    pub e_flags: u32,
    /// ELF header size in bytes
    pub e_ehsize: u16,
    /// Program header table entry size
    pub e_phentsize: u16,
    /// Program header table entry count
    pub e_phnum: u16,
    /// Section header table entry size
    pub e_shentsize: u16,
    /// Section header table entry count
    pub e_shnum: u16,
    /// Section name string table index
    pub e_shstrndx: u16,
}

impl Elf64Header {
    /// Validate ELF header
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.e_ident[0..4] != ELF_MAGIC {
            return Err("Invalid ELF magic");
        }
        if self.e_ident[4] != ELFCLASS64 {
            return Err("Not 64-bit ELF");
        }
        if self.e_ident[5] != ELFDATA2LSB {
            return Err("Not little-endian");
        }
        Ok(())
    }

    /// Check if this is a PIE/shared object
    pub fn is_pie(&self) -> bool {
        self.e_type == ET_DYN
    }

    /// Check if this is a static executable
    pub fn is_static(&self) -> bool {
        self.e_type == ET_EXEC
    }

    /// Get entry point
    pub fn entry_point(&self) -> u64 {
        self.e_entry
    }
}

/// ELF64 program header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Phdr {
    /// Segment type
    pub p_type: u32,
    /// Segment flags
    pub p_flags: u32,
    /// Segment file offset
    pub p_offset: u64,
    /// Segment virtual address
    pub p_vaddr: u64,
    /// Segment physical address
    pub p_paddr: u64,
    /// Segment size in file
    pub p_filesz: u64,
    /// Segment size in memory
    pub p_memsz: u64,
    /// Segment alignment
    pub p_align: u64,
}

impl Elf64Phdr {
    /// Check if this is a loadable segment
    pub fn is_loadable(&self) -> bool {
        self.p_type == PT_LOAD
    }

    /// Check if this is the dynamic segment
    pub fn is_dynamic(&self) -> bool {
        self.p_type == PT_DYNAMIC
    }

    /// Check if executable
    pub fn is_executable(&self) -> bool {
        self.p_flags & 1 != 0
    }

    /// Check if writable
    pub fn is_writable(&self) -> bool {
        self.p_flags & 2 != 0
    }
}

/// ELF64 section header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Shdr {
    /// Section name (string table index)
    pub sh_name: u32,
    /// Section type
    pub sh_type: u32,
    /// Section flags
    pub sh_flags: u64,
    /// Section virtual address
    pub sh_addr: u64,
    /// Section file offset
    pub sh_offset: u64,
    /// Section size in bytes
    pub sh_size: u64,
    /// Link to another section
    pub sh_link: u32,
    /// Additional section info
    pub sh_info: u32,
    /// Section alignment
    pub sh_addralign: u64,
    /// Entry size (if section holds table)
    pub sh_entsize: u64,
}

impl Elf64Shdr {
    /// Check if this is a RELA section
    pub fn is_rela(&self) -> bool {
        self.sh_type == SHT_RELA
    }

    /// Check if this is a dynamic section
    pub fn is_dynamic(&self) -> bool {
        self.sh_type == SHT_DYNAMIC
    }

    /// Get number of entries
    pub fn entry_count(&self) -> usize {
        if self.sh_entsize > 0 {
            (self.sh_size / self.sh_entsize) as usize
        } else {
            0
        }
    }
}

/// ELF64 relocation entry with addend
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Rela {
    /// Offset in section to relocate
    pub r_offset: u64,
    /// Relocation type and symbol index
    pub r_info: u64,
    /// Addend for calculation
    pub r_addend: i64,
}

impl Elf64Rela {
    /// Get relocation type
    pub fn r_type(&self) -> u32 {
        (self.r_info & 0xFFFFFFFF) as u32
    }

    /// Get symbol index
    pub fn r_sym(&self) -> u32 {
        (self.r_info >> 32) as u32
    }

    /// Get offset
    pub fn offset(&self) -> u64 {
        self.r_offset
    }

    /// Get addend
    pub fn addend(&self) -> i64 {
        self.r_addend
    }
}

/// ELF64 symbol table entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Sym {
    /// Symbol name (string table index)
    pub st_name: u32,
    /// Symbol type and binding
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

impl Elf64Sym {
    /// Get symbol binding
    pub fn binding(&self) -> u8 {
        self.st_info >> 4
    }

    /// Get symbol type
    pub fn stype(&self) -> u8 {
        self.st_info & 0xF
    }

    /// Check if defined
    pub fn is_defined(&self) -> bool {
        self.st_shndx != 0
    }

    /// Get value
    pub fn value(&self) -> u64 {
        self.st_value
    }
}

/// ELF64 dynamic entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Dyn {
    /// Dynamic entry type
    pub d_tag: i64,
    /// Value or address
    pub d_val: u64,
}

// ============================================================================
// ELF INFO
// ============================================================================

/// ELF class (32 or 64 bit)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfClass {
    /// 32-bit ELF
    Elf32,
    /// 64-bit ELF
    Elf64,
}

/// ELF machine type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfMachine {
    /// x86_64
    X86_64,
    /// AArch64
    AArch64,
    /// RISC-V
    RiscV,
    /// Unknown
    Unknown(u16),
}

impl From<u16> for ElfMachine {
    fn from(value: u16) -> Self {
        match value {
            EM_X86_64 => Self::X86_64,
            EM_AARCH64 => Self::AArch64,
            EM_RISCV => Self::RiscV,
            other => Self::Unknown(other),
        }
    }
}

/// Parsed ELF information
#[derive(Debug, Clone)]
pub struct ElfInfo {
    /// ELF class
    pub class: ElfClass,
    /// Machine type
    pub machine: ElfMachine,
    /// Is PIE/shared object
    pub is_pie: bool,
    /// Entry point
    pub entry_point: u64,
    /// Base address (lowest load address)
    pub base_address: u64,
    /// RELA table virtual address
    pub rela_addr: Option<u64>,
    /// RELA table size
    pub rela_size: usize,
    /// RELA entry count
    pub rela_count: usize,
    /// PLT RELA address
    pub plt_rela_addr: Option<u64>,
    /// PLT RELA size
    pub plt_rela_size: usize,
    /// Dynamic section address
    pub dynamic_addr: Option<u64>,
    /// GOT address
    pub got_addr: Option<u64>,
    /// Symbol table address
    pub symtab_addr: Option<u64>,
    /// String table address
    pub strtab_addr: Option<u64>,
}

impl ElfInfo {
    /// Create from header pointer
    ///
    /// # Safety
    /// Pointer must point to valid ELF header
    pub unsafe fn from_header(header_ptr: *const u8) -> Result<Self, &'static str> {
        let header = unsafe { &*(header_ptr as *const Elf64Header) };
        header.validate()?;

        let mut info = Self {
            class: ElfClass::Elf64,
            machine: ElfMachine::from(header.e_machine),
            is_pie: header.is_pie(),
            entry_point: header.entry_point(),
            base_address: u64::MAX,
            rela_addr: None,
            rela_size: 0,
            rela_count: 0,
            plt_rela_addr: None,
            plt_rela_size: 0,
            dynamic_addr: None,
            got_addr: None,
            symtab_addr: None,
            strtab_addr: None,
        };

        // Parse program headers
        let phdr_base = unsafe { header_ptr.add(header.e_phoff as usize) } as *const Elf64Phdr;
        for i in 0..header.e_phnum as usize {
            let phdr = unsafe { &*phdr_base.add(i) };

            // Find lowest load address
            if phdr.is_loadable() && phdr.p_vaddr < info.base_address {
                info.base_address = phdr.p_vaddr;
            }

            // Find dynamic segment
            if phdr.is_dynamic() {
                info.dynamic_addr = Some(phdr.p_vaddr);

                // Parse dynamic entries
                let dyn_ptr = unsafe { header_ptr.add(phdr.p_offset as usize) } as *const Elf64Dyn;
                let mut i = 0;
                loop {
                    let dyn_entry = unsafe { &*dyn_ptr.add(i) };
                    if dyn_entry.d_tag == DT_NULL {
                        break;
                    }

                    match dyn_entry.d_tag {
                        DT_RELA => info.rela_addr = Some(dyn_entry.d_val),
                        DT_RELASZ => info.rela_size = dyn_entry.d_val as usize,
                        DT_RELACOUNT => info.rela_count = dyn_entry.d_val as usize,
                        DT_JMPREL => info.plt_rela_addr = Some(dyn_entry.d_val),
                        DT_PLTRELSZ => info.plt_rela_size = dyn_entry.d_val as usize,
                        DT_SYMTAB => info.symtab_addr = Some(dyn_entry.d_val),
                        DT_STRTAB => info.strtab_addr = Some(dyn_entry.d_val),
                        _ => {},
                    }

                    i += 1;
                }
            }
        }

        // Calculate rela count if not provided
        if info.rela_count == 0 && info.rela_size > 0 {
            info.rela_count = info.rela_size / mem::size_of::<Elf64Rela>();
        }

        Ok(info)
    }

    /// Check if relocations are available
    pub fn has_relocations(&self) -> bool {
        self.rela_addr.is_some() && self.rela_count > 0
    }

    /// Get total relocation count (RELA + PLT)
    pub fn total_relocation_count(&self) -> usize {
        let plt_count = self.plt_rela_size / mem::size_of::<Elf64Rela>();
        self.rela_count + plt_count
    }
}

// ============================================================================
// MODULE EXPORTS
// ============================================================================

pub mod parser;
pub mod relocations;
pub mod sections;
pub mod symbols;
