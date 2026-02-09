//! Audit Event Types
//!
//! Audit event and process context structures.

use alloc::string::String;
use alloc::vec::Vec;

use super::{AuditEventId, AuditMessageType, AuditResult, Gid, Pid, SyscallInfo, Uid};

/// Process context
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessContext {
    /// Process ID
    pub pid: Pid,
    /// Parent process ID
    pub ppid: Pid,
    /// Real UID
    pub uid: Uid,
    /// Effective UID
    pub euid: Uid,
    /// Saved set-UID
    pub suid: Uid,
    /// Filesystem UID
    pub fsuid: Uid,
    /// Real GID
    pub gid: Gid,
    /// Effective GID
    pub egid: Gid,
    /// Saved set-GID
    pub sgid: Gid,
    /// Filesystem GID
    pub fsgid: Gid,
    /// Session ID
    pub ses: u32,
    /// Audit user ID
    pub auid: Uid,
    /// TTY
    pub tty: Option<String>,
    /// Executable
    pub exe: Option<String>,
    /// Command line
    pub comm: Option<String>,
    /// Security context (SELinux/AppArmor)
    pub subj: Option<String>,
}

impl ProcessContext {
    /// Create new process context
    pub fn new(pid: Pid, uid: Uid) -> Self {
        Self {
            pid,
            ppid: Pid::new(0),
            uid,
            euid: uid,
            suid: uid,
            fsuid: uid,
            gid: Gid::ROOT,
            egid: Gid::ROOT,
            sgid: Gid::ROOT,
            fsgid: Gid::ROOT,
            ses: 0,
            auid: Uid::INVALID,
            tty: None,
            exe: None,
            comm: None,
            subj: None,
        }
    }

    /// Is root user
    #[inline(always)]
    pub fn is_root(&self) -> bool {
        self.euid == Uid::ROOT
    }

    /// Has privilege escalation
    #[inline(always)]
    pub fn has_escalation(&self) -> bool {
        self.euid != self.uid && self.euid == Uid::ROOT
    }

    /// Set parent PID
    #[inline(always)]
    pub fn with_ppid(mut self, ppid: Pid) -> Self {
        self.ppid = ppid;
        self
    }

    /// Set executable
    #[inline(always)]
    pub fn with_exe(mut self, exe: String) -> Self {
        self.exe = Some(exe);
        self
    }

    /// Set command
    #[inline(always)]
    pub fn with_comm(mut self, comm: String) -> Self {
        self.comm = Some(comm);
        self
    }
}

/// Path record
#[derive(Debug, Clone)]
pub struct PathRecord {
    /// Item number
    pub item: u32,
    /// Path name
    pub name: String,
    /// Inode
    pub inode: u64,
    /// Device
    pub dev: u64,
    /// Mode
    pub mode: u32,
    /// Owner UID
    pub ouid: Uid,
    /// Owner GID
    pub ogid: Gid,
    /// File type
    pub nametype: String,
    /// Object context
    pub obj: Option<String>,
}

impl PathRecord {
    /// Create new path record
    pub fn new(item: u32, name: String) -> Self {
        Self {
            item,
            name,
            inode: 0,
            dev: 0,
            mode: 0,
            ouid: Uid::ROOT,
            ogid: Gid::ROOT,
            nametype: String::new(),
            obj: None,
        }
    }
}

/// Audit event
#[derive(Debug, Clone)]
pub struct AuditEvent {
    /// Event ID
    pub id: AuditEventId,
    /// Message type
    pub msg_type: AuditMessageType,
    /// Timestamp (nanoseconds)
    pub timestamp: u64,
    /// Serial number
    pub serial: u64,
    /// Process context
    pub process: ProcessContext,
    /// Syscall info (if syscall event)
    pub syscall: Option<SyscallInfo>,
    /// Result
    pub result: AuditResult,
    /// Path records
    pub paths: Vec<PathRecord>,
    /// Key (audit rule key)
    pub key: Option<String>,
    /// Architecture
    pub arch: u32,
    /// Raw message
    pub raw: Option<String>,
}

impl AuditEvent {
    /// Create new audit event
    pub fn new(
        id: AuditEventId,
        msg_type: AuditMessageType,
        timestamp: u64,
        process: ProcessContext,
    ) -> Self {
        Self {
            id,
            msg_type,
            timestamp,
            serial: 0,
            process,
            syscall: None,
            result: AuditResult::Unknown,
            paths: Vec::new(),
            key: None,
            arch: 0,
            raw: None,
        }
    }

    /// Is security event
    #[inline(always)]
    pub fn is_security_event(&self) -> bool {
        self.msg_type.is_security()
    }

    /// Is failure
    #[inline(always)]
    pub fn is_failure(&self) -> bool {
        matches!(self.result, AuditResult::Failure)
    }

    /// Set syscall info
    #[inline(always)]
    pub fn with_syscall(mut self, syscall: SyscallInfo) -> Self {
        self.syscall = Some(syscall);
        self
    }

    /// Set result
    #[inline(always)]
    pub fn with_result(mut self, result: AuditResult) -> Self {
        self.result = result;
        self
    }

    /// Add path record
    #[inline(always)]
    pub fn add_path(&mut self, path: PathRecord) {
        self.paths.push(path);
    }
}
