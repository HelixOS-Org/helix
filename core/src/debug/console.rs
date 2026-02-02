//! # Debug Console
//!
//! Kernel debug console for early debugging.

use core::fmt::{self, Write};

use spin::Mutex;

/// Console writer trait
pub trait ConsoleWriter: Send {
    /// Write a byte to the console
    fn write_byte(&mut self, byte: u8);

    /// Write a string to the console
    fn write_str(&mut self, s: &str) {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
    }

    /// Flush the console
    fn flush(&mut self) {}
}

/// Null console (discards output)
struct NullConsole;

impl ConsoleWriter for NullConsole {
    fn write_byte(&mut self, _byte: u8) {}
}

/// Global console writer
static CONSOLE: Mutex<Option<&'static mut dyn ConsoleWriter>> = Mutex::new(None);

/// Early console for before the real console is set up
static EARLY_CONSOLE: Mutex<Option<&'static mut dyn ConsoleWriter>> = Mutex::new(None);

/// Set the console writer
pub fn set_console(writer: &'static mut dyn ConsoleWriter) {
    *CONSOLE.lock() = Some(writer);
}

/// Set the early console writer
pub fn set_early_console(writer: &'static mut dyn ConsoleWriter) {
    *EARLY_CONSOLE.lock() = Some(writer);
}

/// Print to the console
pub fn print(args: fmt::Arguments) {
    if let Some(console) = CONSOLE.lock().as_mut() {
        let _ = write_to_console(*console, args);
    } else if let Some(console) = EARLY_CONSOLE.lock().as_mut() {
        let _ = write_to_console(*console, args);
    }
}

/// Helper to write formatted output to a console
fn write_to_console(console: &mut dyn ConsoleWriter, args: fmt::Arguments) -> fmt::Result {
    // Wrapper to implement Write for a ConsoleWriter reference
    struct ConsoleWriteAdapter<'a>(&'a mut dyn ConsoleWriter);

    impl<'a> Write for ConsoleWriteAdapter<'a> {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            ConsoleWriter::write_str(self.0, s);
            Ok(())
        }
    }

    ConsoleWriteAdapter(console).write_fmt(args)
}

/// Print macro
#[macro_export]
macro_rules! kprint {
    ($($arg:tt)*) => {
        $crate::debug::console::print(format_args!($($arg)*))
    };
}

/// Print line macro
#[macro_export]
macro_rules! kprintln {
    () => {
        $crate::kprint!("\n")
    };
    ($($arg:tt)*) => {
        $crate::kprint!("{}\n", format_args!($($arg)*))
    };
}

/// Debug print macro (only in debug builds)
#[macro_export]
macro_rules! kdebug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::kprintln!("[DEBUG] {}", format_args!($($arg)*))
    };
}

/// Info print macro
#[macro_export]
macro_rules! kinfo {
    ($($arg:tt)*) => {
        $crate::kprintln!("[INFO] {}", format_args!($($arg)*))
    };
}

/// Warning print macro
#[macro_export]
macro_rules! kwarn {
    ($($arg:tt)*) => {
        $crate::kprintln!("[WARN] {}", format_args!($($arg)*))
    };
}

/// Error print macro
#[macro_export]
macro_rules! kerror {
    ($($arg:tt)*) => {
        $crate::kprintln!("[ERROR] {}", format_args!($($arg)*))
    };
}
