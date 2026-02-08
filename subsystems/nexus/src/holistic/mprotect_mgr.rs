//! # Holistic Memory Protection Manager
//!
//! Memory protection and access control management:
//! - Per-page and per-region permission tracking
//! - mprotect() operation handling
//! - Guard page management
//! - W^X enforcement
//! - Protection key (pkey) management
//! - Stack guard and canary verification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Memory protection flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtFlags {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
}

impl ProtFlags {
    pub const NONE: Self = Self { read: false, write: false, exec: false };
    pub const READ: Self = Self { read: true, write: false, exec: false };
    pub const RW: Self = Self { read: true, write: true, exec: false };
    pub const RX: Self = Self { read: true, write: false, exec: true };
    pub const RWX: Self = Self { read: true, write: true, exec: true };

    pub fn violates_wx(&self) -> bool { self.write && self.exec }

    pub fn allows(&self, op: AccessType) -> bool {
        match op {
            AccessType::Read => self.read,
            AccessType::Write => self.write,
            AccessType::Execute => self.exec,
        }
    }

    pub fn as_bits(&self) -> u8 {
        let mut bits = 0u8;
        if self.read { bits |= 1; }
        if self.write { bits |= 2; }
        if self.exec { bits |= 4; }
        bits
    }
}

/// Access type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
    Execute,
}

/// Protection violation
#[derive(Debug, Clone)]
pub struct ProtViolation {
    pub violation_id: u64,
    pub pid: u32,
    pub addr: u64,
    pub attempted: AccessType,
    pub region_flags: ProtFlags,
    pub timestamp_ns: u64,
    pub instruction_ptr: u64,
}

/// Protected memory region
#[derive(Debug, Clone)]
pub struct ProtRegion {
    pub start: u64,
    pub end: u64,
    pub flags: ProtFlags,
    pub pkey: Option<u16>,
    pub owner_pid: u32,
    pub is_guard: bool,
    pub change_count: u32,
    pub last_change_ts: u64,
}

impl ProtRegion {
    pub fn new(start: u64, end: u64, flags: ProtFlags, pid: u32, ts: u64) -> Self {
        Self {
            start, end, flags, pkey: None, owner_pid: pid,
            is_guard: false, change_count: 0, last_change_ts: ts,
        }
    }

    pub fn size(&self) -> u64 { self.end.saturating_sub(self.start) }
    pub fn contains(&self, addr: u64) -> bool { addr >= self.start && addr < self.end }

    pub fn change_protection(&mut self, new_flags: ProtFlags, ts: u64) {
        self.flags = new_flags;
        self.change_count += 1;
        self.last_change_ts = ts;
    }
}

/// Protection key (pkey) state
#[derive(Debug, Clone)]
pub struct ProtectionKey {
    pub pkey: u16,
    pub access_disable: bool,
    pub write_disable: bool,
    pub allocated_to: u32, // pid
    pub regions_using: u32,
}

impl ProtectionKey {
    pub fn new(pkey: u16, pid: u32) -> Self {
        Self { pkey, access_disable: false, write_disable: false, allocated_to: pid, regions_using: 0 }
    }
}

/// Stack guard state
#[derive(Debug, Clone)]
pub struct StackGuard {
    pub pid: u32,
    pub tid: u32,
    pub guard_start: u64,
    pub guard_end: u64,
    pub canary_value: u64,
    pub canary_valid: bool,
    pub violations: u32,
}

impl StackGuard {
    pub fn new(pid: u32, tid: u32, start: u64, end: u64, canary: u64) -> Self {
        Self {
            pid, tid, guard_start: start, guard_end: end,
            canary_value: canary, canary_valid: true, violations: 0,
        }
    }
}

/// W^X enforcement policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WxPolicy {
    /// Allow W+X (insecure)
    Permissive,
    /// Warn on W+X but allow
    Warn,
    /// Strictly enforce W^X
    Strict,
}

/// Per-process protection state
#[derive(Debug, Clone)]
pub struct ProcessProtState {
    pub pid: u32,
    pub regions: Vec<u64>, // region start addrs
    pub pkeys_allocated: Vec<u16>,
    pub violations: u64,
    pub wx_violations: u64,
    pub mprotect_calls: u64,
}

/// Mprotect manager stats
#[derive(Debug, Clone, Default)]
pub struct MprotectMgrStats {
    pub total_regions: usize,
    pub total_processes: usize,
    pub total_violations: u64,
    pub wx_violations: u64,
    pub total_mprotect_calls: u64,
    pub guard_pages: usize,
    pub pkeys_allocated: usize,
    pub avg_regions_per_process: f64,
}

/// Holistic memory protection manager
pub struct HolisticMprotectMgr {
    regions: BTreeMap<u64, ProtRegion>,
    processes: BTreeMap<u32, ProcessProtState>,
    pkeys: BTreeMap<u16, ProtectionKey>,
    guards: Vec<StackGuard>,
    violations: Vec<ProtViolation>,
    wx_policy: WxPolicy,
    next_violation_id: u64,
    max_violations: usize,
    stats: MprotectMgrStats,
}

impl HolisticMprotectMgr {
    pub fn new(policy: WxPolicy) -> Self {
        Self {
            regions: BTreeMap::new(), processes: BTreeMap::new(),
            pkeys: BTreeMap::new(), guards: Vec::new(),
            violations: Vec::new(), wx_policy: policy,
            next_violation_id: 1, max_violations: 10_000,
            stats: MprotectMgrStats::default(),
        }
    }

    pub fn register_process(&mut self, pid: u32) {
        self.processes.insert(pid, ProcessProtState {
            pid, regions: Vec::new(), pkeys_allocated: Vec::new(),
            violations: 0, wx_violations: 0, mprotect_calls: 0,
        });
    }

    pub fn add_region(&mut self, start: u64, end: u64, flags: ProtFlags, pid: u32, ts: u64) -> bool {
        if self.wx_policy == WxPolicy::Strict && flags.violates_wx() { return false; }
        let region = ProtRegion::new(start, end, flags, pid, ts);
        self.regions.insert(start, region);
        if let Some(proc) = self.processes.get_mut(&pid) { proc.regions.push(start); }
        true
    }

    pub fn mprotect(&mut self, start: u64, new_flags: ProtFlags, ts: u64) -> bool {
        if self.wx_policy == WxPolicy::Strict && new_flags.violates_wx() { return false; }
        if let Some(region) = self.regions.get_mut(&start) {
            let pid = region.owner_pid;
            if self.wx_policy == WxPolicy::Warn && new_flags.violates_wx() {
                if let Some(proc) = self.processes.get_mut(&pid) { proc.wx_violations += 1; }
            }
            region.change_protection(new_flags, ts);
            if let Some(proc) = self.processes.get_mut(&pid) { proc.mprotect_calls += 1; }
            true
        } else {
            false
        }
    }

    pub fn check_access(&mut self, addr: u64, access: AccessType, pid: u32, ip: u64, ts: u64) -> bool {
        for region in self.regions.values() {
            if region.contains(addr) && region.owner_pid == pid {
                if region.flags.allows(access) { return true; }
                let vid = self.next_violation_id; self.next_violation_id += 1;
                self.violations.push(ProtViolation {
                    violation_id: vid, pid, addr, attempted: access,
                    region_flags: region.flags, timestamp_ns: ts, instruction_ptr: ip,
                });
                if self.violations.len() > self.max_violations { self.violations.remove(0); }
                if let Some(proc) = self.processes.get_mut(&pid) { proc.violations += 1; }
                return false;
            }
        }
        false
    }

    pub fn add_guard(&mut self, guard: StackGuard) { self.guards.push(guard); }

    pub fn allocate_pkey(&mut self, pkey: u16, pid: u32) {
        self.pkeys.insert(pkey, ProtectionKey::new(pkey, pid));
        if let Some(proc) = self.processes.get_mut(&pid) { proc.pkeys_allocated.push(pkey); }
    }

    pub fn recompute(&mut self) {
        self.stats.total_regions = self.regions.len();
        self.stats.total_processes = self.processes.len();
        self.stats.total_violations = self.violations.len() as u64;
        self.stats.wx_violations = self.processes.values().map(|p| p.wx_violations).sum();
        self.stats.total_mprotect_calls = self.processes.values().map(|p| p.mprotect_calls).sum();
        self.stats.guard_pages = self.guards.len();
        self.stats.pkeys_allocated = self.pkeys.len();
        if !self.processes.is_empty() {
            self.stats.avg_regions_per_process = self.regions.len() as f64 / self.processes.len() as f64;
        }
    }

    pub fn stats(&self) -> &MprotectMgrStats { &self.stats }
}
