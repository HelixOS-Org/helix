// SPDX-License-Identifier: GPL-2.0
//! NEXUS Bridge — Affinity (CPU affinity bridge)

extern crate alloc;
use alloc::collections::BTreeMap;

/// Affinity scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeAffinityScope {
    Thread,
    Process,
    ProcessGroup,
    CpuSet,
}

/// Affinity mask entry
#[derive(Debug, Clone)]
pub struct BridgeAffinityEntry {
    pub id: u64,
    pub scope: BridgeAffinityScope,
    pub cpu_mask: u64,
    pub preferred_cpu: u32,
    pub numa_node: u32,
    pub migration_count: u64,
}

/// Stats for affinity operations
#[derive(Debug, Clone)]
pub struct BridgeAffinityStats {
    pub total_sets: u64,
    pub total_gets: u64,
    pub migrations: u64,
    pub numa_violations: u64,
    pub mask_changes: u64,
}

/// Manager for affinity bridge operations
pub struct BridgeAffinityManager {
    entries: BTreeMap<u64, BridgeAffinityEntry>,
    stats: BridgeAffinityStats,
    max_cpus: u32,
}

impl BridgeAffinityManager {
    pub fn new(max_cpus: u32) -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: BridgeAffinityStats {
                total_sets: 0,
                total_gets: 0,
                migrations: 0,
                numa_violations: 0,
                mask_changes: 0,
            },
            max_cpus,
        }
    }

    pub fn set_affinity(&mut self, id: u64, scope: BridgeAffinityScope, cpu_mask: u64) {
        self.stats.total_sets += 1;
        if let Some(entry) = self.entries.get_mut(&id) {
            if entry.cpu_mask != cpu_mask {
                self.stats.mask_changes += 1;
                entry.cpu_mask = cpu_mask;
            }
        } else {
            let entry = BridgeAffinityEntry {
                id,
                scope,
                cpu_mask,
                preferred_cpu: 0,
                numa_node: 0,
                migration_count: 0,
            };
            self.entries.insert(id, entry);
        }
    }

    pub fn get_affinity(&mut self, id: u64) -> Option<u64> {
        self.stats.total_gets += 1;
        self.entries.get(&id).map(|e| e.cpu_mask)
    }

    pub fn record_migration(&mut self, id: u64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.migration_count += 1;
            self.stats.migrations += 1;
        }
    }

    pub fn stats(&self) -> &BridgeAffinityStats {
        &self.stats
    }
}
