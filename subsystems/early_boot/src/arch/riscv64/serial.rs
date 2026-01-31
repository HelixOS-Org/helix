//! # RISC-V UART Serial Driver
//!
//! Early boot serial console driver for RISC-V.
//! Supports 8250/16550 compatible UARTs and SBI console.

use core::fmt::{self, Write};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{sbi, *};
use crate::core::BootContext;
use crate::error::BootResult;

// =============================================================================
// UART 16550 REGISTERS
// =============================================================================

/// Receive Buffer Register (read) / Transmit Holding Register (write)
pub const RBR_THR: u64 = 0;
/// Interrupt Enable Register
pub const IER: u64 = 1;
/// Interrupt Identification Register (read) / FIFO Control Register (write)
pub const IIR_FCR: u64 = 2;
/// Line Control Register
pub const LCR: u64 = 3;
/// Modem Control Register
pub const MCR: u64 = 4;
/// Line Status Register
pub const LSR: u64 = 5;
/// Modem Status Register
pub const MSR: u64 = 6;
/// Scratch Register
pub const SCR: u64 = 7;

// Divisor Latch (when DLAB = 1)
/// Divisor Latch Low
pub const DLL: u64 = 0;
/// Divisor Latch High
pub const DLH: u64 = 1;

// =============================================================================
// LINE STATUS REGISTER BITS
// =============================================================================

/// Data Ready
pub const LSR_DR: u8 = 1 << 0;
/// Overrun Error
pub const LSR_OE: u8 = 1 << 1;
/// Parity Error
pub const LSR_PE: u8 = 1 << 2;
/// Framing Error
pub const LSR_FE: u8 = 1 << 3;
/// Break Interrupt
pub const LSR_BI: u8 = 1 << 4;
/// Transmitter Holding Register Empty
pub const LSR_THRE: u8 = 1 << 5;
/// Transmitter Empty
pub const LSR_TEMT: u8 = 1 << 6;
/// Error in RCVR FIFO
pub const LSR_ERR: u8 = 1 << 7;

// =============================================================================
// LINE CONTROL REGISTER BITS
// =============================================================================

/// Word Length 5 bits
pub const LCR_WLS_5: u8 = 0b00;
/// Word Length 6 bits
pub const LCR_WLS_6: u8 = 0b01;
/// Word Length 7 bits
pub const LCR_WLS_7: u8 = 0b10;
/// Word Length 8 bits
pub const LCR_WLS_8: u8 = 0b11;
/// Number of Stop Bits (0 = 1, 1 = 2)
pub const LCR_STB: u8 = 1 << 2;
/// Parity Enable
pub const LCR_PEN: u8 = 1 << 3;
/// Even Parity Select
pub const LCR_EPS: u8 = 1 << 4;
/// Stick Parity
pub const LCR_SP: u8 = 1 << 5;
/// Set Break
pub const LCR_SB: u8 = 1 << 6;
/// Divisor Latch Access Bit
pub const LCR_DLAB: u8 = 1 << 7;

// =============================================================================
// FIFO CONTROL REGISTER BITS
// =============================================================================

/// FIFO Enable
pub const FCR_FIFO_EN: u8 = 1 << 0;
/// Clear Receive FIFO
pub const FCR_CLEAR_RX: u8 = 1 << 1;
/// Clear Transmit FIFO
pub const FCR_CLEAR_TX: u8 = 1 << 2;
/// DMA Mode Select
pub const FCR_DMA: u8 = 1 << 3;
/// FIFO Trigger Level 1 byte
pub const FCR_TRIGGER_1: u8 = 0b00 << 6;
/// FIFO Trigger Level 4 bytes
pub const FCR_TRIGGER_4: u8 = 0b01 << 6;
/// FIFO Trigger Level 8 bytes
pub const FCR_TRIGGER_8: u8 = 0b10 << 6;
/// FIFO Trigger Level 14 bytes
pub const FCR_TRIGGER_14: u8 = 0b11 << 6;

// =============================================================================
// MODEM CONTROL REGISTER BITS
// =============================================================================

/// Data Terminal Ready
pub const MCR_DTR: u8 = 1 << 0;
/// Request To Send
pub const MCR_RTS: u8 = 1 << 1;
/// Out 1
pub const MCR_OUT1: u8 = 1 << 2;
/// Out 2 (enables interrupts)
pub const MCR_OUT2: u8 = 1 << 3;
/// Loopback Mode
pub const MCR_LOOP: u8 = 1 << 4;

// =============================================================================
// INTERRUPT ENABLE REGISTER BITS
// =============================================================================

/// Received Data Available Interrupt
pub const IER_RDA: u8 = 1 << 0;
/// Transmitter Holding Register Empty Interrupt
pub const IER_THRE: u8 = 1 << 1;
/// Receiver Line Status Interrupt
pub const IER_RLS: u8 = 1 << 2;
/// Modem Status Interrupt
pub const IER_MS: u8 = 1 << 3;

// =============================================================================
// DEFAULT ADDRESSES
// =============================================================================

/// QEMU virt machine UART0 base
pub const QEMU_UART0_BASE: u64 = 0x1000_0000;
/// Default UART clock (for baud rate calculation)
pub const DEFAULT_CLOCK: u32 = 3_686_400; // 3.6864 MHz

// =============================================================================
// UART STATE
// =============================================================================

/// Primary UART base address
static UART_BASE: AtomicU64 = AtomicU64::new(QEMU_UART0_BASE);
/// UART register stride (bytes between registers)
static UART_STRIDE: AtomicU64 = AtomicU64::new(1);
/// Use SBI console instead of direct UART
static USE_SBI: AtomicBool = AtomicBool::new(false);

// =============================================================================
// UART 16550 DRIVER
// =============================================================================

/// UART 16550 driver
pub struct Uart16550 {
    base: u64,
    stride: u64,
}

impl Uart16550 {
    /// Create new UART driver
    pub const fn new(base: u64, stride: u64) -> Self {
        Self { base, stride }
    }

    /// Read register
    #[inline]
    unsafe fn read(&self, reg: u64) -> u8 {
        let addr = self.base + reg * self.stride;
        core::ptr::read_volatile(addr as *const u8)
    }

    /// Write register
    #[inline]
    unsafe fn write(&self, reg: u64, value: u8) {
        let addr = self.base + reg * self.stride;
        core::ptr::write_volatile(addr as *mut u8, value);
    }

    /// Initialize UART
    pub unsafe fn init(&self, baud: u32, clock: u32) {
        // Disable interrupts
        self.write(IER, 0);

        // Set DLAB to access divisor
        self.write(LCR, LCR_DLAB);

        // Set divisor (baud rate)
        let divisor = clock / (16 * baud);
        self.write(DLL, (divisor & 0xFF) as u8);
        self.write(DLH, ((divisor >> 8) & 0xFF) as u8);

        // 8N1, no DLAB
        self.write(LCR, LCR_WLS_8);

        // Enable and clear FIFOs
        self.write(
            IIR_FCR,
            FCR_FIFO_EN | FCR_CLEAR_RX | FCR_CLEAR_TX | FCR_TRIGGER_14,
        );

        // Enable RTS/DTR
        self.write(MCR, MCR_DTR | MCR_RTS | MCR_OUT2);

        // Clear any pending data
        let _ = self.read(RBR_THR);
    }

    /// Initialize with default settings (115200 8N1)
    pub unsafe fn init_default(&self) {
        self.init(115200, DEFAULT_CLOCK);
    }

    /// Check if transmitter is empty
    #[inline]
    pub unsafe fn tx_empty(&self) -> bool {
        self.read(LSR) & LSR_THRE != 0
    }

    /// Check if data is available
    #[inline]
    pub unsafe fn rx_ready(&self) -> bool {
        self.read(LSR) & LSR_DR != 0
    }

    /// Write byte (blocking)
    pub unsafe fn write_byte(&self, byte: u8) {
        while !self.tx_empty() {
            core::hint::spin_loop();
        }
        self.write(RBR_THR, byte);
    }

    /// Write byte (non-blocking)
    pub unsafe fn try_write_byte(&self, byte: u8) -> bool {
        if !self.tx_empty() {
            return false;
        }
        self.write(RBR_THR, byte);
        true
    }

    /// Read byte (blocking)
    pub unsafe fn read_byte(&self) -> u8 {
        while !self.rx_ready() {
            core::hint::spin_loop();
        }
        self.read(RBR_THR)
    }

    /// Read byte (non-blocking)
    pub unsafe fn try_read_byte(&self) -> Option<u8> {
        if !self.rx_ready() {
            return None;
        }
        Some(self.read(RBR_THR))
    }

    /// Write string
    pub unsafe fn write_str(&self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }

    /// Flush output
    pub unsafe fn flush(&self) {
        while self.read(LSR) & LSR_TEMT == 0 {
            core::hint::spin_loop();
        }
    }

    /// Get line status
    pub unsafe fn line_status(&self) -> u8 {
        self.read(LSR)
    }

    /// Enable interrupts
    pub unsafe fn enable_interrupts(&self, mask: u8) {
        let current = self.read(IER);
        self.write(IER, current | mask);
    }

    /// Disable interrupts
    pub unsafe fn disable_interrupts(&self, mask: u8) {
        let current = self.read(IER);
        self.write(IER, current & !mask);
    }
}

impl Write for Uart16550 {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        unsafe {
            Uart16550::write_str(self, s);
        }
        Ok(())
    }
}

// =============================================================================
// GLOBAL UART FUNCTIONS
// =============================================================================

/// Get global UART instance
pub fn get_uart() -> Uart16550 {
    Uart16550::new(
        UART_BASE.load(Ordering::SeqCst),
        UART_STRIDE.load(Ordering::SeqCst),
    )
}

/// Write byte
pub fn write_byte(byte: u8) {
    if USE_SBI.load(Ordering::SeqCst) {
        sbi::console_putchar(byte);
    } else {
        unsafe {
            get_uart().write_byte(byte);
        }
    }
}

/// Write string
pub fn write_str(s: &str) {
    if USE_SBI.load(Ordering::SeqCst) {
        for byte in s.bytes() {
            if byte == b'\n' {
                sbi::console_putchar(b'\r');
            }
            sbi::console_putchar(byte);
        }
    } else {
        unsafe {
            get_uart().write_str(s);
        }
    }
}

/// Read byte (blocking)
pub fn read_byte() -> u8 {
    if USE_SBI.load(Ordering::SeqCst) {
        loop {
            if let Some(byte) = sbi::console_getchar() {
                return byte;
            }
            core::hint::spin_loop();
        }
    } else {
        unsafe { get_uart().read_byte() }
    }
}

/// Try to read byte (non-blocking)
pub fn try_read_byte() -> Option<u8> {
    if USE_SBI.load(Ordering::SeqCst) {
        sbi::console_getchar()
    } else {
        unsafe { get_uart().try_read_byte() }
    }
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
    write_str("[");
    write_str(level.as_str());
    write_str("] ");

    let mut writer = SerialWriter;
    let _ = writer.write_fmt(args);

    write_str("\n");
}

// =============================================================================
// MACROS
// =============================================================================

/// Print macro (no newline)
#[macro_export]
macro_rules! serial_print_riscv {
    ($($arg:tt)*) => {
        $crate::arch::riscv64::serial::write_str(&alloc::format!($($arg)*))
    };
}

/// Println macro
#[macro_export]
macro_rules! serial_println_riscv {
    () => { $crate::arch::riscv64::serial::write_str("\n") };
    ($($arg:tt)*) => {
        $crate::arch::riscv64::serial::write_str(&alloc::format!("{}\n", format_args!($($arg)*)))
    };
}

/// Trace log
#[macro_export]
macro_rules! log_trace_riscv {
    ($($arg:tt)*) => {
        $crate::arch::riscv64::serial::log(
            $crate::arch::riscv64::serial::LogLevel::Trace,
            format_args!($($arg)*)
        )
    };
}

/// Debug log
#[macro_export]
macro_rules! log_debug_riscv {
    ($($arg:tt)*) => {
        $crate::arch::riscv64::serial::log(
            $crate::arch::riscv64::serial::LogLevel::Debug,
            format_args!($($arg)*)
        )
    };
}

/// Info log
#[macro_export]
macro_rules! log_info_riscv {
    ($($arg:tt)*) => {
        $crate::arch::riscv64::serial::log(
            $crate::arch::riscv64::serial::LogLevel::Info,
            format_args!($($arg)*)
        )
    };
}

/// Warn log
#[macro_export]
macro_rules! log_warn_riscv {
    ($($arg:tt)*) => {
        $crate::arch::riscv64::serial::log(
            $crate::arch::riscv64::serial::LogLevel::Warn,
            format_args!($($arg)*)
        )
    };
}

/// Error log
#[macro_export]
macro_rules! log_error_riscv {
    ($($arg:tt)*) => {
        $crate::arch::riscv64::serial::log(
            $crate::arch::riscv64::serial::LogLevel::Error,
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
    write_byte(b'\r');
    write_byte(b'\n');
}

/// Print hex value
pub fn print_hex(value: u64) {
    write_str("0x");
    for i in (0..16).rev() {
        let nibble = ((value >> (i * 4)) & 0xF) as u8;
        let c = if nibble < 10 {
            b'0' + nibble
        } else {
            b'a' + nibble - 10
        };
        write_byte(c);
    }
}

/// Print decimal value
pub fn print_dec(mut value: u64) {
    if value == 0 {
        write_byte(b'0');
        return;
    }

    let mut buf = [0u8; 20];
    let mut i = 0;

    while value > 0 {
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
        i += 1;
    }

    while i > 0 {
        i -= 1;
        write_byte(buf[i]);
    }
}

// =============================================================================
// PANIC SUPPORT
// =============================================================================

/// Panic output (does not depend on allocator)
pub fn panic_print(s: &str) {
    write_str(s);
}

/// Print panic info
pub fn panic_info(location: Option<&core::panic::Location>, msg: &str) {
    write_str("\n!!! PANIC !!!\n");

    if let Some(loc) = location {
        write_str("Location: ");
        write_str(loc.file());
        write_str(":");
        print_dec(loc.line() as u64);
        write_str("\n");
    }

    write_str("Message: ");
    write_str(msg);
    write_str("\n");

    // Print hart ID
    write_str("Hart ID: ");
    print_dec(get_hart_id());
    write_str("\n");
}

// =============================================================================
// INITIALIZATION
// =============================================================================

/// Initialize serial console
pub unsafe fn init(ctx: &mut BootContext) -> BootResult<()> {
    // Determine if we should use SBI or direct UART
    // In S-mode, we typically use SBI; in M-mode, we use direct UART

    // Try to get UART address from device tree
    let base = if let Some(ref dt_info) = ctx.boot_info.device_tree {
        // TODO: Parse device tree for UART address
        QEMU_UART0_BASE
    } else {
        QEMU_UART0_BASE
    };

    UART_BASE.store(base, Ordering::SeqCst);
    UART_STRIDE.store(1, Ordering::SeqCst);

    // For now, prefer SBI console in S-mode for compatibility
    // Direct UART access may not work in all environments
    USE_SBI.store(true, Ordering::SeqCst);

    // If we have direct UART access and not using SBI
    if !USE_SBI.load(Ordering::SeqCst) {
        let uart = get_uart();
        uart.init_default();
    }

    Ok(())
}

/// Use direct UART access
pub fn use_direct_uart(base: u64, stride: u64) {
    UART_BASE.store(base, Ordering::SeqCst);
    UART_STRIDE.store(stride, Ordering::SeqCst);
    USE_SBI.store(false, Ordering::SeqCst);

    unsafe {
        let uart = get_uart();
        uart.init_default();
    }
}

/// Use SBI console
pub fn use_sbi_console() {
    USE_SBI.store(true, Ordering::SeqCst);
}
