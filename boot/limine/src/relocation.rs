//! # Kernel Relocation for Limine
//!
//! Integration of the helix-relocation subsystem with Limine boot protocol.
//!
//! Limine fully supports PIE kernels and can perform KASLR automatically.
//! This module provides the integration layer for manual relocation when needed.

pub use helix_relocation::boot::BootContext;
pub use helix_relocation::context::BootProtocol;
pub use helix_relocation::kaslr::{EntropyQuality, Kaslr, KaslrConfig};
pub use helix_relocation::{
    RelocError, RelocResult, Relocatable, RelocatableKernel, RelocationContext,
    RelocationContextBuilder, RelocationEngine, RelocationStats, RelocationStrategy,
};

use crate::requests::{KernelAddressRequest, LimineRequest};

// ============================================================================
// LIMINE RELOCATION CONTEXT
// ============================================================================

/// Create relocation context from Limine kernel address response
///
/// # Arguments
/// * `kernel_addr` - Limine kernel address request (must have response)
/// * `kernel_size` - Size of kernel in bytes
///
/// # Returns
/// A configured `RelocationContext` for the Limine boot environment
pub fn create_context_from_limine(
    kernel_addr: &KernelAddressRequest,
    kernel_size: usize,
) -> RelocResult<RelocationContext> {
    let response = kernel_addr.response().ok_or(RelocError::NotInitialized)?;

    // Limine provides both physical and virtual base addresses
    let phys_base = response.physical_base();
    let virt_base = response.virtual_base();

    // Calculate the slide (difference between where we were linked and where we are)
    // For Limine, this is typically zero unless KASLR is enabled
    let _slide = virt_base as i64 - phys_base as i64;

    RelocationContextBuilder::new()
        .phys_base(phys_base)
        .virt_base(virt_base)
        .link_base(virt_base) // Limine usually loads at link address
        .kernel_size(kernel_size)
        .boot_protocol(helix_relocation::context::BootProtocol::Limine)
        .strategy(RelocationStrategy::FullPie)
        .build()
}

/// Create boot context for relocation subsystem from Limine
pub fn create_boot_context(
    kernel_addr: &KernelAddressRequest,
    kernel_size: usize,
) -> RelocResult<BootContext> {
    let response = kernel_addr.response().ok_or(RelocError::NotInitialized)?;

    Ok(BootContext {
        protocol: BootProtocol::Limine,
        kernel_phys_base: helix_relocation::PhysAddr::new(response.physical_base()),
        kernel_size,
        kernel_virt_base: response.virtual_base(),
        memory_map: None, // Would be filled from memory map response
        initrd: None,
        cmdline: None,
        rsdp_addr: None,
        framebuffer: None,
    })
}

// ============================================================================
// KASLR INTEGRATION
// ============================================================================

/// KASLR configuration for Limine
///
/// Limine has built-in KASLR support (controlled via limine.conf),
/// but this provides additional randomization if needed.
pub struct LimineKaslr {
    inner: Kaslr,
    enabled: bool,
}

impl LimineKaslr {
    /// Create new KASLR engine for Limine
    pub fn new(config: KaslrConfig) -> Self {
        Self {
            inner: Kaslr::new(config),
            enabled: true,
        }
    }

    /// Create with default configuration
    pub fn default_config() -> Self {
        Self::new(KaslrConfig::default())
    }

    /// Disable KASLR (use Limine's built-in only)
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the KASLR slide value
    ///
    /// If Limine already applied KASLR, this returns 0.
    /// Otherwise, generates a new slide value.
    pub fn get_slide(&mut self, kernel_size: usize) -> RelocResult<i64> {
        if !self.enabled {
            return Ok(0);
        }
        self.inner.initialize(kernel_size)
    }

    /// Get entropy quality
    pub fn entropy_quality(&self) -> EntropyQuality {
        self.inner.entropy_quality()
    }
}

// ============================================================================
// EARLY RELOCATION (Pre-MMU)
// ============================================================================

/// Apply early relocations for Limine
///
/// Limine already handles most relocation, but this can be used for
/// additional relocations if needed.
///
/// # Safety
/// - Must be called before kernel starts modifying its own memory
/// - ELF relocation tables must be valid
/// - Kernel must be writable
#[inline(never)]
pub unsafe fn apply_early_relocations(
    rela_base: *const u8,
    rela_size: usize,
    kernel_base: u64,
    kernel_size: usize,
    slide: i64,
) -> RelocResult<RelocationStats> {
    use helix_relocation::elf::Elf64Rela;
    use helix_relocation::engine::EarlyRelocator;

    // Limine typically handles all relocations, so slide is usually 0
    if slide == 0 {
        return Ok(RelocationStats::default());
    }

    let phys_base = helix_relocation::PhysAddr::new(kernel_base);
    let link_base = helix_relocation::PhysAddr::new((kernel_base as i64 - slide) as u64);
    let relocator = EarlyRelocator::new(phys_base, link_base, kernel_size);

    let rela_count = rela_size / core::mem::size_of::<Elf64Rela>();
    let rela_ptr = rela_base as *const Elf64Rela;

    unsafe { relocator.apply_relative_relocations(rela_ptr, rela_count) }
}

// ============================================================================
// FULL RELOCATION (Post-MMU)
// ============================================================================

/// Relocatable kernel wrapper for Limine
pub struct LimineRelocatableKernel {
    inner: RelocatableKernel,
}

impl LimineRelocatableKernel {
    /// Create from Limine boot info
    pub fn from_limine(
        kernel_addr: &KernelAddressRequest,
        kernel_size: usize,
    ) -> RelocResult<Self> {
        let ctx = create_context_from_limine(kernel_addr, kernel_size)?;
        Ok(Self {
            inner: RelocatableKernel::new(ctx),
        })
    }

    /// Apply all relocations
    ///
    /// # Safety
    /// - Kernel memory must be writable
    /// - Should be called early in boot before kernel modifies itself
    pub unsafe fn apply_all(&mut self) -> RelocResult<RelocationStats> {
        // Limine already performs most relocations
        // This is for any additional processing needed
        Ok(RelocationStats::default())
    }

    /// Get the relocation context
    pub fn context(&self) -> &RelocationContext {
        self.inner.relocation_context()
    }

    /// Check if relocation was successful
    pub fn is_relocated(&self) -> bool {
        true // Limine handles relocation
    }
}

// ============================================================================
// LINKER SYMBOLS
// ============================================================================

/// Linker symbols for relocation (defined in linker script)
extern "C" {
    /// Start of kernel image
    static __kernel_start: u8;
    /// End of kernel image
    static __kernel_end: u8;
    /// Start of .rela.dyn section
    static __rela_start: u8;
    /// End of .rela.dyn section
    static __rela_end: u8;
    /// Start of .dynamic section
    static __dynamic_start: u8;
    /// End of .dynamic section
    static __dynamic_end: u8;
}

/// Get kernel start address
pub fn kernel_start() -> *const u8 {
    unsafe { core::ptr::addr_of!(__kernel_start) }
}

/// Get kernel end address
pub fn kernel_end() -> *const u8 {
    unsafe { core::ptr::addr_of!(__kernel_end) }
}

/// Get kernel size
pub fn kernel_size() -> usize {
    kernel_end() as usize - kernel_start() as usize
}

/// Get relocation table
pub fn rela_section() -> (*const u8, usize) {
    unsafe {
        let start = core::ptr::addr_of!(__rela_start);
        let end = core::ptr::addr_of!(__rela_end);
        (start, end as usize - start as usize)
    }
}

/// Get dynamic section
pub fn dynamic_section() -> (*const u8, usize) {
    unsafe {
        let start = core::ptr::addr_of!(__dynamic_start);
        let end = core::ptr::addr_of!(__dynamic_end);
        (start, end as usize - start as usize)
    }
}

// ============================================================================
// CONVENIENCE MACROS
// ============================================================================

/// Apply relocations at kernel entry
///
/// This macro should be called at the very beginning of `kernel_main`
/// if manual relocation is needed (typically not required with Limine).
#[macro_export]
macro_rules! apply_limine_relocations {
    ($kernel_addr:expr) => {{
        let kernel_size = $crate::relocation::kernel_size();
        let (rela_base, rela_size) = $crate::relocation::rela_section();

        // Get slide from Limine
        let response = $kernel_addr
            .response()
            .expect("Kernel address response required");
        let slide = response.virtual_base as i64 - response.physical_base as i64;

        // Only relocate if there's a slide
        if slide != 0 {
            unsafe {
                $crate::relocation::apply_early_relocations(
                    rela_base,
                    rela_size,
                    response.physical_base,
                    kernel_size,
                    slide,
                )
            }
        } else {
            Ok($crate::relocation::RelocationStats::default())
        }
    }};
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    #[test]
    fn test_kaslr_config() {
        let mut kaslr = LimineKaslr::default_config();
        assert!(kaslr.is_enabled());
        kaslr.disable();
        assert!(!kaslr.is_enabled());
    }
}
