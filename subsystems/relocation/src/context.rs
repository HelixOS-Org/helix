//! # Relocation Context
//!
//! Central state management for kernel relocation operations.

#[cfg(feature = "stats")]
use crate::RelocationStats;
use crate::elf::ElfInfo;
use crate::{PhysAddr, RelocError, RelocResult, VirtAddr};

// ============================================================================
// BOOT PROTOCOL
// ============================================================================

/// Boot protocol used to load the kernel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootProtocol {
    /// UEFI direct boot
    Uefi,
    /// Limine boot protocol
    Limine,
    /// Multiboot2 (GRUB)
    Multiboot2,
    /// Direct boot (no bootloader)
    DirectBoot,
    /// Unknown/Custom
    Unknown,
}

impl Default for BootProtocol {
    fn default() -> Self {
        Self::Unknown
    }
}

// ============================================================================
// RELOCATION STRATEGY
// ============================================================================

/// Strategy for applying relocations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelocationStrategy {
    /// Full PIE relocation (all types: GOT, PLT, TLS)
    FullPie,
    /// Static binary with minimal relocations
    StaticMinimal,
    /// Hybrid: PIE code sections, static data
    Hybrid,
    /// Early boot (pre-MMU, physical addresses only)
    EarlyBoot,
    /// No relocations (identity mapping)
    None,
}

impl Default for RelocationStrategy {
    fn default() -> Self {
        Self::FullPie
    }
}

// ============================================================================
// KERNEL STATE
// ============================================================================

/// Current state of kernel relocation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelState {
    /// Initial state, no relocations applied
    Initial,
    /// Early relocations applied (pre-MMU)
    EarlyRelocated,
    /// Full relocations applied
    FullyRelocated,
    /// Verified and finalized
    Finalized,
    /// Error state
    Error,
}

impl Default for KernelState {
    fn default() -> Self {
        Self::Initial
    }
}

// ============================================================================
// RELOCATION CONTEXT
// ============================================================================

/// Central context for all relocation operations
///
/// This structure holds all the state needed to perform kernel relocation,
/// including addresses, ELF information, and configuration.
#[derive(Debug)]
pub struct RelocationContext {
    /// Physical base address where kernel is loaded
    pub phys_base: PhysAddr,
    /// Virtual base address (after MMU enabled)
    pub virt_base: VirtAddr,
    /// Link-time virtual address (from linker script)
    pub link_base: VirtAddr,
    /// Total kernel size in bytes
    pub kernel_size: usize,
    /// KASLR slide value (virt_base - link_base)
    pub slide: i64,
    /// ELF information
    pub elf: Option<ElfInfo>,
    /// Relocation strategy
    pub strategy: RelocationStrategy,
    /// Boot protocol used
    pub boot_protocol: BootProtocol,
    /// Current kernel state
    pub state: KernelState,
    /// Statistics (if enabled)
    #[cfg(feature = "stats")]
    pub stats: RelocationStats,
    /// Entropy quality (for KASLR auditing)
    #[cfg(feature = "kaslr")]
    pub entropy_quality: crate::kaslr::EntropyQuality,
}

impl RelocationContext {
    /// Create a new relocation context
    pub fn new(
        phys_base: PhysAddr,
        virt_base: VirtAddr,
        link_base: VirtAddr,
        kernel_size: usize,
    ) -> Self {
        let slide = (virt_base.0 as i64).wrapping_sub(link_base.0 as i64);

        Self {
            phys_base,
            virt_base,
            link_base,
            kernel_size,
            slide,
            elf: None,
            strategy: RelocationStrategy::default(),
            boot_protocol: BootProtocol::default(),
            state: KernelState::Initial,
            #[cfg(feature = "stats")]
            stats: RelocationStats::new(),
            #[cfg(feature = "kaslr")]
            entropy_quality: crate::kaslr::EntropyQuality::None,
        }
    }

    /// Create builder for context
    pub fn builder() -> RelocationContextBuilder {
        RelocationContextBuilder::new()
    }

    /// Calculate slide from addresses
    pub fn calculate_slide(&self) -> i64 {
        self.slide
    }

    /// Check if relocation is needed
    pub fn needs_relocation(&self) -> bool {
        self.slide != 0 || matches!(self.strategy, RelocationStrategy::FullPie { .. })
    }

    /// Check if address is within kernel bounds
    pub fn is_in_bounds(&self, offset: u64) -> bool {
        offset < self.kernel_size as u64
    }

    /// Get physical address for kernel offset
    pub fn phys_addr_of(&self, offset: u64) -> Option<PhysAddr> {
        if self.is_in_bounds(offset) {
            Some(self.phys_base + offset)
        } else {
            None
        }
    }

    /// Get virtual address for kernel offset
    pub fn virt_addr_of(&self, offset: u64) -> Option<VirtAddr> {
        if self.is_in_bounds(offset) {
            Some(self.virt_base + offset)
        } else {
            None
        }
    }

    /// Transition to a new state
    pub fn transition_to(&mut self, new_state: KernelState) -> RelocResult<()> {
        // Validate state transitions
        let valid = matches!(
            (&self.state, &new_state),
            (KernelState::Initial, KernelState::EarlyRelocated)
                | (KernelState::Initial, KernelState::FullyRelocated)
                | (KernelState::EarlyRelocated, KernelState::FullyRelocated)
                | (KernelState::FullyRelocated, KernelState::Finalized)
                | (_, KernelState::Error)
        );

        if valid {
            self.state = new_state;
            Ok(())
        } else {
            Err(RelocError::NotInitialized)
        }
    }

    /// Check if kernel has been finalized
    pub fn is_finalized(&self) -> bool {
        self.state == KernelState::Finalized
    }

    /// Get kernel size in pages (4KB)
    pub fn size_in_pages(&self) -> usize {
        (self.kernel_size + 0xFFF) / 0x1000
    }

    /// Get kernel size in huge pages (2MB)
    pub fn size_in_huge_pages(&self) -> usize {
        (self.kernel_size + 0x1FFFFF) / 0x200000
    }

    /// Set ELF information
    pub fn set_elf_info(&mut self, elf: ElfInfo) {
        self.elf = Some(elf);
    }

    /// Get ELF information
    pub fn elf_info(&self) -> Option<&ElfInfo> {
        self.elf.as_ref()
    }
}

// ============================================================================
// CONTEXT BUILDER
// ============================================================================

/// Builder for RelocationContext
#[derive(Debug, Default)]
pub struct RelocationContextBuilder {
    phys_base: Option<PhysAddr>,
    virt_base: Option<VirtAddr>,
    link_base: Option<VirtAddr>,
    kernel_size: Option<usize>,
    strategy: Option<RelocationStrategy>,
    boot_protocol: Option<BootProtocol>,
}

impl RelocationContextBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set physical base address
    pub fn phys_base(mut self, addr: u64) -> Self {
        self.phys_base = Some(PhysAddr::new(addr));
        self
    }

    /// Set virtual base address
    pub fn virt_base(mut self, addr: u64) -> Self {
        self.virt_base = Some(VirtAddr::new(addr));
        self
    }

    /// Set link-time base address
    pub fn link_base(mut self, addr: u64) -> Self {
        self.link_base = Some(VirtAddr::new(addr));
        self
    }

    /// Set kernel size
    pub fn kernel_size(mut self, size: usize) -> Self {
        self.kernel_size = Some(size);
        self
    }

    /// Set relocation strategy
    pub fn strategy(mut self, strategy: RelocationStrategy) -> Self {
        self.strategy = Some(strategy);
        self
    }

    /// Set boot protocol
    pub fn boot_protocol(mut self, protocol: BootProtocol) -> Self {
        self.boot_protocol = Some(protocol);
        self
    }

    /// Build the context
    pub fn build(self) -> RelocResult<RelocationContext> {
        let phys_base = self.phys_base.ok_or(RelocError::NotInitialized)?;
        let virt_base = self.virt_base.ok_or(RelocError::NotInitialized)?;
        let link_base = self.link_base.unwrap_or(virt_base);
        let kernel_size = self.kernel_size.ok_or(RelocError::NotInitialized)?;

        // Validate alignment
        if !phys_base.is_page_aligned() {
            return Err(RelocError::InvalidAlignment {
                required: 0x1000,
                actual: phys_base.0 & 0xFFF,
            });
        }

        let mut ctx = RelocationContext::new(phys_base, virt_base, link_base, kernel_size);

        if let Some(strategy) = self.strategy {
            ctx.strategy = strategy;
        }
        if let Some(protocol) = self.boot_protocol {
            ctx.boot_protocol = protocol;
        }

        Ok(ctx)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = RelocationContext::new(
            PhysAddr::new(0x100000),
            VirtAddr::new(0xFFFFFFFF80000000),
            VirtAddr::new(0xFFFFFFFF80000000),
            0x200000,
        );

        assert_eq!(ctx.slide, 0);
        assert_eq!(ctx.kernel_size, 0x200000);
        assert!(
            !ctx.needs_relocation() || matches!(ctx.strategy, RelocationStrategy::FullPie { .. })
        );
    }

    #[test]
    fn test_context_with_slide() {
        let ctx = RelocationContext::new(
            PhysAddr::new(0x100000),
            VirtAddr::new(0xFFFFFFFF82000000), // +32MB
            VirtAddr::new(0xFFFFFFFF80000000),
            0x200000,
        );

        assert_eq!(ctx.slide, 0x2000000); // 32MB slide
    }

    #[test]
    fn test_builder() {
        let ctx = RelocationContext::builder()
            .phys_base(0x100000)
            .virt_base(0xFFFFFFFF80000000)
            .kernel_size(0x200000)
            .strategy(RelocationStrategy::StaticMinimal)
            .build()
            .unwrap();

        assert_eq!(ctx.strategy, RelocationStrategy::StaticMinimal);
    }

    #[test]
    fn test_bounds_checking() {
        let ctx = RelocationContext::new(
            PhysAddr::new(0x100000),
            VirtAddr::new(0xFFFFFFFF80000000),
            VirtAddr::new(0xFFFFFFFF80000000),
            0x200000,
        );

        assert!(ctx.is_in_bounds(0));
        assert!(ctx.is_in_bounds(0x1FFFFF));
        assert!(!ctx.is_in_bounds(0x200000));
        assert!(!ctx.is_in_bounds(0x1000000));
    }
}
