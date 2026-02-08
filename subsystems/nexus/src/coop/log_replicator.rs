//! # Coop Log Replicator
//!
//! Log-based state replication for cooperative subsystems:
//! - Append-only log entries with sequence numbers
//! - Follower catch-up via snapshot + replay
//! - Log compaction and garbage collection
//! - Replication lag tracking
//! - Multi-follower fan-out
//! - Entry checksum verification

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Log entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEntryKind {
    Data,
    Config,
    Snapshot,
    Barrier,
    Noop,
    Checkpoint,
}

/// A single log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub sequence: u64,
    pub term: u64,
    pub kind: LogEntryKind,
    pub key: String,
    pub data_len: usize,
    pub checksum: u64,
    pub timestamp_ns: u64,
}

impl LogEntry {
    pub fn new(seq: u64, term: u64, kind: LogEntryKind, key: String, data_len: usize, ts: u64) -> Self {
        let checksum = Self::compute_checksum(seq, term, &key, data_len);
        Self { sequence: seq, term, kind, key, data_len, checksum, timestamp_ns: ts }
    }

    fn compute_checksum(seq: u64, term: u64, key: &str, data_len: usize) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in seq.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        for b in term.to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        for b in key.bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        for b in (data_len as u64).to_le_bytes() { hash ^= b as u64; hash = hash.wrapping_mul(0x100000001b3); }
        hash
    }

    pub fn verify_checksum(&self) -> bool {
        self.checksum == Self::compute_checksum(self.sequence, self.term, &self.key, self.data_len)
    }
}

/// Replication state of a follower
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowerState {
    Synced,
    Catching,
    Stale,
    Disconnected,
}

/// Per-follower replication tracking
#[derive(Debug, Clone)]
pub struct FollowerInfo {
    pub follower_id: u64,
    pub state: FollowerState,
    pub match_sequence: u64,
    pub next_sequence: u64,
    pub last_ack_ts: u64,
    pub entries_sent: u64,
    pub entries_acked: u64,
    pub bytes_sent: u64,
}

impl FollowerInfo {
    pub fn new(id: u64) -> Self {
        Self {
            follower_id: id, state: FollowerState::Disconnected,
            match_sequence: 0, next_sequence: 1, last_ack_ts: 0,
            entries_sent: 0, entries_acked: 0, bytes_sent: 0,
        }
    }

    pub fn replication_lag(&self, leader_seq: u64) -> u64 {
        leader_seq.saturating_sub(self.match_sequence)
    }

    pub fn ack_ratio(&self) -> f64 {
        if self.entries_sent == 0 { return 1.0; }
        self.entries_acked as f64 / self.entries_sent as f64
    }
}

/// Snapshot metadata
#[derive(Debug, Clone)]
pub struct SnapshotMeta {
    pub snapshot_id: u64,
    pub last_included_seq: u64,
    pub last_included_term: u64,
    pub size_bytes: usize,
    pub created_ts: u64,
    pub entry_count: u64,
}

/// Log compaction policy
#[derive(Debug, Clone)]
pub struct CompactionPolicy {
    pub max_log_entries: usize,
    pub max_log_bytes: usize,
    pub snapshot_interval_entries: u64,
    pub min_entries_to_keep: usize,
}

impl Default for CompactionPolicy {
    fn default() -> Self {
        Self {
            max_log_entries: 100_000, max_log_bytes: 64 * 1024 * 1024,
            snapshot_interval_entries: 10_000, min_entries_to_keep: 1000,
        }
    }
}

/// Log replicator stats
#[derive(Debug, Clone, Default)]
pub struct LogReplicatorStats {
    pub total_entries: usize,
    pub total_bytes: usize,
    pub current_sequence: u64,
    pub current_term: u64,
    pub follower_count: usize,
    pub synced_followers: usize,
    pub snapshot_count: usize,
    pub compactions_done: u64,
    pub checksum_failures: u64,
    pub avg_replication_lag: f64,
}

/// Cooperative log replicator
pub struct CoopLogReplicator {
    log: Vec<LogEntry>,
    followers: BTreeMap<u64, FollowerInfo>,
    snapshots: Vec<SnapshotMeta>,
    current_sequence: u64,
    current_term: u64,
    first_log_sequence: u64,
    total_bytes: usize,
    compaction_policy: CompactionPolicy,
    compactions_done: u64,
    checksum_failures: u64,
    stats: LogReplicatorStats,
}

impl CoopLogReplicator {
    pub fn new() -> Self {
        Self {
            log: Vec::new(), followers: BTreeMap::new(), snapshots: Vec::new(),
            current_sequence: 0, current_term: 1, first_log_sequence: 1,
            total_bytes: 0, compaction_policy: CompactionPolicy::default(),
            compactions_done: 0, checksum_failures: 0,
            stats: LogReplicatorStats::default(),
        }
    }

    pub fn append(&mut self, kind: LogEntryKind, key: String, data_len: usize, ts: u64) -> u64 {
        self.current_sequence += 1;
        let entry = LogEntry::new(self.current_sequence, self.current_term, kind, key, data_len, ts);
        self.total_bytes += data_len + 64; // overhead estimate
        self.log.push(entry);
        self.current_sequence
    }

    pub fn set_term(&mut self, term: u64) { self.current_term = term; }

    pub fn get_entries_from(&self, from_seq: u64, max: usize) -> Vec<&LogEntry> {
        self.log.iter()
            .filter(|e| e.sequence >= from_seq)
            .take(max)
            .collect()
    }

    pub fn register_follower(&mut self, id: u64) {
        self.followers.insert(id, FollowerInfo::new(id));
    }

    pub fn unregister_follower(&mut self, id: u64) { self.followers.remove(&id); }

    pub fn send_entries(&mut self, follower_id: u64, count: usize) -> Vec<u64> {
        let leader_seq = self.current_sequence;
        let follower = match self.followers.get_mut(&follower_id) { Some(f) => f, None => return Vec::new() };
        let start = follower.next_sequence;
        let entries: Vec<u64> = self.log.iter()
            .filter(|e| e.sequence >= start)
            .take(count)
            .map(|e| e.sequence)
            .collect();
        let sent_count = entries.len() as u64;
        let sent_bytes: usize = self.log.iter()
            .filter(|e| e.sequence >= start)
            .take(count)
            .map(|e| e.data_len + 64)
            .sum();
        follower.entries_sent += sent_count;
        follower.bytes_sent += sent_bytes as u64;
        if let Some(&last) = entries.last() { follower.next_sequence = last + 1; }
        follower.state = if follower.replication_lag(leader_seq) == 0 {
            FollowerState::Synced
        } else {
            FollowerState::Catching
        };
        entries
    }

    pub fn ack(&mut self, follower_id: u64, seq: u64, ts: u64) {
        if let Some(f) = self.followers.get_mut(&follower_id) {
            if seq > f.match_sequence {
                f.entries_acked += seq - f.match_sequence;
                f.match_sequence = seq;
            }
            f.last_ack_ts = ts;
            if f.match_sequence >= self.current_sequence {
                f.state = FollowerState::Synced;
            }
        }
    }

    pub fn create_snapshot(&mut self, ts: u64) -> u64 {
        let id = self.snapshots.len() as u64 + 1;
        let meta = SnapshotMeta {
            snapshot_id: id, last_included_seq: self.current_sequence,
            last_included_term: self.current_term, size_bytes: self.total_bytes,
            created_ts: ts, entry_count: self.log.len() as u64,
        };
        self.snapshots.push(meta);
        id
    }

    pub fn compact(&mut self) {
        if self.log.len() <= self.compaction_policy.min_entries_to_keep { return; }
        let min_follower_seq = self.followers.values().map(|f| f.match_sequence).min().unwrap_or(self.current_sequence);
        let safe_seq = min_follower_seq.saturating_sub(self.compaction_policy.min_entries_to_keep as u64);
        let before = self.log.len();
        self.log.retain(|e| e.sequence > safe_seq);
        if self.log.len() < before {
            self.first_log_sequence = self.log.first().map(|e| e.sequence).unwrap_or(self.current_sequence + 1);
            self.compactions_done += 1;
        }
    }

    pub fn verify_log_integrity(&mut self) -> usize {
        let mut failures = 0;
        for entry in &self.log {
            if !entry.verify_checksum() { failures += 1; }
        }
        self.checksum_failures += failures as u64;
        failures
    }

    pub fn recompute(&mut self) {
        self.stats.total_entries = self.log.len();
        self.stats.total_bytes = self.total_bytes;
        self.stats.current_sequence = self.current_sequence;
        self.stats.current_term = self.current_term;
        self.stats.follower_count = self.followers.len();
        self.stats.synced_followers = self.followers.values().filter(|f| f.state == FollowerState::Synced).count();
        self.stats.snapshot_count = self.snapshots.len();
        self.stats.compactions_done = self.compactions_done;
        self.stats.checksum_failures = self.checksum_failures;
        if !self.followers.is_empty() {
            let total_lag: u64 = self.followers.values().map(|f| f.replication_lag(self.current_sequence)).sum();
            self.stats.avg_replication_lag = total_lag as f64 / self.followers.len() as f64;
        }
    }

    pub fn stats(&self) -> &LogReplicatorStats { &self.stats }
}
