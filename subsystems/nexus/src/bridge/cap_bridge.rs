// SPDX-License-Identifier: GPL-2.0
//! Bridge cap_bridge — Linux capability management bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Capability set type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapSetType {
    Effective,
    Permitted,
    Inheritable,
    Bounding,
    Ambient,
}

/// Known capability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    ChOwn, DacOverride, DacReadSearch, FOwner, FSetId,
    Kill, SetGid, SetUid, SetPcap, LinuxImmutable,
    NetBindService, NetBroadcast, NetAdmin, NetRaw,
    IpcLock, IpcOwner, SysModule, SysRawio, SysChroot,
    SysPtrace, SysPacket, SysAdmin, SysBoot, SysNice,
    SysResource, SysTime, SysTtyConfig, Mknod, Lease,
    AuditWrite, AuditControl, SetFCap, MacOverride,
    MacAdmin, Syslog, WakeAlarm, BlockSuspend,
    AuditRead, Perfmon, Bpf, CheckpointRestore,
}

/// Capability bitmask
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapBitmask(pub u64);

impl CapBitmask {
    pub fn empty() -> Self { Self(0) }
    pub fn full() -> Self { Self(u64::MAX) }
    pub fn set(&mut self, cap: u32) { self.0 |= 1u64 << cap; }
    pub fn clear(&mut self, cap: u32) { self.0 &= !(1u64 << cap); }
    pub fn has(&self, cap: u32) -> bool { self.0 & (1u64 << cap) != 0 }
    pub fn count(&self) -> u32 { self.0.count_ones() }
    pub fn intersect(&self, other: &Self) -> Self { Self(self.0 & other.0) }
    pub fn union(&self, other: &Self) -> Self { Self(self.0 | other.0) }
}

/// Process capability state
#[derive(Debug, Clone)]
pub struct ProcessCaps {
    pub pid: u64,
    pub effective: CapBitmask,
    pub permitted: CapBitmask,
    pub inheritable: CapBitmask,
    pub bounding: CapBitmask,
    pub ambient: CapBitmask,
    pub securebits: u32,
    pub no_new_privs: bool,
}

impl ProcessCaps {
    pub fn new(pid: u64) -> Self {
        Self {
            pid, effective: CapBitmask::empty(), permitted: CapBitmask::empty(),
            inheritable: CapBitmask::empty(), bounding: CapBitmask::full(),
            ambient: CapBitmask::empty(), securebits: 0, no_new_privs: false,
        }
    }

    pub fn capable(&self, cap: u32) -> bool { self.effective.has(cap) }

    pub fn raise_effective(&mut self, cap: u32) -> bool {
        if self.permitted.has(cap) { self.effective.set(cap); true } else { false }
    }

    pub fn drop_capability(&mut self, cap: u32) {
        self.effective.clear(cap);
        self.permitted.clear(cap);
    }

    pub fn exec_transform(&mut self, file_caps: &CapBitmask) {
        let new_permitted = self.inheritable.intersect(&self.bounding).union(file_caps);
        self.permitted = new_permitted;
        self.effective = new_permitted;
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct CapBridgeStats {
    pub total_processes: u32,
    pub privileged_processes: u32,
    pub total_checks: u64,
    pub total_denials: u64,
    pub avg_caps_per_process: f64,
}

/// Main cap bridge
pub struct BridgeCap {
    processes: BTreeMap<u64, ProcessCaps>,
    total_checks: u64,
    total_denials: u64,
}

impl BridgeCap {
    pub fn new() -> Self { Self { processes: BTreeMap::new(), total_checks: 0, total_denials: 0 } }

    pub fn register(&mut self, pid: u64) { self.processes.insert(pid, ProcessCaps::new(pid)); }

    pub fn check(&mut self, pid: u64, cap: u32) -> bool {
        self.total_checks += 1;
        if let Some(p) = self.processes.get(&pid) {
            if p.capable(cap) { true } else { self.total_denials += 1; false }
        } else { self.total_denials += 1; false }
    }

    pub fn stats(&self) -> CapBridgeStats {
        let privileged = self.processes.values().filter(|p| p.effective.count() > 0).count() as u32;
        let caps: Vec<f64> = self.processes.values().map(|p| p.effective.count() as f64).collect();
        let avg = if caps.is_empty() { 0.0 } else { caps.iter().sum::<f64>() / caps.len() as f64 };
        CapBridgeStats {
            total_processes: self.processes.len() as u32, privileged_processes: privileged,
            total_checks: self.total_checks, total_denials: self.total_denials,
            avg_caps_per_process: avg,
        }
    }
}

// ============================================================================
// Merged from cap_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CapIdV2 {
    Chown,
    DacOverride,
    DacReadSearch,
    Fowner,
    Fsetid,
    Kill,
    Setgid,
    Setuid,
    Setpcap,
    LinuxImmutable,
    NetBindService,
    NetBroadcast,
    NetAdmin,
    NetRaw,
    IpcLock,
    IpcOwner,
    SysModule,
    SysRawio,
    SysChroot,
    SysPtrace,
    SysAdmin,
    SysBoot,
    SysNice,
    SysResource,
    SysTime,
    SysTtyConfig,
    Mknod,
    Lease,
    AuditWrite,
    AuditControl,
    SysLog,
    WakeAlarm,
    BlockSuspend,
    AuditRead,
    Perfmon,
    Bpf,
    CheckpointRestore,
}

/// Capability set
#[derive(Debug, Clone)]
pub struct CapSetV2 {
    pub effective: u64,
    pub permitted: u64,
    pub inheritable: u64,
    pub bounding: u64,
    pub ambient: u64,
}

impl CapSetV2 {
    pub fn empty() -> Self { Self { effective: 0, permitted: 0, inheritable: 0, bounding: 0, ambient: 0 } }
    pub fn full() -> Self { Self { effective: u64::MAX, permitted: u64::MAX, inheritable: 0, bounding: u64::MAX, ambient: 0 } }

    pub fn has_effective(&self, bit: u32) -> bool { self.effective & (1u64 << bit) != 0 }
    pub fn grant(&mut self, bit: u32) { self.effective |= 1u64 << bit; self.permitted |= 1u64 << bit; }
    pub fn revoke(&mut self, bit: u32) { self.effective &= !(1u64 << bit); }
}

/// Process capability record
#[derive(Debug)]
pub struct ProcessCapsV2 {
    pub pid: u64,
    pub caps: CapSetV2,
    pub no_new_privs: bool,
    pub securebits: u32,
}

/// Stats
#[derive(Debug, Clone)]
pub struct CapV2BridgeStats {
    pub tracked_processes: u32,
    pub privileged_count: u32,
    pub no_new_privs_count: u32,
}

/// Main bridge capability v2
pub struct BridgeCapV2 {
    processes: BTreeMap<u64, ProcessCapsV2>,
}

impl BridgeCapV2 {
    pub fn new() -> Self { Self { processes: BTreeMap::new() } }

    pub fn register(&mut self, pid: u64, caps: CapSetV2) {
        self.processes.insert(pid, ProcessCapsV2 { pid, caps, no_new_privs: false, securebits: 0 });
    }

    pub fn check(&self, pid: u64, bit: u32) -> bool {
        self.processes.get(&pid).map_or(false, |p| p.caps.has_effective(bit))
    }

    pub fn stats(&self) -> CapV2BridgeStats {
        let priv_count = self.processes.values().filter(|p| p.caps.effective != 0).count() as u32;
        let nnp = self.processes.values().filter(|p| p.no_new_privs).count() as u32;
        CapV2BridgeStats { tracked_processes: self.processes.len() as u32, privileged_count: priv_count, no_new_privs_count: nnp }
    }
}
