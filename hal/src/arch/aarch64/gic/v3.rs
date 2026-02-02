//! # GICv3 Implementation
//!
//! This module provides the GICv3-specific implementation with system register
//! CPU interface, Redistributors, and advanced features like LPIs and affinity
//! routing.
//!
//! ## GICv3 vs GICv2 Key Differences
//!
//! | Feature              | GICv2                    | GICv3                        |
//! |---------------------|--------------------------|------------------------------|
//! | CPU Interface       | Memory-mapped (GICC)     | System registers (ICC_*)     |
//! | Per-CPU management  | GICD banked registers    | Redistributor (GICR)         |
//! | Interrupt routing   | Target list (8 CPUs max) | Affinity routing (any CPU)   |
//! | LPIs                | Not supported            | Supported (8192+)            |
//! | SGI generation      | GICD_SGIR                | ICC_SGI1R_EL1                |
//! | ITS                 | Not supported            | Optional (PCIe MSI)          |
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────┐
//! │                        GICv3 Architecture                                │
//! ├──────────────────────────────────────────────────────────────────────────┤
//! │                                                                          │
//! │  ┌────────────────────────────────────────────────────────────────────┐ │
//! │  │                    GICD (Distributor)                               │ │
//! │  │  - SPI management (32-1019)                                         │ │
//! │  │  - Affinity routing via IROUTER                                     │ │
//! │  │  - No CPU target list (use affinity instead)                        │ │
//! │  └────────────────────────────────────────────────────────────────────┘ │
//! │                              │                                           │
//! │              ┌───────────────┼───────────────┐                          │
//! │              ▼               ▼               ▼                          │
//! │  ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐        │
//! │  │   GICR (CPU 0)   │ │   GICR (CPU 1)   │ │   GICR (CPU N)   │        │
//! │  │   Redistributor  │ │   Redistributor  │ │   Redistributor  │        │
//! │  │   - SGI/PPI      │ │                  │ │                  │        │
//! │  │   - LPI tables   │ │                  │ │                  │        │
//! │  └──────────────────┘ └──────────────────┘ └──────────────────┘        │
//! │              │               │               │                          │
//! │              ▼               ▼               ▼                          │
//! │  ┌──────────────────┐ ┌──────────────────┐ ┌──────────────────┐        │
//! │  │  ICC_* (CPU 0)   │ │  ICC_* (CPU 1)   │ │  ICC_* (CPU N)   │        │
//! │  │  System Regs     │ │  System Regs     │ │  System Regs     │        │
//! │  │  - Acknowledge   │ │                  │ │                  │        │
//! │  │  - EOI           │ │                  │ │                  │        │
//! │  │  - SGI send      │ │                  │ │                  │        │
//! │  └──────────────────┘ └──────────────────┘ └──────────────────┘        │
//! │                                                                          │
//! │  Optional: ITS (Interrupt Translation Service) for PCIe MSI-X           │
//! └──────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Platforms
//!
//! - QEMU virt (with -machine virt,gic-version=3)
//! - ARM FVP (Fixed Virtual Platform)
//! - AWS Graviton / Ampere Altra
//! - Apple M1/M2 (uses custom interrupt controller but similar concepts)

use core::ptr::{read_volatile, write_volatile};

use super::cpu_interface::{
    self, acknowledge_group1, disable_interrupt_groups, enable_interrupt_groups,
    enable_system_register_interface, end_of_interrupt_group1, set_priority_mask,
    write_icc_sgi1r_el1,
};
use super::distributor::{
    Distributor, GICD_CTLR, GICD_CTLR_ARE_NS, GICD_CTLR_ARE_S, GICD_CTLR_ENABLE_GRP0,
    GICD_CTLR_ENABLE_GRP1NS, GICD_IROUTER,
};
use super::redistributor::{find_redistributor_for_current_cpu, Redistributor};
use super::{CpuAffinity, CpuTargetList, Priority, TriggerMode, SPI_BASE};

// ============================================================================
// IROUTER Register Bits
// ============================================================================

/// Interrupt Routing Mode: Route to any PE (1 of N)
pub const IROUTER_MODE_ANY: u64 = 1 << 31;

/// Interrupt Routing Mode: Route to specific PE
pub const IROUTER_MODE_SPECIFIC: u64 = 0;

// ============================================================================
// SGI Register Bits
// ============================================================================

/// Target List filter: Target specific CPUs
pub const SGI_TARGET_LIST: u64 = 0;

/// Target List filter: Target all except self
pub const SGI_TARGET_ALL_EXCEPT_SELF: u64 = 1 << 40;

/// Interrupt Routing Mode for SGI
pub const SGI_IRM: u64 = 1 << 40;

// ============================================================================
// GICv3 Distributor
// ============================================================================

/// GICv3 Distributor implementation
pub struct Gicv3Distributor {
    inner: Distributor,
}

impl Gicv3Distributor {
    /// Create a new GICv3 Distributor
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

    /// Initialize the GICv3 Distributor
    pub unsafe fn init(&self) {
        // Disable the distributor
        self.inner.write_ctlr(0);

        // Wait for any pending changes
        self.inner.wait_for_rwp();

        // Disable all SPIs
        self.inner.disable_all_spis();

        // Set all SPIs to Group 1 (non-secure)
        self.inner.set_all_spis_group1();

        // Set all SPI priorities to default
        self.inner.set_all_spi_priorities(Priority::DEFAULT);

        // Configure all SPIs to route to any CPU (1-of-N)
        self.set_all_spi_routing_any();

        // Wait for completion
        self.inner.wait_for_rwp();

        // Enable the distributor with Affinity Routing
        let ctlr =
            GICD_CTLR_ENABLE_GRP0 | GICD_CTLR_ENABLE_GRP1NS | GICD_CTLR_ARE_NS | GICD_CTLR_ARE_S;

        self.inner.write_ctlr(ctlr);
    }

    /// Set the routing for an SPI to a specific CPU affinity
    pub unsafe fn set_routing(&self, intid: u32, affinity: CpuAffinity) {
        if intid < SPI_BASE {
            return;
        }

        let value = affinity.to_routing_value() | IROUTER_MODE_SPECIFIC;
        self.inner.write_irouter(intid, value);
    }

    /// Set the routing for an SPI to any available CPU (1-of-N)
    pub unsafe fn set_routing_any(&self, intid: u32) {
        if intid < SPI_BASE {
            return;
        }

        self.inner.write_irouter(intid, IROUTER_MODE_ANY);
    }

    /// Get the routing for an SPI
    pub unsafe fn get_routing(&self, intid: u32) -> (CpuAffinity, bool) {
        let value = self.inner.read_irouter(intid);
        let any_mode = (value & IROUTER_MODE_ANY) != 0;
        let affinity = CpuAffinity {
            aff0: (value & 0xFF) as u8,
            aff1: ((value >> 8) & 0xFF) as u8,
            aff2: ((value >> 16) & 0xFF) as u8,
            aff3: ((value >> 32) & 0xFF) as u8,
        };
        (affinity, any_mode)
    }

    /// Set all SPIs to route to any CPU
    pub unsafe fn set_all_spi_routing_any(&self) {
        let num_irqs = self.inner.num_interrupts();
        for intid in SPI_BASE..num_irqs {
            self.inner.write_irouter(intid, IROUTER_MODE_ANY);
        }
    }

    /// Set all SPIs to route to the current CPU
    pub unsafe fn set_all_spi_routing_current(&self) {
        // Read current CPU's affinity from MPIDR_EL1
        let mpidr: u64;
        core::arch::asm!("mrs {}, mpidr_el1", out(reg) mpidr, options(nomem, nostack));

        let affinity = CpuAffinity::from_mpidr(mpidr);
        let num_irqs = self.inner.num_interrupts();

        for intid in SPI_BASE..num_irqs {
            self.inner.write_irouter(intid, affinity.to_routing_value());
        }
    }

    /// Get the inner Distributor
    #[inline]
    pub const fn inner(&self) -> &Distributor {
        &self.inner
    }
}

// ============================================================================
// GICv3 CPU Interface
// ============================================================================

/// GICv3 CPU Interface (system register based)
///
/// This is a stateless interface since all operations use system registers.
pub struct Gicv3CpuInterface;

impl Gicv3CpuInterface {
    /// Initialize the GICv3 CPU interface
    pub fn init() {
        // Enable the system register interface
        enable_system_register_interface();

        // Set priority mask to allow all interrupts
        set_priority_mask(Priority::LOWEST);

        // Set binary point to 0
        cpu_interface::set_binary_point_group0(0);
        cpu_interface::set_binary_point_group1(0);

        // Enable interrupt groups
        enable_interrupt_groups();
    }

    /// Disable the CPU interface
    pub fn disable() {
        disable_interrupt_groups();
        set_priority_mask(Priority::HIGHEST);
    }

    /// Acknowledge an interrupt
    #[inline]
    pub fn acknowledge() -> u32 {
        acknowledge_group1()
    }

    /// Signal end of interrupt
    #[inline]
    pub fn end_of_interrupt(intid: u32) {
        end_of_interrupt_group1(intid);
    }

    /// Set the priority mask
    #[inline]
    pub fn set_priority_mask(priority: Priority) {
        cpu_interface::set_priority_mask(priority);
    }

    /// Get the priority mask
    #[inline]
    pub fn get_priority_mask() -> Priority {
        cpu_interface::get_priority_mask()
    }

    /// Get the running priority
    #[inline]
    pub fn get_running_priority() -> Priority {
        cpu_interface::get_running_priority()
    }

    /// Send an SGI using the target list
    pub fn send_sgi(sgi_id: u8, targets: CpuTargetList) {
        // Convert CPU mask to affinity targets
        // For simplicity, assume targets are on the same cluster (aff1=0)
        // and encode them in the target list field
        let sgi_value = ((targets.mask() as u64) << 0)  // Target list
            | ((sgi_id as u64 & 0xF) << 24); // INTID

        write_icc_sgi1r_el1(sgi_value);
    }

    /// Send an SGI to a specific CPU affinity
    pub fn send_sgi_to_affinity(sgi_id: u8, affinity: CpuAffinity) {
        let target_list = 1u64 << (affinity.aff0 & 0xF);
        let sgi_value = target_list
            | ((affinity.aff1 as u64) << 16)
            | ((sgi_id as u64 & 0xF) << 24)
            | ((affinity.aff2 as u64) << 32)
            | ((affinity.aff3 as u64) << 48);

        write_icc_sgi1r_el1(sgi_value);
    }

    /// Send an SGI to all CPUs except self
    pub fn send_sgi_all_except_self(sgi_id: u8) {
        let sgi_value = SGI_IRM | ((sgi_id as u64 & 0xF) << 24);
        write_icc_sgi1r_el1(sgi_value);
    }
}

// ============================================================================
// Complete GICv3 Instance
// ============================================================================

/// Complete GICv3 instance with Distributor and current CPU's Redistributor
pub struct Gicv3 {
    distributor: Gicv3Distributor,
    gicr_base: *mut u8,
}

impl Gicv3 {
    /// Create a new GICv3 instance
    ///
    /// # Safety
    ///
    /// Caller must ensure both base addresses are valid.
    pub const unsafe fn new(gicd_base: *mut u8, gicr_base: *mut u8) -> Self {
        Self {
            distributor: Gicv3Distributor::new(gicd_base),
            gicr_base,
        }
    }

    /// Initialize the GICv3 Distributor (call once on BSP)
    pub unsafe fn init_distributor(&self) {
        self.distributor.init();
    }

    /// Initialize the current CPU's interface and Redistributor
    pub unsafe fn init_cpu(&self) {
        // Find and initialize our Redistributor
        if let Some(redist) = find_redistributor_for_current_cpu(self.gicr_base) {
            redist.init();
        }

        // Initialize the CPU interface
        Gicv3CpuInterface::init();
    }

    /// Full initialization (BSP should call this)
    pub unsafe fn init(&self) {
        self.init_distributor();
        self.init_cpu();
    }

    /// Initialize for a secondary CPU (AP)
    pub unsafe fn init_secondary(&self) {
        self.init_cpu();
    }

    /// Get the distributor
    #[inline]
    pub const fn distributor(&self) -> &Gicv3Distributor {
        &self.distributor
    }

    /// Get the Redistributor base address
    #[inline]
    pub const fn gicr_base(&self) -> *mut u8 {
        self.gicr_base
    }

    /// Get the current CPU's Redistributor
    pub unsafe fn current_redistributor(&self) -> Option<Redistributor> {
        find_redistributor_for_current_cpu(self.gicr_base)
    }

    /// Enable an SPI
    pub unsafe fn enable_spi(&self, intid: u32, priority: Priority, trigger: TriggerMode) {
        if intid < SPI_BASE {
            return;
        }

        let dist = self.distributor.inner();

        // Set priority
        dist.set_priority(intid, priority);

        // Set trigger mode
        dist.set_trigger_mode(intid, trigger);

        // Set group 1
        dist.set_group1(intid);

        // Route to any CPU
        self.distributor.set_routing_any(intid);

        // Enable
        dist.enable_interrupt(intid);
    }

    /// Enable an SPI with specific affinity routing
    pub unsafe fn enable_spi_with_affinity(
        &self,
        intid: u32,
        priority: Priority,
        trigger: TriggerMode,
        affinity: CpuAffinity,
    ) {
        if intid < SPI_BASE {
            return;
        }

        let dist = self.distributor.inner();

        dist.set_priority(intid, priority);
        dist.set_trigger_mode(intid, trigger);
        dist.set_group1(intid);
        self.distributor.set_routing(intid, affinity);
        dist.enable_interrupt(intid);
    }

    /// Enable a PPI on the current CPU
    pub unsafe fn enable_ppi(&self, ppi: u8, priority: Priority, trigger: TriggerMode) {
        let intid = 16 + ppi as u32;
        if intid >= 32 {
            return;
        }

        if let Some(redist) = self.current_redistributor() {
            redist.set_priority(intid, priority);
            redist.set_ppi_trigger_mode(intid, trigger);
            redist.set_group1(intid);
            redist.enable_interrupt(intid);
        }
    }

    /// Disable an interrupt
    pub unsafe fn disable_interrupt(&self, intid: u32) {
        if intid >= SPI_BASE {
            self.distributor.inner().disable_interrupt(intid);
        } else if intid < 32 {
            if let Some(redist) = self.current_redistributor() {
                redist.disable_interrupt(intid);
            }
        }
    }

    /// Acknowledge an interrupt
    #[inline]
    pub fn acknowledge(&self) -> u32 {
        Gicv3CpuInterface::acknowledge()
    }

    /// Signal end of interrupt
    #[inline]
    pub fn end_of_interrupt(&self, intid: u32) {
        Gicv3CpuInterface::end_of_interrupt(intid);
    }

    /// Send an SGI
    pub fn send_sgi(&self, sgi_id: u8, targets: CpuTargetList) {
        Gicv3CpuInterface::send_sgi(sgi_id, targets);
    }

    /// Send an SGI to a specific affinity
    pub fn send_sgi_to_affinity(&self, sgi_id: u8, affinity: CpuAffinity) {
        Gicv3CpuInterface::send_sgi_to_affinity(sgi_id, affinity);
    }

    /// Send an SGI to all CPUs except self
    pub fn send_sgi_all_except_self(&self, sgi_id: u8) {
        Gicv3CpuInterface::send_sgi_all_except_self(sgi_id);
    }

    /// Get the number of supported interrupts
    pub unsafe fn num_interrupts(&self) -> u32 {
        self.distributor.inner().num_interrupts()
    }
}

// Safety: Gicv3 uses raw pointers but the data they point to is inherently
// thread-safe (memory-mapped registers). Proper locking must be used at
// higher levels when configuring interrupts.
unsafe impl Send for Gicv3 {}
unsafe impl Sync for Gicv3 {}

// ============================================================================
// Platform Constants
// ============================================================================

/// QEMU virt platform GICD base address (GICv3)
pub const QEMU_VIRT_GICD_BASE: usize = 0x0800_0000;

/// QEMU virt platform GICR base address (GICv3)
pub const QEMU_VIRT_GICR_BASE: usize = 0x080A_0000;

/// ARM FVP GICD base address
pub const ARM_FVP_GICD_BASE: usize = 0x2F00_0000;

/// ARM FVP GICR base address
pub const ARM_FVP_GICR_BASE: usize = 0x2F10_0000;

// ============================================================================
// Convenience Functions
// ============================================================================

/// Initialize a GICv3 for QEMU virt platform
///
/// # Safety
///
/// Must only be called on QEMU virt with GICv3.
pub unsafe fn init_qemu_virt_gicv3() -> Gicv3 {
    let gic = Gicv3::new(
        QEMU_VIRT_GICD_BASE as *mut u8,
        QEMU_VIRT_GICR_BASE as *mut u8,
    );
    gic.init();
    gic
}

/// Initialize a GICv3 for ARM FVP
///
/// # Safety
///
/// Must only be called on ARM FVP with GICv3.
pub unsafe fn init_arm_fvp_gicv3() -> Gicv3 {
    let gic = Gicv3::new(ARM_FVP_GICD_BASE as *mut u8, ARM_FVP_GICR_BASE as *mut u8);
    gic.init();
    gic
}

// ============================================================================
// LPI Support
// ============================================================================

/// LPI (Locality-specific Peripheral Interrupt) configuration
#[derive(Debug)]
pub struct LpiConfig {
    /// Property table physical address (must be 4KB aligned)
    pub prop_table_pa: u64,
    /// Pending table physical address (must be 64KB aligned)
    pub pend_table_pa: u64,
    /// Number of ID bits (determines max LPI count)
    pub id_bits: u8,
}

impl LpiConfig {
    /// Calculate the required property table size
    pub const fn prop_table_size(id_bits: u8) -> usize {
        1 << id_bits
    }

    /// Calculate the required pending table size
    pub const fn pend_table_size(id_bits: u8) -> usize {
        (1 << id_bits) / 8
    }

    /// Enable LPIs on the current CPU's Redistributor
    ///
    /// # Safety
    ///
    /// Caller must ensure tables are properly allocated and configured.
    pub unsafe fn enable_on_redistributor(&self, redist: &Redistributor) {
        // Configure property table
        redist.set_propbase(self.prop_table_pa, self.id_bits);

        // Configure pending table
        redist.set_pendbase(self.pend_table_pa);

        // Enable LPIs
        redist.enable_lpis();
    }
}

/// LPI property entry format
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct LpiProperty {
    /// Priority (upper 2 bits) and enable/group bits (lower bits)
    pub data: u8,
}

impl LpiProperty {
    /// LPI is enabled
    pub const ENABLED: u8 = 1 << 0;

    /// LPI is in Group 1
    pub const GROUP1: u8 = 1 << 1;

    /// Create a new LPI property
    pub const fn new(priority: Priority, enabled: bool, group1: bool) -> Self {
        let mut data = priority.value() & 0xFC; // Upper 6 bits for priority
        if enabled {
            data |= Self::ENABLED;
        }
        if group1 {
            data |= Self::GROUP1;
        }
        Self { data }
    }

    /// Create a disabled LPI
    pub const fn disabled() -> Self {
        Self { data: 0 }
    }

    /// Create an enabled Group 1 LPI with default priority
    pub const fn enabled_group1(priority: Priority) -> Self {
        Self::new(priority, true, true)
    }
}
