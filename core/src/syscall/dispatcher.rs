//! # Syscall Dispatcher
//!
//! Routes syscalls to their handlers.

use alloc::sync::Arc;

use super::{SyscallArgs, SyscallContext, SyscallHandler, SyscallNumber, SyscallReturn};

/// Dispatch a syscall to its handler
pub fn dispatch_syscall(
    _number: SyscallNumber,
    handler: &dyn SyscallHandler,
    args: SyscallArgs,
    _context: &SyscallContext,
) -> SyscallReturn {
    // Validate arguments
    if let Err(e) = handler.validate(&args) {
        return SyscallReturn::Error(e);
    }

    // Call the handler
    handler.handle(&args)
}

/// Syscall dispatcher with pre/post hooks
pub struct SyscallDispatcher {
    /// Pre-syscall hooks
    pre_hooks: spin::RwLock<alloc::vec::Vec<Arc<dyn SyscallHook>>>,
    /// Post-syscall hooks
    post_hooks: spin::RwLock<alloc::vec::Vec<Arc<dyn SyscallHook>>>,
}

impl Default for SyscallDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl SyscallDispatcher {
    /// Create a new dispatcher
    pub const fn new() -> Self {
        Self {
            pre_hooks: spin::RwLock::new(alloc::vec::Vec::new()),
            post_hooks: spin::RwLock::new(alloc::vec::Vec::new()),
        }
    }

    /// Add a pre-syscall hook
    pub fn add_pre_hook(&self, hook: Arc<dyn SyscallHook>) {
        self.pre_hooks.write().push(hook);
    }

    /// Add a post-syscall hook
    pub fn add_post_hook(&self, hook: Arc<dyn SyscallHook>) {
        self.post_hooks.write().push(hook);
    }

    /// Dispatch with hooks
    pub fn dispatch(
        &self,
        number: SyscallNumber,
        handler: &dyn SyscallHandler,
        args: SyscallArgs,
        context: &SyscallContext,
    ) -> SyscallReturn {
        // Run pre-hooks
        for hook in self.pre_hooks.read().iter() {
            if let Some(result) = hook.pre_syscall(number, &args, context) {
                return result;
            }
        }

        // Dispatch
        let result = dispatch_syscall(number, handler, args, context);

        // Run post-hooks
        for hook in self.post_hooks.read().iter() {
            hook.post_syscall(number, &args, context, &result);
        }

        result
    }
}

/// Syscall hook trait
pub trait SyscallHook: Send + Sync {
    /// Called before syscall execution
    ///
    /// Return Some(result) to short-circuit the syscall
    fn pre_syscall(
        &self,
        _number: SyscallNumber,
        _args: &SyscallArgs,
        _context: &SyscallContext,
    ) -> Option<SyscallReturn> {
        None
    }

    /// Called after syscall execution
    fn post_syscall(
        &self,
        _number: SyscallNumber,
        _args: &SyscallArgs,
        _context: &SyscallContext,
        _result: &SyscallReturn,
    ) {
    }
}
