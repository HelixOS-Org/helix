//! # Coop Versioned State
//!
//! Multi-version concurrency control for cooperative state:
//! - MVCC with snapshot isolation
//! - Version chains per key
//! - Garbage collection of old versions
//! - Read/write timestamp ordering
//! - Conflict serialization graph
//! - Optimistic concurrency control

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Version visibility
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionVisibility {
    Visible,
    Committed,
    Aborted,
    InProgress,
}

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MvccTxnState {
    Active,
    Committed,
    Aborted,
    Validating,
}

/// Version record
#[derive(Debug, Clone)]
pub struct VersionRecord {
    pub version_id: u64,
    pub key: u64,
    pub value_hash: u64,
    pub value_size: u32,
    pub created_by_txn: u64,
    pub created_ts: u64,
    pub visibility: VersionVisibility,
    pub prev_version: Option<u64>,
}

impl VersionRecord {
    pub fn new(id: u64, key: u64, txn: u64, ts: u64) -> Self {
        Self {
            version_id: id, key, value_hash: 0, value_size: 0,
            created_by_txn: txn, created_ts: ts,
            visibility: VersionVisibility::InProgress, prev_version: None,
        }
    }

    pub fn commit(&mut self) { self.visibility = VersionVisibility::Committed; }
    pub fn abort(&mut self) { self.visibility = VersionVisibility::Aborted; }
}

/// Version chain for a key
#[derive(Debug, Clone)]
pub struct VersionChain {
    pub key: u64,
    pub head_version: u64,
    pub chain_length: u32,
    pub oldest_version_ts: u64,
    pub newest_version_ts: u64,
    pub total_versions: u64,
    pub gc_eligible: u32,
}

impl VersionChain {
    pub fn new(key: u64, head: u64, ts: u64) -> Self {
        Self { key, head_version: head, chain_length: 1, oldest_version_ts: ts, newest_version_ts: ts, total_versions: 1, gc_eligible: 0 }
    }
}

/// MVCC Transaction
#[derive(Debug, Clone)]
pub struct MvccTransaction {
    pub txn_id: u64,
    pub state: MvccTxnState,
    pub start_ts: u64,
    pub commit_ts: Option<u64>,
    pub snapshot_ts: u64,
    pub read_set: Vec<u64>,
    pub write_set: Vec<u64>,
    pub versions_created: Vec<u64>,
    pub is_read_only: bool,
}

impl MvccTransaction {
    pub fn new(id: u64, ts: u64) -> Self {
        Self {
            txn_id: id, state: MvccTxnState::Active, start_ts: ts,
            commit_ts: None, snapshot_ts: ts, read_set: Vec::new(),
            write_set: Vec::new(), versions_created: Vec::new(), is_read_only: true,
        }
    }

    pub fn read(&mut self, key: u64) { self.read_set.push(key); }

    pub fn write(&mut self, key: u64, version_id: u64) {
        self.write_set.push(key);
        self.versions_created.push(version_id);
        self.is_read_only = false;
    }

    pub fn commit(&mut self, ts: u64) { self.state = MvccTxnState::Committed; self.commit_ts = Some(ts); }
    pub fn abort(&mut self) { self.state = MvccTxnState::Aborted; }

    pub fn has_conflict(&self, other: &MvccTransaction) -> bool {
        // Write-write conflict: overlapping write sets
        for key in &self.write_set {
            if other.write_set.contains(key) { return true; }
        }
        // Read-write conflict: this reads what other writes
        for key in &self.read_set {
            if other.write_set.contains(key) { return true; }
        }
        false
    }
}

/// Snapshot descriptor
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub snapshot_ts: u64,
    pub active_txns: Vec<u64>,
    pub min_active_ts: u64,
}

impl Snapshot {
    pub fn new(ts: u64, active: Vec<u64>, min_active: u64) -> Self {
        Self { snapshot_ts: ts, active_txns: active, min_active_ts: min_active }
    }

    pub fn is_visible(&self, version: &VersionRecord) -> bool {
        if version.visibility == VersionVisibility::Aborted { return false; }
        if version.created_ts > self.snapshot_ts { return false; }
        if self.active_txns.contains(&version.created_by_txn) { return false; }
        version.visibility == VersionVisibility::Committed
    }
}

/// MVCC stats
#[derive(Debug, Clone, Default)]
pub struct MvccStats {
    pub total_keys: usize,
    pub total_versions: usize,
    pub active_txns: usize,
    pub committed_txns: u64,
    pub aborted_txns: u64,
    pub conflict_aborts: u64,
    pub gc_eligible_versions: usize,
    pub avg_chain_length: f64,
    pub max_chain_length: u32,
}

/// Coop versioned state store
pub struct CoopVersionedState {
    versions: BTreeMap<u64, VersionRecord>,
    chains: BTreeMap<u64, VersionChain>,
    transactions: BTreeMap<u64, MvccTransaction>,
    stats: MvccStats,
    next_id: u64,
    committed_txns: u64,
    aborted_txns: u64,
    conflict_aborts: u64,
}

impl CoopVersionedState {
    pub fn new() -> Self {
        Self {
            versions: BTreeMap::new(), chains: BTreeMap::new(),
            transactions: BTreeMap::new(), stats: MvccStats::default(),
            next_id: 1, committed_txns: 0, aborted_txns: 0, conflict_aborts: 0,
        }
    }

    pub fn begin_txn(&mut self, ts: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.transactions.insert(id, MvccTransaction::new(id, ts));
        id
    }

    pub fn read(&mut self, txn_id: u64, key: u64) -> Option<u64> {
        if let Some(txn) = self.transactions.get_mut(&txn_id) { txn.read(key); }
        // Find visible version for key
        let chain = self.chains.get(&key)?;
        let txn = self.transactions.get(&txn_id)?;
        let snapshot = Snapshot::new(txn.snapshot_ts, Vec::new(), 0);
        let mut vid = Some(chain.head_version);
        while let Some(v) = vid {
            if let Some(ver) = self.versions.get(&v) {
                if snapshot.is_visible(ver) { return Some(v); }
                vid = ver.prev_version;
            } else { break; }
        }
        None
    }

    pub fn write(&mut self, txn_id: u64, key: u64, ts: u64) -> Option<u64> {
        let ver_id = self.next_id; self.next_id += 1;
        let mut ver = VersionRecord::new(ver_id, key, txn_id, ts);
        // Link to previous
        if let Some(chain) = self.chains.get(&key) {
            ver.prev_version = Some(chain.head_version);
        }
        self.versions.insert(ver_id, ver);
        // Update chain
        if let Some(chain) = self.chains.get_mut(&key) {
            chain.head_version = ver_id;
            chain.chain_length += 1;
            chain.newest_version_ts = ts;
            chain.total_versions += 1;
        } else {
            self.chains.insert(key, VersionChain::new(key, ver_id, ts));
        }
        if let Some(txn) = self.transactions.get_mut(&txn_id) { txn.write(key, ver_id); }
        Some(ver_id)
    }

    pub fn commit_txn(&mut self, txn_id: u64, ts: u64) -> bool {
        // Validate: check for conflicts with other committed txns since snapshot
        let txn = match self.transactions.get(&txn_id) { Some(t) => t.clone(), None => return false };
        if !txn.is_read_only {
            for other in self.transactions.values() {
                if other.txn_id == txn_id { continue; }
                if other.state != MvccTxnState::Committed { continue; }
                if let Some(cts) = other.commit_ts {
                    if cts > txn.snapshot_ts && txn.has_conflict(other) {
                        self.abort_txn(txn_id);
                        self.conflict_aborts += 1;
                        return false;
                    }
                }
            }
        }
        if let Some(t) = self.transactions.get_mut(&txn_id) {
            t.commit(ts);
            for &vid in &t.versions_created.clone() {
                if let Some(v) = self.versions.get_mut(&vid) { v.commit(); }
            }
        }
        self.committed_txns += 1;
        true
    }

    pub fn abort_txn(&mut self, txn_id: u64) {
        if let Some(t) = self.transactions.get_mut(&txn_id) {
            t.abort();
            for &vid in &t.versions_created.clone() {
                if let Some(v) = self.versions.get_mut(&vid) { v.abort(); }
            }
        }
        self.aborted_txns += 1;
    }

    pub fn gc(&mut self, min_active_ts: u64) {
        let gc: Vec<u64> = self.versions.iter()
            .filter(|(_, v)| v.visibility == VersionVisibility::Aborted || (v.visibility == VersionVisibility::Committed && v.created_ts < min_active_ts && v.prev_version.is_some()))
            .map(|(&id, _)| id).collect();
        for id in gc { self.versions.remove(&id); }
    }

    pub fn recompute(&mut self) {
        self.stats.total_keys = self.chains.len();
        self.stats.total_versions = self.versions.len();
        self.stats.active_txns = self.transactions.values().filter(|t| t.state == MvccTxnState::Active).count();
        self.stats.committed_txns = self.committed_txns;
        self.stats.aborted_txns = self.aborted_txns;
        self.stats.conflict_aborts = self.conflict_aborts;
        self.stats.gc_eligible_versions = self.versions.values().filter(|v| v.visibility == VersionVisibility::Aborted).count();
        let lengths: Vec<u32> = self.chains.values().map(|c| c.chain_length).collect();
        self.stats.avg_chain_length = if lengths.is_empty() { 0.0 } else { lengths.iter().map(|&l| l as f64).sum::<f64>() / lengths.len() as f64 };
        self.stats.max_chain_length = lengths.iter().copied().max().unwrap_or(0);
    }

    pub fn stats(&self) -> &MvccStats { &self.stats }
}
