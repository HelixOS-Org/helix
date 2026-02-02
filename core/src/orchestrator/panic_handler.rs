//! # Panic Handler
//!
//! Kernel panic handling and recovery mechanisms.

use core::panic::PanicInfo;
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use crate::{set_kernel_state, KernelState};

/// Maximum number of nested panics before giving up
const MAX_PANIC_DEPTH: usize = 3;

static PANIC_DEPTH: AtomicUsize = AtomicUsize::new(0);
static IN_PANIC_HANDLER: AtomicBool = AtomicBool::new(false);

/// Panic action to take
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanicAction {
    /// Halt the system
    Halt,
    /// Reboot the system
    Reboot,
    /// Enter debugger
    Debug,
    /// Attempt recovery
    Recover,
}

/// Panic context information
#[derive(Debug)]
pub struct PanicContext {
    /// CPU that panicked
    pub cpu: usize,
    /// Panic message
    pub message: &'static str,
    /// File where panic occurred
    pub file: &'static str,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Stack trace (if available)
    pub stack_trace: Option<&'static [usize]>,
}

/// Panic handler trait for custom panic handling
pub trait PanicHandler: Send + Sync {
    /// Handle a kernel panic
    fn handle_panic(&self, context: &PanicContext) -> PanicAction;

    /// Called before the system halts
    fn pre_halt(&self) {}
}

/// The kernel panic handler
pub fn kernel_panic_handler(info: &PanicInfo) -> ! {
    // Prevent recursive panics
    let depth = PANIC_DEPTH.fetch_add(1, Ordering::SeqCst);

    if depth >= MAX_PANIC_DEPTH {
        // Too many nested panics, just halt
        loop {
            core::hint::spin_loop();
        }
    }

    // Prevent concurrent panic handling
    if IN_PANIC_HANDLER.swap(true, Ordering::SeqCst) {
        loop {
            core::hint::spin_loop();
        }
    }

    set_kernel_state(KernelState::Panic);

    // Try to print panic information
    if let Some(location) = info.location() {
        log::error!(
            "KERNEL PANIC at {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }

    // Print the panic message
    log::error!("Message: {}", info.message());

    // Print stack trace if possible
    print_stack_trace();

    // TODO: Call registered panic handlers

    // Halt
    log::error!("System halted");

    loop {
        core::hint::spin_loop();
    }
}

/// Print a stack trace (placeholder)
fn print_stack_trace() {
    log::error!("Stack trace:");
    log::error!("  <stack trace not yet implemented>");

    // TODO: Implement actual stack walking
    // This requires frame pointer or DWARF unwinding support
}

/// Register a panic hook
pub fn register_panic_hook(handler: &'static dyn PanicHandler) {
    // TODO: Store the handler for later use
    let _ = handler;
}

/// Trigger a kernel panic programmatically
#[inline(never)]
pub fn panic(message: &str) -> ! {
    panic!("{}", message);
}

/// Assert a condition, panic if false
#[macro_export]
macro_rules! kernel_assert {
    ($cond:expr) => {
        if !$cond {
            $crate::orchestrator::panic_handler::panic(
                concat!("assertion failed: ", stringify!($cond))
            );
        }
    };
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            $crate::orchestrator::panic_handler::panic(
                &alloc::format!($($arg)*)
            );
        }
    };
}

/// Debug assertion (only in debug builds)
#[macro_export]
macro_rules! kernel_debug_assert {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::kernel_assert!($($arg)*);
    };
}
