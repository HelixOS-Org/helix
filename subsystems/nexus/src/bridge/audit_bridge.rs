// SPDX-License-Identifier: GPL-2.0
//! Bridge audit_bridge — kernel audit subsystem bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Audit message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditMsgType {
    Syscall,
    FileAccess,
    NetworkOp,
    UserAuth,
    ConfigChange,
    ProcessOp,
    IpcOp,
    Anomaly,
}

/// Audit rule field
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditField {
    Pid,
    Uid,
    Gid,
    Syscall,
    Success,
    Arch,
    Path,
    Perm,
}

/// Audit rule
#[derive(Debug)]
pub struct AuditRule {
    pub id: u64,
    pub field: AuditField,
    pub value: u64,
    pub enabled: bool,
    pub hit_count: u64,
}

/// Audit record
#[derive(Debug)]
pub struct AuditRecord {
    pub seq: u64,
    pub msg_type: AuditMsgType,
    pub pid: u64,
    pub uid: u32,
    pub syscall_nr: u32,
    pub success: bool,
    pub timestamp: u64,
    pub data_hash: u64,
}

/// Stats
#[derive(Debug, Clone)]
pub struct AuditBridgeStats {
    pub total_records: u64,
    pub total_rules: u32,
    pub enabled_rules: u32,
    pub total_hits: u64,
    pub dropped: u64,
}

/// Main audit bridge
pub struct BridgeAudit {
    rules: Vec<AuditRule>,
    records: BTreeMap<u64, AuditRecord>,
    next_seq: u64,
    backlog_limit: u32,
    dropped: u64,
}

impl BridgeAudit {
    pub fn new(backlog: u32) -> Self { Self { rules: Vec::new(), records: BTreeMap::new(), next_seq: 1, backlog_limit: backlog, dropped: 0 } }

    pub fn add_rule(&mut self, field: AuditField, value: u64) -> u64 {
        let id = self.rules.len() as u64 + 1;
        self.rules.push(AuditRule { id, field, value, enabled: true, hit_count: 0 });
        id
    }

    pub fn log(&mut self, msg_type: AuditMsgType, pid: u64, uid: u32, syscall: u32, success: bool, now: u64) -> u64 {
        if self.records.len() as u32 >= self.backlog_limit { self.dropped += 1; return 0; }
        let seq = self.next_seq; self.next_seq += 1;
        self.records.insert(seq, AuditRecord { seq, msg_type, pid, uid, syscall_nr: syscall, success, timestamp: now, data_hash: 0 });
        seq
    }

    pub fn stats(&self) -> AuditBridgeStats {
        let enabled = self.rules.iter().filter(|r| r.enabled).count() as u32;
        let hits: u64 = self.rules.iter().map(|r| r.hit_count).sum();
        AuditBridgeStats { total_records: self.records.len() as u64, total_rules: self.rules.len() as u32, enabled_rules: enabled, total_hits: hits, dropped: self.dropped }
    }
}

// ============================================================================
// Merged from audit_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditV2MsgType {
    Syscall,
    Path,
    Ipc,
    Socketcall,
    Config,
    UserAcct,
    UserLogin,
    UserAuth,
    Anom,
    Integrity,
}

/// Audit v2 operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditV2Op {
    AddRule,
    DeleteRule,
    ListRules,
    GetStatus,
    SetStatus,
    EmitEvent,
    Login,
}

/// Audit v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditV2Result {
    Success,
    PermissionDenied,
    InvalidRule,
    BufferFull,
    Error,
}

/// Audit v2 record
#[derive(Debug, Clone)]
pub struct AuditV2Record {
    pub op: AuditV2Op,
    pub msg_type: AuditV2MsgType,
    pub result: AuditV2Result,
    pub serial: u64,
    pub pid: u32,
    pub uid: u32,
    pub syscall_nr: u32,
}

impl AuditV2Record {
    pub fn new(op: AuditV2Op, msg_type: AuditV2MsgType) -> Self {
        Self { op, msg_type, result: AuditV2Result::Success, serial: 0, pid: 0, uid: 0, syscall_nr: 0 }
    }
}

/// Audit v2 bridge stats
#[derive(Debug, Clone)]
pub struct AuditV2BridgeStats {
    pub total_ops: u64,
    pub events_emitted: u64,
    pub rules_added: u64,
    pub buffer_overflows: u64,
    pub errors: u64,
}

/// Main bridge audit v2
#[derive(Debug)]
pub struct BridgeAuditV2 {
    pub stats: AuditV2BridgeStats,
}

impl BridgeAuditV2 {
    pub fn new() -> Self {
        Self { stats: AuditV2BridgeStats { total_ops: 0, events_emitted: 0, rules_added: 0, buffer_overflows: 0, errors: 0 } }
    }

    pub fn record(&mut self, rec: &AuditV2Record) {
        self.stats.total_ops += 1;
        match rec.op {
            AuditV2Op::EmitEvent => self.stats.events_emitted += 1,
            AuditV2Op::AddRule => self.stats.rules_added += 1,
            _ => {}
        }
        if rec.result == AuditV2Result::BufferFull { self.stats.buffer_overflows += 1; }
        if rec.result == AuditV2Result::Error { self.stats.errors += 1; }
    }
}
