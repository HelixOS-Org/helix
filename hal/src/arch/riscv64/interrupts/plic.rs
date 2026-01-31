//! # Platform-Level Interrupt Controller (PLIC) Driver
//!
//! The PLIC is the standard external interrupt controller for RISC-V systems.
//! It supports priority-based interrupt routing to multiple contexts (harts/modes).
//!
//! ## Memory Map
//!
//! ```text
//! +------------------+---------+------------------------------------------+
//! | Offset           | Size    | Description                              |
//! +------------------+---------+------------------------------------------+
//! | 0x000000         | 4*N     | Priority registers (N sources)           |
//! | 0x001000         | 128     | Pending bits (1024 sources)              |
//! | 0x002000         | 128*C   | Enable bits (per context)                |
//! | 0x200000         | 0x1000*C| Threshold and Claim/Complete (per ctx)   |
//! +------------------+---------+------------------------------------------+
//! ```
//!
//! ## Context Mapping
//!
//! Context = hart_id * 2 + mode
//! - M-mode context: hart_id * 2
//! - S-mode context: hart_id * 2 + 1

use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// PLIC Register Offsets
// ============================================================================

/// Priority register base (4 bytes per source, source 0 is reserved)
pub const PRIORITY_OFFSET: usize = 0x00_0000;

/// Pending bits base (bit per source)
pub const PENDING_OFFSET: usize = 0x00_1000;

/// Enable bits base (bit per source, per context)
pub const ENABLE_OFFSET: usize = 0x00_2000;

/// Enable block size per context
pub const ENABLE_BLOCK_SIZE: usize = 0x80;

/// Threshold and Claim/Complete base
pub const THRESHOLD_OFFSET: usize = 0x20_0000;

/// Threshold and Claim block size per context
pub const CONTEXT_BLOCK_SIZE: usize = 0x1000;

/// Claim register offset within context block
pub const CLAIM_OFFSET: usize = 0x04;

/// Maximum number of interrupt sources
pub const MAX_SOURCES: usize = 1024;

/// Maximum number of contexts
pub const MAX_CONTEXTS: usize = 15872;

/// Maximum priority value (0 = disabled)
pub const MAX_PRIORITY: u32 = 7;

/// PLIC total size
pub const PLIC_SIZE: usize = 0x400000;

// ============================================================================
// PLIC State
// ============================================================================

/// Global PLIC base address
static PLIC_BASE: AtomicUsize = AtomicUsize::new(0);

/// Number of interrupt sources
static NUM_SOURCES: AtomicUsize = AtomicUsize::new(127);

/// Initialize PLIC with base address
///
/// # Safety
/// Must be called with valid PLIC base address.
pub unsafe fn init(base: usize, context: PlicContext) {
    PLIC_BASE.store(base, Ordering::SeqCst);

    // Initialize the context
    init_context(base, context);
}

/// Initialize a specific PLIC context
///
/// # Safety
/// Must be called with valid PLIC base address.
pub unsafe fn init_context(base: usize, context: PlicContext) {
    let plic = Plic::new(base);

    // Set threshold to 0 (accept all priorities > 0)
    plic.set_threshold(context, 0);

    // Disable all interrupts initially
    for source in 1..get_num_sources() {
        plic.disable_source(context, source);
    }
}

/// Get the PLIC base address
#[inline]
pub fn get_base() -> usize {
    PLIC_BASE.load(Ordering::Relaxed)
}

/// Set the number of interrupt sources
pub fn set_num_sources(count: usize) {
    NUM_SOURCES.store(count.min(MAX_SOURCES), Ordering::SeqCst);
}

/// Get the number of interrupt sources
pub fn get_num_sources() -> usize {
    NUM_SOURCES.load(Ordering::Relaxed)
}

// ============================================================================
// PLIC Context
// ============================================================================

/// PLIC context identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlicContext(u32);

impl PlicContext {
    /// Create a context from raw value
    pub const fn from_raw(value: u32) -> Self {
        Self(value)
    }

    /// Get the raw context value
    pub const fn as_raw(self) -> u32 {
        self.0
    }

    /// Get the M-mode context for a hart
    pub const fn m_mode(hart_id: usize) -> Self {
        Self((hart_id * 2) as u32)
    }

    /// Get the S-mode context for a hart
    pub const fn s_mode(hart_id: usize) -> Self {
        Self((hart_id * 2 + 1) as u32)
    }
}

/// Convert hart ID to S-mode context
#[inline]
pub const fn hart_to_smode_context(hart_id: usize) -> PlicContext {
    PlicContext::s_mode(hart_id)
}

/// Convert hart ID to M-mode context
#[inline]
pub const fn hart_to_mmode_context(hart_id: usize) -> PlicContext {
    PlicContext::m_mode(hart_id)
}

// ============================================================================
// PLIC Structure
// ============================================================================

/// PLIC interface
#[derive(Debug)]
pub struct Plic {
    base: usize,
}

impl Plic {
    /// Create a new PLIC instance with the given base address
    ///
    /// # Safety
    /// The base address must point to valid PLIC registers.
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }

    /// Get PLIC for the current platform
    pub fn current() -> Self {
        Self {
            base: get_base(),
        }
    }

    // ========================================================================
    // Priority Registers
    // ========================================================================

    /// Get the priority register address for a source
    #[inline]
    fn priority_addr(&self, source: usize) -> *mut u32 {
        debug_assert!(source > 0 && source < MAX_SOURCES);
        (self.base + PRIORITY_OFFSET + source * 4) as *mut u32
    }

    /// Get the priority of an interrupt source
    #[inline]
    pub fn get_priority(&self, source: usize) -> u32 {
        unsafe { read_volatile(self.priority_addr(source)) }
    }

    /// Set the priority of an interrupt source (0 = disabled)
    #[inline]
    pub fn set_priority(&self, source: usize, priority: u32) {
        let priority = priority.min(MAX_PRIORITY);
        unsafe { write_volatile(self.priority_addr(source), priority) }
    }

    // ========================================================================
    // Pending Registers
    // ========================================================================

    /// Get the pending register address for a source (32 sources per register)
    #[inline]
    fn pending_addr(&self, source: usize) -> *const u32 {
        let reg = source / 32;
        (self.base + PENDING_OFFSET + reg * 4) as *const u32
    }

    /// Check if an interrupt source is pending
    #[inline]
    pub fn is_pending(&self, source: usize) -> bool {
        let reg = unsafe { read_volatile(self.pending_addr(source)) };
        let bit = source % 32;
        (reg >> bit) & 1 != 0
    }

    /// Get all pending interrupts as a bitmask
    pub fn get_pending_mask(&self) -> [u32; 32] {
        let mut mask = [0u32; 32];
        for i in 0..32 {
            let addr = (self.base + PENDING_OFFSET + i * 4) as *const u32;
            mask[i] = unsafe { read_volatile(addr) };
        }
        mask
    }

    // ========================================================================
    // Enable Registers
    // ========================================================================

    /// Get the enable register address for a source and context
    #[inline]
    fn enable_addr(&self, context: PlicContext, source: usize) -> *mut u32 {
        let ctx = context.as_raw() as usize;
        let reg = source / 32;
        (self.base + ENABLE_OFFSET + ctx * ENABLE_BLOCK_SIZE + reg * 4) as *mut u32
    }

    /// Enable an interrupt source for a context
    #[inline]
    pub fn enable_source(&self, context: PlicContext, source: usize) {
        let addr = self.enable_addr(context, source);
        let bit = 1u32 << (source % 32);
        unsafe {
            let current = read_volatile(addr);
            write_volatile(addr, current | bit);
        }
    }

    /// Disable an interrupt source for a context
    #[inline]
    pub fn disable_source(&self, context: PlicContext, source: usize) {
        let addr = self.enable_addr(context, source);
        let bit = 1u32 << (source % 32);
        unsafe {
            let current = read_volatile(addr);
            write_volatile(addr, current & !bit);
        }
    }

    /// Check if an interrupt source is enabled for a context
    #[inline]
    pub fn is_enabled(&self, context: PlicContext, source: usize) -> bool {
        let addr = self.enable_addr(context, source);
        let bit = source % 32;
        unsafe { (read_volatile(addr) >> bit) & 1 != 0 }
    }

    /// Set the enable mask for a range of 32 sources
    pub fn set_enable_mask(&self, context: PlicContext, reg: usize, mask: u32) {
        let ctx = context.as_raw() as usize;
        let addr = (self.base + ENABLE_OFFSET + ctx * ENABLE_BLOCK_SIZE + reg * 4) as *mut u32;
        unsafe { write_volatile(addr, mask) }
    }

    /// Get the enable mask for a range of 32 sources
    pub fn get_enable_mask(&self, context: PlicContext, reg: usize) -> u32 {
        let ctx = context.as_raw() as usize;
        let addr = (self.base + ENABLE_OFFSET + ctx * ENABLE_BLOCK_SIZE + reg * 4) as *const u32;
        unsafe { read_volatile(addr) }
    }

    // ========================================================================
    // Threshold and Claim/Complete
    // ========================================================================

    /// Get the threshold register address for a context
    #[inline]
    fn threshold_addr(&self, context: PlicContext) -> *mut u32 {
        let ctx = context.as_raw() as usize;
        (self.base + THRESHOLD_OFFSET + ctx * CONTEXT_BLOCK_SIZE) as *mut u32
    }

    /// Get the claim/complete register address for a context
    #[inline]
    fn claim_addr(&self, context: PlicContext) -> *mut u32 {
        let ctx = context.as_raw() as usize;
        (self.base + THRESHOLD_OFFSET + ctx * CONTEXT_BLOCK_SIZE + CLAIM_OFFSET) as *mut u32
    }

    /// Get the threshold for a context
    #[inline]
    pub fn get_threshold(&self, context: PlicContext) -> u32 {
        unsafe { read_volatile(self.threshold_addr(context)) }
    }

    /// Set the threshold for a context
    ///
    /// Only interrupts with priority > threshold will be delivered
    #[inline]
    pub fn set_threshold(&self, context: PlicContext, threshold: u32) {
        let threshold = threshold.min(MAX_PRIORITY);
        unsafe { write_volatile(self.threshold_addr(context), threshold) }
    }

    /// Claim an interrupt (returns source ID, 0 if none pending)
    #[inline]
    pub fn claim(&self, context: PlicContext) -> u32 {
        unsafe { read_volatile(self.claim_addr(context)) }
    }

    /// Complete an interrupt
    #[inline]
    pub fn complete(&self, context: PlicContext, source: u32) {
        unsafe { write_volatile(self.claim_addr(context), source) }
    }

    // ========================================================================
    // Convenience Methods
    // ========================================================================

    /// Configure an interrupt source
    pub fn configure_source(&self, context: PlicContext, source: usize, priority: u32, enable: bool) {
        self.set_priority(source, priority);
        if enable {
            self.enable_source(context, source);
        } else {
            self.disable_source(context, source);
        }
    }

    /// Handle an interrupt (claim, call handler, complete)
    ///
    /// Returns the source that was handled, or 0 if none.
    pub fn handle_interrupt<F>(&self, context: PlicContext, mut handler: F) -> u32
    where
        F: FnMut(u32),
    {
        let source = self.claim(context);
        if source != 0 {
            handler(source);
            self.complete(context, source);
        }
        source
    }

    /// Handle all pending interrupts
    ///
    /// Returns the number of interrupts handled.
    pub fn handle_all_pending<F>(&self, context: PlicContext, mut handler: F) -> usize
    where
        F: FnMut(u32),
    {
        let mut count = 0;
        loop {
            let source = self.claim(context);
            if source == 0 {
                break;
            }
            handler(source);
            self.complete(context, source);
            count += 1;
        }
        count
    }
}

// ============================================================================
// Global Functions
// ============================================================================

/// Claim the highest-priority pending interrupt
pub fn claim_interrupt(context: PlicContext) -> u32 {
    Plic::current().claim(context)
}

/// Complete an interrupt
pub fn complete_interrupt(context: PlicContext, source: u32) {
    Plic::current().complete(context, source);
}

/// Enable an interrupt source
pub fn enable_irq(context: PlicContext, source: usize, priority: u32) {
    let plic = Plic::current();
    plic.set_priority(source, priority);
    plic.enable_source(context, source);
}

/// Disable an interrupt source
pub fn disable_irq(context: PlicContext, source: usize) {
    let plic = Plic::current();
    plic.disable_source(context, source);
}

/// Set the priority threshold
pub fn set_threshold(context: PlicContext, threshold: u32) {
    Plic::current().set_threshold(context, threshold);
}

// ============================================================================
// Interrupt Source Types
// ============================================================================

/// Common PLIC source IDs (platform-specific)
pub mod sources {
    /// UART0 interrupt (QEMU virt)
    pub const UART0: usize = 10;

    /// Virtio block device (QEMU virt)
    pub const VIRTIO_BLK: usize = 1;

    /// Virtio network device (QEMU virt)
    pub const VIRTIO_NET: usize = 2;

    /// RTC interrupt (QEMU virt)
    pub const RTC: usize = 11;

    /// PCIe interrupt base (QEMU virt)
    pub const PCIE_BASE: usize = 32;
}

// ============================================================================
// Interrupt Affinity
// ============================================================================

/// Interrupt affinity settings
#[derive(Debug, Clone)]
pub struct InterruptAffinity {
    /// Bit mask of contexts that can receive this interrupt
    pub context_mask: u64,
    /// Priority for this interrupt
    pub priority: u32,
}

impl InterruptAffinity {
    /// Create affinity for a single context
    pub const fn single(context: PlicContext, priority: u32) -> Self {
        Self {
            context_mask: 1 << context.as_raw(),
            priority,
        }
    }

    /// Create affinity for all contexts
    pub const fn all(priority: u32) -> Self {
        Self {
            context_mask: u64::MAX,
            priority,
        }
    }

    /// Create affinity for S-mode contexts only
    pub fn s_mode_only(hart_count: usize, priority: u32) -> Self {
        let mut mask = 0u64;
        for hart in 0..hart_count {
            mask |= 1 << PlicContext::s_mode(hart).as_raw();
        }
        Self {
            context_mask: mask,
            priority,
        }
    }
}

/// Apply affinity settings for an interrupt source
pub fn set_affinity(source: usize, affinity: &InterruptAffinity) {
    let plic = Plic::current();
    plic.set_priority(source, affinity.priority);

    // Enable for each context in the mask
    for ctx_id in 0..64 {
        if (affinity.context_mask >> ctx_id) & 1 != 0 {
            let context = PlicContext::from_raw(ctx_id as u32);
            plic.enable_source(context, source);
        }
    }
}

// ============================================================================
// PLIC State Save/Restore
// ============================================================================

/// Saved PLIC state for a context
#[derive(Debug)]
pub struct PlicContextState {
    /// Threshold
    pub threshold: u32,
    /// Enable masks
    pub enable_masks: [u32; 32],
}

impl PlicContextState {
    /// Save the current state
    pub fn save(context: PlicContext) -> Self {
        let plic = Plic::current();
        let mut enable_masks = [0u32; 32];

        for i in 0..32 {
            enable_masks[i] = plic.get_enable_mask(context, i);
        }

        Self {
            threshold: plic.get_threshold(context),
            enable_masks,
        }
    }

    /// Restore the state
    pub fn restore(&self, context: PlicContext) {
        let plic = Plic::current();

        plic.set_threshold(context, self.threshold);
        for i in 0..32 {
            plic.set_enable_mask(context, i, self.enable_masks[i]);
        }
    }
}

/// Saved PLIC global state (priorities)
#[derive(Debug)]
pub struct PlicGlobalState {
    /// Priority for each source
    pub priorities: [u32; 128],
}

impl PlicGlobalState {
    /// Save global state
    pub fn save() -> Self {
        let plic = Plic::current();
        let mut priorities = [0u32; 128];

        for i in 1..128 {
            priorities[i] = plic.get_priority(i);
        }

        Self { priorities }
    }

    /// Restore global state
    pub fn restore(&self) {
        let plic = Plic::current();

        for i in 1..128 {
            plic.set_priority(i, self.priorities[i]);
        }
    }
}

// ============================================================================
// PLIC Detection
// ============================================================================

/// Check if PLIC is present at the given address
///
/// # Safety
/// Address must be mapped and accessible.
pub unsafe fn probe_plic(base: usize) -> bool {
    let plic = Plic::new(base);

    // Try to read and write a priority register
    let old = plic.get_priority(1);

    // Write a known value
    plic.set_priority(1, MAX_PRIORITY);
    let read_back = plic.get_priority(1);

    // Restore
    plic.set_priority(1, old);

    // Check if write worked
    read_back == MAX_PRIORITY
}

/// Detect number of interrupt sources
///
/// # Safety
/// Address must be mapped and accessible.
pub unsafe fn detect_num_sources(base: usize) -> usize {
    let plic = Plic::new(base);

    // Binary search for highest valid source
    let mut low = 1;
    let mut high = MAX_SOURCES;

    while low < high {
        let mid = (low + high) / 2;

        // Try to write priority
        let old = plic.get_priority(mid);
        plic.set_priority(mid, MAX_PRIORITY);
        let valid = plic.get_priority(mid) == MAX_PRIORITY;
        plic.set_priority(mid, old);

        if valid {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    low.saturating_sub(1)
}
