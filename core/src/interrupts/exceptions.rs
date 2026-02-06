//! # Exception Handling
//!
//! CPU exception handling framework.

use alloc::sync::Arc;

use helix_hal::interrupts::{Exception, PageFaultInfo};
use spin::RwLock;

/// Exception handler trait
pub trait ExceptionHandler: Send + Sync {
    /// Handle an exception
    ///
    /// Returns true if the exception was handled, false otherwise
    fn handle(&self, exception: Exception, info: &ExceptionInfo) -> bool;
}

/// Exception information
pub struct ExceptionInfo {
    /// Instruction pointer
    pub ip: u64,
    /// Stack pointer
    pub sp: u64,
    /// Error code (if any)
    pub error_code: Option<u64>,
    /// Was this from user mode?
    pub from_user: bool,
    /// Page fault info (if page fault)
    pub page_fault: Option<PageFaultInfo>,
    /// CPU that raised the exception
    pub cpu: usize,
}

/// Exception dispatcher
pub struct ExceptionDispatcher {
    /// Handlers by exception type
    handlers: RwLock<[Option<Arc<dyn ExceptionHandler>>; 32]>,
    /// Default handler
    default_handler: RwLock<Option<Arc<dyn ExceptionHandler>>>,
}

impl Default for ExceptionDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl ExceptionDispatcher {
    /// Create a new dispatcher
    pub const fn new() -> Self {
        Self {
            handlers: RwLock::new([const { None }; 32]),
            default_handler: RwLock::new(None),
        }
    }

    /// Register an exception handler
    pub fn register(&self, exception: Exception, handler: Arc<dyn ExceptionHandler>) {
        let idx = exception_to_index(exception);
        if idx < 32 {
            self.handlers.write()[idx] = Some(handler);
        }
    }

    /// Set the default handler
    pub fn set_default_handler(&self, handler: Arc<dyn ExceptionHandler>) {
        *self.default_handler.write() = Some(handler);
    }

    /// Dispatch an exception
    pub fn dispatch(&self, exception: Exception, info: &ExceptionInfo) -> bool {
        let idx = exception_to_index(exception);

        // Try specific handler
        if idx < 32 {
            if let Some(handler) = &self.handlers.read()[idx] {
                if handler.handle(exception, info) {
                    return true;
                }
            }
        }

        // Try default handler
        if let Some(handler) = &*self.default_handler.read() {
            return handler.handle(exception, info);
        }

        false
    }
}

fn exception_to_index(exception: Exception) -> usize {
    match exception {
        Exception::DivisionByZero => 0,
        Exception::Debug => 1,
        Exception::NonMaskable => 2,
        Exception::Breakpoint => 3,
        Exception::Overflow => 4,
        Exception::BoundRangeExceeded => 5,
        Exception::InvalidOpcode => 6,
        Exception::DeviceNotAvailable => 7,
        Exception::DoubleFault => 8,
        Exception::InvalidTss => 10,
        Exception::SegmentNotPresent => 11,
        Exception::StackSegmentFault => 12,
        Exception::GeneralProtectionFault => 13,
        Exception::PageFault => 14,
        Exception::FloatingPoint => 16,
        Exception::AlignmentCheck => 17,
        Exception::MachineCheck => 18,
        Exception::Simd => 19,
        Exception::Virtualization => 20,
        Exception::Security => 30,
        Exception::Unknown(n) => n as usize,
    }
}

/// Global exception dispatcher
static DISPATCHER: ExceptionDispatcher = ExceptionDispatcher::new();

/// Get the exception dispatcher
pub fn dispatcher() -> &'static ExceptionDispatcher {
    &DISPATCHER
}
