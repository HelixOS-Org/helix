//! # GIC Redistributor (GICR) - GICv3 Only
//!
//! The Redistributor is a component introduced in GICv3 that handles per-CPU
//! interrupt state management for SGIs, PPIs, and LPIs. Each CPU has its own
//! Redistributor.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                    GICv3 Redistributor                              │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │   CPU 0                CPU 1                CPU N                   │
//! │  ┌──────────┐         ┌──────────┐         ┌──────────┐            │
//! │  │  GICR    │         │  GICR    │         │  GICR    │            │
//! │  │ Frame 0  │         │ Frame 1  │         │ Frame N  │            │
//! │  ├──────────┤         ├──────────┤         ├──────────┤            │
//! │  │ RD_base  │         │ RD_base  │         │ RD_base  │            │
//! │  │ (64 KB)  │         │ (64 KB)  │         │ (64 KB)  │            │
//! │  ├──────────┤         ├──────────┤         ├──────────┤            │
//! │  │ SGI_base │         │ SGI_base │         │ SGI_base │            │
//! │  │ (64 KB)  │         │ (64 KB)  │         │ (64 KB)  │            │
//! │  └──────────┘         └──────────┘         └──────────┘            │
//! │                                                                     │
//! │  Each Redistributor manages:                                        │
//! │  - SGIs (0-15): Software Generated Interrupts                       │
//! │  - PPIs (16-31): Private Peripheral Interrupts                      │
//! │  - LPI state: Enable/pending tables (if supported)                  │
//! │                                                                     │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Register Frames
//!
//! Each Redistributor consists of two 64KB frames:
//!
//! - **RD_base (0x0000-0xFFFF)**: Control, power management, LPI configuration
//! - **SGI_base (0x10000-0x1FFFF)**: SGI/PPI enable, pending, priority, config
//!
//! ## LPI Support
//!
//! LPIs (Locality-specific Peripheral Interrupts) are a new interrupt type in GICv3:
//! - Range: 8192 and above
//! - Configured via memory-resident tables
//! - Support for very large interrupt counts
//! - Used with ITS (Interrupt Translation Service) for PCIe MSI

use core::ptr::{read_volatile, write_volatile};

use super::{bit_reg_offset, byte_reg_offset, config_reg_offset, Priority, TriggerMode, PPI_BASE};

// ============================================================================
// GICR Frame Sizes and Offsets
// ============================================================================

/// Size of the RD_base frame (64 KB)
pub const GICR_RD_BASE_SIZE: usize = 0x10000;

/// Size of the SGI_base frame (64 KB)
pub const GICR_SGI_BASE_SIZE: usize = 0x10000;

/// Size of a complete Redistributor (RD_base + SGI_base)
pub const GICR_FRAME_SIZE: usize = GICR_RD_BASE_SIZE + GICR_SGI_BASE_SIZE;

/// Offset from RD_base to SGI_base
pub const GICR_SGI_BASE_OFFSET: usize = 0x10000;

// ============================================================================
// GICR RD_base Register Offsets
// ============================================================================

/// GICR Control Register
pub const GICR_CTLR: usize = 0x0000;

/// GICR Implementer Identification Register
pub const GICR_IIDR: usize = 0x0004;

/// GICR Type Register
pub const GICR_TYPER: usize = 0x0008;

/// GICR Status Register
pub const GICR_STATUSR: usize = 0x0010;

/// GICR Wake Request Register
pub const GICR_WAKER: usize = 0x0014;

/// GICR Set LPI Pending Register
pub const GICR_SETLPIR: usize = 0x0040;

/// GICR Clear LPI Pending Register
pub const GICR_CLRLPIR: usize = 0x0048;

/// GICR Properties Base Address Register
pub const GICR_PROPBASER: usize = 0x0070;

/// GICR Pending Base Address Register
pub const GICR_PENDBASER: usize = 0x0078;

/// GICR Invalidate LPI Register
pub const GICR_INVLPIR: usize = 0x00A0;

/// GICR Invalidate All Register
pub const GICR_INVALLR: usize = 0x00B0;

/// GICR Synchronize Register
pub const GICR_SYNCR: usize = 0x00C0;

// ============================================================================
// GICR SGI_base Register Offsets (relative to SGI_base)
// ============================================================================

/// GICR SGI/PPI Group Register 0
pub const GICR_IGROUPR0: usize = 0x0080;

/// GICR SGI/PPI Set-Enable Register 0
pub const GICR_ISENABLER0: usize = 0x0100;

/// GICR SGI/PPI Clear-Enable Register 0
pub const GICR_ICENABLER0: usize = 0x0180;

/// GICR SGI/PPI Set-Pending Register 0
pub const GICR_ISPENDR0: usize = 0x0200;

/// GICR SGI/PPI Clear-Pending Register 0
pub const GICR_ICPENDR0: usize = 0x0280;

/// GICR SGI/PPI Set-Active Register 0
pub const GICR_ISACTIVER0: usize = 0x0300;

/// GICR SGI/PPI Clear-Active Register 0
pub const GICR_ICACTIVER0: usize = 0x0380;

/// GICR SGI/PPI Priority Registers
pub const GICR_IPRIORITYR: usize = 0x0400;

/// GICR SGI/PPI Configuration Register 0 (SGIs, read-only)
pub const GICR_ICFGR0: usize = 0x0C00;

/// GICR SGI/PPI Configuration Register 1 (PPIs)
pub const GICR_ICFGR1: usize = 0x0C04;

/// GICR Group Modifier Register 0
pub const GICR_IGRPMODR0: usize = 0x0D00;

/// GICR Non-secure Access Control Register
pub const GICR_NSACR: usize = 0x0E00;

// ============================================================================
// GICR_CTLR Bits
// ============================================================================

/// Enable LPIs
pub const GICR_CTLR_ENABLE_LPIS: u32 = 1 << 0;

/// Register Write Pending
pub const GICR_CTLR_RWP: u32 = 1 << 3;

/// Upstream Write Pending
pub const GICR_CTLR_UWP: u32 = 1 << 31;

// ============================================================================
// GICR_TYPER Bits
// ============================================================================

/// Physical LPIs supported
pub const GICR_TYPER_PLPIS: u64 = 1 << 0;

/// Virtual LPIs supported
pub const GICR_TYPER_VLPIS: u64 = 1 << 1;

/// Dirty bit supported (GICv3.1)
pub const GICR_TYPER_DIRTY: u64 = 1 << 2;

/// Direct Virtual LPI injection supported
pub const GICR_TYPER_DIRECTLPI: u64 = 1 << 3;

/// Last Redistributor in a series
pub const GICR_TYPER_LAST: u64 = 1 << 4;

/// DPGS (Disable Processor state Group) support
pub const GICR_TYPER_DPGS: u64 = 1 << 5;

/// Processor Number mask
pub const GICR_TYPER_PRCNUM_MASK: u64 = 0xFFFF << 8;

/// Common LPI Affinity Group (64-bit)
pub const GICR_TYPER_COMMONLPIAFF_MASK: u64 = 0x3 << 24;

/// Affinity value mask
pub const GICR_TYPER_AFFINITY_MASK: u64 = 0xFFFF_FFFF << 32;

// ============================================================================
// GICR_WAKER Bits
// ============================================================================

/// Processor Sleep
pub const GICR_WAKER_PROCESSOR_SLEEP: u32 = 1 << 1;

/// Children Asleep
pub const GICR_WAKER_CHILDREN_ASLEEP: u32 = 1 << 2;

// ============================================================================
// Redistributor Structure
// ============================================================================

/// GIC Redistributor for a single CPU
pub struct Redistributor {
    /// Base address of RD_base frame
    rd_base: *mut u8,
    /// Base address of SGI_base frame
    sgi_base: *mut u8,
}

impl Redistributor {
    /// Create a new Redistributor from base address
    ///
    /// # Safety
    ///
    /// Caller must ensure the base address is valid and points to this CPU's
    /// Redistributor.
    #[inline]
    pub const unsafe fn new(rd_base: *mut u8) -> Self {
        Self {
            rd_base,
            sgi_base: rd_base.add(GICR_SGI_BASE_OFFSET),
        }
    }

    /// Get the RD_base address
    #[inline]
    pub const fn rd_base(&self) -> *mut u8 {
        self.rd_base
    }

    /// Get the SGI_base address
    #[inline]
    pub const fn sgi_base(&self) -> *mut u8 {
        self.sgi_base
    }

    // ========================================================================
    // Register Access Helpers
    // ========================================================================

    /// Read a 32-bit register from RD_base
    #[inline]
    unsafe fn read_rd_reg(&self, offset: usize) -> u32 {
        read_volatile((self.rd_base as *const u32).add(offset / 4))
    }

    /// Write a 32-bit register to RD_base
    #[inline]
    unsafe fn write_rd_reg(&self, offset: usize, value: u32) {
        write_volatile((self.rd_base as *mut u32).add(offset / 4), value);
    }

    /// Read a 64-bit register from RD_base
    #[inline]
    unsafe fn read_rd_reg64(&self, offset: usize) -> u64 {
        read_volatile((self.rd_base as *const u64).add(offset / 8))
    }

    /// Write a 64-bit register to RD_base
    #[inline]
    unsafe fn write_rd_reg64(&self, offset: usize, value: u64) {
        write_volatile((self.rd_base as *mut u64).add(offset / 8), value);
    }

    /// Read a 32-bit register from SGI_base
    #[inline]
    unsafe fn read_sgi_reg(&self, offset: usize) -> u32 {
        read_volatile((self.sgi_base as *const u32).add(offset / 4))
    }

    /// Write a 32-bit register to SGI_base
    #[inline]
    unsafe fn write_sgi_reg(&self, offset: usize, value: u32) {
        write_volatile((self.sgi_base as *mut u32).add(offset / 4), value);
    }

    // ========================================================================
    // Control and Status
    // ========================================================================

    /// Read GICR_CTLR
    #[inline]
    pub unsafe fn read_ctlr(&self) -> u32 {
        self.read_rd_reg(GICR_CTLR)
    }

    /// Write GICR_CTLR
    #[inline]
    pub unsafe fn write_ctlr(&self, value: u32) {
        self.write_rd_reg(GICR_CTLR, value);
    }

    /// Read GICR_TYPER
    #[inline]
    pub unsafe fn read_typer(&self) -> u64 {
        self.read_rd_reg64(GICR_TYPER)
    }

    /// Read GICR_WAKER
    #[inline]
    pub unsafe fn read_waker(&self) -> u32 {
        self.read_rd_reg(GICR_WAKER)
    }

    /// Write GICR_WAKER
    #[inline]
    pub unsafe fn write_waker(&self, value: u32) {
        self.write_rd_reg(GICR_WAKER, value);
    }

    /// Wait for register write to complete
    #[inline]
    pub unsafe fn wait_for_rwp(&self) {
        while (self.read_ctlr() & GICR_CTLR_RWP) != 0 {
            core::hint::spin_loop();
        }
    }

    // ========================================================================
    // Wake/Sleep Management
    // ========================================================================

    /// Wake up the Redistributor (clear ProcessorSleep)
    pub unsafe fn wake(&self) {
        let mut waker = self.read_waker();
        waker &= !GICR_WAKER_PROCESSOR_SLEEP;
        self.write_waker(waker);

        // Wait for ChildrenAsleep to clear
        while (self.read_waker() & GICR_WAKER_CHILDREN_ASLEEP) != 0 {
            core::hint::spin_loop();
        }
    }

    /// Put the Redistributor to sleep
    pub unsafe fn sleep(&self) {
        let mut waker = self.read_waker();
        waker |= GICR_WAKER_PROCESSOR_SLEEP;
        self.write_waker(waker);

        // Wait for ChildrenAsleep to set
        while (self.read_waker() & GICR_WAKER_CHILDREN_ASLEEP) == 0 {
            core::hint::spin_loop();
        }
    }

    /// Check if this is the last Redistributor
    pub unsafe fn is_last(&self) -> bool {
        (self.read_typer() & GICR_TYPER_LAST) != 0
    }

    /// Get the processor number
    pub unsafe fn processor_number(&self) -> u16 {
        ((self.read_typer() & GICR_TYPER_PRCNUM_MASK) >> 8) as u16
    }

    /// Get the affinity value
    pub unsafe fn affinity(&self) -> u32 {
        ((self.read_typer() & GICR_TYPER_AFFINITY_MASK) >> 32) as u32
    }

    /// Check if LPIs are supported
    pub unsafe fn supports_lpis(&self) -> bool {
        (self.read_typer() & GICR_TYPER_PLPIS) != 0
    }

    /// Check if virtual LPIs are supported
    pub unsafe fn supports_vlpis(&self) -> bool {
        (self.read_typer() & GICR_TYPER_VLPIS) != 0
    }

    // ========================================================================
    // SGI/PPI Enable/Disable
    // ========================================================================

    /// Enable an SGI or PPI (intid 0-31)
    #[inline]
    pub unsafe fn enable_interrupt(&self, intid: u32) {
        debug_assert!(intid < 32);
        self.write_sgi_reg(GICR_ISENABLER0, 1 << intid);
    }

    /// Disable an SGI or PPI (intid 0-31)
    #[inline]
    pub unsafe fn disable_interrupt(&self, intid: u32) {
        debug_assert!(intid < 32);
        self.write_sgi_reg(GICR_ICENABLER0, 1 << intid);
    }

    /// Check if an SGI or PPI is enabled
    #[inline]
    pub unsafe fn is_enabled(&self, intid: u32) -> bool {
        debug_assert!(intid < 32);
        (self.read_sgi_reg(GICR_ISENABLER0) & (1 << intid)) != 0
    }

    // ========================================================================
    // Pending State
    // ========================================================================

    /// Set an SGI or PPI pending
    #[inline]
    pub unsafe fn set_pending(&self, intid: u32) {
        debug_assert!(intid < 32);
        self.write_sgi_reg(GICR_ISPENDR0, 1 << intid);
    }

    /// Clear an SGI or PPI pending state
    #[inline]
    pub unsafe fn clear_pending(&self, intid: u32) {
        debug_assert!(intid < 32);
        self.write_sgi_reg(GICR_ICPENDR0, 1 << intid);
    }

    /// Check if an SGI or PPI is pending
    #[inline]
    pub unsafe fn is_pending(&self, intid: u32) -> bool {
        debug_assert!(intid < 32);
        (self.read_sgi_reg(GICR_ISPENDR0) & (1 << intid)) != 0
    }

    // ========================================================================
    // Active State
    // ========================================================================

    /// Set an SGI or PPI active
    #[inline]
    pub unsafe fn set_active(&self, intid: u32) {
        debug_assert!(intid < 32);
        self.write_sgi_reg(GICR_ISACTIVER0, 1 << intid);
    }

    /// Clear an SGI or PPI active state
    #[inline]
    pub unsafe fn clear_active(&self, intid: u32) {
        debug_assert!(intid < 32);
        self.write_sgi_reg(GICR_ICACTIVER0, 1 << intid);
    }

    /// Check if an SGI or PPI is active
    #[inline]
    pub unsafe fn is_active(&self, intid: u32) -> bool {
        debug_assert!(intid < 32);
        (self.read_sgi_reg(GICR_ISACTIVER0) & (1 << intid)) != 0
    }

    // ========================================================================
    // Priority
    // ========================================================================

    /// Set the priority of an SGI or PPI
    pub unsafe fn set_priority(&self, intid: u32, priority: Priority) {
        debug_assert!(intid < 32);
        let addr = self.sgi_base.add(GICR_IPRIORITYR + intid as usize);
        write_volatile(addr, priority.value());
    }

    /// Get the priority of an SGI or PPI
    pub unsafe fn get_priority(&self, intid: u32) -> Priority {
        debug_assert!(intid < 32);
        let addr = self.sgi_base.add(GICR_IPRIORITYR + intid as usize) as *const u8;
        Priority(read_volatile(addr))
    }

    /// Set all SGI/PPI priorities to a default value
    pub unsafe fn set_all_priorities(&self, priority: Priority) {
        let value = (priority.value() as u32) * 0x01010101;
        for i in 0..8 {
            self.write_sgi_reg(GICR_IPRIORITYR + i * 4, value);
        }
    }

    // ========================================================================
    // Trigger Configuration
    // ========================================================================

    /// Set the trigger mode of a PPI
    pub unsafe fn set_ppi_trigger_mode(&self, ppi: u32, mode: TriggerMode) {
        debug_assert!(ppi >= PPI_BASE && ppi < 32);
        let bit_offset = (ppi - PPI_BASE) * 2;
        let mut config = self.read_sgi_reg(GICR_ICFGR1);

        config &= !(0x3 << bit_offset);
        if matches!(mode, TriggerMode::Edge) {
            config |= 0x2 << bit_offset;
        }

        self.write_sgi_reg(GICR_ICFGR1, config);
    }

    // ========================================================================
    // Group Configuration
    // ========================================================================

    /// Set an SGI or PPI to Group 0
    #[inline]
    pub unsafe fn set_group0(&self, intid: u32) {
        debug_assert!(intid < 32);
        let mut group = self.read_sgi_reg(GICR_IGROUPR0);
        group &= !(1 << intid);
        self.write_sgi_reg(GICR_IGROUPR0, group);
    }

    /// Set an SGI or PPI to Group 1
    #[inline]
    pub unsafe fn set_group1(&self, intid: u32) {
        debug_assert!(intid < 32);
        let mut group = self.read_sgi_reg(GICR_IGROUPR0);
        group |= 1 << intid;
        self.write_sgi_reg(GICR_IGROUPR0, group);
    }

    /// Set all SGIs and PPIs to Group 1
    pub unsafe fn set_all_group1(&self) {
        self.write_sgi_reg(GICR_IGROUPR0, 0xFFFF_FFFF);
    }

    // ========================================================================
    // LPI Configuration
    // ========================================================================

    /// Enable LPIs
    pub unsafe fn enable_lpis(&self) {
        let mut ctlr = self.read_ctlr();
        ctlr |= GICR_CTLR_ENABLE_LPIS;
        self.write_ctlr(ctlr);
    }

    /// Disable LPIs
    pub unsafe fn disable_lpis(&self) {
        let mut ctlr = self.read_ctlr();
        ctlr &= !GICR_CTLR_ENABLE_LPIS;
        self.write_ctlr(ctlr);
        self.wait_for_rwp();
    }

    /// Configure LPI property table base
    pub unsafe fn set_propbase(&self, base: u64, id_bits: u8) {
        // PROPBASER format:
        // [4:0] = ID bits (number of LPIs = 2^(IDbits+1))
        // [9:7] = InnerCache (Write-Back, Write-Allocate = 7)
        // [11:10] = Shareability (Inner Shareable = 1)
        // [51:12] = Physical address
        // [58:56] = OuterCache
        let value = (base & !0xFFF)
            | ((id_bits.saturating_sub(1) & 0x1F) as u64)
            | (7 << 7)   // InnerCache: WB, WA
            | (1 << 10); // Inner Shareable

        self.write_rd_reg64(GICR_PROPBASER, value);
    }

    /// Configure LPI pending table base
    pub unsafe fn set_pendbase(&self, base: u64) {
        // PENDBASER format:
        // [9:7] = InnerCache (Write-Back, Write-Allocate = 7)
        // [11:10] = Shareability (Inner Shareable = 1)
        // [51:16] = Physical address (64KB aligned)
        // [62] = PTZ (Pending Table Zero)
        let value = (base & !0xFFFF)
            | (7 << 7)    // InnerCache: WB, WA
            | (1 << 10)   // Inner Shareable
            | (1 << 62); // PTZ: Zero pending bits on enable

        self.write_rd_reg64(GICR_PENDBASER, value);
    }

    /// Invalidate an LPI
    pub unsafe fn invalidate_lpi(&self, intid: u32) {
        self.write_rd_reg64(GICR_INVLPIR, intid as u64);
    }

    /// Invalidate all LPIs
    pub unsafe fn invalidate_all_lpis(&self) {
        self.write_rd_reg64(GICR_INVALLR, 0);
    }

    /// Wait for sync operation to complete
    pub unsafe fn sync(&self) {
        // Write to SYNCR triggers sync
        self.write_rd_reg(GICR_SYNCR, 0);
        // Wait for busy bit to clear (bit 0)
        while (self.read_rd_reg(GICR_SYNCR) & 1) != 0 {
            core::hint::spin_loop();
        }
    }

    // ========================================================================
    // Initialization
    // ========================================================================

    /// Initialize the Redistributor for this CPU
    pub unsafe fn init(&self) {
        // Wake up the Redistributor
        self.wake();

        // Set all SGIs/PPIs to Group 1 (non-secure)
        self.set_all_group1();

        // Set default priority for all SGIs/PPIs
        self.set_all_priorities(Priority::DEFAULT);

        // Wait for completion
        self.wait_for_rwp();
    }
}

// ============================================================================
// Redistributor Discovery
// ============================================================================

/// Iterator over all Redistributors in a GICv3 system
pub struct RedistributorIter {
    current: *mut u8,
    done: bool,
}

impl RedistributorIter {
    /// Create a new iterator starting at the first Redistributor
    ///
    /// # Safety
    ///
    /// Caller must ensure the base address points to valid GICR memory.
    pub const unsafe fn new(gicr_base: *mut u8) -> Self {
        Self {
            current: gicr_base,
            done: false,
        }
    }
}

impl Iterator for RedistributorIter {
    type Item = Redistributor;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let redist = unsafe { Redistributor::new(self.current) };

        // Check if this is the last Redistributor
        if unsafe { redist.is_last() } {
            self.done = true;
        } else {
            // Move to next Redistributor frame
            self.current = unsafe { self.current.add(GICR_FRAME_SIZE) };
        }

        Some(redist)
    }
}

/// Find the Redistributor for the current CPU
///
/// # Safety
///
/// Caller must ensure the base address is valid.
pub unsafe fn find_redistributor_for_current_cpu(gicr_base: *mut u8) -> Option<Redistributor> {
    // Read current CPU's affinity from MPIDR_EL1
    let mpidr: u64;
    core::arch::asm!("mrs {}, mpidr_el1", out(reg) mpidr, options(nomem, nostack));
    let aff = ((mpidr & 0xFF) as u32)
        | (((mpidr >> 8) & 0xFF) as u32) << 8
        | (((mpidr >> 16) & 0xFF) as u32) << 16
        | (((mpidr >> 32) & 0xFF) as u32) << 24;

    for redist in RedistributorIter::new(gicr_base) {
        if redist.affinity() == aff {
            return Some(redist);
        }
    }

    None
}

// ============================================================================
// Redistributor Information
// ============================================================================

/// Information about a Redistributor
#[derive(Debug, Clone)]
pub struct RedistributorInfo {
    /// Processor number
    pub processor_number: u16,
    /// Affinity value
    pub affinity: u32,
    /// Is this the last Redistributor
    pub is_last: bool,
    /// LPIs supported
    pub supports_lpis: bool,
    /// Virtual LPIs supported
    pub supports_vlpis: bool,
}

impl RedistributorInfo {
    /// Read information from a Redistributor
    ///
    /// # Safety
    ///
    /// Caller must ensure the Redistributor is valid.
    pub unsafe fn from_redistributor(redist: &Redistributor) -> Self {
        Self {
            processor_number: redist.processor_number(),
            affinity: redist.affinity(),
            is_last: redist.is_last(),
            supports_lpis: redist.supports_lpis(),
            supports_vlpis: redist.supports_vlpis(),
        }
    }
}

// ============================================================================
// Platform Constants
// ============================================================================

/// QEMU virt platform GICR base address
pub const QEMU_VIRT_GICR_BASE: usize = 0x080A_0000;

/// ARM FVP GICR base address
pub const ARM_FVP_GICR_BASE: usize = 0x2F10_0000;
