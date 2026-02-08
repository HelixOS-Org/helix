// SPDX-License-Identifier: GPL-2.0
//! Bridge membarrier_bridge — membarrier syscall bridge.

extern crate alloc;

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

/// Membarrier registration
#[derive(Debug)]
pub struct MembarrierRegistration {
    pub pid: u64,
    pub registered_cmds: u32,
    pub timestamp: u64,
}

/// Membarrier invocation
#[derive(Debug)]
pub struct MembarrierInvocation {
    pub pid: u64,
    pub cmd: MembarrierCmd,
    pub flags: u32,
    pub cpu_id: i32,
    pub timestamp: u64,
    pub duration_ns: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct MembarrierBridgeStats {
    pub total_invocations: u64,
    pub total_registrations: u32,
    pub global_barriers: u64,
    pub private_barriers: u64,
    pub avg_duration_ns: u64,
}

/// Main bridge membarrier
pub struct BridgeMembarrier {
    registrations: Vec<MembarrierRegistration>,
    invocations: Vec<MembarrierInvocation>,
    supported_cmds: u32,
}

impl BridgeMembarrier {
    pub fn new() -> Self {
        Self { registrations: Vec::new(), invocations: Vec::new(), supported_cmds: 0x1FF }
    }

    pub fn register(&mut self, pid: u64, cmds: u32, now: u64) {
        self.registrations.push(MembarrierRegistration { pid, registered_cmds: cmds, timestamp: now });
    }

    pub fn invoke(&mut self, pid: u64, cmd: MembarrierCmd, flags: u32, cpu: i32, now: u64, dur: u64) {
        self.invocations.push(MembarrierInvocation { pid, cmd, flags, cpu_id: cpu, timestamp: now, duration_ns: dur });
    }

    pub fn stats(&self) -> MembarrierBridgeStats {
        let global = self.invocations.iter().filter(|i| matches!(i.cmd, MembarrierCmd::Global | MembarrierCmd::GlobalExpedited)).count() as u64;
        let private = self.invocations.iter().filter(|i| matches!(i.cmd, MembarrierCmd::Private | MembarrierCmd::PrivateExpedited | MembarrierCmd::PrivateExpeditedSyncCore | MembarrierCmd::PrivateExpeditedRseq)).count() as u64;
        let durs: Vec<u64> = self.invocations.iter().map(|i| i.duration_ns).collect();
        let avg = if durs.is_empty() { 0 } else { durs.iter().sum::<u64>() / durs.len() as u64 };
        MembarrierBridgeStats { total_invocations: self.invocations.len() as u64, total_registrations: self.registrations.len() as u32, global_barriers: global, private_barriers: private, avg_duration_ns: avg }
    }
}
