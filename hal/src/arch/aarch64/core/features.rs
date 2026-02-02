//! # AArch64 CPU Feature Detection
//!
//! This module provides comprehensive CPU feature detection using
//! the ID_AA64* system registers.

use core::arch::asm;

// =============================================================================
// Feature Flags
// =============================================================================

bitflags::bitflags! {
    /// ARM64 CPU features
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CpuFeatures: u64 {
        // Processor Feature Register 0 (ID_AA64PFR0_EL1)
        /// EL0 supports AArch64
        const EL0_AARCH64 = 1 << 0;
        /// EL1 supports AArch64
        const EL1_AARCH64 = 1 << 1;
        /// EL2 implemented
        const EL2 = 1 << 2;
        /// EL3 implemented
        const EL3 = 1 << 3;
        /// FP implemented
        const FP = 1 << 4;
        /// Advanced SIMD implemented
        const ASIMD = 1 << 5;
        /// GIC system register interface
        const GIC = 1 << 6;
        /// RAS Extension
        const RAS = 1 << 7;
        /// SVE implemented
        const SVE = 1 << 8;
        /// Secure EL2
        const SEL2 = 1 << 9;
        /// Activity Monitors Extension
        const AMU = 1 << 10;
        /// Data Independent Timing
        const DIT = 1 << 11;
        /// CSV2 (Spectre mitigation)
        const CSV2 = 1 << 12;
        /// CSV3 (Spectre mitigation)
        const CSV3 = 1 << 13;

        // Instruction Set Attribute Register 0 (ID_AA64ISAR0_EL1)
        /// AES instructions
        const AES = 1 << 16;
        /// PMULL instructions
        const PMULL = 1 << 17;
        /// SHA1 instructions
        const SHA1 = 1 << 18;
        /// SHA256 instructions
        const SHA256 = 1 << 19;
        /// SHA512 instructions
        const SHA512 = 1 << 20;
        /// CRC32 instructions
        const CRC32 = 1 << 21;
        /// Atomic instructions (LSE)
        const ATOMICS = 1 << 22;
        /// RDMA instructions
        const RDM = 1 << 23;
        /// SHA3 instructions
        const SHA3 = 1 << 24;
        /// SM3 instructions
        const SM3 = 1 << 25;
        /// SM4 instructions
        const SM4 = 1 << 26;
        /// Dot product instructions
        const DP = 1 << 27;
        /// FHM instructions
        const FHM = 1 << 28;
        /// Flag manipulation
        const TS = 1 << 29;
        /// TLB range maintenance
        const TLB = 1 << 30;
        /// RNDR random number
        const RNDR = 1 << 31;

        // Memory Model Feature Register 0 (ID_AA64MMFR0_EL1)
        /// 4KB granule support
        const TGRAN4 = 1 << 40;
        /// 16KB granule support
        const TGRAN16 = 1 << 41;
        /// 64KB granule support
        const TGRAN64 = 1 << 42;
        /// 48-bit physical address
        const PA_48BIT = 1 << 43;
        /// 52-bit physical address
        const PA_52BIT = 1 << 44;
        /// Stage 2 translation
        const S2FWB = 1 << 45;
        /// Hardware dirty bit management
        const HAFDBS = 1 << 46;

        // Memory Model Feature Register 2 (ID_AA64MMFR2_EL1)
        /// ASID 16 bits
        const ASID16 = 1 << 48;
        /// Hierarchical permission disables
        const HPDS = 1 << 49;
        /// Translation table level 0 block descriptor
        const LVA = 1 << 50;
        /// IESB (Implicit Error Synchronization Barrier)
        const IESB = 1 << 51;
        /// Enhanced Translation Synchronization
        const E0PD = 1 << 52;
    }
}

impl CpuFeatures {
    /// Detect CPU features from system registers
    pub fn detect() -> Self {
        let mut features = Self::empty();

        // Read ID_AA64PFR0_EL1
        let pfr0 = read_id_aa64pfr0_el1();

        // EL0 AArch64
        if (pfr0 & 0xF) == 1 {
            features |= Self::EL0_AARCH64;
        }
        // EL1 AArch64
        if ((pfr0 >> 4) & 0xF) == 1 {
            features |= Self::EL1_AARCH64;
        }
        // EL2 implemented
        if ((pfr0 >> 8) & 0xF) != 0 {
            features |= Self::EL2;
        }
        // EL3 implemented
        if ((pfr0 >> 12) & 0xF) != 0 {
            features |= Self::EL3;
        }
        // FP
        if ((pfr0 >> 16) & 0xF) != 0xF {
            features |= Self::FP;
        }
        // Advanced SIMD
        if ((pfr0 >> 20) & 0xF) != 0xF {
            features |= Self::ASIMD;
        }
        // GIC system register interface
        if ((pfr0 >> 24) & 0xF) != 0 {
            features |= Self::GIC;
        }
        // RAS
        if ((pfr0 >> 28) & 0xF) != 0 {
            features |= Self::RAS;
        }
        // SVE
        if ((pfr0 >> 32) & 0xF) != 0 {
            features |= Self::SVE;
        }
        // SEL2
        if ((pfr0 >> 36) & 0xF) != 0 {
            features |= Self::SEL2;
        }
        // AMU
        if ((pfr0 >> 44) & 0xF) != 0 {
            features |= Self::AMU;
        }
        // DIT
        if ((pfr0 >> 48) & 0xF) != 0 {
            features |= Self::DIT;
        }
        // CSV2
        if ((pfr0 >> 56) & 0xF) != 0 {
            features |= Self::CSV2;
        }
        // CSV3
        if ((pfr0 >> 60) & 0xF) != 0 {
            features |= Self::CSV3;
        }

        // Read ID_AA64ISAR0_EL1
        let isar0 = read_id_aa64isar0_el1();

        // AES
        if ((isar0 >> 4) & 0xF) != 0 {
            features |= Self::AES;
            if ((isar0 >> 4) & 0xF) >= 2 {
                features |= Self::PMULL;
            }
        }
        // SHA1
        if ((isar0 >> 8) & 0xF) != 0 {
            features |= Self::SHA1;
        }
        // SHA256
        if ((isar0 >> 12) & 0xF) != 0 {
            features |= Self::SHA256;
        }
        // CRC32
        if ((isar0 >> 16) & 0xF) != 0 {
            features |= Self::CRC32;
        }
        // Atomics (LSE)
        if ((isar0 >> 20) & 0xF) != 0 {
            features |= Self::ATOMICS;
        }
        // RDM
        if ((isar0 >> 28) & 0xF) != 0 {
            features |= Self::RDM;
        }
        // SHA3
        if ((isar0 >> 32) & 0xF) != 0 {
            features |= Self::SHA3;
        }
        // SM3
        if ((isar0 >> 36) & 0xF) != 0 {
            features |= Self::SM3;
        }
        // SM4
        if ((isar0 >> 40) & 0xF) != 0 {
            features |= Self::SM4;
        }
        // DP
        if ((isar0 >> 44) & 0xF) != 0 {
            features |= Self::DP;
        }
        // FHM
        if ((isar0 >> 48) & 0xF) != 0 {
            features |= Self::FHM;
        }
        // TS
        if ((isar0 >> 52) & 0xF) != 0 {
            features |= Self::TS;
        }
        // TLB
        if ((isar0 >> 56) & 0xF) != 0 {
            features |= Self::TLB;
        }
        // RNDR
        if ((isar0 >> 60) & 0xF) != 0 {
            features |= Self::RNDR;
        }

        // Read ID_AA64MMFR0_EL1
        let mmfr0 = read_id_aa64mmfr0_el1();

        // Physical address range
        let pa_range = mmfr0 & 0xF;
        if pa_range >= 5 {
            features |= Self::PA_48BIT;
        }
        if pa_range >= 6 {
            features |= Self::PA_52BIT;
        }

        // Granule support
        // TGran4 (bits 31:28): 0 = supported, 0xF = not supported
        if ((mmfr0 >> 28) & 0xF) != 0xF {
            features |= Self::TGRAN4;
        }
        // TGran64 (bits 27:24): 0 = supported, 0xF = not supported
        if ((mmfr0 >> 24) & 0xF) != 0xF {
            features |= Self::TGRAN64;
        }
        // TGran16 (bits 23:20): 1 or 2 = supported
        if ((mmfr0 >> 20) & 0xF) != 0 {
            features |= Self::TGRAN16;
        }

        // HAFDBS
        if ((mmfr0 >> 40) & 0xF) != 0 {
            features |= Self::HAFDBS;
        }

        // Read ID_AA64MMFR2_EL1
        let mmfr2 = read_id_aa64mmfr2_el1();

        // ASID 16
        if ((mmfr2 >> 4) & 0xF) != 0 {
            features |= Self::ASID16;
        }

        // LVA
        if ((mmfr2 >> 16) & 0xF) != 0 {
            features |= Self::LVA;
        }

        // E0PD
        if ((mmfr2 >> 60) & 0xF) != 0 {
            features |= Self::E0PD;
        }

        features
    }

    /// Create an empty feature set
    pub const fn empty() -> Self {
        Self::from_bits_truncate(0)
    }

    /// Check if a feature is supported
    pub fn has(&self, feature: ArmFeature) -> bool {
        match feature {
            ArmFeature::Fp => self.contains(Self::FP),
            ArmFeature::Asimd => self.contains(Self::ASIMD),
            ArmFeature::Atomics => self.contains(Self::ATOMICS),
            ArmFeature::Sve => self.contains(Self::SVE),
            ArmFeature::Aes => self.contains(Self::AES),
            ArmFeature::Sha256 => self.contains(Self::SHA256),
            ArmFeature::Crc32 => self.contains(Self::CRC32),
            ArmFeature::Rndr => self.contains(Self::RNDR),
            ArmFeature::Tgran4 => self.contains(Self::TGRAN4),
            ArmFeature::Tgran16 => self.contains(Self::TGRAN16),
            ArmFeature::Tgran64 => self.contains(Self::TGRAN64),
            ArmFeature::El2 => self.contains(Self::EL2),
            ArmFeature::El3 => self.contains(Self::EL3),
            ArmFeature::Gic => self.contains(Self::GIC),
        }
    }
}

/// Individual ARM features for querying
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmFeature {
    /// Floating point
    Fp,
    /// Advanced SIMD (NEON)
    Asimd,
    /// Large System Extensions (atomics)
    Atomics,
    /// Scalable Vector Extension
    Sve,
    /// AES instructions
    Aes,
    /// SHA-256 instructions
    Sha256,
    /// CRC32 instructions
    Crc32,
    /// Random number generation
    Rndr,
    /// 4KB granule
    Tgran4,
    /// 16KB granule
    Tgran16,
    /// 64KB granule
    Tgran64,
    /// EL2 implemented
    El2,
    /// EL3 implemented
    El3,
    /// GIC system register interface
    Gic,
}

// =============================================================================
// CPU Identification
// =============================================================================

/// CPU implementer codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CpuImplementer {
    /// ARM Ltd.
    Arm          = 0x41,
    /// Broadcom
    Broadcom     = 0x42,
    /// Cavium (Marvell)
    Cavium       = 0x43,
    /// DEC (now part of HP)
    Dec          = 0x44,
    /// Fujitsu
    Fujitsu      = 0x46,
    /// HiSilicon
    HiSilicon    = 0x48,
    /// Infineon
    Infineon     = 0x49,
    /// Motorola/Freescale
    Motorola     = 0x4D,
    /// NVIDIA
    Nvidia       = 0x4E,
    /// Applied Micro
    AppliedMicro = 0x50,
    /// Qualcomm
    Qualcomm     = 0x51,
    /// Marvell
    Marvell      = 0x56,
    /// Intel
    Intel        = 0x69,
    /// Ampere
    Ampere       = 0xC0,
    /// Unknown
    Unknown      = 0xFF,
}

impl From<u8> for CpuImplementer {
    fn from(value: u8) -> Self {
        match value {
            0x41 => Self::Arm,
            0x42 => Self::Broadcom,
            0x43 => Self::Cavium,
            0x44 => Self::Dec,
            0x46 => Self::Fujitsu,
            0x48 => Self::HiSilicon,
            0x49 => Self::Infineon,
            0x4D => Self::Motorola,
            0x4E => Self::Nvidia,
            0x50 => Self::AppliedMicro,
            0x51 => Self::Qualcomm,
            0x56 => Self::Marvell,
            0x69 => Self::Intel,
            0xC0 => Self::Ampere,
            _ => Self::Unknown,
        }
    }
}

/// CPU identification information
#[derive(Debug, Clone, Copy)]
pub struct CpuId {
    /// Implementer
    pub implementer: CpuImplementer,
    /// Variant
    pub variant: u8,
    /// Architecture
    pub architecture: u8,
    /// Part number
    pub part_num: u16,
    /// Revision
    pub revision: u8,
}

impl CpuId {
    /// Read CPU identification
    pub fn read() -> Self {
        let midr = read_midr_el1();
        Self {
            implementer: CpuImplementer::from(((midr >> 24) & 0xFF) as u8),
            variant: ((midr >> 20) & 0xF) as u8,
            architecture: ((midr >> 16) & 0xF) as u8,
            part_num: ((midr >> 4) & 0xFFF) as u16,
            revision: (midr & 0xF) as u8,
        }
    }
}

// =============================================================================
// System Register Readers
// =============================================================================

/// Read ID_AA64PFR0_EL1
#[inline]
pub fn read_id_aa64pfr0_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ID_AA64PFR0_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read ID_AA64PFR1_EL1
#[inline]
pub fn read_id_aa64pfr1_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ID_AA64PFR1_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read ID_AA64ISAR0_EL1
#[inline]
pub fn read_id_aa64isar0_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ID_AA64ISAR0_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read ID_AA64ISAR1_EL1
#[inline]
pub fn read_id_aa64isar1_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ID_AA64ISAR1_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read ID_AA64MMFR0_EL1
#[inline]
pub fn read_id_aa64mmfr0_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ID_AA64MMFR0_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read ID_AA64MMFR1_EL1
#[inline]
pub fn read_id_aa64mmfr1_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ID_AA64MMFR1_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read ID_AA64MMFR2_EL1
#[inline]
pub fn read_id_aa64mmfr2_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, ID_AA64MMFR2_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read MIDR_EL1
#[inline]
pub fn read_midr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, MIDR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read MPIDR_EL1
#[inline]
pub fn read_mpidr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, MPIDR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

/// Read REVIDR_EL1
#[inline]
pub fn read_revidr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, REVIDR_EL1", out(reg) value, options(nomem, nostack, preserves_flags));
    }
    value
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Get maximum supported physical address bits
pub fn max_physical_address_bits() -> u8 {
    let mmfr0 = read_id_aa64mmfr0_el1();
    let pa_range = mmfr0 & 0xF;
    match pa_range {
        0 => 32,
        1 => 36,
        2 => 40,
        3 => 42,
        4 => 44,
        5 => 48,
        6 => 52,
        _ => 48, // Default
    }
}

/// Get maximum supported virtual address bits
pub fn max_virtual_address_bits() -> u8 {
    let mmfr2 = read_id_aa64mmfr2_el1();
    // VARange field
    let va_range = (mmfr2 >> 16) & 0xF;
    match va_range {
        0 => 48,
        1 => 52,
        _ => 48,
    }
}

/// Check if 4KB granule is supported
pub fn supports_4kb_granule() -> bool {
    let mmfr0 = read_id_aa64mmfr0_el1();
    ((mmfr0 >> 28) & 0xF) != 0xF
}

/// Check if 16KB granule is supported
pub fn supports_16kb_granule() -> bool {
    let mmfr0 = read_id_aa64mmfr0_el1();
    ((mmfr0 >> 20) & 0xF) != 0
}

/// Check if 64KB granule is supported
pub fn supports_64kb_granule() -> bool {
    let mmfr0 = read_id_aa64mmfr0_el1();
    ((mmfr0 >> 24) & 0xF) != 0xF
}

/// Check if LSE atomics are supported
pub fn supports_atomics() -> bool {
    let isar0 = read_id_aa64isar0_el1();
    ((isar0 >> 20) & 0xF) >= 2
}

/// Check if hardware random number generator is supported
pub fn supports_rndr() -> bool {
    let isar0 = read_id_aa64isar0_el1();
    ((isar0 >> 60) & 0xF) != 0
}
