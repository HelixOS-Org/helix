//! # GIC Distributor (GICD)
//!
//! The Distributor is the central component of the GIC that manages interrupt
//! routing, priority, and enable state for all interrupts (SPIs, PPIs, SGIs).
//!
//! ## Key Responsibilities
//!
//! - Interrupt enable/disable control
//! - Priority configuration
//! - Target CPU routing (GICv2) or affinity routing (GICv3)
//! - Trigger mode configuration (level/edge)
//! - Interrupt state management (pending/active)
//! - SGI generation
//!
//! ## Register Map
//!
//! | Offset    | Register         | Description                          |
//! |-----------|------------------|--------------------------------------|
//! | 0x0000    | GICD_CTLR        | Distributor Control                  |
//! | 0x0004    | GICD_TYPER       | Interrupt Controller Type            |
//! | 0x0008    | GICD_IIDR        | Implementer Identification           |
//! | 0x0080    | GICD_IGROUPR     | Interrupt Group Registers            |
//! | 0x0100    | GICD_ISENABLER   | Set-Enable Registers                 |
//! | 0x0180    | GICD_ICENABLER   | Clear-Enable Registers               |
//! | 0x0200    | GICD_ISPENDR     | Set-Pending Registers                |
//! | 0x0280    | GICD_ICPENDR     | Clear-Pending Registers              |
//! | 0x0300    | GICD_ISACTIVER   | Set-Active Registers                 |
//! | 0x0380    | GICD_ICACTIVER   | Clear-Active Registers               |
//! | 0x0400    | GICD_IPRIORITYR  | Priority Registers                   |
//! | 0x0800    | GICD_ITARGETSR   | Target Registers (GICv2)             |
//! | 0x0C00    | GICD_ICFGR       | Configuration Registers              |
//! | 0x0F00    | GICD_SGIR        | SGI Register (GICv2)                 |
//! | 0x6000    | GICD_IROUTER     | Routing Registers (GICv3)            |

use super::{
    bit_reg_offset, byte_reg_offset, config_reg_offset, Priority, TriggerMode, SPI_BASE,
};
use core::ptr::{read_volatile, write_volatile};

// ============================================================================
// GICD Register Offsets
// ============================================================================

/// GICD Control Register
pub const GICD_CTLR: usize = 0x0000;

/// GICD Type Register
pub const GICD_TYPER: usize = 0x0004;

/// GICD Implementer Identification Register
pub const GICD_IIDR: usize = 0x0008;

/// GICD Interrupt Group Registers (banked for SGI/PPI)
pub const GICD_IGROUPR: usize = 0x0080;

/// GICD Interrupt Set-Enable Registers
pub const GICD_ISENABLER: usize = 0x0100;

/// GICD Interrupt Clear-Enable Registers
pub const GICD_ICENABLER: usize = 0x0180;

/// GICD Interrupt Set-Pending Registers
pub const GICD_ISPENDR: usize = 0x0200;

/// GICD Interrupt Clear-Pending Registers
pub const GICD_ICPENDR: usize = 0x0280;

/// GICD Interrupt Set-Active Registers
pub const GICD_ISACTIVER: usize = 0x0300;

/// GICD Interrupt Clear-Active Registers
pub const GICD_ICACTIVER: usize = 0x0380;

/// GICD Interrupt Priority Registers
pub const GICD_IPRIORITYR: usize = 0x0400;

/// GICD Interrupt Target Registers (GICv2, banked for SGI/PPI)
pub const GICD_ITARGETSR: usize = 0x0800;

/// GICD Interrupt Configuration Registers
pub const GICD_ICFGR: usize = 0x0C00;

/// GICD Non-secure Access Control Registers
pub const GICD_NSACR: usize = 0x0E00;

/// GICD Software Generated Interrupt Register (GICv2)
pub const GICD_SGIR: usize = 0x0F00;

/// GICD SGI Clear-Pending Registers
pub const GICD_CPENDSGIR: usize = 0x0F10;

/// GICD SGI Set-Pending Registers
pub const GICD_SPENDSGIR: usize = 0x0F20;

/// GICD Interrupt Routing Registers (GICv3, 64-bit)
pub const GICD_IROUTER: usize = 0x6100;

/// GICD Peripheral ID2 Register (contains architecture revision)
pub const GICD_PIDR2: usize = 0xFFE8;

// ============================================================================
// GICD_CTLR Bits
// ============================================================================

/// Enable Group 0 interrupts
pub const GICD_CTLR_ENABLE_GRP0: u32 = 1 << 0;

/// Enable Group 1 interrupts (Non-secure in Secure view)
pub const GICD_CTLR_ENABLE_GRP1: u32 = 1 << 1;

/// Enable Group 1 Non-secure interrupts (GICv3)
pub const GICD_CTLR_ENABLE_GRP1NS: u32 = 1 << 1;

/// Enable Group 1 Secure interrupts (GICv3)
pub const GICD_CTLR_ENABLE_GRP1S: u32 = 1 << 2;

/// Affinity Routing Enable (Non-secure, GICv3)
pub const GICD_CTLR_ARE_NS: u32 = 1 << 4;

/// Affinity Routing Enable (Secure, GICv3)
pub const GICD_CTLR_ARE_S: u32 = 1 << 5;

/// Disable Security (GICv3)
pub const GICD_CTLR_DS: u32 = 1 << 6;

/// Enable 1 of N wakeup functionality (GICv3)
pub const GICD_CTLR_E1NWF: u32 = 1 << 7;

/// Register Write Pending (GICv3)
pub const GICD_CTLR_RWP: u32 = 1 << 31;

// ============================================================================
// GICD_TYPER Bits
// ============================================================================

/// Mask for ITLinesNumber field
pub const GICD_TYPER_ITLINES_MASK: u32 = 0x1F;

/// CPU number mask
pub const GICD_TYPER_CPUNUMBER_MASK: u32 = 0x7 << 5;

/// Security Extension supported
pub const GICD_TYPER_SECURITY_EXTN: u32 = 1 << 10;

/// Number of implemented lockable SPIs
pub const GICD_TYPER_LSPI_MASK: u32 = 0x1F << 11;

/// MBIS (Message Based Interrupt) support
pub const GICD_TYPER_MBIS: u32 = 1 << 16;

/// LPIS support
pub const GICD_TYPER_LPIS: u32 = 1 << 17;

/// Direct Virtual LPI injection support
pub const GICD_TYPER_DVIS: u32 = 1 << 18;

/// Extended SPI range support
pub const GICD_TYPER_ESPI: u32 = 1 << 8;

// ============================================================================
// Distributor Structure
// ============================================================================

/// GIC Distributor (common interface for GICv2/v3)
pub struct Distributor {
    base: *mut u8,
}

impl Distributor {
    /// Create a new Distributor from base address
    ///
    /// # Safety
    ///
    /// Caller must ensure the base address is valid and points to GICD registers.
    #[inline]
    pub const unsafe fn new(base: *mut u8) -> Self {
        Self { base }
    }

    /// Get the base address
    #[inline]
    pub const fn base(&self) -> *mut u8 {
        self.base
    }

    // ========================================================================
    // Register Access Helpers
    // ========================================================================

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

    /// Read a 64-bit register
    #[inline]
    unsafe fn read_reg64(&self, offset: usize) -> u64 {
        read_volatile((self.base as *const u64).add(offset / 8))
    }

    /// Write a 64-bit register
    #[inline]
    unsafe fn write_reg64(&self, offset: usize, value: u64) {
        write_volatile((self.base as *mut u64).add(offset / 8), value);
    }

    // ========================================================================
    // Control and Status
    // ========================================================================

    /// Read GICD_CTLR
    #[inline]
    pub unsafe fn read_ctlr(&self) -> u32 {
        self.read_reg(GICD_CTLR)
    }

    /// Write GICD_CTLR
    #[inline]
    pub unsafe fn write_ctlr(&self, value: u32) {
        self.write_reg(GICD_CTLR, value);
    }

    /// Read GICD_TYPER
    #[inline]
    pub unsafe fn read_typer(&self) -> u32 {
        self.read_reg(GICD_TYPER)
    }

    /// Read GICD_IIDR
    #[inline]
    pub unsafe fn read_iidr(&self) -> u32 {
        self.read_reg(GICD_IIDR)
    }

    /// Get the number of supported interrupt lines
    #[inline]
    pub unsafe fn num_interrupts(&self) -> u32 {
        let typer = self.read_typer();
        let it_lines = typer & GICD_TYPER_ITLINES_MASK;
        32 * (it_lines + 1)
    }

    /// Get the number of implemented CPUs
    #[inline]
    pub unsafe fn num_cpus(&self) -> u32 {
        let typer = self.read_typer();
        ((typer & GICD_TYPER_CPUNUMBER_MASK) >> 5) + 1
    }

    /// Check if security extensions are supported
    #[inline]
    pub unsafe fn has_security_extensions(&self) -> bool {
        (self.read_typer() & GICD_TYPER_SECURITY_EXTN) != 0
    }

    /// Wait for register write to complete (GICv3)
    #[inline]
    pub unsafe fn wait_for_rwp(&self) {
        while (self.read_ctlr() & GICD_CTLR_RWP) != 0 {
            core::hint::spin_loop();
        }
    }

    // ========================================================================
    // Interrupt Enable/Disable
    // ========================================================================

    /// Enable an interrupt
    #[inline]
    pub unsafe fn enable_interrupt(&self, intid: u32) {
        let (reg_index, bit) = bit_reg_offset(intid);
        self.write_reg(GICD_ISENABLER + reg_index * 4, 1 << bit);
    }

    /// Disable an interrupt
    #[inline]
    pub unsafe fn disable_interrupt(&self, intid: u32) {
        let (reg_index, bit) = bit_reg_offset(intid);
        self.write_reg(GICD_ICENABLER + reg_index * 4, 1 << bit);
    }

    /// Check if an interrupt is enabled
    #[inline]
    pub unsafe fn is_enabled(&self, intid: u32) -> bool {
        let (reg_index, bit) = bit_reg_offset(intid);
        (self.read_reg(GICD_ISENABLER + reg_index * 4) & (1 << bit)) != 0
    }

    /// Disable all SPIs
    pub unsafe fn disable_all_spis(&self) {
        let num_irqs = self.num_interrupts();
        for i in (SPI_BASE..num_irqs).step_by(32) {
            let reg_index = (i / 32) as usize;
            self.write_reg(GICD_ICENABLER + reg_index * 4, 0xFFFF_FFFF);
        }
    }

    // ========================================================================
    // Pending State
    // ========================================================================

    /// Set an interrupt pending
    #[inline]
    pub unsafe fn set_pending(&self, intid: u32) {
        let (reg_index, bit) = bit_reg_offset(intid);
        self.write_reg(GICD_ISPENDR + reg_index * 4, 1 << bit);
    }

    /// Clear an interrupt pending state
    #[inline]
    pub unsafe fn clear_pending(&self, intid: u32) {
        let (reg_index, bit) = bit_reg_offset(intid);
        self.write_reg(GICD_ICPENDR + reg_index * 4, 1 << bit);
    }

    /// Check if an interrupt is pending
    #[inline]
    pub unsafe fn is_pending(&self, intid: u32) -> bool {
        let (reg_index, bit) = bit_reg_offset(intid);
        (self.read_reg(GICD_ISPENDR + reg_index * 4) & (1 << bit)) != 0
    }

    // ========================================================================
    // Active State
    // ========================================================================

    /// Set an interrupt active
    #[inline]
    pub unsafe fn set_active(&self, intid: u32) {
        let (reg_index, bit) = bit_reg_offset(intid);
        self.write_reg(GICD_ISACTIVER + reg_index * 4, 1 << bit);
    }

    /// Clear an interrupt active state
    #[inline]
    pub unsafe fn clear_active(&self, intid: u32) {
        let (reg_index, bit) = bit_reg_offset(intid);
        self.write_reg(GICD_ICACTIVER + reg_index * 4, 1 << bit);
    }

    /// Check if an interrupt is active
    #[inline]
    pub unsafe fn is_active(&self, intid: u32) -> bool {
        let (reg_index, bit) = bit_reg_offset(intid);
        (self.read_reg(GICD_ISACTIVER + reg_index * 4) & (1 << bit)) != 0
    }

    // ========================================================================
    // Priority
    // ========================================================================

    /// Set the priority of an interrupt
    #[inline]
    pub unsafe fn set_priority(&self, intid: u32, priority: Priority) {
        let (reg_index, byte_offset) = byte_reg_offset(intid);
        let addr = (self.base as *mut u8).add(GICD_IPRIORITYR + reg_index * 4 + byte_offset as usize);
        write_volatile(addr, priority.value());
    }

    /// Get the priority of an interrupt
    #[inline]
    pub unsafe fn get_priority(&self, intid: u32) -> Priority {
        let (reg_index, byte_offset) = byte_reg_offset(intid);
        let addr = (self.base as *const u8).add(GICD_IPRIORITYR + reg_index * 4 + byte_offset as usize);
        Priority(read_volatile(addr))
    }

    /// Set all SPI priorities to a default value
    pub unsafe fn set_all_spi_priorities(&self, priority: Priority) {
        let num_irqs = self.num_interrupts();
        let value = (priority.value() as u32) * 0x01010101;
        for i in (SPI_BASE..num_irqs).step_by(4) {
            let reg_index = (i / 4) as usize;
            self.write_reg(GICD_IPRIORITYR + reg_index * 4, value);
        }
    }

    // ========================================================================
    // Trigger Configuration
    // ========================================================================

    /// Set the trigger mode of an interrupt
    pub unsafe fn set_trigger_mode(&self, intid: u32, mode: TriggerMode) {
        let (reg_index, bit_offset) = config_reg_offset(intid);
        let mut config = self.read_reg(GICD_ICFGR + reg_index * 4);

        // Clear the config bits for this interrupt
        config &= !(0x3 << bit_offset);

        // Set the new mode (bit 1 of the 2-bit field determines level/edge)
        match mode {
            TriggerMode::Level => {
                // Bit 1 = 0 for level-triggered
            }
            TriggerMode::Edge => {
                // Bit 1 = 1 for edge-triggered
                config |= 0x2 << bit_offset;
            }
        }

        self.write_reg(GICD_ICFGR + reg_index * 4, config);
    }

    /// Get the trigger mode of an interrupt
    pub unsafe fn get_trigger_mode(&self, intid: u32) -> TriggerMode {
        let (reg_index, bit_offset) = config_reg_offset(intid);
        let config = self.read_reg(GICD_ICFGR + reg_index * 4);
        let bits = (config >> bit_offset) & 0x3;

        if (bits & 0x2) != 0 {
            TriggerMode::Edge
        } else {
            TriggerMode::Level
        }
    }

    // ========================================================================
    // Group Configuration
    // ========================================================================

    /// Set an interrupt to Group 0 (secure, typically FIQ)
    #[inline]
    pub unsafe fn set_group0(&self, intid: u32) {
        let (reg_index, bit) = bit_reg_offset(intid);
        let mut group = self.read_reg(GICD_IGROUPR + reg_index * 4);
        group &= !(1 << bit);
        self.write_reg(GICD_IGROUPR + reg_index * 4, group);
    }

    /// Set an interrupt to Group 1 (non-secure, typically IRQ)
    #[inline]
    pub unsafe fn set_group1(&self, intid: u32) {
        let (reg_index, bit) = bit_reg_offset(intid);
        let mut group = self.read_reg(GICD_IGROUPR + reg_index * 4);
        group |= 1 << bit;
        self.write_reg(GICD_IGROUPR + reg_index * 4, group);
    }

    /// Set all SPIs to Group 1 (non-secure)
    pub unsafe fn set_all_spis_group1(&self) {
        let num_irqs = self.num_interrupts();
        for i in (SPI_BASE..num_irqs).step_by(32) {
            let reg_index = (i / 32) as usize;
            self.write_reg(GICD_IGROUPR + reg_index * 4, 0xFFFF_FFFF);
        }
    }

    // ========================================================================
    // Routing (GICv3)
    // ========================================================================

    /// Read IROUTER for an SPI (GICv3)
    #[inline]
    pub unsafe fn read_irouter(&self, intid: u32) -> u64 {
        debug_assert!(intid >= SPI_BASE);
        let offset = GICD_IROUTER + ((intid - SPI_BASE) as usize) * 8;
        self.read_reg64(offset)
    }

    /// Write IROUTER for an SPI (GICv3)
    #[inline]
    pub unsafe fn write_irouter(&self, intid: u32, value: u64) {
        debug_assert!(intid >= SPI_BASE);
        let offset = GICD_IROUTER + ((intid - SPI_BASE) as usize) * 8;
        self.write_reg64(offset, value);
    }
}

// ============================================================================
// Distributor Information
// ============================================================================

/// Information about the GIC Distributor
#[derive(Debug, Clone)]
pub struct DistributorInfo {
    /// Number of supported interrupts
    pub num_interrupts: u32,
    /// Number of implemented CPUs
    pub num_cpus: u32,
    /// Security extensions supported
    pub has_security: bool,
    /// Implementer ID
    pub iidr: u32,
    /// Architecture revision from PIDR2
    pub arch_revision: u8,
}

impl DistributorInfo {
    /// Read distributor information
    ///
    /// # Safety
    ///
    /// Caller must ensure the base address is valid.
    pub unsafe fn from_base(base: *mut u8) -> Self {
        let dist = Distributor::new(base);
        let typer = dist.read_typer();
        let iidr = dist.read_iidr();
        let pidr2 = dist.read_reg(GICD_PIDR2);

        Self {
            num_interrupts: dist.num_interrupts(),
            num_cpus: dist.num_cpus(),
            has_security: (typer & GICD_TYPER_SECURITY_EXTN) != 0,
            iidr,
            arch_revision: ((pidr2 >> 4) & 0xF) as u8,
        }
    }
}

// ============================================================================
// Interrupt Configuration Builder
// ============================================================================

/// Builder for configuring multiple interrupts
pub struct InterruptConfigBuilder<'a> {
    dist: &'a Distributor,
}

impl<'a> InterruptConfigBuilder<'a> {
    /// Create a new builder
    pub const fn new(dist: &'a Distributor) -> Self {
        Self { dist }
    }

    /// Configure a range of SPIs with the same settings
    ///
    /// # Safety
    ///
    /// Caller must ensure interrupt IDs are valid.
    pub unsafe fn configure_spi_range(
        &self,
        start: u32,
        end: u32,
        priority: Priority,
        trigger: TriggerMode,
        enable: bool,
    ) {
        for intid in start..=end {
            if intid < SPI_BASE {
                continue;
            }

            self.dist.set_priority(intid, priority);
            self.dist.set_trigger_mode(intid, trigger);
            self.dist.set_group1(intid);

            if enable {
                self.dist.enable_interrupt(intid);
            } else {
                self.dist.disable_interrupt(intid);
            }
        }
    }
}

// ============================================================================
// Constants for Common Platforms
// ============================================================================

/// QEMU virt platform GICD base address
pub const QEMU_VIRT_GICD_BASE: usize = 0x0800_0000;

/// Raspberry Pi 4 GIC400 GICD base address
pub const RPI4_GICD_BASE: usize = 0xFF84_1000;

/// ARM Juno GICD base address
pub const ARM_JUNO_GICD_BASE: usize = 0x2C01_0000;
