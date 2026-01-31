//! # GICv2 Implementation
//!
//! This module provides the GICv2-specific implementation for systems that
//! use the older memory-mapped GIC interface.
//!
//! ## GICv2 Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────┐
//! │                         GICv2 Architecture                           │
//! ├──────────────────────────────────────────────────────────────────────┤
//! │                                                                      │
//! │   ┌─────────────────────────────────────────────────────────────┐   │
//! │   │                     GICD (Distributor)                       │   │
//! │   │                     Base: Platform-specific                  │   │
//! │   │                                                              │   │
//! │   │  - GICD_CTLR: Control                                        │   │
//! │   │  - GICD_TYPER: Type info                                     │   │
//! │   │  - GICD_ISENABLERn: Enable interrupts                        │   │
//! │   │  - GICD_IPRIORITYRn: Priority                                │   │
//! │   │  - GICD_ITARGETSRn: CPU targets (1-N routing)                │   │
//! │   │  - GICD_ICFGRn: Trigger config                               │   │
//! │   │  - GICD_SGIR: SGI generation                                 │   │
//! │   └─────────────────────────────────────────────────────────────┘   │
//! │                              │                                       │
//! │                              ▼                                       │
//! │   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐     │
//! │   │  GICC (CPU 0)   │  │  GICC (CPU 1)   │  │  GICC (CPU N)   │     │
//! │   │  Base + offset  │  │  (banked)       │  │  (banked)       │     │
//! │   │                 │  │                 │  │                 │     │
//! │   │  GICC_CTLR     │  │                 │  │                 │     │
//! │   │  GICC_PMR      │  │                 │  │                 │     │
//! │   │  GICC_IAR      │  │                 │  │                 │     │
//! │   │  GICC_EOIR     │  │                 │  │                 │     │
//! │   └─────────────────┘  └─────────────────┘  └─────────────────┘     │
//! │                                                                      │
//! └──────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Platforms
//!
//! - QEMU virt (default before -machine virt,gic-version=3)
//! - Raspberry Pi 3/4 (BCM2711 uses GIC-400)
//! - ARM Juno reference platform
//! - Many embedded SoCs

use super::{
    CpuTargetList, Priority, TriggerMode,
    distributor::{
        Distributor, GICD_CTLR, GICD_CTLR_ENABLE_GRP0, GICD_CTLR_ENABLE_GRP1,
        GICD_ITARGETSR, GICD_SGIR,
    },
    SPI_BASE,
};
use core::ptr::{read_volatile, write_volatile};

// ============================================================================
// GICC Register Offsets
// ============================================================================

/// GICC Control Register
pub const GICC_CTLR: usize = 0x0000;

/// GICC Priority Mask Register
pub const GICC_PMR: usize = 0x0004;

/// GICC Binary Point Register
pub const GICC_BPR: usize = 0x0008;

/// GICC Interrupt Acknowledge Register
pub const GICC_IAR: usize = 0x000C;

/// GICC End of Interrupt Register
pub const GICC_EOIR: usize = 0x0010;

/// GICC Running Priority Register
pub const GICC_RPR: usize = 0x0014;

/// GICC Highest Priority Pending Interrupt Register
pub const GICC_HPPIR: usize = 0x0018;

/// GICC Aliased Binary Point Register
pub const GICC_ABPR: usize = 0x001C;

/// GICC Aliased Interrupt Acknowledge Register
pub const GICC_AIAR: usize = 0x0020;

/// GICC Aliased End of Interrupt Register
pub const GICC_AEOIR: usize = 0x0024;

/// GICC Aliased Highest Priority Pending Interrupt Register
pub const GICC_AHPPIR: usize = 0x0028;

/// GICC Active Priority Register 0
pub const GICC_APR0: usize = 0x00D0;

/// GICC Non-secure Active Priority Register 0
pub const GICC_NSAPR0: usize = 0x00E0;

/// GICC Interface Identification Register
pub const GICC_IIDR: usize = 0x00FC;

/// GICC Deactivate Interrupt Register
pub const GICC_DIR: usize = 0x1000;

// ============================================================================
// GICC_CTLR Bits
// ============================================================================

/// Enable signaling of Group 0 interrupts
pub const GICC_CTLR_ENABLE_GRP0: u32 = 1 << 0;

/// Enable signaling of Group 1 interrupts
pub const GICC_CTLR_ENABLE_GRP1: u32 = 1 << 1;

/// Acknowledge Control: Read IAR gets highest priority pending Group 1
pub const GICC_CTLR_ACKCTL: u32 = 1 << 2;

/// FIQ Enable: Group 0 interrupts use FIQ signal
pub const GICC_CTLR_FIQ_EN: u32 = 1 << 3;

/// Common Binary Point Register
pub const GICC_CTLR_CBPR: u32 = 1 << 4;

/// FIQ Bypass Disable (Group 0)
pub const GICC_CTLR_FIQ_BYP_DIS_GRP0: u32 = 1 << 5;

/// IRQ Bypass Disable (Group 0)
pub const GICC_CTLR_IRQ_BYP_DIS_GRP0: u32 = 1 << 6;

/// FIQ Bypass Disable (Group 1)
pub const GICC_CTLR_FIQ_BYP_DIS_GRP1: u32 = 1 << 7;

/// IRQ Bypass Disable (Group 1)
pub const GICC_CTLR_IRQ_BYP_DIS_GRP1: u32 = 1 << 8;

/// EOI mode (Secure)
pub const GICC_CTLR_EOIMODES: u32 = 1 << 9;

/// EOI mode (Non-secure)
pub const GICC_CTLR_EOIMODENS: u32 = 1 << 10;

// ============================================================================
// GICD_SGIR Bits
// ============================================================================

/// Target List Filter: Send to CPUs specified in TargetList
pub const SGIR_TARGET_LIST: u32 = 0 << 24;

/// Target List Filter: Send to all CPUs except self
pub const SGIR_TARGET_ALL_EXCEPT_SELF: u32 = 1 << 24;

/// Target List Filter: Send only to self
pub const SGIR_TARGET_SELF: u32 = 2 << 24;

/// NSATT: Use Non-secure security state
pub const SGIR_NSATT: u32 = 1 << 15;

// ============================================================================
// GICv2 Distributor
// ============================================================================

/// GICv2 Distributor implementation
pub struct Gicv2Distributor {
    inner: Distributor,
}

impl Gicv2Distributor {
    /// Create a new GICv2 Distributor
    ///
    /// # Safety
    ///
    /// Caller must ensure the base address points to valid GICD registers.
    #[inline]
    pub const unsafe fn new(base: *mut u8) -> Self {
        Self {
            inner: Distributor::new(base),
        }
    }

    /// Get the base address
    #[inline]
    pub const fn base(&self) -> *mut u8 {
        self.inner.base()
    }

    /// Initialize the GICv2 Distributor
    pub unsafe fn init(&self) {
        // Disable the distributor
        self.inner.write_ctlr(0);

        // Disable all SPIs
        self.inner.disable_all_spis();

        // Set all SPIs to Group 1 (non-secure)
        self.inner.set_all_spis_group1();

        // Set all SPI priorities to default
        self.inner.set_all_spi_priorities(Priority::DEFAULT);

        // Configure all SPIs to target CPU 0 by default
        self.set_all_spi_targets(CpuTargetList::CPU0);

        // Wait for any pending writes
        self.inner.wait_for_rwp();

        // Enable the distributor
        self.inner.write_ctlr(GICD_CTLR_ENABLE_GRP0 | GICD_CTLR_ENABLE_GRP1);
    }

    /// Set the target CPUs for an SPI
    pub unsafe fn set_target(&self, intid: u32, targets: CpuTargetList) {
        if intid < SPI_BASE {
            return; // SGIs and PPIs have fixed targets
        }

        let offset = GICD_ITARGETSR + intid as usize;
        let addr = self.base().add(offset);
        write_volatile(addr, targets.mask());
    }

    /// Get the target CPUs for an SPI
    pub unsafe fn get_target(&self, intid: u32) -> CpuTargetList {
        if intid < SPI_BASE {
            return CpuTargetList::NONE;
        }

        let offset = GICD_ITARGETSR + intid as usize;
        let addr = self.base().add(offset) as *const u8;
        CpuTargetList(read_volatile(addr))
    }

    /// Set all SPI targets to the same CPU list
    pub unsafe fn set_all_spi_targets(&self, targets: CpuTargetList) {
        let num_irqs = self.inner.num_interrupts();
        let value = (targets.mask() as u32) * 0x01010101;

        for i in (SPI_BASE..num_irqs).step_by(4) {
            let reg_index = (i / 4) as usize;
            write_volatile(
                (self.base() as *mut u32).add(GICD_ITARGETSR / 4 + reg_index),
                value,
            );
        }
    }

    /// Send a Software Generated Interrupt (SGI)
    pub unsafe fn send_sgi(&self, sgi_id: u8, targets: CpuTargetList) {
        let value = ((targets.mask() as u32) << 16)
            | SGIR_TARGET_LIST
            | ((sgi_id as u32) & 0xF);

        write_volatile(
            (self.base() as *mut u32).add(GICD_SGIR / 4),
            value,
        );
    }

    /// Send an SGI to all CPUs except self
    pub unsafe fn send_sgi_all_except_self(&self, sgi_id: u8) {
        let value = SGIR_TARGET_ALL_EXCEPT_SELF | ((sgi_id as u32) & 0xF);
        write_volatile(
            (self.base() as *mut u32).add(GICD_SGIR / 4),
            value,
        );
    }

    /// Send an SGI to self
    pub unsafe fn send_sgi_self(&self, sgi_id: u8) {
        let value = SGIR_TARGET_SELF | ((sgi_id as u32) & 0xF);
        write_volatile(
            (self.base() as *mut u32).add(GICD_SGIR / 4),
            value,
        );
    }

    /// Get the inner Distributor
    #[inline]
    pub const fn inner(&self) -> &Distributor {
        &self.inner
    }
}

// ============================================================================
// GICv2 CPU Interface
// ============================================================================

/// GICv2 CPU Interface (memory-mapped)
pub struct Gicv2CpuInterface {
    base: *mut u8,
}

impl Gicv2CpuInterface {
    /// Create a new GICv2 CPU Interface
    ///
    /// # Safety
    ///
    /// Caller must ensure the base address points to valid GICC registers.
    #[inline]
    pub const unsafe fn new(base: *mut u8) -> Self {
        Self { base }
    }

    /// Get the base address
    #[inline]
    pub const fn base(&self) -> *mut u8 {
        self.base
    }

    /// Read a 32-bit register
    #[inline]
    unsafe fn read_reg(&self, offset: usize) -> u32 {
        read_volatile((self.base as *const u32).add(offset / 4))
    }

    /// Write a 32-bit register
    #[inline]
    unsafe fn write_reg(&self, offset: usize, value: u32) {
        write_volatile((self.base as *mut u32).add(offset / 4), value);
    }

    /// Initialize the CPU interface
    pub unsafe fn init(&self) {
        // Disable the CPU interface
        self.write_reg(GICC_CTLR, 0);

        // Set priority mask to allow all interrupts
        self.write_reg(GICC_PMR, 0xFF);

        // Set binary point to 0 (all bits for group priority)
        self.write_reg(GICC_BPR, 0);

        // Enable the CPU interface with FIQ for Group 0, IRQ for Group 1
        let ctlr = GICC_CTLR_ENABLE_GRP0
            | GICC_CTLR_ENABLE_GRP1
            | GICC_CTLR_FIQ_EN
            | GICC_CTLR_FIQ_BYP_DIS_GRP0
            | GICC_CTLR_IRQ_BYP_DIS_GRP0
            | GICC_CTLR_FIQ_BYP_DIS_GRP1
            | GICC_CTLR_IRQ_BYP_DIS_GRP1;

        self.write_reg(GICC_CTLR, ctlr);
    }

    /// Read GICC_CTLR
    #[inline]
    pub unsafe fn read_ctlr(&self) -> u32 {
        self.read_reg(GICC_CTLR)
    }

    /// Write GICC_CTLR
    #[inline]
    pub unsafe fn write_ctlr(&self, value: u32) {
        self.write_reg(GICC_CTLR, value);
    }

    /// Set the priority mask
    #[inline]
    pub unsafe fn set_priority_mask(&self, priority: Priority) {
        self.write_reg(GICC_PMR, priority.value() as u32);
    }

    /// Get the priority mask
    #[inline]
    pub unsafe fn get_priority_mask(&self) -> Priority {
        Priority(self.read_reg(GICC_PMR) as u8)
    }

    /// Set the binary point register
    #[inline]
    pub unsafe fn set_binary_point(&self, value: u8) {
        self.write_reg(GICC_BPR, (value & 0x7) as u32);
    }

    /// Get the binary point register
    #[inline]
    pub unsafe fn get_binary_point(&self) -> u8 {
        (self.read_reg(GICC_BPR) & 0x7) as u8
    }

    /// Acknowledge an interrupt
    ///
    /// Returns the interrupt ID. Reading this register marks the interrupt
    /// as active.
    #[inline]
    pub unsafe fn acknowledge(&self) -> u32 {
        self.read_reg(GICC_IAR) & 0x3FF
    }

    /// Signal end of interrupt
    #[inline]
    pub unsafe fn end_of_interrupt(&self, intid: u32) {
        self.write_reg(GICC_EOIR, intid);
    }

    /// Deactivate an interrupt (when using EOI mode 1)
    #[inline]
    pub unsafe fn deactivate_interrupt(&self, intid: u32) {
        self.write_reg(GICC_DIR, intid);
    }

    /// Get the running priority
    #[inline]
    pub unsafe fn get_running_priority(&self) -> Priority {
        Priority(self.read_reg(GICC_RPR) as u8)
    }

    /// Get the highest priority pending interrupt
    #[inline]
    pub unsafe fn get_highest_pending(&self) -> u32 {
        self.read_reg(GICC_HPPIR) & 0x3FF
    }

    /// Read the interface identification register
    #[inline]
    pub unsafe fn read_iidr(&self) -> u32 {
        self.read_reg(GICC_IIDR)
    }

    /// Enable the CPU interface
    pub unsafe fn enable(&self) {
        let ctlr = self.read_ctlr() | GICC_CTLR_ENABLE_GRP0 | GICC_CTLR_ENABLE_GRP1;
        self.write_ctlr(ctlr);
    }

    /// Disable the CPU interface
    pub unsafe fn disable(&self) {
        let ctlr = self.read_ctlr() & !(GICC_CTLR_ENABLE_GRP0 | GICC_CTLR_ENABLE_GRP1);
        self.write_ctlr(ctlr);
    }
}

// ============================================================================
// GICv2 Combined Interface
// ============================================================================

/// Complete GICv2 instance with Distributor and CPU Interface
pub struct Gicv2 {
    distributor: Gicv2Distributor,
    cpu_interface: Gicv2CpuInterface,
}

impl Gicv2 {
    /// Create a new GICv2 instance
    ///
    /// # Safety
    ///
    /// Caller must ensure both base addresses are valid.
    pub unsafe fn new(gicd_base: *mut u8, gicc_base: *mut u8) -> Self {
        Self {
            distributor: Gicv2Distributor::new(gicd_base),
            cpu_interface: Gicv2CpuInterface::new(gicc_base),
        }
    }

    /// Initialize the GICv2
    pub unsafe fn init(&self) {
        self.distributor.init();
        self.cpu_interface.init();
    }

    /// Get the distributor
    #[inline]
    pub const fn distributor(&self) -> &Gicv2Distributor {
        &self.distributor
    }

    /// Get the CPU interface
    #[inline]
    pub const fn cpu_interface(&self) -> &Gicv2CpuInterface {
        &self.cpu_interface
    }

    /// Enable an interrupt
    pub unsafe fn enable_interrupt(
        &self,
        intid: u32,
        priority: Priority,
        targets: CpuTargetList,
        trigger: TriggerMode,
    ) {
        let dist = self.distributor.inner();

        // Set priority
        dist.set_priority(intid, priority);

        // Set targets (for SPIs)
        if intid >= SPI_BASE {
            self.distributor.set_target(intid, targets);
            dist.set_trigger_mode(intid, trigger);
        }

        // Set group 1 (non-secure)
        dist.set_group1(intid);

        // Enable the interrupt
        dist.enable_interrupt(intid);
    }

    /// Disable an interrupt
    pub unsafe fn disable_interrupt(&self, intid: u32) {
        self.distributor.inner().disable_interrupt(intid);
    }

    /// Acknowledge an interrupt
    #[inline]
    pub unsafe fn acknowledge(&self) -> u32 {
        self.cpu_interface.acknowledge()
    }

    /// Signal end of interrupt
    #[inline]
    pub unsafe fn end_of_interrupt(&self, intid: u32) {
        self.cpu_interface.end_of_interrupt(intid);
    }

    /// Send an SGI
    pub unsafe fn send_sgi(&self, sgi_id: u8, targets: CpuTargetList) {
        self.distributor.send_sgi(sgi_id, targets);
    }

    /// Get the number of supported interrupts
    pub unsafe fn num_interrupts(&self) -> u32 {
        self.distributor.inner().num_interrupts()
    }
}

// ============================================================================
// Platform Constants
// ============================================================================

/// QEMU virt platform GICC base address
pub const QEMU_VIRT_GICC_BASE: usize = 0x0801_0000;

/// Raspberry Pi 4 GIC400 GICC base address
pub const RPI4_GICC_BASE: usize = 0xFF84_2000;

/// ARM Juno GICC base address
pub const ARM_JUNO_GICC_BASE: usize = 0x2C02_0000;

// ============================================================================
// Convenience Functions
// ============================================================================

/// Initialize a GICv2 for QEMU virt platform
///
/// # Safety
///
/// Must only be called on QEMU virt with GICv2.
pub unsafe fn init_qemu_virt_gicv2() -> Gicv2 {
    use super::distributor::QEMU_VIRT_GICD_BASE;

    let gic = Gicv2::new(
        QEMU_VIRT_GICD_BASE as *mut u8,
        QEMU_VIRT_GICC_BASE as *mut u8,
    );
    gic.init();
    gic
}
