//! # AArch64 PL011 UART Serial Driver
//!
//! Early boot serial console driver for ARM PL011 UART.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicU64, Ordering};

use super::*;
use crate::core::BootContext;
use crate::error::BootResult;

// =============================================================================
// PL011 REGISTERS
// =============================================================================

/// Data Register
pub const UARTDR: u64 = 0x000;
/// Receive Status Register / Error Clear Register
pub const UARTRSR: u64 = 0x004;
/// Flag Register
pub const UARTFR: u64 = 0x018;
/// IrDA Low-Power Counter Register
pub const UARTILPR: u64 = 0x020;
/// Integer Baud Rate Register
pub const UARTIBRD: u64 = 0x024;
/// Fractional Baud Rate Register
pub const UARTFBRD: u64 = 0x028;
/// Line Control Register
pub const UARTLCR_H: u64 = 0x02C;
/// Control Register
pub const UARTCR: u64 = 0x030;
/// Interrupt FIFO Level Select Register
pub const UARTIFLS: u64 = 0x034;
/// Interrupt Mask Set/Clear Register
pub const UARTIMSC: u64 = 0x038;
/// Raw Interrupt Status Register
pub const UARTRIS: u64 = 0x03C;
/// Masked Interrupt Status Register
pub const UARTMIS: u64 = 0x040;
/// Interrupt Clear Register
pub const UARTICR: u64 = 0x044;
/// DMA Control Register
pub const UARTDMACR: u64 = 0x048;
/// Peripheral Identification Register 0
pub const UARTPERIPHID0: u64 = 0xFE0;
/// Peripheral Identification Register 1
pub const UARTPERIPHID1: u64 = 0xFE4;
/// Peripheral Identification Register 2
pub const UARTPERIPHID2: u64 = 0xFE8;
/// Peripheral Identification Register 3
pub const UARTPERIPHID3: u64 = 0xFEC;
/// PrimeCell Identification Register 0
pub const UARTPCELLID0: u64 = 0xFF0;
/// PrimeCell Identification Register 1
pub const UARTPCELLID1: u64 = 0xFF4;
/// PrimeCell Identification Register 2
pub const UARTPCELLID2: u64 = 0xFF8;
/// PrimeCell Identification Register 3
pub const UARTPCELLID3: u64 = 0xFFC;

// =============================================================================
// FLAG REGISTER BITS
// =============================================================================

/// Clear to send
pub const FR_CTS: u32 = 1 << 0;
/// Data set ready
pub const FR_DSR: u32 = 1 << 1;
/// Data carrier detect
pub const FR_DCD: u32 = 1 << 2;
/// UART busy
pub const FR_BUSY: u32 = 1 << 3;
/// Receive FIFO empty
pub const FR_RXFE: u32 = 1 << 4;
/// Transmit FIFO full
pub const FR_TXFF: u32 = 1 << 5;
/// Receive FIFO full
pub const FR_RXFF: u32 = 1 << 6;
/// Transmit FIFO empty
pub const FR_TXFE: u32 = 1 << 7;
/// Ring indicator
pub const FR_RI: u32 = 1 << 8;

// =============================================================================
// CONTROL REGISTER BITS
// =============================================================================

/// UART enable
pub const CR_UARTEN: u32 = 1 << 0;
/// SIR enable
pub const CR_SIREN: u32 = 1 << 1;
/// SIR low-power mode
pub const CR_SIRLP: u32 = 1 << 2;
/// Loopback enable
pub const CR_LBE: u32 = 1 << 7;
/// Transmit enable
pub const CR_TXE: u32 = 1 << 8;
/// Receive enable
pub const CR_RXE: u32 = 1 << 9;
/// Data transmit ready
pub const CR_DTR: u32 = 1 << 10;
/// Request to send
pub const CR_RTS: u32 = 1 << 11;
/// Out1
pub const CR_OUT1: u32 = 1 << 12;
/// Out2
pub const CR_OUT2: u32 = 1 << 13;
/// RTS hardware flow control enable
pub const CR_RTSEN: u32 = 1 << 14;
/// CTS hardware flow control enable
pub const CR_CTSEN: u32 = 1 << 15;

// =============================================================================
// LINE CONTROL REGISTER BITS
// =============================================================================

/// Send break
pub const LCR_H_BRK: u32 = 1 << 0;
/// Parity enable
pub const LCR_H_PEN: u32 = 1 << 1;
/// Even parity select
pub const LCR_H_EPS: u32 = 1 << 2;
/// Two stop bits select
pub const LCR_H_STP2: u32 = 1 << 3;
/// Enable FIFOs
pub const LCR_H_FEN: u32 = 1 << 4;
/// Word length 5 bits
pub const LCR_H_WLEN_5: u32 = 0 << 5;
/// Word length 6 bits
pub const LCR_H_WLEN_6: u32 = 1 << 5;
/// Word length 7 bits
pub const LCR_H_WLEN_7: u32 = 2 << 5;
/// Word length 8 bits
pub const LCR_H_WLEN_8: u32 = 3 << 5;
/// Stick parity select
pub const LCR_H_SPS: u32 = 1 << 7;

// =============================================================================
// INTERRUPT BITS
// =============================================================================

/// Ring indicator modem interrupt
pub const INT_RI: u32 = 1 << 0;
/// CTS modem interrupt
pub const INT_CTS: u32 = 1 << 1;
/// DCD modem interrupt
pub const INT_DCD: u32 = 1 << 2;
/// DSR modem interrupt
pub const INT_DSR: u32 = 1 << 3;
/// Receive interrupt
pub const INT_RX: u32 = 1 << 4;
/// Transmit interrupt
pub const INT_TX: u32 = 1 << 5;
/// Receive timeout interrupt
pub const INT_RT: u32 = 1 << 6;
/// Framing error interrupt
pub const INT_FE: u32 = 1 << 7;
/// Parity error interrupt
pub const INT_PE: u32 = 1 << 8;
/// Break error interrupt
pub const INT_BE: u32 = 1 << 9;
/// Overrun error interrupt
pub const INT_OE: u32 = 1 << 10;
/// All interrupts
pub const INT_ALL: u32 = 0x7FF;

// =============================================================================
// ERROR REGISTER BITS
// =============================================================================

/// Framing error
pub const RSR_FE: u32 = 1 << 0;
/// Parity error
pub const RSR_PE: u32 = 1 << 1;
/// Break error
pub const RSR_BE: u32 = 1 << 2;
/// Overrun error
pub const RSR_OE: u32 = 1 << 3;

// =============================================================================
// UART CONFIGURATION
// =============================================================================

/// Common baud rates
pub mod baudrate {
    pub const B9600: u32 = 9600;
    pub const B19200: u32 = 19200;
    pub const B38400: u32 = 38400;
    pub const B57600: u32 = 57600;
    pub const B115200: u32 = 115200;
    pub const B230400: u32 = 230400;
    pub const B460800: u32 = 460800;
    pub const B921600: u32 = 921600;
}

/// Data bits
#[derive(Debug, Clone, Copy)]
pub enum DataBits {
    Five  = 0,
    Six   = 1,
    Seven = 2,
    Eight = 3,
}

/// Parity
#[derive(Debug, Clone, Copy)]
pub enum Parity {
    None,
    Even,
    Odd,
}

/// Stop bits
#[derive(Debug, Clone, Copy)]
pub enum StopBits {
    One,
    Two,
}

// =============================================================================
// DEFAULT ADDRESSES
// =============================================================================

/// QEMU virt machine UART0 base
pub const QEMU_UART0_BASE: u64 = 0x09000000;
/// QEMU virt machine UART clock (24 MHz)
pub const QEMU_UART_CLOCK: u32 = 24_000_000;

// =============================================================================
// GLOBAL UART STATE
// =============================================================================

/// Primary UART base address
static UART_BASE: AtomicU64 = AtomicU64::new(QEMU_UART0_BASE);
/// UART clock frequency
static UART_CLOCK: AtomicU64 = AtomicU64::new(QEMU_UART_CLOCK as u64);

// =============================================================================
// UART DRIVER
// =============================================================================

/// PL011 UART driver
pub struct Pl011 {
    base: u64,
    clock: u32,
}

impl Pl011 {
    /// Create new UART driver
    pub const fn new(base: u64, clock: u32) -> Self {
        Self { base, clock }
    }

    /// Read register
    #[inline]
    unsafe fn read(&self, offset: u64) -> u32 {
        core::ptr::read_volatile((self.base + offset) as *const u32)
    }

    /// Write register
    #[inline]
    unsafe fn write(&self, offset: u64, value: u32) {
        core::ptr::write_volatile((self.base + offset) as *mut u32, value);
    }

    /// Initialize UART with specified settings
    pub unsafe fn init(&self, baud: u32, data_bits: DataBits, parity: Parity, stop_bits: StopBits) {
        // Disable UART
        self.write(UARTCR, 0);

        // Wait for current transmission to complete
        while self.read(UARTFR) & FR_BUSY != 0 {
            core::hint::spin_loop();
        }

        // Flush FIFOs
        self.write(UARTLCR_H, 0);

        // Clear all interrupts
        self.write(UARTICR, INT_ALL);

        // Set baud rate
        // BAUDDIV = UARTCLK / (16 * baud)
        // IBRD = integer part
        // FBRD = fractional part * 64 + 0.5
        let bauddiv = (self.clock as u64 * 4) / baud as u64; // Scaled by 64
        let ibrd = (bauddiv / 64) as u32;
        let fbrd = (bauddiv % 64) as u32;

        self.write(UARTIBRD, ibrd);
        self.write(UARTFBRD, fbrd);

        // Set line control
        let mut lcr = match data_bits {
            DataBits::Five => LCR_H_WLEN_5,
            DataBits::Six => LCR_H_WLEN_6,
            DataBits::Seven => LCR_H_WLEN_7,
            DataBits::Eight => LCR_H_WLEN_8,
        };

        lcr |= LCR_H_FEN; // Enable FIFOs

        match parity {
            Parity::None => {},
            Parity::Even => lcr |= LCR_H_PEN | LCR_H_EPS,
            Parity::Odd => lcr |= LCR_H_PEN,
        }

        if matches!(stop_bits, StopBits::Two) {
            lcr |= LCR_H_STP2;
        }

        self.write(UARTLCR_H, lcr);

        // Disable all interrupts
        self.write(UARTIMSC, 0);

        // Enable UART, TX, and RX
        self.write(UARTCR, CR_UARTEN | CR_TXE | CR_RXE);
    }

    /// Initialize with default settings (115200 8N1)
    pub unsafe fn init_default(&self) {
        self.init(
            baudrate::B115200,
            DataBits::Eight,
            Parity::None,
            StopBits::One,
        );
    }

    /// Check if transmit FIFO is full
    #[inline]
    pub unsafe fn tx_full(&self) -> bool {
        self.read(UARTFR) & FR_TXFF != 0
    }

    /// Check if transmit FIFO is empty
    #[inline]
    pub unsafe fn tx_empty(&self) -> bool {
        self.read(UARTFR) & FR_TXFE != 0
    }

    /// Check if receive FIFO is empty
    #[inline]
    pub unsafe fn rx_empty(&self) -> bool {
        self.read(UARTFR) & FR_RXFE != 0
    }

    /// Check if receive FIFO is full
    #[inline]
    pub unsafe fn rx_full(&self) -> bool {
        self.read(UARTFR) & FR_RXFF != 0
    }

    /// Check if UART is busy
    #[inline]
    pub unsafe fn busy(&self) -> bool {
        self.read(UARTFR) & FR_BUSY != 0
    }

    /// Write a byte (blocking)
    pub unsafe fn write_byte(&self, byte: u8) {
        // Wait for space in FIFO
        while self.tx_full() {
            core::hint::spin_loop();
        }
        self.write(UARTDR, byte as u32);
    }

    /// Write a byte (non-blocking)
    pub unsafe fn try_write_byte(&self, byte: u8) -> bool {
        if self.tx_full() {
            return false;
        }
        self.write(UARTDR, byte as u32);
        true
    }

    /// Read a byte (blocking)
    pub unsafe fn read_byte(&self) -> u8 {
        // Wait for data
        while self.rx_empty() {
            core::hint::spin_loop();
        }
        (self.read(UARTDR) & 0xFF) as u8
    }

    /// Read a byte (non-blocking)
    pub unsafe fn try_read_byte(&self) -> Option<u8> {
        if self.rx_empty() {
            return None;
        }
        Some((self.read(UARTDR) & 0xFF) as u8)
    }

    /// Write a string
    pub unsafe fn write_str(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }

    /// Write bytes
    pub unsafe fn write_bytes(&self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_byte(byte);
        }
    }

    /// Flush transmit FIFO
    pub unsafe fn flush(&self) {
        while !self.tx_empty() || self.busy() {
            core::hint::spin_loop();
        }
    }

    /// Get receive errors
    pub unsafe fn get_errors(&self) -> u32 {
        self.read(UARTRSR)
    }

    /// Clear receive errors
    pub unsafe fn clear_errors(&self) {
        self.write(UARTRSR, 0);
    }

    /// Enable interrupt
    pub unsafe fn enable_interrupt(&self, mask: u32) {
        let current = self.read(UARTIMSC);
        self.write(UARTIMSC, current | mask);
    }

    /// Disable interrupt
    pub unsafe fn disable_interrupt(&self, mask: u32) {
        let current = self.read(UARTIMSC);
        self.write(UARTIMSC, current & !mask);
    }

    /// Get pending interrupts
    pub unsafe fn get_interrupts(&self) -> u32 {
        self.read(UARTMIS)
    }

    /// Clear interrupts
    pub unsafe fn clear_interrupts(&self, mask: u32) {
        self.write(UARTICR, mask);
    }

    /// Set FIFO levels for interrupts
    pub unsafe fn set_fifo_levels(&self, tx_level: u8, rx_level: u8) {
        let ifls = ((tx_level as u32) & 0x7) | (((rx_level as u32) & 0x7) << 3);
        self.write(UARTIFLS, ifls);
    }

    /// Get peripheral ID
    pub unsafe fn get_periph_id(&self) -> u32 {
        let p0 = self.read(UARTPERIPHID0) & 0xFF;
        let p1 = self.read(UARTPERIPHID1) & 0xFF;
        let p2 = self.read(UARTPERIPHID2) & 0xFF;
        let p3 = self.read(UARTPERIPHID3) & 0xFF;
        p0 | (p1 << 8) | (p2 << 16) | (p3 << 24)
    }
}

impl Write for Pl011 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            Pl011::write_str(self, s);
        }
        Ok(())
    }
}

// =============================================================================
// GLOBAL UART FUNCTIONS
// =============================================================================

/// Initialize global UART
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Get UART address from device tree or use default
    let base = if let Some(ref dt_info) = ctx.boot_info.device_tree {
        // TODO: Parse device tree for UART address
        QEMU_UART0_BASE
    } else {
        QEMU_UART0_BASE
    };

    UART_BASE.store(base, Ordering::SeqCst);
    UART_CLOCK.store(QEMU_UART_CLOCK as u64, Ordering::SeqCst);

    let uart = Pl011::new(base, QEMU_UART_CLOCK);
    uart.init_default();

    Ok(())
}

/// Get global UART
pub fn get_uart() -> Pl011 {
    Pl011::new(
        UART_BASE.load(Ordering::SeqCst),
        UART_CLOCK.load(Ordering::SeqCst) as u32,
    )
}

/// Write byte to global UART
pub fn write_byte(byte: u8) {
    unsafe {
        get_uart().write_byte(byte);
    }
}

/// Write string to global UART
pub fn write_str(s: &str) {
    unsafe {
        get_uart().write_str(s);
    }
}

/// Read byte from global UART (blocking)
pub fn read_byte() -> u8 {
    unsafe { get_uart().read_byte() }
}

/// Try to read byte from global UART (non-blocking)
pub fn try_read_byte() -> Option<u8> {
    unsafe { get_uart().try_read_byte() }
}

// =============================================================================
// LOGGING
// =============================================================================

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info  = 2,
    Warn  = 3,
    Error = 4,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Serial writer for fmt::Write
pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write_str(s);
        Ok(())
    }
}

/// Log message
pub fn log(level: LogLevel, args: fmt::Arguments) {
    let uart = get_uart();
    unsafe {
        uart.write_str("[");
        uart.write_str(level.as_str());
        uart.write_str("] ");

        let mut writer = SerialWriter;
        let _ = writer.write_fmt(args);

        uart.write_str("\n");
    }
}

/// Print macro (no newline)
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::arch::aarch64::serial::write_str(&alloc::format!($($arg)*))
    };
}

/// Println macro
#[macro_export]
macro_rules! serial_println {
    () => { $crate::arch::aarch64::serial::write_str("\n") };
    ($($arg:tt)*) => {
        $crate::arch::aarch64::serial::write_str(&alloc::format!("{}\n", format_args!($($arg)*)))
    };
}

/// Trace log
#[macro_export]
macro_rules! log_trace_arm {
    ($($arg:tt)*) => {
        $crate::arch::aarch64::serial::log(
            $crate::arch::aarch64::serial::LogLevel::Trace,
            format_args!($($arg)*)
        )
    };
}

/// Debug log
#[macro_export]
macro_rules! log_debug_arm {
    ($($arg:tt)*) => {
        $crate::arch::aarch64::serial::log(
            $crate::arch::aarch64::serial::LogLevel::Debug,
            format_args!($($arg)*)
        )
    };
}

/// Info log
#[macro_export]
macro_rules! log_info_arm {
    ($($arg:tt)*) => {
        $crate::arch::aarch64::serial::log(
            $crate::arch::aarch64::serial::LogLevel::Info,
            format_args!($($arg)*)
        )
    };
}

/// Warn log
#[macro_export]
macro_rules! log_warn_arm {
    ($($arg:tt)*) => {
        $crate::arch::aarch64::serial::log(
            $crate::arch::aarch64::serial::LogLevel::Warn,
            format_args!($($arg)*)
        )
    };
}

/// Error log
#[macro_export]
macro_rules! log_error_arm {
    ($($arg:tt)*) => {
        $crate::arch::aarch64::serial::log(
            $crate::arch::aarch64::serial::LogLevel::Error,
            format_args!($($arg)*)
        )
    };
}

// =============================================================================
// CONSOLE
// =============================================================================

/// Early console output
pub fn early_print(s: &str) {
    write_str(s);
}

/// Early console output with newline
pub fn early_println(s: &str) {
    write_str(s);
    write_byte(b'\n');
}

/// Print hex value
pub fn print_hex(value: u64) {
    let uart = get_uart();
    unsafe {
        uart.write_str("0x");
        for i in (0..16).rev() {
            let nibble = ((value >> (i * 4)) & 0xF) as u8;
            let c = if nibble < 10 {
                b'0' + nibble
            } else {
                b'a' + nibble - 10
            };
            uart.write_byte(c);
        }
    }
}

/// Print decimal value
pub fn print_dec(mut value: u64) {
    let uart = get_uart();
    if value == 0 {
        unsafe {
            uart.write_byte(b'0');
        }
        return;
    }

    let mut buf = [0u8; 20];
    let mut i = 0;

    while value > 0 {
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
        i += 1;
    }

    unsafe {
        while i > 0 {
            i -= 1;
            uart.write_byte(buf[i]);
        }
    }
}

// =============================================================================
// PANIC HANDLER SUPPORT
// =============================================================================

/// Panic output (does not depend on allocator)
pub fn panic_print(s: &str) {
    let uart = get_uart();
    unsafe {
        uart.write_str(s);
    }
}

/// Print panic info
pub fn panic_info(location: Option<&core::panic::Location>, msg: &str) {
    let uart = get_uart();
    unsafe {
        uart.write_str("\n!!! PANIC !!!\n");

        if let Some(loc) = location {
            uart.write_str("Location: ");
            uart.write_str(loc.file());
            uart.write_str(":");
            print_dec(loc.line() as u64);
            uart.write_str("\n");
        }

        uart.write_str("Message: ");
        uart.write_str(msg);
        uart.write_str("\n");
    }
}
