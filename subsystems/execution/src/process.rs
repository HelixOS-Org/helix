//! # Process Management
//!
//! Process abstraction and management.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};

use spin::RwLock;

use crate::{ExecError, ExecResult, ProcessId, ThreadId};

/// Process state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    /// Process is being created
    Creating,
    /// Process is running
    Running,
    /// Process is stopped
    Stopped,
    /// Process is a zombie
    Zombie,
    /// Process is dead
    Dead,
}

/// Process structure
pub struct Process {
    /// Process ID
    id: ProcessId,
    /// Parent process
    parent: Option<ProcessId>,
    /// Process name
    name: String,
    /// Current state
    state: AtomicU32,
    /// Main thread
    main_thread: RwLock<Option<ThreadId>>,
    /// All threads
    threads: RwLock<Vec<ThreadId>>,
    /// Child processes
    children: RwLock<Vec<ProcessId>>,
    /// Exit code
    exit_code: RwLock<Option<i32>>,
    /// User ID
    uid: u32,
    /// Group ID
    gid: u32,
}

impl Process {
    /// Create a new process
    pub fn new(id: ProcessId, parent: Option<ProcessId>, name: impl Into<String>) -> Self {
        Self {
            id,
            parent,
            name: name.into(),
            state: AtomicU32::new(ProcessState::Creating as u32),
            main_thread: RwLock::new(None),
            threads: RwLock::new(Vec::new()),
            children: RwLock::new(Vec::new()),
            exit_code: RwLock::new(None),
            uid: 0,
            gid: 0,
        }
    }

    /// Get process ID
    pub fn id(&self) -> ProcessId {
        self.id
    }

    /// Get parent process
    pub fn parent(&self) -> Option<ProcessId> {
        self.parent
    }

    /// Get process name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get state
    pub fn state(&self) -> ProcessState {
        match self.state.load(Ordering::SeqCst) {
            0 => ProcessState::Creating,
            1 => ProcessState::Running,
            2 => ProcessState::Stopped,
            3 => ProcessState::Zombie,
            _ => ProcessState::Dead,
        }
    }

    /// Set state
    pub fn set_state(&self, state: ProcessState) {
        let val = match state {
            ProcessState::Creating => 0,
            ProcessState::Running => 1,
            ProcessState::Stopped => 2,
            ProcessState::Zombie => 3,
            ProcessState::Dead => 4,
        };
        self.state.store(val, Ordering::SeqCst);
    }

    /// Set main thread
    pub fn set_main_thread(&self, thread: ThreadId) {
        *self.main_thread.write() = Some(thread);
        self.threads.write().push(thread);
    }

    /// Get main thread
    pub fn main_thread(&self) -> Option<ThreadId> {
        *self.main_thread.read()
    }

    /// Add a thread
    pub fn add_thread(&self, thread: ThreadId) {
        self.threads.write().push(thread);
    }

    /// Remove a thread
    pub fn remove_thread(&self, thread: ThreadId) {
        self.threads.write().retain(|&t| t != thread);
    }

    /// Get thread count
    pub fn thread_count(&self) -> usize {
        self.threads.read().len()
    }

    /// Add a child process
    pub fn add_child(&self, child: ProcessId) {
        self.children.write().push(child);
    }

    /// Remove a child process
    pub fn remove_child(&self, child: ProcessId) {
        self.children.write().retain(|&c| c != child);
    }

    /// Exit the process
    pub fn exit(&self, code: i32) {
        *self.exit_code.write() = Some(code);
        self.set_state(ProcessState::Zombie);
    }
}

/// Process registry
pub struct ProcessRegistry {
    /// All processes
    processes: RwLock<BTreeMap<ProcessId, Arc<Process>>>,
}

impl ProcessRegistry {
    /// Create a new registry
    pub const fn new() -> Self {
        Self {
            processes: RwLock::new(BTreeMap::new()),
        }
    }

    /// Register a process
    pub fn register(&self, process: Arc<Process>) -> ExecResult<()> {
        let id = process.id();
        let mut processes = self.processes.write();

        if processes.contains_key(&id) {
            return Err(ExecError::AlreadyExists);
        }

        processes.insert(id, process);
        Ok(())
    }

    /// Unregister a process
    pub fn unregister(&self, id: ProcessId) -> ExecResult<Arc<Process>> {
        self.processes
            .write()
            .remove(&id)
            .ok_or(ExecError::ProcessNotFound)
    }

    /// Get a process
    pub fn get(&self, id: ProcessId) -> Option<Arc<Process>> {
        self.processes.read().get(&id).cloned()
    }

    /// Get process count
    pub fn count(&self) -> usize {
        self.processes.read().len()
    }
}

/// Global process registry
static REGISTRY: ProcessRegistry = ProcessRegistry::new();

/// Get the process registry
pub fn registry() -> &'static ProcessRegistry {
    &REGISTRY
}
