// SPDX-License-Identifier: GPL-2.0
//! Bridge acct_bridge — process accounting bridge.

extern crate alloc;

use alloc::collections::BTreeMap;

/// Accounting record type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcctRecordType {
    Fork,
    Exec,
    Exit,
    Core,
}

/// Accounting entry
#[derive(Debug)]
pub struct AcctEntry {
    pub pid: u64,
    pub uid: u32,
    pub gid: u32,
    pub record_type: AcctRecordType,
    pub command_hash: u64,
    pub utime_ticks: u64,
    pub stime_ticks: u64,
    pub elapsed_ticks: u64,
    pub mem_peak_kb: u64,
    pub io_read_bytes: u64,
    pub io_write_bytes: u64,
    pub exit_code: i32,
    pub timestamp: u64,
}

impl AcctEntry {
    pub fn new(pid: u64, uid: u32, rt: AcctRecordType, now: u64) -> Self {
        Self { pid, uid, gid: 0, record_type: rt, command_hash: 0, utime_ticks: 0, stime_ticks: 0, elapsed_ticks: 0, mem_peak_kb: 0, io_read_bytes: 0, io_write_bytes: 0, exit_code: 0, timestamp: now }
    }

    #[inline(always)]
    pub fn total_cpu(&self) -> u64 { self.utime_ticks + self.stime_ticks }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct AcctBridgeStats {
    pub total_records: u32,
    pub fork_records: u32,
    pub exec_records: u32,
    pub exit_records: u32,
    pub total_cpu_ticks: u64,
    pub total_io_bytes: u64,
}

/// Main acct bridge
#[repr(align(64))]
pub struct BridgeAcct {
    records: BTreeMap<u64, AcctEntry>,
    next_id: u64,
    enabled: bool,
}

impl BridgeAcct {
    pub fn new() -> Self { Self { records: BTreeMap::new(), next_id: 1, enabled: true } }

    #[inline]
    pub fn record(&mut self, pid: u64, uid: u32, rt: AcctRecordType, now: u64) -> u64 {
        if !self.enabled { return 0; }
        let id = self.next_id; self.next_id += 1;
        self.records.insert(id, AcctEntry::new(pid, uid, rt, now));
        id
    }

    #[inline]
    pub fn stats(&self) -> AcctBridgeStats {
        let forks = self.records.values().filter(|r| r.record_type == AcctRecordType::Fork).count() as u32;
        let execs = self.records.values().filter(|r| r.record_type == AcctRecordType::Exec).count() as u32;
        let exits = self.records.values().filter(|r| r.record_type == AcctRecordType::Exit).count() as u32;
        let cpu: u64 = self.records.values().map(|r| r.total_cpu()).sum();
        let io: u64 = self.records.values().map(|r| r.io_read_bytes + r.io_write_bytes).sum();
        AcctBridgeStats { total_records: self.records.len() as u32, fork_records: forks, exec_records: execs, exit_records: exits, total_cpu_ticks: cpu, total_io_bytes: io }
    }
}
