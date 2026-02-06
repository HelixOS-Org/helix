//! `x86_64` CPU Detection and Features
//!
//! CPU feature detection using CPUID.

use super::{
    cpuid, cr0, cr4, efer, read_cr0, read_cr4, read_efer, write_cr0, write_cr4, write_efer,
};
use crate::arch::CpuFeatures;
use crate::error::Result;

// =============================================================================
// CPU IDENTIFICATION
// =============================================================================

/// CPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    /// Intel processor
    Intel,
    /// AMD processor
    Amd,
    /// Unknown vendor
    Unknown,
}

impl CpuVendor {
    /// Detect vendor from CPUID
    pub fn detect() -> Self {
        let result = cpuid(0, 0);

        // Vendor string in EBX, EDX, ECX
        let vendor = [
            result.ebx.to_le_bytes(),
            result.edx.to_le_bytes(),
            result.ecx.to_le_bytes(),
        ];

        let vendor_bytes: [u8; 12] = [
            vendor[0][0],
            vendor[0][1],
            vendor[0][2],
            vendor[0][3],
            vendor[1][0],
            vendor[1][1],
            vendor[1][2],
            vendor[1][3],
            vendor[2][0],
            vendor[2][1],
            vendor[2][2],
            vendor[2][3],
        ];

        match &vendor_bytes {
            b"GenuineIntel" => CpuVendor::Intel,
            b"AuthenticAMD" => CpuVendor::Amd,
            _ => CpuVendor::Unknown,
        }
    }
}

/// CPU model information
#[derive(Debug, Clone, Copy)]
pub struct CpuModel {
    /// Family
    pub family: u32,
    /// Model
    pub model: u32,
    /// Stepping
    pub stepping: u32,
    /// Extended family
    pub ext_family: u32,
    /// Extended model
    pub ext_model: u32,
    /// Brand string
    pub brand_string: [u8; 48],
}

impl Default for CpuModel {
    fn default() -> Self {
        Self {
            family: 0,
            model: 0,
            stepping: 0,
            ext_family: 0,
            ext_model: 0,
            brand_string: [0u8; 48],
        }
    }
}

impl CpuModel {
    /// Detect CPU model
    pub fn detect() -> Self {
        let result = cpuid(1, 0);

        let stepping = result.eax & 0xF;
        let model = (result.eax >> 4) & 0xF;
        let family = (result.eax >> 8) & 0xF;
        let ext_model = (result.eax >> 16) & 0xF;
        let ext_family = (result.eax >> 20) & 0xFF;

        // Calculate effective family/model
        let effective_family = if family == 0xF {
            family + ext_family
        } else {
            family
        };

        let effective_model = if family == 0x6 || family == 0xF {
            (ext_model << 4) | model
        } else {
            model
        };

        let mut cpu_model = Self {
            family: effective_family,
            model: effective_model,
            stepping,
            ext_family,
            ext_model,
            brand_string: [0; 48],
        };

        // Get brand string if available
        let max_extended = cpuid(0x8000_0000, 0).eax;
        if max_extended >= 0x8000_0004 {
            for i in 0..3 {
                let result = cpuid(0x8000_0002 + i, 0);
                let offset = (i * 16) as usize;

                cpu_model.brand_string[offset..offset + 4]
                    .copy_from_slice(&result.eax.to_le_bytes());
                cpu_model.brand_string[offset + 4..offset + 8]
                    .copy_from_slice(&result.ebx.to_le_bytes());
                cpu_model.brand_string[offset + 8..offset + 12]
                    .copy_from_slice(&result.ecx.to_le_bytes());
                cpu_model.brand_string[offset + 12..offset + 16]
                    .copy_from_slice(&result.edx.to_le_bytes());
            }
        }

        cpu_model
    }

    /// Get brand string as str
    pub fn brand_str(&self) -> &str {
        let end = self.brand_string.iter().position(|&c| c == 0).unwrap_or(48);

        core::str::from_utf8(&self.brand_string[..end])
            .unwrap_or("")
            .trim()
    }
}

// =============================================================================
// FEATURE DETECTION
// =============================================================================

/// CPUID leaves for feature detection
mod cpuid_leaf {
    pub const BASIC: u32 = 0;
    pub const VERSION_FEATURES: u32 = 1;
    pub const EXTENDED_FEATURES: u32 = 7;
    pub const EXTENDED_INFO: u32 = 0x8000_0001;
    pub const _EXTENDED_BRAND_1: u32 = 0x8000_0002;
    pub const _EXTENDED_BRAND_2: u32 = 0x8000_0003;
    pub const _EXTENDED_BRAND_3: u32 = 0x8000_0004;
    pub const EXTENDED_ADDRESS: u32 = 0x8000_0008;
}

/// Feature bits in CPUID.1.ECX
mod feature_ecx {
    pub const SSE3: u32 = 1 << 0;
    pub const _PCLMULQDQ: u32 = 1 << 1;
    pub const _DTES64: u32 = 1 << 2;
    pub const _MONITOR: u32 = 1 << 3;
    pub const _DS_CPL: u32 = 1 << 4;
    pub const _VMX: u32 = 1 << 5;
    pub const _SMX: u32 = 1 << 6;
    pub const _EIST: u32 = 1 << 7;
    pub const _TM2: u32 = 1 << 8;
    pub const _SSSE3: u32 = 1 << 9;
    pub const _CNXT_ID: u32 = 1 << 10;
    pub const _SDBG: u32 = 1 << 11;
    pub const _FMA: u32 = 1 << 12;
    pub const _CMPXCHG16B: u32 = 1 << 13;
    pub const _XTPR: u32 = 1 << 14;
    pub const _PDCM: u32 = 1 << 15;
    pub const PCID: u32 = 1 << 17;
    pub const _DCA: u32 = 1 << 18;
    pub const SSE4_1: u32 = 1 << 19;
    pub const SSE4_2: u32 = 1 << 20;
    pub const X2APIC: u32 = 1 << 21;
    pub const _MOVBE: u32 = 1 << 22;
    pub const _POPCNT: u32 = 1 << 23;
    pub const _TSC_DEADLINE: u32 = 1 << 24;
    pub const AES: u32 = 1 << 25;
    pub const XSAVE: u32 = 1 << 26;
    pub const _OSXSAVE: u32 = 1 << 27;
    pub const AVX: u32 = 1 << 28;
    pub const _F16C: u32 = 1 << 29;
    pub const RDRAND: u32 = 1 << 30;
    pub const _HYPERVISOR: u32 = 1 << 31;
}

/// Feature bits in CPUID.1.EDX
mod feature_edx {
    pub const _FPU: u32 = 1 << 0;
    pub const _VME: u32 = 1 << 1;
    pub const _DE: u32 = 1 << 2;
    pub const _PSE: u32 = 1 << 3;
    pub const TSC: u32 = 1 << 4;
    pub const _MSR: u32 = 1 << 5;
    pub const _PAE: u32 = 1 << 6;
    pub const _MCE: u32 = 1 << 7;
    pub const _CX8: u32 = 1 << 8;
    pub const _APIC: u32 = 1 << 9;
    pub const _SEP: u32 = 1 << 11;
    pub const _MTRR: u32 = 1 << 12;
    pub const _PGE: u32 = 1 << 13;
    pub const _MCA: u32 = 1 << 14;
    pub const _CMOV: u32 = 1 << 15;
    pub const _PAT: u32 = 1 << 16;
    pub const _PSE36: u32 = 1 << 17;
    pub const _PSN: u32 = 1 << 18;
    pub const _CLFSH: u32 = 1 << 19;
    pub const _DS: u32 = 1 << 21;
    pub const _ACPI: u32 = 1 << 22;
    pub const _MMX: u32 = 1 << 23;
    pub const _FXSR: u32 = 1 << 24;
    pub const SSE: u32 = 1 << 25;
    pub const SSE2: u32 = 1 << 26;
    pub const _SS: u32 = 1 << 27;
    pub const _HTT: u32 = 1 << 28;
    pub const _TM: u32 = 1 << 29;
    pub const _IA64: u32 = 1 << 30;
    pub const _PBE: u32 = 1 << 31;
}

/// Feature bits in CPUID.7.0.EBX
mod feature7_ebx {
    pub const FSGSBASE: u32 = 1 << 0;
    pub const _TSC_ADJUST: u32 = 1 << 1;
    pub const _SGX: u32 = 1 << 2;
    pub const _BMI1: u32 = 1 << 3;
    pub const _HLE: u32 = 1 << 4;
    pub const AVX2: u32 = 1 << 5;
    pub const SMEP: u32 = 1 << 7;
    pub const _BMI2: u32 = 1 << 8;
    pub const _ERMS: u32 = 1 << 9;
    pub const INVPCID: u32 = 1 << 10;
    pub const _RTM: u32 = 1 << 11;
    pub const _PQM: u32 = 1 << 12;
    pub const _MPX: u32 = 1 << 14;
    pub const _PQE: u32 = 1 << 15;
    pub const AVX512F: u32 = 1 << 16;
    pub const _AVX512DQ: u32 = 1 << 17;
    pub const RDSEED: u32 = 1 << 18;
    pub const _ADX: u32 = 1 << 19;
    pub const SMAP: u32 = 1 << 20;
    pub const _AVX512IFMA: u32 = 1 << 21;
    pub const _CLFLUSHOPT: u32 = 1 << 23;
    pub const _CLWB: u32 = 1 << 24;
    pub const _AVX512PF: u32 = 1 << 26;
    pub const _AVX512ER: u32 = 1 << 27;
    pub const _AVX512CD: u32 = 1 << 28;
    pub const SHA: u32 = 1 << 29;
    pub const _AVX512BW: u32 = 1 << 30;
    pub const _AVX512VL: u32 = 1 << 31;
}

/// Feature bits in CPUID.7.0.ECX
mod feature7_ecx {
    pub const _PREFETCHWT1: u32 = 1 << 0;
    pub const _AVX512VBMI: u32 = 1 << 1;
    pub const UMIP: u32 = 1 << 2;
    pub const PKU: u32 = 1 << 3;
    pub const _OSPKE: u32 = 1 << 4;
    pub const _AVX512VBMI2: u32 = 1 << 6;
    pub const _CET_SS: u32 = 1 << 7;
    pub const _GFNI: u32 = 1 << 8;
    pub const _VAES: u32 = 1 << 9;
    pub const _VPCLMULQDQ: u32 = 1 << 10;
    pub const _AVX512VNNI: u32 = 1 << 11;
    pub const _AVX512BITALG: u32 = 1 << 12;
    pub const _AVX512VPOPCNTDQ: u32 = 1 << 14;
    pub const LA57: u32 = 1 << 16;
    pub const _RDPID: u32 = 1 << 22;
    pub const _CLDEMOTE: u32 = 1 << 25;
    pub const _MOVDIRI: u32 = 1 << 27;
    pub const _MOVDIR64B: u32 = 1 << 28;
}

/// Feature bits in CPUID.7.0.EDX
mod feature7_edx {
    pub const _AVX5124VNNIW: u32 = 1 << 2;
    pub const _AVX5124FMAPS: u32 = 1 << 3;
    pub const _FSRM: u32 = 1 << 4;
    pub const _AVX512VP2INTERSECT: u32 = 1 << 8;
    pub const _MD_CLEAR: u32 = 1 << 10;
    pub const _SERIALIZE: u32 = 1 << 14;
    pub const _HYBRID: u32 = 1 << 15;
    pub const _TSXLDTRK: u32 = 1 << 16;
    pub const _PCONFIG: u32 = 1 << 18;
    pub const CET_IBT: u32 = 1 << 20;
    pub const _AMX_BF16: u32 = 1 << 22;
    pub const _AMX_TILE: u32 = 1 << 24;
    pub const _AMX_INT8: u32 = 1 << 25;
    pub const _SPEC_CTRL: u32 = 1 << 26;
    pub const _STIBP: u32 = 1 << 27;
    pub const _FLUSH_CMD: u32 = 1 << 28;
    pub const _ARCH_CAPABILITIES: u32 = 1 << 29;
    pub const _CORE_CAPABILITIES: u32 = 1 << 30;
    pub const _SSBD: u32 = 1 << 31;
}

/// Feature bits in CPUID.0x80000001.EDX
mod ext_feature_edx {
    pub const _SYSCALL: u32 = 1 << 11;
    pub const NX: u32 = 1 << 20;
    pub const PAGE1GB: u32 = 1 << 26;
    pub const _RDTSCP: u32 = 1 << 27;
    pub const _LM: u32 = 1 << 29;
}

/// Feature bits in CPUID.0x80000007.EDX (Advanced Power Management)
mod apm_feature_edx {
    pub const TSC_INVARIANT: u32 = 1 << 8;
}

/// Detect CPU features
pub fn detect_features() -> CpuFeatures {
    let mut features = CpuFeatures::default();

    // Check max CPUID level
    let max_basic = cpuid(cpuid_leaf::BASIC, 0).eax;
    let max_extended = cpuid(0x8000_0000, 0).eax;

    // Get CPUID.1 features
    if max_basic >= cpuid_leaf::VERSION_FEATURES {
        let result = cpuid(cpuid_leaf::VERSION_FEATURES, 0);

        // ECX features
        if (result.ecx & feature_ecx::SSE3) != 0 {
            features.insert(CpuFeatures::SSE3);
        }
        if (result.ecx & feature_ecx::SSE4_1) != 0 {
            features.insert(CpuFeatures::SSE4_1);
        }
        if (result.ecx & feature_ecx::SSE4_2) != 0 {
            features.insert(CpuFeatures::SSE4_2);
        }
        if (result.ecx & feature_ecx::AES) != 0 {
            features.insert(CpuFeatures::AES);
        }
        if (result.ecx & feature_ecx::XSAVE) != 0 {
            features.insert(CpuFeatures::XSAVE);
        }
        if (result.ecx & feature_ecx::AVX) != 0 {
            features.insert(CpuFeatures::AVX);
        }
        if (result.ecx & feature_ecx::RDRAND) != 0 {
            features.insert(CpuFeatures::RDRAND);
        }
        if (result.ecx & feature_ecx::X2APIC) != 0 {
            features.insert(CpuFeatures::X2APIC);
        }
        if (result.ecx & feature_ecx::PCID) != 0 {
            features.insert(CpuFeatures::PCID);
        }

        // EDX features
        if (result.edx & feature_edx::TSC) != 0 {
            features.insert(CpuFeatures::TSC);
        }
        if (result.edx & feature_edx::SSE) != 0 {
            features.insert(CpuFeatures::SSE);
        }
        if (result.edx & feature_edx::SSE2) != 0 {
            features.insert(CpuFeatures::SSE2);
        }
    }

    // Get CPUID.7 features
    if max_basic >= cpuid_leaf::EXTENDED_FEATURES {
        let result = cpuid(cpuid_leaf::EXTENDED_FEATURES, 0);

        // EBX features
        if (result.ebx & feature7_ebx::FSGSBASE) != 0 {
            features.insert(CpuFeatures::FSGSBASE);
        }
        if (result.ebx & feature7_ebx::AVX2) != 0 {
            features.insert(CpuFeatures::AVX2);
        }
        if (result.ebx & feature7_ebx::SMEP) != 0 {
            features.insert(CpuFeatures::SMEP);
        }
        if (result.ebx & feature7_ebx::SMAP) != 0 {
            features.insert(CpuFeatures::SMAP);
        }
        if (result.ebx & feature7_ebx::AVX512F) != 0 {
            features.insert(CpuFeatures::AVX512);
        }
        if (result.ebx & feature7_ebx::SHA) != 0 {
            features.insert(CpuFeatures::SHA);
        }
        if (result.ebx & feature7_ebx::RDSEED) != 0 {
            features.insert(CpuFeatures::RDSEED);
        }
        if (result.ebx & feature7_ebx::INVPCID) != 0 {
            features.insert(CpuFeatures::INVPCID);
        }

        // ECX features
        if (result.ecx & feature7_ecx::UMIP) != 0 {
            features.insert(CpuFeatures::UMIP);
        }
        if (result.ecx & feature7_ecx::PKU) != 0 {
            features.insert(CpuFeatures::PKU);
        }
        if (result.ecx & feature7_ecx::LA57) != 0 {
            features.insert(CpuFeatures::LA57);
        }

        // EDX features
        if (result.edx & feature7_edx::CET_IBT) != 0 {
            features.insert(CpuFeatures::CET);
        }
    }

    // Get extended features
    if max_extended >= cpuid_leaf::EXTENDED_INFO {
        let result = cpuid(cpuid_leaf::EXTENDED_INFO, 0);
        if (result.edx & ext_feature_edx::NX) != 0 {
            features.insert(CpuFeatures::NX);
        }
        if (result.edx & ext_feature_edx::PAGE1GB) != 0 {
            features.insert(CpuFeatures::PAGE_1GB);
        }
    }

    // Get power management features
    if max_extended >= 0x8000_0007 {
        let result = cpuid(0x8000_0007, 0);
        if (result.edx & apm_feature_edx::TSC_INVARIANT) != 0 {
            features.insert(CpuFeatures::TSC_INVARIANT);
        }
    }

    features
}

// =============================================================================
// FEATURE ENABLING
// =============================================================================

/// Enable CPU features
pub fn enable_features(features: &CpuFeatures) -> Result<()> {
    let mut cr4_value = read_cr4();

    // Enable SSE/SSE2 (required for x86_64)
    let mut cr0_value = read_cr0();
    cr0_value &= !(cr0::EM | cr0::TS); // Clear emulation, task switched
    cr0_value |= cr0::MP; // Monitor coprocessor
    unsafe {
        write_cr0(cr0_value);
    }

    // Enable OSFXSR and OSXMMEXCPT
    cr4_value |= cr4::OSFXSR | cr4::OSXMMEXCPT;

    // Enable SMEP if supported
    if features.contains(CpuFeatures::SMEP) {
        cr4_value |= cr4::SMEP;
    }

    // Enable SMAP if supported
    if features.contains(CpuFeatures::SMAP) {
        cr4_value |= cr4::SMAP;
    }

    // Enable UMIP if supported
    if features.contains(CpuFeatures::UMIP) {
        cr4_value |= cr4::UMIP;
    }

    // Enable FSGSBASE if supported
    if features.contains(CpuFeatures::FSGSBASE) {
        cr4_value |= cr4::FSGSBASE;
    }

    // Enable PKE if supported
    if features.contains(CpuFeatures::PKU) {
        cr4_value |= cr4::PKE;
    }

    // Enable PCID if supported
    if features.contains(CpuFeatures::PCID) {
        cr4_value |= cr4::PCIDE;
    }

    // Enable XSAVE if supported
    if features.contains(CpuFeatures::XSAVE) {
        cr4_value |= cr4::OSXSAVE;
    }

    // Enable 5-level paging if supported
    if features.contains(CpuFeatures::LA57) {
        cr4_value |= cr4::LA57;
    }

    // Write CR4
    unsafe {
        write_cr4(cr4_value);
    }

    // Enable NX in EFER if supported
    if features.contains(CpuFeatures::NX) {
        let mut efer_value = read_efer();
        efer_value |= efer::NXE;
        unsafe {
            write_efer(efer_value);
        }
    }

    Ok(())
}

// =============================================================================
// ADDRESS WIDTH
// =============================================================================

/// Get physical and virtual address widths
pub fn get_address_widths() -> (u8, u8) {
    let max_extended = cpuid(0x8000_0000, 0).eax;

    if max_extended >= cpuid_leaf::EXTENDED_ADDRESS {
        let result = cpuid(cpuid_leaf::EXTENDED_ADDRESS, 0);
        let phys_bits = (result.eax & 0xFF) as u8;
        let virt_bits = ((result.eax >> 8) & 0xFF) as u8;
        (phys_bits, virt_bits)
    } else {
        // Default values
        (36, 48)
    }
}

// =============================================================================
// CACHE INFO
// =============================================================================

/// Cache type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType {
    /// No cache
    None,
    /// Data cache
    Data,
    /// Instruction cache
    Instruction,
    /// Unified cache (data and instruction)
    Unified,
}

/// Cache info
#[derive(Debug, Clone, Copy)]
pub struct CacheInfo {
    /// Cache level (1, 2, 3)
    pub level: u8,
    /// Cache type
    pub cache_type: CacheType,
    /// Line size in bytes
    pub line_size: u32,
    /// Total size in bytes
    pub size: u32,
    /// Number of ways
    pub ways: u32,
    /// Number of sets
    pub sets: u32,
}

/// Get cache information
pub fn get_cache_info() -> [Option<CacheInfo>; 4] {
    let mut caches = [None; 4];
    let max_basic = cpuid(cpuid_leaf::BASIC, 0).eax;

    if max_basic < 4 {
        return caches;
    }

    for i in 0..4 {
        let result = cpuid(4, i);
        let cache_type = result.eax & 0x1F;

        if cache_type == 0 {
            break;
        }

        let level = ((result.eax >> 5) & 0x7) as u8;
        let line_size = (result.ebx & 0xFFF) + 1;
        let partitions = ((result.ebx >> 12) & 0x3FF) + 1;
        let ways = ((result.ebx >> 22) & 0x3FF) + 1;
        let sets = result.ecx + 1;

        let size = line_size * partitions * ways * sets;

        let ct = match cache_type {
            1 => CacheType::Data,
            2 => CacheType::Instruction,
            3 => CacheType::Unified,
            _ => CacheType::None,
        };

        caches[i as usize] = Some(CacheInfo {
            level,
            cache_type: ct,
            line_size,
            size,
            ways,
            sets,
        });
    }

    caches
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_vendor_detection() {
        let vendor = CpuVendor::detect();
        assert!(
            vendor == CpuVendor::Intel || vendor == CpuVendor::Amd || vendor == CpuVendor::Unknown
        );
    }

    #[test]
    fn test_cpu_model() {
        let model = CpuModel::detect();
        assert!(model.family > 0);
    }

    #[test]
    fn test_feature_detection() {
        let features = detect_features();
        // These should always be true on x86_64
        assert!(features.contains(CpuFeatures::SSE));
        assert!(features.contains(CpuFeatures::SSE2));
        assert!(features.contains(CpuFeatures::TSC));
    }

    #[test]
    fn test_address_widths() {
        let (phys, virt) = get_address_widths();
        assert!(phys >= 32);
        assert!(virt >= 32);
    }
}
