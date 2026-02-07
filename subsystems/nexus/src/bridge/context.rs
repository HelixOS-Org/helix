//! # Syscall Execution Context Management
//!
//! Manages the full execution context for syscall processing:
//! - Process context (credentials, namespace, capabilities)
//! - Thread context (TLS, signal mask, scheduling)
//! - Resource context (limits, quotas, accounting)
//! - Security context (labels, policies, audit)
//! - Performance context (priorities, budgets)

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::syscall::SyscallType;

// ============================================================================
// PROCESS CONTEXT
// ============================================================================

/// Full process context for syscall execution
#[derive(Debug, Clone)]
pub struct ProcessContext {
    /// Process ID
    pub pid: u64,
    /// Parent process ID
    pub ppid: u64,
    /// Thread group ID
    pub tgid: u64,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Effective user ID
    pub euid: u32,
    /// Effective group ID
    pub egid: u32,
    /// Supplementary groups
    pub groups: Vec<u32>,
    /// Capabilities
    pub capabilities: CapabilitySet,
    /// Namespace context
    pub namespace: NamespaceContext,
    /// Resource limits
    pub rlimits: ResourceLimits,
    /// Security label
    pub security_label: SecurityLabel,
    /// Scheduling class
    pub sched_class: SchedClass,
    /// Nice value (-20 to 19)
    pub nice: i8,
    /// Process creation time
    pub start_time: u64,
    /// Number of threads
    pub num_threads: u32,
    /// Current working directory inode
    pub cwd_inode: u64,
    /// Root directory inode
    pub root_inode: u64,
}

impl ProcessContext {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            ppid: 0,
            tgid: pid,
            uid: 0,
            gid: 0,
            euid: 0,
            egid: 0,
            groups: Vec::new(),
            capabilities: CapabilitySet::empty(),
            namespace: NamespaceContext::default(),
            rlimits: ResourceLimits::default(),
            security_label: SecurityLabel::default(),
            sched_class: SchedClass::Normal,
            nice: 0,
            start_time: 0,
            num_threads: 1,
            cwd_inode: 0,
            root_inode: 0,
        }
    }

    /// Check if process has a capability
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.capabilities.has(cap)
    }

    /// Check if process is root
    pub fn is_root(&self) -> bool {
        self.euid == 0
    }

    /// Check if process can perform a specific syscall
    pub fn can_perform(&self, syscall_type: SyscallType) -> bool {
        match syscall_type {
            SyscallType::Mmap => self.has_capability(Capability::SysMmap),
            SyscallType::Fork | SyscallType::Clone => self.has_capability(Capability::SysFork),
            SyscallType::Exec => self.has_capability(Capability::SysExec),
            SyscallType::Kill => self.has_capability(Capability::SysKill),
            SyscallType::Ioctl => self.has_capability(Capability::SysIoctl),
            _ => true,
        }
    }
}

// ============================================================================
// CAPABILITIES
// ============================================================================

/// System capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// Network operations
    NetBind          = 0,
    NetAdmin         = 1,
    NetRaw           = 2,
    /// File operations
    FileRead         = 3,
    FileWrite        = 4,
    FileExec         = 5,
    FileOwner        = 6,
    /// System operations
    SysAdmin         = 7,
    SysBoot          = 8,
    SysMmap          = 9,
    SysFork          = 10,
    SysExec          = 11,
    SysKill          = 12,
    SysIoctl         = 13,
    SysPtrace        = 14,
    SysModule        = 15,
    /// IPC
    IpcOwner         = 16,
    IpcLock          = 17,
    /// Device
    DevRaw           = 18,
    DevMknod         = 19,
    /// Scheduling
    SchedSetparam    = 20,
    SchedSetaffinity = 21,
    /// Audit
    AuditControl     = 22,
    AuditWrite       = 23,
}

/// Set of capabilities
#[derive(Debug, Clone, Copy)]
pub struct CapabilitySet {
    /// Bitmask of capabilities
    bits: u64,
}

impl CapabilitySet {
    pub fn empty() -> Self {
        Self { bits: 0 }
    }

    pub fn full() -> Self {
        Self { bits: u64::MAX }
    }

    pub fn add(&mut self, cap: Capability) {
        self.bits |= 1u64 << (cap as u32);
    }

    pub fn remove(&mut self, cap: Capability) {
        self.bits &= !(1u64 << (cap as u32));
    }

    pub fn has(&self, cap: Capability) -> bool {
        (self.bits & (1u64 << (cap as u32))) != 0
    }

    pub fn intersect(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            bits: self.bits & other.bits,
        }
    }

    pub fn union(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            bits: self.bits | other.bits,
        }
    }

    pub fn count(&self) -> u32 {
        self.bits.count_ones()
    }
}

// ============================================================================
// NAMESPACES
// ============================================================================

/// Namespace context
#[derive(Debug, Clone)]
pub struct NamespaceContext {
    /// PID namespace ID
    pub pid_ns: u64,
    /// Mount namespace ID
    pub mnt_ns: u64,
    /// Network namespace ID
    pub net_ns: u64,
    /// IPC namespace ID
    pub ipc_ns: u64,
    /// UTS namespace ID
    pub uts_ns: u64,
    /// User namespace ID
    pub user_ns: u64,
    /// Cgroup namespace ID
    pub cgroup_ns: u64,
}

impl Default for NamespaceContext {
    fn default() -> Self {
        Self {
            pid_ns: 1,
            mnt_ns: 1,
            net_ns: 1,
            ipc_ns: 1,
            uts_ns: 1,
            user_ns: 1,
            cgroup_ns: 1,
        }
    }
}

impl NamespaceContext {
    /// Check if two contexts share the same namespace
    pub fn shares_namespace(&self, other: &NamespaceContext, ns_type: NamespaceType) -> bool {
        match ns_type {
            NamespaceType::Pid => self.pid_ns == other.pid_ns,
            NamespaceType::Mount => self.mnt_ns == other.mnt_ns,
            NamespaceType::Network => self.net_ns == other.net_ns,
            NamespaceType::Ipc => self.ipc_ns == other.ipc_ns,
            NamespaceType::Uts => self.uts_ns == other.uts_ns,
            NamespaceType::User => self.user_ns == other.user_ns,
            NamespaceType::Cgroup => self.cgroup_ns == other.cgroup_ns,
        }
    }
}

/// Namespace type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceType {
    Pid,
    Mount,
    Network,
    Ipc,
    Uts,
    User,
    Cgroup,
}

// ============================================================================
// RESOURCE LIMITS
// ============================================================================

/// Per-process resource limits
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Max CPU time (seconds)
    pub cpu_time: RLimit,
    /// Max file size (bytes)
    pub file_size: RLimit,
    /// Max data segment size (bytes)
    pub data_size: RLimit,
    /// Max stack size (bytes)
    pub stack_size: RLimit,
    /// Max core dump size (bytes)
    pub core_size: RLimit,
    /// Max resident set size (bytes)
    pub rss: RLimit,
    /// Max number of processes
    pub nproc: RLimit,
    /// Max open files
    pub nofile: RLimit,
    /// Max locked memory (bytes)
    pub memlock: RLimit,
    /// Max address space (bytes)
    pub address_space: RLimit,
    /// Max pending signals
    pub sigpending: RLimit,
    /// Max message queue size (bytes)
    pub msgqueue: RLimit,
}

/// Resource limit (soft + hard)
#[derive(Debug, Clone, Copy)]
pub struct RLimit {
    /// Soft limit (can be raised up to hard)
    pub soft: u64,
    /// Hard limit (maximum)
    pub hard: u64,
}

impl RLimit {
    pub const UNLIMITED: u64 = u64::MAX;

    pub fn new(soft: u64, hard: u64) -> Self {
        Self { soft, hard }
    }

    pub fn unlimited() -> Self {
        Self {
            soft: Self::UNLIMITED,
            hard: Self::UNLIMITED,
        }
    }

    pub fn check(&self, value: u64) -> LimitCheck {
        if value > self.hard {
            LimitCheck::HardLimitExceeded
        } else if value > self.soft {
            LimitCheck::SoftLimitExceeded
        } else {
            LimitCheck::Ok
        }
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_time: RLimit::unlimited(),
            file_size: RLimit::unlimited(),
            data_size: RLimit::unlimited(),
            stack_size: RLimit::new(8 * 1024 * 1024, RLimit::UNLIMITED),
            core_size: RLimit::new(0, RLimit::UNLIMITED),
            rss: RLimit::unlimited(),
            nproc: RLimit::new(4096, 65536),
            nofile: RLimit::new(1024, 1048576),
            memlock: RLimit::new(65536, 65536),
            address_space: RLimit::unlimited(),
            sigpending: RLimit::new(128, 1024),
            msgqueue: RLimit::new(819200, 819200),
        }
    }
}

/// Result of a limit check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitCheck {
    Ok,
    SoftLimitExceeded,
    HardLimitExceeded,
}

// ============================================================================
// SECURITY LABELS
// ============================================================================

/// Security label for mandatory access control
#[derive(Debug, Clone)]
pub struct SecurityLabel {
    /// Security level (0 = unclassified, higher = more restricted)
    pub level: u32,
    /// Category bitmap (for compartmentalization)
    pub categories: u64,
    /// Domain identifier
    pub domain: u32,
    /// Type identifier
    pub type_id: u32,
    /// Role identifier
    pub role: u32,
    /// Label string (e.g., "system_u:system_r:unconfined_t:s0")
    pub label_string: String,
}

impl Default for SecurityLabel {
    fn default() -> Self {
        Self {
            level: 0,
            categories: 0,
            domain: 0,
            type_id: 0,
            role: 0,
            label_string: String::new(),
        }
    }
}

impl SecurityLabel {
    /// Check if this label dominates another (for MAC)
    pub fn dominates(&self, other: &SecurityLabel) -> bool {
        self.level >= other.level && (self.categories & other.categories) == other.categories
    }

    /// Check if labels are in the same domain
    pub fn same_domain(&self, other: &SecurityLabel) -> bool {
        self.domain == other.domain
    }
}

// ============================================================================
// SCHEDULING CONTEXT
// ============================================================================

/// Scheduling class
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedClass {
    /// Normal (CFS-like)
    Normal,
    /// Batch processing
    Batch,
    /// Idle (only when nothing else to run)
    Idle,
    /// Real-time FIFO
    RealtimeFifo,
    /// Real-time round-robin
    RealtimeRoundRobin,
    /// Deadline-based
    Deadline,
}

// ============================================================================
// CONTEXT MANAGER
// ============================================================================

/// Manages execution contexts for all processes
pub struct ContextManager {
    /// Process contexts
    contexts: BTreeMap<u64, ProcessContext>,
    /// Thread contexts (pid → thread_id → context)
    thread_contexts: BTreeMap<u64, BTreeMap<u64, ThreadContext>>,
    /// Max contexts to track
    max_contexts: usize,
}

/// Thread-specific context
#[derive(Debug, Clone)]
pub struct ThreadContext {
    /// Thread ID
    pub tid: u64,
    /// Thread-local storage base
    pub tls_base: u64,
    /// Signal mask
    pub signal_mask: u64,
    /// CPU affinity mask
    pub cpu_affinity: u64,
    /// Stack base address
    pub stack_base: u64,
    /// Stack size
    pub stack_size: u64,
    /// Whether thread is in syscall
    pub in_syscall: bool,
    /// Current syscall type (if in syscall)
    pub current_syscall: Option<SyscallType>,
    /// Syscall entry timestamp
    pub syscall_entry_time: u64,
}

impl ThreadContext {
    pub fn new(tid: u64) -> Self {
        Self {
            tid,
            tls_base: 0,
            signal_mask: 0,
            cpu_affinity: u64::MAX, // All CPUs
            stack_base: 0,
            stack_size: 0,
            in_syscall: false,
            current_syscall: None,
            syscall_entry_time: 0,
        }
    }

    /// Mark syscall entry
    pub fn enter_syscall(&mut self, syscall_type: SyscallType, timestamp: u64) {
        self.in_syscall = true;
        self.current_syscall = Some(syscall_type);
        self.syscall_entry_time = timestamp;
    }

    /// Mark syscall exit
    pub fn exit_syscall(&mut self) -> u64 {
        self.in_syscall = false;
        let entry = self.syscall_entry_time;
        self.current_syscall = None;
        self.syscall_entry_time = 0;
        entry
    }
}

impl ContextManager {
    pub fn new(max_contexts: usize) -> Self {
        Self {
            contexts: BTreeMap::new(),
            thread_contexts: BTreeMap::new(),
            max_contexts,
        }
    }

    /// Register a new process
    pub fn register_process(&mut self, ctx: ProcessContext) -> bool {
        if self.contexts.len() >= self.max_contexts {
            return false;
        }
        let pid = ctx.pid;
        self.contexts.insert(pid, ctx);
        true
    }

    /// Get process context
    pub fn get_process(&self, pid: u64) -> Option<&ProcessContext> {
        self.contexts.get(&pid)
    }

    /// Get mutable process context
    pub fn get_process_mut(&mut self, pid: u64) -> Option<&mut ProcessContext> {
        self.contexts.get_mut(&pid)
    }

    /// Register a thread
    pub fn register_thread(&mut self, pid: u64, thread: ThreadContext) {
        self.thread_contexts
            .entry(pid)
            .or_insert_with(BTreeMap::new)
            .insert(thread.tid, thread);

        // Update thread count
        if let Some(ctx) = self.contexts.get_mut(&pid) {
            if let Some(threads) = self.thread_contexts.get(&pid) {
                ctx.num_threads = threads.len() as u32;
            }
        }
    }

    /// Get thread context
    pub fn get_thread(&self, pid: u64, tid: u64) -> Option<&ThreadContext> {
        self.thread_contexts.get(&pid).and_then(|m| m.get(&tid))
    }

    /// Get mutable thread context
    pub fn get_thread_mut(&mut self, pid: u64, tid: u64) -> Option<&mut ThreadContext> {
        self.thread_contexts
            .get_mut(&pid)
            .and_then(|m| m.get_mut(&tid))
    }

    /// Remove process and all threads
    pub fn remove_process(&mut self, pid: u64) {
        self.contexts.remove(&pid);
        self.thread_contexts.remove(&pid);
    }

    /// Remove a thread
    pub fn remove_thread(&mut self, pid: u64, tid: u64) {
        if let Some(threads) = self.thread_contexts.get_mut(&pid) {
            threads.remove(&tid);
            if let Some(ctx) = self.contexts.get_mut(&pid) {
                ctx.num_threads = threads.len() as u32;
            }
        }
    }

    /// Number of tracked processes
    pub fn process_count(&self) -> usize {
        self.contexts.len()
    }

    /// Total thread count
    pub fn thread_count(&self) -> usize {
        self.thread_contexts.values().map(|m| m.len()).sum()
    }

    /// Find processes in a specific namespace
    pub fn processes_in_namespace(&self, ns_type: NamespaceType, ns_id: u64) -> Vec<u64> {
        self.contexts
            .iter()
            .filter(|(_, ctx)| match ns_type {
                NamespaceType::Pid => ctx.namespace.pid_ns == ns_id,
                NamespaceType::Mount => ctx.namespace.mnt_ns == ns_id,
                NamespaceType::Network => ctx.namespace.net_ns == ns_id,
                NamespaceType::Ipc => ctx.namespace.ipc_ns == ns_id,
                NamespaceType::Uts => ctx.namespace.uts_ns == ns_id,
                NamespaceType::User => ctx.namespace.user_ns == ns_id,
                NamespaceType::Cgroup => ctx.namespace.cgroup_ns == ns_id,
            })
            .map(|(&pid, _)| pid)
            .collect()
    }
}
