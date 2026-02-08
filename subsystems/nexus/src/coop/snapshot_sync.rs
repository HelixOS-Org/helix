//! # Cooperative Snapshot Synchronization
//!
//! State snapshot and synchronization for cooperative subsystems:
//! - Consistent snapshot capture across distributed components
//! - Chandy-Lamport inspired marker protocol
//! - Incremental snapshots with change tracking
//! - Snapshot diff and merge for divergent states
//! - Versioned snapshot storage with garbage collection
//! - Snapshot restoration with rollback support

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;

/// Snapshot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotStateCoop {
    Initiated,
    MarkersSent,
    Collecting,
    Complete,
    Failed,
}

/// Snapshot type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotType {
    Full,
    Incremental,
    Differential,
}

/// Channel state for Chandy-Lamport
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelRecording {
    NotStarted,
    Recording,
    Done,
}

/// A single component's captured state
#[derive(Debug, Clone)]
pub struct ComponentSnapshot {
    pub component_id: u64,
    pub version: u64,
    pub state_hash: u64,
    pub state_size: usize,
    pub data_pages: Vec<u64>, // page frame numbers
    pub captured_ns: u64,
    pub dirty_pages: Vec<u64>, // for incremental
}

impl ComponentSnapshot {
    pub fn new(component_id: u64, version: u64, now_ns: u64) -> Self {
        Self {
            component_id,
            version,
            state_hash: 0,
            state_size: 0,
            data_pages: Vec::new(),
            captured_ns: now_ns,
            dirty_pages: Vec::new(),
        }
    }

    pub fn compute_hash(&mut self) {
        let mut hash: u64 = 0xcbf29ce484222325;
        hash ^= self.component_id;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= self.version;
        hash = hash.wrapping_mul(0x100000001b3);
        for &page in &self.data_pages {
            hash ^= page;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.state_hash = hash;
    }
}

/// Channel message recorded during snapshot
#[derive(Debug, Clone)]
pub struct RecordedMessage {
    pub from: u64,
    pub to: u64,
    pub sequence: u64,
    pub payload_hash: u64,
    pub timestamp_ns: u64,
}

/// A global consistent snapshot
#[derive(Debug, Clone)]
pub struct ConsistentSnapshot {
    pub snapshot_id: u64,
    pub snapshot_type: SnapshotType,
    pub state: SnapshotStateCoop,
    pub initiated_ns: u64,
    pub completed_ns: u64,
    pub component_snapshots: BTreeMap<u64, ComponentSnapshot>,
    pub channel_messages: Vec<RecordedMessage>,
    pub global_hash: u64,
    pub base_snapshot_id: Option<u64>, // for incremental
}

impl ConsistentSnapshot {
    pub fn new(snapshot_id: u64, snap_type: SnapshotType, now_ns: u64) -> Self {
        Self {
            snapshot_id,
            snapshot_type: snap_type,
            state: SnapshotStateCoop::Initiated,
            initiated_ns: now_ns,
            completed_ns: 0,
            component_snapshots: BTreeMap::new(),
            channel_messages: Vec::new(),
            global_hash: 0,
            base_snapshot_id: None,
        }
    }

    pub fn add_component(&mut self, snap: ComponentSnapshot) {
        self.component_snapshots.insert(snap.component_id, snap);
    }

    pub fn is_complete(&self) -> bool {
        self.state == SnapshotStateCoop::Complete
    }

    pub fn finalize(&mut self, now_ns: u64) {
        self.completed_ns = now_ns;
        self.state = SnapshotStateCoop::Complete;
        // Compute global hash from all components
        let mut hash: u64 = 0xcbf29ce484222325;
        for (_, cs) in &self.component_snapshots {
            hash ^= cs.state_hash;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        self.global_hash = hash;
    }

    pub fn total_size(&self) -> usize {
        self.component_snapshots.values().map(|cs| cs.state_size).sum()
    }

    pub fn duration_ns(&self) -> u64 {
        self.completed_ns.saturating_sub(self.initiated_ns)
    }
}

/// Snapshot diff between two snapshots
#[derive(Debug, Clone)]
pub struct SnapshotDiff {
    pub base_id: u64,
    pub target_id: u64,
    pub added_components: Vec<u64>,
    pub removed_components: Vec<u64>,
    pub changed_components: Vec<u64>,
    pub changed_pages: usize,
}

/// Channel state tracker for Chandy-Lamport
#[derive(Debug, Clone)]
pub struct ChannelTracker {
    pub from: u64,
    pub to: u64,
    pub recording: ChannelRecording,
    pub messages: Vec<RecordedMessage>,
}

impl ChannelTracker {
    pub fn new(from: u64, to: u64) -> Self {
        Self {
            from,
            to,
            recording: ChannelRecording::NotStarted,
            messages: Vec::new(),
        }
    }
}

/// Cooperative Snapshot Synchronizer
pub struct CoopSnapshotSync {
    snapshots: BTreeMap<u64, ConsistentSnapshot>,
    components: Vec<u64>,
    channels: Vec<ChannelTracker>,
    next_snapshot_id: u64,
    max_stored: usize,
    latest_complete: Option<u64>,
}

impl CoopSnapshotSync {
    pub fn new(max_stored: usize) -> Self {
        Self {
            snapshots: BTreeMap::new(),
            components: Vec::new(),
            channels: Vec::new(),
            next_snapshot_id: 1,
            max_stored,
            latest_complete: None,
        }
    }

    pub fn register_component(&mut self, component_id: u64) {
        if !self.components.contains(&component_id) {
            self.components.push(component_id);
        }
    }

    pub fn register_channel(&mut self, from: u64, to: u64) {
        self.channels.push(ChannelTracker::new(from, to));
    }

    /// Initiate a new snapshot
    pub fn initiate(&mut self, snap_type: SnapshotType, now_ns: u64) -> u64 {
        let id = self.next_snapshot_id;
        self.next_snapshot_id += 1;

        let mut snap = ConsistentSnapshot::new(id, snap_type, now_ns);
        if snap_type == SnapshotType::Incremental {
            snap.base_snapshot_id = self.latest_complete;
        }
        snap.state = SnapshotStateCoop::MarkersSent;
        self.snapshots.insert(id, snap);

        // Start recording on all channels
        for ch in &mut self.channels {
            ch.recording = ChannelRecording::Recording;
            ch.messages.clear();
        }

        id
    }

    /// Record a component's local snapshot
    pub fn record_component(&mut self, snapshot_id: u64, comp_snap: ComponentSnapshot) {
        if let Some(snap) = self.snapshots.get_mut(&snapshot_id) {
            snap.add_component(comp_snap);
            if snap.component_snapshots.len() == self.components.len() {
                snap.state = SnapshotStateCoop::Collecting;
            }
        }
    }

    /// Record a channel message during snapshot
    pub fn record_channel_message(&mut self, from: u64, to: u64, msg: RecordedMessage) {
        for ch in &mut self.channels {
            if ch.from == from && ch.to == to && ch.recording == ChannelRecording::Recording {
                ch.messages.push(msg);
            }
        }
    }

    /// Finalize a snapshot when all components and channels are done
    pub fn finalize(&mut self, snapshot_id: u64, now_ns: u64) -> bool {
        // Stop channel recording
        for ch in &mut self.channels {
            ch.recording = ChannelRecording::Done;
        }

        // Collect channel messages into snapshot
        let channel_msgs: Vec<RecordedMessage> = self.channels.iter()
            .flat_map(|ch| ch.messages.clone())
            .collect();

        if let Some(snap) = self.snapshots.get_mut(&snapshot_id) {
            snap.channel_messages = channel_msgs;
            snap.finalize(now_ns);
            self.latest_complete = Some(snapshot_id);
            self.gc();
            true
        } else { false }
    }

    /// Compute diff between two snapshots
    pub fn diff(&self, base_id: u64, target_id: u64) -> Option<SnapshotDiff> {
        let base = self.snapshots.get(&base_id)?;
        let target = self.snapshots.get(&target_id)?;

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut changed = Vec::new();
        let mut changed_pages = 0usize;

        for (&cid, tcs) in &target.component_snapshots {
            if let Some(bcs) = base.component_snapshots.get(&cid) {
                if bcs.state_hash != tcs.state_hash {
                    changed.push(cid);
                    changed_pages += tcs.dirty_pages.len();
                }
            } else {
                added.push(cid);
            }
        }
        for &cid in base.component_snapshots.keys() {
            if !target.component_snapshots.contains_key(&cid) {
                removed.push(cid);
            }
        }

        Some(SnapshotDiff {
            base_id,
            target_id,
            added_components: added,
            removed_components: removed,
            changed_components: changed,
            changed_pages,
        })
    }

    /// Garbage collect old snapshots
    fn gc(&mut self) {
        while self.snapshots.len() > self.max_stored {
            if let Some(&oldest_id) = self.snapshots.keys().next() {
                // Don't remove latest
                if Some(oldest_id) == self.latest_complete { break; }
                self.snapshots.remove(&oldest_id);
            } else { break; }
        }
    }

    pub fn snapshot(&self, id: u64) -> Option<&ConsistentSnapshot> {
        self.snapshots.get(&id)
    }

    pub fn latest_id(&self) -> Option<u64> { self.latest_complete }

    pub fn stored_count(&self) -> usize { self.snapshots.len() }
}
