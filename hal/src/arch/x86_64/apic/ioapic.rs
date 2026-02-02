//! # I/O APIC
//!
//! This module implements the I/O APIC controller for handling
//! external interrupts from devices.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                     I/O APIC                            │
//! ├─────────────────────────────────────────────────────────┤
//! │  ┌─────────────────────────────────────────────────┐    │
//! │  │           Redirection Table (24+ entries)       │    │
//! │  │  ┌────────┬────────┬────────┬────────┬──────┐   │    │
//! │  │  │ Entry 0│ Entry 1│ Entry 2│  ...   │ N-1  │   │    │
//! │  │  └────────┴────────┴────────┴────────┴──────┘   │    │
//! │  └─────────────────────────────────────────────────┘    │
//! │                                                         │
//! │  Registers:                                             │
//! │  ├─ IOREGSEL (0x00) - Register Select                   │
//! │  ├─ IOWIN    (0x10) - I/O Window                        │
//! │  └─ EOI      (0x40) - End of Interrupt (APIC v2)        │
//! │                                                         │
//! │  Indirect Registers:                                    │
//! │  ├─ ID      (0x00) - I/O APIC ID                        │
//! │  ├─ VER     (0x01) - Version                            │
//! │  ├─ ARB     (0x02) - Arbitration ID                     │
//! │  └─ REDIR   (0x10-0x3F) - Redirection Table             │
//! └─────────────────────────────────────────────────────────┘
//! ```

use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

use super::local::ApicError;

// =============================================================================
// Constants
// =============================================================================

/// Maximum number of I/O APICs
pub const MAX_IOAPICS: usize = 8;

/// Default I/O APIC base address
pub const IOAPIC_BASE_DEFAULT: u64 = 0xFEC0_0000;

/// I/O APIC Register offsets (memory-mapped)
mod mmio_registers {
    /// Register select
    pub const IOREGSEL: u32 = 0x00;
    /// I/O window
    pub const IOWIN: u32 = 0x10;
    /// EOI register (APIC v2+)
    pub const EOI: u32 = 0x40;
}

/// I/O APIC indirect register indices
mod indirect_registers {
    /// I/O APIC ID
    pub const ID: u32 = 0x00;
    /// I/O APIC Version
    pub const VERSION: u32 = 0x01;
    /// Arbitration ID
    pub const ARB: u32 = 0x02;
    /// Redirection table base
    pub const REDIR_BASE: u32 = 0x10;
}

// =============================================================================
// Global State
// =============================================================================

/// I/O APIC instances
static IOAPICS: [IoApicState; MAX_IOAPICS] = [const { IoApicState::new() }; MAX_IOAPICS];

/// Number of registered I/O APICs
static IOAPIC_COUNT: AtomicU8 = AtomicU8::new(0);

/// I/O APIC state
struct IoApicState {
    /// Virtual base address
    base: AtomicU64,
    /// Global System Interrupt base
    gsi_base: AtomicU64,
}

impl IoApicState {
    const fn new() -> Self {
        Self {
            base: AtomicU64::new(0),
            gsi_base: AtomicU64::new(0),
        }
    }
}

// =============================================================================
// Redirection Entry
// =============================================================================

/// I/O APIC Redirection Table Entry (64 bits)
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct RedirectionEntry(u64);

impl RedirectionEntry {
    /// Create a new masked redirection entry
    #[inline]
    pub const fn new_masked() -> Self {
        Self(1 << 16) // Masked
    }

    /// Create a new redirection entry
    #[inline]
    pub const fn new(vector: u8, dest: u8) -> Self {
        Self(vector as u64 | ((dest as u64) << 56))
    }

    /// Get the raw value
    #[inline]
    pub const fn bits(&self) -> u64 {
        self.0
    }

    /// Create from raw bits
    #[inline]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Get the interrupt vector
    #[inline]
    pub const fn vector(&self) -> u8 {
        (self.0 & 0xFF) as u8
    }

    /// Set the interrupt vector
    #[inline]
    pub fn set_vector(&mut self, vector: u8) {
        self.0 = (self.0 & !0xFF) | (vector as u64);
    }

    /// Get delivery mode
    #[inline]
    pub fn delivery_mode(&self) -> DeliveryMode {
        match (self.0 >> 8) & 0b111 {
            0 => DeliveryMode::Fixed,
            1 => DeliveryMode::LowestPriority,
            2 => DeliveryMode::Smi,
            4 => DeliveryMode::Nmi,
            5 => DeliveryMode::Init,
            7 => DeliveryMode::ExtInt,
            _ => DeliveryMode::Fixed,
        }
    }

    /// Set delivery mode
    #[inline]
    pub fn set_delivery_mode(&mut self, mode: DeliveryMode) {
        self.0 = (self.0 & !(0b111 << 8)) | ((mode as u64) << 8);
    }

    /// Get destination mode
    #[inline]
    pub fn destination_mode(&self) -> DestinationMode {
        if self.0 & (1 << 11) != 0 {
            DestinationMode::Logical
        } else {
            DestinationMode::Physical
        }
    }

    /// Set destination mode
    #[inline]
    pub fn set_destination_mode(&mut self, mode: DestinationMode) {
        match mode {
            DestinationMode::Physical => self.0 &= !(1 << 11),
            DestinationMode::Logical => self.0 |= 1 << 11,
        }
    }

    /// Check if delivery status is pending
    #[inline]
    pub const fn is_pending(&self) -> bool {
        self.0 & (1 << 12) != 0
    }

    /// Get pin polarity
    #[inline]
    pub fn polarity(&self) -> Polarity {
        if self.0 & (1 << 13) != 0 {
            Polarity::ActiveLow
        } else {
            Polarity::ActiveHigh
        }
    }

    /// Set pin polarity
    #[inline]
    pub fn set_polarity(&mut self, polarity: Polarity) {
        match polarity {
            Polarity::ActiveHigh => self.0 &= !(1 << 13),
            Polarity::ActiveLow => self.0 |= 1 << 13,
        }
    }

    /// Check if remote IRR is set
    #[inline]
    pub const fn is_remote_irr(&self) -> bool {
        self.0 & (1 << 14) != 0
    }

    /// Get trigger mode
    #[inline]
    pub fn trigger_mode(&self) -> TriggerMode {
        if self.0 & (1 << 15) != 0 {
            TriggerMode::Level
        } else {
            TriggerMode::Edge
        }
    }

    /// Set trigger mode
    #[inline]
    pub fn set_trigger_mode(&mut self, mode: TriggerMode) {
        match mode {
            TriggerMode::Edge => self.0 &= !(1 << 15),
            TriggerMode::Level => self.0 |= 1 << 15,
        }
    }

    /// Check if the entry is masked
    #[inline]
    pub const fn is_masked(&self) -> bool {
        self.0 & (1 << 16) != 0
    }

    /// Set the mask bit
    #[inline]
    pub fn set_masked(&mut self, masked: bool) {
        if masked {
            self.0 |= 1 << 16;
        } else {
            self.0 &= !(1 << 16);
        }
    }

    /// Get the destination APIC ID
    #[inline]
    pub const fn destination(&self) -> u8 {
        (self.0 >> 56) as u8
    }

    /// Set the destination APIC ID
    #[inline]
    pub fn set_destination(&mut self, dest: u8) {
        self.0 = (self.0 & !(0xFF << 56)) | ((dest as u64) << 56);
    }

    /// Create a builder for a redirection entry
    #[inline]
    pub fn builder() -> RedirectionEntryBuilder {
        RedirectionEntryBuilder::new()
    }
}

impl Default for RedirectionEntry {
    fn default() -> Self {
        Self::new_masked()
    }
}

// =============================================================================
// Redirection Entry Builder
// =============================================================================

/// Builder for redirection entries
pub struct RedirectionEntryBuilder {
    entry: RedirectionEntry,
}

impl RedirectionEntryBuilder {
    /// Create a new builder
    #[inline]
    pub const fn new() -> Self {
        Self {
            entry: RedirectionEntry(0),
        }
    }

    /// Set the vector
    #[inline]
    pub fn vector(mut self, vector: u8) -> Self {
        self.entry.set_vector(vector);
        self
    }

    /// Set the delivery mode
    #[inline]
    pub fn delivery_mode(mut self, mode: DeliveryMode) -> Self {
        self.entry.set_delivery_mode(mode);
        self
    }

    /// Set the destination mode
    #[inline]
    pub fn destination_mode(mut self, mode: DestinationMode) -> Self {
        self.entry.set_destination_mode(mode);
        self
    }

    /// Set the polarity
    #[inline]
    pub fn polarity(mut self, polarity: Polarity) -> Self {
        self.entry.set_polarity(polarity);
        self
    }

    /// Set the trigger mode
    #[inline]
    pub fn trigger_mode(mut self, mode: TriggerMode) -> Self {
        self.entry.set_trigger_mode(mode);
        self
    }

    /// Set the mask
    #[inline]
    pub fn masked(mut self, masked: bool) -> Self {
        self.entry.set_masked(masked);
        self
    }

    /// Set the destination
    #[inline]
    pub fn destination(mut self, dest: u8) -> Self {
        self.entry.set_destination(dest);
        self
    }

    /// Build the entry
    #[inline]
    pub fn build(self) -> RedirectionEntry {
        self.entry
    }
}

impl Default for RedirectionEntryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Enums
// =============================================================================

/// Delivery Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeliveryMode {
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
    /// External Interrupt
    ExtInt         = 0b111,
}

/// Destination Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestinationMode {
    /// Physical destination (APIC ID)
    Physical,
    /// Logical destination (cluster/flat)
    Logical,
}

/// Pin Polarity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Active high
    ActiveHigh,
    /// Active low
    ActiveLow,
}

/// Trigger Mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerMode {
    /// Edge-triggered
    Edge,
    /// Level-triggered
    Level,
}

// =============================================================================
// I/O APIC Structure
// =============================================================================

/// I/O APIC controller
pub struct IoApic {
    /// Virtual base address
    base: u64,
    /// I/O APIC ID
    id: u8,
    /// Global System Interrupt base
    gsi_base: u32,
    /// Number of redirection entries
    num_entries: u8,
}

impl IoApic {
    /// Create a new I/O APIC instance
    ///
    /// # Safety
    ///
    /// The base address must be a valid mapping of the I/O APIC registers.
    pub unsafe fn new(base: u64, gsi_base: u32) -> Self {
        let version = Self::read_indirect(base, indirect_registers::VERSION);
        let num_entries = ((version >> 16) & 0xFF) as u8 + 1;
        let id = (Self::read_indirect(base, indirect_registers::ID) >> 24) as u8;

        Self {
            base,
            id,
            gsi_base,
            num_entries,
        }
    }

    /// Read an indirect register
    #[inline]
    unsafe fn read_indirect(base: u64, reg: u32) -> u32 {
        core::ptr::write_volatile((base + mmio_registers::IOREGSEL as u64) as *mut u32, reg);
        core::ptr::read_volatile((base + mmio_registers::IOWIN as u64) as *const u32)
    }

    /// Write an indirect register
    #[inline]
    unsafe fn write_indirect(base: u64, reg: u32, value: u32) {
        core::ptr::write_volatile((base + mmio_registers::IOREGSEL as u64) as *mut u32, reg);
        core::ptr::write_volatile((base + mmio_registers::IOWIN as u64) as *mut u32, value);
    }

    /// Get the I/O APIC ID
    #[inline]
    pub fn id(&self) -> u8 {
        self.id
    }

    /// Get the GSI base
    #[inline]
    pub fn gsi_base(&self) -> u32 {
        self.gsi_base
    }

    /// Get the number of redirection entries
    #[inline]
    pub fn num_entries(&self) -> u8 {
        self.num_entries
    }

    /// Get the version
    #[inline]
    pub fn version(&self) -> u8 {
        unsafe { Self::read_indirect(self.base, indirect_registers::VERSION) as u8 }
    }

    /// Read a redirection entry
    pub fn read_entry(&self, index: u8) -> Option<RedirectionEntry> {
        if index >= self.num_entries {
            return None;
        }

        let reg = indirect_registers::REDIR_BASE + (index as u32 * 2);
        unsafe {
            let low = Self::read_indirect(self.base, reg) as u64;
            let high = Self::read_indirect(self.base, reg + 1) as u64;
            Some(RedirectionEntry::from_bits(low | (high << 32)))
        }
    }

    /// Write a redirection entry
    pub fn write_entry(&self, index: u8, entry: RedirectionEntry) -> Result<(), ApicError> {
        if index >= self.num_entries {
            return Err(ApicError::IoApicError);
        }

        let reg = indirect_registers::REDIR_BASE + (index as u32 * 2);
        let bits = entry.bits();

        unsafe {
            Self::write_indirect(self.base, reg, bits as u32);
            Self::write_indirect(self.base, reg + 1, (bits >> 32) as u32);
        }

        Ok(())
    }

    /// Mask an interrupt
    pub fn mask(&self, index: u8) -> Result<(), ApicError> {
        if let Some(mut entry) = self.read_entry(index) {
            entry.set_masked(true);
            self.write_entry(index, entry)
        } else {
            Err(ApicError::IoApicError)
        }
    }

    /// Unmask an interrupt
    pub fn unmask(&self, index: u8) -> Result<(), ApicError> {
        if let Some(mut entry) = self.read_entry(index) {
            entry.set_masked(false);
            self.write_entry(index, entry)
        } else {
            Err(ApicError::IoApicError)
        }
    }

    /// Check if this I/O APIC handles a GSI
    #[inline]
    pub fn handles_gsi(&self, gsi: u32) -> bool {
        gsi >= self.gsi_base && gsi < self.gsi_base + self.num_entries as u32
    }

    /// Convert GSI to local index
    #[inline]
    pub fn gsi_to_index(&self, gsi: u32) -> Option<u8> {
        if self.handles_gsi(gsi) {
            Some((gsi - self.gsi_base) as u8)
        } else {
            None
        }
    }

    /// Configure an IRQ
    pub fn configure_irq(
        &self,
        index: u8,
        vector: u8,
        dest: u8,
        trigger: TriggerMode,
        polarity: Polarity,
    ) -> Result<(), ApicError> {
        let entry = RedirectionEntry::builder()
            .vector(vector)
            .delivery_mode(DeliveryMode::Fixed)
            .destination_mode(DestinationMode::Physical)
            .trigger_mode(trigger)
            .polarity(polarity)
            .masked(false)
            .destination(dest)
            .build();

        self.write_entry(index, entry)
    }

    /// Mask all interrupts
    pub fn mask_all(&self) {
        for i in 0..self.num_entries {
            let _ = self.mask(i);
        }
    }
}

impl core::fmt::Debug for IoApic {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("IoApic")
            .field("base", &format_args!("{:#x}", self.base))
            .field("id", &self.id)
            .field("gsi_base", &self.gsi_base)
            .field("num_entries", &self.num_entries)
            .field("version", &self.version())
            .finish()
    }
}

// =============================================================================
// Module-level Functions
// =============================================================================

/// Register an I/O APIC
///
/// # Safety
///
/// The base address must be a valid mapping of the I/O APIC registers.
pub unsafe fn register_ioapic(base: u64, gsi_base: u32) -> Result<u8, ApicError> {
    let count = IOAPIC_COUNT.load(Ordering::Acquire);
    if count >= MAX_IOAPICS as u8 {
        return Err(ApicError::IoApicError);
    }

    IOAPICS[count as usize].base.store(base, Ordering::SeqCst);
    IOAPICS[count as usize]
        .gsi_base
        .store(gsi_base as u64, Ordering::SeqCst);
    IOAPIC_COUNT.store(count + 1, Ordering::Release);

    Ok(count)
}

/// Get the number of registered I/O APICs
#[inline]
pub fn ioapic_count() -> u8 {
    IOAPIC_COUNT.load(Ordering::Acquire)
}

/// Get an I/O APIC by index
pub fn get_ioapic(index: u8) -> Option<IoApic> {
    if index >= ioapic_count() {
        return None;
    }

    let base = IOAPICS[index as usize].base.load(Ordering::Acquire);
    let gsi_base = IOAPICS[index as usize].gsi_base.load(Ordering::Acquire) as u32;

    if base == 0 {
        return None;
    }

    Some(unsafe { IoApic::new(base, gsi_base) })
}

/// Find the I/O APIC handling a GSI
pub fn find_ioapic_for_gsi(gsi: u32) -> Option<IoApic> {
    for i in 0..ioapic_count() {
        if let Some(ioapic) = get_ioapic(i) {
            if ioapic.handles_gsi(gsi) {
                return Some(ioapic);
            }
        }
    }
    None
}

/// Configure a GSI
pub fn configure_gsi(
    gsi: u32,
    vector: u8,
    dest: u8,
    trigger: TriggerMode,
    polarity: Polarity,
) -> Result<(), ApicError> {
    if let Some(ioapic) = find_ioapic_for_gsi(gsi) {
        if let Some(index) = ioapic.gsi_to_index(gsi) {
            return ioapic.configure_irq(index, vector, dest, trigger, polarity);
        }
    }
    Err(ApicError::IoApicError)
}

/// Mask a GSI
pub fn mask_gsi(gsi: u32) -> Result<(), ApicError> {
    if let Some(ioapic) = find_ioapic_for_gsi(gsi) {
        if let Some(index) = ioapic.gsi_to_index(gsi) {
            return ioapic.mask(index);
        }
    }
    Err(ApicError::IoApicError)
}

/// Unmask a GSI
pub fn unmask_gsi(gsi: u32) -> Result<(), ApicError> {
    if let Some(ioapic) = find_ioapic_for_gsi(gsi) {
        if let Some(index) = ioapic.gsi_to_index(gsi) {
            return ioapic.unmask(index);
        }
    }
    Err(ApicError::IoApicError)
}
