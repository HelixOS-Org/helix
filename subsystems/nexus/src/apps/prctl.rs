// SPDX-License-Identifier: GPL-2.0
//! Apps prctl_v2 — advanced process control operations manager.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
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
    SetTimerSlack,
    GetTimerSlack,
    SetChildSubreaper,
    GetChildSubreaper,
    SetPdeathsig,
    GetPdeathsig,
    SetKeepCaps,
    GetKeepCaps,
    SetTHP,
    SetMdwe,
    SetVmaAnon,
    SetFpMode,
}

/// Seccomp mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompMode {
    Disabled,
    Strict,
    Filter,
}

/// Dumpable setting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DumpableSetting {
    NotDumpable,
    Dumpable,
    Suid,
}

/// Process control state
#[derive(Debug)]
pub struct ProcessPrctlState {
    pub pid: u64,
    pub name: String,
    pub dumpable: DumpableSetting,
    pub seccomp: SeccompMode,
    pub no_new_privs: bool,
    pub timer_slack_ns: u64,
    pub child_subreaper: bool,
    pub pdeathsig: u32,
    pub keep_caps: bool,
    pub thp_disable: bool,
    pub mdwe: bool,
    pub fp_mode: u32,
    pub total_calls: u64,
}

impl ProcessPrctlState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, name: String::new(), dumpable: DumpableSetting::Dumpable,
            seccomp: SeccompMode::Disabled, no_new_privs: false,
            timer_slack_ns: 50_000, child_subreaper: false,
            pdeathsig: 0, keep_caps: false, thp_disable: false,
            mdwe: false, fp_mode: 0, total_calls: 0,
        }
    }

    pub fn apply(&mut self, opt: PrctlOption, arg: u64) -> i64 {
        self.total_calls += 1;
        match opt {
            PrctlOption::SetDumpable => { self.dumpable = match arg { 0 => DumpableSetting::NotDumpable, 1 => DumpableSetting::Dumpable, _ => DumpableSetting::Suid }; 0 }
            PrctlOption::GetDumpable => self.dumpable as i64,
            PrctlOption::SetNoNewPrivs => { self.no_new_privs = arg != 0; 0 }
            PrctlOption::GetNoNewPrivs => self.no_new_privs as i64,
            PrctlOption::SetTimerSlack => { self.timer_slack_ns = arg; 0 }
            PrctlOption::GetTimerSlack => self.timer_slack_ns as i64,
            PrctlOption::SetChildSubreaper => { self.child_subreaper = arg != 0; 0 }
            PrctlOption::GetChildSubreaper => self.child_subreaper as i64,
            PrctlOption::SetPdeathsig => { self.pdeathsig = arg as u32; 0 }
            PrctlOption::GetPdeathsig => self.pdeathsig as i64,
            PrctlOption::SetKeepCaps => { self.keep_caps = arg != 0; 0 }
            PrctlOption::GetKeepCaps => self.keep_caps as i64,
            PrctlOption::SetTHP => { self.thp_disable = arg != 0; 0 }
            PrctlOption::SetMdwe => { self.mdwe = arg != 0; 0 }
            PrctlOption::SetFpMode => { self.fp_mode = arg as u32; 0 }
            _ => -1,
        }
    }
}

/// Prctl audit record
#[derive(Debug, Clone)]
pub struct PrctlAuditRecord {
    pub pid: u64,
    pub option: PrctlOption,
    pub arg: u64,
    pub result: i64,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct PrctlV2Stats {
    pub total_processes: u32,
    pub total_calls: u64,
    pub no_new_privs_count: u32,
    pub seccomp_strict_count: u32,
    pub seccomp_filter_count: u32,
    pub subreaper_count: u32,
}

/// Main prctl v2 manager
pub struct AppPrctlV2 {
    processes: BTreeMap<u64, ProcessPrctlState>,
    audit_log: Vec<PrctlAuditRecord>,
    max_audit: usize,
}

impl AppPrctlV2 {
    pub fn new() -> Self {
        Self { processes: BTreeMap::new(), audit_log: Vec::new(), max_audit: 4096 }
    }

    pub fn prctl(&mut self, pid: u64, opt: PrctlOption, arg: u64, now: u64) -> i64 {
        let state = self.processes.entry(pid).or_insert_with(|| ProcessPrctlState::new(pid));
        let result = state.apply(opt, arg);
        if self.audit_log.len() >= self.max_audit { self.audit_log.drain(..self.max_audit / 4); }
        self.audit_log.push(PrctlAuditRecord { pid, option: opt, arg, result, timestamp: now });
        result
    }

    pub fn stats(&self) -> PrctlV2Stats {
        let calls: u64 = self.processes.values().map(|p| p.total_calls).sum();
        let nnp = self.processes.values().filter(|p| p.no_new_privs).count() as u32;
        let strict = self.processes.values().filter(|p| p.seccomp == SeccompMode::Strict).count() as u32;
        let filter = self.processes.values().filter(|p| p.seccomp == SeccompMode::Filter).count() as u32;
        let sr = self.processes.values().filter(|p| p.child_subreaper).count() as u32;
        PrctlV2Stats {
            total_processes: self.processes.len() as u32, total_calls: calls,
            no_new_privs_count: nnp, seccomp_strict_count: strict,
            seccomp_filter_count: filter, subreaper_count: sr,
        }
    }
}
