//! # CPU Features Detection and Capabilities
//!
//! High-level CPU capability detection and feature management.
//!
//! ## Overview
//!
//! This module provides a unified interface for:
//!
//! - Detecting CPU features at runtime
//! - Caching feature information for fast access
//! - Feature-gated code paths
//! - Hardware capability reporting
//!
//! ## Usage
//!
//! ```rust,no_run
//! use helix_hal::arch::x86_64::core::features::CpuCapabilities;
//!
//! let caps = CpuCapabilities::detect();
//!
//! if caps.has_avx512() {
//!     // Use AVX-512 optimized path
//! } else if caps.has_avx2() {
//!     // Use AVX2 optimized path
//! } else {
//!     // Fallback to SSE2 (always available on x86_64)
//! }
//! ```

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::cpuid::{CpuId, Feature, Vendor};

// =============================================================================
// STATIC CAPABILITIES CACHE
// =============================================================================

/// Global capabilities cache (lazily initialized)
static CAPS_INITIALIZED: AtomicBool = AtomicBool::new(false);
static CAPS_FEATURES_BASIC: AtomicU64 = AtomicU64::new(0);
static CAPS_FEATURES_EXT: AtomicU64 = AtomicU64::new(0);
static CAPS_FEATURES_AMD: AtomicU64 = AtomicU64::new(0);

// =============================================================================
// FEATURE CATEGORIES
// =============================================================================

/// Feature categories for grouping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureCategory {
    /// Basic x86_64 features (always present)
    Basic,
    /// SIMD features (SSE, AVX, AVX-512)
    Simd,
    /// Memory management features (paging, PCID, LA57)
    Memory,
    /// Security features (SMEP, SMAP, PKU)
    Security,
    /// Virtualization features (VMX, SVM)
    Virtualization,
    /// Timing features (TSC, HPET)
    Timing,
    /// Power management features
    Power,
    /// Debugging features
    Debug,
    /// Cryptographic features (AES-NI, SHA)
    Crypto,
}

// =============================================================================
// CPU CAPABILITIES
// =============================================================================

/// Detected CPU capabilities
///
/// This struct caches all detected CPU features for fast runtime checks.
/// It should be initialized once at boot and then queried as needed.
#[derive(Debug, Clone, Copy)]
pub struct CpuCapabilities {
    // Identification
    vendor: Vendor,
    family: u8,
    model: u8,
    stepping: u8,

    // Address sizes
    phys_bits: u8,
    virt_bits: u8,

    // Basic features
    features_basic: u64,

    // Extended features
    features_ext: u64,

    // AMD-specific features
    features_amd: u64,

    // Cache info
    l1d_size: u32,
    l1i_size: u32,
    l2_size: u32,
    l3_size: u32,
    cache_line_size: u8,

    // Topology
    logical_cores: u16,
    physical_cores: u16,
}

impl CpuCapabilities {
    /// Detect CPU capabilities from CPUID
    pub fn detect() -> Self {
        let cpuid = CpuId::new();

        // Build feature bitmaps
        let mut basic = 0u64;
        let mut ext = 0u64;
        let mut amd = 0u64;

        // Basic features
        if cpuid.has_sse() {
            basic |= 1 << 0;
        }
        if cpuid.has_sse2() {
            basic |= 1 << 1;
        }
        if cpuid.has_sse3() {
            basic |= 1 << 2;
        }
        if cpuid.has_ssse3() {
            basic |= 1 << 3;
        }
        if cpuid.has_sse4_1() {
            basic |= 1 << 4;
        }
        if cpuid.has_sse4_2() {
            basic |= 1 << 5;
        }
        if cpuid.has_avx() {
            basic |= 1 << 6;
        }
        if cpuid.has_avx2() {
            basic |= 1 << 7;
        }
        if cpuid.has_avx512f() {
            basic |= 1 << 8;
        }
        if cpuid.has_feature(Feature::FPU) {
            basic |= 1 << 9;
        }
        if cpuid.has_feature(Feature::APIC) {
            basic |= 1 << 10;
        }
        if cpuid.has_x2apic() {
            basic |= 1 << 11;
        }
        if cpuid.has_feature(Feature::MSR) {
            basic |= 1 << 12;
        }
        if cpuid.has_feature(Feature::PAE) {
            basic |= 1 << 13;
        }
        if cpuid.has_feature(Feature::MTRR) {
            basic |= 1 << 14;
        }
        if cpuid.has_feature(Feature::PGE) {
            basic |= 1 << 15;
        }
        if cpuid.has_feature(Feature::PAT) {
            basic |= 1 << 16;
        }
        if cpuid.has_feature(Feature::FXSR) {
            basic |= 1 << 17;
        }
        if cpuid.has_xsave() {
            basic |= 1 << 18;
        }
        if cpuid.has_feature(Feature::CMPXCHG16B) {
            basic |= 1 << 19;
        }
        if cpuid.has_feature(Feature::POPCNT) {
            basic |= 1 << 20;
        }
        if cpuid.has_feature(Feature::MONITOR) {
            basic |= 1 << 21;
        }
        if cpuid.has_feature(Feature::VMX) {
            basic |= 1 << 22;
        }
        if cpuid.has_feature(Feature::SMX) {
            basic |= 1 << 23;
        }
        if cpuid.has_feature(Feature::HTT) {
            basic |= 1 << 24;
        }
        if cpuid.has_feature(Feature::TM) {
            basic |= 1 << 25;
        }

        // Extended features
        if cpuid.has_nx() {
            ext |= 1 << 0;
        }
        if cpuid.has_feature(Feature::LM) {
            ext |= 1 << 1;
        }
        if cpuid.has_1gb_pages() {
            ext |= 1 << 2;
        }
        if cpuid.has_la57() {
            ext |= 1 << 3;
        }
        if cpuid.has_pcid() {
            ext |= 1 << 4;
        }
        if cpuid.has_invpcid() {
            ext |= 1 << 5;
        }
        if cpuid.has_smep() {
            ext |= 1 << 6;
        }
        if cpuid.has_smap() {
            ext |= 1 << 7;
        }
        if cpuid.has_umip() {
            ext |= 1 << 8;
        }
        if cpuid.has_feature(Feature::PKU) {
            ext |= 1 << 9;
        }
        if cpuid.has_feature(Feature::PKS) {
            ext |= 1 << 10;
        }
        if cpuid.has_fsgsbase() {
            ext |= 1 << 11;
        }
        if cpuid.has_feature(Feature::SYSCALL) {
            ext |= 1 << 12;
        }
        if cpuid.has_rdtscp() {
            ext |= 1 << 13;
        }
        if cpuid.has_invariant_tsc() {
            ext |= 1 << 14;
        }
        if cpuid.has_tsc_deadline() {
            ext |= 1 << 15;
        }
        if cpuid.has_rdrand() {
            ext |= 1 << 16;
        }
        if cpuid.has_rdseed() {
            ext |= 1 << 17;
        }
        if cpuid.has_feature(Feature::AESNI) {
            ext |= 1 << 18;
        }
        if cpuid.has_feature(Feature::SHA) {
            ext |= 1 << 19;
        }
        if cpuid.has_feature(Feature::CLFLUSHOPT) {
            ext |= 1 << 20;
        }
        if cpuid.has_feature(Feature::CLWB) {
            ext |= 1 << 21;
        }
        if cpuid.has_feature(Feature::ERMS) {
            ext |= 1 << 22;
        }
        if cpuid.has_feature(Feature::FSRM) {
            ext |= 1 << 23;
        }

        // AMD features
        if cpuid.has_svm() {
            amd |= 1 << 0;
        }
        if cpuid.has_feature(Feature::LZCNT) {
            amd |= 1 << 1;
        }
        if cpuid.has_feature(Feature::SSE4A) {
            amd |= 1 << 2;
        }
        if cpuid.has_feature(Feature::PREFETCHW) {
            amd |= 1 << 3;
        }
        if cpuid.has_feature(Feature::FMA4) {
            amd |= 1 << 4;
        }
        if cpuid.has_feature(Feature::XOP) {
            amd |= 1 << 5;
        }
        if cpuid.has_feature(Feature::TBM) {
            amd |= 1 << 6;
        }
        if cpuid.has_feature(Feature::MWAITX) {
            amd |= 1 << 7;
        }

        // Get cache info
        let (l1d, l1i, l2, l3, line_size) = Self::detect_cache_sizes();

        // Get core counts (simplified - would need full topology enumeration)
        let logical = Self::detect_logical_cores();
        let physical = Self::detect_physical_cores();

        Self {
            vendor: cpuid.vendor(),
            family: cpuid.family(),
            model: cpuid.model(),
            stepping: cpuid.stepping(),
            phys_bits: cpuid.phys_addr_bits(),
            virt_bits: cpuid.virt_addr_bits(),
            features_basic: basic,
            features_ext: ext,
            features_amd: amd,
            l1d_size: l1d,
            l1i_size: l1i,
            l2_size: l2,
            l3_size: l3,
            cache_line_size: line_size,
            logical_cores: logical,
            physical_cores: physical,
        }
    }

    /// Detect and cache globally
    pub fn init_global() {
        let caps = Self::detect();
        CAPS_FEATURES_BASIC.store(caps.features_basic, Ordering::Release);
        CAPS_FEATURES_EXT.store(caps.features_ext, Ordering::Release);
        CAPS_FEATURES_AMD.store(caps.features_amd, Ordering::Release);
        CAPS_INITIALIZED.store(true, Ordering::Release);
    }

    /// Get global cached capabilities
    pub fn global() -> Self {
        if !CAPS_INITIALIZED.load(Ordering::Acquire) {
            Self::init_global();
        }
        Self::detect() // For now, just re-detect (cheap)
    }

    fn detect_cache_sizes() -> (u32, u32, u32, u32, u8) {
        use super::cpuid::{enumerate_caches, CacheType};

        let mut l1d = 0u32;
        let mut l1i = 0u32;
        let mut l2 = 0u32;
        let mut l3 = 0u32;
        let mut line_size = 64u8;

        for cache in enumerate_caches() {
            match (cache.level, cache.cache_type) {
                (1, CacheType::Data) => {
                    l1d = cache.size as u32;
                    line_size = cache.line_size as u8;
                },
                (1, CacheType::Instruction) => {
                    l1i = cache.size as u32;
                },
                (2, CacheType::Unified) => {
                    l2 = cache.size as u32;
                },
                (3, CacheType::Unified) => {
                    l3 = cache.size as u32;
                },
                _ => {},
            }
        }

        (l1d, l1i, l2, l3, line_size)
    }

    fn detect_logical_cores() -> u16 {
        use super::cpuid::enumerate_topology;

        for topo in enumerate_topology() {
            if topo.num_processors > 0 {
                return topo.num_processors as u16;
            }
        }
        1 // Default to 1
    }

    fn detect_physical_cores() -> u16 {
        use super::cpuid::{enumerate_topology, TopologyLevel};

        let mut thread_per_core = 1;
        let mut total_logical = 1;

        for topo in enumerate_topology() {
            match topo.level_type {
                TopologyLevel::Thread => {
                    thread_per_core = topo.num_processors as u16;
                },
                TopologyLevel::Core => {
                    total_logical = topo.num_processors as u16;
                },
                _ => {},
            }
        }

        if thread_per_core > 0 {
            total_logical / thread_per_core
        } else {
            1
        }
    }

    // =========================================================================
    // Identification
    // =========================================================================

    /// Get CPU vendor
    pub fn vendor(&self) -> Vendor {
        self.vendor
    }

    /// Check if Intel CPU
    pub fn is_intel(&self) -> bool {
        matches!(self.vendor, Vendor::Intel)
    }

    /// Check if AMD CPU
    pub fn is_amd(&self) -> bool {
        matches!(self.vendor, Vendor::Amd)
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

    // =========================================================================
    // Address Sizes
    // =========================================================================

    /// Get physical address bits
    pub fn phys_addr_bits(&self) -> u8 {
        self.phys_bits
    }

    /// Get virtual address bits
    pub fn virt_addr_bits(&self) -> u8 {
        self.virt_bits
    }

    /// Get maximum physical address
    pub fn max_phys_addr(&self) -> u64 {
        (1u64 << self.phys_bits) - 1
    }

    /// Check if 5-level paging is supported
    pub fn supports_la57(&self) -> bool {
        (self.features_ext & (1 << 3)) != 0
    }

    // =========================================================================
    // SIMD Features
    // =========================================================================

    /// Check for SSE (always available on x86_64)
    pub fn has_sse(&self) -> bool {
        (self.features_basic & (1 << 0)) != 0
    }

    /// Check for SSE2 (always available on x86_64)
    pub fn has_sse2(&self) -> bool {
        (self.features_basic & (1 << 1)) != 0
    }

    /// Check for SSE3
    pub fn has_sse3(&self) -> bool {
        (self.features_basic & (1 << 2)) != 0
    }

    /// Check for SSSE3
    pub fn has_ssse3(&self) -> bool {
        (self.features_basic & (1 << 3)) != 0
    }

    /// Check for SSE4.1
    pub fn has_sse4_1(&self) -> bool {
        (self.features_basic & (1 << 4)) != 0
    }

    /// Check for SSE4.2
    pub fn has_sse4_2(&self) -> bool {
        (self.features_basic & (1 << 5)) != 0
    }

    /// Check for AVX
    pub fn has_avx(&self) -> bool {
        (self.features_basic & (1 << 6)) != 0
    }

    /// Check for AVX2
    pub fn has_avx2(&self) -> bool {
        (self.features_basic & (1 << 7)) != 0
    }

    /// Check for AVX-512 Foundation
    pub fn has_avx512(&self) -> bool {
        (self.features_basic & (1 << 8)) != 0
    }

    /// Get maximum SIMD width in bytes
    pub fn max_simd_width(&self) -> usize {
        if self.has_avx512() {
            64
        } else if self.has_avx() {
            32
        } else {
            16 // SSE
        }
    }

    // =========================================================================
    // Memory Features
    // =========================================================================

    /// Check for NX bit support
    pub fn has_nx(&self) -> bool {
        (self.features_ext & (1 << 0)) != 0
    }

    /// Check for 1GB page support
    pub fn has_1gb_pages(&self) -> bool {
        (self.features_ext & (1 << 2)) != 0
    }

    /// Check for PCID support
    pub fn has_pcid(&self) -> bool {
        (self.features_ext & (1 << 4)) != 0
    }

    /// Check for INVPCID support
    pub fn has_invpcid(&self) -> bool {
        (self.features_ext & (1 << 5)) != 0
    }

    // =========================================================================
    // Security Features
    // =========================================================================

    /// Check for SMEP support
    pub fn has_smep(&self) -> bool {
        (self.features_ext & (1 << 6)) != 0
    }

    /// Check for SMAP support
    pub fn has_smap(&self) -> bool {
        (self.features_ext & (1 << 7)) != 0
    }

    /// Check for UMIP support
    pub fn has_umip(&self) -> bool {
        (self.features_ext & (1 << 8)) != 0
    }

    /// Check for PKU support
    pub fn has_pku(&self) -> bool {
        (self.features_ext & (1 << 9)) != 0
    }

    // =========================================================================
    // Virtualization
    // =========================================================================

    /// Check for VMX (Intel VT-x) support
    pub fn has_vmx(&self) -> bool {
        (self.features_basic & (1 << 22)) != 0
    }

    /// Check for SVM (AMD-V) support
    pub fn has_svm(&self) -> bool {
        (self.features_amd & (1 << 0)) != 0
    }

    /// Check for any hardware virtualization
    pub fn has_hardware_virt(&self) -> bool {
        self.has_vmx() || self.has_svm()
    }

    // =========================================================================
    // Timing Features
    // =========================================================================

    /// Check for invariant TSC
    pub fn has_invariant_tsc(&self) -> bool {
        (self.features_ext & (1 << 14)) != 0
    }

    /// Check for RDTSCP
    pub fn has_rdtscp(&self) -> bool {
        (self.features_ext & (1 << 13)) != 0
    }

    /// Check for TSC-deadline timer
    pub fn has_tsc_deadline(&self) -> bool {
        (self.features_ext & (1 << 15)) != 0
    }

    // =========================================================================
    // Crypto Features
    // =========================================================================

    /// Check for AES-NI
    pub fn has_aesni(&self) -> bool {
        (self.features_ext & (1 << 18)) != 0
    }

    /// Check for SHA extensions
    pub fn has_sha(&self) -> bool {
        (self.features_ext & (1 << 19)) != 0
    }

    /// Check for RDRAND
    pub fn has_rdrand(&self) -> bool {
        (self.features_ext & (1 << 16)) != 0
    }

    /// Check for RDSEED
    pub fn has_rdseed(&self) -> bool {
        (self.features_ext & (1 << 17)) != 0
    }

    // =========================================================================
    // Other Features
    // =========================================================================

    /// Check for APIC
    pub fn has_apic(&self) -> bool {
        (self.features_basic & (1 << 10)) != 0
    }

    /// Check for x2APIC
    pub fn has_x2apic(&self) -> bool {
        (self.features_basic & (1 << 11)) != 0
    }

    /// Check for FSGSBASE instructions
    pub fn has_fsgsbase(&self) -> bool {
        (self.features_ext & (1 << 11)) != 0
    }

    /// Check for XSAVE
    pub fn has_xsave(&self) -> bool {
        (self.features_basic & (1 << 18)) != 0
    }

    // =========================================================================
    // Cache Information
    // =========================================================================

    /// Get L1 data cache size in bytes
    pub fn l1d_cache_size(&self) -> u32 {
        self.l1d_size
    }

    /// Get L1 instruction cache size in bytes
    pub fn l1i_cache_size(&self) -> u32 {
        self.l1i_size
    }

    /// Get L2 cache size in bytes
    pub fn l2_cache_size(&self) -> u32 {
        self.l2_size
    }

    /// Get L3 cache size in bytes
    pub fn l3_cache_size(&self) -> u32 {
        self.l3_size
    }

    /// Get cache line size in bytes
    pub fn cache_line_size(&self) -> u8 {
        self.cache_line_size
    }

    // =========================================================================
    // Topology
    // =========================================================================

    /// Get number of logical CPU cores
    pub fn logical_cores(&self) -> u16 {
        self.logical_cores
    }

    /// Get number of physical CPU cores
    pub fn physical_cores(&self) -> u16 {
        self.physical_cores
    }

    /// Check if hyperthreading/SMT is available
    pub fn has_hyperthreading(&self) -> bool {
        self.logical_cores > self.physical_cores
    }
}

// =============================================================================
// FEATURE REQUIREMENTS
// =============================================================================

/// Required features for a given kernel configuration
#[derive(Debug, Clone, Copy)]
pub struct FeatureRequirements {
    /// Minimum physical address bits
    pub min_phys_bits: u8,
    /// Require NX bit
    pub require_nx: bool,
    /// Require APIC
    pub require_apic: bool,
    /// Require x2APIC
    pub require_x2apic: bool,
    /// Require invariant TSC
    pub require_invariant_tsc: bool,
    /// Require SMEP
    pub require_smep: bool,
    /// Require SMAP
    pub require_smap: bool,
    /// Require XSAVE
    pub require_xsave: bool,
    /// Require FSGSBASE
    pub require_fsgsbase: bool,
    /// Require PCID
    pub require_pcid: bool,
}

impl Default for FeatureRequirements {
    fn default() -> Self {
        Self {
            min_phys_bits: 36,
            require_nx: true,
            require_apic: true,
            require_x2apic: false,
            require_invariant_tsc: false,
            require_smep: false,
            require_smap: false,
            require_xsave: false,
            require_fsgsbase: false,
            require_pcid: false,
        }
    }
}

impl FeatureRequirements {
    /// Strict requirements for modern kernels
    pub fn strict() -> Self {
        Self {
            min_phys_bits: 40,
            require_nx: true,
            require_apic: true,
            require_x2apic: true,
            require_invariant_tsc: true,
            require_smep: true,
            require_smap: true,
            require_xsave: true,
            require_fsgsbase: true,
            require_pcid: true,
        }
    }

    /// Check if capabilities meet requirements
    pub fn check(&self, caps: &CpuCapabilities) -> Result<(), MissingFeature> {
        if caps.phys_addr_bits() < self.min_phys_bits {
            return Err(MissingFeature::PhysAddrBits {
                required: self.min_phys_bits,
                available: caps.phys_addr_bits(),
            });
        }

        if self.require_nx && !caps.has_nx() {
            return Err(MissingFeature::Feature("NX"));
        }

        if self.require_apic && !caps.has_apic() {
            return Err(MissingFeature::Feature("APIC"));
        }

        if self.require_x2apic && !caps.has_x2apic() {
            return Err(MissingFeature::Feature("x2APIC"));
        }

        if self.require_invariant_tsc && !caps.has_invariant_tsc() {
            return Err(MissingFeature::Feature("Invariant TSC"));
        }

        if self.require_smep && !caps.has_smep() {
            return Err(MissingFeature::Feature("SMEP"));
        }

        if self.require_smap && !caps.has_smap() {
            return Err(MissingFeature::Feature("SMAP"));
        }

        if self.require_xsave && !caps.has_xsave() {
            return Err(MissingFeature::Feature("XSAVE"));
        }

        if self.require_fsgsbase && !caps.has_fsgsbase() {
            return Err(MissingFeature::Feature("FSGSBASE"));
        }

        if self.require_pcid && !caps.has_pcid() {
            return Err(MissingFeature::Feature("PCID"));
        }

        Ok(())
    }
}

/// Missing feature error
#[derive(Debug, Clone, Copy)]
pub enum MissingFeature {
    /// Physical address bits insufficient
    PhysAddrBits { required: u8, available: u8 },
    /// Named feature missing
    Feature(&'static str),
}

impl core::fmt::Display for MissingFeature {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PhysAddrBits {
                required,
                available,
            } => {
                write!(
                    f,
                    "Physical address bits: {} required, {} available",
                    required, available
                )
            },
            Self::Feature(name) => {
                write!(f, "CPU feature '{}' is required but not available", name)
            },
        }
    }
}

// =============================================================================
// FEATURE REPORT
// =============================================================================

/// Generate a text report of CPU capabilities
pub fn generate_report(caps: &CpuCapabilities) -> alloc::string::String {
    use alloc::format;
    use alloc::string::String;

    let mut report = String::new();

    report.push_str("=== CPU Capabilities Report ===\n\n");

    // Identification
    report.push_str(&format!("Vendor: {:?}\n", caps.vendor()));
    report.push_str(&format!(
        "Family: {}, Model: {}, Stepping: {}\n",
        caps.family(),
        caps.model(),
        caps.stepping()
    ));
    report.push_str(&format!(
        "Physical Address Bits: {}\n",
        caps.phys_addr_bits()
    ));
    report.push_str(&format!(
        "Virtual Address Bits: {}\n\n",
        caps.virt_addr_bits()
    ));

    // SIMD
    report.push_str("SIMD Features:\n");
    report.push_str(&format!(
        "  SSE:     {}\n",
        if caps.has_sse() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  SSE2:    {}\n",
        if caps.has_sse2() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  SSE3:    {}\n",
        if caps.has_sse3() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  SSSE3:   {}\n",
        if caps.has_ssse3() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  SSE4.1:  {}\n",
        if caps.has_sse4_1() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  SSE4.2:  {}\n",
        if caps.has_sse4_2() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  AVX:     {}\n",
        if caps.has_avx() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  AVX2:    {}\n",
        if caps.has_avx2() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  AVX-512: {}\n\n",
        if caps.has_avx512() { "✓" } else { "✗" }
    ));

    // Security
    report.push_str("Security Features:\n");
    report.push_str(&format!(
        "  NX:    {}\n",
        if caps.has_nx() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  SMEP:  {}\n",
        if caps.has_smep() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  SMAP:  {}\n",
        if caps.has_smap() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  UMIP:  {}\n",
        if caps.has_umip() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  PKU:   {}\n\n",
        if caps.has_pku() { "✓" } else { "✗" }
    ));

    // Memory
    report.push_str("Memory Features:\n");
    report.push_str(&format!(
        "  1GB Pages: {}\n",
        if caps.has_1gb_pages() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  LA57:      {}\n",
        if caps.supports_la57() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  PCID:      {}\n",
        if caps.has_pcid() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  INVPCID:   {}\n\n",
        if caps.has_invpcid() { "✓" } else { "✗" }
    ));

    // Timing
    report.push_str("Timing Features:\n");
    report.push_str(&format!(
        "  Invariant TSC: {}\n",
        if caps.has_invariant_tsc() {
            "✓"
        } else {
            "✗"
        }
    ));
    report.push_str(&format!(
        "  RDTSCP:        {}\n",
        if caps.has_rdtscp() { "✓" } else { "✗" }
    ));
    report.push_str(&format!(
        "  TSC-Deadline:  {}\n\n",
        if caps.has_tsc_deadline() {
            "✓"
        } else {
            "✗"
        }
    ));

    // Cache
    report.push_str("Cache:\n");
    report.push_str(&format!("  L1D: {} KB\n", caps.l1d_cache_size() / 1024));
    report.push_str(&format!("  L1I: {} KB\n", caps.l1i_cache_size() / 1024));
    report.push_str(&format!("  L2:  {} KB\n", caps.l2_cache_size() / 1024));
    report.push_str(&format!("  L3:  {} KB\n", caps.l3_cache_size() / 1024));
    report.push_str(&format!(
        "  Line Size: {} bytes\n\n",
        caps.cache_line_size()
    ));

    // Topology
    report.push_str("Topology:\n");
    report.push_str(&format!("  Logical Cores:  {}\n", caps.logical_cores()));
    report.push_str(&format!("  Physical Cores: {}\n", caps.physical_cores()));
    report.push_str(&format!(
        "  HyperThreading: {}\n",
        if caps.has_hyperthreading() {
            "✓"
        } else {
            "✗"
        }
    ));

    report
}

extern crate alloc;

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_capabilities() {
        let caps = CpuCapabilities::detect();

        // Basic x86_64 requirements
        assert!(caps.has_sse());
        assert!(caps.has_sse2());
        assert!(caps.has_apic());
        assert!(caps.has_nx());

        // Address sizes should be reasonable
        assert!(caps.phys_addr_bits() >= 36);
        assert!(caps.virt_addr_bits() >= 48);
    }

    #[test]
    fn test_default_requirements() {
        let caps = CpuCapabilities::detect();
        let reqs = FeatureRequirements::default();

        // Default requirements should pass on any x86_64
        assert!(reqs.check(&caps).is_ok());
    }

    #[test]
    fn test_simd_width() {
        let caps = CpuCapabilities::detect();

        // At minimum SSE (16 bytes)
        assert!(caps.max_simd_width() >= 16);
    }
}
