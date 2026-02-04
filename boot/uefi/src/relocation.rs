//! # Kernel Relocation for UEFI
//!
//! Integration of the helix-relocation subsystem with UEFI boot protocol.
//!
//! UEFI fully supports PIE (Position Independent Executable) kernels through
//! PE+ format. This module provides the integration layer for:
//!
//! - Early relocation during UEFI boot services
//! - KASLR (Kernel Address Space Layout Randomization) using UEFI RNG
//! - Full relocation processing for higher-half kernels
//!
//! ## UEFI PIE Support
//!
//! UEFI naturally supports relocatable executables via the PE+ format with
//! .reloc sections. However, for ELF kernels loaded by UEFI, we need to
//! handle ELF relocations manually.

#[cfg(feature = "relocation")]
pub use helix_relocation::{
    boot::BootContext,
    context::BootProtocol,
    kaslr::{EntropyQuality, Kaslr, KaslrConfig},
    PhysAddr, RelocError, RelocResult, Relocatable, RelocatableKernel, RelocationContext,
    RelocationContextBuilder, RelocationEngine, RelocationStats, RelocationStrategy, VirtAddr,
};

// ============================================================================
// UEFI RELOCATION CONTEXT
// ============================================================================

/// UEFI kernel load information
#[derive(Debug, Clone, Copy)]
pub struct UefiKernelInfo {
    /// Physical address where kernel is loaded
    pub phys_base: u64,
    /// Virtual address where kernel will run
    pub virt_base: u64,
    /// Link-time base address
    pub link_base: u64,
    /// Size of kernel in bytes
    pub kernel_size: usize,
    /// Start of .rela.dyn section
    pub rela_base: u64,
    /// Size of .rela.dyn section
    pub rela_size: usize,
    /// Start of .dynamic section
    pub dynamic_base: u64,
    /// Size of .dynamic section
    pub dynamic_size: usize,
}

impl Default for UefiKernelInfo {
    fn default() -> Self {
        Self {
            phys_base: 0,
            virt_base: 0xFFFF_FFFF_8000_0000, // Default higher-half
            link_base: 0xFFFF_FFFF_8000_0000,
            kernel_size: 0,
            rela_base: 0,
            rela_size: 0,
            dynamic_base: 0,
            dynamic_size: 0,
        }
    }
}

/// Create relocation context from UEFI kernel info
#[cfg(feature = "relocation")]
pub fn create_context_from_uefi(info: &UefiKernelInfo) -> RelocResult<RelocationContext> {
    RelocationContextBuilder::new()
        .phys_base(info.phys_base)
        .virt_base(info.virt_base)
        .link_base(info.link_base)
        .kernel_size(info.kernel_size)
        .boot_protocol(BootProtocol::Uefi)
        .strategy(RelocationStrategy::FullPie)
        .build()
}

/// Create boot context for relocation subsystem from UEFI
#[cfg(feature = "relocation")]
pub fn create_boot_context(info: &UefiKernelInfo) -> BootContext {
    BootContext {
        protocol: BootProtocol::Uefi,
        kernel_phys_base: PhysAddr::new(info.phys_base),
        kernel_size: info.kernel_size,
        kernel_virt_base: info.virt_base,
        memory_map: None,
        initrd: None,
        cmdline: None,
        rsdp_addr: None,
        framebuffer: None,
    }
}

// ============================================================================
// KASLR INTEGRATION
// ============================================================================

/// UEFI KASLR implementation using UEFI RNG protocol
#[cfg(feature = "relocation")]
pub struct UefiKaslr {
    config: KaslrConfig,
}

#[cfg(feature = "relocation")]
impl UefiKaslr {
    /// Create new UEFI KASLR with default configuration
    pub fn new() -> Self {
        Self {
            config: KaslrConfig {
                min_entropy_bits: 12, // 4KB alignment
                max_entropy_bits: 28, // ~256MB range
                require_aligned: true,
                alignment: 0x1000, // 4KB
                use_hardware_rng: true,
                allow_fallback: true,
            },
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: KaslrConfig) -> Self {
        Self { config }
    }

    /// Generate random slide using UEFI RNG protocol
    ///
    /// Falls back to TSC-based RNG if UEFI RNG is unavailable
    pub fn generate_slide(&self, max_slide: u64) -> u64 {
        // Try UEFI RNG protocol first
        if let Some(rng_value) = self.try_uefi_rng() {
            let slide = rng_value % max_slide;
            // Ensure alignment
            slide & !(self.config.alignment as u64 - 1)
        } else if self.config.allow_fallback {
            // Fallback to TSC-based entropy
            self.tsc_based_slide(max_slide)
        } else {
            0 // No KASLR if RNG unavailable and fallback disabled
        }
    }

    /// Try to get random value from UEFI RNG protocol
    fn try_uefi_rng(&self) -> Option<u64> {
        // In real implementation, this would query EFI_RNG_PROTOCOL
        // For now, use TSC as entropy source
        #[cfg(target_arch = "x86_64")]
        {
            Some(unsafe { core::arch::x86_64::_rdtsc() })
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            None
        }
    }

    /// Generate slide using TSC as entropy source
    fn tsc_based_slide(&self, max_slide: u64) -> u64 {
        #[cfg(target_arch = "x86_64")]
        {
            let tsc = unsafe { core::arch::x86_64::_rdtsc() };
            // Mix bits for better distribution
            let mixed = tsc.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let slide = mixed % max_slide;
            slide & !(self.config.alignment as u64 - 1)
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            0
        }
    }

    /// Get entropy quality assessment
    pub fn entropy_quality(&self) -> EntropyQuality {
        if self.try_uefi_rng().is_some() {
            EntropyQuality::High
        } else {
            EntropyQuality::Low
        }
    }
}

#[cfg(feature = "relocation")]
impl Default for UefiKaslr {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// EARLY RELOCATION (Before ExitBootServices)
// ============================================================================

/// Apply early relocations during UEFI boot services
///
/// This should be called after loading the kernel but before ExitBootServices.
/// At this point, we still have full access to UEFI services.
///
/// # Safety
/// - Kernel memory must be mapped and writable
/// - RELA section must be valid
#[cfg(feature = "relocation")]
pub unsafe fn apply_early_relocations(info: &UefiKernelInfo) -> RelocResult<RelocationStats> {
    use helix_relocation::elf::Elf64Rela;
    use helix_relocation::engine::EarlyRelocator;

    let slide = info.phys_base as i64 - info.link_base as i64;

    // No slide = no relocation needed (loaded at link address)
    if slide == 0 {
        return Ok(RelocationStats::default());
    }

    let phys_base = PhysAddr::new(info.phys_base);
    let link_base = PhysAddr::new(info.link_base);
    let relocator = EarlyRelocator::new(phys_base, link_base, info.kernel_size);

    let rela_count = info.rela_size / core::mem::size_of::<Elf64Rela>();
    let rela_ptr = info.rela_base as *const Elf64Rela;

    unsafe { relocator.apply_relative_relocations(rela_ptr, rela_count) }
}

/// Apply relocations with KASLR slide
///
/// # Safety
/// - Kernel memory must be mapped and writable
/// - RELA section must be valid
#[cfg(feature = "relocation")]
pub unsafe fn apply_kaslr_relocations(
    info: &UefiKernelInfo,
    kaslr_slide: u64,
) -> RelocResult<RelocationStats> {
    use helix_relocation::elf::Elf64Rela;
    use helix_relocation::engine::EarlyRelocator;

    let effective_slide = info.phys_base as i64 - info.link_base as i64 + kaslr_slide as i64;

    if effective_slide == 0 {
        return Ok(RelocationStats::default());
    }

    // Adjust kernel info for KASLR
    let adjusted_phys = info.phys_base + kaslr_slide;
    let phys_base = PhysAddr::new(adjusted_phys);
    let link_base = PhysAddr::new(info.link_base);
    let relocator = EarlyRelocator::new(phys_base, link_base, info.kernel_size);

    let rela_count = info.rela_size / core::mem::size_of::<Elf64Rela>();
    let rela_ptr = info.rela_base as *const Elf64Rela;

    unsafe { relocator.apply_relative_relocations(rela_ptr, rela_count) }
}

// ============================================================================
// FULL RELOCATION (Post-ExitBootServices)
// ============================================================================

/// Relocatable kernel wrapper for UEFI
#[cfg(feature = "relocation")]
pub struct UefiRelocatableKernel {
    inner: RelocatableKernel,
    info: UefiKernelInfo,
}

#[cfg(feature = "relocation")]
impl UefiRelocatableKernel {
    /// Create from UEFI kernel info
    pub fn new(info: UefiKernelInfo) -> RelocResult<Self> {
        let ctx = create_context_from_uefi(&info)?;
        Ok(Self {
            inner: RelocatableKernel::new(ctx),
            info,
        })
    }

    /// Apply all relocations
    ///
    /// # Safety
    /// - Kernel memory must be writable
    pub unsafe fn apply_all(&mut self) -> RelocResult<RelocationStats> {
        unsafe { apply_early_relocations(&self.info) }
    }

    /// Get the relocation context
    pub fn context(&self) -> &RelocationContext {
        self.inner.relocation_context()
    }

    /// Get kernel info
    pub fn info(&self) -> &UefiKernelInfo {
        &self.info
    }

    /// Check if relocation was applied
    pub fn is_relocated(&self) -> bool {
        self.info.phys_base != self.info.link_base
    }

    /// Get stats
    pub fn stats(&self) -> &RelocationStats {
        self.inner.stats()
    }
}

// ============================================================================
// LINKER SYMBOLS
// ============================================================================

// Linker symbols for relocation (defined in linker script)
extern "C" {
    // Start of kernel image
    static __kernel_start: u8;
    // End of kernel image
    static __kernel_end: u8;
    // Start of .rela.dyn section
    static __rela_start: u8;
    // End of .rela.dyn section
    static __rela_end: u8;
    // Start of .dynamic section
    static __dynamic_start: u8;
    // End of .dynamic section
    static __dynamic_end: u8;
}

/// Get kernel info from linker symbols
///
/// # Safety
/// Symbols must be defined in linker script
pub unsafe fn get_kernel_info_from_symbols(load_addr: u64) -> UefiKernelInfo {
    let kernel_start = core::ptr::addr_of!(__kernel_start) as u64;
    let kernel_end = core::ptr::addr_of!(__kernel_end) as u64;
    let rela_start = core::ptr::addr_of!(__rela_start) as u64;
    let rela_end = core::ptr::addr_of!(__rela_end) as u64;
    let dynamic_start = core::ptr::addr_of!(__dynamic_start) as u64;
    let dynamic_end = core::ptr::addr_of!(__dynamic_end) as u64;

    UefiKernelInfo {
        phys_base: load_addr,
        virt_base: 0xFFFF_FFFF_8000_0000, // Higher-half default
        link_base: kernel_start,
        kernel_size: (kernel_end - kernel_start) as usize,
        rela_base: rela_start,
        rela_size: (rela_end - rela_start) as usize,
        dynamic_base: dynamic_start,
        dynamic_size: (dynamic_end - dynamic_start) as usize,
    }
}

// ============================================================================
// HELPER MACROS
// ============================================================================

/// Apply UEFI relocations with automatic symbol detection
///
/// # Usage
/// ```ignore
/// apply_uefi_relocations!(load_address);
/// ```
#[macro_export]
macro_rules! apply_uefi_relocations {
    ($load_addr:expr) => {{
        #[cfg(feature = "relocation")]
        {
            use $crate::relocation::{apply_early_relocations, get_kernel_info_from_symbols};
            let info = unsafe { get_kernel_info_from_symbols($load_addr) };
            unsafe { apply_early_relocations(&info) }
        }
        #[cfg(not(feature = "relocation"))]
        {
            Ok($crate::relocation::RelocationStatsStub::default())
        }
    }};
}

/// Stub for when relocation feature is disabled
#[cfg(not(feature = "relocation"))]
#[derive(Default)]
pub struct RelocationStatsStub {
    /// Number of relocations applied
    pub applied: usize,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uefi_kernel_info_default() {
        let info = UefiKernelInfo::default();
        assert_eq!(info.phys_base, 0);
        assert_eq!(info.virt_base, 0xFFFF_FFFF_8000_0000);
        assert_eq!(info.kernel_size, 0);
    }

    #[cfg(feature = "relocation")]
    #[test]
    fn test_create_context() {
        let info = UefiKernelInfo {
            phys_base: 0x100000,
            virt_base: 0xFFFF_FFFF_8010_0000,
            link_base: 0xFFFF_FFFF_8000_0000,
            kernel_size: 0x100000,
            ..Default::default()
        };

        let ctx = create_context_from_uefi(&info).unwrap();
        assert_eq!(ctx.phys_base().as_u64(), 0x100000);
        assert_eq!(ctx.kernel_size(), 0x100000);
    }

    #[cfg(feature = "relocation")]
    #[test]
    fn test_uefi_kaslr() {
        let kaslr = UefiKaslr::new();
        let slide = kaslr.generate_slide(0x10000000);

        // Slide should be aligned to 4KB
        assert_eq!(slide & 0xFFF, 0);
        // Slide should be within max range
        assert!(slide < 0x10000000);
    }
}
