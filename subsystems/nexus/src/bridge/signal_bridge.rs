// SPDX-License-Identifier: GPL-2.0
//! Bridge signal — kill/tgkill/tkill signal delivery bridge

extern crate alloc;
use alloc::vec::Vec;

/// Signal number
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BridgeSignal {
    SigHup, SigInt, SigQuit, SigIll, SigTrap, SigAbrt, SigBus, SigFpe,
    SigKill, SigUsr1, SigSegv, SigUsr2, SigPipe, SigAlrm, SigTerm,
    SigChld, SigCont, SigStop, SigTstp, SigTtin, SigTtou, SigUrg,
    SigRt(u8),
}

/// Signal delivery method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalMethod {
    Kill,
    Tgkill,
    Tkill,
    RtSigqueueinfo,
    Raise,
}

/// Signal delivery result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalResult {
    Delivered,
    Queued,
    Ignored,
    PermissionDenied,
    NoProcess,
    Error,
}

/// Signal bridge record
#[derive(Debug, Clone)]
pub struct SignalBridgeRecord {
    pub signal: BridgeSignal,
    pub method: SignalMethod,
    pub result: SignalResult,
    pub sender_pid: u32,
    pub target_pid: u32,
}

impl SignalBridgeRecord {
    pub fn new(signal: BridgeSignal, method: SignalMethod) -> Self {
        Self { signal, method, result: SignalResult::Delivered, sender_pid: 0, target_pid: 0 }
    }
}

/// Signal bridge stats
#[derive(Debug, Clone)]
pub struct SignalBridgeStats {
    pub total_ops: u64,
    pub delivered: u64,
    pub denied: u64,
    pub queued: u64,
}

/// Main bridge signal
#[derive(Debug)]
pub struct BridgeSignalMgr {
    pub stats: SignalBridgeStats,
}

impl BridgeSignalMgr {
    pub fn new() -> Self {
        Self { stats: SignalBridgeStats { total_ops: 0, delivered: 0, denied: 0, queued: 0 } }
    }

    pub fn record(&mut self, rec: &SignalBridgeRecord) {
        self.stats.total_ops += 1;
        match rec.result {
            SignalResult::Delivered => self.stats.delivered += 1,
            SignalResult::Queued => self.stats.queued += 1,
            SignalResult::PermissionDenied => self.stats.denied += 1,
            _ => {}
        }
    }
}
