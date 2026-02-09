//! # Coop Consensus Log
//!
//! Replicated log for consensus-based state machines:
//! - Append-only log with compaction
//! - Log entry indexing and term tracking
//! - Snapshot-based log truncation
//! - Log replication progress tracking
//! - Conflict detection in concurrent appends
//! - Write-ahead log semantics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Log entry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEntryState {
    Pending,
    Committed,
    Applied,
    Compacted,
}

/// Log entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogEntryType {
    Command,
    Configuration,
    NoOp,
    Barrier,
    Snapshot,
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub index: u64,
    pub term: u64,
    pub entry_type: LogEntryType,
    pub state: LogEntryState,
    pub payload_hash: u64,
    pub payload_size: u32,
    pub proposer_node: u64,
    pub created_ts: u64,
    pub committed_ts: Option<u64>,
    pub applied_ts: Option<u64>,
}

impl LogEntry {
    pub fn new(index: u64, term: u64, etype: LogEntryType, proposer: u64, ts: u64) -> Self {
        Self {
            index, term, entry_type: etype, state: LogEntryState::Pending,
            payload_hash: 0, payload_size: 0, proposer_node: proposer,
            created_ts: ts, committed_ts: None, applied_ts: None,
        }
    }

    #[inline(always)]
    pub fn commit(&mut self, ts: u64) { self.state = LogEntryState::Committed; self.committed_ts = Some(ts); }
    #[inline(always)]
    pub fn apply(&mut self, ts: u64) { self.state = LogEntryState::Applied; self.applied_ts = Some(ts); }
    #[inline(always)]
    pub fn compact(&mut self) { self.state = LogEntryState::Compacted; }

    #[inline(always)]
    pub fn commit_latency_ns(&self) -> Option<u64> {
        self.committed_ts.map(|ct| ct.saturating_sub(self.created_ts))
    }

    #[inline(always)]
    pub fn apply_latency_ns(&self) -> Option<u64> {
        self.applied_ts.map(|at| at.saturating_sub(self.created_ts))
    }
}

/// Snapshot metadata
#[derive(Debug, Clone)]
pub struct SnapshotMeta {
    pub last_included_index: u64,
    pub last_included_term: u64,
    pub size_bytes: u64,
    pub created_ts: u64,
    pub checksum: u64,
}

impl SnapshotMeta {
    pub fn new(index: u64, term: u64, size: u64, ts: u64) -> Self {
        let mut checksum: u64 = 0xcbf29ce484222325;
        for &b in &index.to_le_bytes() { checksum ^= b as u64; checksum = checksum.wrapping_mul(0x100000001b3); }
        for &b in &term.to_le_bytes() { checksum ^= b as u64; checksum = checksum.wrapping_mul(0x100000001b3); }
        Self { last_included_index: index, last_included_term: term, size_bytes: size, created_ts: ts, checksum }
    }
}

/// Replication progress for a follower
#[derive(Debug, Clone)]
pub struct ReplicationProgress {
    pub node_id: u64,
    pub match_index: u64,
    pub next_index: u64,
    pub in_flight: u32,
    pub max_in_flight: u32,
    pub last_contact_ts: u64,
    pub entries_sent: u64,
    pub entries_acked: u64,
    pub is_stale: bool,
}

impl ReplicationProgress {
    pub fn new(node_id: u64, next_index: u64) -> Self {
        Self {
            node_id, match_index: 0, next_index, in_flight: 0,
            max_in_flight: 64, last_contact_ts: 0, entries_sent: 0,
            entries_acked: 0, is_stale: false,
        }
    }

    #[inline]
    pub fn advance(&mut self, new_match: u64) {
        if new_match > self.match_index {
            self.match_index = new_match;
            self.next_index = new_match + 1;
            self.in_flight = self.in_flight.saturating_sub(1);
            self.entries_acked += 1;
        }
    }

    #[inline(always)]
    pub fn can_send(&self) -> bool { self.in_flight < self.max_in_flight }

    #[inline(always)]
    pub fn lag(&self, leader_last: u64) -> u64 { leader_last.saturating_sub(self.match_index) }
}

/// Consensus log stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ConsensusLogStats {
    pub total_entries: usize,
    pub committed_entries: usize,
    pub applied_entries: usize,
    pub pending_entries: usize,
    pub current_term: u64,
    pub last_index: u64,
    pub commit_index: u64,
    pub last_applied: u64,
    pub snapshot_index: u64,
    pub follower_count: usize,
    pub max_lag: u64,
    pub avg_commit_latency_ns: f64,
}

/// Coop consensus log
pub struct CoopConsensusLog {
    entries: BTreeMap<u64, LogEntry>,
    followers: BTreeMap<u64, ReplicationProgress>,
    snapshots: Vec<SnapshotMeta>,
    stats: ConsensusLogStats,
    current_term: u64,
    commit_index: u64,
    last_applied: u64,
    first_index: u64,
}

impl CoopConsensusLog {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(), followers: BTreeMap::new(),
            snapshots: Vec::new(), stats: ConsensusLogStats::default(),
            current_term: 0, commit_index: 0, last_applied: 0, first_index: 1,
        }
    }

    #[inline(always)]
    pub fn last_index(&self) -> u64 {
        self.entries.keys().last().copied().unwrap_or(0)
    }

    #[inline(always)]
    pub fn last_term(&self) -> u64 {
        self.entries.values().last().map(|e| e.term).unwrap_or(0)
    }

    #[inline]
    pub fn append(&mut self, term: u64, etype: LogEntryType, proposer: u64, ts: u64) -> u64 {
        let index = self.last_index() + 1;
        self.entries.insert(index, LogEntry::new(index, term, etype, proposer, ts));
        if term > self.current_term { self.current_term = term; }
        index
    }

    #[inline]
    pub fn commit_up_to(&mut self, index: u64, ts: u64) {
        for i in (self.commit_index + 1)..=index {
            if let Some(entry) = self.entries.get_mut(&i) {
                if entry.state == LogEntryState::Pending { entry.commit(ts); }
            }
        }
        self.commit_index = index;
    }

    #[inline]
    pub fn apply_up_to(&mut self, index: u64, ts: u64) {
        for i in (self.last_applied + 1)..=index.min(self.commit_index) {
            if let Some(entry) = self.entries.get_mut(&i) {
                if entry.state == LogEntryState::Committed { entry.apply(ts); }
            }
        }
        self.last_applied = index.min(self.commit_index);
    }

    #[inline(always)]
    pub fn truncate_after(&mut self, index: u64) {
        let to_remove: Vec<u64> = self.entries.range((index + 1)..).map(|(&k, _)| k).collect();
        for k in to_remove { self.entries.remove(&k); }
    }

    #[inline]
    pub fn compact_before(&mut self, index: u64) {
        let to_compact: Vec<u64> = self.entries.range(..index).map(|(&k, _)| k).collect();
        for k in to_compact { self.entries.remove(&k); }
        self.first_index = index;
    }

    #[inline(always)]
    pub fn take_snapshot(&mut self, index: u64, term: u64, size: u64, ts: u64) {
        self.snapshots.push(SnapshotMeta::new(index, term, size, ts));
        self.compact_before(index);
    }

    #[inline(always)]
    pub fn add_follower(&mut self, node_id: u64) {
        let next = self.last_index() + 1;
        self.followers.insert(node_id, ReplicationProgress::new(node_id, next));
    }

    #[inline]
    pub fn follower_ack(&mut self, node_id: u64, match_index: u64, ts: u64) {
        if let Some(f) = self.followers.get_mut(&node_id) {
            f.advance(match_index);
            f.last_contact_ts = ts;
        }
    }

    #[inline]
    pub fn compute_commit_index(&self) -> u64 {
        if self.followers.is_empty() { return self.last_index(); }
        let mut matches: Vec<u64> = self.followers.values().map(|f| f.match_index).collect();
        matches.sort_unstable();
        // Majority = (n+1)/2
        let majority_idx = matches.len() / 2;
        matches[majority_idx]
    }

    #[inline(always)]
    pub fn entry(&self, index: u64) -> Option<&LogEntry> { self.entries.get(&index) }

    pub fn recompute(&mut self) {
        self.stats.total_entries = self.entries.len();
        self.stats.committed_entries = self.entries.values().filter(|e| e.state == LogEntryState::Committed || e.state == LogEntryState::Applied).count();
        self.stats.applied_entries = self.entries.values().filter(|e| e.state == LogEntryState::Applied).count();
        self.stats.pending_entries = self.entries.values().filter(|e| e.state == LogEntryState::Pending).count();
        self.stats.current_term = self.current_term;
        self.stats.last_index = self.last_index();
        self.stats.commit_index = self.commit_index;
        self.stats.last_applied = self.last_applied;
        self.stats.snapshot_index = self.snapshots.last().map(|s| s.last_included_index).unwrap_or(0);
        self.stats.follower_count = self.followers.len();
        let last = self.last_index();
        self.stats.max_lag = self.followers.values().map(|f| f.lag(last)).max().unwrap_or(0);
        let commit_lats: Vec<u64> = self.entries.values().filter_map(|e| e.commit_latency_ns()).collect();
        self.stats.avg_commit_latency_ns = if commit_lats.is_empty() { 0.0 } else { commit_lats.iter().sum::<u64>() as f64 / commit_lats.len() as f64 };
    }

    #[inline(always)]
    pub fn stats(&self) -> &ConsensusLogStats { &self.stats }
}
