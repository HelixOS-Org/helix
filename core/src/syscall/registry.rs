//! # Syscall Registry
//!
//! Manages registration and lookup of syscall handlers.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;

use spin::RwLock;

use super::{
    SyscallArgs, SyscallContext, SyscallError, SyscallHandler, SyscallNumber, SyscallReturn,
};

/// Maximum syscall number
pub const MAX_SYSCALL: usize = 512;

/// Syscall table entry
struct SyscallEntry {
    /// Handler
    handler: Arc<dyn SyscallHandler>,
    /// Call count
    call_count: core::sync::atomic::AtomicU64,
}

/// Syscall registry
pub struct SyscallRegistry {
    /// Syscall table (indexed by number)
    table: RwLock<[Option<SyscallEntry>; MAX_SYSCALL]>,
    /// Named syscalls (for dynamic lookup)
    named: RwLock<BTreeMap<String, SyscallNumber>>,
}

impl SyscallRegistry {
    /// Create a new registry
    pub const fn new() -> Self {
        // Can't use array initialization with Option<SyscallEntry> in const context
        // This will be properly initialized at runtime
        Self {
            table: RwLock::new([const { None }; MAX_SYSCALL]),
            named: RwLock::new(BTreeMap::new()),
        }
    }

    /// Register a syscall handler
    pub fn register(
        &self,
        number: SyscallNumber,
        handler: Arc<dyn SyscallHandler>,
    ) -> Result<(), SyscallError> {
        let idx = number as usize;
        if idx >= MAX_SYSCALL {
            return Err(SyscallError::InvalidArgument);
        }

        let mut table = self.table.write();
        if table[idx].is_some() {
            return Err(SyscallError::Exists);
        }

        let name = handler.name().to_string();

        table[idx] = Some(SyscallEntry {
            handler,
            call_count: core::sync::atomic::AtomicU64::new(0),
        });

        drop(table);

        self.named.write().insert(name, number);

        Ok(())
    }

    /// Unregister a syscall handler
    pub fn unregister(&self, number: SyscallNumber) -> Result<(), SyscallError> {
        let idx = number as usize;
        if idx >= MAX_SYSCALL {
            return Err(SyscallError::InvalidArgument);
        }

        let mut table = self.table.write();
        if let Some(entry) = table[idx].take() {
            let name = entry.handler.name();
            drop(table);
            self.named.write().remove(name);
            Ok(())
        } else {
            Err(SyscallError::NoEntry)
        }
    }

    /// Get a syscall handler
    pub fn get(&self, number: SyscallNumber) -> Option<Arc<dyn SyscallHandler>> {
        let idx = number as usize;
        if idx >= MAX_SYSCALL {
            return None;
        }

        self.table.read()[idx].as_ref().map(|e| e.handler.clone())
    }

    /// Look up a syscall by name
    pub fn lookup_by_name(&self, name: &str) -> Option<SyscallNumber> {
        self.named.read().get(name).copied()
    }

    /// Dispatch a syscall
    pub fn dispatch(
        &self,
        number: SyscallNumber,
        args: SyscallArgs,
        context: &SyscallContext,
    ) -> SyscallReturn {
        let idx = number as usize;
        if idx >= MAX_SYSCALL {
            return SyscallReturn::Error(SyscallError::NotImplemented);
        }

        let table = self.table.read();
        if let Some(entry) = &table[idx] {
            entry
                .call_count
                .fetch_add(1, core::sync::atomic::Ordering::Relaxed);
            let handler = entry.handler.clone();
            drop(table);
            handler.handle(&args)
        } else {
            SyscallReturn::Error(SyscallError::NotImplemented)
        }
    }
}

/// Global syscall registry
static REGISTRY: SyscallRegistry = SyscallRegistry::new();

/// Get the global syscall registry
pub fn registry() -> &'static SyscallRegistry {
    &REGISTRY
}

/// Register a syscall (convenience function)
pub fn register(
    number: SyscallNumber,
    handler: Arc<dyn SyscallHandler>,
) -> Result<(), SyscallError> {
    REGISTRY.register(number, handler)
}

/// Dispatch a syscall (convenience function)
pub fn dispatch(
    number: SyscallNumber,
    args: SyscallArgs,
    context: &SyscallContext,
) -> SyscallReturn {
    REGISTRY.dispatch(number, args, context)
}

/// Macro to define a syscall handler
#[macro_export]
macro_rules! define_syscall {
    ($name:ident, $number:expr, $arg_count:expr, | $args:ident | $body:expr) => {
        pub struct $name;

        impl $crate::syscall::SyscallHandler for $name {
            fn handle(
                &self,
                $args: &$crate::syscall::SyscallArgs,
            ) -> $crate::syscall::SyscallReturn {
                $body
            }

            fn name(&self) -> &'static str {
                stringify!($name)
            }

            fn arg_count(&self) -> usize {
                $arg_count
            }
        }
    };
}
