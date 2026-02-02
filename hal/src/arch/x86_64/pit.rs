//! # Programmable Interval Timer (PIT)
//!
//! The 8253/8254 PIT is a legacy timer that generates periodic interrupts.
//! We use it for preemptive multitasking until APIC timer is available.
//!
//! ## Configuration
//!
//! - Channel 0: Connected to IRQ 0, used for system timer
//! - Frequency: Configurable (default 1000 Hz = 1ms ticks)
//! - Mode: Rate generator (square wave)

use core::sync::atomic::{AtomicU64, Ordering};

/// PIT I/O ports
mod ports {
    pub const CHANNEL_0: u16 = 0x40;
    pub const CHANNEL_1: u16 = 0x41;
    pub const CHANNEL_2: u16 = 0x42;
    pub const COMMAND: u16 = 0x43;
}

/// PIT internal oscillator frequency (1.193182 MHz)
pub const PIT_FREQUENCY: u32 = 1_193_182;

/// Default tick frequency (1000 Hz = 1ms per tick)
pub const DEFAULT_FREQUENCY: u32 = 1000;

/// Tick counter
static TICK_COUNT: AtomicU64 = AtomicU64::new(0);

/// Nanoseconds per tick (updated on init)
static NS_PER_TICK: AtomicU64 = AtomicU64::new(1_000_000); // 1ms default

/// PIT Command byte format:
/// Bits 7-6: Channel (0-2)
/// Bits 5-4: Access mode (3 = lobyte/hibyte)
/// Bits 3-1: Operating mode
/// Bit 0: Binary (0) or BCD (1)
mod command {
    pub const CHANNEL_0: u8 = 0b00_000000;
    pub const ACCESS_LOHI: u8 = 0b00_110000;
    pub const MODE_RATE_GEN: u8 = 0b00_000100; // Mode 2: Rate generator
    pub const MODE_SQUARE: u8 = 0b00_000110; // Mode 3: Square wave
    pub const BINARY: u8 = 0b00_000000;
}

/// Initialize the PIT with the specified frequency
///
/// # Safety
/// Must be called only once during early boot.
pub unsafe fn init(frequency: u32) {
    let divisor = PIT_FREQUENCY / frequency;
    let divisor = divisor.clamp(1, 65535) as u16;

    // Calculate actual frequency and nanoseconds per tick
    let actual_freq = PIT_FREQUENCY / divisor as u32;
    let ns_per_tick = 1_000_000_000u64 / actual_freq as u64;
    NS_PER_TICK.store(ns_per_tick, Ordering::Relaxed);

    let cmd = command::CHANNEL_0 | command::ACCESS_LOHI | command::MODE_RATE_GEN | command::BINARY;

    unsafe {
        // Send command byte
        outb(ports::COMMAND, cmd);

        // Send divisor (low byte first, then high byte)
        outb(ports::CHANNEL_0, divisor as u8);
        outb(ports::CHANNEL_0, (divisor >> 8) as u8);
    }

    log::info!(
        "PIT initialized: {} Hz (divisor={}, {}ns/tick)",
        actual_freq,
        divisor,
        ns_per_tick
    );
}

/// Initialize with default 1000 Hz frequency
///
/// # Safety
/// Must be called only once during early boot.
pub unsafe fn init_default() {
    unsafe {
        init(DEFAULT_FREQUENCY);
    }
}

/// Handle a timer tick interrupt
///
/// Called from the IRQ handler. Returns the current tick count.
pub fn tick() -> u64 {
    TICK_COUNT.fetch_add(1, Ordering::Relaxed) + 1
}

/// Get the current tick count
pub fn ticks() -> u64 {
    TICK_COUNT.load(Ordering::Relaxed)
}

/// Get nanoseconds since boot (approximate)
pub fn uptime_ns() -> u64 {
    ticks() * NS_PER_TICK.load(Ordering::Relaxed)
}

/// Get milliseconds since boot
pub fn uptime_ms() -> u64 {
    uptime_ns() / 1_000_000
}

/// Get seconds since boot
pub fn uptime_secs() -> u64 {
    uptime_ns() / 1_000_000_000
}

/// Write a byte to an I/O port
#[inline]
unsafe fn outb(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
}

/// Read a byte from an I/O port
#[inline]
#[allow(dead_code)]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }
    value
}
