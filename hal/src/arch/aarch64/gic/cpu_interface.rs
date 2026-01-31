//! # GIC CPU Interface
//!
//! The CPU Interface is the component that CPUs use to interact with the GIC
//! for interrupt acknowledgment, priority masking, and end-of-interrupt signaling.
//!
//! ## GICv2 vs GICv3 CPU Interface
//!
//! | Feature                | GICv2 (GICC)        | GICv3 (ICC)           |
//! |------------------------|---------------------|-----------------------|
//! | Access Method          | Memory-mapped       | System registers      |
//! | Register Prefix        | GICC_*              | ICC_*_EL1             |
//! | Performance            | Slower (MMIO)       | Faster (sysreg)       |
//! | Priority Bits          | Up to 8             | Up to 8               |
//! | EOI Split              | Optional            | Supported             |
//! | IRQ/FIQ Separation     | Via group config    | Via group config      |
//!
//! ## Key Functions
//!
//! - **Acknowledge**: Read interrupt ID and mark as active
//! - **EOI**: Signal end of interrupt processing
//! - **Priority Mask**: Set threshold for interrupt delivery
//! - **Priority Grouping**: Configure preemption behavior
//! - **Running Priority**: Read currently executing priority

use super::Priority;
use core::arch::asm;

// ============================================================================
// Common CPU Interface Constants
// ============================================================================

/// Enable Group 0 interrupts
pub const ICC_IGRPEN0_EL1_ENABLE: u64 = 1 << 0;

/// Enable Group 1 interrupts
pub const ICC_IGRPEN1_EL1_ENABLE: u64 = 1 << 0;

/// SRE (System Register Enable) bit in ICC_SRE_EL1
pub const ICC_SRE_SRE: u64 = 1 << 0;

/// DFB (Disable FIQ Bypass) bit
pub const ICC_SRE_DFB: u64 = 1 << 1;

/// DIB (Disable IRQ Bypass) bit
pub const ICC_SRE_DIB: u64 = 1 << 2;

/// Enable (for EL3)
pub const ICC_SRE_ENABLE: u64 = 1 << 3;

/// EOI mode bit in ICC_CTLR_EL1
pub const ICC_CTLR_EOIMODE: u64 = 1 << 1;

/// CBPR (Common Binary Point Register) bit
pub const ICC_CTLR_CBPR: u64 = 1 << 0;

// ============================================================================
// System Register Access (GICv3)
// ============================================================================

/// Read ICC_SRE_EL1 (System Register Enable)
#[inline]
pub fn read_icc_sre_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C12_5", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write ICC_SRE_EL1
#[inline]
pub fn write_icc_sre_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C12_C12_5, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read ICC_SRE_EL2 (System Register Enable for EL2)
#[inline]
pub fn read_icc_sre_el2() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_4_C12_C9_5", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write ICC_SRE_EL2
#[inline]
pub fn write_icc_sre_el2(value: u64) {
    unsafe {
        asm!("msr S3_4_C12_C9_5, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read ICC_CTLR_EL1 (Control Register)
#[inline]
pub fn read_icc_ctlr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C12_4", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write ICC_CTLR_EL1
#[inline]
pub fn write_icc_ctlr_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C12_C12_4, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read ICC_PMR_EL1 (Priority Mask)
#[inline]
pub fn read_icc_pmr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C4_C6_0", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write ICC_PMR_EL1
#[inline]
pub fn write_icc_pmr_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C4_C6_0, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read ICC_BPR0_EL1 (Binary Point Register for Group 0)
#[inline]
pub fn read_icc_bpr0_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C8_3", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write ICC_BPR0_EL1
#[inline]
pub fn write_icc_bpr0_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C12_C8_3, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read ICC_BPR1_EL1 (Binary Point Register for Group 1)
#[inline]
pub fn read_icc_bpr1_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C12_3", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write ICC_BPR1_EL1
#[inline]
pub fn write_icc_bpr1_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C12_C12_3, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read ICC_IAR0_EL1 (Interrupt Acknowledge Register for Group 0)
#[inline]
pub fn read_icc_iar0_el1() -> u32 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C8_0", out(reg) value, options(nomem, nostack));
    }
    value as u32
}

/// Read ICC_IAR1_EL1 (Interrupt Acknowledge Register for Group 1)
#[inline]
pub fn read_icc_iar1_el1() -> u32 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C12_0", out(reg) value, options(nomem, nostack));
    }
    value as u32
}

/// Write ICC_EOIR0_EL1 (End of Interrupt Register for Group 0)
#[inline]
pub fn write_icc_eoir0_el1(value: u32) {
    unsafe {
        asm!("msr S3_0_C12_C8_1, {}", in(reg) value as u64, options(nomem, nostack));
    }
}

/// Write ICC_EOIR1_EL1 (End of Interrupt Register for Group 1)
#[inline]
pub fn write_icc_eoir1_el1(value: u32) {
    unsafe {
        asm!("msr S3_0_C12_C12_1, {}", in(reg) value as u64, options(nomem, nostack));
    }
}

/// Write ICC_DIR_EL1 (Deactivate Interrupt Register)
#[inline]
pub fn write_icc_dir_el1(value: u32) {
    unsafe {
        asm!("msr S3_0_C12_C11_1, {}", in(reg) value as u64, options(nomem, nostack));
    }
}

/// Read ICC_RPR_EL1 (Running Priority Register)
#[inline]
pub fn read_icc_rpr_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C11_3", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Read ICC_HPPIR0_EL1 (Highest Priority Pending Interrupt for Group 0)
#[inline]
pub fn read_icc_hppir0_el1() -> u32 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C8_2", out(reg) value, options(nomem, nostack));
    }
    value as u32
}

/// Read ICC_HPPIR1_EL1 (Highest Priority Pending Interrupt for Group 1)
#[inline]
pub fn read_icc_hppir1_el1() -> u32 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C12_2", out(reg) value, options(nomem, nostack));
    }
    value as u32
}

/// Read ICC_IGRPEN0_EL1 (Interrupt Group 0 Enable)
#[inline]
pub fn read_icc_igrpen0_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C12_6", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write ICC_IGRPEN0_EL1
#[inline]
pub fn write_icc_igrpen0_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C12_C12_6, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Read ICC_IGRPEN1_EL1 (Interrupt Group 1 Enable)
#[inline]
pub fn read_icc_igrpen1_el1() -> u64 {
    let value: u64;
    unsafe {
        asm!("mrs {}, S3_0_C12_C12_7", out(reg) value, options(nomem, nostack));
    }
    value
}

/// Write ICC_IGRPEN1_EL1
#[inline]
pub fn write_icc_igrpen1_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C12_C12_7, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Write ICC_SGI0R_EL1 (SGI Register for Group 0)
#[inline]
pub fn write_icc_sgi0r_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C12_C11_7, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Write ICC_SGI1R_EL1 (SGI Register for Group 1)
#[inline]
pub fn write_icc_sgi1r_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C12_C11_5, {}", in(reg) value, options(nomem, nostack));
    }
}

/// Write ICC_ASGI1R_EL1 (Alias SGI Register for Group 1)
#[inline]
pub fn write_icc_asgi1r_el1(value: u64) {
    unsafe {
        asm!("msr S3_0_C12_C11_6, {}", in(reg) value, options(nomem, nostack));
    }
}

// ============================================================================
// CPU Interface Operations
// ============================================================================

/// Enable the system register interface (GICv3)
///
/// This must be done before using any ICC_* registers.
pub fn enable_system_register_interface() {
    // Enable SRE, disable IRQ/FIQ bypass
    let sre = ICC_SRE_SRE | ICC_SRE_DFB | ICC_SRE_DIB;
    write_icc_sre_el1(sre);

    // ISB to ensure SRE is visible before accessing other ICC registers
    unsafe {
        asm!("isb", options(nomem, nostack));
    }
}

/// Check if the system register interface is enabled
pub fn is_system_register_interface_enabled() -> bool {
    (read_icc_sre_el1() & ICC_SRE_SRE) != 0
}

/// Enable interrupt groups
pub fn enable_interrupt_groups() {
    // Enable Group 0 and Group 1 interrupts
    write_icc_igrpen0_el1(ICC_IGRPEN0_EL1_ENABLE);
    write_icc_igrpen1_el1(ICC_IGRPEN1_EL1_ENABLE);
}

/// Disable interrupt groups
pub fn disable_interrupt_groups() {
    write_icc_igrpen0_el1(0);
    write_icc_igrpen1_el1(0);
}

/// Set the priority mask
///
/// Only interrupts with priority higher (numerically lower) than this
/// value will be delivered.
#[inline]
pub fn set_priority_mask(priority: Priority) {
    write_icc_pmr_el1(priority.value() as u64);
}

/// Get the priority mask
#[inline]
pub fn get_priority_mask() -> Priority {
    Priority(read_icc_pmr_el1() as u8)
}

/// Get the running priority (priority of currently active interrupt)
#[inline]
pub fn get_running_priority() -> Priority {
    Priority(read_icc_rpr_el1() as u8)
}

/// Acknowledge a Group 1 interrupt
///
/// Returns the interrupt ID. Reading this register marks the interrupt
/// as active and returns the INTID.
#[inline]
pub fn acknowledge_group1() -> u32 {
    read_icc_iar1_el1()
}

/// Acknowledge a Group 0 interrupt
#[inline]
pub fn acknowledge_group0() -> u32 {
    read_icc_iar0_el1()
}

/// Signal End of Interrupt for Group 1
#[inline]
pub fn end_of_interrupt_group1(intid: u32) {
    write_icc_eoir1_el1(intid);
}

/// Signal End of Interrupt for Group 0
#[inline]
pub fn end_of_interrupt_group0(intid: u32) {
    write_icc_eoir0_el1(intid);
}

/// Deactivate an interrupt (when using EOI mode 1)
#[inline]
pub fn deactivate_interrupt(intid: u32) {
    write_icc_dir_el1(intid);
}

/// Get the highest priority pending interrupt for Group 1
#[inline]
pub fn highest_priority_pending_group1() -> u32 {
    read_icc_hppir1_el1()
}

/// Get the highest priority pending interrupt for Group 0
#[inline]
pub fn highest_priority_pending_group0() -> u32 {
    read_icc_hppir0_el1()
}

/// Set the binary point for Group 1 (controls preemption granularity)
#[inline]
pub fn set_binary_point_group1(value: u8) {
    write_icc_bpr1_el1((value & 0x7) as u64);
}

/// Set the binary point for Group 0
#[inline]
pub fn set_binary_point_group0(value: u8) {
    write_icc_bpr0_el1((value & 0x7) as u64);
}

/// Enable EOI mode (separate priority drop and deactivation)
pub fn enable_eoi_mode() {
    let ctlr = read_icc_ctlr_el1() | ICC_CTLR_EOIMODE;
    write_icc_ctlr_el1(ctlr);
}

/// Disable EOI mode (combined priority drop and deactivation)
pub fn disable_eoi_mode() {
    let ctlr = read_icc_ctlr_el1() & !ICC_CTLR_EOIMODE;
    write_icc_ctlr_el1(ctlr);
}

// ============================================================================
// EOI Mode
// ============================================================================

/// EOI (End of Interrupt) modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EoiMode {
    /// Combined: Writing EOIR drops priority and deactivates interrupt
    Combined,
    /// Split: Writing EOIR drops priority, DIR deactivates interrupt
    Split,
}

impl EoiMode {
    /// Get the current EOI mode
    pub fn current() -> Self {
        if (read_icc_ctlr_el1() & ICC_CTLR_EOIMODE) != 0 {
            EoiMode::Split
        } else {
            EoiMode::Combined
        }
    }

    /// Set the EOI mode
    pub fn set(mode: EoiMode) {
        match mode {
            EoiMode::Combined => disable_eoi_mode(),
            EoiMode::Split => enable_eoi_mode(),
        }
    }
}

// ============================================================================
// Binary Point Register
// ============================================================================

/// Binary Point configuration for priority grouping
///
/// The Binary Point Register (BPR) determines how the 8-bit priority field
/// is split between group priority (for preemption) and subpriority.
///
/// | BPR Value | Group Priority Bits | Subpriority Bits |
/// |-----------|---------------------|------------------|
/// | 0         | 7                   | 1                |
/// | 1         | 6                   | 2                |
/// | 2         | 5                   | 3                |
/// | 3         | 4                   | 4                |
/// | 4         | 3                   | 5                |
/// | 5         | 2                   | 6                |
/// | 6         | 1                   | 7                |
/// | 7         | 0                   | 8                |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryPoint(pub u8);

impl BinaryPoint {
    /// Maximum preemption (7 group priority bits)
    pub const MAX_PREEMPTION: Self = BinaryPoint(0);

    /// No preemption (all subpriority)
    pub const NO_PREEMPTION: Self = BinaryPoint(7);

    /// Balanced (4 bits each)
    pub const BALANCED: Self = BinaryPoint(3);

    /// Get the number of group priority bits
    pub const fn group_priority_bits(self) -> u8 {
        7 - (self.0 & 0x7)
    }

    /// Get the number of subpriority bits
    pub const fn subpriority_bits(self) -> u8 {
        (self.0 & 0x7) + 1
    }
}

// ============================================================================
// CPU Interface Information
// ============================================================================

/// Information about the CPU interface
#[derive(Debug, Clone)]
pub struct CpuInterfaceInfo {
    /// System register interface enabled
    pub sre_enabled: bool,
    /// EOI mode
    pub eoi_mode: EoiMode,
    /// Current priority mask
    pub priority_mask: Priority,
    /// Running priority
    pub running_priority: Priority,
    /// Group 0 enabled
    pub group0_enabled: bool,
    /// Group 1 enabled
    pub group1_enabled: bool,
}

impl CpuInterfaceInfo {
    /// Read current CPU interface information
    pub fn current() -> Self {
        Self {
            sre_enabled: is_system_register_interface_enabled(),
            eoi_mode: EoiMode::current(),
            priority_mask: get_priority_mask(),
            running_priority: get_running_priority(),
            group0_enabled: (read_icc_igrpen0_el1() & ICC_IGRPEN0_EL1_ENABLE) != 0,
            group1_enabled: (read_icc_igrpen1_el1() & ICC_IGRPEN1_EL1_ENABLE) != 0,
        }
    }
}

// ============================================================================
// Initialization Helpers
// ============================================================================

/// Initialize the CPU interface for GICv3
///
/// This sets up the system register interface and enables interrupt groups.
pub fn init_gicv3_cpu_interface() {
    // Enable the system register interface
    enable_system_register_interface();

    // Set priority mask to allow all interrupts
    set_priority_mask(Priority::LOWEST);

    // Set default binary point (allow all preemption levels)
    set_binary_point_group0(0);
    set_binary_point_group1(0);

    // Use combined EOI mode
    EoiMode::set(EoiMode::Combined);

    // Enable interrupt groups
    enable_interrupt_groups();
}

/// Disable interrupts at the CPU interface level
pub fn disable_cpu_interface() {
    // Disable interrupt groups
    disable_interrupt_groups();

    // Set priority mask to block all interrupts
    set_priority_mask(Priority::HIGHEST);
}
