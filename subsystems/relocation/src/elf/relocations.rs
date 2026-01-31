//! # ELF Relocations
//!
//! Relocation type definitions and processing.

use crate::{RelocResult, RelocError};

// ============================================================================
// X86_64 RELOCATION TYPES
// ============================================================================

/// x86_64 relocation types
pub mod x86_64 {
    /// No relocation
    pub const R_X86_64_NONE: u32 = 0;
    /// Direct 64-bit
    pub const R_X86_64_64: u32 = 1;
    /// PC relative 32-bit signed
    pub const R_X86_64_PC32: u32 = 2;
    /// 32-bit GOT entry
    pub const R_X86_64_GOT32: u32 = 3;
    /// 32-bit PLT address
    pub const R_X86_64_PLT32: u32 = 4;
    /// Copy symbol at runtime
    pub const R_X86_64_COPY: u32 = 5;
    /// Create GOT entry
    pub const R_X86_64_GLOB_DAT: u32 = 6;
    /// Create PLT entry
    pub const R_X86_64_JUMP_SLOT: u32 = 7;
    /// Adjust by slide (base + addend)
    pub const R_X86_64_RELATIVE: u32 = 8;
    /// 32-bit signed PC relative offset to GOT
    pub const R_X86_64_GOTPCREL: u32 = 9;
    /// Direct 32-bit zero extended
    pub const R_X86_64_32: u32 = 10;
    /// Direct 32-bit sign extended
    pub const R_X86_64_32S: u32 = 11;
    /// Direct 16-bit zero extended
    pub const R_X86_64_16: u32 = 12;
    /// 16-bit PC relative
    pub const R_X86_64_PC16: u32 = 13;
    /// Direct 8-bit sign extended
    pub const R_X86_64_8: u32 = 14;
    /// 8-bit PC relative
    pub const R_X86_64_PC8: u32 = 15;
    /// PC relative 64 bit
    pub const R_X86_64_PC64: u32 = 24;
    /// 64-bit offset to GOT
    pub const R_X86_64_GOTOFF64: u32 = 25;
    /// Signed 32-bit PC relative offset to GOT
    pub const R_X86_64_GOTPC32: u32 = 26;
    /// 64-bit GOT entry offset
    pub const R_X86_64_GOT64: u32 = 27;
    /// 64-bit PC relative offset to GOT entry
    pub const R_X86_64_GOTPCREL64: u32 = 28;
    /// 64-bit PC relative offset to GOT
    pub const R_X86_64_GOTPC64: u32 = 29;
    /// 64-bit GOT offset for PLT
    pub const R_X86_64_GOTPLT64: u32 = 30;
    /// Size of symbol plus addend
    pub const R_X86_64_SIZE32: u32 = 32;
    /// Size of symbol plus addend (64-bit)
    pub const R_X86_64_SIZE64: u32 = 33;
    /// TLS initial exec model
    pub const R_X86_64_TPOFF64: u32 = 18;
    /// TLS local dynamic model
    pub const R_X86_64_DTPMOD64: u32 = 16;
    /// TLS offset in module
    pub const R_X86_64_DTPOFF64: u32 = 17;

    /// Get relocation name
    pub fn name(r_type: u32) -> &'static str {
        match r_type {
            R_X86_64_NONE => "R_X86_64_NONE",
            R_X86_64_64 => "R_X86_64_64",
            R_X86_64_PC32 => "R_X86_64_PC32",
            R_X86_64_GOT32 => "R_X86_64_GOT32",
            R_X86_64_PLT32 => "R_X86_64_PLT32",
            R_X86_64_COPY => "R_X86_64_COPY",
            R_X86_64_GLOB_DAT => "R_X86_64_GLOB_DAT",
            R_X86_64_JUMP_SLOT => "R_X86_64_JUMP_SLOT",
            R_X86_64_RELATIVE => "R_X86_64_RELATIVE",
            R_X86_64_GOTPCREL => "R_X86_64_GOTPCREL",
            R_X86_64_32 => "R_X86_64_32",
            R_X86_64_32S => "R_X86_64_32S",
            R_X86_64_16 => "R_X86_64_16",
            R_X86_64_PC16 => "R_X86_64_PC16",
            R_X86_64_8 => "R_X86_64_8",
            R_X86_64_PC8 => "R_X86_64_PC8",
            R_X86_64_PC64 => "R_X86_64_PC64",
            R_X86_64_GOTOFF64 => "R_X86_64_GOTOFF64",
            R_X86_64_GOTPC32 => "R_X86_64_GOTPC32",
            R_X86_64_SIZE32 => "R_X86_64_SIZE32",
            R_X86_64_SIZE64 => "R_X86_64_SIZE64",
            R_X86_64_TPOFF64 => "R_X86_64_TPOFF64",
            R_X86_64_DTPMOD64 => "R_X86_64_DTPMOD64",
            R_X86_64_DTPOFF64 => "R_X86_64_DTPOFF64",
            _ => "UNKNOWN",
        }
    }

    /// Check if relocation is supported
    pub fn is_supported(r_type: u32) -> bool {
        matches!(
            r_type,
            R_X86_64_NONE
                | R_X86_64_RELATIVE
                | R_X86_64_64
                | R_X86_64_32
                | R_X86_64_32S
                | R_X86_64_PC32
                | R_X86_64_PC64
                | R_X86_64_GLOB_DAT
                | R_X86_64_JUMP_SLOT
                | R_X86_64_PLT32
        )
    }
}

// ============================================================================
// AARCH64 RELOCATION TYPES
// ============================================================================

/// AArch64 relocation types
pub mod aarch64 {
    /// No relocation
    pub const R_AARCH64_NONE: u32 = 0;
    /// Direct 64-bit
    pub const R_AARCH64_ABS64: u32 = 257;
    /// Direct 32-bit
    pub const R_AARCH64_ABS32: u32 = 258;
    /// PC relative 32-bit
    pub const R_AARCH64_PREL32: u32 = 261;
    /// PC relative 64-bit
    pub const R_AARCH64_PREL64: u32 = 260;
    /// Adjust by slide
    pub const R_AARCH64_RELATIVE: u32 = 1027;
    /// Create GOT entry
    pub const R_AARCH64_GLOB_DAT: u32 = 1025;
    /// Create PLT entry
    pub const R_AARCH64_JUMP_SLOT: u32 = 1026;

    /// Get relocation name
    pub fn name(r_type: u32) -> &'static str {
        match r_type {
            R_AARCH64_NONE => "R_AARCH64_NONE",
            R_AARCH64_ABS64 => "R_AARCH64_ABS64",
            R_AARCH64_ABS32 => "R_AARCH64_ABS32",
            R_AARCH64_PREL32 => "R_AARCH64_PREL32",
            R_AARCH64_PREL64 => "R_AARCH64_PREL64",
            R_AARCH64_RELATIVE => "R_AARCH64_RELATIVE",
            R_AARCH64_GLOB_DAT => "R_AARCH64_GLOB_DAT",
            R_AARCH64_JUMP_SLOT => "R_AARCH64_JUMP_SLOT",
            _ => "UNKNOWN",
        }
    }

    /// Check if relocation is supported
    pub fn is_supported(r_type: u32) -> bool {
        matches!(
            r_type,
            R_AARCH64_NONE
                | R_AARCH64_RELATIVE
                | R_AARCH64_ABS64
                | R_AARCH64_ABS32
                | R_AARCH64_PREL32
                | R_AARCH64_PREL64
                | R_AARCH64_GLOB_DAT
                | R_AARCH64_JUMP_SLOT
        )
    }
}

// ============================================================================
// GENERIC RELOCATION INFO
// ============================================================================

/// Generic relocation descriptor
#[derive(Debug, Clone, Copy)]
pub struct RelocationInfo {
    /// Offset in kernel image
    pub offset: u64,
    /// Relocation type
    pub r_type: u32,
    /// Symbol index (0 for RELATIVE)
    pub sym_index: u32,
    /// Addend value
    pub addend: i64,
}

impl RelocationInfo {
    /// Create from ELF RELA entry
    pub fn from_rela(rela: &super::Elf64Rela) -> Self {
        Self {
            offset: rela.offset(),
            r_type: rela.r_type(),
            sym_index: rela.r_sym(),
            addend: rela.addend(),
        }
    }

    /// Check if this is a RELATIVE relocation
    pub fn is_relative(&self) -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            self.r_type == x86_64::R_X86_64_RELATIVE
        }
        #[cfg(target_arch = "aarch64")]
        {
            self.r_type == aarch64::R_AARCH64_RELATIVE
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            false
        }
    }

    /// Check if this needs symbol resolution
    pub fn needs_symbol(&self) -> bool {
        self.sym_index != 0
    }

    /// Get relocation name
    pub fn name(&self) -> &'static str {
        #[cfg(target_arch = "x86_64")]
        {
            x86_64::name(self.r_type)
        }
        #[cfg(target_arch = "aarch64")]
        {
            aarch64::name(self.r_type)
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            "UNKNOWN"
        }
    }
}

// ============================================================================
// RELOCATION APPLICATION
// ============================================================================

/// Apply a single x86_64 relocation
///
/// # Safety
/// - `target` must be a valid, writable pointer within kernel bounds
/// - `slide` must be the correct KASLR slide value
#[cfg(target_arch = "x86_64")]
pub unsafe fn apply_x86_64_relocation(
    target: *mut u8,
    r_type: u32,
    addend: i64,
    slide: i64,
    symbol_value: u64,
) -> RelocResult<()> {
    use x86_64::*;

    match r_type {
        R_X86_64_NONE => {
            // No operation
        }

        R_X86_64_RELATIVE => {
            // *target = base + addend (where base is adjusted by slide)
            // Result: *target += slide
            let ptr = target as *mut u64;
            let current = unsafe { core::ptr::read_unaligned(ptr) };
            let new_value = (current as i64).wrapping_add(slide) as u64;
            unsafe { core::ptr::write_unaligned(ptr, new_value) };
        }

        R_X86_64_64 => {
            // *target = symbol + addend
            let ptr = target as *mut u64;
            let new_value = symbol_value.wrapping_add(addend as u64);
            unsafe { core::ptr::write_unaligned(ptr, new_value) };
        }

        R_X86_64_32 => {
            // *target = (uint32)(symbol + addend)
            let ptr = target as *mut u32;
            let value = symbol_value.wrapping_add(addend as u64);
            if value > u32::MAX as u64 {
                return Err(RelocError::Overflow(target as u64));
            }
            unsafe { core::ptr::write_unaligned(ptr, value as u32) };
        }

        R_X86_64_32S => {
            // *target = (int32)(symbol + addend)
            let ptr = target as *mut i32;
            let value = (symbol_value as i64).wrapping_add(addend);
            if value < i32::MIN as i64 || value > i32::MAX as i64 {
                return Err(RelocError::Overflow(target as u64));
            }
            unsafe { core::ptr::write_unaligned(ptr, value as i32) };
        }

        R_X86_64_PC32 | R_X86_64_PLT32 => {
            // *target = (int32)(symbol + addend - target)
            let ptr = target as *mut i32;
            let pc = target as u64;
            let value = (symbol_value as i64)
                .wrapping_add(addend)
                .wrapping_sub(pc as i64);
            if value < i32::MIN as i64 || value > i32::MAX as i64 {
                return Err(RelocError::Overflow(target as u64));
            }
            unsafe { core::ptr::write_unaligned(ptr, value as i32) };
        }

        R_X86_64_PC64 => {
            // *target = (int64)(symbol + addend - target)
            let ptr = target as *mut i64;
            let pc = target as u64;
            let value = (symbol_value as i64)
                .wrapping_add(addend)
                .wrapping_sub(pc as i64);
            unsafe { core::ptr::write_unaligned(ptr, value) };
        }

        R_X86_64_GLOB_DAT | R_X86_64_JUMP_SLOT => {
            // *target = symbol
            let ptr = target as *mut u64;
            unsafe { core::ptr::write_unaligned(ptr, symbol_value) };
        }

        _ => {
            return Err(RelocError::UnsupportedRelocType(r_type));
        }
    }

    Ok(())
}

/// Apply a single AArch64 relocation
#[cfg(target_arch = "aarch64")]
pub unsafe fn apply_aarch64_relocation(
    target: *mut u8,
    r_type: u32,
    addend: i64,
    slide: i64,
    symbol_value: u64,
) -> RelocResult<()> {
    use aarch64::*;

    match r_type {
        R_AARCH64_NONE => {}

        R_AARCH64_RELATIVE => {
            let ptr = target as *mut u64;
            let current = unsafe { core::ptr::read_unaligned(ptr) };
            let new_value = (current as i64).wrapping_add(slide) as u64;
            unsafe { core::ptr::write_unaligned(ptr, new_value) };
        }

        R_AARCH64_ABS64 => {
            let ptr = target as *mut u64;
            let new_value = symbol_value.wrapping_add(addend as u64);
            unsafe { core::ptr::write_unaligned(ptr, new_value) };
        }

        R_AARCH64_ABS32 => {
            let ptr = target as *mut u32;
            let value = symbol_value.wrapping_add(addend as u64);
            if value > u32::MAX as u64 {
                return Err(RelocError::Overflow(value));
            }
            unsafe { core::ptr::write_unaligned(ptr, value as u32) };
        }

        R_AARCH64_GLOB_DAT | R_AARCH64_JUMP_SLOT => {
            let ptr = target as *mut u64;
            unsafe { core::ptr::write_unaligned(ptr, symbol_value) };
        }

        _ => {
            return Err(RelocError::UnsupportedRelocType(r_type));
        }
    }

    Ok(())
}
