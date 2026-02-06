//! # CPUID Framework
//!
//! Complete CPUID enumeration for x86_64 processors.
//! Supports Intel, AMD, and compatible processors.
//!
//! ## Overview
//!
//! CPUID is the primary mechanism for detecting CPU features, capabilities,
//! and topology information. This module provides:
//!
//! - Raw CPUID access
//! - Parsed vendor information
//! - Feature flags extraction
//! - Cache/TLB information
//! - Topology enumeration
//!
//! ## Usage
//!
//! ```rust,no_run
//! use helix_hal::arch::x86_64::core::cpuid::CpuId;
//!
//! let cpuid = CpuId::new();
//!
//! // Check vendor
//! println!("Vendor: {:?}", cpuid.vendor());
//!
//! // Check features
//! if cpuid.has_feature(Feature::AVX512F) {
//!     // Use AVX-512
//! }
//! ```

// In inline assembly, we intentionally use rbx with a 32-bit output
// since we save/restore the full register but only use the lower bits
#![allow(asm_sub_register)]

use core::arch::asm;

// =============================================================================
// RAW CPUID ACCESS
// =============================================================================

/// Raw CPUID result
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct CpuIdResult {
    /// EAX register value
    pub eax: u32,
    /// EBX register value
    pub ebx: u32,
    /// ECX register value
    pub ecx: u32,
    /// EDX register value
    pub edx: u32,
}

impl CpuIdResult {
    /// Create a zeroed result
    pub const fn zero() -> Self {
        Self {
            eax: 0,
            ebx: 0,
            ecx: 0,
            edx: 0,
        }
    }
}

/// Execute CPUID instruction
///
/// # Arguments
/// * `leaf` - CPUID leaf (EAX input)
///
/// # Returns
/// Raw CPUID result with EAX, EBX, ECX, EDX values
#[inline]
pub fn cpuid(leaf: u32) -> CpuIdResult {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    unsafe {
        asm!(
            "mov {tmp}, rbx",
            "cpuid",
            "xchg {tmp}, rbx",
            tmp = out(reg) ebx,
            inout("eax") leaf => eax,
            inout("ecx") 0u32 => ecx,
            out("edx") edx,
            options(nomem, nostack, preserves_flags)
        );
    }
    CpuIdResult { eax, ebx, ecx, edx }
}

/// Execute CPUID instruction with subleaf
///
/// # Arguments
/// * `leaf` - CPUID leaf (EAX input)
/// * `subleaf` - CPUID subleaf (ECX input)
///
/// # Returns
/// Raw CPUID result with EAX, EBX, ECX, EDX values
#[inline]
pub fn cpuid_count(leaf: u32, subleaf: u32) -> CpuIdResult {
    let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
    unsafe {
        asm!(
            "mov {tmp}, rbx",
            "cpuid",
            "xchg {tmp}, rbx",
            tmp = out(reg) ebx,
            inout("eax") leaf => eax,
            inout("ecx") subleaf => ecx,
            out("edx") edx,
            options(nomem, nostack, preserves_flags)
        );
    }
    CpuIdResult { eax, ebx, ecx, edx }
}

// =============================================================================
// CPUID LEAVES
// =============================================================================

/// CPUID leaf numbers
pub mod leaf {
    /// Basic CPUID Information (vendor, max leaf)
    pub const VENDOR: u32 = 0x00;
    /// Processor Info and Feature Bits
    pub const FEATURES: u32 = 0x01;
    /// Cache and TLB Descriptor
    pub const CACHE_TLB: u32 = 0x02;
    /// Processor Serial Number (deprecated)
    pub const SERIAL: u32 = 0x03;
    /// Deterministic Cache Parameters
    pub const CACHE_PARAMS: u32 = 0x04;
    /// MONITOR/MWAIT Parameters
    pub const MWAIT: u32 = 0x05;
    /// Thermal and Power Management
    pub const THERMAL: u32 = 0x06;
    /// Structured Extended Feature Flags
    pub const STRUCT_EXT_FEATURES: u32 = 0x07;
    /// Direct Cache Access Information
    pub const DCA: u32 = 0x09;
    /// Architectural Performance Monitoring
    pub const PERF_MON: u32 = 0x0A;
    /// Extended Topology Enumeration
    pub const TOPOLOGY: u32 = 0x0B;
    /// Processor Extended State Enumeration
    pub const XSAVE: u32 = 0x0D;
    /// Intel RDT Monitoring
    pub const RDT_MONITOR: u32 = 0x0F;
    /// Intel RDT Allocation
    pub const RDT_ALLOC: u32 = 0x10;
    /// Intel SGX
    pub const SGX: u32 = 0x12;
    /// Intel Processor Trace
    pub const PT: u32 = 0x14;
    /// Time Stamp Counter and Nominal Core Crystal Clock
    pub const TSC_FREQ: u32 = 0x15;
    /// Processor Frequency Information
    pub const FREQ_INFO: u32 = 0x16;
    /// System-On-Chip Vendor Attribute
    pub const SOC: u32 = 0x17;
    /// V2 Extended Topology
    pub const TOPOLOGY_V2: u32 = 0x1F;

    // Extended CPUID leaves (0x80000000+)
    /// Extended Maximum Input Value
    pub const EXT_MAX: u32 = 0x8000_0000;
    /// Extended Processor Signature and Feature Bits
    pub const EXT_FEATURES: u32 = 0x8000_0001;
    /// Processor Brand String (part 1)
    pub const BRAND_1: u32 = 0x8000_0002;
    /// Processor Brand String (part 2)
    pub const BRAND_2: u32 = 0x8000_0003;
    /// Processor Brand String (part 3)
    pub const BRAND_3: u32 = 0x8000_0004;
    /// L1 Cache and TLB Identifiers
    pub const EXT_CACHE_TLB: u32 = 0x8000_0005;
    /// Extended L2 Cache Features
    pub const EXT_L2_CACHE: u32 = 0x8000_0006;
    /// Advanced Power Management
    pub const APM: u32 = 0x8000_0007;
    /// Virtual and Physical Address Sizes
    pub const ADDR_SIZES: u32 = 0x8000_0008;
    /// AMD SVM Features
    pub const SVM: u32 = 0x8000_000A;
    /// AMD TLB 1GB Page Identifiers
    pub const TLB_1GB: u32 = 0x8000_0019;
    /// AMD Instruction Optimizations
    pub const PERF_OPT: u32 = 0x8000_001A;
    /// AMD IBS Capabilities
    pub const IBS: u32 = 0x8000_001B;
    /// AMD Lightweight Profiling
    pub const LWP: u32 = 0x8000_001C;
    /// AMD Cache Topology
    pub const CACHE_TOPO: u32 = 0x8000_001D;
    /// AMD Processor Topology
    pub const PROC_TOPO: u32 = 0x8000_001E;
}

// =============================================================================
// VENDOR IDENTIFICATION
// =============================================================================

/// CPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    /// Intel Corporation
    Intel,
    /// Advanced Micro Devices
    Amd,
    /// Unknown vendor
    Unknown([u8; 12]),
}

impl Vendor {
    /// Intel vendor string
    const INTEL: &'static [u8] = b"GenuineIntel";
    /// AMD vendor string
    const AMD: &'static [u8] = b"AuthenticAMD";

    /// Parse vendor from CPUID result
    pub fn from_cpuid(result: CpuIdResult) -> Self {
        let mut vendor = [0u8; 12];
        vendor[0..4].copy_from_slice(&result.ebx.to_le_bytes());
        vendor[4..8].copy_from_slice(&result.edx.to_le_bytes());
        vendor[8..12].copy_from_slice(&result.ecx.to_le_bytes());

        if vendor == *Self::INTEL {
            Vendor::Intel
        } else if vendor == *Self::AMD {
            Vendor::Amd
        } else {
            Vendor::Unknown(vendor)
        }
    }

    /// Get vendor string
    pub fn as_str(&self) -> &str {
        match self {
            Vendor::Intel => "GenuineIntel",
            Vendor::Amd => "AuthenticAMD",
            Vendor::Unknown(bytes) => core::str::from_utf8(bytes).unwrap_or("Unknown"),
        }
    }
}

// =============================================================================
// FEATURE FLAGS
// =============================================================================

/// CPU Feature (from CPUID)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[allow(non_camel_case_types)]
pub enum Feature {
    // CPUID.01H:ECX features
    /// SSE3 (Streaming SIMD Extensions 3)
    SSE3               = 0,
    /// PCLMULQDQ (Carry-less Multiplication)
    PCLMULQDQ          = 1,
    /// DTES64 (64-bit Debug Store)
    DTES64             = 2,
    /// MONITOR/MWAIT
    MONITOR            = 3,
    /// DS-CPL (Debug Store CPL Qualified)
    DSCPL              = 4,
    /// VMX (Virtual Machine Extensions)
    VMX                = 5,
    /// SMX (Safer Mode Extensions)
    SMX                = 6,
    /// EIST (Enhanced Intel SpeedStep)
    EIST               = 7,
    /// TM2 (Thermal Monitor 2)
    TM2                = 8,
    /// SSSE3 (Supplemental SSE3)
    SSSE3              = 9,
    /// CNXT-ID (L1 Context ID)
    CNXTID             = 10,
    /// SDBG (Silicon Debug)
    SDBG               = 11,
    /// FMA (Fused Multiply-Add)
    FMA                = 12,
    /// CMPXCHG16B
    CMPXCHG16B         = 13,
    /// xTPR Update Control
    XTPR               = 14,
    /// PDCM (Perfmon and Debug Capability)
    PDCM               = 15,
    /// PCID (Process-Context Identifiers)
    PCID               = 17,
    /// DCA (Direct Cache Access)
    DCA                = 18,
    /// SSE4.1
    SSE4_1             = 19,
    /// SSE4.2
    SSE4_2             = 20,
    /// x2APIC
    X2APIC             = 21,
    /// MOVBE
    MOVBE              = 22,
    /// POPCNT
    POPCNT             = 23,
    /// TSC-Deadline
    TSC_DEADLINE       = 24,
    /// AES-NI
    AESNI              = 25,
    /// XSAVE
    XSAVE              = 26,
    /// OSXSAVE
    OSXSAVE            = 27,
    /// AVX
    AVX                = 28,
    /// F16C (Half-precision FP)
    F16C               = 29,
    /// RDRAND
    RDRAND             = 30,
    /// Hypervisor present
    HYPERVISOR         = 31,

    // CPUID.01H:EDX features (add 100 to distinguish)
    /// x87 FPU
    FPU                = 100,
    /// Virtual 8086 Mode Extensions
    VME                = 101,
    /// Debugging Extensions
    DE                 = 102,
    /// Page Size Extension
    PSE                = 103,
    /// Time Stamp Counter
    TSC                = 104,
    /// RDMSR/WRMSR
    MSR                = 105,
    /// Physical Address Extension
    PAE                = 106,
    /// Machine Check Exception
    MCE                = 107,
    /// CMPXCHG8B
    CX8                = 108,
    /// APIC On-Chip
    APIC               = 109,
    /// SYSENTER/SYSEXIT
    SEP                = 111,
    /// Memory Type Range Registers
    MTRR               = 112,
    /// Page Global Bit
    PGE                = 113,
    /// Machine Check Architecture
    MCA                = 114,
    /// Conditional Move
    CMOV               = 115,
    /// Page Attribute Table
    PAT                = 116,
    /// 36-Bit Page Size Extension
    PSE36              = 117,
    /// Processor Serial Number
    PSN                = 118,
    /// CLFLUSH
    CLFSH              = 119,
    /// Debug Store
    DS                 = 121,
    /// ACPI Thermal Monitor and Clock Control
    ACPI               = 122,
    /// MMX
    MMX                = 123,
    /// FXSAVE/FXRSTOR
    FXSR               = 124,
    /// SSE
    SSE                = 125,
    /// SSE2
    SSE2               = 126,
    /// Self Snoop
    SS                 = 127,
    /// Hyper-Threading Technology
    HTT                = 128,
    /// Thermal Monitor
    TM                 = 129,
    /// IA64 Processor Emulating x86
    IA64               = 130,
    /// Pending Break Enable
    PBE                = 131,

    // CPUID.07H:EBX features (add 200 to distinguish)
    /// FSGSBASE
    FSGSBASE           = 200,
    /// IA32_TSC_ADJUST MSR
    TSC_ADJUST         = 201,
    /// SGX (Software Guard Extensions)
    SGX                = 202,
    /// BMI1 (Bit Manipulation Instruction Set 1)
    BMI1               = 203,
    /// HLE (Hardware Lock Elision)
    HLE                = 204,
    /// AVX2
    AVX2               = 205,
    /// FDP_EXCPTN_ONLY
    FDP_EXCPTN         = 206,
    /// SMEP (Supervisor Mode Execution Prevention)
    SMEP               = 207,
    /// BMI2 (Bit Manipulation Instruction Set 2)
    BMI2               = 208,
    /// Enhanced REP MOVSB/STOSB
    ERMS               = 209,
    /// INVPCID
    INVPCID            = 210,
    /// RTM (Restricted Transactional Memory)
    RTM                = 211,
    /// RDT-M (Resource Director Technology Monitoring)
    PQM                = 212,
    /// Deprecates FPU CS and FPU DS
    FPCSDS             = 213,
    /// MPX (Memory Protection Extensions)
    MPX                = 214,
    /// RDT-A (Resource Director Technology Allocation)
    PQE                = 215,
    /// AVX-512 Foundation
    AVX512F            = 216,
    /// AVX-512 Doubleword and Quadword
    AVX512DQ           = 217,
    /// RDSEED
    RDSEED             = 218,
    /// ADX (Multi-Precision Add-Carry)
    ADX                = 219,
    /// SMAP (Supervisor Mode Access Prevention)
    SMAP               = 220,
    /// AVX-512 Integer FMA
    AVX512IFMA         = 221,
    /// PCOMMIT (deprecated)
    PCOMMIT            = 222,
    /// CLFLUSHOPT
    CLFLUSHOPT         = 223,
    /// CLWB (Cache Line Write Back)
    CLWB               = 224,
    /// Intel Processor Trace
    PT                 = 225,
    /// AVX-512 Prefetch
    AVX512PF           = 226,
    /// AVX-512 Exponential and Reciprocal
    AVX512ER           = 227,
    /// AVX-512 Conflict Detection
    AVX512CD           = 228,
    /// SHA Extensions
    SHA                = 229,
    /// AVX-512 Byte and Word
    AVX512BW           = 230,
    /// AVX-512 Vector Length Extensions
    AVX512VL           = 231,

    // CPUID.07H:ECX features (add 300 to distinguish)
    /// PREFETCHWT1
    PREFETCHWT1        = 300,
    /// AVX-512 Vector Bit Manipulation
    AVX512VBMI         = 301,
    /// UMIP (User-Mode Instruction Prevention)
    UMIP               = 302,
    /// PKU (Protection Keys for User-mode pages)
    PKU                = 303,
    /// OSPKE (OS has enabled PKU)
    OSPKE              = 304,
    /// WAITPKG
    WAITPKG            = 305,
    /// AVX-512 VBMI2
    AVX512VBMI2        = 306,
    /// CET Shadow Stack
    CETSS              = 307,
    /// GFNI (Galois Field NI)
    GFNI               = 308,
    /// VAES
    VAES               = 309,
    /// VPCLMULQDQ
    VPCLMULQDQ         = 310,
    /// AVX-512 Vector Neural Network Instructions
    AVX512VNNI         = 311,
    /// AVX-512 Bit Algorithms
    AVX512BITALG       = 312,
    /// TME (Total Memory Encryption)
    TME                = 313,
    /// AVX-512 VPOPCNTDQ
    AVX512VPOPCNTDQ    = 314,
    /// LA57 (5-level paging)
    LA57               = 316,
    /// RDPID
    RDPID              = 322,
    /// Key Locker
    KL                 = 323,
    /// Bus Lock Detect
    BUSLOCKTRAP        = 324,
    /// CLDEMOTE
    CLDEMOTE           = 325,
    /// MOVDIRI
    MOVDIRI            = 327,
    /// MOVDIR64B
    MOVDIR64B          = 328,
    /// ENQCMD
    ENQCMD             = 329,
    /// SGX Launch Configuration
    SGXLC              = 330,
    /// PKS (Protection Keys for Supervisor-mode pages)
    PKS                = 331,

    // CPUID.07H:EDX features (add 400 to distinguish)
    /// SGX-KEYS
    SGXKEYS            = 401,
    /// AVX-512 4VNNIW
    AVX5124VNNIW       = 402,
    /// AVX-512 4FMAPS
    AVX5124FMAPS       = 403,
    /// Fast Short REP MOV
    FSRM               = 404,
    /// UINTR (User Interrupts)
    UINTR              = 405,
    /// AVX-512 VP2INTERSECT
    AVX512VP2INTERSECT = 408,
    /// SRBDS Mitigation
    SRBDSCTRL          = 409,
    /// MD Clear
    MDCLEAR            = 410,
    /// RTM Force Abort
    RTMFA              = 411,
    /// SERIALIZE
    SERIALIZE          = 414,
    /// Hybrid part
    HYBRID             = 415,
    /// TSXLDTRK
    TSXLDTRK           = 416,
    /// PCONFIG
    PCONFIG            = 418,
    /// Architectural LBR
    ARCHLBR            = 419,
    /// CET Indirect Branch Tracking
    CETIBT             = 420,
    /// AMX-BF16
    AMXBF16            = 422,
    /// AVX-512 FP16
    AVX512FP16         = 423,
    /// AMX Tile
    AMXTILE            = 424,
    /// AMX INT8
    AMXINT8            = 425,
    /// IBRS/IBPB
    IBRS               = 426,
    /// STIBP
    STIBP              = 427,
    /// L1D Flush
    L1DFLUSH           = 428,
    /// IA32_ARCH_CAPABILITIES
    ARCHCAP            = 429,
    /// IA32_CORE_CAPABILITIES
    CORECAP            = 430,
    /// SSBD
    SSBD               = 431,

    // Extended CPUID.80000001H:ECX features (add 500 to distinguish)
    /// LAHF/SAHF in 64-bit mode
    LAHFSAHF           = 500,
    /// Core Multi-Processing (AMD)
    CMP                = 501,
    /// SVM (Secure Virtual Machine)
    SVM                = 502,
    /// Extended APIC Space
    EXTAPIC            = 503,
    /// CR8 in 32-bit mode
    CR8LEGACY          = 504,
    /// ABM (Advanced Bit Manipulation) / LZCNT
    LZCNT              = 505,
    /// SSE4a
    SSE4A              = 506,
    /// Misaligned SSE Mode
    MISALIGNSSE        = 507,
    /// PREFETCH/PREFETCHW
    PREFETCHW          = 508,
    /// OS Visible Workaround
    OSVW               = 509,
    /// IBS (Instruction Based Sampling)
    IBS                = 510,
    /// XOP (Extended Operations)
    XOP                = 511,
    /// SKINIT/STGI
    SKINIT             = 512,
    /// Watchdog Timer
    WDT                = 513,
    /// Lightweight Profiling
    LWP                = 515,
    /// FMA4
    FMA4               = 516,
    /// Translation Cache Extension
    TCE                = 517,
    /// NodeID MSR
    NODEID             = 519,
    /// TBM (Trailing Bit Manipulation)
    TBM                = 521,
    /// Topology Extensions
    TOPOEXT            = 522,
    /// PerfCtrExtCore
    PERFCTRCORE        = 523,
    /// PerfCtrExtNB
    PERFCTRNB          = 524,
    /// Streaming Performance Monitor
    STREAMPERFMON      = 525,
    /// Data Breakpoint Extension
    DBX                = 526,
    /// PerfTsc
    PERFTSC            = 527,
    /// L2I (L2 Instruction Cache Performance Counter)
    L2I                = 528,
    /// MWAITX
    MWAITX             = 529,
    /// Address Mask Extension
    ADDRMASKEXT        = 530,

    // Extended CPUID.80000001H:EDX features (add 600 to distinguish)
    /// SYSCALL/SYSRET
    SYSCALL            = 611,
    /// NX (No-Execute)
    NX                 = 620,
    /// MMXEXT (AMD Extended MMX)
    MMXEXT             = 622,
    /// FXSR Optimizations
    FXSROPT            = 625,
    /// 1GB Pages
    PDPE1GB            = 626,
    /// RDTSCP
    RDTSCP             = 627,
    /// Long Mode (64-bit)
    LM                 = 629,
    /// 3DNow!+ (AMD Extended 3DNow!)
    _3DNOWEXT          = 630,
    /// 3DNow!
    _3DNOW             = 631,

    // Extended CPUID.80000007H:EDX features (add 700 to distinguish)
    /// Invariant TSC
    INVARIANT_TSC      = 708,
}

// =============================================================================
// CPUID WRAPPER
// =============================================================================

/// Complete CPUID information for the current processor
pub struct CpuId {
    vendor: Vendor,
    max_basic_leaf: u32,
    max_extended_leaf: u32,
    family: u8,
    model: u8,
    stepping: u8,
    brand_string: [u8; 48],
    features_ecx_01: u32,
    features_edx_01: u32,
    features_ebx_07: u32,
    features_ecx_07: u32,
    features_edx_07: u32,
    ext_features_ecx: u32,
    ext_features_edx: u32,
    ext_features_edx_07: u32,
    phys_addr_bits: u8,
    virt_addr_bits: u8,
}

impl CpuId {
    /// Query CPUID and create a new instance
    pub fn new() -> Self {
        // Get vendor and max leaf
        let vendor_result = cpuid(leaf::VENDOR);
        let vendor = Vendor::from_cpuid(vendor_result);
        let max_basic_leaf = vendor_result.eax;

        // Get extended max leaf
        let ext_max = cpuid(leaf::EXT_MAX);
        let max_extended_leaf = ext_max.eax;

        // Get processor info (leaf 01H)
        let features_01 = if max_basic_leaf >= 1 {
            cpuid(leaf::FEATURES)
        } else {
            CpuIdResult::zero()
        };

        // Parse family/model/stepping
        let (family, model, stepping) = Self::parse_version(features_01.eax);

        // Get extended features (leaf 07H)
        let features_07 = if max_basic_leaf >= 7 {
            cpuid_count(leaf::STRUCT_EXT_FEATURES, 0)
        } else {
            CpuIdResult::zero()
        };

        // Get extended features EDX (subleaf 1)
        let _features_07_1 = if max_basic_leaf >= 7 {
            cpuid_count(leaf::STRUCT_EXT_FEATURES, 1)
        } else {
            CpuIdResult::zero()
        };

        // Get extended CPUID features (80000001H)
        let ext_features = if max_extended_leaf >= leaf::EXT_FEATURES {
            cpuid(leaf::EXT_FEATURES)
        } else {
            CpuIdResult::zero()
        };

        // Get address sizes (80000008H)
        let addr_sizes = if max_extended_leaf >= leaf::ADDR_SIZES {
            cpuid(leaf::ADDR_SIZES)
        } else {
            CpuIdResult::zero()
        };

        // Get invariant TSC (80000007H)
        let apm = if max_extended_leaf >= leaf::APM {
            cpuid(leaf::APM)
        } else {
            CpuIdResult::zero()
        };

        // Get brand string
        let mut brand_string = [0u8; 48];
        if max_extended_leaf >= leaf::BRAND_3 {
            let b1 = cpuid(leaf::BRAND_1);
            let b2 = cpuid(leaf::BRAND_2);
            let b3 = cpuid(leaf::BRAND_3);

            brand_string[0..4].copy_from_slice(&b1.eax.to_le_bytes());
            brand_string[4..8].copy_from_slice(&b1.ebx.to_le_bytes());
            brand_string[8..12].copy_from_slice(&b1.ecx.to_le_bytes());
            brand_string[12..16].copy_from_slice(&b1.edx.to_le_bytes());
            brand_string[16..20].copy_from_slice(&b2.eax.to_le_bytes());
            brand_string[20..24].copy_from_slice(&b2.ebx.to_le_bytes());
            brand_string[24..28].copy_from_slice(&b2.ecx.to_le_bytes());
            brand_string[28..32].copy_from_slice(&b2.edx.to_le_bytes());
            brand_string[32..36].copy_from_slice(&b3.eax.to_le_bytes());
            brand_string[36..40].copy_from_slice(&b3.ebx.to_le_bytes());
            brand_string[40..44].copy_from_slice(&b3.ecx.to_le_bytes());
            brand_string[44..48].copy_from_slice(&b3.edx.to_le_bytes());
        }

        Self {
            vendor,
            max_basic_leaf,
            max_extended_leaf,
            family,
            model,
            stepping,
            brand_string,
            features_ecx_01: features_01.ecx,
            features_edx_01: features_01.edx,
            features_ebx_07: features_07.ebx,
            features_ecx_07: features_07.ecx,
            features_edx_07: features_07.edx,
            ext_features_ecx: ext_features.ecx,
            ext_features_edx: ext_features.edx,
            ext_features_edx_07: apm.edx,
            phys_addr_bits: (addr_sizes.eax & 0xFF) as u8,
            virt_addr_bits: ((addr_sizes.eax >> 8) & 0xFF) as u8,
        }
    }

    /// Parse version information from EAX
    fn parse_version(eax: u32) -> (u8, u8, u8) {
        let stepping = (eax & 0xF) as u8;
        let mut model = ((eax >> 4) & 0xF) as u8;
        let mut family = ((eax >> 8) & 0xF) as u8;

        // Extended family/model
        if family == 0xF {
            family += ((eax >> 20) & 0xFF) as u8;
        }
        if family == 0x6 || family == 0xF {
            model += (((eax >> 16) & 0xF) << 4) as u8;
        }

        (family, model, stepping)
    }

    /// Get CPU vendor
    pub fn vendor(&self) -> Vendor {
        self.vendor
    }

    /// Get maximum basic CPUID leaf
    pub fn max_basic_leaf(&self) -> u32 {
        self.max_basic_leaf
    }

    /// Get maximum extended CPUID leaf
    pub fn max_extended_leaf(&self) -> u32 {
        self.max_extended_leaf
    }

    /// Get CPU family
    pub fn family(&self) -> u8 {
        self.family
    }

    /// Get CPU model
    pub fn model(&self) -> u8 {
        self.model
    }

    /// Get CPU stepping
    pub fn stepping(&self) -> u8 {
        self.stepping
    }

    /// Get processor brand string
    pub fn brand_string(&self) -> &str {
        // Find null terminator or end
        let end = self.brand_string.iter().position(|&c| c == 0).unwrap_or(48);
        core::str::from_utf8(&self.brand_string[..end])
            .unwrap_or("")
            .trim()
    }

    /// Get physical address bits
    pub fn phys_addr_bits(&self) -> u8 {
        if self.phys_addr_bits == 0 {
            36
        } else {
            self.phys_addr_bits
        }
    }

    /// Get virtual address bits
    pub fn virt_addr_bits(&self) -> u8 {
        if self.virt_addr_bits == 0 {
            48
        } else {
            self.virt_addr_bits
        }
    }

    /// Check if a specific feature is supported
    pub fn has_feature(&self, feature: Feature) -> bool {
        let value = feature as u32;

        match value {
            0..=31 => (self.features_ecx_01 & (1 << value)) != 0,
            100..=131 => (self.features_edx_01 & (1 << (value - 100))) != 0,
            200..=231 => (self.features_ebx_07 & (1 << (value - 200))) != 0,
            300..=331 => (self.features_ecx_07 & (1 << (value - 300))) != 0,
            400..=431 => (self.features_edx_07 & (1 << (value - 400))) != 0,
            500..=531 => (self.ext_features_ecx & (1 << (value - 500))) != 0,
            600..=631 => (self.ext_features_edx & (1 << (value - 600))) != 0,
            700..=731 => (self.ext_features_edx_07 & (1 << (value - 700))) != 0,
            _ => false,
        }
    }

    // =========================================================================
    // Convenience feature checks
    // =========================================================================

    /// Check if SSE is supported
    pub fn has_sse(&self) -> bool {
        self.has_feature(Feature::SSE)
    }

    /// Check if SSE2 is supported
    pub fn has_sse2(&self) -> bool {
        self.has_feature(Feature::SSE2)
    }

    /// Check if SSE3 is supported
    pub fn has_sse3(&self) -> bool {
        self.has_feature(Feature::SSE3)
    }

    /// Check if SSSE3 is supported
    pub fn has_ssse3(&self) -> bool {
        self.has_feature(Feature::SSSE3)
    }

    /// Check if SSE4.1 is supported
    pub fn has_sse4_1(&self) -> bool {
        self.has_feature(Feature::SSE4_1)
    }

    /// Check if SSE4.2 is supported
    pub fn has_sse4_2(&self) -> bool {
        self.has_feature(Feature::SSE4_2)
    }

    /// Check if AVX is supported
    pub fn has_avx(&self) -> bool {
        self.has_feature(Feature::AVX)
    }

    /// Check if AVX2 is supported
    pub fn has_avx2(&self) -> bool {
        self.has_feature(Feature::AVX2)
    }

    /// Check if AVX-512 Foundation is supported
    pub fn has_avx512f(&self) -> bool {
        self.has_feature(Feature::AVX512F)
    }

    /// Check if RDRAND is supported
    pub fn has_rdrand(&self) -> bool {
        self.has_feature(Feature::RDRAND)
    }

    /// Check if RDSEED is supported
    pub fn has_rdseed(&self) -> bool {
        self.has_feature(Feature::RDSEED)
    }

    /// Check if x2APIC is supported
    pub fn has_x2apic(&self) -> bool {
        self.has_feature(Feature::X2APIC)
    }

    /// Check if TSC-Deadline mode is supported
    pub fn has_tsc_deadline(&self) -> bool {
        self.has_feature(Feature::TSC_DEADLINE)
    }

    /// Check if invariant TSC is supported
    pub fn has_invariant_tsc(&self) -> bool {
        self.has_feature(Feature::INVARIANT_TSC)
    }

    /// Check if RDTSCP is supported
    pub fn has_rdtscp(&self) -> bool {
        self.has_feature(Feature::RDTSCP)
    }

    /// Check if PCID is supported
    pub fn has_pcid(&self) -> bool {
        self.has_feature(Feature::PCID)
    }

    /// Check if INVPCID is supported
    pub fn has_invpcid(&self) -> bool {
        self.has_feature(Feature::INVPCID)
    }

    /// Check if SMEP is supported
    pub fn has_smep(&self) -> bool {
        self.has_feature(Feature::SMEP)
    }

    /// Check if SMAP is supported
    pub fn has_smap(&self) -> bool {
        self.has_feature(Feature::SMAP)
    }

    /// Check if UMIP is supported
    pub fn has_umip(&self) -> bool {
        self.has_feature(Feature::UMIP)
    }

    /// Check if LA57 (5-level paging) is supported
    pub fn has_la57(&self) -> bool {
        self.has_feature(Feature::LA57)
    }

    /// Check if 1GB pages are supported
    pub fn has_1gb_pages(&self) -> bool {
        self.has_feature(Feature::PDPE1GB)
    }

    /// Check if NX bit is supported
    pub fn has_nx(&self) -> bool {
        self.has_feature(Feature::NX)
    }

    /// Check if VMX (Intel VT-x) is supported
    pub fn has_vmx(&self) -> bool {
        self.has_feature(Feature::VMX)
    }

    /// Check if SVM (AMD-V) is supported
    pub fn has_svm(&self) -> bool {
        self.has_feature(Feature::SVM)
    }

    /// Check if XSAVE is supported
    pub fn has_xsave(&self) -> bool {
        self.has_feature(Feature::XSAVE)
    }

    /// Check if FSGSBASE instructions are supported
    pub fn has_fsgsbase(&self) -> bool {
        self.has_feature(Feature::FSGSBASE)
    }
}

impl Default for CpuId {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// TOPOLOGY
// =============================================================================

/// CPU topology level type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyLevel {
    /// Invalid
    Invalid,
    /// SMT (Hyper-Threading)
    Thread,
    /// Core
    Core,
    /// Module
    Module,
    /// Tile
    Tile,
    /// Die
    Die,
    /// Package
    Package,
}

impl TopologyLevel {
    fn from_u8(value: u8) -> Self {
        match value {
            0 => TopologyLevel::Invalid,
            1 => TopologyLevel::Thread,
            2 => TopologyLevel::Core,
            3 => TopologyLevel::Module,
            4 => TopologyLevel::Tile,
            5 => TopologyLevel::Die,
            _ => TopologyLevel::Invalid,
        }
    }
}

/// Topology information for a single level
#[derive(Debug, Clone, Copy)]
pub struct TopologyInfo {
    /// Level type
    pub level_type: TopologyLevel,
    /// Number of logical processors at this level
    pub num_processors: u32,
    /// Shift count for next level ID
    pub shift: u8,
}

/// Enumerate CPU topology using CPUID.0BH or CPUID.1FH
pub fn enumerate_topology() -> impl Iterator<Item = TopologyInfo> {
    TopologyIterator {
        subleaf: 0,
        use_1f: cpuid(0).eax >= 0x1F,
    }
}

struct TopologyIterator {
    subleaf: u32,
    use_1f: bool,
}

impl Iterator for TopologyIterator {
    type Item = TopologyInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let leaf = if self.use_1f {
            leaf::TOPOLOGY_V2
        } else {
            leaf::TOPOLOGY
        };
        let result = cpuid_count(leaf, self.subleaf);

        // Check if this level is valid
        if result.ebx == 0 {
            return None;
        }

        let level_type = TopologyLevel::from_u8(((result.ecx >> 8) & 0xFF) as u8);
        let num_processors = result.ebx & 0xFFFF;
        let shift = (result.eax & 0x1F) as u8;

        self.subleaf += 1;

        Some(TopologyInfo {
            level_type,
            num_processors,
            shift,
        })
    }
}

// =============================================================================
// CACHE INFORMATION
// =============================================================================

/// Cache type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType {
    /// Null (no more caches)
    Null,
    /// Data cache
    Data,
    /// Instruction cache
    Instruction,
    /// Unified cache (data + instruction)
    Unified,
}

impl CacheType {
    fn from_u32(value: u32) -> Self {
        match value & 0x1F {
            0 => CacheType::Null,
            1 => CacheType::Data,
            2 => CacheType::Instruction,
            3 => CacheType::Unified,
            _ => CacheType::Null,
        }
    }
}

/// Cache parameters
#[derive(Debug, Clone, Copy)]
pub struct CacheInfo {
    /// Cache level (1, 2, 3, ...)
    pub level: u8,
    /// Cache type
    pub cache_type: CacheType,
    /// Total size in bytes
    pub size: usize,
    /// Line size in bytes
    pub line_size: usize,
    /// Number of ways (associativity)
    pub ways: usize,
    /// Number of sets
    pub sets: usize,
    /// Is self-initializing
    pub self_init: bool,
    /// Is fully associative
    pub fully_assoc: bool,
    /// Max cores sharing this cache
    pub max_sharing_cores: usize,
}

/// Enumerate cache information using CPUID.04H
pub fn enumerate_caches() -> impl Iterator<Item = CacheInfo> {
    CacheIterator { subleaf: 0 }
}

struct CacheIterator {
    subleaf: u32,
}

impl Iterator for CacheIterator {
    type Item = CacheInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let result = cpuid_count(leaf::CACHE_PARAMS, self.subleaf);

        let cache_type = CacheType::from_u32(result.eax);
        if cache_type == CacheType::Null {
            return None;
        }

        let level = ((result.eax >> 5) & 0x7) as u8;
        let self_init = (result.eax & (1 << 8)) != 0;
        let fully_assoc = (result.eax & (1 << 9)) != 0;
        let max_sharing_cores = (((result.eax >> 14) & 0xFFF) + 1) as usize;

        let line_size = ((result.ebx & 0xFFF) + 1) as usize;
        let partitions = (((result.ebx >> 12) & 0x3FF) + 1) as usize;
        let ways = (((result.ebx >> 22) & 0x3FF) + 1) as usize;
        let sets = (result.ecx + 1) as usize;

        let size = line_size * partitions * ways * sets;

        self.subleaf += 1;

        Some(CacheInfo {
            level,
            cache_type,
            size,
            line_size,
            ways,
            sets,
            self_init,
            fully_assoc,
            max_sharing_cores,
        })
    }
}

// =============================================================================
// TSC FREQUENCY
// =============================================================================

/// TSC frequency information from CPUID
#[derive(Debug, Clone, Copy)]
pub struct TscFrequency {
    /// Numerator (TSC/core crystal clock ratio)
    pub numerator: u32,
    /// Denominator
    pub denominator: u32,
    /// Core crystal clock frequency in Hz (0 if not enumerated)
    pub crystal_hz: u64,
    /// Calculated TSC frequency in Hz (0 if cannot be calculated)
    pub tsc_hz: u64,
}

/// Get TSC frequency information from CPUID.15H
pub fn get_tsc_frequency() -> Option<TscFrequency> {
    let max_leaf = cpuid(0).eax;
    if max_leaf < leaf::TSC_FREQ {
        return None;
    }

    let result = cpuid(leaf::TSC_FREQ);

    let denominator = result.eax;
    let numerator = result.ebx;
    let crystal_hz = result.ecx as u64;

    if denominator == 0 || numerator == 0 {
        return None;
    }

    let tsc_hz = if crystal_hz != 0 {
        (crystal_hz * numerator as u64) / denominator as u64
    } else {
        0
    };

    Some(TscFrequency {
        numerator,
        denominator,
        crystal_hz,
        tsc_hz,
    })
}

/// Get processor base/max frequency from CPUID.16H
pub fn get_processor_frequency() -> Option<(u16, u16, u16)> {
    let max_leaf = cpuid(0).eax;
    if max_leaf < leaf::FREQ_INFO {
        return None;
    }

    let result = cpuid(leaf::FREQ_INFO);

    let base_mhz = (result.eax & 0xFFFF) as u16;
    let max_mhz = (result.ebx & 0xFFFF) as u16;
    let bus_mhz = (result.ecx & 0xFFFF) as u16;

    if base_mhz == 0 {
        return None;
    }

    Some((base_mhz, max_mhz, bus_mhz))
}

// =============================================================================
// XSAVE INFORMATION
// =============================================================================

/// XSAVE area information
#[derive(Debug, Clone, Copy)]
pub struct XsaveInfo {
    /// Required size for enabled features (CPUID.0DH:0.EBX)
    pub enabled_size: u32,
    /// Maximum size for all features (CPUID.0DH:0.ECX)
    pub max_size: u32,
    /// Supported features bitmap (XCR0 | IA32_XSS)
    pub features: u64,
}

/// Get XSAVE area information
pub fn get_xsave_info() -> Option<XsaveInfo> {
    let cpuid_info = CpuId::new();
    if !cpuid_info.has_xsave() {
        return None;
    }

    let result = cpuid_count(leaf::XSAVE, 0);

    let features_lo = result.eax;
    let enabled_size = result.ebx;
    let max_size = result.ecx;
    let features_hi = result.edx;

    let features = ((features_hi as u64) << 32) | (features_lo as u64);

    Some(XsaveInfo {
        enabled_size,
        max_size,
        features,
    })
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpuid_basic() {
        let cpuid = CpuId::new();

        // Should have valid vendor
        let vendor = cpuid.vendor();
        assert!(matches!(
            vendor,
            Vendor::Intel | Vendor::Amd | Vendor::Unknown(_)
        ));

        // Should support basic x86_64 features
        assert!(cpuid.has_feature(Feature::FPU));
        assert!(cpuid.has_feature(Feature::MSR));
        assert!(cpuid.has_feature(Feature::PAE));
        assert!(cpuid.has_feature(Feature::APIC));
        assert!(cpuid.has_feature(Feature::LM)); // Long mode = 64-bit
    }

    #[test]
    fn test_address_sizes() {
        let cpuid = CpuId::new();

        // Physical address should be at least 36 bits
        assert!(cpuid.phys_addr_bits() >= 36);

        // Virtual address should be at least 48 bits
        assert!(cpuid.virt_addr_bits() >= 48);
    }

    #[test]
    fn test_brand_string() {
        let cpuid = CpuId::new();
        let brand = cpuid.brand_string();

        // Should have some content
        assert!(!brand.is_empty());
    }
}
