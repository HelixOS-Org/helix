//! # Helix OS Early Boot - Device Drivers Module
//!
//! This module provides unified device driver abstractions for early boot.
//! These drivers are designed to work before the full kernel driver subsystem
//! is available, providing essential I/O capabilities for debugging and
//! initialization.
//!
//! ## Architecture
//!
//! The drivers module provides:
//! - **Generic Console**: Unified console abstraction across all platforms
//! - **Framebuffer**: Early graphical output for boot splash and diagnostics
//! - **Storage**: Minimal block device access for loading additional modules
//! - **Input**: Basic keyboard input for recovery mode
//!
//! ## Design Principles
//!
//! 1. **Minimal Dependencies**: Drivers work with `#![no_std]` and no allocator
//! 2. **Polling Mode**: All I/O is polling-based, no interrupt handlers required
//! 3. **Fail-Safe**: Graceful degradation when hardware is unavailable
//! 4. **Cross-Platform**: Unified APIs with platform-specific implementations
//!
//! ## Usage
//!
//! ```rust,ignore
//! use helix_early_boot::drivers::{Console, Framebuffer};
//!
//! // Initialize console (auto-detects platform)
//! let console = Console::init()?;
//! console.write_str("Helix OS booting...\n");
//!
//! // Initialize framebuffer if available
//! if let Some(fb) = Framebuffer::init()? {
//!     fb.clear(Color::BLACK);
//!     fb.draw_logo(100, 100);
//! }
//! ```

#![allow(dead_code)]

// Sub-modules
pub mod console;
pub mod framebuffer;

// Re-exports
pub use console::{Console, ConsoleBackend, ConsoleColor, ConsoleWriter};
pub use framebuffer::{Color, Framebuffer, FramebufferInfo, PixelFormat};

use crate::error::{BootError, BootResult};
use crate::info::BootInfo;

// =============================================================================
// DRIVER MANAGER
// =============================================================================

/// Early boot driver manager
///
/// Coordinates initialization and access to all early boot drivers.
pub struct DriverManager {
    /// Console driver (always required)
    console: Option<Console>,

    /// Framebuffer driver (optional)
    framebuffer: Option<Framebuffer>,

    /// Initialization flags
    flags: DriverFlags,
}

bitflags::bitflags! {
    /// Driver initialization flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DriverFlags: u32 {
        /// Console initialized
        const CONSOLE_INIT = 1 << 0;
        /// Framebuffer initialized
        const FRAMEBUFFER_INIT = 1 << 1;
        /// Serial output available
        const SERIAL_AVAILABLE = 1 << 2;
        /// VGA text mode available
        const VGA_TEXT_AVAILABLE = 1 << 3;
        /// Graphical framebuffer available
        const GRAPHICS_AVAILABLE = 1 << 4;
        /// Keyboard input available
        const KEYBOARD_AVAILABLE = 1 << 5;
    }
}

impl DriverManager {
    /// Create a new driver manager
    pub const fn new() -> Self {
        Self {
            console: None,
            framebuffer: None,
            flags: DriverFlags::empty(),
        }
    }

    /// Initialize all early boot drivers
    ///
    /// # Arguments
    /// * `boot_info` - Boot information from bootloader
    ///
    /// # Returns
    /// * `Ok(())` on success
    /// * `Err(BootError)` if critical drivers fail
    pub fn init(&mut self, boot_info: &BootInfo) -> BootResult<()> {
        // Initialize console first (required for error reporting)
        self.init_console(boot_info)?;

        // Try to initialize framebuffer (optional)
        if let Err(e) = self.init_framebuffer(boot_info) {
            // Log but don't fail - framebuffer is optional
            if let Some(ref console) = self.console {
                console.write_str("Warning: Framebuffer init failed\n");
            }
        }

        Ok(())
    }

    /// Initialize console driver
    fn init_console(&mut self, boot_info: &BootInfo) -> BootResult<()> {
        let console = Console::init(boot_info)?;
        self.console = Some(console);
        self.flags.insert(DriverFlags::CONSOLE_INIT);

        // Check what backends are available
        if let Some(ref c) = self.console {
            if c.has_serial() {
                self.flags.insert(DriverFlags::SERIAL_AVAILABLE);
            }
            if c.has_vga() {
                self.flags.insert(DriverFlags::VGA_TEXT_AVAILABLE);
            }
        }

        Ok(())
    }

    /// Initialize framebuffer driver
    fn init_framebuffer(&mut self, boot_info: &BootInfo) -> BootResult<()> {
        let fb = Framebuffer::init(boot_info)?;
        self.framebuffer = Some(fb);
        self.flags.insert(DriverFlags::FRAMEBUFFER_INIT);
        self.flags.insert(DriverFlags::GRAPHICS_AVAILABLE);
        Ok(())
    }

    /// Get console reference
    pub fn console(&self) -> Option<&Console> {
        self.console.as_ref()
    }

    /// Get mutable console reference
    pub fn console_mut(&mut self) -> Option<&mut Console> {
        self.console.as_mut()
    }

    /// Get framebuffer reference
    pub fn framebuffer(&self) -> Option<&Framebuffer> {
        self.framebuffer.as_ref()
    }

    /// Get mutable framebuffer reference
    pub fn framebuffer_mut(&mut self) -> Option<&mut Framebuffer> {
        self.framebuffer.as_mut()
    }

    /// Get driver flags
    pub fn flags(&self) -> DriverFlags {
        self.flags
    }

    /// Check if console is available
    pub fn has_console(&self) -> bool {
        self.flags.contains(DriverFlags::CONSOLE_INIT)
    }

    /// Check if framebuffer is available
    pub fn has_framebuffer(&self) -> bool {
        self.flags.contains(DriverFlags::FRAMEBUFFER_INIT)
    }

    /// Write to console
    pub fn write_console(&self, s: &str) {
        if let Some(ref console) = self.console {
            console.write_str(s);
        }
    }

    /// Write formatted to console
    pub fn write_console_fmt(&self, args: core::fmt::Arguments) {
        if let Some(ref console) = self.console {
            console.write_fmt(args);
        }
    }
}

impl Default for DriverManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// GLOBAL DRIVER ACCESS
// =============================================================================

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

/// Global driver manager instance
static mut DRIVER_MANAGER: UnsafeCell<DriverManager> = UnsafeCell::new(DriverManager::new());
static DRIVER_MANAGER_INIT: AtomicBool = AtomicBool::new(false);

/// Initialize global driver manager
///
/// # Safety
/// Must be called exactly once during boot, from BSP only.
pub unsafe fn init_drivers(boot_info: &BootInfo) -> BootResult<()> {
    if DRIVER_MANAGER_INIT.swap(true, Ordering::SeqCst) {
        return Err(BootError::AlreadyInitialized);
    }

    let manager = &mut *DRIVER_MANAGER.get();
    manager.init(boot_info)
}

/// Get reference to global driver manager
///
/// # Safety
/// Drivers must be initialized first.
pub unsafe fn drivers() -> &'static DriverManager {
    &*DRIVER_MANAGER.get()
}

/// Get mutable reference to global driver manager
///
/// # Safety
/// Drivers must be initialized first. Must ensure exclusive access.
pub unsafe fn drivers_mut() -> &'static mut DriverManager {
    &mut *DRIVER_MANAGER.get()
}

/// Write to global console
///
/// This is safe to call before initialization (will do nothing).
pub fn early_print(s: &str) {
    if DRIVER_MANAGER_INIT.load(Ordering::SeqCst) {
        unsafe {
            drivers().write_console(s);
        }
    }
}

/// Write formatted to global console
pub fn early_print_fmt(args: core::fmt::Arguments) {
    if DRIVER_MANAGER_INIT.load(Ordering::SeqCst) {
        unsafe {
            drivers().write_console_fmt(args);
        }
    }
}

// =============================================================================
// EARLY PRINT MACROS
// =============================================================================

/// Early print macro (no newline)
#[macro_export]
macro_rules! early_print {
    ($($arg:tt)*) => {
        $crate::drivers::early_print_fmt(format_args!($($arg)*))
    };
}

/// Early println macro (with newline)
#[macro_export]
macro_rules! early_println {
    () => {
        $crate::drivers::early_print("\n")
    };
    ($($arg:tt)*) => {
        $crate::drivers::early_print_fmt(format_args!($($arg)*));
        $crate::drivers::early_print("\n")
    };
}

/// Early info log
#[macro_export]
macro_rules! early_info {
    ($($arg:tt)*) => {
        $crate::early_print!("[INFO] ");
        $crate::early_println!($($arg)*)
    };
}

/// Early warning log
#[macro_export]
macro_rules! early_warn {
    ($($arg:tt)*) => {
        $crate::early_print!("[WARN] ");
        $crate::early_println!($($arg)*)
    };
}

/// Early error log
#[macro_export]
macro_rules! early_error {
    ($($arg:tt)*) => {
        $crate::early_print!("[ERROR] ");
        $crate::early_println!($($arg)*)
    };
}

/// Early debug log (only in debug builds)
#[macro_export]
macro_rules! early_debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            $crate::early_print!("[DEBUG] ");
            $crate::early_println!($($arg)*)
        }
    };
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_manager_new() {
        let dm = DriverManager::new();
        assert!(!dm.has_console());
        assert!(!dm.has_framebuffer());
        assert!(dm.flags().is_empty());
    }

    #[test]
    fn test_driver_flags() {
        let mut flags = DriverFlags::empty();
        flags.insert(DriverFlags::CONSOLE_INIT);
        flags.insert(DriverFlags::SERIAL_AVAILABLE);

        assert!(flags.contains(DriverFlags::CONSOLE_INIT));
        assert!(flags.contains(DriverFlags::SERIAL_AVAILABLE));
        assert!(!flags.contains(DriverFlags::FRAMEBUFFER_INIT));
    }
}
