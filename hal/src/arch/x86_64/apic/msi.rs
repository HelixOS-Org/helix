//! # Message Signaled Interrupts (MSI/MSI-X)
//!
//! This module provides support for Message Signaled Interrupts,
//! which are more efficient than legacy pin-based interrupts.
//!
//! ## MSI vs Legacy Interrupts
//!
//! ```text
//! Legacy Interrupts:
//! ┌────────┐     ┌─────────┐     ┌──────────┐     ┌───────────┐
//! │ Device ├────►│ PIC/    ├────►│ I/O APIC ├────►│ Local APIC│
//! └────────┘     │ IOAPIC  │     └──────────┘     └───────────┘
//!                └─────────┘
//!
//! MSI:
//! ┌────────┐                                      ┌───────────┐
//! │ Device ├─────────────────────────────────────►│ Local APIC│
//! └────────┘                                      └───────────┘
//!     (Memory Write to APIC address)
//! ```
//!
//! ## MSI Message Format
//!
//! - **Address**: 0xFEE0_0000 + destination info
//! - **Data**: Vector + delivery mode + trigger mode
//!
//! ## MSI-X Enhancements
//!
//! - More vectors (up to 2048 per device)
//! - Per-vector masking
//! - Dedicated table in device memory

use core::fmt;

// =============================================================================
// Constants
// =============================================================================

/// MSI address base (same as LAPIC address)
pub const MSI_ADDRESS_BASE: u64 = 0xFEE0_0000;

/// MSI address mask (bits that can be set)
pub const MSI_ADDRESS_MASK: u64 = 0xFFFF_FFFF;

/// MSI data mask
pub const MSI_DATA_MASK: u32 = 0xFFFF;

// =============================================================================
// MSI Address Format
// =============================================================================

/// MSI Address register format
///
/// Bits:
/// - 31-20: 0xFEE (constant)
/// - 19-12: Destination APIC ID
/// - 11-4: Reserved
/// - 3: Redirection hint (RH)
/// - 2: Destination mode (DM)
/// - 1-0: Reserved
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MsiAddress(u64);

impl MsiAddress {
    /// Create a new MSI address for physical destination mode
    #[inline]
    pub const fn new_physical(dest_apic_id: u8) -> Self {
        let addr = MSI_ADDRESS_BASE | ((dest_apic_id as u64) << 12);
        Self(addr)
    }

    /// Create a new MSI address for logical destination mode
    #[inline]
    pub const fn new_logical(dest_id: u8) -> Self {
        let addr = MSI_ADDRESS_BASE
            | ((dest_id as u64) << 12)
            | (1 << 2)  // Destination mode = logical
            | (1 << 3); // Redirection hint = 1 for logical
        Self(addr)
    }

    /// Create from raw value
    #[inline]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }

    /// Get the raw value
    #[inline]
    pub const fn raw(&self) -> u64 {
        self.0
    }

    /// Get the low 32 bits (for 32-bit MSI)
    #[inline]
    pub const fn low(&self) -> u32 {
        self.0 as u32
    }

    /// Get the high 32 bits (for 64-bit MSI)
    #[inline]
    pub const fn high(&self) -> u32 {
        (self.0 >> 32) as u32
    }

    /// Get the destination APIC ID
    #[inline]
    pub const fn dest_id(&self) -> u8 {
        ((self.0 >> 12) & 0xFF) as u8
    }

    /// Check if using logical destination mode
    #[inline]
    pub const fn is_logical(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    /// Check if redirection hint is set
    #[inline]
    pub const fn redirection_hint(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    /// Create address with redirection hint (for lowest priority)
    #[inline]
    pub const fn with_redirection_hint(mut self) -> Self {
        self.0 |= 1 << 3;
        self
    }
}

impl fmt::Debug for MsiAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MsiAddress")
            .field("raw", &format_args!("{:#x}", self.0))
            .field("dest_id", &self.dest_id())
            .field("logical", &self.is_logical())
            .field("rh", &self.redirection_hint())
            .finish()
    }
}

// =============================================================================
// MSI Data Format
// =============================================================================

/// MSI Data register format
///
/// Bits:
/// - 15: Trigger mode (0=edge, 1=level)
/// - 14: Level (0=deassert, 1=assert) - only for level-triggered
/// - 13-11: Reserved
/// - 10-8: Delivery mode
/// - 7-0: Vector
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct MsiData(u32);

impl MsiData {
    /// Create new MSI data with edge trigger
    #[inline]
    pub const fn new_edge(vector: u8, delivery_mode: MsiDeliveryMode) -> Self {
        Self((vector as u32) | ((delivery_mode as u32) << 8))
    }

    /// Create new MSI data with level trigger
    #[inline]
    pub const fn new_level(vector: u8, delivery_mode: MsiDeliveryMode, assert: bool) -> Self {
        let mut data = (vector as u32) | ((delivery_mode as u32) << 8) | (1 << 15);
        if assert {
            data |= 1 << 14;
        }
        Self(data)
    }

    /// Create from raw value
    #[inline]
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }

    /// Get the raw value
    #[inline]
    pub const fn raw(&self) -> u32 {
        self.0
    }

    /// Get the vector
    #[inline]
    pub const fn vector(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// Get the delivery mode
    #[inline]
    pub fn delivery_mode(&self) -> MsiDeliveryMode {
        match (self.0 >> 8) & 0b111 {
            0 => MsiDeliveryMode::Fixed,
            1 => MsiDeliveryMode::LowestPriority,
            2 => MsiDeliveryMode::Smi,
            4 => MsiDeliveryMode::Nmi,
            5 => MsiDeliveryMode::Init,
            7 => MsiDeliveryMode::ExtInt,
            _ => MsiDeliveryMode::Fixed,
        }
    }

    /// Check if level-triggered
    #[inline]
    pub const fn is_level(&self) -> bool {
        self.0 & (1 << 15) != 0
    }

    /// Check if level is asserted
    #[inline]
    pub const fn is_asserted(&self) -> bool {
        self.0 & (1 << 14) != 0
    }
}

impl fmt::Debug for MsiData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MsiData")
            .field("raw", &format_args!("{:#x}", self.0))
            .field("vector", &format_args!("{:#x}", self.vector()))
            .field("delivery", &self.delivery_mode())
            .field("level_triggered", &self.is_level())
            .finish()
    }
}

// =============================================================================
// MSI Delivery Mode
// =============================================================================

/// MSI Delivery Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsiDeliveryMode {
    /// Fixed delivery to specific APIC
    Fixed          = 0b000,
    /// Lowest priority delivery
    LowestPriority = 0b001,
    /// System Management Interrupt
    Smi            = 0b010,
    /// Non-Maskable Interrupt
    Nmi            = 0b100,
    /// INIT signal
    Init           = 0b101,
    /// External interrupt
    ExtInt         = 0b111,
}

// =============================================================================
// MSI Message
// =============================================================================

/// Complete MSI message (address + data)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MsiMessage {
    /// Message address
    pub address: MsiAddress,
    /// Message data
    pub data: MsiData,
}

impl MsiMessage {
    /// Create a new MSI message for fixed delivery
    #[inline]
    pub const fn new_fixed(dest_apic_id: u8, vector: u8) -> Self {
        Self {
            address: MsiAddress::new_physical(dest_apic_id),
            data: MsiData::new_edge(vector, MsiDeliveryMode::Fixed),
        }
    }

    /// Create a new MSI message for lowest priority delivery
    #[inline]
    pub const fn new_lowest_priority(dest_id: u8, vector: u8) -> Self {
        Self {
            address: MsiAddress::new_logical(dest_id).with_redirection_hint(),
            data: MsiData::new_edge(vector, MsiDeliveryMode::LowestPriority),
        }
    }

    /// Create a builder for more complex configurations
    #[inline]
    pub fn builder() -> MsiMessageBuilder {
        MsiMessageBuilder::new()
    }
}

// =============================================================================
// MSI Message Builder
// =============================================================================

/// Builder for MSI messages
pub struct MsiMessageBuilder {
    dest_id: u8,
    vector: u8,
    logical: bool,
    delivery_mode: MsiDeliveryMode,
    level_triggered: bool,
    assert: bool,
    redirection_hint: bool,
}

impl MsiMessageBuilder {
    /// Create a new builder with defaults
    #[inline]
    pub const fn new() -> Self {
        Self {
            dest_id: 0,
            vector: 0,
            logical: false,
            delivery_mode: MsiDeliveryMode::Fixed,
            level_triggered: false,
            assert: true,
            redirection_hint: false,
        }
    }

    /// Set destination APIC ID
    #[inline]
    pub const fn dest_id(mut self, id: u8) -> Self {
        self.dest_id = id;
        self
    }

    /// Set vector
    #[inline]
    pub const fn vector(mut self, vector: u8) -> Self {
        self.vector = vector;
        self
    }

    /// Use logical destination mode
    #[inline]
    pub const fn logical(mut self) -> Self {
        self.logical = true;
        self
    }

    /// Use physical destination mode
    #[inline]
    pub const fn physical(mut self) -> Self {
        self.logical = false;
        self
    }

    /// Set delivery mode
    #[inline]
    pub const fn delivery_mode(mut self, mode: MsiDeliveryMode) -> Self {
        self.delivery_mode = mode;
        self
    }

    /// Use level-triggered mode
    #[inline]
    pub const fn level_triggered(mut self, assert: bool) -> Self {
        self.level_triggered = true;
        self.assert = assert;
        self
    }

    /// Use edge-triggered mode
    #[inline]
    pub const fn edge_triggered(mut self) -> Self {
        self.level_triggered = false;
        self
    }

    /// Enable redirection hint
    #[inline]
    pub const fn with_redirection_hint(mut self) -> Self {
        self.redirection_hint = true;
        self
    }

    /// Build the MSI message
    #[inline]
    pub fn build(self) -> MsiMessage {
        let mut address = if self.logical {
            MsiAddress::new_logical(self.dest_id)
        } else {
            MsiAddress::new_physical(self.dest_id)
        };

        if self.redirection_hint {
            address = address.with_redirection_hint();
        }

        let data = if self.level_triggered {
            MsiData::new_level(self.vector, self.delivery_mode, self.assert)
        } else {
            MsiData::new_edge(self.vector, self.delivery_mode)
        };

        MsiMessage { address, data }
    }
}

impl Default for MsiMessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MSI-X Table Entry
// =============================================================================

/// MSI-X Table Entry (16 bytes)
///
/// Layout:
/// - Bytes 0-7: Message Address (64-bit)
/// - Bytes 8-11: Message Data (32-bit)
/// - Bytes 12-15: Vector Control (32-bit)
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy)]
pub struct MsixTableEntry {
    /// Message address (low 32 bits)
    pub address_low: u32,
    /// Message address (high 32 bits)
    pub address_high: u32,
    /// Message data
    pub data: u32,
    /// Vector control (bit 0 = mask)
    pub vector_control: u32,
}

impl MsixTableEntry {
    /// Create a new entry (masked by default)
    pub const fn new() -> Self {
        Self {
            address_low: 0,
            address_high: 0,
            data: 0,
            vector_control: 1, // Masked
        }
    }

    /// Configure the entry with an MSI message
    pub fn configure(&mut self, msg: MsiMessage) {
        self.address_low = msg.address.low();
        self.address_high = msg.address.high();
        self.data = msg.data.raw();
    }

    /// Check if the entry is masked
    #[inline]
    pub const fn is_masked(&self) -> bool {
        self.vector_control & 1 != 0
    }

    /// Mask the entry
    #[inline]
    pub fn mask(&mut self) {
        self.vector_control |= 1;
    }

    /// Unmask the entry
    #[inline]
    pub fn unmask(&mut self) {
        self.vector_control &= !1;
    }

    /// Get the configured message
    pub fn message(&self) -> MsiMessage {
        let addr = ((self.address_high as u64) << 32) | (self.address_low as u64);
        MsiMessage {
            address: MsiAddress::from_raw(addr),
            data: MsiData::from_raw(self.data),
        }
    }
}

impl Default for MsixTableEntry {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// MSI-X Pending Bit Array Entry
// =============================================================================

/// MSI-X Pending Bit Array
///
/// Each bit corresponds to one MSI-X vector.
/// If a vector is masked when an interrupt arrives, the pending bit is set.
#[repr(transparent)]
pub struct MsixPba([u64]);

impl MsixPba {
    /// Check if a vector is pending
    #[inline]
    pub fn is_pending(&self, vector: usize) -> bool {
        let qword = vector / 64;
        let bit = vector % 64;
        if qword < self.0.len() {
            self.0[qword] & (1 << bit) != 0
        } else {
            false
        }
    }
}

// =============================================================================
// MSI Capability Helper
// =============================================================================

/// MSI Capability Register offsets (in PCI config space)
pub mod msi_cap {
    /// Message Control
    pub const MSG_CTRL: u8 = 0x02;
    /// Message Address (lower 32 bits)
    pub const MSG_ADDR_LO: u8 = 0x04;
    /// Message Address (upper 32 bits) - only if 64-bit capable
    pub const MSG_ADDR_HI: u8 = 0x08;
    /// Message Data register offset for 32-bit MSI capability
    pub const MSG_DATA_32: u8 = 0x08;
    /// Message Data register offset for 64-bit MSI capability
    pub const MSG_DATA_64: u8 = 0x0C;
    /// Mask Bits register offset for 32-bit MSI (if per-vector masking supported)
    pub const MASK_BITS_32: u8 = 0x0C;
    /// Mask Bits register offset for 64-bit MSI (if per-vector masking supported)
    pub const MASK_BITS_64: u8 = 0x10;
    /// Pending Bits register offset for 32-bit MSI
    pub const PEND_BITS_32: u8 = 0x10;
    /// Pending Bits register offset for 64-bit MSI
    pub const PEND_BITS_64: u8 = 0x14;
}

/// MSI-X Capability Register offsets (in PCI config space)
pub mod msix_cap {
    /// Message Control
    pub const MSG_CTRL: u8 = 0x02;
    /// Table Offset and BIR
    pub const TABLE: u8 = 0x04;
    /// PBA Offset and BIR
    pub const PBA: u8 = 0x08;
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Calculate the number of MSI vectors needed for a table size
#[inline]
pub const fn vectors_needed(table_size: u16) -> u8 {
    // MSI supports 1, 2, 4, 8, 16, or 32 vectors
    match table_size {
        0 => 0,
        1 => 1,
        2 => 2,
        3..=4 => 4,
        5..=8 => 8,
        9..=16 => 16,
        _ => 32,
    }
}

/// Check if a vector count is valid for MSI (must be power of 2, max 32)
#[inline]
pub const fn is_valid_msi_count(count: u8) -> bool {
    matches!(count, 1 | 2 | 4 | 8 | 16 | 32)
}

/// Compose an MSI message for a simple fixed interrupt
#[inline]
pub const fn compose_simple_msi(dest_apic_id: u8, vector: u8) -> (u64, u32) {
    let msg = MsiMessage::new_fixed(dest_apic_id, vector);
    (msg.address.raw(), msg.data.raw())
}
