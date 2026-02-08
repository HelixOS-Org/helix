//! # Bridge Process Lifecycle Manager
//!
//! Syscall bridge for process lifecycle operations:
//! - fork/clone/exec syscall handling
//! - Process group and session management
//! - Exit status propagation
//! - Orphan process reparenting
//! - Zombie reaping coordination
//! - Process tree tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Process state in lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcLifecycleState {
    Creating,
    Running,
    Stopped,
    Zombie,
    Dead,
}

/// Clone flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneFlagBridge {
    Thread,
    Files,
    Sighand,
    Vm,
    Fs,
    Newns,
    Newpid,
    Newuser,
    Newnet,
    Newipc,
    Vfork,
}

/// Exit reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitReason {
    Normal(i32),
    Signal(u32),
    GroupExit(i32),
    Coredump(u32),
    OomKill,
}

/// Process lifecycle entry
#[derive(Debug, Clone)]
pub struct ProcEntry {
    pub pid: u64,
    pub parent_pid: u64,
    pub tgid: u64,
    pub session_id: u64,
    pub pgrp: u64,
    pub state: ProcLifecycleState,
    pub exit_reason: Option<ExitReason>,
    pub created_ns: u64,
    pub exited_ns: u64,
    pub children: Vec<u64>,
    pub threads: Vec<u64>,
    pub clone_flags: Vec<CloneFlagBridge>,
    pub waited: bool,
}

impl ProcEntry {
    pub fn new(pid: u64, parent: u64, now: u64) -> Self {
        Self {
            pid,
            parent_pid: parent,
            tgid: pid,
            session_id: 0,
            pgrp: 0,
            state: ProcLifecycleState::Creating,
            exit_reason: None,
            created_ns: now,
            exited_ns: 0,
            children: Vec::new(),
            threads: Vec::new(),
            clone_flags: Vec::new(),
            waited: false,
        }
    }

    pub fn is_zombie(&self) -> bool { self.state == ProcLifecycleState::Zombie }
    pub fn is_alive(&self) -> bool {
        matches!(self.state, ProcLifecycleState::Creating | ProcLifecycleState::Running)
    }
    pub fn is_thread_leader(&self) -> bool { self.pid == self.tgid }

    pub fn lifetime_ns(&self) -> u64 {
        if self.exited_ns > 0 { self.exited_ns - self.created_ns }
        else { 0 }
    }
}

/// Process tree node (for efficient traversal)
#[derive(Debug, Clone)]
pub struct ProcTreeNode {
    pub pid: u64,
    pub depth: u32,
    pub subtree_size: u32,
}

/// Bridge Process Lifecycle stats
#[derive(Debug, Clone, Default)]
pub struct BridgeProcLifecycleStats {
    pub total_processes: usize,
    pub running: usize,
    pub zombies: usize,
    pub total_forks: u64,
    pub total_exits: u64,
    pub orphan_reparents: u64,
}

/// Bridge Process Lifecycle Manager
pub struct BridgeProcLifecycle {
    processes: BTreeMap<u64, ProcEntry>,
    init_pid: u64,
    next_pid: u64,
    stats: BridgeProcLifecycleStats,
}

impl BridgeProcLifecycle {
    pub fn new(init_pid: u64) -> Self {
        let mut mgr = Self {
            processes: BTreeMap::new(),
            init_pid,
            next_pid: init_pid + 1,
            stats: BridgeProcLifecycleStats::default(),
        };
        let mut init = ProcEntry::new(init_pid, 0, 0);
        init.state = ProcLifecycleState::Running;
        init.session_id = init_pid;
        init.pgrp = init_pid;
        mgr.processes.insert(init_pid, init);
        mgr
    }

    /// Fork a new process
    pub fn fork(&mut self, parent_pid: u64, flags: Vec<CloneFlagBridge>, now: u64) -> Option<u64> {
        if !self.processes.get(&parent_pid).map(|p| p.is_alive()).unwrap_or(false) {
            return None;
        }

        let child_pid = self.next_pid;
        self.next_pid += 1;

        let parent_session = self.processes.get(&parent_pid)
            .map(|p| p.session_id).unwrap_or(0);
        let parent_pgrp = self.processes.get(&parent_pid)
            .map(|p| p.pgrp).unwrap_or(0);

        let is_thread = flags.contains(&CloneFlagBridge::Thread);

        let mut child = ProcEntry::new(child_pid, parent_pid, now);
        child.state = ProcLifecycleState::Running;
        child.session_id = parent_session;
        child.pgrp = parent_pgrp;
        child.clone_flags = flags;

        if is_thread {
            child.tgid = self.processes.get(&parent_pid)
                .map(|p| p.tgid).unwrap_or(parent_pid);
            if let Some(leader) = self.processes.get_mut(&child.tgid) {
                leader.threads.push(child_pid);
            }
        }

        self.processes.insert(child_pid, child);
        if let Some(parent) = self.processes.get_mut(&parent_pid) {
            if !is_thread {
                parent.children.push(child_pid);
            }
        }

        self.stats.total_forks += 1;
        Some(child_pid)
    }

    /// Exit a process
    pub fn exit(&mut self, pid: u64, reason: ExitReason, now: u64) {
        // Reparent children to init
        let children = self.processes.get(&pid)
            .map(|p| p.children.clone())
            .unwrap_or_default();

        for child_pid in children {
            if let Some(child) = self.processes.get_mut(&child_pid) {
                child.parent_pid = self.init_pid;
                self.stats.orphan_reparents += 1;
            }
            if let Some(init) = self.processes.get_mut(&self.init_pid) {
                if !init.children.contains(&child_pid) {
                    init.children.push(child_pid);
                }
            }
        }

        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.state = ProcLifecycleState::Zombie;
            proc.exit_reason = Some(reason);
            proc.exited_ns = now;
        }

        self.stats.total_exits += 1;
    }

    /// Wait (reap) a zombie child
    pub fn wait(&mut self, parent_pid: u64, child_pid: Option<u64>) -> Option<(u64, ExitReason)> {
        let target = if let Some(cpid) = child_pid {
            if self.processes.get(&cpid).map(|p| p.is_zombie()).unwrap_or(false) {
                Some(cpid)
            } else { None }
        } else {
            // Find any zombie child
            self.processes.get(&parent_pid)
                .and_then(|parent| {
                    parent.children.iter()
                        .find(|&&c| self.processes.get(&c).map(|p| p.is_zombie()).unwrap_or(false))
                        .copied()
                })
        };

        if let Some(zpid) = target {
            let reason = self.processes.get(&zpid)
                .and_then(|p| p.exit_reason)?;
            // Remove zombie
            if let Some(proc) = self.processes.get_mut(&zpid) {
                proc.state = ProcLifecycleState::Dead;
                proc.waited = true;
            }
            // Remove from parent's children
            if let Some(parent) = self.processes.get_mut(&parent_pid) {
                parent.children.retain(|&c| c != zpid);
            }
            Some((zpid, reason))
        } else { None }
    }

    pub fn recompute(&mut self) {
        self.stats.total_processes = self.processes.len();
        self.stats.running = self.processes.values()
            .filter(|p| p.state == ProcLifecycleState::Running).count();
        self.stats.zombies = self.processes.values()
            .filter(|p| p.is_zombie()).count();
    }

    pub fn process(&self, pid: u64) -> Option<&ProcEntry> { self.processes.get(&pid) }
    pub fn stats(&self) -> &BridgeProcLifecycleStats { &self.stats }
}
