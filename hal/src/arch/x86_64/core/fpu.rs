//! # FPU/SSE/AVX State Management
//!
//! x87 FPU, SSE, AVX, and extended state management.
//!
//! ## Overview
//!
//! Modern x86_64 processors have extensive floating-point and SIMD state:
//!
//! - **x87 FPU**: Legacy floating-point (80-bit registers)
//! - **SSE**: 128-bit XMM registers (XMM0-XMM15)
//! - **AVX**: 256-bit YMM registers (extends XMM)
//! - **AVX-512**: 512-bit ZMM registers (ZMM0-ZMM31) + opmask registers
//!
//! This module handles:
//!
//! - State save/restore (FXSAVE/FXRSTOR, XSAVE/XRSTOR)
//! - Lazy FPU context switching
//! - XSAVE area management
//! - FPU exception handling

use core::arch::asm;

use super::control_regs::{Cr0, Cr4, Xcr0};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Size of FXSAVE area (512 bytes)
pub const FXSAVE_AREA_SIZE: usize = 512;

/// Default MXCSR value (all exceptions masked)
pub const DEFAULT_MXCSR: u32 = 0x1F80;

/// MXCSR mask for valid bits
pub const MXCSR_MASK: u32 = 0xFFFF_FFC0;

// =============================================================================
// FXSAVE AREA (Legacy 512-byte format)
// =============================================================================

/// FXSAVE/FXRSTOR area (512 bytes, 16-byte aligned)
///
/// This is the legacy format for saving x87/SSE state.
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct FxSaveArea {
    /// x87 FPU Control Word
    pub fcw: u16,
    /// x87 FPU Status Word
    pub fsw: u16,
    /// x87 FPU Tag Word (abridged)
    pub ftw: u8,
    /// Reserved
    pub _reserved1: u8,
    /// x87 FPU Opcode
    pub fop: u16,
    /// x87 FPU Instruction Pointer (low 32 bits)
    pub fip: u32,
    /// x87 FPU Instruction Pointer (high 16 bits) / CS
    pub fcs: u16,
    /// Reserved
    pub _reserved2: u16,
    /// x87 FPU Data Pointer (low 32 bits)
    pub fdp: u32,
    /// x87 FPU Data Pointer (high 16 bits) / DS
    pub fds: u16,
    /// Reserved
    pub _reserved3: u16,
    /// MXCSR Register State
    pub mxcsr: u32,
    /// MXCSR Mask
    pub mxcsr_mask: u32,
    /// x87 FPU/MMX registers (ST0-ST7 / MM0-MM7)
    pub st_mm: [[u8; 16]; 8],
    /// XMM registers (XMM0-XMM15)
    pub xmm: [[u8; 16]; 16],
    /// Reserved space
    pub _reserved4: [u8; 96],
}

impl FxSaveArea {
    /// Create a new zeroed FXSAVE area
    pub const fn new() -> Self {
        Self {
            fcw: 0x037F, // Default control word
            fsw: 0,
            ftw: 0,
            _reserved1: 0,
            fop: 0,
            fip: 0,
            fcs: 0,
            _reserved2: 0,
            fdp: 0,
            fds: 0,
            _reserved3: 0,
            mxcsr: DEFAULT_MXCSR,
            mxcsr_mask: MXCSR_MASK,
            st_mm: [[0; 16]; 8],
            xmm: [[0; 16]; 16],
            _reserved4: [0; 96],
        }
    }

    /// Save FPU/SSE state
    ///
    /// # Safety
    /// Must have OSFXSR bit set in CR4.
    #[inline]
    pub unsafe fn save(&mut self) {
        unsafe {
            asm!(
                "fxsave64 [{}]",
                in(reg) self as *mut Self,
                options(nostack, preserves_flags)
            );
        }
    }

    /// Restore FPU/SSE state
    ///
    /// # Safety
    /// Must have OSFXSR bit set in CR4.
    /// Area must contain valid state.
    #[inline]
    pub unsafe fn restore(&self) {
        unsafe {
            asm!(
                "fxrstor64 [{}]",
                in(reg) self as *const Self,
                options(nostack, preserves_flags)
            );
        }
    }
}

impl Default for FxSaveArea {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for FxSaveArea {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FxSaveArea")
            .field("fcw", &format_args!("{:#06x}", self.fcw))
            .field("fsw", &format_args!("{:#06x}", self.fsw))
            .field("ftw", &self.ftw)
            .field("mxcsr", &format_args!("{:#010x}", self.mxcsr))
            .finish()
    }
}

// =============================================================================
// XSAVE AREA (Extended format)
// =============================================================================

/// XSAVE header (64 bytes)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct XsaveHeader {
    /// State-component bitmap (which components are valid)
    pub xstate_bv: u64,
    /// State-component bitmap (which components are in compacted format)
    pub xcomp_bv: u64,
    /// Reserved
    pub _reserved: [u64; 6],
}

impl XsaveHeader {
    pub const fn new() -> Self {
        Self {
            xstate_bv: 0,
            xcomp_bv: 0,
            _reserved: [0; 6],
        }
    }
}

impl Default for XsaveHeader {
    fn default() -> Self {
        Self::new()
    }
}

/// XSAVE area (variable size)
///
/// The actual size depends on enabled features.
/// Minimum: 576 bytes (legacy + header)
/// Maximum: depends on CPU features (can be >8KB with AVX-512)
#[repr(C, align(64))]
pub struct XsaveArea {
    /// Legacy FPU/SSE area (512 bytes)
    pub legacy: FxSaveArea,
    /// XSAVE header (64 bytes)
    pub header: XsaveHeader,
    /// Extended region (variable size)
    /// Contains AVX, AVX-512, MPX, etc.
    extended: [u8; 0], // Zero-size for now, actual allocation varies
}

impl XsaveArea {
    /// Get required size for current CPU configuration
    pub fn required_size() -> usize {
        use super::cpuid::{cpuid_count, leaf};

        let result = cpuid_count(leaf::XSAVE, 0);
        result.ebx as usize // Current enabled size
    }

    /// Get maximum size for all possible features
    pub fn max_size() -> usize {
        use super::cpuid::{cpuid_count, leaf};

        let result = cpuid_count(leaf::XSAVE, 0);
        result.ecx as usize // Maximum possible size
    }
}

// =============================================================================
// XSAVE OPERATIONS
// =============================================================================

/// Save extended state using XSAVE
///
/// # Arguments
/// * `area` - Pointer to 64-byte aligned XSAVE area
/// * `mask` - Which components to save (XCR0 bits)
///
/// # Safety
/// - CR4.OSXSAVE must be set
/// - Area must be properly sized and aligned
#[inline]
pub unsafe fn xsave(area: *mut u8, mask: u64) {
    let lo = mask as u32;
    let hi = (mask >> 32) as u32;
    unsafe {
        asm!(
            "xsave64 [{}]",
            in(reg) area,
            in("eax") lo,
            in("edx") hi,
            options(nostack, preserves_flags)
        );
    }
}

/// Optimized save (XSAVEOPT)
///
/// # Safety
/// Same as `xsave`.
#[inline]
pub unsafe fn xsaveopt(area: *mut u8, mask: u64) {
    let lo = mask as u32;
    let hi = (mask >> 32) as u32;
    unsafe {
        asm!(
            "xsaveopt64 [{}]",
            in(reg) area,
            in("eax") lo,
            in("edx") hi,
            options(nostack, preserves_flags)
        );
    }
}

/// Compacted save (XSAVEC)
///
/// Saves state in compacted format (no holes).
///
/// # Safety
/// Same as `xsave`. Requires XSAVEC support.
#[inline]
pub unsafe fn xsavec(area: *mut u8, mask: u64) {
    let lo = mask as u32;
    let hi = (mask >> 32) as u32;
    unsafe {
        asm!(
            "xsavec64 [{}]",
            in(reg) area,
            in("eax") lo,
            in("edx") hi,
            options(nostack, preserves_flags)
        );
    }
}

/// Supervisor save (XSAVES)
///
/// Saves supervisor state components.
///
/// # Safety
/// Same as `xsave`. Requires XSAVES support.
#[inline]
pub unsafe fn xsaves(area: *mut u8, mask: u64) {
    let lo = mask as u32;
    let hi = (mask >> 32) as u32;
    unsafe {
        asm!(
            "xsaves64 [{}]",
            in(reg) area,
            in("eax") lo,
            in("edx") hi,
            options(nostack, preserves_flags)
        );
    }
}

/// Restore extended state using XRSTOR
///
/// # Safety
/// - CR4.OSXSAVE must be set
/// - Area must contain valid state
#[inline]
pub unsafe fn xrstor(area: *const u8, mask: u64) {
    let lo = mask as u32;
    let hi = (mask >> 32) as u32;
    unsafe {
        asm!(
            "xrstor64 [{}]",
            in(reg) area,
            in("eax") lo,
            in("edx") hi,
            options(nostack, preserves_flags)
        );
    }
}

/// Supervisor restore (XRSTORS)
///
/// # Safety
/// Same as `xrstor`. Requires XRSTORS support.
#[inline]
pub unsafe fn xrstors(area: *const u8, mask: u64) {
    let lo = mask as u32;
    let hi = (mask >> 32) as u32;
    unsafe {
        asm!(
            "xrstors64 [{}]",
            in(reg) area,
            in("eax") lo,
            in("edx") hi,
            options(nostack, preserves_flags)
        );
    }
}

// =============================================================================
// MXCSR OPERATIONS
// =============================================================================

/// Read MXCSR register
#[inline]
pub fn get_mxcsr() -> u32 {
    let value: u32;
    unsafe {
        let mut tmp: u32 = 0;
        asm!(
            "stmxcsr [{}]",
            in(reg) &mut tmp,
            options(nostack, preserves_flags)
        );
        value = tmp;
    }
    value
}

/// Write MXCSR register
///
/// # Safety
/// Invalid values can cause undefined behavior.
#[inline]
pub unsafe fn set_mxcsr(value: u32) {
    unsafe {
        asm!(
            "ldmxcsr [{}]",
            in(reg) &value,
            options(nostack, preserves_flags)
        );
    }
}

/// MXCSR exception flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MxcsrExceptions {
    /// Invalid operation exception
    pub invalid: bool,
    /// Denormal operand exception
    pub denormal: bool,
    /// Divide by zero exception
    pub divzero: bool,
    /// Overflow exception
    pub overflow: bool,
    /// Underflow exception
    pub underflow: bool,
    /// Precision exception
    pub precision: bool,
}

impl MxcsrExceptions {
    /// Extract exception flags from MXCSR value
    pub fn from_mxcsr(mxcsr: u32) -> Self {
        Self {
            invalid: (mxcsr & (1 << 0)) != 0,
            denormal: (mxcsr & (1 << 1)) != 0,
            divzero: (mxcsr & (1 << 2)) != 0,
            overflow: (mxcsr & (1 << 3)) != 0,
            underflow: (mxcsr & (1 << 4)) != 0,
            precision: (mxcsr & (1 << 5)) != 0,
        }
    }

    /// Check if any exception occurred
    pub fn any(&self) -> bool {
        self.invalid
            || self.denormal
            || self.divzero
            || self.overflow
            || self.underflow
            || self.precision
    }
}

/// Clear MXCSR exception flags
pub fn clear_mxcsr_exceptions() {
    let mxcsr = get_mxcsr() & !0x3F; // Clear bits 0-5
    unsafe {
        set_mxcsr(mxcsr);
    }
}

// =============================================================================
// X87 FPU OPERATIONS
// =============================================================================

/// Initialize x87 FPU
///
/// # Safety
/// Should only be called during CPU initialization.
#[inline]
pub unsafe fn fninit() {
    unsafe {
        asm!("fninit", options(nostack, preserves_flags));
    }
}

/// Clear x87 exception flags
#[inline]
pub fn fclex() {
    unsafe {
        asm!("fnclex", options(nostack, preserves_flags));
    }
}

/// Store x87 status word
#[inline]
pub fn fstsw() -> u16 {
    let sw: u16;
    unsafe {
        asm!(
            "fnstsw {0:x}",
            out(reg) sw,
            options(nostack, preserves_flags)
        );
    }
    sw
}

/// Store x87 control word
#[inline]
pub fn fstcw() -> u16 {
    let cw: u16;
    unsafe {
        let mut tmp: u16 = 0;
        asm!(
            "fnstcw [{}]",
            in(reg) &mut tmp,
            options(nostack, preserves_flags)
        );
        cw = tmp;
    }
    cw
}

/// Load x87 control word
///
/// # Safety
/// Invalid values can cause undefined behavior.
#[inline]
pub unsafe fn fldcw(cw: u16) {
    unsafe {
        asm!(
            "fldcw [{}]",
            in(reg) &cw,
            options(nostack, preserves_flags)
        );
    }
}

/// Wait for pending x87 exceptions
#[inline]
pub fn fwait() {
    unsafe {
        asm!("fwait", options(nostack, preserves_flags));
    }
}

// =============================================================================
// FPU STATE MANAGER
// =============================================================================

/// FPU state save mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FpuSaveMode {
    /// Use FXSAVE/FXRSTOR (legacy, always available)
    Fxsave,
    /// Use XSAVE/XRSTOR
    Xsave,
    /// Use XSAVEOPT/XRSTOR (optimized)
    XsaveOpt,
    /// Use XSAVEC/XRSTOR (compacted)
    XsaveC,
}

/// Determine best FPU save mode for current CPU
pub fn detect_save_mode() -> FpuSaveMode {
    use super::cpuid::{CpuId, Feature};

    let cpuid = CpuId::new();

    if cpuid.has_feature(Feature::XSAVE) {
        if cpuid.has_feature(Feature::AVX512F) {
            // Need compacted format for large AVX-512 state
            FpuSaveMode::XsaveC
        } else {
            FpuSaveMode::XsaveOpt
        }
    } else {
        FpuSaveMode::Fxsave
    }
}

/// Initialize FPU/SSE/AVX for the current CPU
///
/// # Safety
/// Should only be called once per CPU during initialization.
pub unsafe fn init_fpu() {
    // Enable FPU
    unsafe {
        Cr0::update(|cr0| {
            cr0.remove(Cr0::EM); // No emulation
            cr0.insert(Cr0::MP); // Monitor coprocessor
            cr0.insert(Cr0::NE); // Native exceptions
            cr0.remove(Cr0::TS); // Clear task switched flag
        });
    }

    // Enable SSE/SSE2
    unsafe {
        Cr4::update(|cr4| {
            cr4.insert(Cr4::OSFXSR); // Enable FXSAVE/FXRSTOR
            cr4.insert(Cr4::OSXMMEXCPT); // Enable SSE exceptions
        });
    }

    // Initialize FPU state
    unsafe { fninit() };

    // Set default MXCSR
    unsafe { set_mxcsr(DEFAULT_MXCSR) };

    // If XSAVE is available, enable it
    let cpuid = super::cpuid::CpuId::new();
    if cpuid.has_xsave() {
        // Enable XSAVE
        unsafe {
            Cr4::update(|cr4| {
                cr4.insert(Cr4::OSXSAVE);
            });
        }

        // Enable x87 + SSE + AVX in XCR0
        let mut xcr0 = Xcr0::X87 | Xcr0::SSE;

        if cpuid.has_avx() {
            xcr0 |= Xcr0::AVX;
        }

        if cpuid.has_avx512f() {
            xcr0 |= Xcr0::OPMASK | Xcr0::ZMM_HI256 | Xcr0::HI16_ZMM;
        }

        unsafe { Xcr0::write(xcr0) };
    }
}

/// Clear TS flag to allow FPU use
///
/// Called when handling #NM (Device Not Available) exception
/// in lazy FPU context switching.
#[inline]
pub unsafe fn clear_ts() {
    unsafe {
        asm!("clts", options(nostack, preserves_flags));
    }
}

/// Set TS flag to trigger #NM on FPU use
#[inline]
pub unsafe fn set_ts() {
    unsafe {
        Cr0::update(|cr0| {
            cr0.insert(Cr0::TS);
        });
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fxsave_area_size() {
        assert_eq!(core::mem::size_of::<FxSaveArea>(), FXSAVE_AREA_SIZE);
    }

    #[test]
    fn test_fxsave_area_alignment() {
        assert_eq!(core::mem::align_of::<FxSaveArea>(), 16);
    }

    #[test]
    fn test_mxcsr_default() {
        let mxcsr = get_mxcsr();
        // All exceptions should be masked
        assert_eq!(mxcsr & 0x1F80, 0x1F80);
    }

    #[test]
    fn test_mxcsr_exceptions() {
        let exceptions = MxcsrExceptions::from_mxcsr(0);
        assert!(!exceptions.any());

        let exceptions = MxcsrExceptions::from_mxcsr(0x01);
        assert!(exceptions.invalid);
        assert!(exceptions.any());
    }

    #[test]
    fn test_fpu_status() {
        let sw = fstsw();
        // Just verify we can read it
        let _ = sw;
    }

    #[test]
    fn test_detect_save_mode() {
        let mode = detect_save_mode();
        // Should return some valid mode
        assert!(matches!(
            mode,
            FpuSaveMode::Fxsave | FpuSaveMode::Xsave | FpuSaveMode::XsaveOpt | FpuSaveMode::XsaveC
        ));
    }
}
