// SPDX-License-Identifier: GPL-2.0
//! App prctl — process control syscall interface

extern crate alloc;
use alloc::vec::Vec;

/// Prctl option
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrctlOption {
    SetName,
    GetName,
    SetDumpable,
    GetDumpable,
    SetSeccomp,
    GetSeccomp,
    SetNoNewPrivs,
    GetNoNewPrivs,
    SetKeepCaps,
    GetKeepCaps,
    SetTimerSlack,
    GetTimerSlack,
    SetPdeathsig,
    GetPdeathsig,
}

/// Prctl app result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrctlResult {
    Success,
    InvalidArg,
    PermissionDenied,
    Nosys,
    Error,
}

/// Prctl record
#[derive(Debug, Clone)]
pub struct PrctlRecord {
    pub option: PrctlOption,
    pub result: PrctlResult,
    pub arg2: u64,
    pub pid: u32,
}

impl PrctlRecord {
    pub fn new(option: PrctlOption) -> Self {
        Self { option, result: PrctlResult::Success, arg2: 0, pid: 0 }
    }
}

/// Prctl app stats
#[derive(Debug, Clone)]
pub struct PrctlAppStats {
    pub total_ops: u64,
    pub gets: u64,
    pub sets: u64,
    pub errors: u64,
}

/// Main app prctl
#[derive(Debug)]
pub struct AppPrctl {
    pub stats: PrctlAppStats,
}

impl AppPrctl {
    pub fn new() -> Self {
        Self { stats: PrctlAppStats { total_ops: 0, gets: 0, sets: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &PrctlRecord) {
        self.stats.total_ops += 1;
        match rec.option {
            PrctlOption::GetName | PrctlOption::GetDumpable | PrctlOption::GetSeccomp |
            PrctlOption::GetNoNewPrivs | PrctlOption::GetKeepCaps | PrctlOption::GetTimerSlack |
            PrctlOption::GetPdeathsig => self.stats.gets += 1,
            _ => self.stats.sets += 1,
        }
        if rec.result != PrctlResult::Success { self.stats.errors += 1; }
    }
}

// ============================================================================
// Merged from prctl_v2_app
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppPrctlV2Op {
    SetName,
    GetName,
    SetDumpable,
    GetDumpable,
    SetSeccomp,
    SetNoNewPrivs,
    GetNoNewPrivs,
    SetTimerSlack,
    GetTimerSlack,
    SetChildSubreaper,
    GetChildSubreaper,
}

/// Prctl entry per process
#[derive(Debug, Clone)]
pub struct AppPrctlV2Entry {
    pub pid: u64,
    pub name: String,
    pub dumpable: bool,
    pub no_new_privs: bool,
    pub child_subreaper: bool,
    pub timer_slack_ns: u64,
    pub seccomp_mode: u32,
}

/// Stats for prctl operations
#[derive(Debug, Clone)]
pub struct AppPrctlV2Stats {
    pub total_ops: u64,
    pub name_changes: u64,
    pub seccomp_sets: u64,
    pub no_new_privs_sets: u64,
    pub errors: u64,
}

/// Manager for prctl application operations
pub struct AppPrctlV2Manager {
    entries: BTreeMap<u64, AppPrctlV2Entry>,
    stats: AppPrctlV2Stats,
}

impl AppPrctlV2Manager {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: AppPrctlV2Stats {
                total_ops: 0,
                name_changes: 0,
                seccomp_sets: 0,
                no_new_privs_sets: 0,
                errors: 0,
            },
        }
    }

    pub fn init_process(&mut self, pid: u64, name: &str) {
        let entry = AppPrctlV2Entry {
            pid,
            name: String::from(name),
            dumpable: true,
            no_new_privs: false,
            child_subreaper: false,
            timer_slack_ns: 50000,
            seccomp_mode: 0,
        };
        self.entries.insert(pid, entry);
    }

    pub fn set_name(&mut self, pid: u64, name: &str) -> bool {
        self.stats.total_ops += 1;
        if let Some(entry) = self.entries.get_mut(&pid) {
            entry.name = String::from(name);
            self.stats.name_changes += 1;
            true
        } else {
            self.stats.errors += 1;
            false
        }
    }

    pub fn set_no_new_privs(&mut self, pid: u64) -> bool {
        self.stats.total_ops += 1;
        if let Some(entry) = self.entries.get_mut(&pid) {
            entry.no_new_privs = true;
            self.stats.no_new_privs_sets += 1;
            true
        } else {
            false
        }
    }

    pub fn set_seccomp(&mut self, pid: u64, mode: u32) -> bool {
        self.stats.total_ops += 1;
        if let Some(entry) = self.entries.get_mut(&pid) {
            entry.seccomp_mode = mode;
            self.stats.seccomp_sets += 1;
            true
        } else {
            false
        }
    }

    pub fn get_entry(&self, pid: u64) -> Option<&AppPrctlV2Entry> {
        self.entries.get(&pid)
    }

    pub fn stats(&self) -> &AppPrctlV2Stats {
        &self.stats
    }
}
