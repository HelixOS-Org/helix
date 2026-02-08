//! # Coop Write-Ahead Log
//!
//! WAL for cooperative state persistence:
//! - Sequential log segments with rotation
//! - Checkpoint-based recovery points
//! - Log entry serialization with checksums
//! - Truncation after checkpoint
//! - Segment size limits and rotation
//! - Replay support for crash recovery

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// WAL entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalEntryType {
    Insert,
    Update,
    Delete,
    Checkpoint,
    BeginTx,
    CommitTx,
    AbortTx,
    CompensatingLog,
}

/// WAL entry
#[derive(Debug, Clone)]
pub struct WalEntry {
    pub lsn: u64,
    pub entry_type: WalEntryType,
    pub tx_id: Option<u64>,
    pub table_id: u32,
    pub key: String,
    pub data_size: usize,
    pub prev_lsn: Option<u64>,
    pub checksum: u64,
    pub timestamp_ns: u64,
}

impl WalEntry {
    pub fn new(
        lsn: u64, entry_type: WalEntryType, tx_id: Option<u64>,
        table_id: u32, key: String, data_size: usize, prev_lsn: Option<u64>, ts: u64,
    ) -> Self {
        let checksum = Self::compute_checksum(lsn, entry_type as u8, table_id, &key, data_size);
        Self { lsn, entry_type, tx_id, table_id, key, data_size, prev_lsn, checksum, timestamp_ns: ts }
    }

    fn compute_checksum(lsn: u64, etype: u8, table_id: u32, key: &str, data_size: usize) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in lsn.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        hash ^= etype as u64; hash = hash.wrapping_mul(0x100000001b3);
        for b in table_id.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        for b in key.bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        for b in (data_size as u64).to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        hash
    }

    pub fn verify(&self) -> bool {
        self.checksum == Self::compute_checksum(self.lsn, self.entry_type as u8, self.table_id, &self.key, self.data_size)
    }

    pub fn is_tx_boundary(&self) -> bool {
        matches!(self.entry_type, WalEntryType::BeginTx | WalEntryType::CommitTx | WalEntryType::AbortTx)
    }
}

/// WAL segment
#[derive(Debug, Clone)]
pub struct WalSegment {
    pub segment_id: u64,
    pub first_lsn: u64,
    pub last_lsn: u64,
    pub entry_count: usize,
    pub byte_size: usize,
    pub max_byte_size: usize,
    pub created_ts: u64,
    pub sealed: bool,
    entries: Vec<WalEntry>,
}

impl WalSegment {
    pub fn new(id: u64, first_lsn: u64, max_size: usize, ts: u64) -> Self {
        Self {
            segment_id: id, first_lsn, last_lsn: first_lsn,
            entry_count: 0, byte_size: 0, max_byte_size: max_size,
            created_ts: ts, sealed: false, entries: Vec::new(),
        }
    }

    pub fn can_append(&self, entry_size: usize) -> bool {
        !self.sealed && self.byte_size + entry_size <= self.max_byte_size
    }

    pub fn append(&mut self, entry: WalEntry) {
        let size = entry.data_size + 128; // overhead
        self.last_lsn = entry.lsn;
        self.entry_count += 1;
        self.byte_size += size;
        self.entries.push(entry);
    }

    pub fn seal(&mut self) { self.sealed = true; }

    pub fn entries(&self) -> &[WalEntry] { &self.entries }

    pub fn entries_from(&self, lsn: u64) -> Vec<&WalEntry> {
        self.entries.iter().filter(|e| e.lsn >= lsn).collect()
    }

    pub fn fill_ratio(&self) -> f64 {
        if self.max_byte_size == 0 { return 0.0; }
        self.byte_size as f64 / self.max_byte_size as f64
    }
}

/// Checkpoint record
#[derive(Debug, Clone)]
pub struct WalCheckpoint {
    pub checkpoint_id: u64,
    pub lsn: u64,
    pub active_txs: Vec<u64>,
    pub timestamp_ns: u64,
    pub dirty_pages: usize,
}

/// Transaction state tracking
#[derive(Debug, Clone)]
pub struct ActiveTransaction {
    pub tx_id: u64,
    pub begin_lsn: u64,
    pub last_lsn: u64,
    pub entry_count: u32,
    pub bytes_written: usize,
}

/// WAL stats
#[derive(Debug, Clone, Default)]
pub struct WalStats {
    pub total_segments: usize,
    pub sealed_segments: usize,
    pub total_entries: u64,
    pub total_bytes: u64,
    pub current_lsn: u64,
    pub active_transactions: usize,
    pub checkpoints: usize,
    pub last_checkpoint_lsn: u64,
    pub checksum_failures: u64,
    pub truncated_segments: u64,
}

/// Cooperative write-ahead log
pub struct CoopWriteAheadLog {
    segments: BTreeMap<u64, WalSegment>,
    checkpoints: Vec<WalCheckpoint>,
    active_txs: BTreeMap<u64, ActiveTransaction>,
    current_lsn: u64,
    current_segment_id: u64,
    segment_max_size: usize,
    checksum_failures: u64,
    truncated_segments: u64,
    stats: WalStats,
}

impl CoopWriteAheadLog {
    pub fn new(segment_max_size: usize) -> Self {
        Self {
            segments: BTreeMap::new(), checkpoints: Vec::new(),
            active_txs: BTreeMap::new(), current_lsn: 0,
            current_segment_id: 0, segment_max_size,
            checksum_failures: 0, truncated_segments: 0,
            stats: WalStats::default(),
        }
    }

    fn ensure_segment(&mut self, ts: u64) {
        let needs_new = self.segments.is_empty() || {
            let seg = self.segments.get(&self.current_segment_id);
            seg.map(|s| s.sealed || !s.can_append(256)).unwrap_or(true)
        };
        if needs_new {
            // Seal current
            if let Some(seg) = self.segments.get_mut(&self.current_segment_id) { seg.seal(); }
            self.current_segment_id += 1;
            let seg = WalSegment::new(self.current_segment_id, self.current_lsn + 1, self.segment_max_size, ts);
            self.segments.insert(self.current_segment_id, seg);
        }
    }

    pub fn append(&mut self, entry_type: WalEntryType, tx_id: Option<u64>, table_id: u32, key: String, data_size: usize, ts: u64) -> u64 {
        self.current_lsn += 1;
        let lsn = self.current_lsn;
        let prev_lsn = tx_id.and_then(|tid| self.active_txs.get(&tid).map(|t| t.last_lsn));
        let entry = WalEntry::new(lsn, entry_type, tx_id, table_id, key, data_size, prev_lsn, ts);

        self.ensure_segment(ts);
        if let Some(seg) = self.segments.get_mut(&self.current_segment_id) {
            seg.append(entry);
        }

        // Track transaction
        if let Some(tid) = tx_id {
            match entry_type {
                WalEntryType::BeginTx => {
                    self.active_txs.insert(tid, ActiveTransaction {
                        tx_id: tid, begin_lsn: lsn, last_lsn: lsn, entry_count: 1, bytes_written: data_size,
                    });
                }
                WalEntryType::CommitTx | WalEntryType::AbortTx => {
                    self.active_txs.remove(&tid);
                }
                _ => {
                    if let Some(tx) = self.active_txs.get_mut(&tid) {
                        tx.last_lsn = lsn;
                        tx.entry_count += 1;
                        tx.bytes_written += data_size;
                    }
                }
            }
        }

        lsn
    }

    pub fn checkpoint(&mut self, ts: u64) -> u64 {
        let cp_id = self.checkpoints.len() as u64 + 1;
        let active: Vec<u64> = self.active_txs.keys().copied().collect();
        self.checkpoints.push(WalCheckpoint {
            checkpoint_id: cp_id, lsn: self.current_lsn,
            active_txs: active, timestamp_ns: ts, dirty_pages: 0,
        });
        cp_id
    }

    pub fn truncate_before(&mut self, lsn: u64) {
        let to_remove: Vec<u64> = self.segments.iter()
            .filter(|(_, s)| s.last_lsn < lsn && s.sealed)
            .map(|(&id, _)| id)
            .collect();
        for id in to_remove {
            self.segments.remove(&id);
            self.truncated_segments += 1;
        }
    }

    pub fn replay_from(&self, lsn: u64) -> Vec<&WalEntry> {
        let mut entries: Vec<&WalEntry> = Vec::new();
        for seg in self.segments.values() {
            if seg.last_lsn < lsn { continue; }
            entries.extend(seg.entries_from(lsn));
        }
        entries.sort_by_key(|e| e.lsn);
        entries
    }

    pub fn verify_integrity(&mut self) -> usize {
        let mut failures = 0;
        for seg in self.segments.values() {
            for entry in seg.entries() {
                if !entry.verify() { failures += 1; }
            }
        }
        self.checksum_failures += failures as u64;
        failures
    }

    pub fn last_checkpoint_lsn(&self) -> u64 {
        self.checkpoints.last().map(|c| c.lsn).unwrap_or(0)
    }

    pub fn recompute(&mut self) {
        self.stats.total_segments = self.segments.len();
        self.stats.sealed_segments = self.segments.values().filter(|s| s.sealed).count();
        self.stats.total_entries = self.segments.values().map(|s| s.entry_count as u64).sum();
        self.stats.total_bytes = self.segments.values().map(|s| s.byte_size as u64).sum();
        self.stats.current_lsn = self.current_lsn;
        self.stats.active_transactions = self.active_txs.len();
        self.stats.checkpoints = self.checkpoints.len();
        self.stats.last_checkpoint_lsn = self.last_checkpoint_lsn();
        self.stats.checksum_failures = self.checksum_failures;
        self.stats.truncated_segments = self.truncated_segments;
    }

    pub fn stats(&self) -> &WalStats { &self.stats }
}
