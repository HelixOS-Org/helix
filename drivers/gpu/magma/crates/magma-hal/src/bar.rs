//! # BAR (Base Address Register) Management
//!
//! GPU BAR mapping and region handling.

use magma_core::{ByteSize, Error, PhysAddr, Result};

// =============================================================================
// BAR TYPES
// =============================================================================

/// BAR type (memory or I/O)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarType {
    /// Memory BAR (32-bit)
    Memory32,
    /// Memory BAR (64-bit)
    Memory64,
    /// I/O BAR
    Io,
    /// Disabled/empty BAR
    Disabled,
}

/// BAR prefetchable flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarPrefetch {
    /// Non-prefetchable (MMIO registers)
    NonPrefetchable,
    /// Prefetchable (VRAM)
    Prefetchable,
}

// =============================================================================
// NVIDIA GPU BARS
// =============================================================================

/// NVIDIA GPU BAR layout
///
/// NVIDIA GPUs typically have the following BAR layout:
/// - BAR0: MMIO registers (16MB-32MB)
/// - BAR1: GPU framebuffer/VRAM aperture (up to 256MB visible)
/// - BAR2/3: RAMIN (instance memory, 16MB-64MB)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NvidiaBar {
    /// BAR0: MMIO registers
    Mmio        = 0,
    /// BAR1: Framebuffer aperture
    Framebuffer = 1,
    /// BAR2/3: RAMIN (instance memory)
    Ramin       = 2,
}

impl NvidiaBar {
    /// Get BAR index
    pub const fn index(self) -> u8 {
        self as u8
    }

    /// Expected minimum size for this BAR
    pub const fn min_size(self) -> ByteSize {
        match self {
            NvidiaBar::Mmio => ByteSize::from_mib(16),
            NvidiaBar::Framebuffer => ByteSize::from_mib(16),
            NvidiaBar::Ramin => ByteSize::from_mib(16),
        }
    }
}

// =============================================================================
// BAR INFO
// =============================================================================

/// Information about a single BAR
#[derive(Debug, Clone)]
pub struct BarInfo {
    /// BAR index (0-5)
    pub index: u8,
    /// BAR type
    pub bar_type: BarType,
    /// Prefetchable flag
    pub prefetch: BarPrefetch,
    /// Physical base address
    pub base_addr: PhysAddr,
    /// BAR size
    pub size: ByteSize,
}

impl BarInfo {
    /// Check if BAR is valid/enabled
    pub fn is_enabled(&self) -> bool {
        self.bar_type != BarType::Disabled && self.size.as_bytes() > 0
    }

    /// Check if BAR is 64-bit
    pub fn is_64bit(&self) -> bool {
        self.bar_type == BarType::Memory64
    }

    /// Check if BAR is prefetchable
    pub fn is_prefetchable(&self) -> bool {
        self.prefetch == BarPrefetch::Prefetchable
    }
}

// =============================================================================
// BAR REGION
// =============================================================================

/// Mapped BAR region for memory access
#[derive(Debug)]
pub struct BarRegion {
    /// Physical base address
    pub phys_addr: PhysAddr,
    /// Virtual address (kernel mapping)
    pub virt_addr: usize,
    /// Region size
    pub size: ByteSize,
    /// BAR type
    pub bar_type: BarType,
}

impl BarRegion {
    /// Get a pointer to the mapped region
    ///
    /// # Safety
    /// The caller must ensure:
    /// - The offset is within bounds
    /// - The access type is appropriate for the BAR region
    pub unsafe fn as_ptr(&self) -> *const u8 {
        self.virt_addr as *const u8
    }

    /// Get a mutable pointer to the mapped region
    ///
    /// # Safety
    /// The caller must ensure:
    /// - The offset is within bounds
    /// - The access type is appropriate for the BAR region
    /// - Proper synchronization for concurrent access
    pub unsafe fn as_mut_ptr(&mut self) -> *mut u8 {
        self.virt_addr as *mut u8
    }

    /// Check if offset is within bounds
    pub fn contains_offset(&self, offset: usize) -> bool {
        offset < self.size.as_bytes() as usize
    }

    /// Read u32 at offset
    ///
    /// # Safety
    /// - Offset must be aligned to 4 bytes
    /// - Offset + 4 must be within bounds
    pub unsafe fn read32(&self, offset: usize) -> u32 {
        debug_assert!(offset % 4 == 0);
        debug_assert!(offset + 4 <= self.size.as_bytes() as usize);

        let ptr = (self.virt_addr + offset) as *const u32;
        // SAFETY: Caller guarantees alignment and bounds
        unsafe { core::ptr::read_volatile(ptr) }
    }

    /// Write u32 at offset
    ///
    /// # Safety
    /// - Offset must be aligned to 4 bytes
    /// - Offset + 4 must be within bounds
    pub unsafe fn write32(&mut self, offset: usize, value: u32) {
        debug_assert!(offset % 4 == 0);
        debug_assert!(offset + 4 <= self.size.as_bytes() as usize);

        let ptr = (self.virt_addr + offset) as *mut u32;
        // SAFETY: Caller guarantees alignment and bounds
        unsafe { core::ptr::write_volatile(ptr, value) }
    }
}

// =============================================================================
// BAR MANAGER TRAIT
// =============================================================================

/// Trait for BAR management operations
pub trait BarManager {
    /// Get info for a BAR
    fn get_bar_info(&self, bar_index: u8) -> Result<BarInfo>;

    /// Map a BAR into kernel address space
    fn map_bar(&mut self, bar_index: u8) -> Result<BarRegion>;

    /// Unmap a BAR
    fn unmap_bar(&mut self, bar_index: u8) -> Result<()>;

    /// Map NVIDIA MMIO BAR
    fn map_mmio(&mut self) -> Result<BarRegion> {
        self.map_bar(NvidiaBar::Mmio.index())
    }

    /// Map NVIDIA framebuffer BAR
    fn map_framebuffer(&mut self) -> Result<BarRegion> {
        self.map_bar(NvidiaBar::Framebuffer.index())
    }

    /// Map NVIDIA RAMIN BAR
    fn map_ramin(&mut self) -> Result<BarRegion> {
        self.map_bar(NvidiaBar::Ramin.index())
    }
}
