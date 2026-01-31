//! # Programmable Interval Timer (PIT)
//!
//! The 8253/8254 PIT is a legacy timer present in all x86 systems.
//! It's primarily used for early boot calibration before more
//! accurate timers are available.
//!
//! ## Channels
//!
//! - **Channel 0**: System timer (IRQ 0)
//! - **Channel 1**: DRAM refresh (legacy, not used)
//! - **Channel 2**: PC speaker / calibration
//!
//! ## Frequency
//!
//! The PIT runs at 1.193182 MHz (14.31818 MHz / 12).
//! With a 16-bit counter, this gives a maximum period of ~55ms.

use core::sync::atomic::{AtomicBool, AtomicU16, Ordering};

// =============================================================================
// Constants
// =============================================================================

/// PIT base frequency (1.193182 MHz)
pub const PIT_FREQUENCY: u64 = 1_193_182;

/// PIT I/O ports
mod ports {
    /// Channel 0 data port
    pub const CHANNEL0: u16 = 0x40;
    /// Channel 1 data port (legacy)
    pub const CHANNEL1: u16 = 0x41;
    /// Channel 2 data port
    pub const CHANNEL2: u16 = 0x42;
    /// Command/mode register
    pub const COMMAND: u16 = 0x43;
    /// Port B (speaker control, channel 2 gate)
    pub const PORT_B: u16 = 0x61;
}

/// Minimum divisor (for highest frequency)
pub const MIN_DIVISOR: u16 = 1;

/// Maximum divisor (0 means 65536)
pub const MAX_DIVISOR: u16 = 0;

/// Default divisor for ~1ms tick (1193)
pub const DEFAULT_DIVISOR: u16 = 1193;

// =============================================================================
// PIT Channel
// =============================================================================

/// PIT channel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PitChannel {
    /// Channel 0 - System timer (IRQ 0)
    Channel0 = 0b00,
    /// Channel 1 - DRAM refresh (not used)
    Channel1 = 0b01,
    /// Channel 2 - Speaker / calibration
    Channel2 = 0b10,
}

impl PitChannel {
    /// Get the data port for this channel
    #[inline]
    pub fn data_port(&self) -> u16 {
        match self {
            PitChannel::Channel0 => ports::CHANNEL0,
            PitChannel::Channel1 => ports::CHANNEL1,
            PitChannel::Channel2 => ports::CHANNEL2,
        }
    }
}

// =============================================================================
// PIT Mode
// =============================================================================

/// PIT operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PitMode {
    /// Mode 0: Interrupt on terminal count
    InterruptOnTerminalCount = 0b000,
    /// Mode 1: Hardware retriggerable one-shot
    HardwareOneShot = 0b001,
    /// Mode 2: Rate generator (for periodic interrupts)
    RateGenerator = 0b010,
    /// Mode 3: Square wave generator
    SquareWave = 0b011,
    /// Mode 4: Software triggered strobe
    SoftwareStrobe = 0b100,
    /// Mode 5: Hardware triggered strobe
    HardwareStrobe = 0b101,
}

/// Access mode for counter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PitAccess {
    /// Latch count value command
    Latch = 0b00,
    /// Low byte only
    LowByte = 0b01,
    /// High byte only
    HighByte = 0b10,
    /// Low byte then high byte
    LowHigh = 0b11,
}

// =============================================================================
// Port I/O
// =============================================================================

#[inline]
unsafe fn outb(port: u16, value: u8) {
    core::arch::asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nostack, nomem, preserves_flags),
    );
}

#[inline]
unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    core::arch::asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nostack, nomem, preserves_flags),
    );
    value
}

// =============================================================================
// Global State
// =============================================================================

/// PIT initialized
static PIT_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Current divisor for channel 0
static CHANNEL0_DIVISOR: AtomicU16 = AtomicU16::new(0);

// =============================================================================
// Initialization
// =============================================================================

/// Initialize the PIT
///
/// # Safety
///
/// Must be called during early boot.
pub unsafe fn init() {
    if PIT_INITIALIZED.swap(true, Ordering::SeqCst) {
        return;
    }

    // Disable channel 0 initially (set very low frequency)
    set_divisor(PitChannel::Channel0, 0); // 0 = 65536, ~18.2 Hz

    log::debug!("PIT: Initialized");
}

/// Set the divisor for a channel
///
/// # Safety
///
/// Must be called with interrupts disabled.
pub unsafe fn set_divisor(channel: PitChannel, divisor: u16) {
    // Build command byte
    let command = ((channel as u8) << 6)
        | ((PitAccess::LowHigh as u8) << 4)
        | ((PitMode::RateGenerator as u8) << 1);

    // Send command
    outb(ports::COMMAND, command);

    // Send divisor (low byte then high byte)
    outb(channel.data_port(), divisor as u8);
    outb(channel.data_port(), (divisor >> 8) as u8);

    if channel == PitChannel::Channel0 {
        CHANNEL0_DIVISOR.store(divisor, Ordering::SeqCst);
    }
}

/// Set the frequency for channel 0 (system timer)
///
/// Returns the actual frequency achieved.
///
/// # Safety
///
/// Must be called with interrupts disabled.
pub unsafe fn set_frequency(frequency_hz: u32) -> u32 {
    let divisor = if frequency_hz == 0 {
        0 // Maximum period
    } else {
        let d = PIT_FREQUENCY / frequency_hz as u64;
        if d > 65535 {
            0 // Use 65536
        } else if d == 0 {
            1
        } else {
            d as u16
        }
    };

    set_divisor(PitChannel::Channel0, divisor);

    // Calculate actual frequency
    let actual_divisor = if divisor == 0 { 65536u64 } else { divisor as u64 };
    (PIT_FREQUENCY / actual_divisor) as u32
}

/// Read the current count of a channel
///
/// # Safety
///
/// Should be called with interrupts disabled for accurate reading.
pub unsafe fn read_count(channel: PitChannel) -> u16 {
    // Send latch command
    let command = (channel as u8) << 6;
    outb(ports::COMMAND, command);

    // Read low byte then high byte
    let low = inb(channel.data_port()) as u16;
    let high = inb(channel.data_port()) as u16;

    low | (high << 8)
}

/// Get the current divisor for channel 0
pub fn get_channel0_divisor() -> u16 {
    CHANNEL0_DIVISOR.load(Ordering::Relaxed)
}

/// Calculate the frequency for a given divisor
pub fn divisor_to_frequency(divisor: u16) -> u64 {
    let d = if divisor == 0 { 65536u64 } else { divisor as u64 };
    PIT_FREQUENCY / d
}

/// Calculate the period in nanoseconds for a given divisor
pub fn divisor_to_period_ns(divisor: u16) -> u64 {
    let d = if divisor == 0 { 65536u64 } else { divisor as u64 };
    (d * 1_000_000_000) / PIT_FREQUENCY
}

// =============================================================================
// Channel 2 (Speaker / Calibration)
// =============================================================================

/// Enable channel 2 gate (for calibration)
///
/// # Safety
///
/// Modifies system I/O ports.
pub unsafe fn enable_channel2_gate() {
    let value = inb(ports::PORT_B);
    outb(ports::PORT_B, value | 0x01); // Set gate bit
}

/// Disable channel 2 gate
///
/// # Safety
///
/// Modifies system I/O ports.
pub unsafe fn disable_channel2_gate() {
    let value = inb(ports::PORT_B);
    outb(ports::PORT_B, value & !0x01); // Clear gate bit
}

/// Read channel 2 output status
pub fn read_channel2_output() -> bool {
    unsafe { inb(ports::PORT_B) & 0x20 != 0 }
}

/// Wait for a specified number of PIT ticks using channel 2
///
/// This is useful for calibration.
///
/// # Safety
///
/// Must be called with interrupts disabled.
pub unsafe fn wait_ticks(ticks: u16) {
    // Set up channel 2 in one-shot mode
    let command = ((PitChannel::Channel2 as u8) << 6)
        | ((PitAccess::LowHigh as u8) << 4)
        | ((PitMode::InterruptOnTerminalCount as u8) << 1);

    outb(ports::COMMAND, command);

    // Write count
    outb(ports::CHANNEL2, ticks as u8);
    outb(ports::CHANNEL2, (ticks >> 8) as u8);

    // Enable gate
    let port_b = inb(ports::PORT_B);
    outb(ports::PORT_B, (port_b & 0xFC) | 0x01);

    // Wait for output to go high
    while inb(ports::PORT_B) & 0x20 == 0 {
        core::hint::spin_loop();
    }

    // Disable gate
    outb(ports::PORT_B, port_b & 0xFC);
}

/// Wait for a specified time in microseconds
///
/// # Safety
///
/// Must be called with interrupts disabled.
pub unsafe fn wait_us(us: u64) {
    // Calculate number of ticks
    // ticks = us * PIT_FREQUENCY / 1_000_000
    let ticks = (us * PIT_FREQUENCY) / 1_000_000;

    if ticks == 0 {
        return;
    }

    // May need multiple waits for long durations
    let mut remaining = ticks;
    while remaining > 0 {
        let this_wait = if remaining > 65535 { 65535 } else { remaining as u16 };
        wait_ticks(this_wait);
        remaining -= this_wait as u64;
    }
}

/// Wait for a specified time in milliseconds
///
/// # Safety
///
/// Must be called with interrupts disabled.
pub unsafe fn wait_ms(ms: u64) {
    wait_us(ms * 1000);
}

// =============================================================================
// PIT Structure
// =============================================================================

/// PIT controller abstraction
pub struct Pit;

impl Pit {
    /// Create a new PIT instance
    pub const fn new() -> Self {
        Self
    }

    /// Initialize the PIT
    ///
    /// # Safety
    ///
    /// Must be called during early boot.
    pub unsafe fn init(&self) {
        init();
    }

    /// Set the system timer frequency (channel 0)
    ///
    /// # Safety
    ///
    /// Must be called with interrupts disabled.
    pub unsafe fn set_frequency(&self, frequency_hz: u32) -> u32 {
        set_frequency(frequency_hz)
    }

    /// Read the current count of channel 0
    ///
    /// # Safety
    ///
    /// Should be called with interrupts disabled.
    pub unsafe fn read_count(&self) -> u16 {
        read_count(PitChannel::Channel0)
    }

    /// Wait for microseconds using channel 2
    ///
    /// # Safety
    ///
    /// Must be called with interrupts disabled.
    pub unsafe fn wait_us(&self, us: u64) {
        wait_us(us);
    }

    /// Wait for milliseconds using channel 2
    ///
    /// # Safety
    ///
    /// Must be called with interrupts disabled.
    pub unsafe fn wait_ms(&self, ms: u64) {
        wait_ms(ms);
    }
}

impl Default for Pit {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Speaker Support (for debugging)
// =============================================================================

/// Enable the PC speaker with a specified frequency
///
/// # Safety
///
/// Modifies system I/O ports.
pub unsafe fn speaker_on(frequency_hz: u32) {
    if frequency_hz == 0 {
        speaker_off();
        return;
    }

    let divisor = PIT_FREQUENCY / frequency_hz as u64;
    let divisor = if divisor > 65535 { 65535 } else if divisor == 0 { 1 } else { divisor as u16 };

    // Configure channel 2 for square wave
    let command = ((PitChannel::Channel2 as u8) << 6)
        | ((PitAccess::LowHigh as u8) << 4)
        | ((PitMode::SquareWave as u8) << 1);

    outb(ports::COMMAND, command);
    outb(ports::CHANNEL2, divisor as u8);
    outb(ports::CHANNEL2, (divisor >> 8) as u8);

    // Enable speaker
    let port_b = inb(ports::PORT_B);
    outb(ports::PORT_B, port_b | 0x03);
}

/// Disable the PC speaker
///
/// # Safety
///
/// Modifies system I/O ports.
pub unsafe fn speaker_off() {
    let port_b = inb(ports::PORT_B);
    outb(ports::PORT_B, port_b & 0xFC);
}

/// Beep for a specified duration
///
/// # Safety
///
/// Must be called with interrupts disabled (for accurate timing).
pub unsafe fn beep(frequency_hz: u32, duration_ms: u64) {
    speaker_on(frequency_hz);
    wait_ms(duration_ms);
    speaker_off();
}
