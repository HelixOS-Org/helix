// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — PID (cooperative PID namespace management)

extern crate alloc;
use alloc::collections::BTreeMap;

/// PID namespace level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopPidNsLevel {
    Root,
    Child,
    Nested,
    Container,
}

/// PID namespace entry
#[derive(Debug, Clone)]
pub struct CoopPidNsEntry {
    pub ns_id: u64,
    pub level: CoopPidNsLevel,
    pub parent_ns: Option<u64>,
    pub pid_count: u32,
    pub max_pids: u32,
}

/// PID mapping between namespaces
#[derive(Debug, Clone)]
pub struct CoopPidMapping {
    pub local_pid: u64,
    pub global_pid: u64,
    pub ns_id: u64,
}

/// PID cooperation stats
#[derive(Debug, Clone)]
pub struct CoopPidStats {
    pub total_namespaces: u64,
    pub total_mappings: u64,
    pub translations: u64,
    pub nested_lookups: u64,
    pub cache_hits: u64,
}

/// Manager for cooperative PID operations
pub struct CoopPidManager {
    namespaces: BTreeMap<u64, CoopPidNsEntry>,
    local_to_global: BTreeMap<(u64, u64), u64>,
    global_to_local: BTreeMap<(u64, u64), u64>,
    next_ns: u64,
    stats: CoopPidStats,
}

impl CoopPidManager {
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            local_to_global: BTreeMap::new(),
            global_to_local: BTreeMap::new(),
            next_ns: 1,
            stats: CoopPidStats {
                total_namespaces: 0,
                total_mappings: 0,
                translations: 0,
                nested_lookups: 0,
                cache_hits: 0,
            },
        }
    }

    pub fn create_namespace(&mut self, level: CoopPidNsLevel, parent: Option<u64>) -> u64 {
        let ns_id = self.next_ns;
        self.next_ns += 1;
        let entry = CoopPidNsEntry {
            ns_id,
            level,
            parent_ns: parent,
            pid_count: 0,
            max_pids: 32768,
        };
        self.namespaces.insert(ns_id, entry);
        self.stats.total_namespaces += 1;
        ns_id
    }

    pub fn add_mapping(&mut self, ns_id: u64, local: u64, global: u64) {
        self.local_to_global.insert((ns_id, local), global);
        self.global_to_local.insert((ns_id, global), local);
        if let Some(ns) = self.namespaces.get_mut(&ns_id) {
            ns.pid_count += 1;
        }
        self.stats.total_mappings += 1;
    }

    pub fn translate_to_global(&mut self, ns_id: u64, local: u64) -> Option<u64> {
        self.stats.translations += 1;
        self.local_to_global.get(&(ns_id, local)).cloned()
    }

    pub fn translate_to_local(&mut self, ns_id: u64, global: u64) -> Option<u64> {
        self.stats.translations += 1;
        self.global_to_local.get(&(ns_id, global)).cloned()
    }

    pub fn stats(&self) -> &CoopPidStats {
        &self.stats
    }
}
