// SPDX-License-Identifier: GPL-2.0
//! Apps clone_app — process clone/fork application layer.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Clone flag type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneAppFlag {
    CloneVm,
    CloneFs,
    CloneFiles,
    CloneSignal,
    ClonePtrace,
    CloneVfork,
    CloneParent,
    CloneThread,
    CloneNewNs,
    CloneSysVSem,
    CloneSettls,
    CloneParentSettid,
    CloneChildCleartid,
    CloneDetached,
    CloneUntraced,
    CloneNewCgroup,
    CloneNewUts,
    CloneNewIpc,
    CloneNewUser,
    CloneNewPid,
    CloneNewNet,
    CloneIo,
    CloneClear,
    CloneIntoGroup,
}

/// Clone result
#[derive(Debug)]
pub struct CloneResult {
    pub parent_pid: u64,
    pub child_pid: u64,
    pub flags: u32,
    pub shared_vm: bool,
    pub shared_fs: bool,
    pub shared_files: bool,
    pub timestamp: u64,
}

/// Process tree node
#[derive(Debug)]
pub struct ProcessTreeNode {
    pub pid: u64,
    pub parent_pid: u64,
    pub flags: u32,
    pub children_count: u32,
    pub total_clones: u64,
    pub total_forks: u64,
    pub vfork_count: u64,
}

impl ProcessTreeNode {
    pub fn new(pid: u64, ppid: u64) -> Self {
        Self { pid, parent_pid: ppid, flags: 0, children_count: 0, total_clones: 0, total_forks: 0, vfork_count: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CloneAppStats {
    pub total_processes: u32,
    pub total_clones: u64,
    pub total_forks: u64,
    pub total_vforks: u64,
}

/// Main app clone
pub struct AppClone {
    procs: BTreeMap<u64, ProcessTreeNode>,
}

impl AppClone {
    pub fn new() -> Self { Self { procs: BTreeMap::new() } }

    #[inline(always)]
    pub fn register(&mut self, pid: u64, ppid: u64) { self.procs.insert(pid, ProcessTreeNode::new(pid, ppid)); }

    #[inline(always)]
    pub fn do_fork(&mut self, ppid: u64, child: u64) {
        if let Some(p) = self.procs.get_mut(&ppid) { p.total_forks += 1; p.children_count += 1; }
        self.procs.insert(child, ProcessTreeNode::new(child, ppid));
    }

    #[inline]
    pub fn do_clone(&mut self, ppid: u64, child: u64, flags: u32) {
        if let Some(p) = self.procs.get_mut(&ppid) { p.total_clones += 1; p.children_count += 1; }
        let mut node = ProcessTreeNode::new(child, ppid);
        node.flags = flags;
        self.procs.insert(child, node);
    }

    #[inline(always)]
    pub fn do_vfork(&mut self, ppid: u64, child: u64) {
        if let Some(p) = self.procs.get_mut(&ppid) { p.vfork_count += 1; p.children_count += 1; }
        self.procs.insert(child, ProcessTreeNode::new(child, ppid));
    }

    #[inline(always)]
    pub fn exit(&mut self, pid: u64) { self.procs.remove(&pid); }

    #[inline]
    pub fn stats(&self) -> CloneAppStats {
        let clones: u64 = self.procs.values().map(|p| p.total_clones).sum();
        let forks: u64 = self.procs.values().map(|p| p.total_forks).sum();
        let vforks: u64 = self.procs.values().map(|p| p.vfork_count).sum();
        CloneAppStats { total_processes: self.procs.len() as u32, total_clones: clones, total_forks: forks, total_vforks: vforks }
    }
}

// ============================================================================
// Merged from clone_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCloneV2Flag {
    NewThread,
    ShareVm,
    ShareFs,
    ShareFiles,
    ShareSignals,
    NewPidNs,
    NewNetNs,
    NewMntNs,
    NewUserNs,
    Detached,
}

/// Clone result
#[derive(Debug, Clone)]
pub struct AppCloneV2Result {
    pub child_id: u64,
    pub is_thread: bool,
    pub namespaces_created: u32,
    pub latency_us: u64,
}

/// Stats for clone operations
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AppCloneV2Stats {
    pub total_clones: u64,
    pub thread_clones: u64,
    pub process_clones: u64,
    pub ns_clones: u64,
    pub failed: u64,
}

/// Manager for clone application operations
pub struct AppCloneV2Manager {
    results: Vec<AppCloneV2Result>,
    parent_children: BTreeMap<u64, Vec<u64>>,
    next_id: u64,
    stats: AppCloneV2Stats,
}

impl AppCloneV2Manager {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            parent_children: BTreeMap::new(),
            next_id: 30000,
            stats: AppCloneV2Stats {
                total_clones: 0,
                thread_clones: 0,
                process_clones: 0,
                ns_clones: 0,
                failed: 0,
            },
        }
    }

    pub fn clone_process(&mut self, parent: u64, flags: &[AppCloneV2Flag]) -> AppCloneV2Result {
        let child = self.next_id;
        self.next_id += 1;
        let is_thread = flags.contains(&AppCloneV2Flag::NewThread);
        let ns_count = flags.iter().filter(|f| matches!(f,
            AppCloneV2Flag::NewPidNs | AppCloneV2Flag::NewNetNs |
            AppCloneV2Flag::NewMntNs | AppCloneV2Flag::NewUserNs
        )).count() as u32;
        let result = AppCloneV2Result {
            child_id: child,
            is_thread,
            namespaces_created: ns_count,
            latency_us: if is_thread { 40 } else { 180 },
        };
        self.parent_children.entry(parent).or_insert_with(Vec::new).push(child);
        self.results.push(result.clone());
        self.stats.total_clones += 1;
        if is_thread { self.stats.thread_clones += 1; } else { self.stats.process_clones += 1; }
        if ns_count > 0 { self.stats.ns_clones += 1; }
        result
    }

    #[inline(always)]
    pub fn stats(&self) -> &AppCloneV2Stats {
        &self.stats
    }
}
