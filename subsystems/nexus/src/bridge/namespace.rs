//! # Bridge Namespace Manager
//!
//! Namespace isolation for syscall bridge:
//! - Namespace types (PID, NET, MNT, UTS, IPC, USER)
//! - Namespace creation and joining
//! - Cross-namespace syscall translation
//! - Namespace hierarchy management
//! - Resource visibility control

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

// ============================================================================
// NAMESPACE TYPES
// ============================================================================

/// Namespace type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamespaceType {
    /// Process ID namespace
    Pid,
    /// Network namespace
    Network,
    /// Mount namespace
    Mount,
    /// UTS (hostname) namespace
    Uts,
    /// IPC namespace
    Ipc,
    /// User namespace
    User,
    /// Cgroup namespace
    Cgroup,
}

/// Namespace state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceState {
    /// Active
    Active,
    /// Being torn down
    Teardown,
    /// Zombie (no processes but references exist)
    Zombie,
    /// Destroyed
    Destroyed,
}

// ============================================================================
// NAMESPACE
// ============================================================================

/// A namespace instance
#[derive(Debug, Clone)]
pub struct Namespace {
    /// Unique namespace ID
    pub id: u64,
    /// Type
    pub ns_type: NamespaceType,
    /// State
    pub state: NamespaceState,
    /// Parent namespace ID
    pub parent_id: Option<u64>,
    /// Creator process ID
    pub creator_pid: u64,
    /// Creation timestamp
    pub created_ns: u64,
    /// Member process count
    pub member_count: u32,
    /// Child namespace count
    pub child_count: u32,
    /// Name (for UTS)
    pub name: String,
    /// Reference count
    pub ref_count: u32,
}

impl Namespace {
    pub fn new(id: u64, ns_type: NamespaceType, creator_pid: u64, now: u64) -> Self {
        Self {
            id,
            ns_type,
            state: NamespaceState::Active,
            parent_id: None,
            creator_pid,
            created_ns: now,
            member_count: 0,
            child_count: 0,
            name: String::new(),
            ref_count: 1,
        }
    }

    /// Add member
    pub fn add_member(&mut self) {
        self.member_count += 1;
    }

    /// Remove member
    pub fn remove_member(&mut self) {
        if self.member_count > 0 {
            self.member_count -= 1;
        }
        if self.member_count == 0 && self.child_count == 0 {
            self.state = NamespaceState::Zombie;
        }
    }

    /// Add child namespace
    pub fn add_child(&mut self) {
        self.child_count += 1;
    }

    /// Remove child namespace
    pub fn remove_child(&mut self) {
        if self.child_count > 0 {
            self.child_count -= 1;
        }
    }

    /// Take reference
    pub fn take_ref(&mut self) {
        self.ref_count += 1;
    }

    /// Drop reference
    pub fn drop_ref(&mut self) {
        if self.ref_count > 0 {
            self.ref_count -= 1;
        }
        if self.ref_count == 0 {
            self.state = NamespaceState::Destroyed;
        }
    }

    /// Is active
    pub fn is_active(&self) -> bool {
        self.state == NamespaceState::Active
    }
}

// ============================================================================
// PROCESS NAMESPACE SET
// ============================================================================

/// Namespace set for a process
#[derive(Debug, Clone)]
pub struct ProcessNamespaceSet {
    /// Process ID
    pub pid: u64,
    /// Namespace IDs per type
    pub namespaces: BTreeMap<u8, u64>,
}

impl ProcessNamespaceSet {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            namespaces: BTreeMap::new(),
        }
    }

    /// Set namespace for type
    pub fn set(&mut self, ns_type: NamespaceType, ns_id: u64) {
        self.namespaces.insert(ns_type as u8, ns_id);
    }

    /// Get namespace for type
    pub fn get(&self, ns_type: NamespaceType) -> Option<u64> {
        self.namespaces.get(&(ns_type as u8)).copied()
    }

    /// Are two processes in same namespace?
    pub fn shares_namespace(&self, other: &ProcessNamespaceSet, ns_type: NamespaceType) -> bool {
        match (self.get(ns_type), other.get(ns_type)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

// ============================================================================
// SYSCALL TRANSLATION
// ============================================================================

/// Translation rule for cross-namespace syscalls
#[derive(Debug, Clone)]
pub struct TranslationRule {
    /// Source namespace ID
    pub source_ns: u64,
    /// Target namespace ID
    pub target_ns: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Translation type
    pub translation: TranslationType,
}

/// Translation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationType {
    /// Pass through unchanged
    PassThrough,
    /// Translate PID
    TranslatePid,
    /// Translate file descriptor
    TranslateFd,
    /// Block (not allowed)
    Block,
    /// Emulate
    Emulate,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Namespace manager stats
#[derive(Debug, Clone, Default)]
pub struct BridgeNamespaceStats {
    /// Total namespaces
    pub total_namespaces: usize,
    /// Active namespaces
    pub active_namespaces: usize,
    /// Processes tracked
    pub tracked_processes: usize,
    /// Namespace creates
    pub creates: u64,
    /// Namespace destroys
    pub destroys: u64,
    /// Cross-namespace translations
    pub translations: u64,
    /// Blocked cross-namespace calls
    pub blocked: u64,
}

/// Bridge namespace manager
pub struct BridgeNamespaceManager {
    /// All namespaces
    namespaces: BTreeMap<u64, Namespace>,
    /// Per-process namespace sets
    process_sets: BTreeMap<u64, ProcessNamespaceSet>,
    /// Translation rules
    rules: Vec<TranslationRule>,
    /// Next namespace ID
    next_id: u64,
    /// Stats
    stats: BridgeNamespaceStats,
}

impl BridgeNamespaceManager {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            process_sets: BTreeMap::new(),
            rules: Vec::new(),
            next_id: 1,
            stats: BridgeNamespaceStats::default(),
        }
    }

    /// Create namespace
    pub fn create_namespace(&mut self, ns_type: NamespaceType, creator_pid: u64, parent_id: Option<u64>, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut ns = Namespace::new(id, ns_type, creator_pid, now);
        ns.parent_id = parent_id;
        if let Some(pid) = parent_id {
            if let Some(parent) = self.namespaces.get_mut(&pid) {
                parent.add_child();
            }
        }
        self.namespaces.insert(id, ns);
        self.stats.creates += 1;
        self.update_stats();
        id
    }

    /// Join process to namespace
    pub fn join(&mut self, pid: u64, ns_id: u64) -> bool {
        if let Some(ns) = self.namespaces.get_mut(&ns_id) {
            if !ns.is_active() {
                return false;
            }
            ns.add_member();
            let set = self.process_sets.entry(pid).or_insert_with(|| ProcessNamespaceSet::new(pid));
            set.set(ns.ns_type, ns_id);
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Leave namespace
    pub fn leave(&mut self, pid: u64, ns_type: NamespaceType) {
        if let Some(set) = self.process_sets.get_mut(&pid) {
            if let Some(ns_id) = set.get(ns_type) {
                if let Some(ns) = self.namespaces.get_mut(&ns_id) {
                    ns.remove_member();
                }
                set.namespaces.remove(&(ns_type as u8));
            }
        }
        self.update_stats();
    }

    /// Process exit: leave all namespaces
    pub fn process_exit(&mut self, pid: u64) {
        if let Some(set) = self.process_sets.remove(&pid) {
            for (_, &ns_id) in &set.namespaces {
                if let Some(ns) = self.namespaces.get_mut(&ns_id) {
                    ns.remove_member();
                }
            }
        }
        self.cleanup_zombies();
        self.update_stats();
    }

    /// Translate cross-namespace syscall
    pub fn translate(&mut self, source_pid: u64, target_pid: u64, syscall_nr: u32) -> TranslationType {
        self.stats.translations += 1;

        let source_set = match self.process_sets.get(&source_pid) {
            Some(s) => s.clone(),
            None => return TranslationType::Block,
        };
        let target_set = match self.process_sets.get(&target_pid) {
            Some(s) => s,
            None => return TranslationType::Block,
        };

        // Check PID namespace
        if !source_set.shares_namespace(target_set, NamespaceType::Pid) {
            // Check for specific rule
            for rule in &self.rules {
                if rule.syscall_nr == syscall_nr {
                    if let (Some(src_ns), Some(tgt_ns)) = (
                        source_set.get(NamespaceType::Pid),
                        target_set.get(NamespaceType::Pid),
                    ) {
                        if rule.source_ns == src_ns && rule.target_ns == tgt_ns {
                            return rule.translation;
                        }
                    }
                }
            }
            self.stats.blocked += 1;
            return TranslationType::Block;
        }

        TranslationType::PassThrough
    }

    /// Add translation rule
    pub fn add_rule(&mut self, rule: TranslationRule) {
        self.rules.push(rule);
    }

    /// Check if two processes share all namespaces
    pub fn same_context(&self, pid_a: u64, pid_b: u64) -> bool {
        match (self.process_sets.get(&pid_a), self.process_sets.get(&pid_b)) {
            (Some(a), Some(b)) => a.namespaces == b.namespaces,
            _ => false,
        }
    }

    fn cleanup_zombies(&mut self) {
        let zombies: Vec<u64> = self.namespaces.iter()
            .filter(|(_, ns)| ns.state == NamespaceState::Zombie && ns.ref_count == 0)
            .map(|(&id, _)| id)
            .collect();
        for id in zombies {
            if let Some(ns) = self.namespaces.remove(&id) {
                if let Some(parent_id) = ns.parent_id {
                    if let Some(parent) = self.namespaces.get_mut(&parent_id) {
                        parent.remove_child();
                    }
                }
                self.stats.destroys += 1;
            }
        }
    }

    fn update_stats(&mut self) {
        self.stats.total_namespaces = self.namespaces.len();
        self.stats.active_namespaces = self.namespaces.values().filter(|n| n.is_active()).count();
        self.stats.tracked_processes = self.process_sets.len();
    }

    /// Stats
    pub fn stats(&self) -> &BridgeNamespaceStats {
        &self.stats
    }
}
