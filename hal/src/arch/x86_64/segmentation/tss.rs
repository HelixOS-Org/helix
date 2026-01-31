//! # Task State Segment (TSS)
//!
//! 64-bit TSS management for x86_64.
//!
//! ## Overview
//!
//! In 64-bit mode, the TSS is primarily used for:
//!
//! 1. **RSP0-RSP2**: Stack pointers for privilege level changes
//! 2. **IST1-IST7**: Interrupt Stack Table for dedicated interrupt stacks
//! 3. **I/O Permission Bitmap**: (Optional) I/O port access control
//!
//! ## IST (Interrupt Stack Table)
//!
//! The IST provides up to 7 dedicated stacks for interrupts/exceptions.
//! This allows switching to a known-good stack even during catastrophic
//! situations (e.g., double fault on a corrupted stack).
//!
//! ## Memory Layout
//!
//! ```text
//! Offset  Size  Field
//! ──────────────────────────────────────
//! 0x00    4     Reserved
//! 0x04    8     RSP0 (Ring 0 stack)
//! 0x0C    8     RSP1 (Ring 1 stack)
//! 0x14    8     RSP2 (Ring 2 stack)
//! 0x1C    8     Reserved
//! 0x24    8     Reserved
//! 0x2C    8     IST1
//! 0x34    8     IST2
//! 0x3C    8     IST3
//! 0x44    8     IST4
//! 0x4C    8     IST5
//! 0x54    8     IST6
//! 0x5C    8     IST7
//! 0x64    8     Reserved
//! 0x6C    2     Reserved
//! 0x6E    2     I/O Map Base Address
//! ──────────────────────────────────────
//! Total: 0x68 (104) bytes minimum
//! ```

use core::mem::size_of;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Size of TSS structure in bytes (without I/O bitmap)
pub const TSS_SIZE: usize = 104;

/// Default kernel stack size (16 KB)
pub const KERNEL_STACK_SIZE: usize = 16 * 1024;

/// IST stack size for critical handlers (8 KB)
pub const IST_STACK_SIZE: usize = 8 * 1024;

/// IST stack size for double fault (larger for safety) (16 KB)
pub const IST_DOUBLE_FAULT_SIZE: usize = 16 * 1024;

/// Number of IST entries
pub const IST_COUNT: usize = 7;

// =============================================================================
// IST INDEX
// =============================================================================

/// Interrupt Stack Table index
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IstIndex {
    /// IST 1: Double Fault (#DF)
    DoubleFault = 1,
    /// IST 2: Non-Maskable Interrupt (#NMI)
    Nmi = 2,
    /// IST 3: Machine Check (#MC)
    MachineCheck = 3,
    /// IST 4: Debug (#DB)
    Debug = 4,
    /// IST 5: Reserved for future use
    Reserved5 = 5,
    /// IST 6: Reserved for future use
    Reserved6 = 6,
    /// IST 7: Reserved for future use
    Reserved7 = 7,
}

impl IstIndex {
    /// Convert to zero-based array index
    pub const fn as_index(self) -> usize {
        (self as usize) - 1
    }

    /// Convert from 1-based IST number
    pub const fn from_ist_number(num: u8) -> Option<Self> {
        match num {
            1 => Some(Self::DoubleFault),
            2 => Some(Self::Nmi),
            3 => Some(Self::MachineCheck),
            4 => Some(Self::Debug),
            5 => Some(Self::Reserved5),
            6 => Some(Self::Reserved6),
            7 => Some(Self::Reserved7),
            _ => None,
        }
    }

    /// Get required stack size for this IST
    pub const fn stack_size(self) -> usize {
        match self {
            Self::DoubleFault => IST_DOUBLE_FAULT_SIZE,
            _ => IST_STACK_SIZE,
        }
    }
}

// =============================================================================
// IST STACK
// =============================================================================

/// An IST stack allocation
#[derive(Debug)]
pub struct IstStack {
    /// Stack memory (grows downward, so top is at end)
    stack: &'static mut [u8],
}

impl IstStack {
    /// Create a new IST stack from allocated memory
    ///
    /// # Safety
    /// The provided slice must be valid for the lifetime of the kernel
    /// and must not be used for anything else.
    pub unsafe fn from_slice(stack: &'static mut [u8]) -> Self {
        // Ensure 16-byte alignment at top of stack
        Self { stack }
    }

    /// Get the stack top (initial RSP value)
    pub fn top(&self) -> u64 {
        let addr = self.stack.as_ptr() as u64 + self.stack.len() as u64;
        // Align down to 16 bytes for x86_64 ABI
        addr & !0xF
    }

    /// Get the stack size
    pub fn size(&self) -> usize {
        self.stack.len()
    }
}

// =============================================================================
// TASK STATE SEGMENT
// =============================================================================

/// 64-bit Task State Segment
///
/// This structure must be exactly 104 bytes and properly aligned.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Tss {
    /// Reserved
    reserved_0: u32,
    /// Stack pointer for Ring 0
    pub rsp0: u64,
    /// Stack pointer for Ring 1 (unused in most OS designs)
    pub rsp1: u64,
    /// Stack pointer for Ring 2 (unused in most OS designs)
    pub rsp2: u64,
    /// Reserved
    reserved_1: u64,
    /// Interrupt Stack Table 1-7
    pub ist: [u64; 7],
    /// Reserved
    reserved_2: u64,
    /// Reserved
    reserved_3: u16,
    /// I/O Map Base Address (offset from TSS base)
    pub iopb_offset: u16,
}

impl Tss {
    /// Create a new zeroed TSS
    pub const fn new() -> Self {
        Self {
            reserved_0: 0,
            rsp0: 0,
            rsp1: 0,
            rsp2: 0,
            reserved_1: 0,
            ist: [0; 7],
            reserved_2: 0,
            reserved_3: 0,
            // Set IOPB to end of TSS (no I/O bitmap)
            iopb_offset: TSS_SIZE as u16,
        }
    }

    /// Set the kernel stack pointer (RSP0)
    ///
    /// This is the stack used when transitioning from user mode to kernel mode.
    pub fn set_kernel_stack(&mut self, stack_top: u64) {
        self.rsp0 = stack_top;
    }

    /// Set an IST entry
    ///
    /// # Arguments
    /// * `index` - IST index (1-7)
    /// * `stack_top` - Stack top address (must be 16-byte aligned)
    pub fn set_ist(&mut self, index: IstIndex, stack_top: u64) {
        debug_assert!(stack_top & 0xF == 0, "IST stack must be 16-byte aligned");
        self.ist[index.as_index()] = stack_top;
    }

    /// Get an IST entry
    pub fn get_ist(&self, index: IstIndex) -> u64 {
        self.ist[index.as_index()]
    }

    /// Check if an IST entry is set
    pub fn ist_is_set(&self, index: IstIndex) -> bool {
        self.ist[index.as_index()] != 0
    }
}

impl Default for Tss {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Debug for Tss {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Tss")
            .field("rsp0", &format_args!("{:#018x}", self.rsp0))
            .field("ist1", &format_args!("{:#018x}", self.ist[0]))
            .field("ist2", &format_args!("{:#018x}", self.ist[1]))
            .field("ist3", &format_args!("{:#018x}", self.ist[2]))
            .field("ist4", &format_args!("{:#018x}", self.ist[3]))
            .field("iopb_offset", &self.iopb_offset)
            .finish()
    }
}

// Verify TSS size at compile time
static_assertions::const_assert_eq!(size_of::<Tss>(), TSS_SIZE);

// =============================================================================
// TSS GDT ENTRY
// =============================================================================

/// TSS Descriptor entry for GDT (16 bytes in 64-bit mode)
///
/// In 64-bit mode, the TSS descriptor is 16 bytes (2 GDT entries).
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct TssEntry {
    /// Limit bits 0-15
    pub limit_low: u16,
    /// Base bits 0-15
    pub base_low: u16,
    /// Base bits 16-23
    pub base_mid: u8,
    /// Access byte
    pub access: u8,
    /// Limit bits 16-19 + flags
    pub limit_high_flags: u8,
    /// Base bits 24-31
    pub base_high: u8,
    /// Base bits 32-63
    pub base_upper: u32,
    /// Reserved (must be 0)
    pub reserved: u32,
}

impl TssEntry {
    /// Create a null TSS entry
    pub const fn null() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0,
            limit_high_flags: 0,
            base_high: 0,
            base_upper: 0,
            reserved: 0,
        }
    }

    /// Create a TSS entry from TSS address
    ///
    /// # Arguments
    /// * `tss` - Pointer to the TSS
    pub fn from_tss(tss: *const Tss) -> Self {
        let base = tss as u64;
        let limit = (TSS_SIZE - 1) as u64;

        Self {
            limit_low: (limit & 0xFFFF) as u16,
            base_low: (base & 0xFFFF) as u16,
            base_mid: ((base >> 16) & 0xFF) as u8,
            // Type: 64-bit TSS (available) = 0x89
            // P=1 (present), DPL=0, S=0 (system), Type=9 (available TSS)
            access: 0x89,
            // G=0, bits 16-19 of limit
            limit_high_flags: ((limit >> 16) & 0x0F) as u8,
            base_high: ((base >> 24) & 0xFF) as u8,
            base_upper: ((base >> 32) & 0xFFFF_FFFF) as u32,
            reserved: 0,
        }
    }

    /// Mark TSS as busy
    pub fn set_busy(&mut self) {
        self.access |= 0x02;
    }

    /// Clear busy flag
    pub fn clear_busy(&mut self) {
        self.access &= !0x02;
    }

    /// Check if TSS is busy
    pub fn is_busy(&self) -> bool {
        (self.access & 0x02) != 0
    }

    /// Get the base address
    pub fn base(&self) -> u64 {
        (self.base_low as u64)
            | ((self.base_mid as u64) << 16)
            | ((self.base_high as u64) << 24)
            | ((self.base_upper as u64) << 32)
    }

    /// Get the limit
    pub fn limit(&self) -> u32 {
        (self.limit_low as u32) | (((self.limit_high_flags & 0x0F) as u32) << 16)
    }
}

impl core::fmt::Debug for TssEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TssEntry")
            .field("base", &format_args!("{:#018x}", self.base()))
            .field("limit", &format_args!("{:#x}", self.limit()))
            .field("access", &format_args!("{:#04x}", self.access))
            .field("busy", &self.is_busy())
            .finish()
    }
}

// Verify TSS entry size at compile time (must be 16 bytes)
static_assertions::const_assert_eq!(size_of::<TssEntry>(), 16);

// =============================================================================
// TSS LOAD INSTRUCTION
// =============================================================================

/// Load the TSS
///
/// # Safety
/// The selector must reference a valid TSS descriptor in the GDT.
#[inline]
pub unsafe fn load_tss(selector: super::SegmentSelector) {
    core::arch::asm!(
        "ltr {0:x}",
        in(reg) selector.raw(),
        options(nomem, nostack, preserves_flags)
    );
}

/// Get the current TSS selector
#[inline]
pub fn get_tss_selector() -> super::SegmentSelector {
    let value: u16;
    unsafe {
        core::arch::asm!(
            "str {0:x}",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    super::SegmentSelector::gdt(value >> 3, super::Rpl::Ring0)
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tss_size() {
        assert_eq!(size_of::<Tss>(), TSS_SIZE);
    }

    #[test]
    fn test_tss_entry_size() {
        assert_eq!(size_of::<TssEntry>(), 16);
    }

    #[test]
    fn test_tss_new() {
        let tss = Tss::new();
        assert_eq!(tss.rsp0, 0);
        assert_eq!(tss.iopb_offset, TSS_SIZE as u16);
    }

    #[test]
    fn test_ist_index() {
        assert_eq!(IstIndex::DoubleFault.as_index(), 0);
        assert_eq!(IstIndex::Nmi.as_index(), 1);
        assert_eq!(IstIndex::MachineCheck.as_index(), 2);
        assert_eq!(IstIndex::Debug.as_index(), 3);
    }

    #[test]
    fn test_tss_entry_creation() {
        let tss = Tss::new();
        let entry = TssEntry::from_tss(&tss);

        assert_eq!(entry.base(), &tss as *const Tss as u64);
        assert_eq!(entry.limit(), TSS_SIZE as u32 - 1);
        assert!(!entry.is_busy());
    }
}
