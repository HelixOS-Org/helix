//! # ELF Symbols
//!
//! Symbol table handling.

use super::*;

/// Symbol binding types
pub mod binding {
    pub const STB_LOCAL: u8 = 0;
    pub const STB_GLOBAL: u8 = 1;
    pub const STB_WEAK: u8 = 2;
}

/// Symbol types
pub mod stype {
    pub const STT_NOTYPE: u8 = 0;
    pub const STT_OBJECT: u8 = 1;
    pub const STT_FUNC: u8 = 2;
    pub const STT_SECTION: u8 = 3;
    pub const STT_FILE: u8 = 4;
}

/// Symbol visibility
pub mod visibility {
    pub const STV_DEFAULT: u8 = 0;
    pub const STV_INTERNAL: u8 = 1;
    pub const STV_HIDDEN: u8 = 2;
    pub const STV_PROTECTED: u8 = 3;
}

/// Resolved symbol information
#[derive(Debug, Clone, Copy)]
pub struct ResolvedSymbol {
    /// Symbol index
    pub index: u32,
    /// Symbol value (address)
    pub value: u64,
    /// Symbol size
    pub size: u64,
    /// Is defined
    pub defined: bool,
    /// Is function
    pub is_function: bool,
    /// Is global
    pub is_global: bool,
}

impl ResolvedSymbol {
    /// Create from ELF symbol
    pub fn from_elf(index: u32, sym: &Elf64Sym) -> Self {
        Self {
            index,
            value: sym.value(),
            size: sym.st_size,
            defined: sym.is_defined(),
            is_function: sym.stype() == stype::STT_FUNC,
            is_global: sym.binding() == binding::STB_GLOBAL,
        }
    }

    /// Get value with slide applied
    pub fn value_with_slide(&self, slide: i64) -> u64 {
        (self.value as i64).wrapping_add(slide) as u64
    }
}
