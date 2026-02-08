// SPDX-License-Identifier: GPL-2.0
//! Apps membarrier_app — memory barrier operations.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Membarrier command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembarrierCmd {
    Query,
    Global,
    GlobalExpedited,
    RegisterGlobalExpedited,
    Private,
    PrivateExpedited,
    RegisterPrivateExpedited,
    PrivateExpeditedSyncCore,
    RegisterPrivateExpeditedSyncCore,
    PrivateExpeditedRseq,
    RegisterPrivateExpeditedRseq,
}

/// Registration state per process
#[derive(Debug)]
pub struct MembarrierReg {
    pub pid: u64,
    pub global_expedited: bool,
    pub private_expedited: bool,
    pub sync_core: bool,
    pub rseq: bool,
    pub total_barriers: u64,
    pub total_ipi_sent: u64,
}

impl MembarrierReg {
    pub fn new(pid: u64) -> Self {
        Self { pid, global_expedited: false, private_expedited: false, sync_core: false, rseq: false, total_barriers: 0, total_ipi_sent: 0 }
    }

    pub fn register(&mut self, cmd: MembarrierCmd) {
        match cmd {
            MembarrierCmd::RegisterGlobalExpedited => self.global_expedited = true,
            MembarrierCmd::RegisterPrivateExpedited => self.private_expedited = true,
            MembarrierCmd::RegisterPrivateExpeditedSyncCore => self.sync_core = true,
            MembarrierCmd::RegisterPrivateExpeditedRseq => self.rseq = true,
            _ => {}
        }
    }

    pub fn supported(&self) -> u32 {
        let mut s = 0u32;
        if self.global_expedited { s |= 1; }
        if self.private_expedited { s |= 2; }
        if self.sync_core { s |= 4; }
        if self.rseq { s |= 8; }
        s
    }
}

/// Barrier event
#[derive(Debug, Clone)]
pub struct BarrierEvent {
    pub pid: u64,
    pub cmd: MembarrierCmd,
    pub cpus_targeted: u32,
    pub duration_ns: u64,
    pub timestamp: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct MembarrierAppStats {
    pub total_processes: u32,
    pub total_barriers: u64,
    pub total_ipis: u64,
    pub expedited_count: u64,
    pub sync_core_count: u64,
    pub avg_duration_ns: u64,
}

/// Main membarrier app
pub struct AppMembarrier {
    registrations: BTreeMap<u64, MembarrierReg>,
    events: Vec<BarrierEvent>,
    max_events: usize,
}

impl AppMembarrier {
    pub fn new() -> Self { Self { registrations: BTreeMap::new(), events: Vec::new(), max_events: 4096 } }

    pub fn register(&mut self, pid: u64, cmd: MembarrierCmd) {
        let reg = self.registrations.entry(pid).or_insert_with(|| MembarrierReg::new(pid));
        reg.register(cmd);
    }

    pub fn barrier(&mut self, pid: u64, cmd: MembarrierCmd, cpus: u32, duration: u64, now: u64) {
        if let Some(reg) = self.registrations.get_mut(&pid) {
            reg.total_barriers += 1;
            reg.total_ipi_sent += cpus as u64;
        }
        if self.events.len() >= self.max_events { self.events.drain(..self.max_events / 2); }
        self.events.push(BarrierEvent { pid, cmd, cpus_targeted: cpus, duration_ns: duration, timestamp: now });
    }

    pub fn stats(&self) -> MembarrierAppStats {
        let barriers: u64 = self.registrations.values().map(|r| r.total_barriers).sum();
        let ipis: u64 = self.registrations.values().map(|r| r.total_ipi_sent).sum();
        let exp = self.events.iter().filter(|e| matches!(e.cmd, MembarrierCmd::GlobalExpedited | MembarrierCmd::PrivateExpedited)).count() as u64;
        let sc = self.events.iter().filter(|e| matches!(e.cmd, MembarrierCmd::PrivateExpeditedSyncCore)).count() as u64;
        let durs: Vec<u64> = self.events.iter().map(|e| e.duration_ns).collect();
        let avg = if durs.is_empty() { 0 } else { durs.iter().sum::<u64>() / durs.len() as u64 };
        MembarrierAppStats { total_processes: self.registrations.len() as u32, total_barriers: barriers, total_ipis: ipis, expedited_count: exp, sync_core_count: sc, avg_duration_ns: avg }
    }
}
