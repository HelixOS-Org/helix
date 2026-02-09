// SPDX-License-Identifier: GPL-2.0
//! Bridge msgqueue — System V message queue bridge

extern crate alloc;

/// Message queue operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgqueueOp {
    Msgget,
    Msgsnd,
    Msgrcv,
    Msgctl,
}

/// Msgctl command
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgctlCmd {
    IpcStat,
    IpcSet,
    IpcRmid,
    IpcInfo,
    MsgInfo,
    MsgStat,
}

/// Msgqueue result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgqueueResult {
    Success,
    PermissionDenied,
    NoMsg,
    QueueFull,
    InvalidId,
    Error,
}

/// Msgqueue record
#[derive(Debug, Clone)]
pub struct MsgqueueRecord {
    pub op: MsgqueueOp,
    pub result: MsgqueueResult,
    pub msqid: i32,
    pub msg_type: i64,
    pub msg_size: u32,
    pub key: u32,
}

impl MsgqueueRecord {
    pub fn new(op: MsgqueueOp) -> Self {
        Self { op, result: MsgqueueResult::Success, msqid: -1, msg_type: 0, msg_size: 0, key: 0 }
    }
}

/// Msgqueue bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MsgqueueBridgeStats {
    pub total_ops: u64,
    pub sends: u64,
    pub receives: u64,
    pub queues_created: u64,
    pub errors: u64,
}

/// Main bridge msgqueue
#[derive(Debug)]
pub struct BridgeMsgqueue {
    pub stats: MsgqueueBridgeStats,
}

impl BridgeMsgqueue {
    pub fn new() -> Self {
        Self { stats: MsgqueueBridgeStats { total_ops: 0, sends: 0, receives: 0, queues_created: 0, errors: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &MsgqueueRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            MsgqueueOp::Msgsnd => self.stats.sends += 1,
            MsgqueueOp::Msgrcv => self.stats.receives += 1,
            MsgqueueOp::Msgget => self.stats.queues_created += 1,
            _ => {}
        }
        if rec.result != MsgqueueResult::Success { self.stats.errors += 1; }
    }
}
