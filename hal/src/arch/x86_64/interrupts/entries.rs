//! # IDT Entry Structures
//!
//! This module defines the 64-bit IDT gate descriptor format and options.
//!
//! ## 64-bit IDT Gate Descriptor Format (16 bytes)
//!
//! ```text
//! Bits      Field
//! ───────────────────────────────────────────────────
//! 127:96    Offset [63:32]
//! 95:64     Reserved (must be 0)
//! 63:48     Offset [31:16]
//! 47        Present (P)
//! 46:45     DPL (Descriptor Privilege Level)
//! 44        Zero (0)
//! 43:40     Gate Type (0xE = Interrupt, 0xF = Trap)
//! 39:37     Reserved (must be 0)
//! 36:32     IST (Interrupt Stack Table index, 0-7)
//! 31:16     Segment Selector
//! 15:0      Offset [15:0]
//! ```

use core::fmt;

// =============================================================================
// Gate Types
// =============================================================================

/// IDT Gate Type
///
/// In 64-bit mode, only Interrupt and Trap gates are valid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GateType {
    /// Interrupt Gate (clears IF flag on entry)
    ///
    /// Use for hardware interrupts and most exceptions.
    Interrupt = 0xE,

    /// Trap Gate (does not clear IF flag)
    ///
    /// Use for software interrupts and breakpoints.
    Trap      = 0xF,
}

impl GateType {
    /// Convert from raw value
    #[inline]
    pub const fn from_bits(bits: u8) -> Option<Self> {
        match bits & 0xF {
            0xE => Some(GateType::Interrupt),
            0xF => Some(GateType::Trap),
            _ => None,
        }
    }
}

// =============================================================================
// Descriptor Privilege Level
// =============================================================================

/// Descriptor Privilege Level (DPL)
///
/// Determines the minimum privilege level required to invoke
/// this interrupt via the INT instruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Dpl {
    /// Ring 0 (kernel mode) - only kernel can invoke
    Ring0 = 0,
    /// Ring 1 (unused in most OS designs)
    Ring1 = 1,
    /// Ring 2 (unused in most OS designs)
    Ring2 = 2,
    /// Ring 3 (user mode) - user code can invoke
    Ring3 = 3,
}

impl Dpl {
    /// Convert from raw value
    #[inline]
    pub const fn from_bits(bits: u8) -> Self {
        match bits & 0x3 {
            0 => Dpl::Ring0,
            1 => Dpl::Ring1,
            2 => Dpl::Ring2,
            _ => Dpl::Ring3,
        }
    }
}

// =============================================================================
// Gate Options
// =============================================================================

/// IDT Gate Options
///
/// Combines the type attributes for an IDT entry.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct GateOptions(u16);

impl GateOptions {
    /// Bit positions
    const IST_MASK: u16 = 0x07; // bits 0-2
    const TYPE_SHIFT: u8 = 8; // bits 8-11
    const TYPE_MASK: u16 = 0x0F00;
    const DPL_SHIFT: u8 = 13; // bits 13-14
    const DPL_MASK: u16 = 0x6000;
    const PRESENT_BIT: u16 = 1 << 15; // bit 15

    /// Create empty (not present) gate options
    #[inline]
    pub const fn empty() -> Self {
        Self(0)
    }

    /// Create a new interrupt gate with Ring0 DPL
    #[inline]
    pub const fn new_interrupt() -> Self {
        Self(Self::PRESENT_BIT | ((GateType::Interrupt as u16) << Self::TYPE_SHIFT))
    }

    /// Create a new trap gate with Ring0 DPL
    #[inline]
    pub const fn new_trap() -> Self {
        Self(Self::PRESENT_BIT | ((GateType::Trap as u16) << Self::TYPE_SHIFT))
    }

    /// Create from gate type with Ring0 DPL
    #[inline]
    pub const fn from_type(gate_type: GateType) -> Self {
        Self(Self::PRESENT_BIT | ((gate_type as u16) << Self::TYPE_SHIFT))
    }

    /// Set the Interrupt Stack Table index (1-7, 0 = no IST)
    #[inline]
    pub const fn with_ist(self, ist: u8) -> Self {
        debug_assert!(ist <= 7);
        Self((self.0 & !Self::IST_MASK) | (ist as u16 & Self::IST_MASK))
    }

    /// Set the DPL
    #[inline]
    pub const fn with_dpl(self, dpl: Dpl) -> Self {
        Self((self.0 & !Self::DPL_MASK) | ((dpl as u16) << Self::DPL_SHIFT))
    }

    /// Set present bit
    #[inline]
    pub const fn with_present(self, present: bool) -> Self {
        if present {
            Self(self.0 | Self::PRESENT_BIT)
        } else {
            Self(self.0 & !Self::PRESENT_BIT)
        }
    }

    /// Get IST index
    #[inline]
    pub const fn ist(&self) -> u8 {
        (self.0 & Self::IST_MASK) as u8
    }

    /// Get gate type
    #[inline]
    pub const fn gate_type(&self) -> Option<GateType> {
        GateType::from_bits(((self.0 & Self::TYPE_MASK) >> Self::TYPE_SHIFT) as u8)
    }

    /// Get DPL
    #[inline]
    pub const fn dpl(&self) -> Dpl {
        Dpl::from_bits(((self.0 & Self::DPL_MASK) >> Self::DPL_SHIFT) as u8)
    }

    /// Check if present
    #[inline]
    pub const fn is_present(&self) -> bool {
        self.0 & Self::PRESENT_BIT != 0
    }

    /// Get raw value
    #[inline]
    pub const fn bits(&self) -> u16 {
        self.0
    }
}

impl fmt::Debug for GateOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GateOptions")
            .field("present", &self.is_present())
            .field("dpl", &self.dpl())
            .field("type", &self.gate_type())
            .field("ist", &self.ist())
            .finish()
    }
}

// =============================================================================
// IDT Entry (Gate Descriptor)
// =============================================================================

/// 64-bit IDT Entry (Gate Descriptor)
///
/// This is the hardware-defined structure for an IDT entry in 64-bit mode.
/// Each entry is 16 bytes (128 bits).
#[derive(Clone, Copy)]
#[repr(C, packed)]
pub struct IdtEntry {
    /// Offset bits 0-15
    offset_low: u16,
    /// Segment selector (should be kernel code segment)
    selector: u16,
    /// Gate options (IST, type, DPL, present)
    options: GateOptions,
    /// Offset bits 16-31
    offset_mid: u16,
    /// Offset bits 32-63
    offset_high: u32,
    /// Reserved, must be zero
    reserved: u32,
}

impl IdtEntry {
    /// Create an empty (not present) IDT entry
    #[inline]
    pub const fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            options: GateOptions::empty(),
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Create a new IDT entry
    ///
    /// # Arguments
    ///
    /// * `handler` - Address of the interrupt handler function
    /// * `selector` - Segment selector (typically kernel code segment)
    /// * `options` - Gate options (type, DPL, IST, present)
    #[inline]
    pub const fn new(handler: u64, selector: u16, options: GateOptions) -> Self {
        Self {
            offset_low: handler as u16,
            selector,
            options,
            offset_mid: (handler >> 16) as u16,
            offset_high: (handler >> 32) as u32,
            reserved: 0,
        }
    }

    /// Create an interrupt gate entry
    ///
    /// Interrupt gates clear the IF flag on entry.
    #[inline]
    pub const fn interrupt(handler: u64, selector: u16) -> Self {
        Self::new(handler, selector, GateOptions::new_interrupt())
    }

    /// Create a trap gate entry
    ///
    /// Trap gates do not clear the IF flag.
    #[inline]
    pub const fn trap(handler: u64, selector: u16) -> Self {
        Self::new(handler, selector, GateOptions::new_trap())
    }

    /// Create an interrupt gate with IST
    #[inline]
    pub const fn interrupt_with_ist(handler: u64, selector: u16, ist: u8) -> Self {
        Self::new(
            handler,
            selector,
            GateOptions::new_interrupt().with_ist(ist),
        )
    }

    /// Create a user-callable gate (DPL=3)
    ///
    /// This allows user-space code to invoke this interrupt via INT instruction.
    #[inline]
    pub const fn user_callable(handler: u64, selector: u16, gate_type: GateType) -> Self {
        Self::new(
            handler,
            selector,
            GateOptions::from_type(gate_type).with_dpl(Dpl::Ring3),
        )
    }

    /// Set the handler address
    #[inline]
    pub fn set_handler(&mut self, handler: u64) {
        self.offset_low = handler as u16;
        self.offset_mid = (handler >> 16) as u16;
        self.offset_high = (handler >> 32) as u32;
    }

    /// Get the handler address
    #[inline]
    pub const fn handler(&self) -> u64 {
        (self.offset_low as u64)
            | ((self.offset_mid as u64) << 16)
            | ((self.offset_high as u64) << 32)
    }

    /// Get the segment selector
    #[inline]
    pub const fn selector(&self) -> u16 {
        self.selector
    }

    /// Get the gate options
    #[inline]
    pub const fn options(&self) -> GateOptions {
        self.options
    }

    /// Set the segment selector
    #[inline]
    pub fn set_selector(&mut self, selector: u16) {
        self.selector = selector;
    }

    /// Set the gate options
    #[inline]
    pub fn set_options(&mut self, options: GateOptions) {
        self.options = options;
    }

    /// Check if this entry is present
    #[inline]
    pub const fn is_present(&self) -> bool {
        // Copy to avoid unaligned reference in packed struct
        let options = self.options;
        options.is_present()
    }

    /// Set the IST index
    #[inline]
    pub fn set_ist(&mut self, ist: u8) {
        self.options = self.options.with_ist(ist);
    }
}

impl Default for IdtEntry {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Debug for IdtEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy fields to avoid unaligned references in packed struct
        let handler = self.handler();
        let selector = { self.selector };
        let options = { self.options };
        f.debug_struct("IdtEntry")
            .field("handler", &format_args!("{:#018x}", handler))
            .field("selector", &format_args!("{:#06x}", selector))
            .field("options", &options)
            .finish()
    }
}

// =============================================================================
// Compile-time Assertions
// =============================================================================

const _: () = {
    use core::mem::size_of;

    // IDT entry must be exactly 16 bytes
    assert!(size_of::<IdtEntry>() == 16);

    // GateOptions must be 2 bytes
    assert!(size_of::<GateOptions>() == 2);
};

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_options() {
        let opts = GateOptions::new_interrupt()
            .with_ist(1)
            .with_dpl(Dpl::Ring3);

        assert!(opts.is_present());
        assert_eq!(opts.gate_type(), Some(GateType::Interrupt));
        assert_eq!(opts.ist(), 1);
        assert_eq!(opts.dpl(), Dpl::Ring3);
    }

    #[test]
    fn test_idt_entry() {
        let handler = 0xFFFF_8000_0000_1234u64;
        let entry = IdtEntry::interrupt(handler, 0x08);

        assert_eq!(entry.handler(), handler);
        assert_eq!(entry.selector(), 0x08);
        assert!(entry.is_present());
    }

    #[test]
    fn test_idt_entry_with_ist() {
        let entry = IdtEntry::interrupt_with_ist(0x1000, 0x08, 1);
        assert_eq!(entry.options().ist(), 1);
    }
}
