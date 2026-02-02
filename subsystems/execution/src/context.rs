//! # Context Switching
//!
//! Context switch infrastructure.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::thread::ThreadContext;
use crate::ThreadId;

/// Context switch reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwitchReason {
    /// Voluntary yield
    Yield,
    /// Timer preemption
    Preemption,
    /// Thread blocked
    Blocked,
    /// Thread exited
    Exit,
    /// Interrupt handling
    Interrupt,
    /// New thread started
    NewThread,
}

/// Context switch statistics
pub struct SwitchStats {
    /// Total switches
    pub total: u64,
    /// Voluntary switches
    pub voluntary: u64,
    /// Involuntary switches
    pub involuntary: u64,
    /// Average switch time (nanoseconds)
    pub avg_time_ns: u64,
}

/// Context switch hook
pub trait ContextSwitchHook: Send + Sync {
    /// Called before context switch
    fn pre_switch(&self, from: ThreadId, to: ThreadId, reason: SwitchReason);

    /// Called after context switch
    fn post_switch(&self, from: ThreadId, to: ThreadId, reason: SwitchReason);
}

/// Context switch engine
pub struct ContextSwitchEngine {
    /// Is FPU lazy switching enabled?
    fpu_lazy: AtomicBool,
    /// Hooks
    hooks: spin::RwLock<alloc::vec::Vec<alloc::sync::Arc<dyn ContextSwitchHook>>>,
}

impl ContextSwitchEngine {
    /// Create a new context switch engine
    pub const fn new() -> Self {
        Self {
            fpu_lazy: AtomicBool::new(true),
            hooks: spin::RwLock::new(alloc::vec::Vec::new()),
        }
    }

    /// Enable/disable FPU lazy switching
    pub fn set_fpu_lazy(&self, enabled: bool) {
        self.fpu_lazy.store(enabled, Ordering::SeqCst);
    }

    /// Check if FPU lazy switching is enabled
    pub fn fpu_lazy(&self) -> bool {
        self.fpu_lazy.load(Ordering::SeqCst)
    }

    /// Add a hook
    pub fn add_hook(&self, hook: alloc::sync::Arc<dyn ContextSwitchHook>) {
        self.hooks.write().push(hook);
    }

    /// Perform a context switch
    ///
    /// # Safety
    /// This function performs low-level CPU state manipulation.
    pub unsafe fn switch(
        &self,
        from: ThreadId,
        to: ThreadId,
        from_ctx: &mut ThreadContext,
        to_ctx: &ThreadContext,
        reason: SwitchReason,
    ) {
        // Pre-switch hooks
        for hook in self.hooks.read().iter() {
            hook.pre_switch(from, to, reason);
        }

        // Actual context switch (architecture-specific)
        // This would call into HAL or assembly code
        // SAFETY: The caller is responsible for ensuring the contexts are valid
        unsafe {
            self.do_switch(from_ctx, to_ctx);
        }

        // Post-switch hooks
        for hook in self.hooks.read().iter() {
            hook.post_switch(from, to, reason);
        }
    }

    /// Low-level switch (placeholder)
    unsafe fn do_switch(&self, _from: &mut ThreadContext, _to: &ThreadContext) {
        // This would be implemented in assembly for each architecture
        // For now, just a placeholder
    }
}

/// Global context switch engine
static ENGINE: ContextSwitchEngine = ContextSwitchEngine::new();

/// Get the context switch engine
pub fn engine() -> &'static ContextSwitchEngine {
    &ENGINE
}
