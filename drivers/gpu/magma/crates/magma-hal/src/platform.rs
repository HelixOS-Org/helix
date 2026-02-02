//! # Platform Abstraction
//!
//! Platform-specific implementations for different architectures.

use magma_core::Result;

use crate::bar::BarManager;
use crate::iommu::Iommu;
use crate::irq::IrqManager;
use crate::pci::{PciDevice, PciEnumerator};

// =============================================================================
// PLATFORM TRAIT
// =============================================================================

/// Platform abstraction trait
///
/// Each target platform (Helix, Linux, FreeBSD) implements this trait
/// to provide access to platform-specific hardware resources.
pub trait Platform {
    /// PCI device type
    type PciDevice: PciDevice;
    /// PCI enumerator type
    type PciEnumerator: PciEnumerator<Device = Self::PciDevice>;
    /// BAR manager type
    type BarManager: BarManager;
    /// IRQ manager type
    type IrqManager: IrqManager;
    /// IOMMU type
    type Iommu: Iommu;

    /// Get platform name
    fn name(&self) -> &'static str;

    /// Initialize platform
    fn init(&mut self) -> Result<()>;

    /// Get PCI enumerator
    fn pci_enumerator(&self) -> &Self::PciEnumerator;

    /// Get BAR manager for a device
    fn bar_manager(&self, device: &Self::PciDevice) -> Result<Self::BarManager>;

    /// Get IRQ manager for a device
    fn irq_manager(&self, device: &Self::PciDevice) -> Result<Self::IrqManager>;

    /// Get IOMMU interface
    fn iommu(&self) -> Option<&Self::Iommu>;

    /// Allocate DMA-capable memory
    fn alloc_dma_memory(&mut self, size: usize) -> Result<*mut u8>;

    /// Free DMA-capable memory
    fn free_dma_memory(&mut self, ptr: *mut u8, size: usize) -> Result<()>;

    /// Get physical address of virtual address
    fn virt_to_phys(&self, virt: usize) -> Result<usize>;

    /// Map physical address to virtual
    fn phys_to_virt(&self, phys: usize, size: usize) -> Result<usize>;

    /// Unmap virtual address
    fn unmap_virt(&mut self, virt: usize, size: usize) -> Result<()>;

    /// Sleep for microseconds (busy-wait in no_std)
    fn sleep_us(&self, us: u64);

    /// Get current time in nanoseconds
    fn time_ns(&self) -> u64;
}

// =============================================================================
// PLATFORM CAPABILITIES
// =============================================================================

/// Platform capabilities
#[derive(Debug, Clone)]
pub struct PlatformCaps {
    /// Maximum DMA address width
    pub dma_bits: u8,
    /// Supports MSI-X
    pub msix: bool,
    /// Supports IOMMU
    pub iommu: bool,
    /// Supports coherent DMA
    pub coherent_dma: bool,
    /// Supports 64-bit BAR mapping
    pub bar64: bool,
    /// Maximum number of interrupt vectors
    pub max_vectors: u16,
}

impl Default for PlatformCaps {
    fn default() -> Self {
        Self {
            dma_bits: 48,
            msix: true,
            iommu: false,
            coherent_dma: true,
            bar64: true,
            max_vectors: 32,
        }
    }
}

// =============================================================================
// HELIX PLATFORM STUB
// =============================================================================

/// Helix OS platform implementation (stub for compilation)
#[cfg(feature = "helix")]
pub mod helix {
    //! Helix OS platform

    use super::*;

    /// Helix platform
    pub struct HelixPlatform {
        caps: PlatformCaps,
    }

    impl HelixPlatform {
        /// Create new Helix platform
        pub fn new() -> Self {
            Self {
                caps: PlatformCaps::default(),
            }
        }

        /// Get platform capabilities
        pub fn caps(&self) -> &PlatformCaps {
            &self.caps
        }
    }
}

// =============================================================================
// ARCHITECTURE HELPERS
// =============================================================================

/// Architecture-specific operations
pub mod arch {
    //! Architecture-specific helpers

    /// Memory barrier
    #[inline(always)]
    pub fn memory_barrier() {
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    }

    /// I/O memory barrier
    #[inline(always)]
    pub fn io_barrier() {
        #[cfg(target_arch = "x86_64")]
        {
            // x86_64: mfence for full barrier
            // SAFETY: mfence is a safe instruction
            unsafe {
                core::arch::asm!("mfence", options(nostack, preserves_flags));
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            // ARM64: dsb sy for full barrier
            // SAFETY: dsb is a safe instruction
            unsafe {
                core::arch::asm!("dsb sy", options(nostack, preserves_flags));
            }
        }

        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        }
    }

    /// Pause hint for spin loops
    #[inline(always)]
    pub fn spin_hint() {
        core::hint::spin_loop();
    }
}
