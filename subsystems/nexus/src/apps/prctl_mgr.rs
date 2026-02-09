//! # Apps Prctl Manager
//!
//! prctl operation management:
//! - PR_SET_NAME / PR_GET_NAME tracking
//! - PR_SET_DUMPABLE management
//! - PR_SET_SECCOMP mode tracking
//! - PR_SET_NO_NEW_PRIVS enforcement
//! - PR_SET_CHILD_SUBREAPER
//! - PR_SET_PDEATHSIG
//! - Per-process prctl state aggregation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::string::String;

/// Known prctl operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrctlOp {
    SetName,
    GetName,
    SetDumpable,
    GetDumpable,
    SetSeccomp,
    GetSeccomp,
    SetNoNewPrivs,
    GetNoNewPrivs,
    SetChildSubreaper,
    GetChildSubreaper,
    SetPdeathsig,
    GetPdeathsig,
    SetTimerSlack,
    GetTimerSlack,
    SetKeepCaps,
    GetKeepCaps,
    SetSecureBits,
    GetSecureBits,
    SetThpDisable,
    GetThpDisable,
    Other(u32),
}

impl PrctlOp {
    #[inline]
    pub fn is_set(&self) -> bool {
        matches!(self,
            Self::SetName | Self::SetDumpable | Self::SetSeccomp | Self::SetNoNewPrivs |
            Self::SetChildSubreaper | Self::SetPdeathsig | Self::SetTimerSlack |
            Self::SetKeepCaps | Self::SetSecureBits | Self::SetThpDisable
        )
    }

    #[inline]
    pub fn is_get(&self) -> bool {
        matches!(self,
            Self::GetName | Self::GetDumpable | Self::GetSeccomp | Self::GetNoNewPrivs |
            Self::GetChildSubreaper | Self::GetPdeathsig | Self::GetTimerSlack |
            Self::GetKeepCaps | Self::GetSecureBits | Self::GetThpDisable
        )
    }

    #[inline]
    pub fn is_security_related(&self) -> bool {
        matches!(self,
            Self::SetSeccomp | Self::GetSeccomp | Self::SetNoNewPrivs | Self::GetNoNewPrivs |
            Self::SetDumpable | Self::GetDumpable | Self::SetKeepCaps | Self::GetKeepCaps |
            Self::SetSecureBits | Self::GetSecureBits
        )
    }
}

/// Per-process prctl state
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ProcessPrctlState {
    pub pid: u64,
    pub name: String,
    pub dumpable: u8,
    pub seccomp_mode: u8,
    pub no_new_privs: bool,
    pub child_subreaper: bool,
    pub pdeathsig: u8,
    pub timer_slack_ns: u64,
    pub keep_caps: bool,
    pub secure_bits: u32,
    pub thp_disable: bool,
    pub op_count: u64,
    pub set_count: u64,
    pub security_ops: u64,
    pub last_op_ts: u64,
}

impl ProcessPrctlState {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, name: String::new(), dumpable: 1, seccomp_mode: 0,
            no_new_privs: false, child_subreaper: false, pdeathsig: 0,
            timer_slack_ns: 50000, keep_caps: false, secure_bits: 0,
            thp_disable: false, op_count: 0, set_count: 0,
            security_ops: 0, last_op_ts: 0,
        }
    }

    pub fn apply_op(&mut self, op: PrctlOp, arg: u64, ts: u64) {
        self.op_count += 1;
        self.last_op_ts = ts;
        if op.is_set() { self.set_count += 1; }
        if op.is_security_related() { self.security_ops += 1; }

        match op {
            PrctlOp::SetDumpable => self.dumpable = arg as u8,
            PrctlOp::SetSeccomp => self.seccomp_mode = arg as u8,
            PrctlOp::SetNoNewPrivs => self.no_new_privs = arg != 0,
            PrctlOp::SetChildSubreaper => self.child_subreaper = arg != 0,
            PrctlOp::SetPdeathsig => self.pdeathsig = arg as u8,
            PrctlOp::SetTimerSlack => self.timer_slack_ns = arg,
            PrctlOp::SetKeepCaps => self.keep_caps = arg != 0,
            PrctlOp::SetSecureBits => self.secure_bits = arg as u32,
            PrctlOp::SetThpDisable => self.thp_disable = arg != 0,
            _ => {}
        }
    }

    #[inline(always)]
    pub fn is_hardened(&self) -> bool {
        self.no_new_privs && self.seccomp_mode > 0 && self.dumpable == 0
    }

    #[inline(always)]
    pub fn is_reaper(&self) -> bool { self.child_subreaper }
}

/// Prctl operation record
#[derive(Debug, Clone)]
pub struct PrctlRecord {
    pub pid: u64,
    pub op: PrctlOp,
    pub arg: u64,
    pub timestamp: u64,
    pub success: bool,
}

/// Prctl manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct PrctlMgrStats {
    pub tracked_processes: usize,
    pub total_ops: u64,
    pub security_ops: u64,
    pub hardened_processes: usize,
    pub reaper_processes: usize,
    pub seccomp_enabled: usize,
    pub no_new_privs_set: usize,
}

/// Apps prctl manager
pub struct AppsPrctlMgr {
    processes: BTreeMap<u64, ProcessPrctlState>,
    records: VecDeque<PrctlRecord>,
    max_records: usize,
    stats: PrctlMgrStats,
}

impl AppsPrctlMgr {
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(), records: VecDeque::new(),
            max_records: 512, stats: PrctlMgrStats::default(),
        }
    }

    #[inline]
    pub fn record_op(&mut self, pid: u64, op: PrctlOp, arg: u64, success: bool, ts: u64) {
        let state = self.processes.entry(pid).or_insert_with(|| ProcessPrctlState::new(pid));
        if success { state.apply_op(op, arg, ts); }
        self.records.push_back(PrctlRecord { pid, op, arg, timestamp: ts, success });
        if self.records.len() > self.max_records { self.records.pop_front(); }
    }

    #[inline]
    pub fn set_name(&mut self, pid: u64, name: String, ts: u64) {
        let state = self.processes.entry(pid).or_insert_with(|| ProcessPrctlState::new(pid));
        state.name = name;
        state.op_count += 1;
        state.last_op_ts = ts;
    }

    #[inline(always)]
    pub fn process_exit(&mut self, pid: u64) { self.processes.remove(&pid); }

    pub fn fork_inherit(&mut self, parent: u64, child: u64) {
        if let Some(p) = self.processes.get(&parent) {
            let mut c = ProcessPrctlState::new(child);
            c.dumpable = p.dumpable;
            c.no_new_privs = p.no_new_privs;
            c.timer_slack_ns = p.timer_slack_ns;
            c.keep_caps = p.keep_caps;
            c.secure_bits = p.secure_bits;
            c.thp_disable = p.thp_disable;
            self.processes.insert(child, c);
        }
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.tracked_processes = self.processes.len();
        self.stats.total_ops = self.processes.values().map(|p| p.op_count).sum();
        self.stats.security_ops = self.processes.values().map(|p| p.security_ops).sum();
        self.stats.hardened_processes = self.processes.values().filter(|p| p.is_hardened()).count();
        self.stats.reaper_processes = self.processes.values().filter(|p| p.is_reaper()).count();
        self.stats.seccomp_enabled = self.processes.values().filter(|p| p.seccomp_mode > 0).count();
        self.stats.no_new_privs_set = self.processes.values().filter(|p| p.no_new_privs).count();
    }

    #[inline(always)]
    pub fn process(&self, pid: u64) -> Option<&ProcessPrctlState> { self.processes.get(&pid) }
    #[inline(always)]
    pub fn stats(&self) -> &PrctlMgrStats { &self.stats }
}
