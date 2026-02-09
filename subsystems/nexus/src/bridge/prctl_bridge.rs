// SPDX-License-Identifier: GPL-2.0
//! Bridge prctl_bridge — prctl process control bridge.

extern crate alloc;

use alloc::collections::BTreeMap;

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
    SetKeepCaps,
    GetKeepCaps,
    SetTHP,
    CapBsetRead,
    CapBsetDrop,
    SetMDWE,
}

/// Process prctl state
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessPrctlState {
    pub pid: u64,
    pub dumpable: bool,
    pub no_new_privs: bool,
    pub keep_caps: bool,
    pub child_subreaper: bool,
    pub timer_slack_ns: u64,
    pub seccomp_mode: u32,
    pub name_hash: u64,
    pub thp_mode: u32,
    pub total_calls: u64,
}

impl ProcessPrctlState {
    pub fn new(pid: u64) -> Self {
        Self { pid, dumpable: true, no_new_privs: false, keep_caps: false, child_subreaper: false, timer_slack_ns: 50000, seccomp_mode: 0, name_hash: 0, thp_mode: 0, total_calls: 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PrctlBridgeStats {
    pub tracked_procs: u32,
    pub total_calls: u64,
    pub no_new_privs_count: u32,
    pub seccomp_enabled: u32,
}

/// Main bridge prctl
#[repr(align(64))]
pub struct BridgePrctl {
    procs: BTreeMap<u64, ProcessPrctlState>,
}

impl BridgePrctl {
    pub fn new() -> Self { Self { procs: BTreeMap::new() } }

    #[inline(always)]
    pub fn track(&mut self, pid: u64) { self.procs.insert(pid, ProcessPrctlState::new(pid)); }

    pub fn prctl(&mut self, pid: u64, opt: PrctlOption, arg: u64) {
        if let Some(p) = self.procs.get_mut(&pid) {
            p.total_calls += 1;
            match opt {
                PrctlOption::SetDumpable => p.dumpable = arg != 0,
                PrctlOption::SetNoNewPrivs => p.no_new_privs = arg != 0,
                PrctlOption::SetKeepCaps => p.keep_caps = arg != 0,
                PrctlOption::SetChildSubreaper => p.child_subreaper = arg != 0,
                PrctlOption::SetTimerSlack => p.timer_slack_ns = arg,
                PrctlOption::SetSeccomp => p.seccomp_mode = arg as u32,
                PrctlOption::SetName => p.name_hash = arg,
                PrctlOption::SetTHP => p.thp_mode = arg as u32,
                _ => {}
            }
        }
    }

    #[inline(always)]
    pub fn untrack(&mut self, pid: u64) { self.procs.remove(&pid); }

    #[inline]
    pub fn stats(&self) -> PrctlBridgeStats {
        let calls: u64 = self.procs.values().map(|p| p.total_calls).sum();
        let nnp = self.procs.values().filter(|p| p.no_new_privs).count() as u32;
        let seccomp = self.procs.values().filter(|p| p.seccomp_mode > 0).count() as u32;
        PrctlBridgeStats { tracked_procs: self.procs.len() as u32, total_calls: calls, no_new_privs_count: nnp, seccomp_enabled: seccomp }
    }
}

// ============================================================================
// Merged from prctl_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrctlV2Op {
    SetName,
    GetName,
    SetDumpable,
    GetDumpable,
    SetSecurebits,
    GetSecurebits,
    SetNoNewPrivs,
    GetNoNewPrivs,
    SetTimerSlack,
    GetTimerSlack,
    SetChildSubreaper,
    GetChildSubreaper,
    SetPdeathsig,
    GetPdeathsig,
    SetThpDisable,
    GetThpDisable,
    SetMdwe,
    GetMdwe,
    SetVma,
    GetAuxv,
    SetMmExeFile,
    SetSpecCtrl,
    GetSpecCtrl,
    PacResetKeys,
    TaggedAddrCtrl,
    SveSdSetVl,
    SmeSetVl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrctlV2SpecCtrl {
    Disable,
    Enable,
    ForceDisable,
    NotAffected,
    PrctlStoreBypass,
    IndirectBranch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrctlV2Securebits {
    Noroot,
    NorootLocked,
    NoSetuidFixup,
    NoSetuidFixupLocked,
    KeepCaps,
    KeepCapsLocked,
    NoCapAmbientRaise,
    NoCapAmbientRaiseLocked,
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PrctlV2ProcessState {
    pub pid: u64,
    pub name_hash: u64,
    pub dumpable: bool,
    pub no_new_privs: bool,
    pub child_subreaper: bool,
    pub pdeathsig: u32,
    pub timer_slack_ns: u64,
    pub securebits: u32,
    pub thp_disabled: bool,
    pub mdwe_enabled: bool,
    pub spec_ctrl_flags: u32,
    pub tagged_addr_enabled: bool,
    pub total_ops: u64,
}

impl PrctlV2ProcessState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            name_hash: 0,
            dumpable: true,
            no_new_privs: false,
            child_subreaper: false,
            pdeathsig: 0,
            timer_slack_ns: 50_000,
            securebits: 0,
            thp_disabled: false,
            mdwe_enabled: false,
            spec_ctrl_flags: 0,
            tagged_addr_enabled: false,
            total_ops: 0,
        }
    }

    #[inline]
    pub fn set_name(&mut self, name: &[u8]) {
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in name {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        self.name_hash = h;
        self.total_ops += 1;
    }

    #[inline]
    pub fn set_securebit(&mut self, bit: PrctlV2Securebits) {
        let flag = 1u32 << (bit as u32);
        self.securebits |= flag;
        self.total_ops += 1;
    }

    #[inline(always)]
    pub fn has_securebit(&self, bit: PrctlV2Securebits) -> bool {
        let flag = 1u32 << (bit as u32);
        (self.securebits & flag) != 0
    }

    #[inline(always)]
    pub fn is_locked_down(&self) -> bool {
        self.no_new_privs && self.mdwe_enabled && !self.dumpable
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct PrctlV2BridgeStats {
    pub total_processes: u64,
    pub total_ops: u64,
    pub no_new_privs_count: u64,
    pub subreaper_count: u64,
    pub mdwe_count: u64,
}

#[repr(align(64))]
pub struct BridgePrctlV2 {
    processes: BTreeMap<u64, PrctlV2ProcessState>,
    stats: PrctlV2BridgeStats,
}

impl BridgePrctlV2 {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            stats: PrctlV2BridgeStats {
                total_processes: 0,
                total_ops: 0,
                no_new_privs_count: 0,
                subreaper_count: 0,
                mdwe_count: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register_process(&mut self, pid: u64) {
        self.processes.insert(pid, PrctlV2ProcessState::new(pid));
        self.stats.total_processes += 1;
    }

    #[inline]
    pub fn set_no_new_privs(&mut self, pid: u64) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.no_new_privs = true;
            p.total_ops += 1;
            self.stats.no_new_privs_count += 1;
            self.stats.total_ops += 1;
        }
    }

    #[inline]
    pub fn set_child_subreaper(&mut self, pid: u64) {
        if let Some(p) = self.processes.get_mut(&pid) {
            p.child_subreaper = true;
            p.total_ops += 1;
            self.stats.subreaper_count += 1;
            self.stats.total_ops += 1;
        }
    }

    #[inline(always)]
    pub fn stats(&self) -> &PrctlV2BridgeStats {
        &self.stats
    }
}
