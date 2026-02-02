//! # AArch64 Generic Interrupt Controller (GIC) Framework
//!
//! This module provides comprehensive support for ARM Generic Interrupt Controllers,
//! including both GICv2 and GICv3 architectures. The GIC is the standard interrupt
//! controller for ARM-based systems.
//!
//! ## Architecture Overview
//!
//! The GIC manages interrupt routing from peripheral devices to CPU cores:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         GIC Architecture                            │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                                                                     │
//! │  Peripherals        GIC Distributor         CPU Interface    Core   │
//! │  ┌─────────┐       ┌──────────────┐        ┌───────────┐   ┌─────┐ │
//! │  │   SPI   │──────▶│              │        │           │──▶│     │ │
//! │  │ (32-1019)│       │   Routing    │───────▶│  Priority │   │ CPU │ │
//! │  └─────────┘       │   & State    │        │  & Ack    │   │  0  │ │
//! │                    │              │        │           │   │     │ │
//! │  ┌─────────┐       │              │        └───────────┘   └─────┘ │
//! │  │   PPI   │──────▶│              │        ┌───────────┐   ┌─────┐ │
//! │  │ (16-31) │       │              │───────▶│           │──▶│ CPU │ │
//! │  └─────────┘       │              │        │           │   │  1  │ │
//! │                    │              │        └───────────┘   └─────┘ │
//! │  ┌─────────┐       │              │        ┌───────────┐   ┌─────┐ │
//! │  │   SGI   │──────▶│              │───────▶│           │──▶│ CPU │ │
//! │  │  (0-15) │       │              │        │           │   │  N  │ │
//! │  └─────────┘       └──────────────┘        └───────────┘   └─────┘ │
//! │                                                                     │
//! │  GICv3 adds: LPIs (8192+), Redistributor per CPU, ITS               │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Interrupt Types
//!
//! | Type | Range     | Scope      | Description                          |
//! |------|-----------|------------|--------------------------------------|
//! | SGI  | 0-15      | Per-CPU    | Software Generated Interrupts (IPI)  |
//! | PPI  | 16-31     | Per-CPU    | Private Peripheral Interrupts        |
//! | SPI  | 32-1019   | Shared     | Shared Peripheral Interrupts         |
//! | LPI  | 8192+     | GICv3 only | Locality-specific Peripheral Int     |
//!
//! ## Platform Support
//!
//! - **QEMU virt**: GICv2 or GICv3 (configurable)
//! - **Raspberry Pi 4/5**: GICv2 (BCM2711)
//! - **ARM Juno**: GICv2
//! - **ARM FVP**: GICv3
//! - **Server platforms**: GICv3 with ITS
//!
//! ## Usage
//!
//! ```ignore
//! use hal::arch::aarch64::gic::{Gic, GicVersion};
//!
//! // Detect and initialize GIC
//! let gic = Gic::detect(gicd_base, gicc_base);
//!
//! match gic.version() {
//!     GicVersion::V2 => println!("GICv2 detected"),
//!     GicVersion::V3 => println!("GICv3 detected"),
//! }
//!
//! // Initialize distributor
//! gic.init_distributor();
//!
//! // Initialize CPU interface
//! gic.init_cpu_interface();
//!
//! // Enable an interrupt
//! gic.enable_interrupt(33, Priority::default(), CpuTargetList::cpu(0));
//! ```

pub mod cpu_interface;
pub mod distributor;
pub mod redistributor;
pub mod v2;
pub mod v3;

pub use cpu_interface::*;
pub use distributor::*;
pub use redistributor::*;
pub use v2::*;
pub use v3::*;

// ============================================================================
// GIC Constants
// ============================================================================

/// Maximum number of SPIs supported (extended range)
pub const MAX_SPI_ID: u32 = 1019;

/// First SPI interrupt ID
pub const SPI_BASE: u32 = 32;

/// First PPI interrupt ID
pub const PPI_BASE: u32 = 16;

/// Number of SGIs
pub const SGI_COUNT: u32 = 16;

/// Number of PPIs
pub const PPI_COUNT: u32 = 16;

/// First LPI interrupt ID (GICv3)
pub const LPI_BASE: u32 = 8192;

/// Number of interrupts per register (32-bit register, 1 bit per interrupt)
pub const IRQS_PER_ENABLE_REG: u32 = 32;

/// Number of interrupts per priority register (32-bit register, 8 bits per interrupt)
pub const IRQS_PER_PRIORITY_REG: u32 = 4;

/// Number of interrupts per target register (32-bit register, 8 bits per interrupt)
pub const IRQS_PER_TARGET_REG: u32 = 4;

/// Number of interrupts per config register (32-bit register, 2 bits per interrupt)
pub const IRQS_PER_CONFIG_REG: u32 = 16;

/// Special interrupt ID indicating no pending interrupt
pub const INTID_SPURIOUS: u32 = 1023;

/// GICv3 extended spurious ID
pub const INTID_SPURIOUS_EL3: u32 = 1022;

// ============================================================================
// GIC Version Detection
// ============================================================================

/// GIC architecture version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GicVersion {
    /// GICv2: Memory-mapped CPU interface
    V2 = 2,
    /// GICv3: System register CPU interface, Redistributor
    V3 = 3,
    /// GICv4: Virtual LPI support (extension of v3)
    V4 = 4,
}

impl GicVersion {
    /// Detect GIC version from distributor PIDR2 register
    pub fn detect(gicd_base: *const u8) -> Option<Self> {
        // Read GICD_PIDR2 at offset 0xFFE8
        let pidr2 = unsafe { (gicd_base.add(0xFFE8) as *const u32).read_volatile() };
        let arch_rev = (pidr2 >> 4) & 0xF;

        match arch_rev {
            0x1 | 0x2 => Some(GicVersion::V2),
            0x3 => Some(GicVersion::V3),
            0x4 => Some(GicVersion::V4),
            _ => None,
        }
    }

    /// Check if this version supports system register CPU interface
    #[inline]
    pub const fn has_sysreg_interface(self) -> bool {
        matches!(self, GicVersion::V3 | GicVersion::V4)
    }

    /// Check if this version supports Redistributors
    #[inline]
    pub const fn has_redistributor(self) -> bool {
        matches!(self, GicVersion::V3 | GicVersion::V4)
    }

    /// Check if this version supports LPIs
    #[inline]
    pub const fn has_lpis(self) -> bool {
        matches!(self, GicVersion::V3 | GicVersion::V4)
    }

    /// Check if this version supports virtual LPIs
    #[inline]
    pub const fn has_vlpis(self) -> bool {
        matches!(self, GicVersion::V4)
    }
}

// ============================================================================
// Interrupt Types
// ============================================================================

/// Interrupt type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptType {
    /// Software Generated Interrupt (0-15)
    Sgi(u8),
    /// Private Peripheral Interrupt (16-31)
    Ppi(u8),
    /// Shared Peripheral Interrupt (32-1019)
    Spi(u16),
    /// Locality-specific Peripheral Interrupt (8192+, GICv3 only)
    Lpi(u32),
    /// Reserved interrupt ID
    Reserved(u32),
}

impl InterruptType {
    /// Classify an interrupt ID
    pub const fn from_id(intid: u32) -> Self {
        match intid {
            0..=15 => InterruptType::Sgi(intid as u8),
            16..=31 => InterruptType::Ppi((intid - PPI_BASE) as u8),
            32..=1019 => InterruptType::Spi((intid - SPI_BASE) as u16),
            1020..=8191 => InterruptType::Reserved(intid),
            _ => InterruptType::Lpi(intid),
        }
    }

    /// Convert to raw interrupt ID
    pub const fn to_id(self) -> u32 {
        match self {
            InterruptType::Sgi(n) => n as u32,
            InterruptType::Ppi(n) => PPI_BASE + n as u32,
            InterruptType::Spi(n) => SPI_BASE + n as u32,
            InterruptType::Lpi(n) => n,
            InterruptType::Reserved(n) => n,
        }
    }

    /// Check if this is a per-CPU interrupt (SGI or PPI)
    pub const fn is_banked(self) -> bool {
        matches!(self, InterruptType::Sgi(_) | InterruptType::Ppi(_))
    }

    /// Check if this is a shared interrupt (SPI)
    pub const fn is_shared(self) -> bool {
        matches!(self, InterruptType::Spi(_))
    }
}

// ============================================================================
// Priority and Targeting
// ============================================================================

/// Interrupt priority (0 = highest, 255 = lowest)
///
/// Note: Many GIC implementations only support a subset of priority bits.
/// Common implementations support 16 or 32 priority levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Priority(pub u8);

impl Priority {
    /// Highest priority (most urgent)
    pub const HIGHEST: Self = Priority(0);

    /// High priority
    pub const HIGH: Self = Priority(0x40);

    /// Default/medium priority
    pub const DEFAULT: Self = Priority(0x80);

    /// Low priority
    pub const LOW: Self = Priority(0xC0);

    /// Lowest priority (least urgent)
    pub const LOWEST: Self = Priority(0xFF);

    /// Create from raw value
    #[inline]
    pub const fn new(value: u8) -> Self {
        Priority(value)
    }

    /// Get raw priority value
    #[inline]
    pub const fn value(self) -> u8 {
        self.0
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::DEFAULT
    }
}

/// Interrupt trigger configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TriggerMode {
    /// Level-sensitive: Interrupt remains asserted while signal is active
    Level = 0,
    /// Edge-triggered: Interrupt on rising edge
    Edge  = 1,
}

impl Default for TriggerMode {
    fn default() -> Self {
        TriggerMode::Level
    }
}

/// CPU target specification for interrupt routing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuTargetList(pub u8);

impl CpuTargetList {
    /// No CPUs targeted
    pub const NONE: Self = CpuTargetList(0);

    /// All CPUs targeted (broadcast)
    pub const ALL: Self = CpuTargetList(0xFF);

    /// Target a specific CPU (0-7)
    #[inline]
    pub const fn cpu(n: u8) -> Self {
        CpuTargetList(1 << (n & 7))
    }

    /// Target CPU 0
    pub const CPU0: Self = CpuTargetList(1 << 0);

    /// Target CPU 1
    pub const CPU1: Self = CpuTargetList(1 << 1);

    /// Add a CPU to the target list
    #[inline]
    pub const fn with_cpu(self, n: u8) -> Self {
        CpuTargetList(self.0 | (1 << (n & 7)))
    }

    /// Check if a CPU is targeted
    #[inline]
    pub const fn targets_cpu(self, n: u8) -> bool {
        (self.0 & (1 << (n & 7))) != 0
    }

    /// Get the raw target mask
    #[inline]
    pub const fn mask(self) -> u8 {
        self.0
    }
}

// ============================================================================
// GICv3 Affinity Routing
// ============================================================================

/// CPU affinity for GICv3 interrupt routing
///
/// Matches MPIDR affinity fields: Aff3.Aff2.Aff1.Aff0
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuAffinity {
    /// Affinity level 0 (typically core within cluster)
    pub aff0: u8,
    /// Affinity level 1 (typically cluster)
    pub aff1: u8,
    /// Affinity level 2 (typically socket)
    pub aff2: u8,
    /// Affinity level 3 (typically node)
    pub aff3: u8,
}

impl CpuAffinity {
    /// Create affinity from individual levels
    pub const fn new(aff3: u8, aff2: u8, aff1: u8, aff0: u8) -> Self {
        Self {
            aff0,
            aff1,
            aff2,
            aff3,
        }
    }

    /// Create from MPIDR value
    pub const fn from_mpidr(mpidr: u64) -> Self {
        Self {
            aff0: (mpidr & 0xFF) as u8,
            aff1: ((mpidr >> 8) & 0xFF) as u8,
            aff2: ((mpidr >> 16) & 0xFF) as u8,
            aff3: ((mpidr >> 32) & 0xFF) as u8,
        }
    }

    /// Convert to 64-bit affinity value for IROUTER
    pub const fn to_routing_value(self) -> u64 {
        (self.aff0 as u64)
            | ((self.aff1 as u64) << 8)
            | ((self.aff2 as u64) << 16)
            | ((self.aff3 as u64) << 32)
    }

    /// Create affinity targeting the current CPU (1:1 routing mode)
    pub const fn current_cpu() -> Self {
        // Bit 31 of IROUTER means "route to executing PE"
        Self {
            aff0: 0,
            aff1: 0,
            aff2: 0,
            aff3: 0,
        }
    }
}

// ============================================================================
// Interrupt State
// ============================================================================

/// Interrupt state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptState {
    /// Inactive: Not pending or active
    Inactive,
    /// Pending: Waiting to be acknowledged
    Pending,
    /// Active: Being processed by a CPU
    Active,
    /// Active and Pending: Being processed and will re-trigger
    ActiveAndPending,
}

impl InterruptState {
    /// Decode from ISPENDR and ISACTIVER bit values
    pub const fn from_bits(pending: bool, active: bool) -> Self {
        match (pending, active) {
            (false, false) => InterruptState::Inactive,
            (true, false) => InterruptState::Pending,
            (false, true) => InterruptState::Active,
            (true, true) => InterruptState::ActiveAndPending,
        }
    }
}

// ============================================================================
// High-Level GIC Interface
// ============================================================================

/// Unified GIC interface for both GICv2 and GICv3
pub struct Gic {
    /// GIC version
    version: GicVersion,
    /// Distributor base address (GICD)
    gicd_base: *mut u8,
    /// CPU interface base address (GICC, GICv2 only)
    gicc_base: *mut u8,
    /// Redistributor base address (GICR, GICv3 only)
    gicr_base: *mut u8,
    /// Number of supported interrupts
    num_irqs: u32,
    /// Number of implemented priority bits
    priority_bits: u8,
}

impl Gic {
    /// Create a new GIC instance with auto-detection
    ///
    /// # Safety
    ///
    /// Caller must ensure the base addresses are valid and point to GIC registers.
    pub unsafe fn new(gicd_base: *mut u8, gicc_base: *mut u8, gicr_base: *mut u8) -> Option<Self> {
        let version = GicVersion::detect(gicd_base)?;

        // Read GICD_TYPER to get interrupt count
        let typer = (gicd_base as *const u32).add(0x4 / 4).read_volatile();
        let it_lines = typer & 0x1F;
        let num_irqs = 32 * (it_lines + 1);

        // Priority bits (common: 4 or 5 bits = 16 or 32 levels)
        let priority_bits = 8; // Default to 8, actual may be less

        Some(Self {
            version,
            gicd_base,
            gicc_base,
            gicr_base,
            num_irqs,
            priority_bits,
        })
    }

    /// Get the GIC version
    #[inline]
    pub const fn version(&self) -> GicVersion {
        self.version
    }

    /// Get the number of supported interrupts
    #[inline]
    pub const fn num_irqs(&self) -> u32 {
        self.num_irqs
    }

    /// Get the distributor base address
    #[inline]
    pub const fn gicd_base(&self) -> *mut u8 {
        self.gicd_base
    }

    /// Initialize the GIC
    ///
    /// This initializes the distributor, sets up the CPU interface,
    /// and enables interrupt delivery.
    pub fn init(&mut self) {
        self.init_distributor();
        self.init_cpu_interface();
    }

    /// Initialize the GIC Distributor
    pub fn init_distributor(&self) {
        match self.version {
            GicVersion::V2 => unsafe {
                v2::Gicv2Distributor::new(self.gicd_base).init();
            },
            GicVersion::V3 | GicVersion::V4 => unsafe {
                v3::Gicv3Distributor::new(self.gicd_base).init();
            },
        }
    }

    /// Initialize the CPU interface for the current CPU
    pub fn init_cpu_interface(&self) {
        match self.version {
            GicVersion::V2 => unsafe {
                v2::Gicv2CpuInterface::new(self.gicc_base).init();
            },
            GicVersion::V3 | GicVersion::V4 => {
                v3::Gicv3CpuInterface::init();
            },
        }
    }

    /// Enable an interrupt
    pub fn enable_interrupt(&self, intid: u32, priority: Priority, trigger: TriggerMode) {
        if intid >= self.num_irqs {
            return;
        }

        unsafe {
            let dist = Distributor::new(self.gicd_base);

            // Set priority
            dist.set_priority(intid, priority);

            // Set trigger mode (only for SPIs and some PPIs)
            if intid >= SPI_BASE {
                dist.set_trigger_mode(intid, trigger);
            }

            // Enable the interrupt
            dist.enable_interrupt(intid);
        }
    }

    /// Disable an interrupt
    pub fn disable_interrupt(&self, intid: u32) {
        if intid >= self.num_irqs {
            return;
        }

        unsafe {
            Distributor::new(self.gicd_base).disable_interrupt(intid);
        }
    }

    /// Set interrupt target CPUs (GICv2 only)
    pub fn set_target(&self, intid: u32, targets: CpuTargetList) {
        if self.version == GicVersion::V2 && intid >= SPI_BASE {
            unsafe {
                v2::Gicv2Distributor::new(self.gicd_base).set_target(intid, targets);
            }
        }
    }

    /// Set interrupt routing affinity (GICv3 only)
    pub fn set_routing(&self, intid: u32, affinity: CpuAffinity) {
        if self.version.has_sysreg_interface() && intid >= SPI_BASE {
            unsafe {
                v3::Gicv3Distributor::new(self.gicd_base).set_routing(intid, affinity);
            }
        }
    }

    /// Acknowledge an interrupt
    ///
    /// Returns the interrupt ID, or None if spurious.
    pub fn acknowledge(&self) -> Option<u32> {
        let intid = match self.version {
            GicVersion::V2 => unsafe { v2::Gicv2CpuInterface::new(self.gicc_base).acknowledge() },
            GicVersion::V3 | GicVersion::V4 => v3::Gicv3CpuInterface::acknowledge(),
        };

        if intid == INTID_SPURIOUS || intid == INTID_SPURIOUS_EL3 {
            None
        } else {
            Some(intid)
        }
    }

    /// Signal end of interrupt processing
    pub fn end_of_interrupt(&self, intid: u32) {
        match self.version {
            GicVersion::V2 => unsafe {
                v2::Gicv2CpuInterface::new(self.gicc_base).end_of_interrupt(intid);
            },
            GicVersion::V3 | GicVersion::V4 => {
                v3::Gicv3CpuInterface::end_of_interrupt(intid);
            },
        }
    }

    /// Send a Software Generated Interrupt (SGI)
    pub fn send_sgi(&self, sgi_id: u8, targets: CpuTargetList) {
        if sgi_id >= 16 {
            return;
        }

        match self.version {
            GicVersion::V2 => unsafe {
                v2::Gicv2Distributor::new(self.gicd_base).send_sgi(sgi_id, targets);
            },
            GicVersion::V3 | GicVersion::V4 => {
                v3::Gicv3CpuInterface::send_sgi(sgi_id, targets);
            },
        }
    }

    /// Get the state of an interrupt
    pub fn get_interrupt_state(&self, intid: u32) -> InterruptState {
        unsafe {
            let dist = Distributor::new(self.gicd_base);
            let pending = dist.is_pending(intid);
            let active = dist.is_active(intid);
            InterruptState::from_bits(pending, active)
        }
    }

    /// Set the priority mask (threshold)
    pub fn set_priority_mask(&self, priority: Priority) {
        match self.version {
            GicVersion::V2 => unsafe {
                v2::Gicv2CpuInterface::new(self.gicc_base).set_priority_mask(priority);
            },
            GicVersion::V3 | GicVersion::V4 => {
                v3::Gicv3CpuInterface::set_priority_mask(priority);
            },
        }
    }
}

// Safety: GIC is not Send/Sync by default due to raw pointers
// In a real kernel, you'd implement proper synchronization
unsafe impl Send for Gic {}
unsafe impl Sync for Gic {}

// ============================================================================
// Helper Functions
// ============================================================================

/// Calculate the register offset for a given interrupt ID
/// For registers with 1 bit per interrupt (enable, pending, active)
#[inline]
pub const fn bit_reg_offset(intid: u32) -> (usize, u32) {
    let reg_index = (intid / 32) as usize;
    let bit_index = intid % 32;
    (reg_index, bit_index)
}

/// Calculate the register offset for a given interrupt ID
/// For registers with 8 bits per interrupt (priority, targets)
#[inline]
pub const fn byte_reg_offset(intid: u32) -> (usize, u32) {
    let reg_index = (intid / 4) as usize;
    let byte_offset = (intid % 4) * 8;
    (reg_index, byte_offset)
}

/// Calculate the register offset for configuration registers
/// 2 bits per interrupt
#[inline]
pub const fn config_reg_offset(intid: u32) -> (usize, u32) {
    let reg_index = (intid / 16) as usize;
    let bit_offset = (intid % 16) * 2;
    (reg_index, bit_offset)
}
