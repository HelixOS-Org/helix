//! # Helix OS Early Boot - Console Driver
//!
//! This module provides a unified console abstraction for early boot output.
//! It supports multiple backends (serial, VGA text, framebuffer console) and
//! provides a consistent API across all platforms.
//!
//! ## Backends
//!
//! - **Serial**: UART/COM port output (all platforms)
//! - **VGA Text**: 80x25 text mode (x86_64 legacy)
//! - **Framebuffer Console**: Text rendering on graphical framebuffer
//! - **SBI Console**: RISC-V SBI console interface
//!
//! ## Features
//!
//! - Color support (16 colors for text mode, full color for framebuffer)
//! - Scrolling and cursor management
//! - Multiple output backends (can write to all simultaneously)
//! - Safe API with platform-specific implementations

#![allow(dead_code)]

use core::fmt::{self, Write};

use crate::error::{BootError, BootResult};
use crate::info::BootInfo;

// =============================================================================
// CONSOLE COLORS
// =============================================================================

/// Standard 16-color console palette
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConsoleColor {
    Black      = 0,
    Blue       = 1,
    Green      = 2,
    Cyan       = 3,
    Red        = 4,
    Magenta    = 5,
    Brown      = 6,
    LightGray  = 7,
    DarkGray   = 8,
    LightBlue  = 9,
    LightGreen = 10,
    LightCyan  = 11,
    LightRed   = 12,
    Pink       = 13,
    Yellow     = 14,
    White      = 15,
}

impl ConsoleColor {
    /// Convert to VGA text mode attribute
    pub fn to_vga_attr(self, bg: ConsoleColor) -> u8 {
        (bg as u8) << 4 | (self as u8)
    }

    /// Convert to RGB888
    pub fn to_rgb(self) -> (u8, u8, u8) {
        match self {
            ConsoleColor::Black => (0, 0, 0),
            ConsoleColor::Blue => (0, 0, 170),
            ConsoleColor::Green => (0, 170, 0),
            ConsoleColor::Cyan => (0, 170, 170),
            ConsoleColor::Red => (170, 0, 0),
            ConsoleColor::Magenta => (170, 0, 170),
            ConsoleColor::Brown => (170, 85, 0),
            ConsoleColor::LightGray => (170, 170, 170),
            ConsoleColor::DarkGray => (85, 85, 85),
            ConsoleColor::LightBlue => (85, 85, 255),
            ConsoleColor::LightGreen => (85, 255, 85),
            ConsoleColor::LightCyan => (85, 255, 255),
            ConsoleColor::LightRed => (255, 85, 85),
            ConsoleColor::Pink => (255, 85, 255),
            ConsoleColor::Yellow => (255, 255, 85),
            ConsoleColor::White => (255, 255, 255),
        }
    }

    /// Convert to RGB32 (0x00RRGGBB)
    pub fn to_rgb32(self) -> u32 {
        let (r, g, b) = self.to_rgb();
        ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }
}

impl Default for ConsoleColor {
    fn default() -> Self {
        ConsoleColor::LightGray
    }
}

// =============================================================================
// CONSOLE BACKEND TRAIT
// =============================================================================

/// Console backend trait
///
/// Implemented by each platform-specific console driver.
pub trait ConsoleBackend: Send + Sync {
    /// Get backend name
    fn name(&self) -> &'static str;

    /// Check if backend is available
    fn is_available(&self) -> bool;

    /// Write a single byte
    fn write_byte(&self, byte: u8);

    /// Write a string
    fn write_str(&self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// Write bytes
    fn write_bytes(&self, bytes: &[u8]) {
        for &byte in bytes {
            self.write_byte(byte);
        }
    }

    /// Read a byte (if input is supported)
    fn read_byte(&self) -> Option<u8> {
        None
    }

    /// Check if input is available
    fn input_available(&self) -> bool {
        false
    }

    /// Set foreground color (if supported)
    fn set_foreground(&mut self, _color: ConsoleColor) {}

    /// Set background color (if supported)
    fn set_background(&mut self, _color: ConsoleColor) {}

    /// Clear screen (if supported)
    fn clear(&mut self) {}

    /// Set cursor position (if supported)
    fn set_cursor(&mut self, _x: usize, _y: usize) {}

    /// Get cursor position
    fn get_cursor(&self) -> (usize, usize) {
        (0, 0)
    }

    /// Get console dimensions
    fn dimensions(&self) -> (usize, usize) {
        (80, 25)
    }

    /// Scroll up by n lines
    fn scroll_up(&mut self, _lines: usize) {}

    /// Enable cursor (if supported)
    fn enable_cursor(&mut self, _enable: bool) {}
}

// =============================================================================
// CONSOLE WRITER
// =============================================================================

/// Console writer implementing core::fmt::Write
pub struct ConsoleWriter<'a> {
    console: &'a Console,
}

impl<'a> ConsoleWriter<'a> {
    pub fn new(console: &'a Console) -> Self {
        Self { console }
    }
}

impl Write for ConsoleWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.console.write_str(s);
        Ok(())
    }
}

// =============================================================================
// MAIN CONSOLE
// =============================================================================

/// Unified console for early boot output
///
/// Manages multiple backends and provides a consistent API.
pub struct Console {
    /// Serial backend
    serial: Option<SerialConsole>,

    /// VGA text mode backend (x86_64 only)
    #[cfg(target_arch = "x86_64")]
    vga: Option<VgaConsole>,

    /// SBI console backend (RISC-V only)
    #[cfg(target_arch = "riscv64")]
    sbi: Option<SbiConsole>,

    /// Current foreground color
    fg_color: ConsoleColor,

    /// Current background color
    bg_color: ConsoleColor,

    /// Echo to all backends
    echo_all: bool,
}

impl Console {
    /// Initialize console with auto-detection
    pub fn init(boot_info: &BootInfo) -> BootResult<Self> {
        let mut console = Self {
            serial: None,
            #[cfg(target_arch = "x86_64")]
            vga: None,
            #[cfg(target_arch = "riscv64")]
            sbi: None,
            fg_color: ConsoleColor::LightGray,
            bg_color: ConsoleColor::Black,
            echo_all: true,
        };

        // Initialize serial console (platform-specific base address)
        console.init_serial(boot_info);

        // Initialize platform-specific backends
        #[cfg(target_arch = "x86_64")]
        console.init_vga(boot_info);

        #[cfg(target_arch = "riscv64")]
        console.init_sbi(boot_info);

        // Ensure at least one backend is available
        if !console.has_any_backend() {
            return Err(BootError::HardwareNotFound);
        }

        Ok(console)
    }

    /// Initialize serial console
    fn init_serial(&mut self, _boot_info: &BootInfo) {
        // Platform-specific serial initialization
        #[cfg(target_arch = "x86_64")]
        {
            // COM1 at 0x3F8
            self.serial = Some(SerialConsole::new(0x3F8));
        }

        #[cfg(target_arch = "aarch64")]
        {
            // PL011 at QEMU virt default
            self.serial = Some(SerialConsole::new(0x0900_0000));
        }

        #[cfg(target_arch = "riscv64")]
        {
            // 16550 UART at QEMU virt default
            self.serial = Some(SerialConsole::new(0x1000_0000));
        }
    }

    /// Initialize VGA console (x86_64 only)
    #[cfg(target_arch = "x86_64")]
    fn init_vga(&mut self, _boot_info: &BootInfo) {
        // VGA text buffer at 0xB8000
        self.vga = Some(VgaConsole::new(0xB8000));
    }

    /// Initialize SBI console (RISC-V only)
    #[cfg(target_arch = "riscv64")]
    fn init_sbi(&mut self, _boot_info: &BootInfo) {
        self.sbi = Some(SbiConsole::new());
    }

    /// Check if any backend is available
    fn has_any_backend(&self) -> bool {
        if self.serial.is_some() {
            return true;
        }

        #[cfg(target_arch = "x86_64")]
        if self.vga.is_some() {
            return true;
        }

        #[cfg(target_arch = "riscv64")]
        if self.sbi.is_some() {
            return true;
        }

        false
    }

    /// Check if serial is available
    pub fn has_serial(&self) -> bool {
        self.serial.is_some()
    }

    /// Check if VGA is available
    #[cfg(target_arch = "x86_64")]
    pub fn has_vga(&self) -> bool {
        self.vga.is_some()
    }

    #[cfg(not(target_arch = "x86_64"))]
    pub fn has_vga(&self) -> bool {
        false
    }

    /// Write a single byte
    pub fn write_byte(&self, byte: u8) {
        if let Some(ref serial) = self.serial {
            serial.write_byte(byte);
        }

        #[cfg(target_arch = "x86_64")]
        if let Some(ref vga) = self.vga {
            vga.write_byte(byte);
        }

        #[cfg(target_arch = "riscv64")]
        if let Some(ref sbi) = self.sbi {
            sbi.write_byte(byte);
        }
    }

    /// Write a string
    pub fn write_str(&self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// Write formatted output
    pub fn write_fmt(&self, args: fmt::Arguments) {
        let mut writer = ConsoleWriter::new(self);
        let _ = fmt::write(&mut writer, args);
    }

    /// Read a byte (from serial if available)
    pub fn read_byte(&self) -> Option<u8> {
        if let Some(ref serial) = self.serial {
            return serial.read_byte();
        }

        #[cfg(target_arch = "riscv64")]
        if let Some(ref sbi) = self.sbi {
            return sbi.read_byte();
        }

        None
    }

    /// Check if input is available
    pub fn input_available(&self) -> bool {
        if let Some(ref serial) = self.serial {
            return serial.input_available();
        }

        #[cfg(target_arch = "riscv64")]
        if let Some(ref sbi) = self.sbi {
            return sbi.input_available();
        }

        false
    }

    /// Set foreground color
    pub fn set_foreground(&mut self, color: ConsoleColor) {
        self.fg_color = color;

        #[cfg(target_arch = "x86_64")]
        if let Some(ref mut vga) = self.vga {
            vga.set_foreground(color);
        }
    }

    /// Set background color
    pub fn set_background(&mut self, color: ConsoleColor) {
        self.bg_color = color;

        #[cfg(target_arch = "x86_64")]
        if let Some(ref mut vga) = self.vga {
            vga.set_background(color);
        }
    }

    /// Clear all backends
    pub fn clear(&mut self) {
        #[cfg(target_arch = "x86_64")]
        if let Some(ref mut vga) = self.vga {
            vga.clear();
        }
    }

    /// Print with color
    pub fn print_colored(&mut self, s: &str, fg: ConsoleColor, bg: ConsoleColor) {
        let old_fg = self.fg_color;
        let old_bg = self.bg_color;

        self.set_foreground(fg);
        self.set_background(bg);
        self.write_str(s);

        self.set_foreground(old_fg);
        self.set_background(old_bg);
    }

    /// Print status line
    pub fn print_status(&mut self, label: &str, status: &str, ok: bool) {
        self.write_str("[ ");
        if ok {
            self.print_colored("OK", ConsoleColor::LightGreen, ConsoleColor::Black);
        } else {
            self.print_colored("FAIL", ConsoleColor::LightRed, ConsoleColor::Black);
        }
        self.write_str(" ] ");
        self.write_str(label);
        self.write_str(": ");
        self.write_str(status);
        self.write_str("\n");
    }

    /// Print boot stage header
    pub fn print_stage(&mut self, stage: &str) {
        self.print_colored("==> ", ConsoleColor::LightCyan, ConsoleColor::Black);
        self.write_str(stage);
        self.write_str("\n");
    }

    /// Print boot progress
    pub fn print_progress(&mut self, current: usize, total: usize, label: &str) {
        self.write_str("[");
        let filled = (current * 20) / total;
        for i in 0..20 {
            if i < filled {
                self.write_str("=");
            } else if i == filled {
                self.write_str(">");
            } else {
                self.write_str(" ");
            }
        }
        self.write_str("] ");
        // Print percentage
        let percent = (current * 100) / total;
        self.write_fmt(format_args!("{:3}% {}\n", percent, label));
    }

    /// Get a writer for this console
    pub fn writer(&self) -> ConsoleWriter<'_> {
        ConsoleWriter::new(self)
    }
}

// =============================================================================
// SERIAL CONSOLE BACKEND
// =============================================================================

/// Serial console backend
pub struct SerialConsole {
    base: u64,
    initialized: bool,
}

impl SerialConsole {
    /// Create new serial console
    pub fn new(base: u64) -> Self {
        let mut console = Self {
            base,
            initialized: false,
        };
        console.init();
        console
    }

    /// Initialize serial port
    fn init(&mut self) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            // x86_64: COM port at I/O port address
            use core::arch::asm;
            let port = self.base as u16;

            // Disable interrupts
            asm!("out dx, al", in("dx") port + 1, in("al") 0u8);
            // Enable DLAB
            asm!("out dx, al", in("dx") port + 3, in("al") 0x80u8);
            // Set baud rate divisor to 1 (115200 baud)
            asm!("out dx, al", in("dx") port, in("al") 1u8);
            asm!("out dx, al", in("dx") port + 1, in("al") 0u8);
            // 8 bits, no parity, one stop bit
            asm!("out dx, al", in("dx") port + 3, in("al") 0x03u8);
            // Enable FIFO, clear them, 14-byte threshold
            asm!("out dx, al", in("dx") port + 2, in("al") 0xC7u8);
            // IRQs disabled, RTS/DTR set
            asm!("out dx, al", in("dx") port + 4, in("al") 0x03u8);
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            // PL011 initialization
            let base = self.base as *mut u32;

            // Disable UART
            core::ptr::write_volatile(base.add(12), 0);

            // Set baud rate (assuming 24MHz clock, 115200 baud)
            // IBRD = 24000000 / (16 * 115200) = 13
            // FBRD = ((24000000 % (16 * 115200)) * 64) / (16 * 115200) = 1
            core::ptr::write_volatile(base.add(9), 13); // UARTIBRD
            core::ptr::write_volatile(base.add(10), 1); // UARTFBRD

            // 8 bits, FIFO enabled
            core::ptr::write_volatile(base.add(11), 0x70); // UARTLCR_H

            // Enable UART, TX, RX
            core::ptr::write_volatile(base.add(12), 0x301); // UARTCR
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            // 16550 UART initialization
            let base = self.base as *mut u8;

            // Disable interrupts
            core::ptr::write_volatile(base.add(1), 0);

            // Enable DLAB
            core::ptr::write_volatile(base.add(3), 0x80);

            // Set baud rate divisor (assuming 1.8432 MHz clock)
            core::ptr::write_volatile(base, 1);
            core::ptr::write_volatile(base.add(1), 0);

            // 8 bits, no parity, 1 stop bit
            core::ptr::write_volatile(base.add(3), 0x03);

            // Enable FIFO
            core::ptr::write_volatile(base.add(2), 0xC7);

            // RTS/DTR
            core::ptr::write_volatile(base.add(4), 0x0B);
        }

        self.initialized = true;
    }

    /// Write a byte to serial
    fn write_byte(&self, byte: u8) {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use core::arch::asm;
            let port = self.base as u16;

            // Wait for transmit buffer empty
            loop {
                let mut status: u8;
                asm!("in al, dx", out("al") status, in("dx") port + 5);
                if status & 0x20 != 0 {
                    break;
                }
            }

            // Write byte
            asm!("out dx, al", in("dx") port, in("al") byte);
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let base = self.base as *mut u32;

            // Wait for transmit FIFO not full
            while core::ptr::read_volatile(base.add(6)) & (1 << 5) != 0 {
                core::hint::spin_loop();
            }

            // Write byte
            core::ptr::write_volatile(base, byte as u32);
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            let base = self.base as *mut u8;

            // Wait for transmit buffer empty (LSR bit 5)
            while core::ptr::read_volatile(base.add(5)) & 0x20 == 0 {
                core::hint::spin_loop();
            }

            // Write byte
            core::ptr::write_volatile(base, byte);
        }
    }

    /// Read a byte from serial
    fn read_byte(&self) -> Option<u8> {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use core::arch::asm;
            let port = self.base as u16;

            // Check if data available
            let mut status: u8;
            asm!("in al, dx", out("al") status, in("dx") port + 5);
            if status & 0x01 != 0 {
                let mut data: u8;
                asm!("in al, dx", out("al") data, in("dx") port);
                return Some(data);
            }
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let base = self.base as *mut u32;

            // Check receive FIFO not empty
            if core::ptr::read_volatile(base.add(6)) & (1 << 4) == 0 {
                return Some(core::ptr::read_volatile(base) as u8);
            }
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            let base = self.base as *mut u8;

            // Check if data available (LSR bit 0)
            if core::ptr::read_volatile(base.add(5)) & 0x01 != 0 {
                return Some(core::ptr::read_volatile(base));
            }
        }

        None
    }

    /// Check if input is available
    fn input_available(&self) -> bool {
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use core::arch::asm;
            let port = self.base as u16;
            let mut status: u8;
            asm!("in al, dx", out("al") status, in("dx") port + 5);
            return status & 0x01 != 0;
        }

        #[cfg(target_arch = "aarch64")]
        unsafe {
            let base = self.base as *mut u32;
            return core::ptr::read_volatile(base.add(6)) & (1 << 4) == 0;
        }

        #[cfg(target_arch = "riscv64")]
        unsafe {
            let base = self.base as *mut u8;
            return core::ptr::read_volatile(base.add(5)) & 0x01 != 0;
        }

        #[allow(unreachable_code)]
        false
    }
}

impl ConsoleBackend for SerialConsole {
    fn name(&self) -> &'static str {
        "Serial"
    }

    fn is_available(&self) -> bool {
        self.initialized
    }

    fn write_byte(&self, byte: u8) {
        SerialConsole::write_byte(self, byte);
    }

    fn read_byte(&self) -> Option<u8> {
        SerialConsole::read_byte(self)
    }

    fn input_available(&self) -> bool {
        SerialConsole::input_available(self)
    }
}

// =============================================================================
// VGA TEXT MODE CONSOLE (x86_64)
// =============================================================================

#[cfg(target_arch = "x86_64")]
pub struct VgaConsole {
    base: u64,
    cursor_x: usize,
    cursor_y: usize,
    attr: u8,
}

#[cfg(target_arch = "x86_64")]
impl VgaConsole {
    const WIDTH: usize = 80;
    const HEIGHT: usize = 25;

    pub fn new(base: u64) -> Self {
        Self {
            base,
            cursor_x: 0,
            cursor_y: 0,
            attr: 0x07, // Light gray on black
        }
    }

    fn put_char(&self, x: usize, y: usize, ch: u8, attr: u8) {
        if x >= Self::WIDTH || y >= Self::HEIGHT {
            return;
        }

        let offset = (y * Self::WIDTH + x) * 2;
        unsafe {
            let ptr = (self.base as *mut u8).add(offset);
            core::ptr::write_volatile(ptr, ch);
            core::ptr::write_volatile(ptr.add(1), attr);
        }
    }

    fn scroll(&mut self) {
        unsafe {
            let base = self.base as *mut u8;

            // Copy lines up
            for y in 1..Self::HEIGHT {
                for x in 0..Self::WIDTH {
                    let src = ((y * Self::WIDTH) + x) * 2;
                    let dst = (((y - 1) * Self::WIDTH) + x) * 2;
                    let ch = core::ptr::read_volatile(base.add(src));
                    let attr = core::ptr::read_volatile(base.add(src + 1));
                    core::ptr::write_volatile(base.add(dst), ch);
                    core::ptr::write_volatile(base.add(dst + 1), attr);
                }
            }

            // Clear last line
            for x in 0..Self::WIDTH {
                let offset = ((Self::HEIGHT - 1) * Self::WIDTH + x) * 2;
                core::ptr::write_volatile(base.add(offset), b' ');
                core::ptr::write_volatile(base.add(offset + 1), self.attr);
            }
        }
    }

    fn update_cursor(&self) {
        let pos = self.cursor_y * Self::WIDTH + self.cursor_x;
        unsafe {
            use core::arch::asm;
            // Cursor location low
            asm!("out dx, al", in("dx") 0x3D4u16, in("al") 0x0Fu8);
            asm!("out dx, al", in("dx") 0x3D5u16, in("al") (pos & 0xFF) as u8);
            // Cursor location high
            asm!("out dx, al", in("dx") 0x3D4u16, in("al") 0x0Eu8);
            asm!("out dx, al", in("dx") 0x3D5u16, in("al") ((pos >> 8) & 0xFF) as u8);
        }
    }
}

#[cfg(target_arch = "x86_64")]
impl ConsoleBackend for VgaConsole {
    fn name(&self) -> &'static str {
        "VGA"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn write_byte(&self, byte: u8) {
        // Note: This needs mutable access, but trait uses &self for simplicity
        // In practice, we'd use interior mutability
        unsafe {
            let this = self as *const Self as *mut Self;

            match byte {
                b'\n' => {
                    (*this).cursor_x = 0;
                    (*this).cursor_y += 1;
                    if (*this).cursor_y >= Self::HEIGHT {
                        (*this).scroll();
                        (*this).cursor_y = Self::HEIGHT - 1;
                    }
                },
                b'\r' => {
                    (*this).cursor_x = 0;
                },
                b'\t' => {
                    (*this).cursor_x = ((*this).cursor_x + 8) & !7;
                    if (*this).cursor_x >= Self::WIDTH {
                        (*this).cursor_x = 0;
                        (*this).cursor_y += 1;
                        if (*this).cursor_y >= Self::HEIGHT {
                            (*this).scroll();
                            (*this).cursor_y = Self::HEIGHT - 1;
                        }
                    }
                },
                _ => {
                    (*this).put_char((*this).cursor_x, (*this).cursor_y, byte, (*this).attr);
                    (*this).cursor_x += 1;
                    if (*this).cursor_x >= Self::WIDTH {
                        (*this).cursor_x = 0;
                        (*this).cursor_y += 1;
                        if (*this).cursor_y >= Self::HEIGHT {
                            (*this).scroll();
                            (*this).cursor_y = Self::HEIGHT - 1;
                        }
                    }
                },
            }

            (*this).update_cursor();
        }
    }

    fn set_foreground(&mut self, color: ConsoleColor) {
        self.attr = (self.attr & 0xF0) | (color as u8);
    }

    fn set_background(&mut self, color: ConsoleColor) {
        self.attr = (self.attr & 0x0F) | ((color as u8) << 4);
    }

    fn clear(&mut self) {
        for y in 0..Self::HEIGHT {
            for x in 0..Self::WIDTH {
                self.put_char(x, y, b' ', self.attr);
            }
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.update_cursor();
    }

    fn set_cursor(&mut self, x: usize, y: usize) {
        self.cursor_x = x.min(Self::WIDTH - 1);
        self.cursor_y = y.min(Self::HEIGHT - 1);
        self.update_cursor();
    }

    fn get_cursor(&self) -> (usize, usize) {
        (self.cursor_x, self.cursor_y)
    }

    fn dimensions(&self) -> (usize, usize) {
        (Self::WIDTH, Self::HEIGHT)
    }

    fn scroll_up(&mut self, lines: usize) {
        for _ in 0..lines {
            self.scroll();
        }
    }
}

// =============================================================================
// SBI CONSOLE (RISC-V)
// =============================================================================

#[cfg(target_arch = "riscv64")]
pub struct SbiConsole {
    initialized: bool,
}

#[cfg(target_arch = "riscv64")]
impl SbiConsole {
    pub fn new() -> Self {
        Self { initialized: true }
    }

    /// Legacy SBI console putchar
    fn sbi_console_putchar(ch: u8) {
        unsafe {
            core::arch::asm!(
                "li a7, 1",      // SBI_CONSOLE_PUTCHAR
                "mv a0, {0}",
                "ecall",
                in(reg) ch as usize,
                out("a0") _,
                out("a7") _,
            );
        }
    }

    /// Legacy SBI console getchar
    fn sbi_console_getchar() -> isize {
        let ret: isize;
        unsafe {
            core::arch::asm!(
                "li a7, 2",      // SBI_CONSOLE_GETCHAR
                "ecall",
                out("a0") ret,
                out("a7") _,
            );
        }
        ret
    }
}

#[cfg(target_arch = "riscv64")]
impl Default for SbiConsole {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "riscv64")]
impl ConsoleBackend for SbiConsole {
    fn name(&self) -> &'static str {
        "SBI"
    }

    fn is_available(&self) -> bool {
        self.initialized
    }

    fn write_byte(&self, byte: u8) {
        Self::sbi_console_putchar(byte);
    }

    fn read_byte(&self) -> Option<u8> {
        let ch = Self::sbi_console_getchar();
        if ch >= 0 {
            Some(ch as u8)
        } else {
            None
        }
    }

    fn input_available(&self) -> bool {
        // SBI doesn't provide a non-blocking check, so we try to read
        // This is not ideal but works for early boot
        Self::sbi_console_getchar() >= 0
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_color_vga_attr() {
        let attr = ConsoleColor::White.to_vga_attr(ConsoleColor::Blue);
        assert_eq!(attr, 0x1F); // Blue bg (1) << 4 | White fg (15)
    }

    #[test]
    fn test_console_color_rgb() {
        let (r, g, b) = ConsoleColor::Red.to_rgb();
        assert_eq!(r, 170);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
    }

    #[test]
    fn test_console_color_rgb32() {
        let rgb = ConsoleColor::Blue.to_rgb32();
        assert_eq!(rgb, 0x0000AA);
    }
}
