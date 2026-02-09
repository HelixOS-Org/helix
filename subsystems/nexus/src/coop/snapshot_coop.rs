//! # Cooperative Snapshot Service
//!
//! Cooperative state snapshots for coordination:
//! - Point-in-time state capture
//! - Incremental snapshots via dirty tracking
//! - Snapshot comparison and diffing
//! - Snapshot-based rollback
//! - Multi-process coordinated snapshots

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// SNAPSHOT TYPES
// ============================================================================

/// Snapshot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotState {
    /// Being created
    Creating,
    /// Complete and valid
    Complete,
    /// Partial (incomplete)
    Partial,
    /// Corrupted
    Corrupted,
    /// Deleted
    Deleted,
}

/// Data type in snapshot
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotDataType {
    /// Register state
    Registers,
    /// Memory region
    Memory,
    /// File descriptors
    FileDescriptors,
    /// Signal handlers
    SignalHandlers,
    /// IPC state
    IpcState,
    /// Custom key-value
    KeyValue,
}

/// Snapshot entry
#[derive(Debug, Clone)]
pub struct SnapshotEntry {
    /// Data type
    pub data_type: SnapshotDataType,
    /// Key/identifier
    pub key: String,
    /// Value hash (for comparison)
    pub value_hash: u64,
    /// Size in bytes
    pub size: u64,
    /// Is dirty since last snapshot
    pub is_dirty: bool,
}

// ============================================================================
// SNAPSHOT
// ============================================================================

/// A point-in-time snapshot
#[derive(Debug)]
pub struct CoopSnapshot {
    /// Snapshot id
    pub id: u64,
    /// Owner process
    pub owner: u64,
    /// State
    pub state: SnapshotState,
    /// Creation timestamp
    pub created_at: u64,
    /// Entries
    entries: Vec<SnapshotEntry>,
    /// Parent snapshot id (for incremental)
    pub parent_id: Option<u64>,
    /// Total size
    pub total_size: u64,
    /// Dirty size (incremental portion)
    pub dirty_size: u64,
    /// Sequence number
    pub sequence: u64,
    /// Checksum (FNV-1a of all entry hashes)
    pub checksum: u64,
}

impl CoopSnapshot {
    pub fn new(id: u64, owner: u64, sequence: u64, now: u64) -> Self {
        Self {
            id,
            owner,
            state: SnapshotState::Creating,
            created_at: now,
            entries: Vec::new(),
            parent_id: None,
            total_size: 0,
            dirty_size: 0,
            sequence,
            checksum: 0,
        }
    }

    /// Add entry
    #[inline]
    pub fn add_entry(&mut self, entry: SnapshotEntry) {
        self.total_size += entry.size;
        if entry.is_dirty {
            self.dirty_size += entry.size;
        }
        self.entries.push(entry);
    }

    /// Finalize snapshot
    #[inline(always)]
    pub fn finalize(&mut self) {
        self.checksum = self.compute_checksum();
        self.state = SnapshotState::Complete;
    }

    fn compute_checksum(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for entry in &self.entries {
            hash ^= entry.value_hash;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Entry count
    #[inline(always)]
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Dirty ratio
    #[inline]
    pub fn dirty_ratio(&self) -> f64 {
        if self.total_size == 0 {
            return 0.0;
        }
        self.dirty_size as f64 / self.total_size as f64
    }

    /// Entries
    #[inline(always)]
    pub fn entries(&self) -> &[SnapshotEntry] {
        &self.entries
    }
}

// ============================================================================
// DIFF
// ============================================================================

/// Diff type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffType {
    /// Entry added
    Added,
    /// Entry removed
    Removed,
    /// Entry modified
    Modified,
    /// Entry unchanged
    Unchanged,
}

/// Diff entry
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    /// Key
    pub key: String,
    /// Diff type
    pub diff_type: DiffType,
    /// Old hash
    pub old_hash: Option<u64>,
    /// New hash
    pub new_hash: Option<u64>,
    /// Size delta
    pub size_delta: i64,
}

/// Compare two snapshots
pub fn compare_snapshots(old: &CoopSnapshot, new: &CoopSnapshot) -> Vec<SnapshotDiff> {
    let mut diffs = Vec::new();

    // Build map of old entries
    let mut old_map: BTreeMap<u64, &SnapshotEntry> = BTreeMap::new();
    for entry in &old.entries {
        old_map.insert(entry.value_hash ^ fnv_hash_str(&entry.key), entry);
    }

    // Check new entries against old
    for entry in &new.entries {
        let key_hash = entry.value_hash ^ fnv_hash_str(&entry.key);
        if let Some(old_entry) = old_map.remove(&key_hash) {
            if old_entry.value_hash != entry.value_hash {
                diffs.push(SnapshotDiff {
                    key: entry.key.clone(),
                    diff_type: DiffType::Modified,
                    old_hash: Some(old_entry.value_hash),
                    new_hash: Some(entry.value_hash),
                    size_delta: entry.size as i64 - old_entry.size as i64,
                });
            }
        } else {
            diffs.push(SnapshotDiff {
                key: entry.key.clone(),
                diff_type: DiffType::Added,
                old_hash: None,
                new_hash: Some(entry.value_hash),
                size_delta: entry.size as i64,
            });
        }
    }

    // Remaining old entries are removed
    for (_, entry) in old_map {
        diffs.push(SnapshotDiff {
            key: entry.key.clone(),
            diff_type: DiffType::Removed,
            old_hash: Some(entry.value_hash),
            new_hash: None,
            size_delta: -(entry.size as i64),
        });
    }

    diffs
}

fn fnv_hash_str(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

// ============================================================================
// COORDINATED SNAPSHOT
// ============================================================================

/// Multi-process coordinated snapshot
#[derive(Debug)]
pub struct CoordinatedSnapshot {
    /// Coordination id
    pub id: u64,
    /// Participant pids
    pub participants: Vec<u64>,
    /// Per-participant snapshot ids
    pub snapshots: LinearMap<u64, 64>,
    /// All participants completed?
    pub is_complete: bool,
    /// Timestamp
    pub timestamp: u64,
    /// Barrier count
    pub barrier_count: usize,
}

impl CoordinatedSnapshot {
    pub fn new(id: u64, participants: Vec<u64>, now: u64) -> Self {
        Self {
            id,
            participants,
            snapshots: LinearMap::new(),
            is_complete: false,
            timestamp: now,
            barrier_count: 0,
        }
    }

    /// Register participant snapshot
    #[inline]
    pub fn register_snapshot(&mut self, pid: u64, snapshot_id: u64) {
        self.snapshots.insert(pid, snapshot_id);
        self.barrier_count = self.snapshots.len();
        if self.barrier_count == self.participants.len() {
            self.is_complete = true;
        }
    }

    /// Progress ratio
    #[inline]
    pub fn progress(&self) -> f64 {
        if self.participants.is_empty() {
            return 1.0;
        }
        self.snapshots.len() as f64 / self.participants.len() as f64
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Snapshot stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopSnapshotStats {
    /// Total snapshots
    pub total_snapshots: u64,
    /// Active snapshots
    pub active_count: usize,
    /// Total size (bytes)
    pub total_size: u64,
    /// Coordinated snapshots
    pub coordinated_count: usize,
}

/// Cooperative snapshot manager
pub struct CoopSnapshotManager {
    /// Snapshots
    snapshots: BTreeMap<u64, CoopSnapshot>,
    /// Coordinated snapshots
    coordinated: BTreeMap<u64, CoordinatedSnapshot>,
    /// Next ids
    next_snap_id: u64,
    next_coord_id: u64,
    /// Stats
    stats: CoopSnapshotStats,
}

impl CoopSnapshotManager {
    pub fn new() -> Self {
        Self {
            snapshots: LinearMap::new(),
            coordinated: BTreeMap::new(),
            next_snap_id: 1,
            next_coord_id: 1,
            stats: CoopSnapshotStats::default(),
        }
    }

    /// Create snapshot
    #[inline]
    pub fn create(&mut self, owner: u64, now: u64) -> u64 {
        let id = self.next_snap_id;
        self.next_snap_id += 1;
        let seq = self.snapshots.values().filter(|s| s.owner == owner).count() as u64;
        let snapshot = CoopSnapshot::new(id, owner, seq, now);
        self.snapshots.insert(id, snapshot);
        self.stats.total_snapshots += 1;
        self.update_stats();
        id
    }

    /// Add entry to snapshot
    #[inline]
    pub fn add_entry(&mut self, snap_id: u64, entry: SnapshotEntry) -> bool {
        if let Some(snap) = self.snapshots.get_mut(&snap_id) {
            snap.add_entry(entry);
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Finalize snapshot
    #[inline]
    pub fn finalize(&mut self, snap_id: u64) -> bool {
        if let Some(snap) = self.snapshots.get_mut(&snap_id) {
            snap.finalize();
            true
        } else {
            false
        }
    }

    /// Delete snapshot
    #[inline]
    pub fn delete(&mut self, snap_id: u64) -> bool {
        if let Some(snap) = self.snapshots.get_mut(&snap_id) {
            snap.state = SnapshotState::Deleted;
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Create coordinated snapshot
    #[inline]
    pub fn create_coordinated(&mut self, participants: Vec<u64>, now: u64) -> u64 {
        let id = self.next_coord_id;
        self.next_coord_id += 1;
        let coord = CoordinatedSnapshot::new(id, participants, now);
        self.coordinated.insert(id, coord);
        self.update_stats();
        id
    }

    /// Register participant in coordinated snapshot
    #[inline]
    pub fn register_participant(&mut self, coord_id: u64, pid: u64, snap_id: u64) -> bool {
        if let Some(coord) = self.coordinated.get_mut(&coord_id) {
            coord.register_snapshot(pid, snap_id);
            true
        } else {
            false
        }
    }

    /// Get snapshot
    #[inline(always)]
    pub fn snapshot(&self, id: u64) -> Option<&CoopSnapshot> {
        self.snapshots.get(id)
    }

    fn update_stats(&mut self) {
        self.stats.active_count = self
            .snapshots
            .values()
            .filter(|s| s.state == SnapshotState::Complete)
            .count();
        self.stats.total_size = self
            .snapshots
            .values()
            .filter(|s| s.state == SnapshotState::Complete)
            .map(|s| s.total_size)
            .sum();
        self.stats.coordinated_count = self.coordinated.values().filter(|c| !c.is_complete).count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopSnapshotStats {
        &self.stats
    }
}
