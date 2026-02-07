//! # Application Environment Tracker
//!
//! Process environment and configuration analysis:
//! - Environment variable tracking
//! - Configuration drift detection
//! - Locale/timezone awareness
//! - Resource limit inheritance
//! - Namespace configuration

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ENVIRONMENT TYPES
// ============================================================================

/// Environment variable category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnvCategory {
    /// Path related (PATH, LD_LIBRARY_PATH)
    Path,
    /// Locale (LANG, LC_*)
    Locale,
    /// Display (DISPLAY, WAYLAND_DISPLAY)
    Display,
    /// Auth/credentials
    Auth,
    /// Runtime config
    Runtime,
    /// Debug/trace
    Debug,
    /// Custom/application specific
    Custom,
}

/// Namespace type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamespaceType {
    /// PID namespace
    Pid,
    /// Network namespace
    Net,
    /// Mount namespace
    Mnt,
    /// User namespace
    User,
    /// UTS namespace
    Uts,
    /// IPC namespace
    Ipc,
    /// Cgroup namespace
    Cgroup,
}

// ============================================================================
// ENVIRONMENT SNAPSHOT
// ============================================================================

/// Environment variable entry
#[derive(Debug, Clone)]
pub struct EnvEntry {
    /// Key hash
    pub key_hash: u64,
    /// Value hash
    pub value_hash: u64,
    /// Category
    pub category: EnvCategory,
    /// Value length
    pub value_len: usize,
    /// Is sensitive (auth related)
    pub is_sensitive: bool,
}

/// Environment snapshot
#[derive(Debug)]
pub struct EnvironmentSnapshot {
    /// Variables
    pub variables: BTreeMap<u64, EnvEntry>,
    /// Captured at
    pub captured_at: u64,
    /// Working directory hash
    pub cwd_hash: u64,
    /// Umask
    pub umask: u32,
}

impl EnvironmentSnapshot {
    pub fn new(now: u64) -> Self {
        Self {
            variables: BTreeMap::new(),
            captured_at: now,
            cwd_hash: 0,
            umask: 0o022,
        }
    }

    /// Add variable
    pub fn add_variable(&mut self, entry: EnvEntry) {
        self.variables.insert(entry.key_hash, entry);
    }

    /// Variable count
    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    /// Sensitive variable count
    pub fn sensitive_count(&self) -> usize {
        self.variables.values().filter(|e| e.is_sensitive).count()
    }

    /// Diff with another snapshot
    pub fn diff(&self, other: &EnvironmentSnapshot) -> EnvDiff {
        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();

        for (&key, entry) in &self.variables {
            if let Some(other_entry) = other.variables.get(&key) {
                if entry.value_hash != other_entry.value_hash {
                    changed.push(key);
                }
            } else {
                added.push(key);
            }
        }
        for &key in other.variables.keys() {
            if !self.variables.contains_key(&key) {
                removed.push(key);
            }
        }

        EnvDiff {
            added,
            removed,
            changed,
        }
    }
}

/// Diff between environments
#[derive(Debug, Clone)]
pub struct EnvDiff {
    /// Added keys
    pub added: Vec<u64>,
    /// Removed keys
    pub removed: Vec<u64>,
    /// Changed keys
    pub changed: Vec<u64>,
}

impl EnvDiff {
    /// Is empty (no changes)?
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }

    /// Total changes
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.removed.len() + self.changed.len()
    }
}

// ============================================================================
// NAMESPACE INFO
// ============================================================================

/// Process namespace info
#[derive(Debug, Clone)]
pub struct NamespaceInfo {
    /// Namespace type
    pub ns_type: NamespaceType,
    /// Namespace id (inode)
    pub ns_id: u64,
    /// Is custom (non-default)
    pub is_custom: bool,
}

/// Namespace set
#[derive(Debug)]
pub struct NamespaceSet {
    /// Namespaces
    namespaces: BTreeMap<u8, NamespaceInfo>,
}

impl NamespaceSet {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
        }
    }

    /// Set namespace
    pub fn set(&mut self, info: NamespaceInfo) {
        self.namespaces.insert(info.ns_type as u8, info);
    }

    /// Get namespace
    pub fn get(&self, ns_type: NamespaceType) -> Option<&NamespaceInfo> {
        self.namespaces.get(&(ns_type as u8))
    }

    /// Custom namespace count
    pub fn custom_count(&self) -> usize {
        self.namespaces.values().filter(|n| n.is_custom).count()
    }

    /// Is containerized? (multiple custom namespaces)
    pub fn is_containerized(&self) -> bool {
        self.custom_count() >= 3
    }
}

// ============================================================================
// ENVIRONMENT TRACKER ENGINE
// ============================================================================

/// Process environment
#[derive(Debug)]
pub struct ProcessEnvironment {
    /// Pid
    pub pid: u64,
    /// Current snapshot
    pub current: EnvironmentSnapshot,
    /// Previous snapshot (for drift detection)
    pub previous: Option<EnvironmentSnapshot>,
    /// Namespaces
    pub namespaces: NamespaceSet,
    /// Change count
    pub change_count: u64,
}

impl ProcessEnvironment {
    pub fn new(pid: u64, now: u64) -> Self {
        Self {
            pid,
            current: EnvironmentSnapshot::new(now),
            previous: None,
            namespaces: NamespaceSet::new(),
            change_count: 0,
        }
    }

    /// Update snapshot
    pub fn update(&mut self, new_snapshot: EnvironmentSnapshot) {
        let old = core::mem::replace(&mut self.current, new_snapshot);
        let diff = self.current.diff(&old);
        if !diff.is_empty() {
            self.change_count += 1;
        }
        self.previous = Some(old);
    }

    /// Has drifted?
    pub fn has_drifted(&self) -> bool {
        if let Some(ref prev) = self.previous {
            !self.current.diff(prev).is_empty()
        } else {
            false
        }
    }
}

/// Environment stats
#[derive(Debug, Clone, Default)]
pub struct AppEnvironmentStats {
    /// Tracked processes
    pub tracked_processes: usize,
    /// Containerized processes
    pub containerized_count: usize,
    /// Drifted environments
    pub drifted_count: usize,
}

/// App environment tracker
pub struct AppEnvironmentTracker {
    /// Processes
    processes: BTreeMap<u64, ProcessEnvironment>,
    /// Stats
    stats: AppEnvironmentStats,
}

impl AppEnvironmentTracker {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: AppEnvironmentStats::default(),
        }
    }

    /// Register process
    pub fn register(&mut self, pid: u64, now: u64) {
        self.processes.insert(pid, ProcessEnvironment::new(pid, now));
        self.update_stats();
    }

    /// Update environment
    pub fn update(&mut self, pid: u64, snapshot: EnvironmentSnapshot) {
        if let Some(proc_env) = self.processes.get_mut(&pid) {
            proc_env.update(snapshot);
        }
        self.update_stats();
    }

    /// Remove process
    pub fn remove(&mut self, pid: u64) {
        self.processes.remove(&pid);
        self.update_stats();
    }

    /// Get process env
    pub fn environment(&self, pid: u64) -> Option<&ProcessEnvironment> {
        self.processes.get(&pid)
    }

    fn update_stats(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.containerized_count = self.processes.values()
            .filter(|p| p.namespaces.is_containerized()).count();
        self.stats.drifted_count = self.processes.values()
            .filter(|p| p.has_drifted()).count();
    }

    /// Stats
    pub fn stats(&self) -> &AppEnvironmentStats {
        &self.stats
    }
}
