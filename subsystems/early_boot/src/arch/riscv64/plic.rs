//! # RISC-V PLIC (Platform-Level Interrupt Controller)
//!
//! Driver for the PLIC interrupt controller.

use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::BootContext;
use crate::error::BootResult;

// =============================================================================
// PLIC CONSTANTS
// =============================================================================

/// Default PLIC base address (QEMU virt)
pub const PLIC_BASE_DEFAULT: u64 = 0x0C00_0000;

/// Maximum number of interrupt sources
pub const MAX_SOURCES: usize = 1024;
/// Maximum number of contexts (hart × 2 for M/S mode)
pub const MAX_CONTEXTS: usize = 15872;

// =============================================================================
// PLIC REGISTER OFFSETS
// =============================================================================

/// Priority registers base (source 0 at offset 0)
/// Each source has a 32-bit priority register
pub const PLIC_PRIORITY_BASE: u64 = 0x0000;

/// Pending registers base
/// Bit i corresponds to source i
pub const PLIC_PENDING_BASE: u64 = 0x1000;

/// Enable registers base per context
/// Context c starts at 0x2000 + c * 0x80
pub const PLIC_ENABLE_BASE: u64 = 0x2000;
pub const PLIC_ENABLE_STRIDE: u64 = 0x80;

/// Context registers base (threshold and claim/complete)
/// Context c: threshold at 0x200000 + c * 0x1000
///            claim/complete at 0x200000 + c * 0x1000 + 4
pub const PLIC_CONTEXT_BASE: u64 = 0x20_0000;
pub const PLIC_CONTEXT_STRIDE: u64 = 0x1000;
pub const PLIC_THRESHOLD_OFFSET: u64 = 0x0;
pub const PLIC_CLAIM_OFFSET: u64 = 0x4;

// =============================================================================
// PLIC STATE
// =============================================================================

/// PLIC base address
static PLIC_BASE: AtomicU64 = AtomicU64::new(PLIC_BASE_DEFAULT);
/// Number of sources
static NUM_SOURCES: AtomicU64 = AtomicU64::new(0);
/// Number of contexts
static NUM_CONTEXTS: AtomicU64 = AtomicU64::new(0);

// =============================================================================
// PLIC ACCESS
// =============================================================================

/// Read PLIC register
#[inline]
unsafe fn plic_read(offset: u64) -> u32 {
    let addr = PLIC_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::read_volatile(addr as *const u32)
}

/// Write PLIC register
#[inline]
unsafe fn plic_write(offset: u64, value: u32) {
    let addr = PLIC_BASE.load(Ordering::SeqCst) + offset;
    core::ptr::write_volatile(addr as *mut u32, value);
}

// =============================================================================
// PRIORITY MANAGEMENT
// =============================================================================

/// Set interrupt priority (0 = disabled, 1-7 = priority)
///
/// # Safety
///
/// The caller must ensure the interrupt ID is valid for this GIC implementation.
pub unsafe fn set_priority(source: u32, priority: u8) {
    if source as usize >= MAX_SOURCES {
        return;
    }
    let offset = PLIC_PRIORITY_BASE + (source as u64 * 4);
    plic_write(offset, priority as u32);
}

/// Get interrupt priority
///
/// # Safety
///
/// The caller must ensure the interrupt ID is valid.
pub unsafe fn get_priority(source: u32) -> u8 {
    if source as usize >= MAX_SOURCES {
        return 0;
    }
    let offset = PLIC_PRIORITY_BASE + (source as u64 * 4);
    plic_read(offset) as u8
}

// =============================================================================
// PENDING STATUS
// =============================================================================

/// Check if interrupt is pending
///
/// # Safety
///
/// The caller must ensure the interrupt ID is valid.
pub unsafe fn is_pending(source: u32) -> bool {
    if source as usize >= MAX_SOURCES {
        return false;
    }
    let reg = source / 32;
    let bit = source % 32;
    let offset = PLIC_PENDING_BASE + (reg as u64 * 4);
    (plic_read(offset) & (1 << bit)) != 0
}

/// Get all pending interrupts as bitmap
///
/// # Safety
///
/// The caller must ensure the interrupt ID is valid.
pub unsafe fn get_pending_bitmap(start_reg: u32) -> u32 {
    let offset = PLIC_PENDING_BASE + (start_reg as u64 * 4);
    plic_read(offset)
}

// =============================================================================
// ENABLE/DISABLE PER CONTEXT
// =============================================================================

/// Get context ID for hart and privilege mode
///
/// For QEMU virt machine:
/// - Context 2*hartid = M-mode
/// - Context 2*hartid + 1 = S-mode
pub fn get_context(hartid: u64, supervisor: bool) -> u32 {
    (hartid * 2 + if supervisor { 1 } else { 0 }) as u32
}

/// Enable interrupt for context
///
/// # Safety
///
/// The caller must ensure the system is ready for this feature to be enabled.
pub unsafe fn enable(source: u32, context: u32) {
    if source as usize >= MAX_SOURCES {
        return;
    }
    let reg = source / 32;
    let bit = source % 32;
    let offset = PLIC_ENABLE_BASE + (context as u64 * PLIC_ENABLE_STRIDE) + (reg as u64 * 4);
    let val = plic_read(offset);
    plic_write(offset, val | (1 << bit));
}

/// Disable interrupt for context
///
/// # Safety
///
/// The caller must ensure disabling this feature won't cause system instability.
pub unsafe fn disable(source: u32, context: u32) {
    if source as usize >= MAX_SOURCES {
        return;
    }
    let reg = source / 32;
    let bit = source % 32;
    let offset = PLIC_ENABLE_BASE + (context as u64 * PLIC_ENABLE_STRIDE) + (reg as u64 * 4);
    let val = plic_read(offset);
    plic_write(offset, val & !(1 << bit));
}

/// Check if interrupt is enabled for context
///
/// # Safety
///
/// The caller must ensure the system is ready for this feature to be enabled.
pub unsafe fn is_enabled(source: u32, context: u32) -> bool {
    if source as usize >= MAX_SOURCES {
        return false;
    }
    let reg = source / 32;
    let bit = source % 32;
    let offset = PLIC_ENABLE_BASE + (context as u64 * PLIC_ENABLE_STRIDE) + (reg as u64 * 4);
    (plic_read(offset) & (1 << bit)) != 0
}

/// Enable all sources for context
///
/// # Safety
///
/// The caller must ensure the system is ready for this feature to be enabled.
pub unsafe fn enable_all(context: u32, num_sources: u32) {
    let num_regs = (num_sources + 31) / 32;
    for reg in 0..num_regs {
        let offset = PLIC_ENABLE_BASE + (context as u64 * PLIC_ENABLE_STRIDE) + (reg as u64 * 4);
        plic_write(offset, 0xFFFF_FFFF);
    }
}

/// Disable all sources for context
///
/// # Safety
///
/// The caller must ensure disabling this feature won't cause system instability.
pub unsafe fn disable_all(context: u32, num_sources: u32) {
    let num_regs = (num_sources + 31) / 32;
    for reg in 0..num_regs {
        let offset = PLIC_ENABLE_BASE + (context as u64 * PLIC_ENABLE_STRIDE) + (reg as u64 * 4);
        plic_write(offset, 0);
    }
}

// =============================================================================
// THRESHOLD MANAGEMENT
// =============================================================================

/// Set priority threshold for context
/// Interrupts with priority <= threshold are masked
///
/// # Safety
///
/// The caller must ensure the context ID is valid.
pub unsafe fn set_threshold(context: u32, threshold: u8) {
    let offset = PLIC_CONTEXT_BASE + (context as u64 * PLIC_CONTEXT_STRIDE) + PLIC_THRESHOLD_OFFSET;
    plic_write(offset, threshold as u32);
}

/// Get priority threshold for context
///
/// # Safety
///
/// The caller must ensure the context ID is valid.
pub unsafe fn get_threshold(context: u32) -> u8 {
    let offset = PLIC_CONTEXT_BASE + (context as u64 * PLIC_CONTEXT_STRIDE) + PLIC_THRESHOLD_OFFSET;
    plic_read(offset) as u8
}

// =============================================================================
// CLAIM/COMPLETE
// =============================================================================

/// Claim highest priority pending interrupt
/// Returns 0 if no interrupt pending
///
/// # Safety
///
/// The caller must ensure an interrupt is pending for this context.
pub unsafe fn claim(context: u32) -> u32 {
    let offset = PLIC_CONTEXT_BASE + (context as u64 * PLIC_CONTEXT_STRIDE) + PLIC_CLAIM_OFFSET;
    plic_read(offset)
}

/// Complete interrupt handling
///
/// # Safety
///
/// The caller must ensure the interrupt was properly handled.
pub unsafe fn complete(context: u32, source: u32) {
    let offset = PLIC_CONTEXT_BASE + (context as u64 * PLIC_CONTEXT_STRIDE) + PLIC_CLAIM_OFFSET;
    plic_write(offset, source);
}

// =============================================================================
// HIGH-LEVEL API
// =============================================================================

/// PLIC instance
pub struct Plic {
    base: u64,
    num_sources: u32,
    num_contexts: u32,
}

impl Plic {
    /// Create new PLIC instance
    pub const fn new(base: u64, num_sources: u32, num_contexts: u32) -> Self {
        Self {
            base,
            num_sources,
            num_contexts,
        }
    }

    /// Initialize PLIC for a hart
    ///
    /// # Safety
    ///
    /// The caller must ensure system is in a valid state for initialization.
    pub unsafe fn init_hart(&self, hartid: u64) {
        // Get S-mode context for this hart
        let context = get_context(hartid, true);

        // Disable all interrupts initially
        disable_all(context, self.num_sources);

        // Set threshold to 0 (allow all priorities)
        set_threshold(context, 0);
    }

    /// Configure and enable an interrupt source
    ///
    /// # Safety
    ///
    /// The caller must ensure the hardware supports this configuration.
    pub unsafe fn configure_source(&self, source: u32, priority: u8, hartid: u64) {
        if source >= self.num_sources {
            return;
        }

        // Set priority
        set_priority(source, priority);

        // Enable for S-mode on specified hart
        let context = get_context(hartid, true);
        enable(source, context);
    }

    /// Handle interrupt (claim, return source)
    ///
    /// # Safety
    ///
    /// The caller must ensure an interrupt is pending and this context is safe for handling.
    pub unsafe fn handle_interrupt(&self, hartid: u64) -> Option<u32> {
        let context = get_context(hartid, true);
        let source = claim(context);
        if source == 0 {
            None
        } else {
            Some(source)
        }
    }

    /// Complete interrupt
    ///
    /// # Safety
    ///
    /// The caller must ensure the interrupt was properly handled.
    pub unsafe fn complete_interrupt(&self, hartid: u64, source: u32) {
        let context = get_context(hartid, true);
        complete(context, source);
    }
}

// =============================================================================
// VIRTIO INTERRUPT SOURCES (QEMU virt)
// =============================================================================

/// UART0 interrupt source
pub const UART0_IRQ: u32 = 10;
/// Virtio base IRQ (QEMU)
pub const VIRTIO_BASE_IRQ: u32 = 1;
/// Number of virtio devices
pub const VIRTIO_COUNT: u32 = 8;

/// Get virtio device IRQ
pub fn virtio_irq(index: u32) -> u32 {
    VIRTIO_BASE_IRQ + index
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize PLIC
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Get PLIC base from device tree or use default
    let base = if let Some(ref dt_info) = ctx.boot_info.device_tree {
        // TODO: Parse device tree
        PLIC_BASE_DEFAULT
    } else {
        PLIC_BASE_DEFAULT
    };

    PLIC_BASE.store(base, Ordering::SeqCst);

    // QEMU virt has 96 sources (0-95), but source 0 is reserved
    let num_sources = 96u32;
    NUM_SOURCES.store(num_sources as u64, Ordering::SeqCst);

    // Number of contexts (2 per hart for M/S mode)
    let num_harts = ctx.boot_info.num_cpus as u32;
    let num_contexts = num_harts * 2;
    NUM_CONTEXTS.store(num_contexts as u64, Ordering::SeqCst);

    // Initialize for boot hart
    let hartid = read_mhartid();

    // Disable all interrupts for this hart in S-mode
    let context = get_context(hartid, true);
    disable_all(context, num_sources);

    // Set threshold to 0 (accept all priorities)
    set_threshold(context, 0);

    // Set all sources to priority 1 (default)
    for source in 1..num_sources {
        set_priority(source, 1);
    }

    // Store PLIC info
    ctx.arch_data.riscv.plic_base = base;
    ctx.arch_data.riscv.plic_num_sources = num_sources;

    Ok(())
}

/// Initialize PLIC for secondary hart
///
/// # Safety
///
/// The caller must ensure system is in a valid state for initialization.
pub unsafe fn init_secondary(hartid: u64) {
    let num_sources = NUM_SOURCES.load(Ordering::SeqCst) as u32;
    let context = get_context(hartid, true);

    // Disable all interrupts
    disable_all(context, num_sources);

    // Set threshold to 0
    set_threshold(context, 0);
}

// =============================================================================
// EXTERNAL INTERRUPT HANDLER
// =============================================================================

/// External interrupt handler (call from trap handler)
///
/// # Safety
///
/// The caller must ensure an interrupt is pending and this context is safe for handling.
pub unsafe fn handle_external_interrupt(hartid: u64) -> u32 {
    let context = get_context(hartid, true);
    let source = claim(context);

    // Handle the interrupt here or dispatch to registered handler
    // ...

    // Complete the interrupt
    if source != 0 {
        complete(context, source);
    }

    source
}
