// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Prctl (cooperative process control)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// Process control cooperation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopPrctlType {
    NameChange,
    SecurityBit,
    ResourceLimit,
    SignalPolicy,
    TimerPolicy,
    SubreaperSet,
}

/// Process control entry
#[derive(Debug, Clone)]
pub struct CoopPrctlEntry {
    pub pid: u64,
    pub name: String,
    pub dumpable: bool,
    pub subreaper: bool,
    pub timer_slack_ns: u64,
    pub seccomp_level: u32,
}

/// Prctl cooperation stats
#[derive(Debug, Clone)]
pub struct CoopPrctlStats {
    pub total_ops: u64,
    pub name_changes: u64,
    pub security_changes: u64,
    pub subreaper_sets: u64,
    pub errors: u64,
}

/// Manager for cooperative prctl operations
pub struct CoopPrctlManager {
    entries: BTreeMap<u64, CoopPrctlEntry>,
    stats: CoopPrctlStats,
}

impl CoopPrctlManager {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: CoopPrctlStats {
                total_ops: 0,
                name_changes: 0,
                security_changes: 0,
                subreaper_sets: 0,
                errors: 0,
            },
        }
    }

    pub fn init_entry(&mut self, pid: u64, name: &str) {
        let entry = CoopPrctlEntry {
            pid,
            name: String::from(name),
            dumpable: true,
            subreaper: false,
            timer_slack_ns: 50000,
            seccomp_level: 0,
        };
        self.entries.insert(pid, entry);
    }

    pub fn set_name(&mut self, pid: u64, name: &str) -> bool {
        self.stats.total_ops += 1;
        if let Some(e) = self.entries.get_mut(&pid) {
            e.name = String::from(name);
            self.stats.name_changes += 1;
            true
        } else {
            self.stats.errors += 1;
            false
        }
    }

    pub fn set_subreaper(&mut self, pid: u64) -> bool {
        self.stats.total_ops += 1;
        if let Some(e) = self.entries.get_mut(&pid) {
            e.subreaper = true;
            self.stats.subreaper_sets += 1;
            true
        } else {
            false
        }
    }

    pub fn set_seccomp(&mut self, pid: u64, level: u32) -> bool {
        self.stats.total_ops += 1;
        if let Some(e) = self.entries.get_mut(&pid) {
            e.seccomp_level = level;
            self.stats.security_changes += 1;
            true
        } else {
            false
        }
    }

    pub fn stats(&self) -> &CoopPrctlStats {
        &self.stats
    }
}
