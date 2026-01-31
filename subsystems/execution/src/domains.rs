//! # Execution Domains
//!
//! Defines execution domains for isolation.

use crate::{ThreadId, ProcessId};
use alloc::collections::BTreeSet;
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::RwLock;

/// Domain identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DomainId(u64);

impl DomainId {
    /// Create a new domain ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    /// Kernel domain
    pub const fn kernel() -> Self {
        Self(0)
    }
}

impl Default for DomainId {
    fn default() -> Self {
        Self::new()
    }
}

/// Execution domain type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainType {
    /// Kernel domain (ring 0)
    Kernel,
    /// User domain (ring 3)
    User,
    /// Driver domain (may be ring 0 or separate)
    Driver,
    /// Sandbox domain (restricted user)
    Sandbox,
}

/// Execution domain
pub struct ExecutionDomain {
    /// Domain ID
    id: DomainId,
    /// Domain name
    name: String,
    /// Domain type
    domain_type: DomainType,
    /// Threads in this domain
    threads: RwLock<BTreeSet<ThreadId>>,
    /// Processes in this domain
    processes: RwLock<BTreeSet<ProcessId>>,
    /// Parent domain (if any)
    parent: Option<DomainId>,
}

impl ExecutionDomain {
    /// Create a new domain
    pub fn new(name: impl Into<String>, domain_type: DomainType, parent: Option<DomainId>) -> Self {
        Self {
            id: DomainId::new(),
            name: name.into(),
            domain_type,
            threads: RwLock::new(BTreeSet::new()),
            processes: RwLock::new(BTreeSet::new()),
            parent,
        }
    }

    /// Create the kernel domain
    pub fn kernel() -> Self {
        Self {
            id: DomainId::kernel(),
            name: "kernel".into(),
            domain_type: DomainType::Kernel,
            threads: RwLock::new(BTreeSet::new()),
            processes: RwLock::new(BTreeSet::new()),
            parent: None,
        }
    }

    /// Get domain ID
    pub fn id(&self) -> DomainId {
        self.id
    }

    /// Get domain name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get domain type
    pub fn domain_type(&self) -> DomainType {
        self.domain_type
    }

    /// Add a thread to this domain
    pub fn add_thread(&self, thread: ThreadId) {
        self.threads.write().insert(thread);
    }

    /// Remove a thread from this domain
    pub fn remove_thread(&self, thread: ThreadId) {
        self.threads.write().remove(&thread);
    }

    /// Add a process to this domain
    pub fn add_process(&self, process: ProcessId) {
        self.processes.write().insert(process);
    }

    /// Remove a process from this domain
    pub fn remove_process(&self, process: ProcessId) {
        self.processes.write().remove(&process);
    }

    /// Check if domain contains a thread
    pub fn contains_thread(&self, thread: ThreadId) -> bool {
        self.threads.read().contains(&thread)
    }

    /// Check if domain contains a process
    pub fn contains_process(&self, process: ProcessId) -> bool {
        self.processes.read().contains(&process)
    }

    /// Get thread count
    pub fn thread_count(&self) -> usize {
        self.threads.read().len()
    }

    /// Get process count
    pub fn process_count(&self) -> usize {
        self.processes.read().len()
    }
}
