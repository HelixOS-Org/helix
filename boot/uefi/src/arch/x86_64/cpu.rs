//! x86_64 CPU Detection and Features
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
    Intel,
    Amd,
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
        let max_extended = cpuid(0x80000000, 0).eax;
        if max_extended >= 0x80000004 {
            for i in 0..3 {
                let result = cpuid(0x80000002 + i, 0);
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
    pub const EXTENDED_INFO: u32 = 0x80000001;
    pub const EXTENDED_BRAND_1: u32 = 0x80000002;
    pub const EXTENDED_BRAND_2: u32 = 0x80000003;
    pub const EXTENDED_BRAND_3: u32 = 0x80000004;
    pub const EXTENDED_ADDRESS: u32 = 0x80000008;
}

/// Feature bits in CPUID.1.ECX
mod feature_ecx {
    pub const SSE3: u32 = 1 << 0;
    pub const PCLMULQDQ: u32 = 1 << 1;
    pub const DTES64: u32 = 1 << 2;
    pub const MONITOR: u32 = 1 << 3;
    pub const DS_CPL: u32 = 1 << 4;
    pub const VMX: u32 = 1 << 5;
    pub const SMX: u32 = 1 << 6;
    pub const EIST: u32 = 1 << 7;
    pub const TM2: u32 = 1 << 8;
    pub const SSSE3: u32 = 1 << 9;
    pub const CNXT_ID: u32 = 1 << 10;
    pub const SDBG: u32 = 1 << 11;
    pub const FMA: u32 = 1 << 12;
    pub const CMPXCHG16B: u32 = 1 << 13;
    pub const XTPR: u32 = 1 << 14;
    pub const PDCM: u32 = 1 << 15;
    pub const PCID: u32 = 1 << 17;
    pub const DCA: u32 = 1 << 18;
    pub const SSE4_1: u32 = 1 << 19;
    pub const SSE4_2: u32 = 1 << 20;
    pub const X2APIC: u32 = 1 << 21;
    pub const MOVBE: u32 = 1 << 22;
    pub const POPCNT: u32 = 1 << 23;
    pub const TSC_DEADLINE: u32 = 1 << 24;
    pub const AES: u32 = 1 << 25;
    pub const XSAVE: u32 = 1 << 26;
    pub const OSXSAVE: u32 = 1 << 27;
    pub const AVX: u32 = 1 << 28;
    pub const F16C: u32 = 1 << 29;
    pub const RDRAND: u32 = 1 << 30;
    pub const HYPERVISOR: u32 = 1 << 31;
}

/// Feature bits in CPUID.1.EDX
mod feature_edx {
    pub const FPU: u32 = 1 << 0;
    pub const VME: u32 = 1 << 1;
    pub const DE: u32 = 1 << 2;
    pub const PSE: u32 = 1 << 3;
    pub const TSC: u32 = 1 << 4;
    pub const MSR: u32 = 1 << 5;
    pub const PAE: u32 = 1 << 6;
    pub const MCE: u32 = 1 << 7;
    pub const CX8: u32 = 1 << 8;
    pub const APIC: u32 = 1 << 9;
    pub const SEP: u32 = 1 << 11;
    pub const MTRR: u32 = 1 << 12;
    pub const PGE: u32 = 1 << 13;
    pub const MCA: u32 = 1 << 14;
    pub const CMOV: u32 = 1 << 15;
    pub const PAT: u32 = 1 << 16;
    pub const PSE36: u32 = 1 << 17;
    pub const PSN: u32 = 1 << 18;
    pub const CLFSH: u32 = 1 << 19;
    pub const DS: u32 = 1 << 21;
    pub const ACPI: u32 = 1 << 22;
    pub const MMX: u32 = 1 << 23;
    pub const FXSR: u32 = 1 << 24;
    pub const SSE: u32 = 1 << 25;
    pub const SSE2: u32 = 1 << 26;
    pub const SS: u32 = 1 << 27;
    pub const HTT: u32 = 1 << 28;
    pub const TM: u32 = 1 << 29;
    pub const IA64: u32 = 1 << 30;
    pub const PBE: u32 = 1 << 31;
}

/// Feature bits in CPUID.7.0.EBX
mod feature7_ebx {
    pub const FSGSBASE: u32 = 1 << 0;
    pub const TSC_ADJUST: u32 = 1 << 1;
    pub const SGX: u32 = 1 << 2;
    pub const BMI1: u32 = 1 << 3;
    pub const HLE: u32 = 1 << 4;
    pub const AVX2: u32 = 1 << 5;
    pub const SMEP: u32 = 1 << 7;
    pub const BMI2: u32 = 1 << 8;
    pub const ERMS: u32 = 1 << 9;
    pub const INVPCID: u32 = 1 << 10;
    pub const RTM: u32 = 1 << 11;
    pub const PQM: u32 = 1 << 12;
    pub const MPX: u32 = 1 << 14;
    pub const PQE: u32 = 1 << 15;
    pub const AVX512F: u32 = 1 << 16;
    pub const AVX512DQ: u32 = 1 << 17;
    pub const RDSEED: u32 = 1 << 18;
    pub const ADX: u32 = 1 << 19;
    pub const SMAP: u32 = 1 << 20;
    pub const AVX512IFMA: u32 = 1 << 21;
    pub const CLFLUSHOPT: u32 = 1 << 23;
    pub const CLWB: u32 = 1 << 24;
    pub const AVX512PF: u32 = 1 << 26;
    pub const AVX512ER: u32 = 1 << 27;
    pub const AVX512CD: u32 = 1 << 28;
    pub const SHA: u32 = 1 << 29;
    pub const AVX512BW: u32 = 1 << 30;
    pub const AVX512VL: u32 = 1 << 31;
}

/// Feature bits in CPUID.7.0.ECX
mod feature7_ecx {
    pub const PREFETCHWT1: u32 = 1 << 0;
    pub const AVX512VBMI: u32 = 1 << 1;
    pub const UMIP: u32 = 1 << 2;
    pub const PKU: u32 = 1 << 3;
    pub const OSPKE: u32 = 1 << 4;
    pub const AVX512VBMI2: u32 = 1 << 6;
    pub const CET_SS: u32 = 1 << 7;
    pub const GFNI: u32 = 1 << 8;
    pub const VAES: u32 = 1 << 9;
    pub const VPCLMULQDQ: u32 = 1 << 10;
    pub const AVX512VNNI: u32 = 1 << 11;
    pub const AVX512BITALG: u32 = 1 << 12;
    pub const AVX512VPOPCNTDQ: u32 = 1 << 14;
    pub const LA57: u32 = 1 << 16;
    pub const RDPID: u32 = 1 << 22;
    pub const CLDEMOTE: u32 = 1 << 25;
    pub const MOVDIRI: u32 = 1 << 27;
    pub const MOVDIR64B: u32 = 1 << 28;
}

/// Feature bits in CPUID.7.0.EDX
mod feature7_edx {
    pub const AVX5124VNNIW: u32 = 1 << 2;
    pub const AVX5124FMAPS: u32 = 1 << 3;
    pub const FSRM: u32 = 1 << 4;
    pub const AVX512VP2INTERSECT: u32 = 1 << 8;
    pub const MD_CLEAR: u32 = 1 << 10;
    pub const SERIALIZE: u32 = 1 << 14;
    pub const HYBRID: u32 = 1 << 15;
    pub const TSXLDTRK: u32 = 1 << 16;
    pub const PCONFIG: u32 = 1 << 18;
    pub const CET_IBT: u32 = 1 << 20;
    pub const AMX_BF16: u32 = 1 << 22;
    pub const AMX_TILE: u32 = 1 << 24;
    pub const AMX_INT8: u32 = 1 << 25;
    pub const SPEC_CTRL: u32 = 1 << 26;
    pub const STIBP: u32 = 1 << 27;
    pub const FLUSH_CMD: u32 = 1 << 28;
    pub const ARCH_CAPABILITIES: u32 = 1 << 29;
    pub const CORE_CAPABILITIES: u32 = 1 << 30;
    pub const SSBD: u32 = 1 << 31;
}

/// Feature bits in CPUID.0x80000001.EDX
mod ext_feature_edx {
    pub const SYSCALL: u32 = 1 << 11;
    pub const NX: u32 = 1 << 20;
    pub const PAGE1GB: u32 = 1 << 26;
    pub const RDTSCP: u32 = 1 << 27;
    pub const LM: u32 = 1 << 29;
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
    let max_extended = cpuid(0x80000000, 0).eax;

    // Get CPUID.1 features
    if max_basic >= cpuid_leaf::VERSION_FEATURES {
        let result = cpuid(cpuid_leaf::VERSION_FEATURES, 0);

        // ECX features
        features.sse3 = (result.ecx & feature_ecx::SSE3) != 0;
        features.sse4_1 = (result.ecx & feature_ecx::SSE4_1) != 0;
        features.sse4_2 = (result.ecx & feature_ecx::SSE4_2) != 0;
        features.aes = (result.ecx & feature_ecx::AES) != 0;
        features.xsave = (result.ecx & feature_ecx::XSAVE) != 0;
        features.avx = (result.ecx & feature_ecx::AVX) != 0;
        features.rdrand = (result.ecx & feature_ecx::RDRAND) != 0;
        features.x2apic = (result.ecx & feature_ecx::X2APIC) != 0;
        features.pcid = (result.ecx & feature_ecx::PCID) != 0;

        // EDX features
        features.tsc = (result.edx & feature_edx::TSC) != 0;
        features.sse = (result.edx & feature_edx::SSE) != 0;
        features.sse2 = (result.edx & feature_edx::SSE2) != 0;
    }

    // Get CPUID.7 features
    if max_basic >= cpuid_leaf::EXTENDED_FEATURES {
        let result = cpuid(cpuid_leaf::EXTENDED_FEATURES, 0);

        // EBX features
        features.fsgsbase = (result.ebx & feature7_ebx::FSGSBASE) != 0;
        features.avx2 = (result.ebx & feature7_ebx::AVX2) != 0;
        features.smep = (result.ebx & feature7_ebx::SMEP) != 0;
        features.smap = (result.ebx & feature7_ebx::SMAP) != 0;
        features.avx512 = (result.ebx & feature7_ebx::AVX512F) != 0;
        features.sha = (result.ebx & feature7_ebx::SHA) != 0;
        features.rdseed = (result.ebx & feature7_ebx::RDSEED) != 0;
        features.invpcid = (result.ebx & feature7_ebx::INVPCID) != 0;

        // ECX features
        features.umip = (result.ecx & feature7_ecx::UMIP) != 0;
        features.pku = (result.ecx & feature7_ecx::PKU) != 0;
        features.la57 = (result.ecx & feature7_ecx::LA57) != 0;

        // EDX features
        features.cet = (result.edx & feature7_edx::CET_IBT) != 0;
    }

    // Get extended features
    if max_extended >= cpuid_leaf::EXTENDED_INFO {
        let result = cpuid(cpuid_leaf::EXTENDED_INFO, 0);
        features.nx = (result.edx & ext_feature_edx::NX) != 0;
        features.page_1gb = (result.edx & ext_feature_edx::PAGE1GB) != 0;
    }

    // Get power management features
    if max_extended >= 0x80000007 {
        let result = cpuid(0x80000007, 0);
        features.tsc_invariant = (result.edx & apm_feature_edx::TSC_INVARIANT) != 0;
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
    if features.smep {
        cr4_value |= cr4::SMEP;
    }

    // Enable SMAP if supported
    if features.smap {
        cr4_value |= cr4::SMAP;
    }

    // Enable UMIP if supported
    if features.umip {
        cr4_value |= cr4::UMIP;
    }

    // Enable FSGSBASE if supported
    if features.fsgsbase {
        cr4_value |= cr4::FSGSBASE;
    }

    // Enable PKE if supported
    if features.pku {
        cr4_value |= cr4::PKE;
    }

    // Enable PCID if supported
    if features.pcid {
        cr4_value |= cr4::PCIDE;
    }

    // Enable XSAVE if supported
    if features.xsave {
        cr4_value |= cr4::OSXSAVE;
    }

    // Enable 5-level paging if supported
    if features.la57 {
        cr4_value |= cr4::LA57;
    }

    // Write CR4
    unsafe {
        write_cr4(cr4_value);
    }

    // Enable NX in EFER if supported
    if features.nx {
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
    let max_extended = cpuid(0x80000000, 0).eax;

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
    None,
    Data,
    Instruction,
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
        assert!(features.sse);
        assert!(features.sse2);
        assert!(features.tsc);
    }

    #[test]
    fn test_address_widths() {
        let (phys, virt) = get_address_widths();
        assert!(phys >= 32);
        assert!(virt >= 32);
    }
}
