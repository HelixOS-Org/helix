//! # Syscall Gateway
//!
//! The entry point for all system calls.

use core::sync::atomic::{AtomicU64, Ordering};

use super::{SyscallArgs, SyscallContext, SyscallReturn};

/// Statistics for syscall handling
#[derive(Debug, Default)]
pub struct SyscallStats {
    /// Total syscalls handled
    pub total_calls: AtomicU64,
    /// Successful syscalls
    pub successful: AtomicU64,
    /// Failed syscalls
    pub failed: AtomicU64,
}

impl SyscallStats {
    /// Create new stats
    pub const fn new() -> Self {
        Self {
            total_calls: AtomicU64::new(0),
            successful: AtomicU64::new(0),
            failed: AtomicU64::new(0),
        }
    }

    /// Record a syscall
    pub fn record(&self, success: bool) {
        self.total_calls.fetch_add(1, Ordering::Relaxed);
        if success {
            self.successful.fetch_add(1, Ordering::Relaxed);
        } else {
            self.failed.fetch_add(1, Ordering::Relaxed);
        }
    }
}

static STATS: SyscallStats = SyscallStats::new();

/// Syscall gateway - main entry point
///
/// This is called from architecture-specific assembly code.
///
/// # Arguments
/// * `number` - The syscall number
/// * `args` - Syscall arguments
/// * `context` - Calling context
///
/// # Returns
/// The syscall return value
pub fn syscall_entry(number: u64, args: SyscallArgs, context: &SyscallContext) -> SyscallReturn {
    // Log syscall (debug only)
    #[cfg(debug_assertions)]
    log::trace!(
        "syscall: num={} args={:?} pid={} tid={}",
        number,
        args,
        context.pid,
        context.tid
    );

    // Dispatch to the registry
    let result = super::registry::dispatch(number, args, context);

    // Record statistics
    STATS.record(matches!(result, SyscallReturn::Success(_)));

    result
}

/// Get syscall statistics
pub fn stats() -> &'static SyscallStats {
    &STATS
}

/// Raw syscall entry point (called from assembly)
///
/// # Safety
/// This function is called from assembly and must not be called directly.
#[no_mangle]
pub unsafe extern "C" fn helix_syscall_entry(
    number: u64,
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
) -> i64 {
    let args = SyscallArgs {
        args: [arg0, arg1, arg2, arg3, arg4, arg5],
        count: 6,
    };

    // TODO: Get actual context from current thread
    let context = SyscallContext {
        pid: 0,
        tid: 0,
        uid: 0,
        gid: 0,
        from_user: true,
    };

    syscall_entry(number, args, &context).to_raw()
}
