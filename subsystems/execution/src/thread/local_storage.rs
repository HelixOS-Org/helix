//! # Thread-Local Storage
//!
//! TLS management for threads.

use crate::{ThreadId, ExecResult, ExecError};
use helix_hal::VirtAddr;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

/// TLS key
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TlsKey(u64);

impl TlsKey {
    /// Create a new TLS key
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

impl Default for TlsKey {
    fn default() -> Self {
        Self::new()
    }
}

/// TLS destructor
pub type TlsDestructor = fn(u64);

/// TLS entry
struct TlsEntry {
    /// Value
    value: u64,
    /// Destructor (if any)
    destructor: Option<TlsDestructor>,
}

/// Per-thread TLS storage
pub struct ThreadLocalStorage {
    /// Thread ID
    thread: ThreadId,
    /// TLS entries
    entries: RwLock<BTreeMap<TlsKey, TlsEntry>>,
}

impl ThreadLocalStorage {
    /// Create new TLS for a thread
    pub fn new(thread: ThreadId) -> Self {
        Self {
            thread,
            entries: RwLock::new(BTreeMap::new()),
        }
    }

    /// Set a TLS value
    pub fn set(&self, key: TlsKey, value: u64, destructor: Option<TlsDestructor>) {
        self.entries.write().insert(key, TlsEntry { value, destructor });
    }

    /// Get a TLS value
    pub fn get(&self, key: TlsKey) -> Option<u64> {
        self.entries.read().get(&key).map(|e| e.value)
    }

    /// Remove a TLS value
    pub fn remove(&self, key: TlsKey) -> Option<u64> {
        self.entries.write().remove(&key).map(|e| e.value)
    }

    /// Run destructors (called on thread exit)
    pub fn run_destructors(&self) {
        let mut guard = self.entries.write();
        let entries: Vec<_> = core::mem::take(&mut *guard).into_iter().collect();
        drop(guard);
        for (_, entry) in entries {
            if let Some(dtor) = entry.destructor {
                dtor(entry.value);
            }
        }
    }
}

/// TLS key registry
pub struct TlsKeyRegistry {
    /// Keys with destructors
    keys: RwLock<BTreeMap<TlsKey, Option<TlsDestructor>>>,
}

impl TlsKeyRegistry {
    /// Create a new registry
    pub const fn new() -> Self {
        Self {
            keys: RwLock::new(BTreeMap::new()),
        }
    }

    /// Create a new TLS key
    pub fn create_key(&self, destructor: Option<TlsDestructor>) -> TlsKey {
        let key = TlsKey::new();
        self.keys.write().insert(key, destructor);
        key
    }

    /// Delete a TLS key
    pub fn delete_key(&self, key: TlsKey) {
        self.keys.write().remove(&key);
    }

    /// Get destructor for a key
    pub fn get_destructor(&self, key: TlsKey) -> Option<TlsDestructor> {
        self.keys.read().get(&key).copied().flatten()
    }
}

/// Global TLS key registry
static KEY_REGISTRY: TlsKeyRegistry = TlsKeyRegistry::new();

/// Get the TLS key registry
pub fn key_registry() -> &'static TlsKeyRegistry {
    &KEY_REGISTRY
}
