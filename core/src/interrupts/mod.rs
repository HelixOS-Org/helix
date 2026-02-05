//! # Interrupt Management
//!
//! Interrupt routing and handling infrastructure.

pub mod exceptions;
pub mod handlers;
pub mod router;

use alloc::sync::Arc;
use alloc::vec::Vec;

use helix_hal::interrupts::InterruptVector;
use spin::RwLock;

/// Interrupt handler function type
pub type InterruptHandlerFn = dyn Fn(InterruptVector) + Send + Sync;

/// Interrupt handler entry
pub struct InterruptHandler {
    /// Handler function
    handler: Arc<InterruptHandlerFn>,
    /// Handler name (for debugging)
    name: &'static str,
    /// Is this handler shared (can coexist with others)?
    shared: bool,
}

/// Interrupt dispatcher
pub struct InterruptDispatcher {
    /// Handlers for each vector
    handlers: RwLock<[Vec<InterruptHandler>; 256]>,
}

impl InterruptDispatcher {
    /// Create a new dispatcher
    pub const fn new() -> Self {
        Self {
            handlers: RwLock::new([const { Vec::new() }; 256]),
        }
    }

    /// Register an interrupt handler
    pub fn register(
        &self,
        vector: InterruptVector,
        name: &'static str,
        handler: Arc<InterruptHandlerFn>,
        shared: bool,
    ) -> Result<(), &'static str> {
        let mut handlers = self.handlers.write();
        let vec_handlers = &mut handlers[vector as usize];

        // Check for conflicts
        if !shared && !vec_handlers.is_empty() {
            return Err("Interrupt vector already has a non-shared handler");
        }

        if !vec_handlers.is_empty() && !vec_handlers[0].shared {
            return Err("Interrupt vector has a non-shared handler");
        }

        vec_handlers.push(InterruptHandler {
            handler,
            name,
            shared,
        });

        Ok(())
    }

    /// Unregister an interrupt handler
    pub fn unregister(&self, vector: InterruptVector, name: &'static str) {
        let mut handlers = self.handlers.write();
        handlers[vector as usize].retain(|h| h.name != name);
    }

    /// Dispatch an interrupt
    pub fn dispatch(&self, vector: InterruptVector) {
        let handlers = self.handlers.read();
        for handler in &handlers[vector as usize] {
            (handler.handler)(vector);
        }
    }
}

/// Global interrupt dispatcher
static DISPATCHER: InterruptDispatcher = InterruptDispatcher::new();

/// Get the interrupt dispatcher
pub fn dispatcher() -> &'static InterruptDispatcher {
    &DISPATCHER
}
