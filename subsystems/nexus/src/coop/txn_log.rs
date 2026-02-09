//! # Coop Transaction Log
//!
//! Write-ahead transaction log for cooperative state machines:
//! - Append-only log with LSN (log sequence number)
//! - Checkpoint/truncation support
//! - Group commit for batching
//! - Log segment management
//! - Recovery scan (redo/undo)
//! - Concurrent reader/writer with generation tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Log record type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogRecordType {
    Data,
    Commit,
    Abort,
    Checkpoint,
    CompensationLogRecord,
    BeginTxn,
    EndTxn,
    Prepare,
    Savepoint,
}

/// Log record
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub lsn: u64,
    pub txn_id: u64,
    pub record_type: LogRecordType,
    pub prev_lsn: u64,
    pub data_hash: u64,
    pub data_size: u32,
    pub ts: u64,
    pub checksum: u64,
}

impl LogRecord {
    pub fn new(lsn: u64, txn_id: u64, rtype: LogRecordType, prev_lsn: u64, data_hash: u64, data_size: u32, ts: u64) -> Self {
        let mut ck: u64 = 0xcbf29ce484222325;
        ck ^= lsn; ck = ck.wrapping_mul(0x100000001b3);
        ck ^= txn_id; ck = ck.wrapping_mul(0x100000001b3);
        ck ^= data_hash; ck = ck.wrapping_mul(0x100000001b3);
        ck ^= data_size as u64; ck = ck.wrapping_mul(0x100000001b3);
        Self { lsn, txn_id, record_type: rtype, prev_lsn, data_hash, data_size, ts, checksum: ck }
    }

    #[inline]
    pub fn verify(&self) -> bool {
        let mut ck: u64 = 0xcbf29ce484222325;
        ck ^= self.lsn; ck = ck.wrapping_mul(0x100000001b3);
        ck ^= self.txn_id; ck = ck.wrapping_mul(0x100000001b3);
        ck ^= self.data_hash; ck = ck.wrapping_mul(0x100000001b3);
        ck ^= self.data_size as u64; ck = ck.wrapping_mul(0x100000001b3);
        ck == self.checksum
    }
}

/// Log segment
#[derive(Debug, Clone)]
pub struct LogSegment {
    pub id: u64,
    pub start_lsn: u64,
    pub end_lsn: u64,
    pub records: Vec<LogRecord>,
    pub size_bytes: u64,
    pub sealed: bool,
    pub create_ts: u64,
}

impl LogSegment {
    pub fn new(id: u64, start_lsn: u64, ts: u64) -> Self {
        Self { id, start_lsn, end_lsn: start_lsn, records: Vec::new(), size_bytes: 0, sealed: false, create_ts: ts }
    }

    #[inline]
    pub fn append(&mut self, record: LogRecord) {
        self.size_bytes += record.data_size as u64 + 64;
        self.end_lsn = record.lsn;
        self.records.push(record);
    }

    #[inline(always)]
    pub fn seal(&mut self) { self.sealed = true; }
    #[inline(always)]
    pub fn record_count(&self) -> usize { self.records.len() }
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxnState {
    Active,
    Preparing,
    Committed,
    Aborted,
    Recovered,
}

/// Transaction metadata
#[derive(Debug, Clone)]
pub struct TxnMeta {
    pub id: u64,
    pub state: TxnState,
    pub first_lsn: u64,
    pub last_lsn: u64,
    pub record_count: u32,
    pub begin_ts: u64,
    pub end_ts: u64,
}

impl TxnMeta {
    pub fn new(id: u64, first_lsn: u64, ts: u64) -> Self {
        Self { id, state: TxnState::Active, first_lsn, last_lsn: first_lsn, record_count: 0, begin_ts: ts, end_ts: 0 }
    }
}

/// Checkpoint descriptor
#[derive(Debug, Clone)]
pub struct TxnCheckpoint {
    pub lsn: u64,
    pub ts: u64,
    pub active_txns: Vec<u64>,
    pub dirty_pages: u32,
    pub checksum: u64,
}

impl TxnCheckpoint {
    pub fn new(lsn: u64, ts: u64, active: Vec<u64>) -> Self {
        let mut ck: u64 = 0xcbf29ce484222325;
        ck ^= lsn; ck = ck.wrapping_mul(0x100000001b3);
        for &t in &active { ck ^= t; ck = ck.wrapping_mul(0x100000001b3); }
        Self { lsn, ts, active_txns: active, dirty_pages: 0, checksum: ck }
    }
}

/// Group commit batch
#[derive(Debug, Clone)]
pub struct GroupCommitBatch {
    pub txns: Vec<u64>,
    pub flush_lsn: u64,
    pub ts: u64,
    pub size_bytes: u64,
}

/// Recovery action
#[derive(Debug, Clone)]
pub struct RecoveryAction {
    pub txn_id: u64,
    pub action: RecoveryActionType,
    pub lsn: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryActionType {
    Redo,
    Undo,
    Skip,
}

/// Transaction log stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct TxnLogStats {
    pub total_records: u64,
    pub total_bytes: u64,
    pub segments: usize,
    pub active_txns: usize,
    pub committed_txns: u64,
    pub aborted_txns: u64,
    pub checkpoints: usize,
    pub current_lsn: u64,
    pub group_commits: u64,
}

/// Cooperative transaction log
pub struct CoopTxnLog {
    segments: Vec<LogSegment>,
    txns: BTreeMap<u64, TxnMeta>,
    checkpoints: Vec<TxnCheckpoint>,
    stats: TxnLogStats,
    current_lsn: u64,
    next_segment_id: u64,
    max_segment_size: u64,
    group_batch: Vec<u64>,
    group_batch_limit: usize,
}

impl CoopTxnLog {
    pub fn new(max_segment_size: u64, batch_limit: usize) -> Self {
        Self {
            segments: Vec::new(), txns: BTreeMap::new(),
            checkpoints: Vec::new(), stats: TxnLogStats::default(),
            current_lsn: 1, next_segment_id: 1,
            max_segment_size, group_batch: Vec::new(),
            group_batch_limit: batch_limit,
        }
    }

    fn active_segment(&mut self, ts: u64) -> &mut LogSegment {
        if self.segments.is_empty() || self.segments.last().map(|s| s.sealed).unwrap_or(true) {
            let id = self.next_segment_id; self.next_segment_id += 1;
            self.segments.push(LogSegment::new(id, self.current_lsn, ts));
        }
        let last = self.segments.last_mut().unwrap();
        if last.size_bytes >= self.max_segment_size {
            last.seal();
            let id = self.next_segment_id; self.next_segment_id += 1;
            self.segments.push(LogSegment::new(id, self.current_lsn, ts));
        }
        self.segments.last_mut().unwrap()
    }

    #[inline]
    pub fn begin_txn(&mut self, txn_id: u64, ts: u64) -> u64 {
        let lsn = self.current_lsn; self.current_lsn += 1;
        let record = LogRecord::new(lsn, txn_id, LogRecordType::BeginTxn, 0, 0, 0, ts);
        self.active_segment(ts).append(record);
        self.txns.insert(txn_id, TxnMeta::new(txn_id, lsn, ts));
        self.stats.total_records += 1;
        lsn
    }

    #[inline]
    pub fn write(&mut self, txn_id: u64, data_hash: u64, data_size: u32, ts: u64) -> u64 {
        let lsn = self.current_lsn; self.current_lsn += 1;
        let prev = self.txns.get(&txn_id).map(|t| t.last_lsn).unwrap_or(0);
        let record = LogRecord::new(lsn, txn_id, LogRecordType::Data, prev, data_hash, data_size, ts);
        self.active_segment(ts).append(record);
        if let Some(t) = self.txns.get_mut(&txn_id) { t.last_lsn = lsn; t.record_count += 1; }
        self.stats.total_records += 1;
        self.stats.total_bytes += data_size as u64;
        lsn
    }

    #[inline]
    pub fn commit(&mut self, txn_id: u64, ts: u64) -> u64 {
        let lsn = self.current_lsn; self.current_lsn += 1;
        let prev = self.txns.get(&txn_id).map(|t| t.last_lsn).unwrap_or(0);
        let record = LogRecord::new(lsn, txn_id, LogRecordType::Commit, prev, 0, 0, ts);
        self.active_segment(ts).append(record);
        if let Some(t) = self.txns.get_mut(&txn_id) { t.state = TxnState::Committed; t.last_lsn = lsn; t.end_ts = ts; }
        self.group_batch.push(txn_id);
        self.stats.total_records += 1;
        self.stats.committed_txns += 1;
        lsn
    }

    #[inline]
    pub fn abort(&mut self, txn_id: u64, ts: u64) -> u64 {
        let lsn = self.current_lsn; self.current_lsn += 1;
        let prev = self.txns.get(&txn_id).map(|t| t.last_lsn).unwrap_or(0);
        let record = LogRecord::new(lsn, txn_id, LogRecordType::Abort, prev, 0, 0, ts);
        self.active_segment(ts).append(record);
        if let Some(t) = self.txns.get_mut(&txn_id) { t.state = TxnState::Aborted; t.last_lsn = lsn; t.end_ts = ts; }
        self.stats.total_records += 1;
        self.stats.aborted_txns += 1;
        lsn
    }

    #[inline]
    pub fn flush_group(&mut self, ts: u64) -> Option<GroupCommitBatch> {
        if self.group_batch.is_empty() { return None; }
        let txns = core::mem::replace(&mut self.group_batch, Vec::new());
        let batch = GroupCommitBatch {
            txns: txns.clone(), flush_lsn: self.current_lsn - 1, ts,
            size_bytes: txns.len() as u64 * 64,
        };
        self.stats.group_commits += 1;
        Some(batch)
    }

    #[inline]
    pub fn checkpoint(&mut self, ts: u64) -> u64 {
        let lsn = self.current_lsn; self.current_lsn += 1;
        let active: Vec<u64> = self.txns.values().filter(|t| t.state == TxnState::Active).map(|t| t.id).collect();
        let record = LogRecord::new(lsn, 0, LogRecordType::Checkpoint, 0, 0, 0, ts);
        self.active_segment(ts).append(record);
        let cp = TxnCheckpoint::new(lsn, ts, active);
        self.checkpoints.push(cp);
        self.stats.total_records += 1;
        lsn
    }

    #[inline(always)]
    pub fn truncate_before(&mut self, lsn: u64) {
        self.segments.retain(|s| s.end_lsn >= lsn);
    }

    pub fn recover(&self) -> Vec<RecoveryAction> {
        let mut actions = Vec::new();
        let last_cp_lsn = self.checkpoints.last().map(|c| c.lsn).unwrap_or(0);
        for seg in &self.segments {
            for rec in &seg.records {
                if rec.lsn < last_cp_lsn { continue; }
                let txn_state = self.txns.get(&rec.txn_id).map(|t| t.state);
                let action = match txn_state {
                    Some(TxnState::Committed) => RecoveryActionType::Redo,
                    Some(TxnState::Aborted) => RecoveryActionType::Undo,
                    Some(TxnState::Active) => RecoveryActionType::Undo,
                    _ => RecoveryActionType::Skip,
                };
                actions.push(RecoveryAction { txn_id: rec.txn_id, action, lsn: rec.lsn });
            }
        }
        actions
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.segments = self.segments.len();
        self.stats.active_txns = self.txns.values().filter(|t| t.state == TxnState::Active).count();
        self.stats.checkpoints = self.checkpoints.len();
        self.stats.current_lsn = self.current_lsn;
    }

    #[inline(always)]
    pub fn txn(&self, id: u64) -> Option<&TxnMeta> { self.txns.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &TxnLogStats { &self.stats }
    #[inline(always)]
    pub fn lsn(&self) -> u64 { self.current_lsn }
}
