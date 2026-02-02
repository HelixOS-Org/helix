//! AArch64 CPU Feature Detection
//!
//! ARM64 CPU identification and feature detection.

use crate::arch::CpuFeatures;

// =============================================================================
// ID REGISTERS
// =============================================================================

/// Read ID_AA64ISAR0_EL1 (Instruction Set Attribute Register 0)
pub fn read_isar0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, ID_AA64ISAR0_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read ID_AA64ISAR1_EL1 (Instruction Set Attribute Register 1)
pub fn read_isar1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, ID_AA64ISAR1_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read ID_AA64ISAR2_EL1 (Instruction Set Attribute Register 2)
pub fn read_isar2() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, ID_AA64ISAR2_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read ID_AA64PFR0_EL1 (Processor Feature Register 0)
pub fn read_pfr0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, ID_AA64PFR0_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read ID_AA64PFR1_EL1 (Processor Feature Register 1)
pub fn read_pfr1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, ID_AA64PFR1_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read ID_AA64MMFR0_EL1 (Memory Model Feature Register 0)
pub fn read_mmfr0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, ID_AA64MMFR0_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read ID_AA64MMFR1_EL1 (Memory Model Feature Register 1)
pub fn read_mmfr1() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, ID_AA64MMFR1_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read ID_AA64MMFR2_EL1 (Memory Model Feature Register 2)
pub fn read_mmfr2() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, ID_AA64MMFR2_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read ID_AA64DFR0_EL1 (Debug Feature Register 0)
pub fn read_dfr0() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, ID_AA64DFR0_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read MIDR_EL1 (Main ID Register)
pub fn read_midr() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, MIDR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

/// Read REVIDR_EL1 (Revision ID Register)
pub fn read_revidr() -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "mrs {}, REVIDR_EL1",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}

// =============================================================================
// MIDR PARSING
// =============================================================================

/// CPU implementer codes
pub mod implementer {
    pub const ARM: u8 = 0x41;
    pub const BROADCOM: u8 = 0x42;
    pub const CAVIUM: u8 = 0x43;
    pub const FUJITSU: u8 = 0x46;
    pub const NVIDIA: u8 = 0x4E;
    pub const APPLE: u8 = 0x61;
    pub const QUALCOMM: u8 = 0x51;
    pub const SAMSUNG: u8 = 0x53;
    pub const MARVELL: u8 = 0x56;
    pub const FARADAY: u8 = 0x66;
    pub const INTEL: u8 = 0x69;
    pub const AMPERE: u8 = 0xC0;
}

/// ARM part numbers
pub mod arm_part {
    pub const CORTEX_A35: u16 = 0xD04;
    pub const CORTEX_A53: u16 = 0xD03;
    pub const CORTEX_A55: u16 = 0xD05;
    pub const CORTEX_A57: u16 = 0xD07;
    pub const CORTEX_A72: u16 = 0xD08;
    pub const CORTEX_A73: u16 = 0xD09;
    pub const CORTEX_A75: u16 = 0xD0A;
    pub const CORTEX_A76: u16 = 0xD0B;
    pub const CORTEX_A77: u16 = 0xD0D;
    pub const CORTEX_A78: u16 = 0xD41;
    pub const CORTEX_A510: u16 = 0xD46;
    pub const CORTEX_A710: u16 = 0xD47;
    pub const CORTEX_A715: u16 = 0xD4D;
    pub const CORTEX_X1: u16 = 0xD44;
    pub const CORTEX_X2: u16 = 0xD48;
    pub const CORTEX_X3: u16 = 0xD4E;
    pub const NEOVERSE_N1: u16 = 0xD0C;
    pub const NEOVERSE_N2: u16 = 0xD49;
    pub const NEOVERSE_V1: u16 = 0xD40;
    pub const NEOVERSE_V2: u16 = 0xD4F;
}

/// MIDR fields
pub struct MidrFields {
    /// Implementer code
    pub implementer: u8,
    /// Variant
    pub variant: u8,
    /// Architecture
    pub architecture: u8,
    /// Part number
    pub part_number: u16,
    /// Revision
    pub revision: u8,
}

impl MidrFields {
    /// Parse MIDR value
    pub fn from_midr(midr: u64) -> Self {
        Self {
            revision: (midr & 0xF) as u8,
            part_number: ((midr >> 4) & 0xFFF) as u16,
            architecture: ((midr >> 16) & 0xF) as u8,
            variant: ((midr >> 20) & 0xF) as u8,
            implementer: ((midr >> 24) & 0xFF) as u8,
        }
    }

    /// Read and parse current CPU MIDR
    pub fn current() -> Self {
        Self::from_midr(read_midr())
    }

    /// Get implementer name
    pub fn implementer_name(&self) -> &'static str {
        match self.implementer {
            implementer::ARM => "ARM",
            implementer::BROADCOM => "Broadcom",
            implementer::CAVIUM => "Cavium",
            implementer::FUJITSU => "Fujitsu",
            implementer::NVIDIA => "NVIDIA",
            implementer::APPLE => "Apple",
            implementer::QUALCOMM => "Qualcomm",
            implementer::SAMSUNG => "Samsung",
            implementer::MARVELL => "Marvell",
            implementer::INTEL => "Intel",
            implementer::AMPERE => "Ampere",
            _ => "Unknown",
        }
    }

    /// Get ARM part name
    pub fn arm_part_name(&self) -> Option<&'static str> {
        if self.implementer != implementer::ARM {
            return None;
        }
        Some(match self.part_number {
            arm_part::CORTEX_A35 => "Cortex-A35",
            arm_part::CORTEX_A53 => "Cortex-A53",
            arm_part::CORTEX_A55 => "Cortex-A55",
            arm_part::CORTEX_A57 => "Cortex-A57",
            arm_part::CORTEX_A72 => "Cortex-A72",
            arm_part::CORTEX_A73 => "Cortex-A73",
            arm_part::CORTEX_A75 => "Cortex-A75",
            arm_part::CORTEX_A76 => "Cortex-A76",
            arm_part::CORTEX_A77 => "Cortex-A77",
            arm_part::CORTEX_A78 => "Cortex-A78",
            arm_part::CORTEX_A510 => "Cortex-A510",
            arm_part::CORTEX_A710 => "Cortex-A710",
            arm_part::CORTEX_A715 => "Cortex-A715",
            arm_part::CORTEX_X1 => "Cortex-X1",
            arm_part::CORTEX_X2 => "Cortex-X2",
            arm_part::CORTEX_X3 => "Cortex-X3",
            arm_part::NEOVERSE_N1 => "Neoverse N1",
            arm_part::NEOVERSE_N2 => "Neoverse N2",
            arm_part::NEOVERSE_V1 => "Neoverse V1",
            arm_part::NEOVERSE_V2 => "Neoverse V2",
            _ => "Unknown ARM Core",
        })
    }
}

// =============================================================================
// FEATURE DETECTION
// =============================================================================

/// ID_AA64ISAR0_EL1 field extraction
pub mod isar0 {
    /// AES support
    pub fn aes(reg: u64) -> u8 {
        ((reg >> 4) & 0xF) as u8
    }
    /// SHA1 support
    pub fn sha1(reg: u64) -> u8 {
        ((reg >> 8) & 0xF) as u8
    }
    /// SHA2 support
    pub fn sha2(reg: u64) -> u8 {
        ((reg >> 12) & 0xF) as u8
    }
    /// CRC32 support
    pub fn crc32(reg: u64) -> u8 {
        ((reg >> 16) & 0xF) as u8
    }
    /// Atomic instructions support
    pub fn atomic(reg: u64) -> u8 {
        ((reg >> 20) & 0xF) as u8
    }
    /// TME support
    pub fn tme(reg: u64) -> u8 {
        ((reg >> 24) & 0xF) as u8
    }
    /// RDM support
    pub fn rdm(reg: u64) -> u8 {
        ((reg >> 28) & 0xF) as u8
    }
    /// SHA3 support
    pub fn sha3(reg: u64) -> u8 {
        ((reg >> 32) & 0xF) as u8
    }
    /// SM3 support
    pub fn sm3(reg: u64) -> u8 {
        ((reg >> 36) & 0xF) as u8
    }
    /// SM4 support
    pub fn sm4(reg: u64) -> u8 {
        ((reg >> 40) & 0xF) as u8
    }
    /// Dot Product support
    pub fn dp(reg: u64) -> u8 {
        ((reg >> 44) & 0xF) as u8
    }
    /// FHM support
    pub fn fhm(reg: u64) -> u8 {
        ((reg >> 48) & 0xF) as u8
    }
    /// TS support
    pub fn ts(reg: u64) -> u8 {
        ((reg >> 52) & 0xF) as u8
    }
    /// TLB support
    pub fn tlb(reg: u64) -> u8 {
        ((reg >> 56) & 0xF) as u8
    }
    /// RNDR support
    pub fn rndr(reg: u64) -> u8 {
        ((reg >> 60) & 0xF) as u8
    }
}

/// ID_AA64ISAR1_EL1 field extraction
pub mod isar1 {
    /// Data Persistence support
    pub fn dpb(reg: u64) -> u8 {
        (reg & 0xF) as u8
    }
    /// Pointer Authentication (address) support
    pub fn apa(reg: u64) -> u8 {
        ((reg >> 4) & 0xF) as u8
    }
    /// Pointer Authentication (generic) support
    pub fn gpi(reg: u64) -> u8 {
        ((reg >> 8) & 0xF) as u8
    }
    /// JavaScript conversion support
    pub fn jscvt(reg: u64) -> u8 {
        ((reg >> 12) & 0xF) as u8
    }
    /// Flag manipulation support
    pub fn fcma(reg: u64) -> u8 {
        ((reg >> 16) & 0xF) as u8
    }
    /// Load/store acquire/release support
    pub fn lrcpc(reg: u64) -> u8 {
        ((reg >> 20) & 0xF) as u8
    }
    /// Pointer Authentication (combined) support
    pub fn gpa(reg: u64) -> u8 {
        ((reg >> 24) & 0xF) as u8
    }
    /// Pointer Authentication (combined) support
    pub fn api(reg: u64) -> u8 {
        ((reg >> 8) & 0xF) as u8
    }
    /// FP16 support
    pub fn frintts(reg: u64) -> u8 {
        ((reg >> 32) & 0xF) as u8
    }
    /// SB support
    pub fn sb(reg: u64) -> u8 {
        ((reg >> 36) & 0xF) as u8
    }
    /// Speculative Store Bypass Safe
    pub fn specres(reg: u64) -> u8 {
        ((reg >> 40) & 0xF) as u8
    }
    /// BF16 support
    pub fn bf16(reg: u64) -> u8 {
        ((reg >> 44) & 0xF) as u8
    }
    /// DGH support
    pub fn dgh(reg: u64) -> u8 {
        ((reg >> 48) & 0xF) as u8
    }
    /// I8MM support
    pub fn i8mm(reg: u64) -> u8 {
        ((reg >> 52) & 0xF) as u8
    }
}

/// ID_AA64PFR0_EL1 field extraction
pub mod pfr0 {
    /// EL0 handling
    pub fn el0(reg: u64) -> u8 {
        (reg & 0xF) as u8
    }
    /// EL1 handling
    pub fn el1(reg: u64) -> u8 {
        ((reg >> 4) & 0xF) as u8
    }
    /// EL2 handling
    pub fn el2(reg: u64) -> u8 {
        ((reg >> 8) & 0xF) as u8
    }
    /// EL3 handling
    pub fn el3(reg: u64) -> u8 {
        ((reg >> 12) & 0xF) as u8
    }
    /// FP support
    pub fn fp(reg: u64) -> u8 {
        ((reg >> 16) & 0xF) as u8
    }
    /// Advanced SIMD support
    pub fn advsimd(reg: u64) -> u8 {
        ((reg >> 20) & 0xF) as u8
    }
    /// GIC system registers support
    pub fn gic(reg: u64) -> u8 {
        ((reg >> 24) & 0xF) as u8
    }
    /// RAS support
    pub fn ras(reg: u64) -> u8 {
        ((reg >> 28) & 0xF) as u8
    }
    /// SVE support
    pub fn sve(reg: u64) -> u8 {
        ((reg >> 32) & 0xF) as u8
    }
    /// SEL2 support
    pub fn sel2(reg: u64) -> u8 {
        ((reg >> 36) & 0xF) as u8
    }
    /// MPAM support
    pub fn mpam(reg: u64) -> u8 {
        ((reg >> 40) & 0xF) as u8
    }
    /// AMU support
    pub fn amu(reg: u64) -> u8 {
        ((reg >> 44) & 0xF) as u8
    }
    /// DIT support
    pub fn dit(reg: u64) -> u8 {
        ((reg >> 48) & 0xF) as u8
    }
    /// RME support
    pub fn rme(reg: u64) -> u8 {
        ((reg >> 52) & 0xF) as u8
    }
    /// CSV2 support
    pub fn csv2(reg: u64) -> u8 {
        ((reg >> 56) & 0xF) as u8
    }
    /// CSV3 support
    pub fn csv3(reg: u64) -> u8 {
        ((reg >> 60) & 0xF) as u8
    }
}

/// ID_AA64MMFR0_EL1 field extraction
pub mod mmfr0 {
    /// PARange (physical address range)
    pub fn parange(reg: u64) -> u8 {
        (reg & 0xF) as u8
    }
    /// ASID bits
    pub fn asidbits(reg: u64) -> u8 {
        ((reg >> 4) & 0xF) as u8
    }
    /// Big endian support
    pub fn bigend(reg: u64) -> u8 {
        ((reg >> 8) & 0xF) as u8
    }
    /// Secure NS support
    pub fn snsmem(reg: u64) -> u8 {
        ((reg >> 12) & 0xF) as u8
    }
    /// Big endian EL0 support
    pub fn bigendel0(reg: u64) -> u8 {
        ((reg >> 16) & 0xF) as u8
    }
    /// TGran16 support
    pub fn tgran16(reg: u64) -> u8 {
        ((reg >> 20) & 0xF) as u8
    }
    /// TGran64 support
    pub fn tgran64(reg: u64) -> u8 {
        ((reg >> 24) & 0xF) as u8
    }
    /// TGran4 support
    pub fn tgran4(reg: u64) -> u8 {
        ((reg >> 28) & 0xF) as u8
    }
    /// TGran16_2 support
    pub fn tgran16_2(reg: u64) -> u8 {
        ((reg >> 32) & 0xF) as u8
    }
    /// TGran64_2 support
    pub fn tgran64_2(reg: u64) -> u8 {
        ((reg >> 36) & 0xF) as u8
    }
    /// TGran4_2 support
    pub fn tgran4_2(reg: u64) -> u8 {
        ((reg >> 40) & 0xF) as u8
    }
    /// ExS support
    pub fn exs(reg: u64) -> u8 {
        ((reg >> 44) & 0xF) as u8
    }
    /// FGT support
    pub fn fgt(reg: u64) -> u8 {
        ((reg >> 56) & 0xF) as u8
    }
    /// ECV support
    pub fn ecv(reg: u64) -> u8 {
        ((reg >> 60) & 0xF) as u8
    }
}

/// Physical address range from PARange field
pub fn physical_address_bits(parange: u8) -> u8 {
    match parange {
        0b0000 => 32,
        0b0001 => 36,
        0b0010 => 40,
        0b0011 => 42,
        0b0100 => 44,
        0b0101 => 48,
        0b0110 => 52,
        _ => 48,
    }
}

// =============================================================================
// ARM64 FEATURES STRUCT
// =============================================================================

/// AArch64-specific CPU features
#[derive(Debug, Clone, Copy, Default)]
pub struct Aarch64Features {
    // SIMD/FP
    /// FP support
    pub fp: bool,
    /// Advanced SIMD (NEON)
    pub asimd: bool,
    /// FP16 support
    pub fp16: bool,
    /// BF16 support
    pub bf16: bool,
    /// Int8 matrix multiply
    pub i8mm: bool,
    /// Dot product
    pub dotprod: bool,

    // Cryptography
    /// AES instructions
    pub aes: bool,
    /// PMULL instructions
    pub pmull: bool,
    /// SHA1 instructions
    pub sha1: bool,
    /// SHA256 instructions
    pub sha256: bool,
    /// SHA512 instructions
    pub sha512: bool,
    /// SHA3 instructions
    pub sha3: bool,
    /// SM3 instructions
    pub sm3: bool,
    /// SM4 instructions
    pub sm4: bool,

    // Atomics
    /// LSE atomics
    pub atomics: bool,
    /// LSE2 atomics
    pub lse2: bool,

    // Random number
    /// Random number (RNDR)
    pub rndr: bool,

    // Memory model
    /// CRC32 instructions
    pub crc32: bool,
    /// RCPC (Release Consistent Processor Consistent)
    pub rcpc: bool,
    /// RCPC2
    pub rcpc2: bool,

    // Pointer authentication
    /// Pointer authentication
    pub pauth: bool,
    /// Pointer authentication (QARMA5)
    pub pauth2: bool,

    // Branch target identification
    /// BTI support
    pub bti: bool,

    // Memory tagging
    /// MTE support
    pub mte: bool,
    /// MTE2 support
    pub mte2: bool,

    // SVE
    /// SVE support
    pub sve: bool,
    /// SVE2 support
    pub sve2: bool,
    /// SVE2 AES
    pub sve2_aes: bool,
    /// SVE2 bit permutations
    pub sve2_bitperm: bool,
    /// SVE2 SHA3
    pub sve2_sha3: bool,
    /// SVE2 SM4
    pub sve2_sm4: bool,

    // SME
    /// SME support
    pub sme: bool,
    /// SME2 support
    pub sme2: bool,

    // Virtualization
    /// VHE support
    pub vhe: bool,
    /// Nested virtualization
    pub nv: bool,

    // Exception handling
    /// RAS support
    pub ras: bool,
    /// RAS v1.1
    pub rasv1p1: bool,

    // Debug
    /// Debug v8.0
    pub debug_v8: bool,
    /// PMU v3
    pub pmuv3: bool,

    // Speculation
    /// SSBS support
    pub ssbs: bool,
    /// Speculative Store Bypass Safe
    pub ssbs2: bool,

    // Cache
    /// DC CVAP
    pub dcpop: bool,
    /// DC CVADP
    pub dcpodp: bool,

    // Address bits
    /// Physical address bits
    pub pa_bits: u8,
    /// Virtual address bits
    pub va_bits: u8,
    /// ASID bits (8 or 16)
    pub asid_bits: u8,

    // Granule support
    /// 4KB granule
    pub tgran4: bool,
    /// 16KB granule
    pub tgran16: bool,
    /// 64KB granule
    pub tgran64: bool,
}

impl Aarch64Features {
    /// Detect current CPU features
    pub fn detect() -> Self {
        let isar0 = read_isar0();
        let isar1 = read_isar1();
        let pfr0 = read_pfr0();
        let mmfr0 = read_mmfr0();

        let parange = mmfr0::parange(mmfr0);

        Self {
            // SIMD/FP
            fp: pfr0::fp(pfr0) != 0xF,
            asimd: pfr0::advsimd(pfr0) != 0xF,
            fp16: pfr0::fp(pfr0) >= 1,
            bf16: isar1::bf16(isar1) >= 1,
            i8mm: isar1::i8mm(isar1) >= 1,
            dotprod: isar0::dp(isar0) >= 1,

            // Crypto
            aes: isar0::aes(isar0) >= 1,
            pmull: isar0::aes(isar0) >= 2,
            sha1: isar0::sha1(isar0) >= 1,
            sha256: isar0::sha2(isar0) >= 1,
            sha512: isar0::sha2(isar0) >= 2,
            sha3: isar0::sha3(isar0) >= 1,
            sm3: isar0::sm3(isar0) >= 1,
            sm4: isar0::sm4(isar0) >= 1,

            // Atomics
            atomics: isar0::atomic(isar0) >= 2,
            lse2: isar0::atomic(isar0) >= 3,

            // Random
            rndr: isar0::rndr(isar0) >= 1,

            // Memory
            crc32: isar0::crc32(isar0) >= 1,
            rcpc: isar1::lrcpc(isar1) >= 1,
            rcpc2: isar1::lrcpc(isar1) >= 2,

            // PAuth
            pauth: isar1::apa(isar1) >= 1 || isar1::api(isar1) >= 1,
            pauth2: isar1::apa(isar1) >= 3 || isar1::api(isar1) >= 3,

            // BTI
            bti: false, // Needs ISAR2 check

            // MTE
            mte: false, // Needs PFR1 check
            mte2: false,

            // SVE
            sve: pfr0::sve(pfr0) >= 1,
            sve2: false, // Needs ZFR0 check
            sve2_aes: false,
            sve2_bitperm: false,
            sve2_sha3: false,
            sve2_sm4: false,

            // SME
            sme: false, // Needs PFR1 check
            sme2: false,

            // Virtualization
            vhe: pfr0::el2(pfr0) >= 1,
            nv: false,

            // RAS
            ras: pfr0::ras(pfr0) >= 1,
            rasv1p1: pfr0::ras(pfr0) >= 2,

            // Debug
            debug_v8: true,
            pmuv3: true,

            // Speculation
            ssbs: isar1::sb(isar1) >= 1,
            ssbs2: isar1::sb(isar1) >= 2,

            // Cache
            dcpop: isar1::dpb(isar1) >= 1,
            dcpodp: isar1::dpb(isar1) >= 2,

            // Address bits
            pa_bits: physical_address_bits(parange),
            va_bits: 48, // TODO: check for LVA
            asid_bits: if mmfr0::asidbits(mmfr0) >= 2 { 16 } else { 8 },

            // Granules
            tgran4: mmfr0::tgran4(mmfr0) != 0xF,
            tgran16: mmfr0::tgran16(mmfr0) >= 1,
            tgran64: mmfr0::tgran64(mmfr0) != 0xF,
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
    fn test_midr_fields() {
        // ARM Cortex-A72 example MIDR
        let midr = 0x410FD081u64;
        let fields = MidrFields::from_midr(midr);
        assert_eq!(fields.implementer, implementer::ARM);
        assert_eq!(fields.part_number, arm_part::CORTEX_A72);
    }

    #[test]
    fn test_physical_address_bits() {
        assert_eq!(physical_address_bits(0b0000), 32);
        assert_eq!(physical_address_bits(0b0101), 48);
        assert_eq!(physical_address_bits(0b0110), 52);
    }
}
