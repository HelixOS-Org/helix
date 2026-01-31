//! # ELF Sections
//!
//! Section handling utilities.

use super::*;

/// Section info
#[derive(Debug, Clone)]
pub struct SectionInfo {
    /// Section name
    pub name: &'static str,
    /// Virtual address
    pub vaddr: u64,
    /// File offset
    pub offset: u64,
    /// Size in bytes
    pub size: u64,
    /// Section type
    pub section_type: SectionType,
    /// Is writable
    pub writable: bool,
    /// Is executable
    pub executable: bool,
}

/// Section types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionType {
    /// Code section
    Text,
    /// Read-only data
    Rodata,
    /// Initialized data
    Data,
    /// Uninitialized data
    Bss,
    /// Relocation table
    Rela,
    /// Dynamic info
    Dynamic,
    /// GOT
    Got,
    /// PLT
    Plt,
    /// Symbol table
    Symtab,
    /// String table
    Strtab,
    /// Other
    Other,
}

/// Known section names
pub mod names {
    pub const TEXT: &str = ".text";
    pub const RODATA: &str = ".rodata";
    pub const DATA: &str = ".data";
    pub const BSS: &str = ".bss";
    pub const RELA_DYN: &str = ".rela.dyn";
    pub const RELA_PLT: &str = ".rela.plt";
    pub const DYNAMIC: &str = ".dynamic";
    pub const GOT: &str = ".got";
    pub const GOT_PLT: &str = ".got.plt";
    pub const PLT: &str = ".plt";
    pub const SYMTAB: &str = ".symtab";
    pub const DYNSYM: &str = ".dynsym";
    pub const STRTAB: &str = ".strtab";
    pub const DYNSTR: &str = ".dynstr";
}
