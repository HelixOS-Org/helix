//! Isolation Analyzer
//!
//! Analyzing process isolation across namespaces.

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

use super::{NamespaceId, NamespaceType, ProcessId};

/// Isolation level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IsolationLevel {
    /// No isolation (init namespace)
    None,
    /// Partial isolation
    Partial,
    /// Full isolation
    Full,
    /// Maximum isolation (all namespaces)
    Maximum,
}

/// Isolation analysis result
#[derive(Debug, Clone)]
pub struct IsolationAnalysis {
    /// Process ID
    pub pid: ProcessId,
    /// Overall isolation level
    pub level: IsolationLevel,
    /// Per-namespace isolation
    pub namespaces: BTreeMap<NamespaceType, bool>,
    /// Security concerns
    pub concerns: Vec<String>,
    /// Isolation score (0-100)
    pub score: f32,
}

/// Isolation analyzer
pub struct IsolationAnalyzer {
    /// Process namespace memberships
    process_ns: BTreeMap<ProcessId, BTreeMap<NamespaceType, NamespaceId>>,
    /// Initial namespaces (for comparison)
    init_ns: BTreeMap<NamespaceType, NamespaceId>,
    /// Analysis cache
    cache: BTreeMap<ProcessId, IsolationAnalysis>,
    /// Cache validity (timestamp)
    cache_valid_until: u64,
}

impl IsolationAnalyzer {
    /// Create new isolation analyzer
    pub fn new() -> Self {
        Self {
            process_ns: BTreeMap::new(),
            init_ns: BTreeMap::new(),
            cache: BTreeMap::new(),
            cache_valid_until: 0,
        }
    }

    /// Set initial namespace
    pub fn set_init_namespace(&mut self, ns_type: NamespaceType, ns_id: NamespaceId) {
        self.init_ns.insert(ns_type, ns_id);
    }

    /// Register process namespaces
    pub fn register_process(
        &mut self,
        pid: ProcessId,
        namespaces: BTreeMap<NamespaceType, NamespaceId>,
    ) {
        self.process_ns.insert(pid, namespaces);
        self.cache.remove(&pid);
    }

    /// Unregister process
    pub fn unregister_process(&mut self, pid: ProcessId) {
        self.process_ns.remove(&pid);
        self.cache.remove(&pid);
    }

    /// Analyze process isolation
    pub fn analyze(&mut self, pid: ProcessId) -> Option<IsolationAnalysis> {
        // Check cache
        if let Some(cached) = self.cache.get(&pid) {
            return Some(cached.clone());
        }

        let process_namespaces = self.process_ns.get(&pid)?;
        let mut isolated_count = 0;
        let mut total_count = 0;
        let mut namespaces = BTreeMap::new();
        let mut concerns = Vec::new();

        for ns_type in NamespaceType::all() {
            total_count += 1;

            let is_isolated = if let Some(&process_ns) = process_namespaces.get(ns_type) {
                if let Some(&init_ns) = self.init_ns.get(ns_type) {
                    process_ns != init_ns
                } else {
                    false
                }
            } else {
                false
            };

            namespaces.insert(*ns_type, is_isolated);

            if is_isolated {
                isolated_count += 1;
            } else {
                // Check for security concerns
                match ns_type {
                    NamespaceType::User => {
                        concerns.push(String::from("Process shares user namespace with host"));
                    }
                    NamespaceType::Net => {
                        concerns.push(String::from("Process shares network namespace with host"));
                    }
                    NamespaceType::Pid => {
                        concerns.push(String::from("Process can see host PIDs"));
                    }
                    _ => {}
                }
            }
        }

        let level = match isolated_count {
            0 => IsolationLevel::None,
            1..=3 => IsolationLevel::Partial,
            4..=6 => IsolationLevel::Full,
            _ => IsolationLevel::Maximum,
        };

        let score = (isolated_count as f32 / total_count as f32) * 100.0;

        let analysis = IsolationAnalysis {
            pid,
            level,
            namespaces,
            concerns,
            score,
        };

        self.cache.insert(pid, analysis.clone());
        Some(analysis)
    }

    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get processes in same namespace
    pub fn processes_in_namespace(
        &self,
        ns_type: NamespaceType,
        ns_id: NamespaceId,
    ) -> Vec<ProcessId> {
        self.process_ns
            .iter()
            .filter_map(|(pid, ns_map)| {
                if ns_map.get(&ns_type) == Some(&ns_id) {
                    Some(*pid)
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for IsolationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
