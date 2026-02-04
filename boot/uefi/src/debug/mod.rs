//! Debugging and Logging Infrastructure
//!
//! Comprehensive debug output, serial port, and logging for UEFI bootloader.

use core::fmt::{self, Write};

// =============================================================================
// LOG LEVELS
// =============================================================================

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Trace level (most verbose)
    Trace = 0,
    /// Debug level
    Debug = 1,
    /// Info level
    Info  = 2,
    /// Warning level
    Warn  = 3,
    /// Error level
    Error = 4,
    /// Fatal level
    Fatal = 5,
    /// Off (no logging)
    Off   = 6,
}

impl LogLevel {
    /// Get level name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
            Self::Fatal => "FATAL",
            Self::Off => "OFF",
        }
    }

    /// Get short name
    pub fn short_name(&self) -> &'static str {
        match self {
            Self::Trace => "T",
            Self::Debug => "D",
            Self::Info => "I",
            Self::Warn => "W",
            Self::Error => "E",
            Self::Fatal => "F",
            Self::Off => "-",
        }
    }

    /// Get color code
    pub fn color(&self) -> Color {
        match self {
            Self::Trace => Color::DarkGray,
            Self::Debug => Color::Cyan,
            Self::Info => Color::Green,
            Self::Warn => Color::Yellow,
            Self::Error => Color::Red,
            Self::Fatal => Color::Magenta,
            Self::Off => Color::White,
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Console color
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    BrightWhite,
}

impl Color {
    /// ANSI escape code
    pub fn ansi_fg(&self) -> &'static str {
        match self {
            Self::Black => "\x1b[30m",
            Self::Red => "\x1b[31m",
            Self::Green => "\x1b[32m",
            Self::Yellow => "\x1b[33m",
            Self::Blue => "\x1b[34m",
            Self::Magenta => "\x1b[35m",
            Self::Cyan => "\x1b[36m",
            Self::White => "\x1b[37m",
            Self::DarkGray => "\x1b[90m",
            Self::LightRed => "\x1b[91m",
            Self::LightGreen => "\x1b[92m",
            Self::LightYellow => "\x1b[93m",
            Self::LightBlue => "\x1b[94m",
            Self::LightMagenta => "\x1b[95m",
            Self::LightCyan => "\x1b[96m",
            Self::BrightWhite => "\x1b[97m",
        }
    }

    /// ANSI reset
    pub fn ansi_reset() -> &'static str {
        "\x1b[0m"
    }
}

// =============================================================================
// SERIAL PORT
// =============================================================================

/// Serial port I/O ports
pub mod serial_port {
    pub const COM1: u16 = 0x3F8;
    pub const COM2: u16 = 0x2F8;
    pub const COM3: u16 = 0x3E8;
    pub const COM4: u16 = 0x2E8;
}

/// Serial port registers
pub mod serial_reg {
    pub const DATA: u16 = 0;
    pub const INTR_ENABLE: u16 = 1;
    pub const FIFO_CTRL: u16 = 2;
    pub const LINE_CTRL: u16 = 3;
    pub const MODEM_CTRL: u16 = 4;
    pub const LINE_STATUS: u16 = 5;
    pub const MODEM_STATUS: u16 = 6;
    pub const SCRATCH: u16 = 7;

    // With DLAB set
    pub const DIVISOR_LOW: u16 = 0;
    pub const DIVISOR_HIGH: u16 = 1;
}

/// Line status bits
pub mod line_status {
    pub const DATA_READY: u8 = 0x01;
    pub const OVERRUN_ERROR: u8 = 0x02;
    pub const PARITY_ERROR: u8 = 0x04;
    pub const FRAMING_ERROR: u8 = 0x08;
    pub const BREAK_INTERRUPT: u8 = 0x10;
    pub const THRE: u8 = 0x20; // Transmit holding register empty
    pub const TEMT: u8 = 0x40; // Transmitter empty
    pub const FIFO_ERROR: u8 = 0x80;
}

/// Baud rate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaudRate {
    B9600,
    B19200,
    B38400,
    B57600,
    B115200,
}

impl BaudRate {
    /// Get divisor for baud rate
    pub fn divisor(&self) -> u16 {
        match self {
            Self::B9600 => 12,
            Self::B19200 => 6,
            Self::B38400 => 3,
            Self::B57600 => 2,
            Self::B115200 => 1,
        }
    }
}

/// Serial port writer
pub struct SerialPort {
    port: u16,
    initialized: bool,
}

impl SerialPort {
    /// Create new serial port
    pub const fn new(port: u16) -> Self {
        Self {
            port,
            initialized: false,
        }
    }

    /// Create COM1 serial port
    pub const fn com1() -> Self {
        Self::new(serial_port::COM1)
    }

    /// Initialize serial port
    #[cfg(target_arch = "x86_64")]
    pub fn init(&mut self, baud: BaudRate) {
        unsafe {
            // Disable interrupts
            self.write_reg(serial_reg::INTR_ENABLE, 0x00);

            // Enable DLAB
            self.write_reg(serial_reg::LINE_CTRL, 0x80);

            // Set divisor
            let divisor = baud.divisor();
            self.write_reg(serial_reg::DIVISOR_LOW, (divisor & 0xFF) as u8);
            self.write_reg(serial_reg::DIVISOR_HIGH, ((divisor >> 8) & 0xFF) as u8);

            // 8 bits, no parity, 1 stop bit
            self.write_reg(serial_reg::LINE_CTRL, 0x03);

            // Enable FIFO, clear, 14-byte threshold
            self.write_reg(serial_reg::FIFO_CTRL, 0xC7);

            // IRQs enabled, RTS/DSR set
            self.write_reg(serial_reg::MODEM_CTRL, 0x0B);

            // Set loopback mode for test
            self.write_reg(serial_reg::MODEM_CTRL, 0x1E);

            // Test serial chip
            self.write_reg(serial_reg::DATA, 0xAE);

            if self.read_reg(serial_reg::DATA) != 0xAE {
                return; // Faulty serial
            }

            // Normal operation mode
            self.write_reg(serial_reg::MODEM_CTRL, 0x0F);

            self.initialized = true;
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn init(&mut self, _baud: BaudRate) {
        // Stub for non-x86_64
    }

    /// Write register
    #[cfg(target_arch = "x86_64")]
    unsafe fn write_reg(&self, reg: u16, value: u8) {
        core::arch::asm!(
            "out dx, al",
            in("dx") self.port + reg,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }

    /// Read register
    #[cfg(target_arch = "x86_64")]
    unsafe fn read_reg(&self, reg: u16) -> u8 {
        let value: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") self.port + reg,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
        value
    }

    /// Check if transmit buffer is empty
    #[cfg(target_arch = "x86_64")]
    fn is_transmit_empty(&self) -> bool {
        unsafe { self.read_reg(serial_reg::LINE_STATUS) & line_status::THRE != 0 }
    }

    /// Write byte
    #[cfg(target_arch = "x86_64")]
    pub fn write_byte(&mut self, byte: u8) {
        if !self.initialized {
            return;
        }

        // Wait for transmit buffer
        while !self.is_transmit_empty() {
            core::hint::spin_loop();
        }

        unsafe {
            self.write_reg(serial_reg::DATA, byte);
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn write_byte(&mut self, _byte: u8) {
        // Stub for non-x86_64
    }

    /// Write string
    pub fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
        }
    }

    /// Check if data is available
    #[cfg(target_arch = "x86_64")]
    pub fn data_available(&self) -> bool {
        if !self.initialized {
            return false;
        }
        unsafe { self.read_reg(serial_reg::LINE_STATUS) & line_status::DATA_READY != 0 }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn data_available(&self) -> bool {
        false
    }

    /// Read byte
    #[cfg(target_arch = "x86_64")]
    pub fn read_byte(&mut self) -> Option<u8> {
        if !self.initialized || !self.data_available() {
            return None;
        }

        unsafe { Some(self.read_reg(serial_reg::DATA)) }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn read_byte(&mut self) -> Option<u8> {
        None
    }
}

impl Write for SerialPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_str(s);
        Ok(())
    }
}

// =============================================================================
// DEBUG OUTPUT
// =============================================================================

/// Debug output target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugTarget {
    /// Serial port
    Serial,
    /// Console (SimpleTextOutput)
    Console,
    /// Both serial and console
    Both,
    /// Debug port (port 0xE9)
    DebugPort,
    /// Memory buffer
    Buffer,
}

/// Debug port writer (port 0xE9, QEMU debug)
pub struct DebugPort;

impl DebugPort {
    /// Write byte
    #[cfg(target_arch = "x86_64")]
    pub fn write_byte(byte: u8) {
        unsafe {
            core::arch::asm!(
                "out dx, al",
                in("dx") 0xE9u16,
                in("al") byte,
                options(nomem, nostack, preserves_flags)
            );
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn write_byte(_byte: u8) {
        // Stub
    }

    /// Write string
    pub fn write_str(s: &str) {
        for byte in s.bytes() {
            Self::write_byte(byte);
        }
    }
}

impl Write for DebugPort {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Self::write_str(s);
        Ok(())
    }
}

// =============================================================================
// LOG BUFFER
// =============================================================================

/// Maximum log buffer size
pub const LOG_BUFFER_SIZE: usize = 64 * 1024; // 64KB

/// Log entry
#[derive(Clone)]
pub struct LogEntry {
    /// Log level
    pub level: LogLevel,
    /// Timestamp (if available)
    pub timestamp: u64,
    /// Message offset in buffer
    pub message_offset: usize,
    /// Message length
    pub message_len: usize,
    /// Module/source
    pub module: Option<&'static str>,
    /// Line number
    pub line: Option<u32>,
}

/// Circular log buffer
pub struct LogBuffer {
    /// Text buffer
    buffer: [u8; LOG_BUFFER_SIZE],
    /// Current write position
    write_pos: usize,
    /// Entries
    entries: [Option<LogEntry>; 256],
    /// Entry write index
    entry_index: usize,
    /// Total bytes written
    total_bytes: usize,
}

impl LogBuffer {
    /// Create new log buffer
    pub const fn new() -> Self {
        const NONE_ENTRY: Option<LogEntry> = None;

        Self {
            buffer: [0; LOG_BUFFER_SIZE],
            write_pos: 0,
            entries: [NONE_ENTRY; 256],
            entry_index: 0,
            total_bytes: 0,
        }
    }

    /// Add log entry
    pub fn push(
        &mut self,
        level: LogLevel,
        message: &str,
        module: Option<&'static str>,
        line: Option<u32>,
    ) {
        let message_bytes = message.as_bytes();
        let len = message_bytes.len().min(LOG_BUFFER_SIZE);

        // Write message to buffer
        let start = self.write_pos;
        for (i, &byte) in message_bytes.iter().take(len).enumerate() {
            self.buffer[(start + i) % LOG_BUFFER_SIZE] = byte;
        }
        self.write_pos = (start + len) % LOG_BUFFER_SIZE;
        self.total_bytes += len;

        // Add entry
        self.entries[self.entry_index] = Some(LogEntry {
            level,
            timestamp: 0, // TODO: Add timestamp support
            message_offset: start,
            message_len: len,
            module,
            line,
        });
        self.entry_index = (self.entry_index + 1) % 256;
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_some()).count()
    }

    /// Get message text for entry
    pub fn message(&self, entry: &LogEntry) -> &str {
        let start = entry.message_offset;
        let end = start + entry.message_len;

        if end <= LOG_BUFFER_SIZE && end > start {
            core::str::from_utf8(&self.buffer[start..end]).unwrap_or("")
        } else {
            ""
        }
    }

    /// Clear buffer
    pub fn clear(&mut self) {
        self.write_pos = 0;
        self.entry_index = 0;
        self.total_bytes = 0;
        for entry in &mut self.entries {
            *entry = None;
        }
    }

    /// Get recent entries
    pub fn recent_entries(&self, count: usize) -> impl Iterator<Item = &LogEntry> {
        self.entries
            .iter()
            .filter_map(|e| e.as_ref())
            .rev()
            .take(count)
    }
}

// =============================================================================
// LOGGER
// =============================================================================

/// Logger configuration
pub struct LoggerConfig {
    /// Minimum log level
    pub min_level: LogLevel,
    /// Output target
    pub target: DebugTarget,
    /// Enable colors
    pub colors: bool,
    /// Show timestamps
    pub timestamps: bool,
    /// Show module/source
    pub show_source: bool,
}

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Info,
            target: DebugTarget::Serial,
            colors: true,
            timestamps: false,
            show_source: true,
        }
    }
}

/// Global logger
pub struct Logger {
    /// Configuration
    config: LoggerConfig,
    /// Serial port
    serial: SerialPort,
    /// Log buffer
    buffer: LogBuffer,
    /// Initialized flag
    initialized: bool,
}

impl Logger {
    /// Create new logger
    pub const fn new() -> Self {
        Self {
            config: LoggerConfig {
                min_level: LogLevel::Info,
                target: DebugTarget::Serial,
                colors: true,
                timestamps: false,
                show_source: true,
            },
            serial: SerialPort::com1(),
            buffer: LogBuffer::new(),
            initialized: false,
        }
    }

    /// Initialize logger
    pub fn init(&mut self, config: LoggerConfig) {
        self.config = config;

        if matches!(self.config.target, DebugTarget::Serial | DebugTarget::Both) {
            self.serial.init(BaudRate::B115200);
        }

        self.initialized = true;
    }

    /// Set minimum log level
    pub fn set_level(&mut self, level: LogLevel) {
        self.config.min_level = level;
    }

    /// Log message
    pub fn log(
        &mut self,
        level: LogLevel,
        module: Option<&'static str>,
        line: Option<u32>,
        args: fmt::Arguments<'_>,
    ) {
        if level < self.config.min_level || !self.initialized {
            return;
        }

        // Format message
        let mut message_buf = [0u8; 512];
        let mut writer = ArrayWriter::new(&mut message_buf);
        let _ = write!(writer, "{}", args);
        let message = writer.as_str();

        // Store in buffer
        self.buffer.push(level, message, module, line);

        // Output based on target
        match self.config.target {
            DebugTarget::Serial | DebugTarget::Both => {
                self.output_serial(level, module, line, message);
            },
            DebugTarget::DebugPort => {
                self.output_debug_port(level, module, line, message);
            },
            DebugTarget::Console => {
                // Console output would need EFI handle
            },
            DebugTarget::Buffer => {
                // Already stored
            },
        }
    }

    /// Output to serial port
    fn output_serial(
        &mut self,
        level: LogLevel,
        module: Option<&'static str>,
        line: Option<u32>,
        message: &str,
    ) {
        // Color prefix
        if self.config.colors {
            self.serial.write_str(level.color().ansi_fg());
        }

        // Level
        self.serial.write_str("[");
        self.serial.write_str(level.name());
        self.serial.write_str("]");

        // Source
        if self.config.show_source {
            if let Some(m) = module {
                self.serial.write_str(" ");
                self.serial.write_str(m);
                if let Some(l) = line {
                    self.serial.write_str(":");
                    let mut num_buf = [0u8; 10];
                    let num_str = format_u32(l, &mut num_buf);
                    self.serial.write_str(num_str);
                }
            }
        }

        // Reset color and message
        if self.config.colors {
            self.serial.write_str(Color::ansi_reset());
        }

        self.serial.write_str(" ");
        self.serial.write_str(message);
        self.serial.write_str("\n");
    }

    /// Output to debug port
    fn output_debug_port(
        &mut self,
        level: LogLevel,
        module: Option<&'static str>,
        _line: Option<u32>,
        message: &str,
    ) {
        DebugPort::write_str("[");
        DebugPort::write_str(level.short_name());
        DebugPort::write_str("]");

        if let Some(m) = module {
            DebugPort::write_str(" ");
            DebugPort::write_str(m);
        }

        DebugPort::write_str(": ");
        DebugPort::write_str(message);
        DebugPort::write_str("\n");
    }

    /// Get log buffer
    pub fn buffer(&self) -> &LogBuffer {
        &self.buffer
    }
}

// =============================================================================
// ARRAY WRITER
// =============================================================================

/// Writer to fixed array
pub struct ArrayWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> ArrayWriter<'a> {
    /// Create new array writer
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Get written bytes as string
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.pos]).unwrap_or("")
    }

    /// Get written length
    pub fn len(&self) -> usize {
        self.pos
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.pos == 0
    }
}

impl<'a> Write for ArrayWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes = s.as_bytes();
        let space = self.buf.len() - self.pos;
        let to_write = bytes.len().min(space);

        self.buf[self.pos..self.pos + to_write].copy_from_slice(&bytes[..to_write]);
        self.pos += to_write;

        Ok(())
    }
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Format u32 to string
fn format_u32(mut n: u32, buf: &mut [u8; 10]) -> &str {
    if n == 0 {
        buf[0] = b'0';
        return core::str::from_utf8(&buf[..1]).unwrap();
    }

    let mut pos = 10;
    while n > 0 && pos > 0 {
        pos -= 1;
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
    }

    core::str::from_utf8(&buf[pos..]).unwrap()
}

/// Format u64 as hex
pub fn format_hex64(n: u64, buf: &mut [u8; 18]) -> &str {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    buf[0] = b'0';
    buf[1] = b'x';

    for i in 0..16 {
        let nibble = ((n >> ((15 - i) * 4)) & 0xF) as usize;
        buf[2 + i] = HEX[nibble];
    }

    core::str::from_utf8(buf).unwrap()
}

/// Format pointer
pub fn format_ptr<T>(ptr: *const T, buf: &mut [u8; 18]) -> &str {
    format_hex64(ptr as u64, buf)
}

// =============================================================================
// PANIC HANDLER HELPERS
// =============================================================================

/// Panic info for display
pub struct PanicInfo<'a> {
    pub message: Option<&'a fmt::Arguments<'a>>,
    pub location: Option<&'a core::panic::Location<'a>>,
}

impl<'a> PanicInfo<'a> {
    /// Format panic for display
    pub fn format(&self, serial: &mut SerialPort) {
        serial.write_str("\n\x1b[91m");
        serial.write_str("=".repeat(60).as_str());
        serial.write_str("\n KERNEL PANIC\n");
        serial.write_str("=".repeat(60).as_str());
        serial.write_str("\x1b[0m\n\n");

        if let Some(loc) = self.location {
            serial.write_str("Location: ");
            serial.write_str(loc.file());
            serial.write_str(":");

            let mut line_buf = [0u8; 10];
            serial.write_str(format_u32(loc.line(), &mut line_buf));
            serial.write_str("\n");
        }

        serial.write_str("\nHalting system.\n");
    }
}

// =============================================================================
// HEXDUMP
// =============================================================================

/// Hexdump configuration
#[derive(Clone)]
pub struct HexdumpConfig {
    /// Bytes per line
    pub bytes_per_line: usize,
    /// Show ASCII
    pub show_ascii: bool,
    /// Show address
    pub show_address: bool,
    /// Group size
    pub group_size: usize,
}

impl Default for HexdumpConfig {
    fn default() -> Self {
        Self {
            bytes_per_line: 16,
            show_ascii: true,
            show_address: true,
            group_size: 2,
        }
    }
}

/// Create hexdump of data
pub struct Hexdump<'a> {
    data: &'a [u8],
    base_address: u64,
    config: HexdumpConfig,
}

impl<'a> Hexdump<'a> {
    /// Create new hexdump
    pub fn new(data: &'a [u8], base_address: u64) -> Self {
        Self {
            data,
            base_address,
            config: HexdumpConfig::default(),
        }
    }

    /// Set configuration
    pub fn with_config(mut self, config: HexdumpConfig) -> Self {
        self.config = config;
        self
    }

    /// Write to serial port
    pub fn write_to(&self, serial: &mut SerialPort) {
        let bytes_per_line = self.config.bytes_per_line;

        for (i, chunk) in self.data.chunks(bytes_per_line).enumerate() {
            // Address
            if self.config.show_address {
                let addr = self.base_address + (i * bytes_per_line) as u64;
                let mut buf = [0u8; 18];
                serial.write_str(format_hex64(addr, &mut buf));
                serial.write_str(": ");
            }

            // Hex bytes
            for (j, &byte) in chunk.iter().enumerate() {
                if j > 0 && j % self.config.group_size == 0 {
                    serial.write_str(" ");
                }

                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                let mut hex = [0u8; 2];
                hex[0] = HEX[(byte >> 4) as usize];
                hex[1] = HEX[(byte & 0xF) as usize];
                serial.write_str(core::str::from_utf8(&hex).unwrap());
            }

            // Padding for short lines
            let missing = bytes_per_line - chunk.len();
            for j in 0..missing {
                if (chunk.len() + j) % self.config.group_size == 0 {
                    serial.write_str(" ");
                }
                serial.write_str("  ");
            }

            // ASCII
            if self.config.show_ascii {
                serial.write_str("  |");
                for &byte in chunk {
                    if byte.is_ascii_graphic() || byte == b' ' {
                        let c = [byte];
                        serial.write_str(core::str::from_utf8(&c).unwrap());
                    } else {
                        serial.write_str(".");
                    }
                }
                for _ in 0..missing {
                    serial.write_str(" ");
                }
                serial.write_str("|");
            }

            serial.write_str("\n");
        }
    }
}

// =============================================================================
// STACK TRACE (PLACEHOLDER)
// =============================================================================

/// Stack frame
#[derive(Debug, Clone, Copy)]
pub struct StackFrame {
    /// Return address
    pub return_addr: u64,
    /// Frame pointer
    pub frame_ptr: u64,
}

/// Walk stack frames (x86_64)
#[cfg(target_arch = "x86_64")]
pub fn walk_stack() -> impl Iterator<Item = StackFrame> {
    struct StackWalker {
        frame_ptr: u64,
        count: usize,
    }

    impl Iterator for StackWalker {
        type Item = StackFrame;

        fn next(&mut self) -> Option<Self::Item> {
            if self.frame_ptr == 0 || self.count > 64 {
                return None;
            }

            // Safety: We're walking the stack, assuming valid frame pointers
            let frame_ptr = self.frame_ptr;
            let return_addr = unsafe { *((frame_ptr + 8) as *const u64) };
            let next_frame = unsafe { *(frame_ptr as *const u64) };

            self.frame_ptr = next_frame;
            self.count += 1;

            Some(StackFrame {
                return_addr,
                frame_ptr,
            })
        }
    }

    let rbp: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, rbp",
            out(reg) rbp,
            options(nomem, nostack)
        );
    }

    StackWalker {
        frame_ptr: rbp,
        count: 0,
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub fn walk_stack() -> impl Iterator<Item = StackFrame> {
    core::iter::empty()
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Fatal);
    }

    #[test]
    fn test_format_hex64() {
        let mut buf = [0u8; 18];
        assert_eq!(format_hex64(0xDEADBEEF, &mut buf), "0x00000000DEADBEEF");
    }

    #[test]
    fn test_array_writer() {
        let mut buf = [0u8; 32];
        let mut writer = ArrayWriter::new(&mut buf);

        write!(writer, "Hello, {}!", "world").unwrap();
        assert_eq!(writer.as_str(), "Hello, world!");
    }

    #[test]
    fn test_baud_rate() {
        assert_eq!(BaudRate::B115200.divisor(), 1);
        assert_eq!(BaudRate::B9600.divisor(), 12);
    }
}
