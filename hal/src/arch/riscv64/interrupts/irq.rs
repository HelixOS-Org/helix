//! # IRQ Management
//!
//! High-level IRQ management for RISC-V systems.
//! Provides IRQ handler registration, dispatch, and management.

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{InterruptType, MAX_EXTERNAL_IRQS};
use super::plic::{self, Plic, PlicContext};
use super::clint;

// ============================================================================
// IRQ Handler Types
// ============================================================================

/// IRQ handler function type
pub type IrqHandler = fn(irq: usize, context: *mut ()) -> IrqResult;

/// Result of an IRQ handler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqResult {
    /// Interrupt was handled
    Handled,
    /// Interrupt was not handled
    NotHandled,
    /// Need to wake up a waiting thread
    WakeUp,
    /// Need to reschedule
    Reschedule,
}

/// IRQ handler entry
#[derive(Clone, Copy)]
pub struct IrqEntry {
    /// Handler function
    pub handler: Option<IrqHandler>,
    /// Context pointer passed to handler
    pub context: *mut (),
    /// Whether this IRQ is registered
    pub registered: bool,
    /// Whether this IRQ is enabled
    pub enabled: bool,
    /// IRQ name (for debugging)
    pub name: &'static str,
}

impl IrqEntry {
    /// Empty entry
    pub const fn empty() -> Self {
        Self {
            handler: None,
            context: core::ptr::null_mut(),
            registered: false,
            enabled: false,
            name: "",
        }
    }
}

// SAFETY: IrqEntry is Send because the context pointer is only used by the handler
unsafe impl Send for IrqEntry {}
unsafe impl Sync for IrqEntry {}

// ============================================================================
// IRQ Table
// ============================================================================

/// Maximum number of IRQ handlers
const MAX_HANDLERS: usize = 256;

/// IRQ handler table
static mut IRQ_TABLE: [IrqEntry; MAX_HANDLERS] = [IrqEntry::empty(); MAX_HANDLERS];

/// IRQ table lock (simple spinlock)
static IRQ_TABLE_LOCK: AtomicBool = AtomicBool::new(false);

/// Acquire IRQ table lock
fn lock_irq_table() {
    while IRQ_TABLE_LOCK
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        core::hint::spin_loop();
    }
}

/// Release IRQ table lock
fn unlock_irq_table() {
    IRQ_TABLE_LOCK.store(false, Ordering::Release);
}

// ============================================================================
// IRQ Registration
// ============================================================================

/// Register an IRQ handler
///
/// # Safety
/// Context pointer must remain valid for the lifetime of the handler.
pub unsafe fn register_irq_handler(
    irq: usize,
    handler: IrqHandler,
    context: *mut (),
    name: &'static str,
) -> Result<(), IrqError> {
    if irq >= MAX_HANDLERS {
        return Err(IrqError::InvalidIrq);
    }

    lock_irq_table();

    if IRQ_TABLE[irq].registered {
        unlock_irq_table();
        return Err(IrqError::AlreadyRegistered);
    }

    IRQ_TABLE[irq] = IrqEntry {
        handler: Some(handler),
        context,
        registered: true,
        enabled: false,
        name,
    };

    unlock_irq_table();
    Ok(())
}

/// Unregister an IRQ handler
pub fn unregister_irq_handler(irq: usize) -> Result<(), IrqError> {
    if irq >= MAX_HANDLERS {
        return Err(IrqError::InvalidIrq);
    }

    lock_irq_table();

    unsafe {
        if !IRQ_TABLE[irq].registered {
            unlock_irq_table();
            return Err(IrqError::NotRegistered);
        }

        // Disable in PLIC first
        let context = PlicContext::s_mode(0); // TODO: Current hart
        plic::disable_irq(context, irq);

        IRQ_TABLE[irq] = IrqEntry::empty();
    }

    unlock_irq_table();
    Ok(())
}

/// Enable an IRQ
pub fn enable_irq(irq: usize, priority: u32) -> Result<(), IrqError> {
    if irq >= MAX_HANDLERS {
        return Err(IrqError::InvalidIrq);
    }

    lock_irq_table();

    unsafe {
        if !IRQ_TABLE[irq].registered {
            unlock_irq_table();
            return Err(IrqError::NotRegistered);
        }

        IRQ_TABLE[irq].enabled = true;
    }

    unlock_irq_table();

    // Enable in PLIC
    let context = PlicContext::s_mode(0); // TODO: Current hart
    plic::enable_irq(context, irq, priority);

    Ok(())
}

/// Disable an IRQ
pub fn disable_irq(irq: usize) -> Result<(), IrqError> {
    if irq >= MAX_HANDLERS {
        return Err(IrqError::InvalidIrq);
    }

    lock_irq_table();

    unsafe {
        IRQ_TABLE[irq].enabled = false;
    }

    unlock_irq_table();

    // Disable in PLIC
    let context = PlicContext::s_mode(0); // TODO: Current hart
    plic::disable_irq(context, irq);

    Ok(())
}

// ============================================================================
// IRQ Errors
// ============================================================================

/// IRQ operation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqError {
    /// Invalid IRQ number
    InvalidIrq,
    /// IRQ already registered
    AlreadyRegistered,
    /// IRQ not registered
    NotRegistered,
    /// Handler not found
    NotFound,
    /// Resource busy
    Busy,
}

// ============================================================================
// Interrupt Dispatch
// ============================================================================

/// Dispatch an external interrupt
pub fn dispatch_interrupt(hart_id: usize) -> IrqResult {
    let context = PlicContext::s_mode(hart_id);
    let plic = Plic::current();

    // Claim the interrupt
    let irq = plic.claim(context) as usize;
    if irq == 0 {
        return IrqResult::NotHandled;
    }

    // Look up handler
    let result = unsafe {
        let entry = &IRQ_TABLE[irq.min(MAX_HANDLERS - 1)];
        if let Some(handler) = entry.handler {
            handler(irq, entry.context)
        } else {
            IrqResult::NotHandled
        }
    };

    // Complete the interrupt
    plic.complete(context, irq as u32);

    // Update statistics
    IRQ_STATS[irq.min(63)].fetch_add(1, Ordering::Relaxed);

    result
}

/// Dispatch all pending external interrupts
pub fn dispatch_all_pending(hart_id: usize) -> usize {
    let context = PlicContext::s_mode(hart_id);
    let plic = Plic::current();
    let mut count = 0;

    loop {
        let irq = plic.claim(context) as usize;
        if irq == 0 {
            break;
        }

        // Look up handler
        unsafe {
            let entry = &IRQ_TABLE[irq.min(MAX_HANDLERS - 1)];
            if let Some(handler) = entry.handler {
                handler(irq, entry.context);
            }
        }

        // Complete the interrupt
        plic.complete(context, irq as u32);
        count += 1;

        // Update statistics
        IRQ_STATS[irq.min(63)].fetch_add(1, Ordering::Relaxed);
    }

    count
}

// ============================================================================
// Specialized Handlers
// ============================================================================

/// Handle a timer interrupt
pub fn handle_timer_interrupt(hart_id: usize) -> IrqResult {
    // Clear the timer by setting far future deadline
    clint::sbi_set_timer(u64::MAX);

    // Call registered timer handler
    unsafe {
        if let Some(handler) = TIMER_HANDLER {
            return handler(hart_id);
        }
    }

    IrqResult::Handled
}

/// Timer interrupt handler type
pub type TimerHandler = fn(hart_id: usize) -> IrqResult;

/// Registered timer handler
static mut TIMER_HANDLER: Option<TimerHandler> = None;

/// Register a timer handler
///
/// # Safety
/// Must be called before timer interrupts are enabled.
pub unsafe fn register_timer_handler(handler: TimerHandler) {
    TIMER_HANDLER = Some(handler);
}

/// Handle a software interrupt (IPI)
pub fn handle_software_interrupt(hart_id: usize) -> IrqResult {
    // Clear the software interrupt
    clint::clear_software_interrupt(hart_id);

    // Call registered IPI handler
    unsafe {
        if let Some(handler) = IPI_HANDLER {
            return handler(hart_id);
        }
    }

    IrqResult::Handled
}

/// IPI handler type
pub type IpiHandler = fn(hart_id: usize) -> IrqResult;

/// Registered IPI handler
static mut IPI_HANDLER: Option<IpiHandler> = None;

/// Register an IPI handler
///
/// # Safety
/// Must be called before software interrupts are enabled.
pub unsafe fn register_ipi_handler(handler: IpiHandler) {
    IPI_HANDLER = Some(handler);
}

// ============================================================================
// Master Interrupt Dispatcher
// ============================================================================

/// Master interrupt dispatcher
///
/// Called from trap handler for all interrupts.
pub fn master_dispatch(cause: u64, hart_id: usize) -> IrqResult {
    use super::super::core::csr::irq_cause;

    let code = cause & !0x8000_0000_0000_0000;

    match code {
        irq_cause::SUPERVISOR_SOFTWARE => handle_software_interrupt(hart_id),
        irq_cause::SUPERVISOR_TIMER => handle_timer_interrupt(hart_id),
        irq_cause::SUPERVISOR_EXTERNAL => dispatch_interrupt(hart_id),
        _ => {
            // Unknown interrupt
            IrqResult::NotHandled
        }
    }
}

// ============================================================================
// IRQ Statistics
// ============================================================================

/// IRQ statistics (first 64 IRQs)
static IRQ_STATS: [AtomicU64; 64] = {
    const INIT: AtomicU64 = AtomicU64::new(0);
    [INIT; 64]
};

/// Get IRQ count
pub fn get_irq_count(irq: usize) -> u64 {
    if irq < 64 {
        IRQ_STATS[irq].load(Ordering::Relaxed)
    } else {
        0
    }
}

/// Reset IRQ statistics
pub fn reset_irq_stats() {
    for stat in &IRQ_STATS {
        stat.store(0, Ordering::Relaxed);
    }
}

// ============================================================================
// IRQ Info
// ============================================================================

/// Get IRQ information
pub fn get_irq_info(irq: usize) -> Option<IrqInfo> {
    if irq >= MAX_HANDLERS {
        return None;
    }

    lock_irq_table();

    let info = unsafe {
        let entry = &IRQ_TABLE[irq];
        if entry.registered {
            Some(IrqInfo {
                irq,
                name: entry.name,
                enabled: entry.enabled,
                count: get_irq_count(irq),
            })
        } else {
            None
        }
    };

    unlock_irq_table();
    info
}

/// IRQ information
#[derive(Debug)]
pub struct IrqInfo {
    /// IRQ number
    pub irq: usize,
    /// IRQ name
    pub name: &'static str,
    /// Whether enabled
    pub enabled: bool,
    /// Interrupt count
    pub count: u64,
}

/// List all registered IRQs
pub fn list_irqs() -> alloc::vec::Vec<IrqInfo> {
    let mut irqs = alloc::vec::Vec::new();

    lock_irq_table();

    for irq in 0..MAX_HANDLERS {
        unsafe {
            let entry = &IRQ_TABLE[irq];
            if entry.registered {
                irqs.push(IrqInfo {
                    irq,
                    name: entry.name,
                    enabled: entry.enabled,
                    count: get_irq_count(irq),
                });
            }
        }
    }

    unlock_irq_table();
    irqs
}

extern crate alloc;

// ============================================================================
// IRQ Affinity
// ============================================================================

/// Set IRQ affinity to specific harts
pub fn set_irq_affinity(irq: usize, hart_mask: u64, priority: u32) -> Result<(), IrqError> {
    if irq >= MAX_HANDLERS {
        return Err(IrqError::InvalidIrq);
    }

    let plic = Plic::current();

    // Set priority
    plic.set_priority(irq, priority);

    // Enable/disable for each hart
    for hart in 0..64 {
        let context = PlicContext::s_mode(hart);
        if (hart_mask >> hart) & 1 != 0 {
            plic.enable_source(context, irq);
        } else {
            plic.disable_source(context, irq);
        }
    }

    Ok(())
}

// ============================================================================
// Interrupt Nesting
// ============================================================================

/// Interrupt nesting depth per hart
static mut NESTING_DEPTH: [u32; 256] = [0; 256];

/// Get current nesting depth
pub fn get_nesting_depth(hart_id: usize) -> u32 {
    if hart_id < 256 {
        unsafe { NESTING_DEPTH[hart_id] }
    } else {
        0
    }
}

/// Enter interrupt context (increment nesting)
pub fn enter_interrupt(hart_id: usize) {
    if hart_id < 256 {
        unsafe {
            NESTING_DEPTH[hart_id] += 1;
        }
    }
}

/// Exit interrupt context (decrement nesting)
pub fn exit_interrupt(hart_id: usize) {
    if hart_id < 256 {
        unsafe {
            NESTING_DEPTH[hart_id] = NESTING_DEPTH[hart_id].saturating_sub(1);
        }
    }
}

/// Check if we're in an interrupt context
pub fn in_interrupt(hart_id: usize) -> bool {
    get_nesting_depth(hart_id) > 0
}

// ============================================================================
// Interrupt-Safe Operations
// ============================================================================

/// Execute a closure with interrupts disabled
#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let was_enabled = super::disable_global_interrupts_save();
    let result = f();
    super::restore_global_interrupts(was_enabled);
    result
}

/// Interrupt guard - disables interrupts for the duration of its lifetime
pub struct InterruptGuard {
    was_enabled: bool,
}

impl InterruptGuard {
    /// Create a new interrupt guard (disables interrupts)
    pub fn new() -> Self {
        Self {
            was_enabled: super::disable_global_interrupts_save(),
        }
    }
}

impl Default for InterruptGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        super::restore_global_interrupts(self.was_enabled);
    }
}

// ============================================================================
// Deferred Work
// ============================================================================

/// Deferred work item
pub type DeferredWork = fn(*mut ());

/// Maximum deferred work items
const MAX_DEFERRED: usize = 64;

/// Deferred work queue
struct DeferredQueue {
    items: [(DeferredWork, *mut ()); MAX_DEFERRED],
    head: usize,
    tail: usize,
}

impl DeferredQueue {
    const fn new() -> Self {
        Self {
            items: [(dummy_work, core::ptr::null_mut()); MAX_DEFERRED],
            head: 0,
            tail: 0,
        }
    }

    fn push(&mut self, work: DeferredWork, context: *mut ()) -> bool {
        let next = (self.tail + 1) % MAX_DEFERRED;
        if next == self.head {
            return false; // Full
        }
        self.items[self.tail] = (work, context);
        self.tail = next;
        true
    }

    fn pop(&mut self) -> Option<(DeferredWork, *mut ())> {
        if self.head == self.tail {
            return None; // Empty
        }
        let item = self.items[self.head];
        self.head = (self.head + 1) % MAX_DEFERRED;
        Some(item)
    }

    fn is_empty(&self) -> bool {
        self.head == self.tail
    }
}

fn dummy_work(_: *mut ()) {}

/// Per-hart deferred work queues
static mut DEFERRED_QUEUES: [DeferredQueue; 256] = {
    const INIT: DeferredQueue = DeferredQueue::new();
    [INIT; 256]
};

/// Queue deferred work
///
/// # Safety
/// Context pointer must remain valid until work is executed.
pub unsafe fn queue_deferred_work(
    hart_id: usize,
    work: DeferredWork,
    context: *mut (),
) -> bool {
    if hart_id >= 256 {
        return false;
    }

    let _guard = InterruptGuard::new();
    DEFERRED_QUEUES[hart_id].push(work, context)
}

/// Process all deferred work for this hart
pub fn process_deferred_work(hart_id: usize) {
    if hart_id >= 256 {
        return;
    }

    loop {
        let item = {
            let _guard = InterruptGuard::new();
            unsafe { DEFERRED_QUEUES[hart_id].pop() }
        };

        match item {
            Some((work, context)) => work(context),
            None => break,
        }
    }
}

/// Check if there's pending deferred work
pub fn has_deferred_work(hart_id: usize) -> bool {
    if hart_id >= 256 {
        return false;
    }
    unsafe { !DEFERRED_QUEUES[hart_id].is_empty() }
}
