//! # x86_64 Serial Port Driver
//!
//! Early boot serial console for debugging output.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, Ordering};

use super::{inb, outb};
use crate::error::{BootError, BootResult};
use crate::{Parity, SerialConfig};

// =============================================================================
// SERIAL PORT ADDRESSES
// =============================================================================

/// COM1 base address
pub const COM1_BASE: u16 = 0x3F8;
/// COM2 base address
pub const COM2_BASE: u16 = 0x2F8;
/// COM3 base address
pub const COM3_BASE: u16 = 0x3E8;
/// COM4 base address
pub const COM4_BASE: u16 = 0x2E8;

// =============================================================================
// SERIAL PORT REGISTERS (offsets from base)
// =============================================================================

/// Data register (read/write)
const REG_DATA: u16 = 0;
/// Interrupt Enable Register
const REG_IER: u16 = 1;
/// Divisor Latch Low (when DLAB=1)
const REG_DLL: u16 = 0;
/// Divisor Latch High (when DLAB=1)
const REG_DLH: u16 = 1;
/// FIFO Control Register (write only)
const REG_FCR: u16 = 2;
/// Interrupt Identification Register (read only)
const REG_IIR: u16 = 2;
/// Line Control Register
const REG_LCR: u16 = 3;
/// Modem Control Register
const REG_MCR: u16 = 4;
/// Line Status Register
const REG_LSR: u16 = 5;
/// Modem Status Register
const REG_MSR: u16 = 6;
/// Scratch Register
const REG_SR: u16 = 7;

// =============================================================================
// LINE STATUS REGISTER FLAGS
// =============================================================================

/// Data Ready
const LSR_DATA_READY: u8 = 1 << 0;
/// Overrun Error
const LSR_OVERRUN: u8 = 1 << 1;
/// Parity Error
const LSR_PARITY: u8 = 1 << 2;
/// Framing Error
const LSR_FRAMING: u8 = 1 << 3;
/// Break Interrupt
const LSR_BREAK: u8 = 1 << 4;
/// Transmitter Holding Register Empty
const LSR_THRE: u8 = 1 << 5;
/// Transmitter Empty
const LSR_TEMT: u8 = 1 << 6;
/// FIFO Error
const LSR_FIFO_ERR: u8 = 1 << 7;

// =============================================================================
// LINE CONTROL REGISTER FLAGS
// =============================================================================

/// Word length: 5 bits
const LCR_WORD_5: u8 = 0;
/// Word length: 6 bits
const LCR_WORD_6: u8 = 1;
/// Word length: 7 bits
const LCR_WORD_7: u8 = 2;
/// Word length: 8 bits
const LCR_WORD_8: u8 = 3;
/// Stop bits: 2 (1.5 for 5-bit word)
const LCR_STOP_2: u8 = 1 << 2;
/// Parity enable
const LCR_PARITY_EN: u8 = 1 << 3;
/// Even parity
const LCR_PARITY_EVEN: u8 = 1 << 4;
/// Stick parity
const LCR_PARITY_STICK: u8 = 1 << 5;
/// Set break
const LCR_BREAK: u8 = 1 << 6;
/// Divisor Latch Access Bit
const LCR_DLAB: u8 = 1 << 7;

// =============================================================================
// MODEM CONTROL REGISTER FLAGS
// =============================================================================

/// Data Terminal Ready
const MCR_DTR: u8 = 1 << 0;
/// Request to Send
const MCR_RTS: u8 = 1 << 1;
/// Output 1 (generic)
const MCR_OUT1: u8 = 1 << 2;
/// Output 2 (interrupt enable)
const MCR_OUT2: u8 = 1 << 3;
/// Loopback mode
const MCR_LOOPBACK: u8 = 1 << 4;

// =============================================================================
// FIFO CONTROL REGISTER FLAGS
// =============================================================================

/// Enable FIFOs
const FCR_ENABLE: u8 = 1 << 0;
/// Clear receive FIFO
const FCR_CLR_RX: u8 = 1 << 1;
/// Clear transmit FIFO
const FCR_CLR_TX: u8 = 1 << 2;
/// DMA mode select
const FCR_DMA: u8 = 1 << 3;
/// 64-byte FIFO enable (16750)
const FCR_64BYTE: u8 = 1 << 5;
/// Trigger level: 1 byte
const FCR_TRIG_1: u8 = 0 << 6;
/// Trigger level: 4 bytes
const FCR_TRIG_4: u8 = 1 << 6;
/// Trigger level: 8 bytes
const FCR_TRIG_8: u8 = 2 << 6;
/// Trigger level: 14 bytes
const FCR_TRIG_14: u8 = 3 << 6;

// =============================================================================
// SERIAL PORT STATE
// =============================================================================

/// Serial port is initialized
static SERIAL_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Current serial port base address
static mut SERIAL_BASE: u16 = COM1_BASE;

// =============================================================================
// SERIAL PORT IMPLEMENTATION
// =============================================================================

/// Serial port driver
pub struct SerialPort {
    /// Base I/O address
    base: u16,
}

impl SerialPort {
    /// Create a new serial port at the given base address
    pub const fn new(base: u16) -> Self {
        Self { base }
    }

    /// Read a register
    unsafe fn read_reg(&self, reg: u16) -> u8 {
        inb(self.base + reg)
    }

    /// Write a register
    unsafe fn write_reg(&self, reg: u16, value: u8) {
        outb(self.base + reg, value)
    }

    /// Check if port exists
    ///
    /// # Safety
    ///
    /// The caller must ensure hardware probing is safe at this point.
    pub unsafe fn exists(&self) -> bool {
        // Write to scratch register and read back
        self.write_reg(REG_SR, 0xAE);
        let val = self.read_reg(REG_SR);
        val == 0xAE
    }

    /// Initialize the serial port
    ///
    /// # Safety
    ///
    /// The caller must ensure system is in a valid state for initialization.
    pub unsafe fn init(&self, config: &SerialConfig) -> BootResult<()> {
        if !self.exists() {
            return Err(BootError::HardwareNotSupported);
        }

        // Disable interrupts
        self.write_reg(REG_IER, 0x00);

        // Calculate divisor for baud rate
        // Base clock is 115200 Hz
        let divisor = 115200u32 / config.baud_rate;

        // Set DLAB to access divisor registers
        self.write_reg(REG_LCR, LCR_DLAB);

        // Set divisor
        self.write_reg(REG_DLL, (divisor & 0xFF) as u8);
        self.write_reg(REG_DLH, ((divisor >> 8) & 0xFF) as u8);

        // Configure line control
        let mut lcr = match config.data_bits {
            5 => LCR_WORD_5,
            6 => LCR_WORD_6,
            7 => LCR_WORD_7,
            _ => LCR_WORD_8,
        };

        if config.stop_bits == 2 {
            lcr |= LCR_STOP_2;
        }

        match config.parity {
            Parity::None => {},
            Parity::Odd => lcr |= LCR_PARITY_EN,
            Parity::Even => lcr |= LCR_PARITY_EN | LCR_PARITY_EVEN,
            Parity::Mark => lcr |= LCR_PARITY_EN | LCR_PARITY_STICK,
            Parity::Space => lcr |= LCR_PARITY_EN | LCR_PARITY_EVEN | LCR_PARITY_STICK,
        }

        self.write_reg(REG_LCR, lcr);

        // Enable and clear FIFOs, set trigger level
        self.write_reg(REG_FCR, FCR_ENABLE | FCR_CLR_RX | FCR_CLR_TX | FCR_TRIG_14);

        // Set modem control (DTR, RTS, OUT2 for interrupts)
        self.write_reg(REG_MCR, MCR_DTR | MCR_RTS | MCR_OUT2);

        // Test loopback mode
        self.write_reg(REG_MCR, MCR_LOOPBACK | MCR_OUT1 | MCR_OUT2);
        self.write_reg(REG_DATA, 0xAE);

        if self.read_reg(REG_DATA) != 0xAE {
            return Err(BootError::HardwareNotSupported);
        }

        // Disable loopback, enable normal operation
        self.write_reg(REG_MCR, MCR_DTR | MCR_RTS | MCR_OUT2);

        Ok(())
    }

    /// Check if transmit buffer is empty
    unsafe fn can_transmit(&self) -> bool {
        (self.read_reg(REG_LSR) & LSR_THRE) != 0
    }

    /// Check if data is available to read
    unsafe fn data_available(&self) -> bool {
        (self.read_reg(REG_LSR) & LSR_DATA_READY) != 0
    }

    /// Write a byte (blocking)
    ///
    /// # Safety
    ///
    /// The caller must ensure the value is valid for the current system state.
    pub unsafe fn write_byte(&self, byte: u8) {
        // Wait for transmit buffer
        while !self.can_transmit() {
            core::hint::spin_loop();
        }
        self.write_reg(REG_DATA, byte);
    }

    /// Read a byte (blocking)
    ///
    /// # Safety
    ///
    /// The caller must ensure the hardware is properly initialized before reading.
    pub unsafe fn read_byte(&self) -> u8 {
        // Wait for data
        while !self.data_available() {
            core::hint::spin_loop();
        }
        self.read_reg(REG_DATA)
    }

    /// Try to write a byte (non-blocking)
    ///
    /// # Safety
    ///
    /// The caller must ensure the value is valid for the current system state.
    pub unsafe fn try_write_byte(&self, byte: u8) -> bool {
        if self.can_transmit() {
            self.write_reg(REG_DATA, byte);
            true
        } else {
            false
        }
    }

    /// Try to read a byte (non-blocking)
    ///
    /// # Safety
    ///
    /// The caller must ensure the hardware is properly initialized before reading.
    pub unsafe fn try_read_byte(&self) -> Option<u8> {
        if self.data_available() {
            Some(self.read_reg(REG_DATA))
        } else {
            None
        }
    }

    /// Write a string
    ///
    /// # Safety
    ///
    /// The caller must ensure the value is valid for the current system state.
    pub unsafe fn write_str(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }

    /// Get base address
    pub fn base(&self) -> u16 {
        self.base
    }

    /// Check for errors
    ///
    /// # Safety
    ///
    /// The caller must ensure all safety invariants are upheld.
    pub unsafe fn check_errors(&self) -> Option<SerialError> {
        let lsr = self.read_reg(REG_LSR);

        if lsr & LSR_OVERRUN != 0 {
            Some(SerialError::Overrun)
        } else if lsr & LSR_PARITY != 0 {
            Some(SerialError::Parity)
        } else if lsr & LSR_FRAMING != 0 {
            Some(SerialError::Framing)
        } else if lsr & LSR_BREAK != 0 {
            Some(SerialError::Break)
        } else {
            None
        }
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            SerialPort::write_str(self, s);
        }
        Ok(())
    }
}

/// Serial port errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerialError {
    /// Receive buffer overrun
    Overrun,
    /// Parity error
    Parity,
    /// Framing error
    Framing,
    /// Break detected
    Break,
}

// =============================================================================
// GLOBAL SERIAL PORT
// =============================================================================

/// Global COM1 serial port
pub static mut COM1: SerialPort = SerialPort::new(COM1_BASE);
/// Global COM2 serial port
pub static mut COM2: SerialPort = SerialPort::new(COM2_BASE);

/// Initialize the primary serial port
///
/// # Safety
///
/// The caller must ensure serial port I/O is safe and the port is not in use.
pub unsafe fn init_serial(config: &SerialConfig) -> BootResult<()> {
    let port = if config.port == 0 {
        COM1_BASE
    } else {
        config.port
    };

    SERIAL_BASE = port;

    let serial = SerialPort::new(port);
    serial.init(config)?;

    SERIAL_INITIALIZED.store(true, Ordering::SeqCst);

    Ok(())
}

/// Write to the primary serial port
///
/// # Safety
///
/// The caller must ensure the value is valid for the current system state.
pub unsafe fn serial_write(s: &str) {
    if !SERIAL_INITIALIZED.load(Ordering::SeqCst) {
        return;
    }

    let serial = SerialPort::new(SERIAL_BASE);
    serial.write_str(s);
}

/// Write a byte to the primary serial port
///
/// # Safety
///
/// The caller must ensure the value is valid for the current system state.
pub unsafe fn serial_write_byte(byte: u8) {
    if !SERIAL_INITIALIZED.load(Ordering::SeqCst) {
        return;
    }

    let serial = SerialPort::new(SERIAL_BASE);
    serial.write_byte(byte);
}

/// Read from the primary serial port (blocking)
///
/// # Safety
///
/// The caller must ensure the hardware is properly initialized before reading.
pub unsafe fn serial_read() -> u8 {
    let serial = SerialPort::new(SERIAL_BASE);
    serial.read_byte()
}

/// Try to read from the primary serial port (non-blocking)
///
/// # Safety
///
/// The caller must ensure the hardware is properly initialized before reading.
pub unsafe fn serial_try_read() -> Option<u8> {
    let serial = SerialPort::new(SERIAL_BASE);
    serial.try_read_byte()
}

// =============================================================================
// SERIAL PRINT MACROS
// =============================================================================

/// Serial print writer
pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            serial_write(s);
        }
        Ok(())
    }
}

/// Print to serial port
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let _ = write!($crate::arch::x86_64::serial::SerialWriter, $($arg)*);
        }
    };
}

/// Print to serial port with newline
#[macro_export]
macro_rules! serial_println {
    () => {
        $crate::serial_print!("\n")
    };
    ($($arg:tt)*) => {
        $crate::serial_print!("{}\n", format_args!($($arg)*))
    };
}

// =============================================================================
// DEBUG OUTPUT
// =============================================================================

/// Debug output levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Current log level
static mut LOG_LEVEL: LogLevel = LogLevel::Info;

/// Set log level
pub fn set_log_level(level: LogLevel) {
    unsafe {
        LOG_LEVEL = level;
    }
}

/// Get log level
pub fn get_log_level() -> LogLevel {
    unsafe { LOG_LEVEL }
}

/// Log a message
pub fn log(level: LogLevel, args: fmt::Arguments) {
    if level < get_log_level() {
        return;
    }

    let prefix = match level {
        LogLevel::Trace => "[TRACE]",
        LogLevel::Debug => "[DEBUG]",
        LogLevel::Info => "[INFO] ",
        LogLevel::Warn => "[WARN] ",
        LogLevel::Error => "[ERROR]",
    };

    unsafe {
        let serial = SerialPort::new(SERIAL_BASE);
        serial.write_str(prefix);
        serial.write_str(" ");
        let _ = write!(SerialWriter, "{}", args);
        serial.write_str("\n");
    }
}

/// Log macros
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        $crate::arch::x86_64::serial::log(
            $crate::arch::x86_64::serial::LogLevel::Trace,
            format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::arch::x86_64::serial::log(
            $crate::arch::x86_64::serial::LogLevel::Debug,
            format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {
        $crate::arch::x86_64::serial::log(
            $crate::arch::x86_64::serial::LogLevel::Info,
            format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {
        $crate::arch::x86_64::serial::log(
            $crate::arch::x86_64::serial::LogLevel::Warn,
            format_args!($($arg)*)
        )
    };
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        $crate::arch::x86_64::serial::log(
            $crate::arch::x86_64::serial::LogLevel::Error,
            format_args!($($arg)*)
        )
    };
}
