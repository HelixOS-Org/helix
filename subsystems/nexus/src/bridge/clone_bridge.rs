// SPDX-License-Identifier: GPL-2.0
//! Bridge clone_bridge — process cloning bridge.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Clone flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneFlag {
    Vm,
    Fs,
    Files,
    Sighand,
    Pidfd,
    Ptrace,
    Vfork,
    Parent,
    Thread,
    NewNs,
    SysVSem,
    Detached,
    Untraced,
    NewCgroup,
    NewUts,
    NewIpc,
    NewUser,
    NewPid,
    NewNet,
    Io,
}

/// Clone request
#[derive(Debug)]
pub struct CloneRequest {
    pub id: u64,
    pub parent_pid: u64,
    pub child_pid: u64,
    pub flags: u64,
    pub stack_size: u64,
    pub timestamp: u64,
    pub duration_ns: u64,
    pub success: bool,
}

impl CloneRequest {
    pub fn new(id: u64, parent: u64, flags: u64, now: u64) -> Self {
        Self { id, parent_pid: parent, child_pid: 0, flags, stack_size: 0, timestamp: now, duration_ns: 0, success: false }
    }

    #[inline]
    pub fn has_flag(&self, flag: CloneFlag) -> bool {
        let bit = match flag {
            CloneFlag::Vm => 0x100, CloneFlag::Fs => 0x200, CloneFlag::Files => 0x400,
            CloneFlag::Sighand => 0x800, CloneFlag::Thread => 0x10000,
            CloneFlag::NewNs => 0x20000, CloneFlag::NewPid => 0x20000000,
            CloneFlag::NewNet => 0x40000000, CloneFlag::NewUser => 0x10000000,
            _ => 0,
        };
        self.flags & bit != 0
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CloneBridgeStats {
    pub total_clones: u32,
    pub successful: u32,
    pub failed: u32,
    pub thread_creates: u32,
    pub avg_duration_ns: u64,
}

/// Main clone bridge
#[repr(align(64))]
pub struct BridgeClone {
    requests: BTreeMap<u64, CloneRequest>,
    next_id: u64,
}

impl BridgeClone {
    pub fn new() -> Self { Self { requests: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn clone_process(&mut self, parent: u64, flags: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.requests.insert(id, CloneRequest::new(id, parent, flags, now));
        id
    }

    #[inline(always)]
    pub fn complete(&mut self, id: u64, child_pid: u64, dur: u64) {
        if let Some(r) = self.requests.get_mut(&id) { r.child_pid = child_pid; r.duration_ns = dur; r.success = true; }
    }

    #[inline]
    pub fn stats(&self) -> CloneBridgeStats {
        let ok = self.requests.values().filter(|r| r.success).count() as u32;
        let fail = self.requests.len() as u32 - ok;
        let threads = self.requests.values().filter(|r| r.has_flag(CloneFlag::Thread)).count() as u32;
        let durs: Vec<u64> = self.requests.values().filter(|r| r.success).map(|r| r.duration_ns).collect();
        let avg = if durs.is_empty() { 0 } else { durs.iter().sum::<u64>() / durs.len() as u64 };
        CloneBridgeStats { total_clones: self.requests.len() as u32, successful: ok, failed: fail, thread_creates: threads, avg_duration_ns: avg }
    }
}

// ============================================================================
// Merged from clone_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeCloneV2Flag {
    NewThread,
    NewPidNs,
    NewNetNs,
    NewMntNs,
    NewUserNs,
    NewUtsNs,
    NewIpcNs,
    NewCgroupNs,
    ShareVm,
    ShareFs,
    ShareFiles,
    ShareSignals,
}

/// Clone request descriptor
#[derive(Debug, Clone)]
pub struct BridgeCloneV2Request {
    pub parent_pid: u64,
    pub flags: Vec<BridgeCloneV2Flag>,
    pub stack_size: usize,
    pub tls_addr: u64,
    pub child_tid_addr: u64,
    pub timestamp: u64,
}

/// Clone result
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeCloneV2Result {
    pub child_pid: u64,
    pub child_tid: u64,
    pub namespaces_created: u32,
    pub latency_us: u64,
}

/// Stats for clone bridge operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct BridgeCloneV2Stats {
    pub total_clones: u64,
    pub thread_clones: u64,
    pub process_clones: u64,
    pub namespace_clones: u64,
    pub failed_clones: u64,
    pub avg_clone_us: u64,
}

/// Manager for clone bridge operations
#[repr(align(64))]
pub struct BridgeCloneV2Manager {
    pending: BTreeMap<u64, BridgeCloneV2Request>,
    results: Vec<BridgeCloneV2Result>,
    next_pid: u64,
    stats: BridgeCloneV2Stats,
}

impl BridgeCloneV2Manager {
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            results: Vec::new(),
            next_pid: 1000,
            stats: BridgeCloneV2Stats {
                total_clones: 0,
                thread_clones: 0,
                process_clones: 0,
                namespace_clones: 0,
                failed_clones: 0,
                avg_clone_us: 0,
            },
        }
    }

    pub fn clone_process(&mut self, parent_pid: u64, flags: Vec<BridgeCloneV2Flag>, stack_size: usize) -> u64 {
        let is_thread = flags.contains(&BridgeCloneV2Flag::NewThread);
        let ns_count = flags.iter().filter(|f| matches!(f,
            BridgeCloneV2Flag::NewPidNs | BridgeCloneV2Flag::NewNetNs |
            BridgeCloneV2Flag::NewMntNs | BridgeCloneV2Flag::NewUserNs
        )).count() as u32;
        let child_pid = self.next_pid;
        self.next_pid += 1;
        let result = BridgeCloneV2Result {
            child_pid,
            child_tid: child_pid,
            namespaces_created: ns_count,
            latency_us: if is_thread { 50 } else { 200 },
        };
        self.results.push(result);
        self.stats.total_clones += 1;
        if is_thread {
            self.stats.thread_clones += 1;
        } else {
            self.stats.process_clones += 1;
        }
        if ns_count > 0 {
            self.stats.namespace_clones += 1;
        }
        child_pid
    }

    #[inline(always)]
    pub fn stats(&self) -> &BridgeCloneV2Stats {
        &self.stats
    }
}
