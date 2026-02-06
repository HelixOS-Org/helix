//! # High Precision Event Timer (HPET)
//!
//! The HPET provides a set of timers that can generate interrupts at
//! programmable intervals. It offers higher resolution than the legacy
//! PIT timer.
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │                            HPET                                 │
//! ├────────────────────────────────────────────────────────────────┤
//! │  ┌──────────────────────────────────────────────────────────┐  │
//! │  │                   Main Counter (64-bit)                   │  │
//! │  │                   Counts at ~10-25 MHz                    │  │
//! │  └──────────────────────────────────────────────────────────┘  │
//! │                              │                                  │
//! │          ┌───────────────────┼───────────────────┐             │
//! │          │                   │                   │              │
//! │     ┌────┴────┐         ┌────┴────┐         ┌────┴────┐        │
//! │     │ Timer 0 │         │ Timer 1 │         │ Timer N │        │
//! │     │         │         │         │         │         │        │
//! │     │Comparator         │Comparator         │Comparator        │
//! │     │ + IRQ   │         │ + IRQ   │         │ + IRQ   │        │
//! │     └─────────┘         └─────────┘         └─────────┘        │
//! │                                                                 │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Features
//!
//! - 64-bit main counter (may wrap)
//! - Multiple comparators (typically 3-8)
//! - Each comparator can generate an interrupt
//! - Periodic and one-shot modes
//! - FSB interrupt delivery (MSI)

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// =============================================================================
// Constants
// =============================================================================

/// Default HPET base address (from ACPI)
pub const HPET_BASE_DEFAULT: u64 = 0xFED0_0000;

/// HPET register offsets
pub mod regs {
    /// General Capabilities and ID Register
    pub const GCAP_ID: u64 = 0x000;
    /// General Configuration Register
    pub const GEN_CONF: u64 = 0x010;
    /// General Interrupt Status Register
    pub const GEN_INT_STS: u64 = 0x020;
    /// Main Counter Value Register
    pub const MAIN_CNT: u64 = 0x0F0;

    /// Timer N Configuration and Capability Register
    pub const fn timer_conf(n: u8) -> u64 {
        0x100 + (n as u64) * 0x20
    }

    /// Timer N Comparator Value Register
    pub const fn timer_comp(n: u8) -> u64 {
        0x108 + (n as u64) * 0x20
    }

    /// Timer N FSB Interrupt Route Register
    pub const fn timer_fsb(n: u8) -> u64 {
        0x110 + (n as u64) * 0x20
    }
}

// =============================================================================
// Global State
// =============================================================================

/// HPET available
static HPET_AVAILABLE: AtomicBool = AtomicBool::new(false);

/// HPET base address
static HPET_BASE: AtomicU64 = AtomicU64::new(0);

/// HPET period in femtoseconds
static HPET_PERIOD_FS: AtomicU64 = AtomicU64::new(0);

/// HPET frequency in Hz
static HPET_FREQUENCY: AtomicU64 = AtomicU64::new(0);

/// Number of timers
static HPET_NUM_TIMERS: AtomicU64 = AtomicU64::new(0);

// =============================================================================
// Register Access
// =============================================================================

/// Read a 64-bit HPET register
#[inline]
unsafe fn read_reg(offset: u64) -> u64 {
    let base = HPET_BASE.load(Ordering::Relaxed);
    unsafe { core::ptr::read_volatile((base + offset) as *const u64) }
}

/// Write a 64-bit HPET register
#[inline]
unsafe fn write_reg(offset: u64, value: u64) {
    let base = HPET_BASE.load(Ordering::Relaxed);
    unsafe { core::ptr::write_volatile((base + offset) as *mut u64, value) };
}

/// Read a 32-bit HPET register
#[inline]
unsafe fn read_reg32(offset: u64) -> u32 {
    let base = HPET_BASE.load(Ordering::Relaxed);
    unsafe { core::ptr::read_volatile((base + offset) as *const u32) }
}

/// Write a 32-bit HPET register
#[inline]
unsafe fn write_reg32(offset: u64, value: u32) {
    let base = HPET_BASE.load(Ordering::Relaxed);
    unsafe { core::ptr::write_volatile((base + offset) as *mut u32, value) };
}

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the HPET
///
/// # Safety
///
/// The base address must be a valid HPET MMIO mapping.
pub unsafe fn init(base: u64) -> Result<(), HpetError> {
    if HPET_AVAILABLE.load(Ordering::Acquire) {
        return Err(HpetError::AlreadyInitialized);
    }

    HPET_BASE.store(base, Ordering::SeqCst);

    // Read capabilities
    let gcap = unsafe { core::ptr::read_volatile(base as *const u64) };

    // Extract period (bits 63:32) in femtoseconds
    let period_fs = gcap >> 32;
    if period_fs == 0 || period_fs > 100_000_000 {
        return Err(HpetError::InvalidPeriod);
    }

    HPET_PERIOD_FS.store(period_fs, Ordering::SeqCst);

    // Calculate frequency (10^15 fs per second)
    let frequency = 1_000_000_000_000_000 / period_fs;
    HPET_FREQUENCY.store(frequency, Ordering::SeqCst);

    // Extract number of timers (bits 12:8)
    let num_timers = ((gcap >> 8) & 0x1F) + 1;
    HPET_NUM_TIMERS.store(num_timers, Ordering::SeqCst);

    // Enable the main counter
    let gen_conf = unsafe { read_reg(regs::GEN_CONF) };
    unsafe { write_reg(regs::GEN_CONF, gen_conf | 1) }; // Set ENABLE_CNF bit

    HPET_AVAILABLE.store(true, Ordering::SeqCst);

    log::info!(
        "HPET: Initialized at {:#x} ({} Hz, {} timers)",
        base,
        frequency,
        num_timers
    );

    Ok(())
}

/// Check if HPET is available
#[inline]
pub fn is_available() -> bool {
    HPET_AVAILABLE.load(Ordering::Relaxed)
}

/// Get HPET frequency in Hz
#[inline]
pub fn frequency() -> u64 {
    HPET_FREQUENCY.load(Ordering::Relaxed)
}

/// Get number of available timers
#[inline]
pub fn num_timers() -> u8 {
    HPET_NUM_TIMERS.load(Ordering::Relaxed) as u8
}

// =============================================================================
// Counter Reading
// =============================================================================

/// Read the main counter value
#[inline]
pub fn read_counter() -> u64 {
    if !is_available() {
        return 0;
    }
    unsafe { read_reg(regs::MAIN_CNT) }
}

/// Read counter as nanoseconds
#[inline]
pub fn read_ns() -> u64 {
    let counter = read_counter();
    let period_fs = HPET_PERIOD_FS.load(Ordering::Relaxed);
    if period_fs == 0 {
        return 0;
    }
    // Convert femtoseconds to nanoseconds (divide by 10^6)
    (counter as u128 * period_fs as u128 / 1_000_000) as u64
}

/// Convert counter ticks to nanoseconds
#[inline]
pub fn ticks_to_ns(ticks: u64) -> u64 {
    let period_fs = HPET_PERIOD_FS.load(Ordering::Relaxed);
    if period_fs == 0 {
        return 0;
    }
    (ticks as u128 * period_fs as u128 / 1_000_000) as u64
}

/// Convert nanoseconds to counter ticks
#[inline]
pub fn ns_to_ticks(ns: u64) -> u64 {
    let period_fs = HPET_PERIOD_FS.load(Ordering::Relaxed);
    if period_fs == 0 {
        return 0;
    }
    (ns as u128 * 1_000_000 / period_fs as u128) as u64
}

// =============================================================================
// Error Type
// =============================================================================

/// HPET error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HpetError {
    /// HPET not available
    NotAvailable,
    /// HPET already initialized
    AlreadyInitialized,
    /// Invalid period value
    InvalidPeriod,
    /// Timer not available
    TimerNotAvailable,
    /// Invalid configuration
    InvalidConfiguration,
}

impl core::fmt::Display for HpetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            HpetError::NotAvailable => write!(f, "HPET not available"),
            HpetError::AlreadyInitialized => write!(f, "HPET already initialized"),
            HpetError::InvalidPeriod => write!(f, "Invalid HPET period"),
            HpetError::TimerNotAvailable => write!(f, "HPET timer not available"),
            HpetError::InvalidConfiguration => write!(f, "Invalid HPET configuration"),
        }
    }
}

// =============================================================================
// HPET Structure
// =============================================================================

/// HPET controller
pub struct Hpet {
    base: u64,
    frequency: u64,
    num_timers: u8,
}

impl Hpet {
    /// Get the global HPET instance
    pub fn get() -> Option<Self> {
        if !is_available() {
            return None;
        }

        Some(Self {
            base: HPET_BASE.load(Ordering::Relaxed),
            frequency: HPET_FREQUENCY.load(Ordering::Relaxed),
            num_timers: HPET_NUM_TIMERS.load(Ordering::Relaxed) as u8,
        })
    }

    /// Get the frequency
    #[inline]
    pub fn frequency(&self) -> u64 {
        self.frequency
    }

    /// Get the number of timers
    #[inline]
    pub fn num_timers(&self) -> u8 {
        self.num_timers
    }

    /// Read the main counter
    #[inline]
    pub fn read(&self) -> u64 {
        read_counter()
    }

    /// Get a timer
    pub fn timer(&self, index: u8) -> Option<HpetTimer> {
        if index >= self.num_timers {
            return None;
        }
        Some(HpetTimer::new(self.base, index))
    }

    /// Delay for nanoseconds
    pub fn delay_ns(&self, ns: u64) {
        let start = self.read();
        let ticks = ns_to_ticks(ns);
        while self.read().wrapping_sub(start) < ticks {
            core::hint::spin_loop();
        }
    }

    /// Delay for microseconds
    #[inline]
    pub fn delay_us(&self, us: u64) {
        self.delay_ns(us * 1_000);
    }

    /// Delay for milliseconds
    #[inline]
    pub fn delay_ms(&self, ms: u64) {
        self.delay_ns(ms * 1_000_000);
    }
}

// =============================================================================
// HPET Timer
// =============================================================================

/// Timer mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HpetTimerMode {
    /// One-shot mode
    OneShot,
    /// Periodic mode
    Periodic,
}

/// Timer interrupt delivery mode
///
/// Specifies how HPET timer interrupts are routed to the processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HpetTimerDelivery {
    /// Legacy I/O APIC routing
    ///
    /// Interrupt is routed through the I/O APIC using the specified IRQ line.
    IoApic(u8),
    /// FSB (Front Side Bus) / MSI delivery
    ///
    /// Interrupt is delivered directly to the processor using Message Signaled Interrupts.
    Fsb {
        /// MSI address (typically contains destination APIC ID and delivery mode)
        address: u64,
        /// MSI data (typically contains the interrupt vector)
        data: u32,
    },
}

/// Individual HPET timer
///
/// Represents a single comparator within the HPET hardware.
/// Each timer can generate interrupts when the main counter
/// matches its comparator value.
pub struct HpetTimer {
    /// Base MMIO address of the HPET registers
    base: u64,
    /// Timer index (0 to N-1 where N is the number of timers)
    index: u8,
}

impl HpetTimer {
    /// Create a new timer reference
    fn new(base: u64, index: u8) -> Self {
        Self { base, index }
    }

    /// Get timer index
    #[inline]
    pub fn index(&self) -> u8 {
        self.index
    }

    /// Read timer configuration
    pub fn read_config(&self) -> HpetTimerConfig {
        let value = unsafe { read_reg(regs::timer_conf(self.index)) };
        HpetTimerConfig(value)
    }

    /// Write timer configuration
    pub fn write_config(&self, config: HpetTimerConfig) {
        unsafe { write_reg(regs::timer_conf(self.index), config.0) };
    }

    /// Read comparator value
    pub fn read_comparator(&self) -> u64 {
        unsafe { read_reg(regs::timer_comp(self.index)) }
    }

    /// Write comparator value
    pub fn write_comparator(&self, value: u64) {
        unsafe { write_reg(regs::timer_comp(self.index), value) };
    }

    /// Check if periodic mode is supported
    pub fn supports_periodic(&self) -> bool {
        self.read_config().periodic_capable()
    }

    /// Check if FSB delivery is supported
    pub fn supports_fsb(&self) -> bool {
        self.read_config().fsb_capable()
    }

    /// Get allowed IRQ routing bitmap
    pub fn allowed_irqs(&self) -> u32 {
        (self.read_config().0 >> 32) as u32
    }

    /// Configure one-shot timer
    pub fn configure_oneshot(&self, comparator: u64, irq: u8) -> Result<(), HpetError> {
        let mut config = self.read_config();

        // Check IRQ is allowed
        if self.allowed_irqs() & (1 << irq) == 0 {
            return Err(HpetError::InvalidConfiguration);
        }

        config.set_irq(irq);
        config.set_periodic(false);
        config.set_enabled(true);
        config.set_interrupt_enabled(true);

        self.write_comparator(comparator);
        self.write_config(config);

        Ok(())
    }

    /// Configure periodic timer
    pub fn configure_periodic(&self, period_ticks: u64, irq: u8) -> Result<(), HpetError> {
        if !self.supports_periodic() {
            return Err(HpetError::InvalidConfiguration);
        }

        let mut config = self.read_config();

        // Check IRQ is allowed
        if self.allowed_irqs() & (1 << irq) == 0 {
            return Err(HpetError::InvalidConfiguration);
        }

        config.set_irq(irq);
        config.set_periodic(true);
        config.set_val_set(true); // Allow direct period write
        config.set_enabled(true);
        config.set_interrupt_enabled(true);

        // For periodic mode, write period to comparator
        self.write_comparator(read_counter() + period_ticks);
        self.write_config(config);
        // Write period value
        self.write_comparator(period_ticks);

        Ok(())
    }

    /// Disable timer
    pub fn disable(&self) {
        let mut config = self.read_config();
        config.set_enabled(false);
        config.set_interrupt_enabled(false);
        self.write_config(config);
    }
}

// =============================================================================
// Timer Configuration
// =============================================================================

/// HPET timer configuration register
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct HpetTimerConfig(u64);

impl HpetTimerConfig {
    /// Check if level-triggered interrupts are supported
    #[inline]
    pub fn level_trigger_capable(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    /// Check if periodic mode is supported
    #[inline]
    pub fn periodic_capable(&self) -> bool {
        self.0 & (1 << 4) != 0
    }

    /// Check if 64-bit comparator is supported
    #[inline]
    pub fn size_64bit(&self) -> bool {
        self.0 & (1 << 5) != 0
    }

    /// Check if FSB delivery is supported
    #[inline]
    pub fn fsb_capable(&self) -> bool {
        self.0 & (1 << 15) != 0
    }

    /// Get IRQ routing
    #[inline]
    pub fn irq(&self) -> u8 {
        ((self.0 >> 9) & 0x1F) as u8
    }

    /// Set IRQ routing
    #[inline]
    pub fn set_irq(&mut self, irq: u8) {
        self.0 = (self.0 & !(0x1F << 9)) | ((irq as u64 & 0x1F) << 9);
    }

    /// Check if interrupt is enabled
    #[inline]
    pub fn interrupt_enabled(&self) -> bool {
        self.0 & (1 << 2) != 0
    }

    /// Set interrupt enabled
    #[inline]
    pub fn set_interrupt_enabled(&mut self, enabled: bool) {
        if enabled {
            self.0 |= 1 << 2;
        } else {
            self.0 &= !(1 << 2);
        }
    }

    /// Check if periodic mode
    #[inline]
    pub fn is_periodic(&self) -> bool {
        self.0 & (1 << 3) != 0
    }

    /// Set periodic mode
    #[inline]
    pub fn set_periodic(&mut self, periodic: bool) {
        if periodic {
            self.0 |= 1 << 3;
        } else {
            self.0 &= !(1 << 3);
        }
    }

    /// Set value set bit (for periodic accumulator write)
    #[inline]
    pub fn set_val_set(&mut self, val: bool) {
        if val {
            self.0 |= 1 << 6;
        } else {
            self.0 &= !(1 << 6);
        }
    }

    /// Check if 32-bit mode forced
    #[inline]
    pub fn force_32bit(&self) -> bool {
        self.0 & (1 << 8) != 0
    }

    /// Check if timer is enabled
    #[inline]
    pub fn is_enabled(&self) -> bool {
        // Timer enable is controlled via comparator write
        self.interrupt_enabled()
    }

    /// Set timer enabled
    #[inline]
    pub fn set_enabled(&mut self, _enabled: bool) {
        // Timer enable is implicit
    }

    /// Check if FSB routing enabled
    #[inline]
    pub fn fsb_enabled(&self) -> bool {
        self.0 & (1 << 14) != 0
    }

    /// Set FSB routing enabled
    #[inline]
    pub fn set_fsb_enabled(&mut self, enabled: bool) {
        if enabled {
            self.0 |= 1 << 14;
        } else {
            self.0 &= !(1 << 14);
        }
    }
}
