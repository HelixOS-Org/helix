//! # Thread Registry
//!
//! Central registry for all threads in the system.

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;

use spin::RwLock;

use super::{Thread, ThreadState};
use crate::{ExecError, ExecResult, ProcessId, ThreadId};

/// Thread registry
pub struct ThreadRegistry {
    /// All threads by ID
    threads: RwLock<BTreeMap<ThreadId, Arc<Thread>>>,
    /// Threads by process
    by_process: RwLock<BTreeMap<ProcessId, Vec<ThreadId>>>,
}

impl ThreadRegistry {
    /// Create a new registry
    pub const fn new() -> Self {
        Self {
            threads: RwLock::new(BTreeMap::new()),
            by_process: RwLock::new(BTreeMap::new()),
        }
    }

    /// Register a new thread
    pub fn register(&self, thread: Arc<Thread>) -> ExecResult<()> {
        let id = thread.id();
        let process = thread.process();

        let mut threads = self.threads.write();
        if threads.contains_key(&id) {
            return Err(ExecError::AlreadyExists);
        }

        threads.insert(id, thread);
        drop(threads);

        self.by_process.write().entry(process).or_default().push(id);

        Ok(())
    }

    /// Unregister a thread
    pub fn unregister(&self, id: ThreadId) -> ExecResult<Arc<Thread>> {
        let mut threads = self.threads.write();
        let thread = threads.remove(&id).ok_or(ExecError::ThreadNotFound)?;

        let process = thread.process();
        drop(threads);

        if let Some(v) = self.by_process.write().get_mut(&process) {
            v.retain(|&t| t != id);
        }

        Ok(thread)
    }

    /// Get a thread by ID
    pub fn get(&self, id: ThreadId) -> Option<Arc<Thread>> {
        self.threads.read().get(&id).cloned()
    }

    /// Get all threads for a process
    pub fn get_by_process(&self, process: ProcessId) -> Vec<Arc<Thread>> {
        let by_process = self.by_process.read();
        let threads = self.threads.read();

        by_process
            .get(&process)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| threads.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all threads in a state
    pub fn get_by_state(&self, state: ThreadState) -> Vec<Arc<Thread>> {
        self.threads
            .read()
            .values()
            .filter(|t| t.state() == state)
            .cloned()
            .collect()
    }

    /// Get thread count
    pub fn count(&self) -> usize {
        self.threads.read().len()
    }

    /// Get thread count for a process
    pub fn count_by_process(&self, process: ProcessId) -> usize {
        self.by_process
            .read()
            .get(&process)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Iterate over all threads
    pub fn for_each<F>(&self, f: F)
    where
        F: Fn(&Arc<Thread>),
    {
        for thread in self.threads.read().values() {
            f(thread);
        }
    }
}

/// Global thread registry
static REGISTRY: ThreadRegistry = ThreadRegistry::new();

/// Get the thread registry
pub fn registry() -> &'static ThreadRegistry {
    &REGISTRY
}
