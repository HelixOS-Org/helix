//! # Apps Clone Tracker
//!
//! Clone/fork tracking for application profiling:
//! - Clone flag combinations tracking
//! - Fork/vfork/clone/clone3 differentiation
//! - Thread creation pattern analysis
//! - Process tree building
//! - Namespace sharing tracking
//! - Clone latency profiling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Clone variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneVariant {
    Fork,
    Vfork,
    Clone,
    Clone3,
    PosixSpawn,
}

/// Clone flags
#[derive(Debug, Clone, Copy)]
pub struct CloneFlags {
    pub bits: u64,
}

impl CloneFlags {
    pub const CLONE_VM: u64 = 0x0100;
    pub const CLONE_FS: u64 = 0x0200;
    pub const CLONE_FILES: u64 = 0x0400;
    pub const CLONE_SIGHAND: u64 = 0x0800;
    pub const CLONE_THREAD: u64 = 0x10000;
    pub const CLONE_NEWNS: u64 = 0x20000;
    pub const CLONE_NEWPID: u64 = 0x20000000;
    pub const CLONE_NEWNET: u64 = 0x40000000;
    pub const CLONE_IO: u64 = 0x80000000;
    pub const CLONE_NEWUSER: u64 = 0x10000000;

    pub fn new(bits: u64) -> Self { Self { bits } }
    pub fn empty() -> Self { Self { bits: 0 } }
    pub fn has(&self, flag: u64) -> bool { self.bits & flag != 0 }
    pub fn is_thread(&self) -> bool { self.has(Self::CLONE_THREAD) }
    pub fn shares_vm(&self) -> bool { self.has(Self::CLONE_VM) }
    pub fn shares_files(&self) -> bool { self.has(Self::CLONE_FILES) }
    pub fn creates_namespace(&self) -> bool {
        self.has(Self::CLONE_NEWNS) || self.has(Self::CLONE_NEWPID) ||
        self.has(Self::CLONE_NEWNET) || self.has(Self::CLONE_NEWUSER)
    }
}

/// Clone event record
#[derive(Debug, Clone)]
pub struct CloneEvent {
    pub parent_pid: u64,
    pub child_pid: u64,
    pub variant: CloneVariant,
    pub flags: CloneFlags,
    pub timestamp: u64,
    pub latency_ns: u64,
    pub success: bool,
    pub exit_signal: u8,
}

impl CloneEvent {
    pub fn new(parent: u64, child: u64, variant: CloneVariant, flags: CloneFlags, ts: u64) -> Self {
        Self {
            parent_pid: parent, child_pid: child, variant, flags,
            timestamp: ts, latency_ns: 0, success: true, exit_signal: 17, // SIGCHLD
        }
    }
}

/// Process tree node
#[derive(Debug, Clone)]
pub struct ProcessTreeNode {
    pub pid: u64,
    pub parent_pid: u64,
    pub children: Vec<u64>,
    pub threads: Vec<u64>,
    pub clone_count: u32,
    pub thread_count: u32,
    pub fork_count: u32,
    pub created_ts: u64,
    pub last_clone_ts: u64,
}

impl ProcessTreeNode {
    pub fn new(pid: u64, parent: u64, ts: u64) -> Self {
        Self {
            pid, parent_pid: parent, children: Vec::new(), threads: Vec::new(),
            clone_count: 0, thread_count: 0, fork_count: 0,
            created_ts: ts, last_clone_ts: 0,
        }
    }

    pub fn add_child(&mut self, child: u64, is_thread: bool, ts: u64) {
        if is_thread {
            self.threads.push(child);
            self.thread_count += 1;
        } else {
            self.children.push(child);
            self.fork_count += 1;
        }
        self.clone_count += 1;
        self.last_clone_ts = ts;
    }

    pub fn remove_child(&mut self, child: u64) {
        self.children.retain(|&c| c != child);
        self.threads.retain(|&t| t != child);
    }

    pub fn total_descendants(&self) -> usize { self.children.len() + self.threads.len() }
}

/// Per-process clone pattern
#[derive(Debug, Clone)]
pub struct ClonePattern {
    pub pid: u64,
    pub thread_bursts: u32,
    pub fork_bursts: u32,
    pub avg_thread_interval_ns: u64,
    pub avg_clone_latency_ns: u64,
    pub namespace_clones: u32,
    pub total_latency_sum: u64,
    pub total_events: u32,
}

impl ClonePattern {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, thread_bursts: 0, fork_bursts: 0,
            avg_thread_interval_ns: 0, avg_clone_latency_ns: 0,
            namespace_clones: 0, total_latency_sum: 0, total_events: 0,
        }
    }

    pub fn record_event(&mut self, latency_ns: u64, is_thread: bool, creates_ns: bool) {
        self.total_events += 1;
        self.total_latency_sum += latency_ns;
        if self.total_events > 0 {
            self.avg_clone_latency_ns = self.total_latency_sum / self.total_events as u64;
        }
        if creates_ns { self.namespace_clones += 1; }
    }
}

/// Clone tracker stats
#[derive(Debug, Clone, Default)]
pub struct CloneTrackerStats {
    pub tracked_processes: usize,
    pub total_events: u64,
    pub total_forks: u64,
    pub total_threads: u64,
    pub total_namespace_clones: u64,
    pub avg_clone_latency_ns: u64,
    pub max_tree_depth: usize,
    pub failed_clones: u64,
}

/// Apps clone tracker
pub struct AppsCloneTracker {
    tree: BTreeMap<u64, ProcessTreeNode>,
    patterns: BTreeMap<u64, ClonePattern>,
    events: VecDeque<CloneEvent>,
    max_events: usize,
    stats: CloneTrackerStats,
}

impl AppsCloneTracker {
    pub fn new() -> Self {
        Self {
            tree: BTreeMap::new(), patterns: BTreeMap::new(),
            events: VecDeque::new(), max_events: 1024,
            stats: CloneTrackerStats::default(),
        }
    }

    pub fn record_clone(&mut self, parent: u64, child: u64, variant: CloneVariant, flags: CloneFlags, latency_ns: u64, ts: u64) {
        let is_thread = flags.is_thread();
        let creates_ns = flags.creates_namespace();

        // Update parent tree node
        let parent_node = self.tree.entry(parent).or_insert_with(|| ProcessTreeNode::new(parent, 0, ts));
        parent_node.add_child(child, is_thread, ts);

        // Create child tree node
        self.tree.insert(child, ProcessTreeNode::new(child, parent, ts));

        // Update pattern
        let pattern = self.patterns.entry(parent).or_insert_with(|| ClonePattern::new(parent));
        pattern.record_event(latency_ns, is_thread, creates_ns);

        // Record event
        let mut event = CloneEvent::new(parent, child, variant, flags, ts);
        event.latency_ns = latency_ns;
        self.events.push_back(event);
        if self.events.len() > self.max_events { self.events.pop_front(); }
    }

    pub fn record_exit(&mut self, pid: u64) {
        if let Some(node) = self.tree.get(&pid) {
            let parent = node.parent_pid;
            if let Some(p) = self.tree.get_mut(&parent) { p.remove_child(pid); }
        }
        self.tree.remove(&pid);
        self.patterns.remove(&pid);
    }

    pub fn tree_depth(&self, pid: u64) -> usize {
        let mut depth = 0;
        let mut current = pid;
        loop {
            if let Some(node) = self.tree.get(&current) {
                if node.parent_pid == 0 || node.parent_pid == current { break; }
                current = node.parent_pid;
                depth += 1;
                if depth > 128 { break; } // safety
            } else { break; }
        }
        depth
    }

    pub fn recompute(&mut self) {
        self.stats.tracked_processes = self.tree.len();
        self.stats.total_events = self.events.len() as u64;
        self.stats.total_forks = self.events.iter().filter(|e| !e.flags.is_thread()).count() as u64;
        self.stats.total_threads = self.events.iter().filter(|e| e.flags.is_thread()).count() as u64;
        self.stats.total_namespace_clones = self.events.iter().filter(|e| e.flags.creates_namespace()).count() as u64;
        self.stats.failed_clones = self.events.iter().filter(|e| !e.success).count() as u64;
        let total_lat: u64 = self.events.iter().map(|e| e.latency_ns).sum();
        if !self.events.is_empty() { self.stats.avg_clone_latency_ns = total_lat / self.events.len() as u64; }
    }

    pub fn tree_node(&self, pid: u64) -> Option<&ProcessTreeNode> { self.tree.get(&pid) }
    pub fn pattern(&self, pid: u64) -> Option<&ClonePattern> { self.patterns.get(&pid) }
    pub fn stats(&self) -> &CloneTrackerStats { &self.stats }
}
