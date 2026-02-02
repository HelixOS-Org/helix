//! # Interrupt Handling
//!
//! GPU interrupt management and MSI-X support.

use magma_core::{Error, Result};

// =============================================================================
// INTERRUPT TYPES
// =============================================================================

/// GPU interrupt sources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum InterruptSource {
    /// FIFO engine interrupt
    Fifo     = 0,
    /// Graphics engine interrupt
    Graphics = 1,
    /// Copy engine interrupt
    Copy     = 2,
    /// Display interrupt
    Display  = 3,
    /// GSP interrupt
    Gsp      = 4,
    /// Fault interrupt (MMU, etc.)
    Fault    = 5,
    /// Timer interrupt
    Timer    = 6,
    /// LTC (L2 cache) interrupt
    Ltc      = 7,
}

/// Interrupt status
#[derive(Debug, Clone)]
pub struct InterruptStatus {
    /// Pending interrupt mask
    pub pending: u32,
    /// Enabled interrupt mask
    pub enabled: u32,
}

impl InterruptStatus {
    /// Check if any interrupt is pending
    pub fn has_pending(&self) -> bool {
        (self.pending & self.enabled) != 0
    }

    /// Check if specific source is pending
    pub fn is_pending(&self, source: InterruptSource) -> bool {
        let mask = 1 << (source as u32);
        (self.pending & self.enabled & mask) != 0
    }
}

// =============================================================================
// MSI-X
// =============================================================================

/// MSI-X table entry
#[derive(Debug, Clone, Copy)]
pub struct MsixEntry {
    /// Message address (low 32 bits)
    pub msg_addr_lo: u32,
    /// Message address (high 32 bits)
    pub msg_addr_hi: u32,
    /// Message data
    pub msg_data: u32,
    /// Vector control (bit 0 = masked)
    pub vector_ctrl: u32,
}

impl MsixEntry {
    /// Check if vector is masked
    pub fn is_masked(&self) -> bool {
        (self.vector_ctrl & 1) != 0
    }
}

/// MSI-X capability information
#[derive(Debug, Clone)]
pub struct MsixCapability {
    /// Capability offset in PCI config space
    pub cap_offset: u8,
    /// Table size (number of vectors)
    pub table_size: u16,
    /// Table BAR index
    pub table_bar: u8,
    /// Table offset within BAR
    pub table_offset: u32,
    /// PBA BAR index
    pub pba_bar: u8,
    /// PBA offset within BAR
    pub pba_offset: u32,
}

// =============================================================================
// INTERRUPT HANDLER
// =============================================================================

/// Interrupt handler trait
pub trait InterruptHandler: Send + Sync {
    /// Handle interrupt
    fn handle(&self, source: InterruptSource);

    /// Get handled sources
    fn sources(&self) -> &[InterruptSource];
}

// =============================================================================
// IRQ MANAGER TRAIT
// =============================================================================

/// Trait for interrupt management
pub trait IrqManager {
    /// Enable interrupts for a source
    fn enable(&mut self, source: InterruptSource) -> Result<()>;

    /// Disable interrupts for a source
    fn disable(&mut self, source: InterruptSource) -> Result<()>;

    /// Read interrupt status
    fn status(&self) -> Result<InterruptStatus>;

    /// Acknowledge interrupt
    fn acknowledge(&mut self, source: InterruptSource) -> Result<()>;

    /// Setup MSI-X
    fn setup_msix(&mut self, vectors: u16) -> Result<MsixCapability>;

    /// Register interrupt handler
    fn register_handler(
        &mut self,
        source: InterruptSource,
        handler: alloc::boxed::Box<dyn InterruptHandler>,
    ) -> Result<()>;
}

// =============================================================================
// NVIDIA INTERRUPT REGISTERS
// =============================================================================

/// NVIDIA interrupt register offsets (from PMC base)
pub mod nvidia_irq {
    //! NVIDIA interrupt register constants

    /// Master interrupt status
    pub const INTR_0: u32 = 0x100;
    /// Interrupt enable
    pub const INTR_EN_0: u32 = 0x140;
    /// Software trigger
    pub const INTR_SW: u32 = 0x180;

    /// Interrupt sources (bit positions in INTR_0)
    pub mod sources {
        //! Interrupt source bit positions

        /// FIFO interrupt
        pub const FIFO: u32 = 1 << 8;
        /// Graphics interrupt
        pub const PGRAPH: u32 = 1 << 12;
        /// Copy engine 0
        pub const PCOPY0: u32 = 1 << 5;
        /// Copy engine 1
        pub const PCOPY1: u32 = 1 << 6;
        /// Display interrupt
        pub const PDISP: u32 = 1 << 26;
        /// LTC interrupt
        pub const PLTC: u32 = 1 << 25;
        /// Fault buffer (MMU faults)
        pub const FAULT: u32 = 1 << 31;
    }
}
