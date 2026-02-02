//! # Memory-Mapped I/O Operations
//!
//! Safe abstractions for NVIDIA GPU register access via MMIO.

use magma_core::{Error, Result, ByteSize};

use crate::bar::BarRegion;

// =============================================================================
// MMIO REGISTER BLOCKS
// =============================================================================

/// NVIDIA GPU register blocks in BAR0
///
/// The GPU MMIO space is organized into functional blocks.
/// Offsets are in bytes from BAR0 base.
pub mod registers {
    //! GPU register block offsets and definitions

    /// Boot/control registers
    pub const BOOT: u32 = 0x0000_0000;
    /// Master control
    pub const PMC: u32 = 0x0000_0000;
    /// Bus control
    pub const PBUS: u32 = 0x0000_1000;
    /// FIFO control
    pub const PFIFO: u32 = 0x0000_2000;
    /// Timer
    pub const PTIMER: u32 = 0x0000_9000;
    /// Falcon (GSP) related
    pub const FALCON: u32 = 0x0010_0000;
    /// Memory controller
    pub const PFB: u32 = 0x0010_0000;
    /// Graphics engine (3D class)
    pub const PGRAPH: u32 = 0x0040_0000;
    /// Display engine
    pub const PDISP: u32 = 0x0061_0000;
    /// Copy engines
    pub const PCOPY: u32 = 0x0010_4000;
    /// NVDEC (video decode)
    pub const PVDEC: u32 = 0x0008_4000;
    /// NVENC (video encode)
    pub const PVENC: u32 = 0x000A_0000;

    /// PMC register: Boot revision
    pub const PMC_BOOT_0: u32 = PMC + 0x000;
    /// PMC register: Interrupt status
    pub const PMC_INTR_0: u32 = PMC + 0x100;
    /// PMC register: Interrupt enable
    pub const PMC_INTR_EN_0: u32 = PMC + 0x140;
    /// PMC register: Enable
    pub const PMC_ENABLE: u32 = PMC + 0x200;

    /// PTIMER: Time low
    pub const PTIMER_TIME_0: u32 = PTIMER + 0x400;
    /// PTIMER: Time high
    pub const PTIMER_TIME_1: u32 = PTIMER + 0x404;
}

// =============================================================================
// MMIO REGION
// =============================================================================

/// MMIO region with safe accessors
#[derive(Debug)]
pub struct MmioRegion {
    /// Underlying BAR region
    bar: BarRegion,
}

impl MmioRegion {
    /// Create from BAR region
    pub fn from_bar(bar: BarRegion) -> Self {
        Self { bar }
    }

    /// Get region size
    pub fn size(&self) -> ByteSize {
        self.bar.size
    }

    /// Read 32-bit register
    ///
    /// # Safety
    /// - Offset must be aligned to 4 bytes
    /// - Offset must be within bounds
    pub unsafe fn read32(&self, offset: u32) -> u32 {
        // SAFETY: Caller guarantees offset validity
        unsafe { self.bar.read32(offset as usize) }
    }

    /// Write 32-bit register
    ///
    /// # Safety
    /// - Offset must be aligned to 4 bytes
    /// - Offset must be within bounds
    pub unsafe fn write32(&mut self, offset: u32, value: u32) {
        // SAFETY: Caller guarantees offset validity
        unsafe { self.bar.write32(offset as usize, value) }
    }

    /// Read with mask
    ///
    /// # Safety
    /// Same as read32
    pub unsafe fn read32_masked(&self, offset: u32, mask: u32) -> u32 {
        // SAFETY: Forwarded from caller
        (unsafe { self.read32(offset) }) & mask
    }

    /// Write with mask (read-modify-write)
    ///
    /// # Safety
    /// Same as read32/write32
    pub unsafe fn write32_masked(&mut self, offset: u32, value: u32, mask: u32) {
        // SAFETY: Forwarded from caller
        unsafe {
            let current = self.read32(offset);
            let new_value = (current & !mask) | (value & mask);
            self.write32(offset, new_value);
        }
    }

    /// Poll register until condition is met or timeout
    ///
    /// # Safety
    /// Same as read32
    pub unsafe fn poll32(
        &self,
        offset: u32,
        mask: u32,
        expected: u32,
        timeout_us: u64,
    ) -> Result<()> {
        let mut elapsed = 0u64;

        loop {
            // SAFETY: Forwarded from caller
            let value = unsafe { self.read32(offset) };
            if (value & mask) == expected {
                return Ok(());
            }

            if elapsed >= timeout_us {
                return Err(Error::Timeout);
            }

            // Simple busy-wait (in real driver would use proper timing)
            for _ in 0..100 {
                core::hint::spin_loop();
            }
            elapsed += 1;
        }
    }
}

// =============================================================================
// MMIO SLICE
// =============================================================================

/// A bounded slice of MMIO space for safe subregion access
#[derive(Debug)]
pub struct MmioSlice<'a> {
    region: &'a MmioRegion,
    base_offset: u32,
    size: u32,
}

impl<'a> MmioSlice<'a> {
    /// Create a new MMIO slice
    pub fn new(region: &'a MmioRegion, base_offset: u32, size: u32) -> Result<Self> {
        if base_offset as u64 + size as u64 > region.size().as_bytes() {
            return Err(Error::OutOfBounds);
        }

        Ok(Self {
            region,
            base_offset,
            size,
        })
    }

    /// Read 32-bit register relative to slice base
    ///
    /// # Safety
    /// - Offset must be aligned to 4 bytes
    /// - Offset must be within slice bounds
    pub unsafe fn read32(&self, offset: u32) -> u32 {
        debug_assert!(offset + 4 <= self.size);
        // SAFETY: Bounds checked above, caller guarantees alignment
        unsafe { self.region.read32(self.base_offset + offset) }
    }

    /// Write 32-bit register relative to slice base
    ///
    /// # Safety
    /// - Offset must be aligned to 4 bytes
    /// - Offset must be within slice bounds
    pub unsafe fn write32(&self, offset: u32, value: u32) {
        debug_assert!(offset + 4 <= self.size);
        // SAFETY: Bounds checked above, caller guarantees alignment
        // NOTE: This is safe because MMIO writes are inherently volatile
        let ptr = (self.region.bar.virt_addr + (self.base_offset + offset) as usize) as *mut u32;
        unsafe { core::ptr::write_volatile(ptr, value) }
    }
}

// =============================================================================
// MMIO UTILS
// =============================================================================

/// Helper to extract fields from register values
pub const fn extract_field(value: u32, low_bit: u8, high_bit: u8) -> u32 {
    let mask = ((1u32 << (high_bit - low_bit + 1)) - 1) << low_bit;
    (value & mask) >> low_bit
}

/// Helper to insert field into register value
pub const fn insert_field(value: u32, field: u32, low_bit: u8, high_bit: u8) -> u32 {
    let mask = ((1u32 << (high_bit - low_bit + 1)) - 1) << low_bit;
    (value & !mask) | ((field << low_bit) & mask)
}

// =============================================================================
// MEMORY FENCE OPERATIONS
// =============================================================================

/// Memory barrier types for MMIO
pub mod fence {
    //! Memory barrier operations

    /// Compiler fence (prevents reordering)
    #[inline(always)]
    pub fn compiler() {
        core::sync::atomic::compiler_fence(core::sync::atomic::Ordering::SeqCst);
    }

    /// Memory barrier before MMIO write
    #[inline(always)]
    pub fn mmio_write_barrier() {
        #[cfg(target_arch = "x86_64")]
        {
            // x86_64 has strong memory model, compiler fence is sufficient
            compiler();
        }

        #[cfg(target_arch = "aarch64")]
        {
            // ARM needs explicit barrier
            // SAFETY: This is a memory barrier instruction
            unsafe {
                core::arch::asm!("dmb st", options(nostack, preserves_flags));
            }
        }
    }

    /// Memory barrier after MMIO read
    #[inline(always)]
    pub fn mmio_read_barrier() {
        #[cfg(target_arch = "x86_64")]
        {
            compiler();
        }

        #[cfg(target_arch = "aarch64")]
        {
            // SAFETY: This is a memory barrier instruction
            unsafe {
                core::arch::asm!("dmb ld", options(nostack, preserves_flags));
            }
        }
    }
}
