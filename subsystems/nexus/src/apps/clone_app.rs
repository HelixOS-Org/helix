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

    pub fn register(&mut self, pid: u64, ppid: u64) { self.procs.insert(pid, ProcessTreeNode::new(pid, ppid)); }

    pub fn do_fork(&mut self, ppid: u64, child: u64) {
        if let Some(p) = self.procs.get_mut(&ppid) { p.total_forks += 1; p.children_count += 1; }
        self.procs.insert(child, ProcessTreeNode::new(child, ppid));
    }

    pub fn do_clone(&mut self, ppid: u64, child: u64, flags: u32) {
        if let Some(p) = self.procs.get_mut(&ppid) { p.total_clones += 1; p.children_count += 1; }
        let mut node = ProcessTreeNode::new(child, ppid);
        node.flags = flags;
        self.procs.insert(child, node);
    }

    pub fn do_vfork(&mut self, ppid: u64, child: u64) {
        if let Some(p) = self.procs.get_mut(&ppid) { p.vfork_count += 1; p.children_count += 1; }
        self.procs.insert(child, ProcessTreeNode::new(child, ppid));
    }

    pub fn exit(&mut self, pid: u64) { self.procs.remove(&pid); }

    pub fn stats(&self) -> CloneAppStats {
        let clones: u64 = self.procs.values().map(|p| p.total_clones).sum();
        let forks: u64 = self.procs.values().map(|p| p.total_forks).sum();
        let vforks: u64 = self.procs.values().map(|p| p.vfork_count).sum();
        CloneAppStats { total_processes: self.procs.len() as u32, total_clones: clones, total_forks: forks, total_vforks: vforks }
    }
}
