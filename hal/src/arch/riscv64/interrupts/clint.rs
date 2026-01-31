//! # Core Local Interruptor (CLINT) Driver
//!
//! The CLINT provides timer and software interrupts on RISC-V platforms.
//! It is the standard timer/IPI controller for RISC-V systems.
//!
//! ## Memory Map
//!
//! ```text
//! +------------------+--------+------------------------------------------+
//! | Offset           | Size   | Description                              |
//! +------------------+--------+------------------------------------------+
//! | 0x0000           | 4*N    | MSIP (Machine Software Interrupt Pending)|
//! | 0x4000           | 8*N    | MTIMECMP (Timer Compare per hart)        |
//! | 0xBFF8           | 8      | MTIME (Timer Value, shared)              |
//! +------------------+--------+------------------------------------------+
//! ```
//!
//! Note: For S-mode, we use SBI calls to access CLINT functionality.
//! Direct CLINT access is only available in M-mode.

use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicUsize, Ordering};

// ============================================================================
// CLINT Register Offsets
// ============================================================================

/// MSIP (Machine Software Interrupt Pending) base offset
pub const MSIP_OFFSET: usize = 0x0000;

/// MTIMECMP (Machine Timer Compare) base offset
pub const MTIMECMP_OFFSET: usize = 0x4000;

/// MTIME (Machine Timer Value) offset
pub const MTIME_OFFSET: usize = 0xBFF8;

/// CLINT size
pub const CLINT_SIZE: usize = 0x10000;

// ============================================================================
// CLINT State
// ============================================================================

/// Global CLINT base address
static CLINT_BASE: AtomicUsize = AtomicUsize::new(0);

/// Initialize CLINT with base address
///
/// # Safety
/// Must be called with valid CLINT base address.
pub unsafe fn init(base: usize, _hart_id: usize) {
    CLINT_BASE.store(base, Ordering::SeqCst);
}

/// Get the CLINT base address
#[inline]
pub fn get_base() -> usize {
    CLINT_BASE.load(Ordering::Relaxed)
}

// ============================================================================
// CLINT Structure
// ============================================================================

/// CLINT interface
#[derive(Debug)]
pub struct Clint {
    base: usize,
}

impl Clint {
    /// Create a new CLINT instance with the given base address
    ///
    /// # Safety
    /// The base address must point to valid CLINT registers.
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }

    /// Get CLINT for the current platform
    pub fn current() -> Self {
        Self {
            base: get_base(),
        }
    }

    // ========================================================================
    // MSIP (Software Interrupt) Operations
    // ========================================================================

    /// Get the MSIP register address for a hart
    #[inline]
    fn msip_addr(&self, hart_id: usize) -> *mut u32 {
        (self.base + MSIP_OFFSET + hart_id * 4) as *mut u32
    }

    /// Read the MSIP register for a hart
    #[inline]
    pub fn read_msip(&self, hart_id: usize) -> u32 {
        unsafe { read_volatile(self.msip_addr(hart_id)) }
    }

    /// Write the MSIP register for a hart
    #[inline]
    pub fn write_msip(&self, hart_id: usize, value: u32) {
        unsafe { write_volatile(self.msip_addr(hart_id), value) }
    }

    /// Send a software interrupt (IPI) to a hart
    #[inline]
    pub fn send_ipi(&self, hart_id: usize) {
        self.write_msip(hart_id, 1);
    }

    /// Clear the software interrupt for a hart
    #[inline]
    pub fn clear_ipi(&self, hart_id: usize) {
        self.write_msip(hart_id, 0);
    }

    /// Check if software interrupt is pending for a hart
    #[inline]
    pub fn is_ipi_pending(&self, hart_id: usize) -> bool {
        self.read_msip(hart_id) != 0
    }

    // ========================================================================
    // MTIMECMP (Timer Compare) Operations
    // ========================================================================

    /// Get the MTIMECMP register address for a hart
    #[inline]
    fn mtimecmp_addr(&self, hart_id: usize) -> *mut u64 {
        (self.base + MTIMECMP_OFFSET + hart_id * 8) as *mut u64
    }

    /// Read the MTIMECMP register for a hart
    #[inline]
    pub fn read_mtimecmp(&self, hart_id: usize) -> u64 {
        unsafe { read_volatile(self.mtimecmp_addr(hart_id)) }
    }

    /// Write the MTIMECMP register for a hart
    #[inline]
    pub fn write_mtimecmp(&self, hart_id: usize, value: u64) {
        unsafe { write_volatile(self.mtimecmp_addr(hart_id), value) }
    }

    /// Set a timer deadline for a hart
    ///
    /// Timer interrupt will fire when MTIME >= MTIMECMP
    #[inline]
    pub fn set_timer(&self, hart_id: usize, deadline: u64) {
        self.write_mtimecmp(hart_id, deadline);
    }

    /// Clear the timer (set to max value to prevent interrupt)
    #[inline]
    pub fn clear_timer(&self, hart_id: usize) {
        self.write_mtimecmp(hart_id, u64::MAX);
    }

    // ========================================================================
    // MTIME (Timer Value) Operations
    // ========================================================================

    /// Get the MTIME register address
    #[inline]
    fn mtime_addr(&self) -> *mut u64 {
        (self.base + MTIME_OFFSET) as *mut u64
    }

    /// Read the current timer value
    #[inline]
    pub fn read_mtime(&self) -> u64 {
        unsafe { read_volatile(self.mtime_addr()) }
    }

    /// Write the timer value (if supported)
    ///
    /// Note: Writing MTIME is typically not supported on most platforms.
    #[inline]
    pub fn write_mtime(&self, value: u64) {
        unsafe { write_volatile(self.mtime_addr(), value) }
    }

    /// Get the current time
    pub fn get_time(&self) -> u64 {
        self.read_mtime()
    }

    // ========================================================================
    // Timer Helpers
    // ========================================================================

    /// Set a relative timer (deadline = now + ticks)
    pub fn set_timer_relative(&self, hart_id: usize, ticks: u64) {
        let deadline = self.read_mtime().saturating_add(ticks);
        self.set_timer(hart_id, deadline);
    }

    /// Get time until timer expires (0 if already expired)
    pub fn time_until_deadline(&self, hart_id: usize) -> u64 {
        let now = self.read_mtime();
        let deadline = self.read_mtimecmp(hart_id);
        deadline.saturating_sub(now)
    }

    /// Check if timer has expired
    pub fn is_timer_expired(&self, hart_id: usize) -> bool {
        self.read_mtime() >= self.read_mtimecmp(hart_id)
    }
}

// ============================================================================
// Global Functions (use current CLINT)
// ============================================================================

/// Read the current timer value
#[inline]
pub fn read_time() -> u64 {
    // Use TIME CSR if available (trap to M-mode)
    unsafe {
        let time: u64;
        core::arch::asm!("rdtime {}", out(reg) time, options(nomem, nostack));
        time
    }
}

/// Read the cycle counter
#[inline]
pub fn read_cycle() -> u64 {
    unsafe {
        let cycle: u64;
        core::arch::asm!("rdcycle {}", out(reg) cycle, options(nomem, nostack));
        cycle
    }
}

/// Read the instruction retired counter
#[inline]
pub fn read_instret() -> u64 {
    unsafe {
        let instret: u64;
        core::arch::asm!("rdinstret {}", out(reg) instret, options(nomem, nostack));
        instret
    }
}

/// Send a software interrupt to a hart
///
/// Note: In S-mode, this goes through SBI.
pub fn send_software_interrupt(hart_id: usize) {
    // For direct CLINT access
    let clint = Clint::current();
    clint.send_ipi(hart_id);
}

/// Clear the software interrupt for the current hart
pub fn clear_software_interrupt(hart_id: usize) {
    let clint = Clint::current();
    clint.clear_ipi(hart_id);
}

/// Set a timer deadline
///
/// Note: In S-mode, this goes through SBI.
pub fn set_timer(hart_id: usize, deadline: u64) {
    let clint = Clint::current();
    clint.set_timer(hart_id, deadline);
}

/// Clear the timer (disable timer interrupt)
pub fn clear_timer(hart_id: usize) {
    let clint = Clint::current();
    clint.clear_timer(hart_id);
}

/// Set a relative timer
pub fn set_timer_relative(hart_id: usize, ticks: u64) {
    let clint = Clint::current();
    clint.set_timer_relative(hart_id, ticks);
}

// ============================================================================
// SBI-based Timer Interface
// ============================================================================

/// SBI extension IDs
mod sbi {
    /// Timer extension
    pub const TIMER_EID: usize = 0x54494D45;
    /// IPI extension
    pub const IPI_EID: usize = 0x735049;
}

/// Set timer via SBI call
///
/// This is the preferred method in S-mode.
#[inline]
pub fn sbi_set_timer(stime_value: u64) {
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") sbi::TIMER_EID,
            in("a6") 0, // FID = 0 for set_timer
            in("a0") stime_value,
            options(nomem, nostack)
        );
    }
}

/// Send IPI via SBI call
///
/// This is the preferred method in S-mode.
#[inline]
pub fn sbi_send_ipi(hart_mask: u64, hart_mask_base: u64) {
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") sbi::IPI_EID,
            in("a6") 0, // FID = 0 for send_ipi
            in("a0") hart_mask,
            in("a1") hart_mask_base,
            options(nomem, nostack)
        );
    }
}

// ============================================================================
// Timer Calibration
// ============================================================================

/// Timer frequency (ticks per second)
/// Default is 10 MHz for QEMU
static TIMER_FREQUENCY: AtomicUsize = AtomicUsize::new(10_000_000);

/// Set the timer frequency
pub fn set_timer_frequency(freq: usize) {
    TIMER_FREQUENCY.store(freq, Ordering::SeqCst);
}

/// Get the timer frequency
pub fn get_timer_frequency() -> usize {
    TIMER_FREQUENCY.load(Ordering::Relaxed)
}

/// Convert microseconds to ticks
pub fn us_to_ticks(us: u64) -> u64 {
    let freq = get_timer_frequency() as u64;
    (us * freq) / 1_000_000
}

/// Convert milliseconds to ticks
pub fn ms_to_ticks(ms: u64) -> u64 {
    let freq = get_timer_frequency() as u64;
    (ms * freq) / 1_000
}

/// Convert seconds to ticks
pub fn s_to_ticks(s: u64) -> u64 {
    let freq = get_timer_frequency() as u64;
    s * freq
}

/// Convert ticks to microseconds
pub fn ticks_to_us(ticks: u64) -> u64 {
    let freq = get_timer_frequency() as u64;
    (ticks * 1_000_000) / freq
}

/// Convert ticks to milliseconds
pub fn ticks_to_ms(ticks: u64) -> u64 {
    let freq = get_timer_frequency() as u64;
    (ticks * 1_000) / freq
}

/// Convert ticks to seconds
pub fn ticks_to_s(ticks: u64) -> u64 {
    let freq = get_timer_frequency() as u64;
    ticks / freq
}

// ============================================================================
// Delay Functions
// ============================================================================

/// Busy-wait delay for a number of ticks
pub fn delay_ticks(ticks: u64) {
    let start = read_time();
    while read_time() < start.saturating_add(ticks) {
        core::hint::spin_loop();
    }
}

/// Busy-wait delay for microseconds
pub fn delay_us(us: u64) {
    delay_ticks(us_to_ticks(us));
}

/// Busy-wait delay for milliseconds
pub fn delay_ms(ms: u64) {
    delay_ticks(ms_to_ticks(ms));
}

// ============================================================================
// Timer Context for Context Switching
// ============================================================================

/// Timer state for a hart
#[derive(Debug, Clone, Copy)]
pub struct TimerState {
    /// Next timer deadline
    pub deadline: u64,
    /// Timer enabled
    pub enabled: bool,
    /// Time quantum for scheduling
    pub quantum_ticks: u64,
}

impl TimerState {
    /// Create new timer state
    pub const fn new() -> Self {
        Self {
            deadline: u64::MAX,
            enabled: false,
            quantum_ticks: 0,
        }
    }

    /// Save current timer state
    pub fn save(hart_id: usize) -> Self {
        let clint = Clint::current();
        Self {
            deadline: clint.read_mtimecmp(hart_id),
            enabled: true, // Assume enabled if we're saving
            quantum_ticks: 0, // Not tracked in hardware
        }
    }

    /// Restore timer state
    pub fn restore(&self, hart_id: usize) {
        let clint = Clint::current();
        if self.enabled {
            clint.set_timer(hart_id, self.deadline);
        } else {
            clint.clear_timer(hart_id);
        }
    }
}

impl Default for TimerState {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Multi-Hart Timer Coordination
// ============================================================================

/// Timer coordinator for multi-hart systems
pub struct TimerCoordinator {
    base: usize,
}

impl TimerCoordinator {
    /// Create a new coordinator
    ///
    /// # Safety
    /// Base must be valid CLINT address.
    pub const unsafe fn new(base: usize) -> Self {
        Self { base }
    }

    /// Synchronize time across all harts
    ///
    /// This is typically called during boot.
    pub fn synchronize_harts(&self, hart_count: usize) {
        let clint = unsafe { Clint::new(self.base) };
        let now = clint.read_mtime();

        // Set all harts to fire at the same time
        for hart in 0..hart_count {
            clint.set_timer(hart, now.saturating_add(ms_to_ticks(100)));
        }
    }

    /// Check if all harts have reached their deadline
    pub fn all_harts_expired(&self, hart_count: usize) -> bool {
        let clint = unsafe { Clint::new(self.base) };
        let now = clint.read_mtime();

        for hart in 0..hart_count {
            if now < clint.read_mtimecmp(hart) {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// CLINT Detection
// ============================================================================

/// Check if CLINT is present at the given address
///
/// # Safety
/// Address must be mapped and accessible.
pub unsafe fn probe_clint(base: usize) -> bool {
    let clint = Clint::new(base);

    // Try reading MTIME - should return a non-zero value
    // (unless system just started, but even then it should be readable)
    let mtime1 = clint.read_mtime();

    // Small delay
    for _ in 0..1000 {
        core::hint::spin_loop();
    }

    let mtime2 = clint.read_mtime();

    // Time should have advanced
    mtime2 >= mtime1
}
