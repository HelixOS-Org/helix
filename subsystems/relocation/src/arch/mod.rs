//! # Architecture-Specific Relocation
//!
//! Per-architecture relocation implementations.

use crate::{RelocResult, RelocError};

// ============================================================================
// ARCHITECTURE TRAIT
// ============================================================================

/// Architecture-specific relocation operations
pub trait ArchRelocation {
    /// Architecture name
    const NAME: &'static str;

    /// RELATIVE relocation type number
    const R_RELATIVE: u32;

    /// Check if relocation type is supported
    fn is_supported(r_type: u32) -> bool;

    /// Get relocation type name
    fn reloc_name(r_type: u32) -> &'static str;

    /// Apply a single relocation
    ///
    /// # Safety
    /// Target must be valid and writable
    unsafe fn apply_relocation(
        target: *mut u8,
        r_type: u32,
        addend: i64,
        slide: i64,
        symbol_value: u64,
    ) -> RelocResult<()>;
}

// ============================================================================
// X86_64 IMPLEMENTATION
// ============================================================================

#[cfg(target_arch = "x86_64")]
pub mod x86_64_impl {
    use super::*;
    use crate::elf::relocations::x86_64::*;

    /// x86_64 relocation implementation
    pub struct X86_64Relocation;

    impl ArchRelocation for X86_64Relocation {
        const NAME: &'static str = "x86_64";
        const R_RELATIVE: u32 = R_X86_64_RELATIVE;

        fn is_supported(r_type: u32) -> bool {
            is_supported(r_type)
        }

        fn reloc_name(r_type: u32) -> &'static str {
            name(r_type)
        }

        unsafe fn apply_relocation(
            target: *mut u8,
            r_type: u32,
            addend: i64,
            slide: i64,
            symbol_value: u64,
        ) -> RelocResult<()> {
            unsafe {
                crate::elf::relocations::apply_x86_64_relocation(
                    target, r_type, addend, slide, symbol_value,
                )
            }
        }
    }
}

// ============================================================================
// AARCH64 IMPLEMENTATION
// ============================================================================

#[cfg(target_arch = "aarch64")]
pub mod aarch64_impl {
    use super::*;
    use crate::elf::relocations::aarch64::*;

    /// AArch64 relocation implementation
    pub struct AArch64Relocation;

    impl ArchRelocation for AArch64Relocation {
        const NAME: &'static str = "aarch64";
        const R_RELATIVE: u32 = R_AARCH64_RELATIVE;

        fn is_supported(r_type: u32) -> bool {
            is_supported(r_type)
        }

        fn reloc_name(r_type: u32) -> &'static str {
            name(r_type)
        }

        unsafe fn apply_relocation(
            target: *mut u8,
            r_type: u32,
            addend: i64,
            slide: i64,
            symbol_value: u64,
        ) -> RelocResult<()> {
            unsafe {
                crate::elf::relocations::apply_aarch64_relocation(
                    target, r_type, addend, slide, symbol_value,
                )
            }
        }
    }
}

// ============================================================================
// CURRENT ARCHITECTURE
// ============================================================================

/// Get current architecture name
pub fn current_arch() -> &'static str {
    #[cfg(target_arch = "x86_64")]
    { "x86_64" }
    #[cfg(target_arch = "aarch64")]
    { "aarch64" }
    #[cfg(target_arch = "riscv64")]
    { "riscv64" }
    #[cfg(not(any(
        target_arch = "x86_64",
        target_arch = "aarch64",
        target_arch = "riscv64"
    )))]
    { "unknown" }
}

/// Get RELATIVE relocation type for current arch
pub fn relative_reloc_type() -> u32 {
    #[cfg(target_arch = "x86_64")]
    { crate::elf::relocations::x86_64::R_X86_64_RELATIVE }
    #[cfg(target_arch = "aarch64")]
    { crate::elf::relocations::aarch64::R_AARCH64_RELATIVE }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    { 0 }
}

/// Apply relocation for current architecture
///
/// # Safety
/// Target must be valid and writable
#[inline]
pub unsafe fn apply_current_arch_relocation(
    target: *mut u8,
    r_type: u32,
    addend: i64,
    slide: i64,
    symbol_value: u64,
) -> RelocResult<()> {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            crate::elf::relocations::apply_x86_64_relocation(
                target, r_type, addend, slide, symbol_value,
            )
        }
    }
    #[cfg(target_arch = "aarch64")]
    {
        unsafe {
            crate::elf::relocations::apply_aarch64_relocation(
                target, r_type, addend, slide, symbol_value,
            )
        }
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        Err(RelocError::UnsupportedRelocType(r_type))
    }
}
