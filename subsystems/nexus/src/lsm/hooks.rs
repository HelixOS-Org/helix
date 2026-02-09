//! LSM Hooks
//!
//! LSM hook definitions and tracking.

use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};

use super::{HookId, LsmType};

/// Hook category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HookCategory {
    /// Task/process hooks
    Task,
    /// File hooks
    File,
    /// Inode hooks
    Inode,
    /// Socket hooks
    Socket,
    /// Network hooks
    Network,
    /// IPC hooks
    Ipc,
    /// Message hooks
    Msg,
    /// Shared memory hooks
    Shm,
    /// Semaphore hooks
    Sem,
    /// Capability hooks
    Capability,
    /// Credential hooks
    Cred,
    /// Kernel module hooks
    Kernel,
    /// Security blob hooks
    Blob,
    /// Audit hooks
    Audit,
    /// BPF hooks
    Bpf,
    /// Perf hooks
    Perf,
    /// Lockdown hooks
    Lockdown,
    /// Unknown
    Unknown,
}

impl HookCategory {
    /// Get category name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::File => "file",
            Self::Inode => "inode",
            Self::Socket => "socket",
            Self::Network => "network",
            Self::Ipc => "ipc",
            Self::Msg => "msg",
            Self::Shm => "shm",
            Self::Sem => "sem",
            Self::Capability => "capability",
            Self::Cred => "cred",
            Self::Kernel => "kernel",
            Self::Blob => "blob",
            Self::Audit => "audit",
            Self::Bpf => "bpf",
            Self::Perf => "perf",
            Self::Lockdown => "lockdown",
            Self::Unknown => "unknown",
        }
    }
}

/// LSM hook
#[derive(Debug)]
pub struct LsmHook {
    /// Hook ID
    pub id: HookId,
    /// Hook name
    pub name: String,
    /// Category
    pub category: HookCategory,
    /// LSM type
    pub lsm: LsmType,
    /// Is registered
    pub registered: bool,
    /// Call count
    pub call_count: AtomicU64,
    /// Deny count
    pub deny_count: AtomicU64,
    /// Average latency (ns)
    pub avg_latency_ns: AtomicU64,
}

impl LsmHook {
    /// Create new hook
    pub fn new(id: HookId, name: String, category: HookCategory, lsm: LsmType) -> Self {
        Self {
            id,
            name,
            category,
            lsm,
            registered: true,
            call_count: AtomicU64::new(0),
            deny_count: AtomicU64::new(0),
            avg_latency_ns: AtomicU64::new(0),
        }
    }

    /// Record call
    pub fn record_call(&self, denied: bool, latency_ns: u64) {
        let count = self.call_count.fetch_add(1, Ordering::Relaxed) + 1;

        if denied {
            self.deny_count.fetch_add(1, Ordering::Relaxed);
        }

        // Update average latency
        let old_avg = self.avg_latency_ns.load(Ordering::Relaxed);
        let new_avg = ((old_avg * (count - 1)) + latency_ns) / count;
        self.avg_latency_ns.store(new_avg, Ordering::Relaxed);
    }

    /// Get call count
    #[inline(always)]
    pub fn call_count(&self) -> u64 {
        self.call_count.load(Ordering::Relaxed)
    }

    /// Get deny count
    #[inline(always)]
    pub fn deny_count(&self) -> u64 {
        self.deny_count.load(Ordering::Relaxed)
    }

    /// Get deny rate
    #[inline]
    pub fn deny_rate(&self) -> f32 {
        let calls = self.call_count();
        if calls == 0 {
            return 0.0;
        }
        (self.deny_count() as f32 / calls as f32) * 100.0
    }

    /// Get average latency
    #[inline(always)]
    pub fn avg_latency(&self) -> u64 {
        self.avg_latency_ns.load(Ordering::Relaxed)
    }
}

/// Common LSM hooks
pub struct LsmHooks;

impl LsmHooks {
    // Task hooks
    pub const TASK_ALLOC: &'static str = "task_alloc";
    pub const TASK_FREE: &'static str = "task_free";
    pub const TASK_FIX_SETUID: &'static str = "task_fix_setuid";
    pub const TASK_KILL: &'static str = "task_kill";
    pub const TASK_PRCTL: &'static str = "task_prctl";

    // File hooks
    pub const FILE_PERMISSION: &'static str = "file_permission";
    pub const FILE_OPEN: &'static str = "file_open";
    pub const FILE_RECEIVE: &'static str = "file_receive";
    pub const FILE_MMAP: &'static str = "file_mmap";
    pub const FILE_MPROTECT: &'static str = "file_mprotect";
    pub const FILE_LOCK: &'static str = "file_lock";
    pub const FILE_IOCTL: &'static str = "file_ioctl";

    // Inode hooks
    pub const INODE_CREATE: &'static str = "inode_create";
    pub const INODE_LINK: &'static str = "inode_link";
    pub const INODE_UNLINK: &'static str = "inode_unlink";
    pub const INODE_SYMLINK: &'static str = "inode_symlink";
    pub const INODE_MKDIR: &'static str = "inode_mkdir";
    pub const INODE_RMDIR: &'static str = "inode_rmdir";
    pub const INODE_RENAME: &'static str = "inode_rename";
    pub const INODE_PERMISSION: &'static str = "inode_permission";
    pub const INODE_GETATTR: &'static str = "inode_getattr";
    pub const INODE_SETATTR: &'static str = "inode_setattr";

    // Socket hooks
    pub const SOCKET_CREATE: &'static str = "socket_create";
    pub const SOCKET_BIND: &'static str = "socket_bind";
    pub const SOCKET_CONNECT: &'static str = "socket_connect";
    pub const SOCKET_LISTEN: &'static str = "socket_listen";
    pub const SOCKET_ACCEPT: &'static str = "socket_accept";
    pub const SOCKET_SENDMSG: &'static str = "socket_sendmsg";
    pub const SOCKET_RECVMSG: &'static str = "socket_recvmsg";

    // Capability hooks
    pub const CAPABLE: &'static str = "capable";
    pub const CAPGET: &'static str = "capget";
    pub const CAPSET: &'static str = "capset";

    // BPF hooks
    pub const BPF: &'static str = "bpf";
    pub const BPF_MAP: &'static str = "bpf_map";
    pub const BPF_PROG: &'static str = "bpf_prog";
}
