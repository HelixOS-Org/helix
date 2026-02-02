//! # Segment Selectors
//!
//! Type-safe segment selector definitions.

use core::fmt;

// =============================================================================
// PRIVILEGE LEVELS
// =============================================================================

/// Requested Privilege Level (Ring)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Rpl {
    /// Ring 0 (Kernel mode)
    Ring0 = 0,
    /// Ring 1 (Unused in modern OS)
    Ring1 = 1,
    /// Ring 2 (Unused in modern OS)
    Ring2 = 2,
    /// Ring 3 (User mode)
    Ring3 = 3,
}

impl Rpl {
    /// Create from raw value
    pub const fn from_raw(value: u8) -> Self {
        match value & 3 {
            0 => Rpl::Ring0,
            1 => Rpl::Ring1,
            2 => Rpl::Ring2,
            _ => Rpl::Ring3,
        }
    }

    /// Check if this is kernel mode
    pub const fn is_kernel(self) -> bool {
        matches!(self, Rpl::Ring0)
    }

    /// Check if this is user mode
    pub const fn is_user(self) -> bool {
        matches!(self, Rpl::Ring3)
    }
}

// =============================================================================
// SEGMENT SELECTOR
// =============================================================================

/// Segment Selector
///
/// A 16-bit value that indexes into the GDT or LDT.
///
/// ```text
/// ┌────────────────────────────────────────────────────┐
/// │ 15                              3   2   1   0      │
/// │ ┌────────────────────────────┬───┬───────────┐     │
/// │ │         Index              │TI │    RPL    │     │
/// │ └────────────────────────────┴───┴───────────┘     │
/// │   Index: Descriptor table index (0-8191)           │
/// │   TI: Table Indicator (0=GDT, 1=LDT)               │
/// │   RPL: Requested Privilege Level (0-3)             │
/// └────────────────────────────────────────────────────┘
/// ```
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SegmentSelector(u16);

impl SegmentSelector {
    /// Create a null selector
    pub const NULL: Self = Self(0);

    /// Create a new segment selector
    ///
    /// # Arguments
    /// * `index` - GDT/LDT index
    /// * `rpl` - Requested privilege level
    pub const fn new(index: u16, rpl: Rpl) -> Self {
        Self((index << 3) | (rpl as u16))
    }

    /// Create selector for GDT entry
    pub const fn gdt(index: u16, rpl: Rpl) -> Self {
        Self::new(index, rpl)
    }

    /// Create selector for LDT entry
    pub const fn ldt(index: u16, rpl: Rpl) -> Self {
        Self((index << 3) | 0x04 | (rpl as u16))
    }

    /// Get the raw selector value
    pub const fn raw(self) -> u16 {
        self.0
    }

    /// Get the table index
    pub const fn index(self) -> u16 {
        self.0 >> 3
    }

    /// Check if this references the LDT
    pub const fn is_ldt(self) -> bool {
        (self.0 & 0x04) != 0
    }

    /// Get the requested privilege level
    pub const fn rpl(self) -> Rpl {
        Rpl::from_raw((self.0 & 3) as u8)
    }

    /// Set the RPL and return new selector
    pub const fn with_rpl(self, rpl: Rpl) -> Self {
        Self((self.0 & !3) | (rpl as u16))
    }

    /// Check if this is the null selector
    pub const fn is_null(self) -> bool {
        self.0 == 0
    }
}

impl fmt::Debug for SegmentSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SegmentSelector")
            .field("raw", &format_args!("{:#06x}", self.0))
            .field("index", &self.index())
            .field("rpl", &self.rpl())
            .field("ldt", &self.is_ldt())
            .finish()
    }
}

impl fmt::Display for SegmentSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#06x}", self.0)
    }
}

// =============================================================================
// STANDARD SELECTORS
// =============================================================================

/// Null segment selector (index 0)
pub const NULL_SELECTOR: SegmentSelector = SegmentSelector::NULL;

/// Kernel Code Segment selector (index 1, RPL 0)
pub const KERNEL_CS: SegmentSelector = SegmentSelector::new(1, Rpl::Ring0);

/// Kernel Data Segment selector (index 2, RPL 0)
pub const KERNEL_DS: SegmentSelector = SegmentSelector::new(2, Rpl::Ring0);

/// User Data Segment selector (index 3, RPL 3)
///
/// Note: User data comes BEFORE user code for SYSRET compatibility.
/// SYSRET expects: STAR[63:48] + 0 = SS, STAR[63:48] + 16 = CS
pub const USER_DS: SegmentSelector = SegmentSelector::new(3, Rpl::Ring3);

/// User Code Segment selector (index 4, RPL 3)
pub const USER_CS: SegmentSelector = SegmentSelector::new(4, Rpl::Ring3);

/// TSS Segment selector (index 5, RPL 0)
///
/// Note: TSS descriptor is 16 bytes (2 entries) in 64-bit mode
pub const TSS_SELECTOR: SegmentSelector = SegmentSelector::new(5, Rpl::Ring0);

// =============================================================================
// SELECTOR OPERATIONS
// =============================================================================

/// Load CS register (requires far jump/return)
///
/// # Safety
/// The selector must reference a valid 64-bit code segment.
#[inline]
pub unsafe fn load_cs(selector: SegmentSelector) {
    // In 64-bit mode, we need to do a far return to change CS
    core::arch::asm!(
        "push {sel}",
        "lea {tmp}, [rip + 1f]",
        "push {tmp}",
        "retfq",
        "1:",
        sel = in(reg) selector.raw() as u64,
        tmp = lateout(reg) _,
        options(preserves_flags)
    );
}

/// Load DS register
///
/// # Safety
/// The selector must reference a valid data segment or be null.
#[inline]
pub unsafe fn load_ds(selector: SegmentSelector) {
    core::arch::asm!(
        "mov ds, {0:x}",
        in(reg) selector.raw(),
        options(nomem, nostack, preserves_flags)
    );
}

/// Load ES register
///
/// # Safety
/// The selector must reference a valid data segment or be null.
#[inline]
pub unsafe fn load_es(selector: SegmentSelector) {
    core::arch::asm!(
        "mov es, {0:x}",
        in(reg) selector.raw(),
        options(nomem, nostack, preserves_flags)
    );
}

/// Load FS register
///
/// # Safety
/// The selector must reference a valid data segment or be null.
#[inline]
pub unsafe fn load_fs(selector: SegmentSelector) {
    core::arch::asm!(
        "mov fs, {0:x}",
        in(reg) selector.raw(),
        options(nomem, nostack, preserves_flags)
    );
}

/// Load GS register
///
/// # Safety
/// The selector must reference a valid data segment or be null.
#[inline]
pub unsafe fn load_gs(selector: SegmentSelector) {
    core::arch::asm!(
        "mov gs, {0:x}",
        in(reg) selector.raw(),
        options(nomem, nostack, preserves_flags)
    );
}

/// Load SS register
///
/// # Safety
/// The selector must reference a valid data segment.
#[inline]
pub unsafe fn load_ss(selector: SegmentSelector) {
    core::arch::asm!(
        "mov ss, {0:x}",
        in(reg) selector.raw(),
        options(nomem, nostack, preserves_flags)
    );
}

/// Get current CS register value
#[inline]
pub fn get_cs() -> SegmentSelector {
    let value: u16;
    unsafe {
        core::arch::asm!(
            "mov {0:x}, cs",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    SegmentSelector(value)
}

/// Get current SS register value
#[inline]
pub fn get_ss() -> SegmentSelector {
    let value: u16;
    unsafe {
        core::arch::asm!(
            "mov {0:x}, ss",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    SegmentSelector(value)
}

/// Get current DS register value
#[inline]
pub fn get_ds() -> SegmentSelector {
    let value: u16;
    unsafe {
        core::arch::asm!(
            "mov {0:x}, ds",
            out(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }
    SegmentSelector(value)
}

/// Get current privilege level from CS
#[inline]
pub fn current_privilege_level() -> Rpl {
    get_cs().rpl()
}

/// Check if currently running in kernel mode
#[inline]
pub fn in_kernel_mode() -> bool {
    current_privilege_level().is_kernel()
}

/// Check if currently running in user mode
#[inline]
pub fn in_user_mode() -> bool {
    current_privilege_level().is_user()
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selector_layout() {
        assert_eq!(KERNEL_CS.raw(), 0x08);
        assert_eq!(KERNEL_DS.raw(), 0x10);
        assert_eq!(USER_DS.raw(), 0x1B); // 3 << 3 | 3
        assert_eq!(USER_CS.raw(), 0x23); // 4 << 3 | 3
        assert_eq!(TSS_SELECTOR.raw(), 0x28);
    }

    #[test]
    fn test_selector_index() {
        assert_eq!(KERNEL_CS.index(), 1);
        assert_eq!(KERNEL_DS.index(), 2);
        assert_eq!(USER_DS.index(), 3);
        assert_eq!(USER_CS.index(), 4);
        assert_eq!(TSS_SELECTOR.index(), 5);
    }

    #[test]
    fn test_selector_rpl() {
        assert_eq!(KERNEL_CS.rpl(), Rpl::Ring0);
        assert_eq!(KERNEL_DS.rpl(), Rpl::Ring0);
        assert_eq!(USER_DS.rpl(), Rpl::Ring3);
        assert_eq!(USER_CS.rpl(), Rpl::Ring3);
    }

    #[test]
    fn test_rpl_check() {
        assert!(Rpl::Ring0.is_kernel());
        assert!(!Rpl::Ring0.is_user());
        assert!(Rpl::Ring3.is_user());
        assert!(!Rpl::Ring3.is_kernel());
    }
}
