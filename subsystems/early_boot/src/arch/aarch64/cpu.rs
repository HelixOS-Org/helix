//! # AArch64 CPU Initialization
//!
//! CPU feature detection and initialization for ARM64.

use super::*;
use crate::core::{BootContext, CpuFeatures};
use crate::error::{BootError, BootResult};

// =============================================================================
// CPU FEATURE REGISTERS
// =============================================================================

/// ID_AA64PFR0_EL1 fields
pub mod pfr0 {
    /// SVE field
    pub const SVE_SHIFT: u64 = 32;
    pub const SVE_MASK: u64 = 0xF;
    /// GIC field
    pub const GIC_SHIFT: u64 = 24;
    pub const GIC_MASK: u64 = 0xF;
    /// Advanced SIMD
    pub const ADVSIMD_SHIFT: u64 = 20;
    pub const ADVSIMD_MASK: u64 = 0xF;
    /// Floating-point
    pub const FP_SHIFT: u64 = 16;
    pub const FP_MASK: u64 = 0xF;
    /// EL3 handling
    pub const EL3_SHIFT: u64 = 12;
    pub const EL3_MASK: u64 = 0xF;
    /// EL2 handling
    pub const EL2_SHIFT: u64 = 8;
    pub const EL2_MASK: u64 = 0xF;
    /// EL1 handling
    pub const EL1_SHIFT: u64 = 4;
    pub const EL1_MASK: u64 = 0xF;
    /// EL0 handling
    pub const EL0_SHIFT: u64 = 0;
    pub const EL0_MASK: u64 = 0xF;
}

/// ID_AA64MMFR0_EL1 fields
pub mod mmfr0 {
    /// Physical address range
    pub const PARANGE_SHIFT: u64 = 0;
    pub const PARANGE_MASK: u64 = 0xF;
    /// ASID bits
    pub const ASIDBITS_SHIFT: u64 = 4;
    pub const ASIDBITS_MASK: u64 = 0xF;
    /// Big endian support
    pub const BIGEND_SHIFT: u64 = 8;
    pub const BIGEND_MASK: u64 = 0xF;
    /// Secure EL2 support
    pub const SNSMEM_SHIFT: u64 = 12;
    pub const SNSMEM_MASK: u64 = 0xF;
    /// Mixed-endian support
    pub const BIGENDEL0_SHIFT: u64 = 16;
    pub const BIGENDEL0_MASK: u64 = 0xF;
    /// 4KB granule support
    pub const TGRAN4_SHIFT: u64 = 28;
    pub const TGRAN4_MASK: u64 = 0xF;
    /// 64KB granule support
    pub const TGRAN64_SHIFT: u64 = 24;
    pub const TGRAN64_MASK: u64 = 0xF;
    /// 16KB granule support
    pub const TGRAN16_SHIFT: u64 = 20;
    pub const TGRAN16_MASK: u64 = 0xF;
}

/// ID_AA64ISAR0_EL1 fields
pub mod isar0 {
    /// AES support
    pub const AES_SHIFT: u64 = 4;
    pub const AES_MASK: u64 = 0xF;
    /// SHA1 support
    pub const SHA1_SHIFT: u64 = 8;
    pub const SHA1_MASK: u64 = 0xF;
    /// SHA2 support
    pub const SHA2_SHIFT: u64 = 12;
    pub const SHA2_MASK: u64 = 0xF;
    /// CRC32 support
    pub const CRC32_SHIFT: u64 = 16;
    pub const CRC32_MASK: u64 = 0xF;
    /// Atomic instructions
    pub const ATOMIC_SHIFT: u64 = 20;
    pub const ATOMIC_MASK: u64 = 0xF;
    /// RDMA support
    pub const RDM_SHIFT: u64 = 28;
    pub const RDM_MASK: u64 = 0xF;
    /// SHA3 support
    pub const SHA3_SHIFT: u64 = 32;
    pub const SHA3_MASK: u64 = 0xF;
    /// SM3 support
    pub const SM3_SHIFT: u64 = 36;
    pub const SM3_MASK: u64 = 0xF;
    /// SM4 support
    pub const SM4_SHIFT: u64 = 40;
    pub const SM4_MASK: u64 = 0xF;
    /// Dot product support
    pub const DP_SHIFT: u64 = 44;
    pub const DP_MASK: u64 = 0xF;
    /// RNDR support
    pub const RNDR_SHIFT: u64 = 60;
    pub const RNDR_MASK: u64 = 0xF;
}

// =============================================================================
// CPU INFO
// =============================================================================

/// CPU implementer codes
pub mod implementer {
    pub const ARM: u8 = 0x41;
    pub const BROADCOM: u8 = 0x42;
    pub const CAVIUM: u8 = 0x43;
    pub const DEC: u8 = 0x44;
    pub const FUJITSU: u8 = 0x46;
    pub const INFINEON: u8 = 0x49;
    pub const MOTOROLA: u8 = 0x4D;
    pub const NVIDIA: u8 = 0x4E;
    pub const AMCC: u8 = 0x50;
    pub const QUALCOMM: u8 = 0x51;
    pub const MARVELL: u8 = 0x56;
    pub const INTEL: u8 = 0x69;
    pub const AMPERE: u8 = 0xC0;
    pub const APPLE: u8 = 0x61;
}

/// Decode MIDR implementer
pub fn get_implementer(midr: u64) -> u8 {
    ((midr >> 24) & 0xFF) as u8
}

/// Decode MIDR variant
pub fn get_variant(midr: u64) -> u8 {
    ((midr >> 20) & 0xF) as u8
}

/// Decode MIDR architecture
pub fn get_architecture(midr: u64) -> u8 {
    ((midr >> 16) & 0xF) as u8
}

/// Decode MIDR part number
pub fn get_part_number(midr: u64) -> u16 {
    ((midr >> 4) & 0xFFF) as u16
}

/// Decode MIDR revision
pub fn get_revision(midr: u64) -> u8 {
    (midr & 0xF) as u8
}

/// Get implementer name
pub fn get_implementer_name(implementer: u8) -> &'static str {
    match implementer {
        implementer::ARM => "ARM",
        implementer::BROADCOM => "Broadcom",
        implementer::CAVIUM => "Cavium",
        implementer::DEC => "DEC",
        implementer::FUJITSU => "Fujitsu",
        implementer::NVIDIA => "NVIDIA",
        implementer::QUALCOMM => "Qualcomm",
        implementer::MARVELL => "Marvell",
        implementer::INTEL => "Intel",
        implementer::AMPERE => "Ampere",
        implementer::APPLE => "Apple",
        _ => "Unknown",
    }
}

// =============================================================================
// PHYSICAL ADDRESS SIZE
// =============================================================================

/// Get physical address size in bits
pub fn get_pa_size(parange: u64) -> u8 {
    match parange {
        0 => 32,
        1 => 36,
        2 => 40,
        3 => 42,
        4 => 44,
        5 => 48,
        6 => 52,
        _ => 48, // Default to 48 bits
    }
}

/// Get IPS value for TCR
pub fn get_ips_value(pa_bits: u8) -> u64 {
    match pa_bits {
        32 => 0,
        36 => 1,
        40 => 2,
        42 => 3,
        44 => 4,
        48 => 5,
        52 => 6,
        _ => 5, // Default to 48 bits
    }
}

// =============================================================================
// FEATURE DETECTION
// =============================================================================

/// Detect CPU features
///
/// # Safety
///
/// The caller must ensure the firmware is accessible.
pub unsafe fn detect_features(ctx: &mut BootContext) -> BootResult<()> {
    // Read feature registers
    let midr: u64;
    let pfr0: u64;
    let mmfr0: u64;
    let isar0: u64;

    core::arch::asm!(
        "mrs {}, MIDR_EL1",
        out(reg) midr,
        options(nomem, nostack)
    );
    core::arch::asm!(
        "mrs {}, ID_AA64PFR0_EL1",
        out(reg) pfr0,
        options(nomem, nostack)
    );
    core::arch::asm!(
        "mrs {}, ID_AA64MMFR0_EL1",
        out(reg) mmfr0,
        options(nomem, nostack)
    );
    core::arch::asm!(
        "mrs {}, ID_AA64ISAR0_EL1",
        out(reg) isar0,
        options(nomem, nostack)
    );

    // Parse MIDR
    let implementer = get_implementer(midr);
    let part = get_part_number(midr);
    let variant = get_variant(midr);
    let revision = get_revision(midr);

    // Store basic info
    ctx.cpu_state.vendor[..4].copy_from_slice(get_implementer_name(implementer).as_bytes());
    ctx.cpu_state.family = part;
    ctx.cpu_state.model = variant as u16;
    ctx.cpu_state.stepping = revision;

    // Detect features
    let mut features = CpuFeatures::empty();

    // Floating-point and SIMD
    let fp = (pfr0 >> pfr0::FP_SHIFT) & pfr0::FP_MASK;
    let simd = (pfr0 >> pfr0::ADVSIMD_SHIFT) & pfr0::ADVSIMD_MASK;

    if fp != 0xF {
        features |= CpuFeatures::FPU;
    }
    if simd != 0xF {
        features |= CpuFeatures::SIMD;
    }

    // SVE (Scalable Vector Extension)
    let sve = (pfr0 >> pfr0::SVE_SHIFT) & pfr0::SVE_MASK;
    if sve != 0 {
        features |= CpuFeatures::SVE;
    }

    // Cryptography
    let aes = (isar0 >> isar0::AES_SHIFT) & isar0::AES_MASK;
    let sha1 = (isar0 >> isar0::SHA1_SHIFT) & isar0::SHA1_MASK;
    let sha2 = (isar0 >> isar0::SHA2_SHIFT) & isar0::SHA2_MASK;
    let crc32 = (isar0 >> isar0::CRC32_SHIFT) & isar0::CRC32_MASK;
    let atomics = (isar0 >> isar0::ATOMIC_SHIFT) & isar0::ATOMIC_MASK;
    let rndr = (isar0 >> isar0::RNDR_SHIFT) & isar0::RNDR_MASK;

    if aes != 0 {
        features |= CpuFeatures::AES;
    }
    if sha1 != 0 || sha2 != 0 {
        features |= CpuFeatures::SHA;
    }
    if crc32 != 0 {
        features |= CpuFeatures::CRC32;
    }
    if atomics != 0 {
        features |= CpuFeatures::ATOMICS;
    }
    if rndr != 0 {
        features |= CpuFeatures::RDRAND;
    }

    // Memory features
    let parange = (mmfr0 >> mmfr0::PARANGE_SHIFT) & mmfr0::PARANGE_MASK;
    let tgran4 = (mmfr0 >> mmfr0::TGRAN4_SHIFT) & mmfr0::TGRAN4_MASK;
    let tgran16 = (mmfr0 >> mmfr0::TGRAN16_SHIFT) & mmfr0::TGRAN16_MASK;
    let tgran64 = (mmfr0 >> mmfr0::TGRAN64_SHIFT) & mmfr0::TGRAN64_MASK;

    ctx.cpu_state.features = features;

    // Store ARM-specific data
    ctx.arch_data.arm.midr = midr;
    ctx.arch_data.arm.pfr0 = pfr0;
    ctx.arch_data.arm.mmfr0 = mmfr0;
    ctx.arch_data.arm.isar0 = isar0;
    ctx.arch_data.arm.pa_bits = get_pa_size(parange);
    ctx.arch_data.arm.tgran4_supported = tgran4 != 0xF;
    ctx.arch_data.arm.tgran16_supported = tgran16 != 0;
    ctx.arch_data.arm.tgran64_supported = tgran64 != 0xF;

    Ok(())
}

// =============================================================================
// CPU INITIALIZATION
// =============================================================================

/// Initialize CPU
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    let el = read_current_el();

    match el {
        3 => init_el3(ctx)?,
        2 => init_el2(ctx)?,
        1 => init_el1(ctx)?,
        _ => return Err(BootError::InvalidState),
    }

    // Enable floating-point and SIMD
    enable_fp_simd();

    // Configure SCTLR
    configure_sctlr();

    Ok(())
}

/// Initialize from EL3
unsafe fn init_el3(ctx: &mut BootContext) -> BootResult<()> {
    // SCR_EL3: Secure Configuration Register
    let scr: u64;
    core::arch::asm!("mrs {}, SCR_EL3", out(reg) scr, options(nomem, nostack));

    // Set NS bit (non-secure), RW bit (EL2 is AArch64)
    let new_scr = scr | (1 << 0) | (1 << 10);
    core::arch::asm!("msr SCR_EL3, {}", in(reg) new_scr, options(nomem, nostack));

    // CPTR_EL3: Disable trapping
    core::arch::asm!("msr CPTR_EL3, xzr", options(nomem, nostack));

    // MDCR_EL3: Debug control
    core::arch::asm!("msr MDCR_EL3, xzr", options(nomem, nostack));

    ctx.arch_data.arm.current_el = 3;

    Ok(())
}

/// Initialize from EL2
unsafe fn init_el2(ctx: &mut BootContext) -> BootResult<()> {
    // HCR_EL2: Hypervisor Configuration Register
    // Set RW bit (EL1 is AArch64)
    core::arch::asm!("msr HCR_EL2, {}", in(reg) 1u64 << 31, options(nomem, nostack));

    // CPTR_EL2: Don't trap FP/SIMD
    core::arch::asm!("msr CPTR_EL2, xzr", options(nomem, nostack));

    // HSTR_EL2: Don't trap any system registers
    core::arch::asm!("msr HSTR_EL2, xzr", options(nomem, nostack));

    // VTTBR_EL2: Clear stage 2 translation
    core::arch::asm!("msr VTTBR_EL2, xzr", options(nomem, nostack));

    // SCTLR_EL2: System control
    let sctlr: u64 = (1 << 11) | (1 << 20) | (1 << 22) | (1 << 23) | (1 << 28) | (1 << 29);
    core::arch::asm!("msr SCTLR_EL2, {}", in(reg) sctlr, options(nomem, nostack));

    ctx.arch_data.arm.current_el = 2;

    Ok(())
}

/// Initialize from EL1
unsafe fn init_el1(ctx: &mut BootContext) -> BootResult<()> {
    ctx.arch_data.arm.current_el = 1;
    Ok(())
}

/// Enable floating-point and SIMD
unsafe fn enable_fp_simd() {
    // CPACR_EL1: Allow EL0 and EL1 access to FP/SIMD
    let cpacr: u64 = 0x3 << 20; // FPEN = 0b11
    core::arch::asm!("msr CPACR_EL1, {}", in(reg) cpacr, options(nomem, nostack));
    isb();
}

/// Configure SCTLR_EL1
unsafe fn configure_sctlr() {
    let mut sctlr: u64;
    core::arch::asm!("mrs {}, SCTLR_EL1", out(reg) sctlr, options(nomem, nostack));

    // Enable:
    // - Stack alignment check (SA, SA0)
    // - Instruction cache (I)
    // - WXN (write implies no-execute)
    sctlr |= SCTLR_SA | SCTLR_SA0 | SCTLR_I;

    // Disable:
    // - Alignment checking (A) for now
    sctlr &= !SCTLR_A;

    core::arch::asm!("msr SCTLR_EL1, {}", in(reg) sctlr, options(nomem, nostack));
    isb();
}

// =============================================================================
// EXCEPTION LEVEL TRANSITION
// =============================================================================

/// Drop from EL3 to EL2
///
/// # Safety
///
/// The caller must ensure the target exception level is properly configured.
pub unsafe fn drop_to_el2(entry: u64, stack: u64) {
    // Set up SPSR for EL2h
    let spsr: u64 = 0b01001; // EL2h, interrupts masked
    core::arch::asm!("msr SPSR_EL3, {}", in(reg) spsr, options(nomem, nostack));

    // Set up ELR
    core::arch::asm!("msr ELR_EL3, {}", in(reg) entry, options(nomem, nostack));

    // Set up SP_EL2
    core::arch::asm!("msr SP_EL2, {}", in(reg) stack, options(nomem, nostack));

    // Return from exception
    core::arch::asm!("eret", options(noreturn));
}

/// Drop from EL2 to EL1
///
/// # Safety
///
/// The caller must ensure the target exception level is properly configured.
pub unsafe fn drop_to_el1(entry: u64, stack: u64) {
    // Set up SPSR for EL1h
    let spsr: u64 = 0b00101; // EL1h, interrupts masked
    core::arch::asm!("msr SPSR_EL2, {}", in(reg) spsr, options(nomem, nostack));

    // Set up ELR
    core::arch::asm!("msr ELR_EL2, {}", in(reg) entry, options(nomem, nostack));

    // Set up SP_EL1
    core::arch::asm!("msr SP_EL1, {}", in(reg) stack, options(nomem, nostack));

    // Return from exception
    core::arch::asm!("eret", options(noreturn));
}

// =============================================================================
// CACHE OPERATIONS
// =============================================================================

/// Clean and invalidate all data caches
///
/// # Safety
///
/// The caller must ensure the virtual address is valid.
pub unsafe fn clean_invalidate_dcache_all() {
    // Get cache info
    let clidr: u64;
    core::arch::asm!("mrs {}, CLIDR_EL1", out(reg) clidr, options(nomem, nostack));

    let loc = (clidr >> 24) & 0x7; // Level of Coherency

    for level in 0..loc {
        let cache_type = (clidr >> (level * 3)) & 0x7;

        if cache_type >= 2 {
            // Data cache or unified cache
            clean_invalidate_dcache_level(level);
        }
    }

    dsb();
    isb();
}

/// Clean and invalidate data cache at a specific level
unsafe fn clean_invalidate_dcache_level(level: u64) {
    // Select cache level
    let csselr = level << 1;
    core::arch::asm!("msr CSSELR_EL1, {}", in(reg) csselr, options(nomem, nostack));
    isb();

    // Read cache size
    let ccsidr: u64;
    core::arch::asm!("mrs {}, CCSIDR_EL1", out(reg) ccsidr, options(nomem, nostack));

    let line_size = ((ccsidr & 0x7) + 4) as u32;
    let ways = (((ccsidr >> 3) & 0x3FF) + 1) as u32;
    let sets = (((ccsidr >> 13) & 0x7FFF) + 1) as u32;

    let way_shift = 32 - ways.leading_zeros();

    for way in 0..ways {
        for set in 0..sets {
            let val = (way << way_shift) | (set << line_size) | (level << 1);
            core::arch::asm!("dc cisw, {}", in(reg) val as u64, options(nomem, nostack));
        }
    }
}
