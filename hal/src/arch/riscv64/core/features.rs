//! # RISC-V ISA Feature Detection
//!
//! This module provides detection and management of RISC-V ISA extensions
//! and CPU features.
//!
//! ## Standard Extensions
//!
//! - **M**: Integer Multiply/Divide
//! - **A**: Atomic Instructions
//! - **F**: Single-Precision Floating-Point
//! - **D**: Double-Precision Floating-Point
//! - **C**: Compressed Instructions
//! - **V**: Vector Extension
//! - **H**: Hypervisor Extension
//! - **Zicsr**: CSR Instructions
//! - **Zifencei**: Instruction-Fetch Fence
//! - **Zba/Zbb/Zbc/Zbs**: Bit Manipulation
//! - **Svpbmt**: Page-Based Memory Types
//! - **Svinval**: Fine-Grained Address-Translation Cache Invalidation

use super::csr::misa;
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// Feature Flags Storage
// ============================================================================

/// Cached MISA value (read once at boot)
static CACHED_MISA: AtomicU64 = AtomicU64::new(0);

/// Additional feature flags detected at runtime
static FEATURE_FLAGS: AtomicU64 = AtomicU64::new(0);

// ============================================================================
// Feature Flag Bits (for FEATURE_FLAGS)
// ============================================================================

/// Runtime-detected feature flags
pub mod features {
    /// Zicsr extension (CSR instructions) - always present in S-mode
    pub const ZICSR: u64 = 1 << 0;
    /// Zifencei extension (FENCE.I instruction)
    pub const ZIFENCEI: u64 = 1 << 1;
    /// Zba extension (address generation)
    pub const ZBA: u64 = 1 << 2;
    /// Zbb extension (basic bit manipulation)
    pub const ZBB: u64 = 1 << 3;
    /// Zbc extension (carry-less multiplication)
    pub const ZBC: u64 = 1 << 4;
    /// Zbs extension (single-bit operations)
    pub const ZBS: u64 = 1 << 5;
    /// Svpbmt extension (page-based memory types)
    pub const SVPBMT: u64 = 1 << 6;
    /// Svinval extension (fine-grained TLB invalidation)
    pub const SVINVAL: u64 = 1 << 7;
    /// Sstc extension (supervisor timer compare)
    pub const SSTC: u64 = 1 << 8;
    /// Sscofpmf extension (perf counter overflow)
    pub const SSCOFPMF: u64 = 1 << 9;
    /// Sv39 paging support
    pub const SV39: u64 = 1 << 10;
    /// Sv48 paging support
    pub const SV48: u64 = 1 << 11;
    /// Sv57 paging support
    pub const SV57: u64 = 1 << 12;
    /// ASID support detected
    pub const ASID: u64 = 1 << 13;
    /// Time CSR accessible from S-mode
    pub const TIME_CSR: u64 = 1 << 14;
    /// Floating point unit present
    pub const FPU: u64 = 1 << 15;
    /// Vector unit present
    pub const VPU: u64 = 1 << 16;
    /// Hypervisor extension present
    pub const HYPERVISOR: u64 = 1 << 17;
}

// ============================================================================
// CPU Features Structure
// ============================================================================

/// Comprehensive CPU feature information
#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    /// Raw MISA value
    pub misa: u64,
    /// Additional feature flags
    pub flags: u64,
    /// Maximum XLEN (32, 64, or 128)
    pub xlen: u8,
    /// Maximum number of ASIDs supported (0 if not supported)
    pub max_asid: u16,
    /// Maximum physical address bits
    pub pa_bits: u8,
    /// Maximum virtual address bits
    pub va_bits: u8,
}

impl CpuFeatures {
    /// Create empty features (unknown)
    pub const fn empty() -> Self {
        Self {
            misa: 0,
            flags: 0,
            xlen: 64,
            max_asid: 0,
            pa_bits: 56,
            va_bits: 39,
        }
    }

    /// Check if extension is present in MISA
    #[inline]
    pub const fn has_misa(&self, ext: u64) -> bool {
        self.misa & ext != 0
    }

    /// Check if feature flag is set
    #[inline]
    pub const fn has_feature(&self, flag: u64) -> bool {
        self.flags & flag != 0
    }

    // Standard extension checks

    /// Integer multiply/divide (M extension)
    #[inline]
    pub const fn has_multiply(&self) -> bool {
        self.has_misa(misa::M)
    }

    /// Atomic instructions (A extension)
    #[inline]
    pub const fn has_atomic(&self) -> bool {
        self.has_misa(misa::A)
    }

    /// Single-precision FP (F extension)
    #[inline]
    pub const fn has_float(&self) -> bool {
        self.has_misa(misa::F)
    }

    /// Double-precision FP (D extension)
    #[inline]
    pub const fn has_double(&self) -> bool {
        self.has_misa(misa::D)
    }

    /// Compressed instructions (C extension)
    #[inline]
    pub const fn has_compressed(&self) -> bool {
        self.has_misa(misa::C)
    }

    /// Vector extension (V extension)
    #[inline]
    pub const fn has_vector(&self) -> bool {
        self.has_misa(misa::V)
    }

    /// Hypervisor extension (H extension)
    #[inline]
    pub const fn has_hypervisor(&self) -> bool {
        self.has_misa(misa::H)
    }

    /// Supervisor mode (S extension)
    #[inline]
    pub const fn has_supervisor(&self) -> bool {
        self.has_misa(misa::S)
    }

    /// User mode (U extension)
    #[inline]
    pub const fn has_user(&self) -> bool {
        self.has_misa(misa::U)
    }

    // Paging mode checks

    /// Sv39 paging supported
    #[inline]
    pub const fn has_sv39(&self) -> bool {
        self.has_feature(features::SV39)
    }

    /// Sv48 paging supported
    #[inline]
    pub const fn has_sv48(&self) -> bool {
        self.has_feature(features::SV48)
    }

    /// Sv57 paging supported
    #[inline]
    pub const fn has_sv57(&self) -> bool {
        self.has_feature(features::SV57)
    }

    /// ASID support
    #[inline]
    pub const fn has_asid(&self) -> bool {
        self.max_asid > 0
    }
}

impl Default for CpuFeatures {
    fn default() -> Self {
        Self::empty()
    }
}

// ============================================================================
// Feature Detection Functions
// ============================================================================

/// Initialize feature detection (call once at boot)
///
/// Note: MISA is only readable from M-mode on most implementations.
/// In S-mode, we rely on device tree or SBI to get feature info.
pub fn init_features() {
    // Try to probe paging modes
    probe_paging_modes();

    // Set baseline features that are always present
    set_feature(features::ZICSR); // CSR instructions always present

    // Assume FENCE.I is present (standard)
    set_feature(features::ZIFENCEI);
}

/// Initialize features from device tree info
pub fn init_from_isa_string(isa: &str) {
    let mut flags = features::ZICSR; // Always present

    // Parse ISA string (e.g., "rv64imafdc_zicsr_zifencei")
    let lower = isa.to_ascii_lowercase();

    // Check base extensions
    if lower.contains('m') {
        CACHED_MISA.fetch_or(misa::M, Ordering::Relaxed);
    }
    if lower.contains('a') {
        CACHED_MISA.fetch_or(misa::A, Ordering::Relaxed);
    }
    if lower.contains('f') {
        CACHED_MISA.fetch_or(misa::F, Ordering::Relaxed);
        flags |= features::FPU;
    }
    if lower.contains('d') {
        CACHED_MISA.fetch_or(misa::D, Ordering::Relaxed);
        flags |= features::FPU;
    }
    if lower.contains('c') {
        CACHED_MISA.fetch_or(misa::C, Ordering::Relaxed);
    }
    if lower.contains('v') {
        CACHED_MISA.fetch_or(misa::V, Ordering::Relaxed);
        flags |= features::VPU;
    }
    if lower.contains('h') {
        CACHED_MISA.fetch_or(misa::H, Ordering::Relaxed);
        flags |= features::HYPERVISOR;
    }
    if lower.contains('s') {
        CACHED_MISA.fetch_or(misa::S, Ordering::Relaxed);
    }
    if lower.contains('u') {
        CACHED_MISA.fetch_or(misa::U, Ordering::Relaxed);
    }

    // Check named extensions
    if lower.contains("zifencei") {
        flags |= features::ZIFENCEI;
    }
    if lower.contains("zba") {
        flags |= features::ZBA;
    }
    if lower.contains("zbb") {
        flags |= features::ZBB;
    }
    if lower.contains("zbc") {
        flags |= features::ZBC;
    }
    if lower.contains("zbs") {
        flags |= features::ZBS;
    }
    if lower.contains("svpbmt") {
        flags |= features::SVPBMT;
    }
    if lower.contains("svinval") {
        flags |= features::SVINVAL;
    }
    if lower.contains("sstc") {
        flags |= features::SSTC;
    }

    FEATURE_FLAGS.fetch_or(flags, Ordering::Relaxed);
}

/// Probe supported paging modes by testing SATP
fn probe_paging_modes() {
    use super::csr::{read_satp, write_satp, satp};

    // Save original SATP
    let original = read_satp();

    // Try Sv39
    unsafe {
        write_satp(satp::MODE_SV39 << satp::MODE_SHIFT);
        let result = read_satp();
        if (result >> satp::MODE_SHIFT) & 0xF == satp::MODE_SV39 {
            set_feature(features::SV39);
        }
    }

    // Try Sv48
    unsafe {
        write_satp(satp::MODE_SV48 << satp::MODE_SHIFT);
        let result = read_satp();
        if (result >> satp::MODE_SHIFT) & 0xF == satp::MODE_SV48 {
            set_feature(features::SV48);
        }
    }

    // Try Sv57
    unsafe {
        write_satp(satp::MODE_SV57 << satp::MODE_SHIFT);
        let result = read_satp();
        if (result >> satp::MODE_SHIFT) & 0xF == satp::MODE_SV57 {
            set_feature(features::SV57);
        }
    }

    // Restore original SATP
    write_satp(original);
}

/// Set a feature flag
#[inline]
pub fn set_feature(flag: u64) {
    FEATURE_FLAGS.fetch_or(flag, Ordering::Relaxed);
}

/// Clear a feature flag
#[inline]
pub fn clear_feature(flag: u64) {
    FEATURE_FLAGS.fetch_and(!flag, Ordering::Relaxed);
}

/// Check if a feature is present
#[inline]
pub fn has_feature(flag: u64) -> bool {
    FEATURE_FLAGS.load(Ordering::Relaxed) & flag != 0
}

/// Get cached MISA value
#[inline]
pub fn get_misa() -> u64 {
    CACHED_MISA.load(Ordering::Relaxed)
}

/// Set cached MISA value (from device tree or SBI)
#[inline]
pub fn set_misa(value: u64) {
    CACHED_MISA.store(value, Ordering::Relaxed);
}

/// Get all feature flags
#[inline]
pub fn get_features() -> u64 {
    FEATURE_FLAGS.load(Ordering::Relaxed)
}

/// Get comprehensive CPU features
pub fn get_cpu_features() -> CpuFeatures {
    let misa = get_misa();
    let flags = get_features();

    // Determine VA bits from paging mode
    let va_bits = if flags & features::SV57 != 0 {
        57
    } else if flags & features::SV48 != 0 {
        48
    } else if flags & features::SV39 != 0 {
        39
    } else {
        39 // Default assumption
    };

    // Determine ASID count (try to detect)
    let max_asid = if flags & features::ASID != 0 {
        65535 // 16-bit ASID
    } else {
        0
    };

    CpuFeatures {
        misa,
        flags,
        xlen: 64,
        max_asid,
        pa_bits: 56, // Sv39/48 max
        va_bits,
    }
}

// ============================================================================
// Extension Name Helpers
// ============================================================================

/// Get human-readable extension list from MISA
pub fn misa_extensions_string(misa: u64) -> &'static str {
    // This is simplified - real impl would build dynamic string
    match misa & (misa::I | misa::M | misa::A | misa::F | misa::D | misa::C) {
        x if x == misa::I | misa::M | misa::A | misa::F | misa::D | misa::C => "IMAFDC (G)",
        x if x == misa::I | misa::M | misa::A | misa::F | misa::D => "IMAFD",
        x if x == misa::I | misa::M | misa::A => "IMA",
        x if x == misa::I | misa::M => "IM",
        _ => "I+",
    }
}

// ============================================================================
// Trap-based Feature Detection
// ============================================================================

/// Result of attempting to execute an instruction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeResult {
    /// Instruction executed successfully
    Supported,
    /// Instruction caused illegal instruction trap
    NotSupported,
    /// Probe failed for unknown reason
    Unknown,
}

/// Probe if an extension is supported by trying to execute an instruction
///
/// # Safety
/// This temporarily modifies trap handlers and may cause exceptions.
#[cfg(feature = "probe_extensions")]
pub unsafe fn probe_extension(test_fn: fn() -> bool) -> ProbeResult {
    // This would require setting up a temporary trap handler
    // and catching illegal instruction exceptions
    // Implementation depends on trap infrastructure
    ProbeResult::Unknown
}

// ============================================================================
// Vector Extension Info
// ============================================================================

/// Vector extension configuration
#[derive(Debug, Clone, Copy, Default)]
pub struct VectorConfig {
    /// Vector register length in bits (vlenb * 8)
    pub vlen: u16,
    /// Maximum element width in bits
    pub elen: u16,
    /// Vector register count
    pub vnum: u8,
}

/// Get vector extension configuration (if V extension present)
#[cfg(feature = "vector")]
pub fn get_vector_config() -> Option<VectorConfig> {
    if !has_feature(features::VPU) {
        return None;
    }

    // Read vlenb CSR
    let vlenb: u64;
    unsafe {
        core::arch::asm!("csrr {}, vlenb", out(reg) vlenb);
    }

    Some(VectorConfig {
        vlen: (vlenb * 8) as u16,
        elen: 64, // Standard ELEN
        vnum: 32, // Standard register count
    })
}

// ============================================================================
// Platform-Specific Feature Overrides
// ============================================================================

/// Platform feature overrides
pub struct PlatformFeatures {
    /// Force-enable features
    pub force_enable: u64,
    /// Force-disable features
    pub force_disable: u64,
}

impl PlatformFeatures {
    /// Apply platform-specific overrides
    pub fn apply(&self) {
        FEATURE_FLAGS.fetch_or(self.force_enable, Ordering::Relaxed);
        FEATURE_FLAGS.fetch_and(!self.force_disable, Ordering::Relaxed);
    }
}

/// QEMU virt platform features
pub const QEMU_VIRT_FEATURES: PlatformFeatures = PlatformFeatures {
    force_enable: features::SV39 | features::SV48 | features::ZIFENCEI | features::TIME_CSR,
    force_disable: 0,
};

/// SiFive U74 features
pub const SIFIVE_U74_FEATURES: PlatformFeatures = PlatformFeatures {
    force_enable: features::SV39 | features::ZIFENCEI | features::ASID,
    force_disable: features::SV48 | features::SV57,
};
