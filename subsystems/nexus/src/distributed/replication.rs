//! # Code Replication
//!
//! Year 3 EVOLUTION - Q4 - Replicate evolved code across distributed nodes

#![allow(dead_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::{Epoch, ImprovementId, NodeId};

// ============================================================================
// REPLICATION TYPES
// ============================================================================

/// Replica ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReplicaId(pub u64);

/// Segment ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SegmentId(pub u64);

static REPLICA_COUNTER: AtomicU64 = AtomicU64::new(1);
static SEGMENT_COUNTER: AtomicU64 = AtomicU64::new(1);

impl ReplicaId {
    pub fn generate() -> Self {
        Self(REPLICA_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl SegmentId {
    pub fn generate() -> Self {
        Self(SEGMENT_COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Code segment
#[derive(Debug, Clone)]
pub struct CodeSegment {
    /// Segment ID
    pub id: SegmentId,
    /// Name
    pub name: String,
    /// Code bytes
    pub code: Vec<u8>,
    /// Version
    pub version: u64,
    /// Hash
    pub hash: u64,
    /// Dependencies
    pub dependencies: Vec<SegmentId>,
    /// Metadata
    pub metadata: SegmentMetadata,
}

/// Segment metadata
#[derive(Debug, Clone)]
pub struct SegmentMetadata {
    /// Size
    pub size: usize,
    /// Created at
    pub created_at: u64,
    /// Last modified
    pub last_modified: u64,
    /// Origin improvement
    pub origin: Option<ImprovementId>,
    /// Checksum
    pub checksum: u64,
}

/// Replica
#[derive(Debug, Clone)]
pub struct Replica {
    /// Replica ID
    pub id: ReplicaId,
    /// Segment ID
    pub segment_id: SegmentId,
    /// Node ID
    pub node_id: NodeId,
    /// Version
    pub version: u64,
    /// Status
    pub status: ReplicaStatus,
    /// Last sync
    pub last_sync: u64,
}

/// Replica status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaStatus {
    /// In sync
    InSync,
    /// Syncing
    Syncing,
    /// Stale
    Stale,
    /// Failed
    Failed,
}

// ============================================================================
// REPLICATION STRATEGY
// ============================================================================

/// Replication strategy
#[derive(Debug, Clone)]
pub enum ReplicationStrategy {
    /// Full replication (all nodes)
    Full,
    /// Quorum (majority)
    Quorum { write: usize, read: usize },
    /// Chain replication
    Chain { length: usize },
    /// Ring replication
    Ring { factor: usize },
    /// Erasure coding
    ErasureCoding { data: usize, parity: usize },
    /// Hierarchical
    Hierarchical { levels: usize },
}

impl Default for ReplicationStrategy {
    fn default() -> Self {
        Self::Quorum { write: 2, read: 2 }
    }
}

/// Replica placement
pub trait ReplicaPlacement: Send + Sync {
    /// Select nodes for new segment
    fn select_nodes(
        &self,
        segment: &CodeSegment,
        available_nodes: &[NodeId],
        factor: usize,
    ) -> Vec<NodeId>;

    /// Name
    fn name(&self) -> &str;
}

/// Random placement
pub struct RandomPlacement {
    state: AtomicU64,
}

impl RandomPlacement {
    pub fn new() -> Self {
        Self {
            state: AtomicU64::new(0x12345678deadbeef),
        }
    }

    fn random(&self, max: usize) -> usize {
        let mut x = self.state.load(Ordering::Relaxed);
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state.store(x, Ordering::Relaxed);
        (x as usize) % max
    }
}

impl Default for RandomPlacement {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicaPlacement for RandomPlacement {
    fn select_nodes(
        &self,
        _segment: &CodeSegment,
        available_nodes: &[NodeId],
        factor: usize,
    ) -> Vec<NodeId> {
        let mut selected = Vec::new();
        let mut remaining: Vec<_> = available_nodes.to_vec();

        while selected.len() < factor && !remaining.is_empty() {
            let idx = self.random(remaining.len());
            selected.push(remaining.remove(idx));
        }

        selected
    }

    fn name(&self) -> &str {
        "Random"
    }
}

/// Load-based placement
pub struct LoadBasedPlacement {
    /// Node loads
    loads: BTreeMap<NodeId, u64>,
}

impl LoadBasedPlacement {
    pub fn new() -> Self {
        Self {
            loads: BTreeMap::new(),
        }
    }

    pub fn update_load(&mut self, node_id: NodeId, load: u64) {
        self.loads.insert(node_id, load);
    }
}

impl Default for LoadBasedPlacement {
    fn default() -> Self {
        Self::new()
    }
}

impl ReplicaPlacement for LoadBasedPlacement {
    fn select_nodes(
        &self,
        _segment: &CodeSegment,
        available_nodes: &[NodeId],
        factor: usize,
    ) -> Vec<NodeId> {
        let mut nodes_with_load: Vec<_> = available_nodes
            .iter()
            .map(|&n| (n, self.loads.get(&n).copied().unwrap_or(0)))
            .collect();

        // Sort by load (ascending)
        nodes_with_load.sort_by_key(|(_, load)| *load);

        nodes_with_load
            .iter()
            .take(factor)
            .map(|(n, _)| *n)
            .collect()
    }

    fn name(&self) -> &str {
        "LoadBased"
    }
}

// ============================================================================
// REPLICATION LOG
// ============================================================================

/// Replication log entry
#[derive(Debug, Clone)]
pub struct ReplicationLogEntry {
    /// Index
    pub index: u64,
    /// Epoch
    pub epoch: Epoch,
    /// Operation
    pub operation: ReplicationOperation,
    /// Timestamp
    pub timestamp: u64,
    /// Committed
    pub committed: bool,
}

/// Replication operation
#[derive(Debug, Clone)]
pub enum ReplicationOperation {
    /// Create segment
    Create(CodeSegment),
    /// Update segment
    Update {
        segment_id: SegmentId,
        new_code: Vec<u8>,
        new_version: u64,
    },
    /// Delete segment
    Delete(SegmentId),
    /// Add replica
    AddReplica {
        segment_id: SegmentId,
        node_id: NodeId,
    },
    /// Remove replica
    RemoveReplica {
        segment_id: SegmentId,
        node_id: NodeId,
    },
}

/// Replication log
pub struct ReplicationLog {
    /// Entries
    entries: Vec<ReplicationLogEntry>,
    /// Commit index
    commit_index: u64,
    /// Next index
    next_index: u64,
}

impl ReplicationLog {
    /// Create new log
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            commit_index: 0,
            next_index: 1,
        }
    }

    /// Append entry
    pub fn append(&mut self, epoch: Epoch, operation: ReplicationOperation) -> u64 {
        let index = self.next_index;
        self.next_index += 1;

        self.entries.push(ReplicationLogEntry {
            index,
            epoch,
            operation,
            timestamp: 0,
            committed: false,
        });

        index
    }

    /// Commit up to index
    pub fn commit(&mut self, up_to: u64) {
        for entry in &mut self.entries {
            if entry.index <= up_to && !entry.committed {
                entry.committed = true;
            }
        }
        self.commit_index = up_to;
    }

    /// Get uncommitted entries
    pub fn uncommitted(&self) -> Vec<&ReplicationLogEntry> {
        self.entries.iter().filter(|e| !e.committed).collect()
    }

    /// Get entries from index
    pub fn from_index(&self, from: u64) -> Vec<&ReplicationLogEntry> {
        self.entries.iter().filter(|e| e.index >= from).collect()
    }

    /// Last index
    pub fn last_index(&self) -> u64 {
        self.entries.last().map(|e| e.index).unwrap_or(0)
    }
}

impl Default for ReplicationLog {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// REPLICATION MANAGER
// ============================================================================

/// Replication manager
pub struct ReplicationManager {
    /// Local node ID
    node_id: NodeId,
    /// Segments
    segments: BTreeMap<SegmentId, CodeSegment>,
    /// Replicas
    replicas: BTreeMap<ReplicaId, Replica>,
    /// Segment to replicas mapping
    segment_replicas: BTreeMap<SegmentId, Vec<ReplicaId>>,
    /// Replication log
    log: ReplicationLog,
    /// Strategy
    strategy: ReplicationStrategy,
    /// Placement
    placement: Box<dyn ReplicaPlacement>,
    /// Configuration
    config: ReplicationConfig,
    /// Running
    running: AtomicBool,
    /// Statistics
    stats: ReplicationStats,
}

/// Replication configuration
#[derive(Debug, Clone)]
pub struct ReplicationConfig {
    /// Replication factor
    pub replication_factor: usize,
    /// Sync interval (ms)
    pub sync_interval: u64,
    /// Repair interval (ms)
    pub repair_interval: u64,
    /// Chunk size
    pub chunk_size: usize,
    /// Enable compression
    pub compression: bool,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            replication_factor: 3,
            sync_interval: 1000,
            repair_interval: 60000,
            chunk_size: 64 * 1024,
            compression: true,
        }
    }
}

/// Replication statistics
#[derive(Debug, Clone, Default)]
pub struct ReplicationStats {
    /// Segments created
    pub segments_created: u64,
    /// Segments updated
    pub segments_updated: u64,
    /// Segments deleted
    pub segments_deleted: u64,
    /// Replicas created
    pub replicas_created: u64,
    /// Bytes replicated
    pub bytes_replicated: u64,
    /// Sync operations
    pub sync_operations: u64,
    /// Repair operations
    pub repair_operations: u64,
}

impl ReplicationManager {
    /// Create new replication manager
    pub fn new(node_id: NodeId, config: ReplicationConfig) -> Self {
        Self {
            node_id,
            segments: BTreeMap::new(),
            replicas: BTreeMap::new(),
            segment_replicas: BTreeMap::new(),
            log: ReplicationLog::new(),
            strategy: ReplicationStrategy::default(),
            placement: Box::new(RandomPlacement::new()),
            config,
            running: AtomicBool::new(false),
            stats: ReplicationStats::default(),
        }
    }

    /// Start the manager
    pub fn start(&self) {
        self.running.store(true, Ordering::Release);
    }

    /// Stop the manager
    pub fn stop(&self) {
        self.running.store(false, Ordering::Release);
    }

    /// Create segment
    pub fn create_segment(
        &mut self,
        name: impl Into<String>,
        code: Vec<u8>,
        dependencies: Vec<SegmentId>,
    ) -> SegmentId {
        let id = SegmentId::generate();
        let hash = Self::hash_code(&code);

        let segment = CodeSegment {
            id,
            name: name.into(),
            code: code.clone(),
            version: 1,
            hash,
            dependencies,
            metadata: SegmentMetadata {
                size: code.len(),
                created_at: 0,
                last_modified: 0,
                origin: None,
                checksum: hash,
            },
        };

        self.segments.insert(id, segment.clone());
        self.segment_replicas.insert(id, Vec::new());

        // Log operation
        self.log
            .append(Epoch(0), ReplicationOperation::Create(segment));

        self.stats.segments_created += 1;
        self.stats.bytes_replicated += code.len() as u64;

        id
    }

    /// Update segment
    pub fn update_segment(
        &mut self,
        id: SegmentId,
        new_code: Vec<u8>,
    ) -> Result<u64, ReplicationError> {
        let segment = self
            .segments
            .get_mut(&id)
            .ok_or(ReplicationError::SegmentNotFound(id))?;

        segment.code = new_code.clone();
        segment.version += 1;
        segment.hash = Self::hash_code(&new_code);
        segment.metadata.size = new_code.len();
        segment.metadata.last_modified = 0;

        let new_version = segment.version;

        // Log operation
        self.log.append(Epoch(0), ReplicationOperation::Update {
            segment_id: id,
            new_code,
            new_version,
        });

        self.stats.segments_updated += 1;

        Ok(new_version)
    }

    /// Delete segment
    pub fn delete_segment(&mut self, id: SegmentId) -> Result<(), ReplicationError> {
        if !self.segments.contains_key(&id) {
            return Err(ReplicationError::SegmentNotFound(id));
        }

        self.segments.remove(&id);

        // Remove replicas
        if let Some(replica_ids) = self.segment_replicas.remove(&id) {
            for replica_id in replica_ids {
                self.replicas.remove(&replica_id);
            }
        }

        // Log operation
        self.log.append(Epoch(0), ReplicationOperation::Delete(id));

        self.stats.segments_deleted += 1;

        Ok(())
    }

    /// Add replica
    pub fn add_replica(
        &mut self,
        segment_id: SegmentId,
        node_id: NodeId,
    ) -> Result<ReplicaId, ReplicationError> {
        let segment = self
            .segments
            .get(&segment_id)
            .ok_or(ReplicationError::SegmentNotFound(segment_id))?;

        let replica_id = ReplicaId::generate();
        let replica = Replica {
            id: replica_id,
            segment_id,
            node_id,
            version: segment.version,
            status: ReplicaStatus::Syncing,
            last_sync: 0,
        };

        self.replicas.insert(replica_id, replica);

        self.segment_replicas
            .entry(segment_id)
            .or_default()
            .push(replica_id);

        // Log operation
        self.log.append(Epoch(0), ReplicationOperation::AddReplica {
            segment_id,
            node_id,
        });

        self.stats.replicas_created += 1;

        Ok(replica_id)
    }

    /// Replicate segment to nodes
    pub fn replicate(
        &mut self,
        segment_id: SegmentId,
        nodes: &[NodeId],
    ) -> Result<Vec<ReplicaId>, ReplicationError> {
        let mut replicas = Vec::new();

        for &node_id in nodes {
            if node_id != self.node_id {
                let id = self.add_replica(segment_id, node_id)?;
                replicas.push(id);
            }
        }

        Ok(replicas)
    }

    /// Auto-replicate with placement strategy
    pub fn auto_replicate(
        &mut self,
        segment_id: SegmentId,
        available_nodes: &[NodeId],
    ) -> Result<Vec<ReplicaId>, ReplicationError> {
        let segment = self
            .segments
            .get(&segment_id)
            .ok_or(ReplicationError::SegmentNotFound(segment_id))?
            .clone();

        let existing_replicas = self
            .segment_replicas
            .get(&segment_id)
            .map(|r| r.len())
            .unwrap_or(0);

        let needed = self
            .config
            .replication_factor
            .saturating_sub(existing_replicas + 1);

        if needed == 0 {
            return Ok(Vec::new());
        }

        // Filter out nodes that already have replicas
        let replica_nodes: Vec<NodeId> = self
            .segment_replicas
            .get(&segment_id)
            .map(|replica_ids| {
                replica_ids
                    .iter()
                    .filter_map(|id| self.replicas.get(id))
                    .map(|r| r.node_id)
                    .collect()
            })
            .unwrap_or_default();

        let available: Vec<NodeId> = available_nodes
            .iter()
            .filter(|n| **n != self.node_id && !replica_nodes.contains(n))
            .copied()
            .collect();

        let selected = self.placement.select_nodes(&segment, &available, needed);

        self.replicate(segment_id, &selected)
    }

    /// Sync replica
    pub fn sync_replica(&mut self, replica_id: ReplicaId) -> Result<(), ReplicationError> {
        let replica = self
            .replicas
            .get_mut(&replica_id)
            .ok_or(ReplicationError::ReplicaNotFound(replica_id))?;

        let segment = self
            .segments
            .get(&replica.segment_id)
            .ok_or(ReplicationError::SegmentNotFound(replica.segment_id))?;

        if replica.version < segment.version {
            replica.version = segment.version;
            replica.status = ReplicaStatus::InSync;
            replica.last_sync = 0;
            self.stats.sync_operations += 1;
        }

        Ok(())
    }

    /// Check replica health
    pub fn check_health(&self, segment_id: SegmentId) -> ReplicaHealth {
        let segment = match self.segments.get(&segment_id) {
            Some(s) => s,
            None => return ReplicaHealth::Missing,
        };

        let replicas = self
            .segment_replicas
            .get(&segment_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.replicas.get(id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let in_sync = replicas
            .iter()
            .filter(|r| r.status == ReplicaStatus::InSync && r.version == segment.version)
            .count();

        let _total = replicas.len() + 1; // +1 for local

        if in_sync + 1 >= self.config.replication_factor {
            ReplicaHealth::Healthy
        } else if in_sync > 0 {
            ReplicaHealth::Degraded
        } else {
            ReplicaHealth::Critical
        }
    }

    /// Get segment
    pub fn get_segment(&self, id: SegmentId) -> Option<&CodeSegment> {
        self.segments.get(&id)
    }

    /// Get replicas for segment
    pub fn get_replicas(&self, segment_id: SegmentId) -> Vec<&Replica> {
        self.segment_replicas
            .get(&segment_id)
            .map(|ids| ids.iter().filter_map(|id| self.replicas.get(id)).collect())
            .unwrap_or_default()
    }

    /// Set placement strategy
    pub fn set_placement(&mut self, placement: Box<dyn ReplicaPlacement>) {
        self.placement = placement;
    }

    /// Set replication strategy
    pub fn set_strategy(&mut self, strategy: ReplicationStrategy) {
        self.strategy = strategy;
    }

    /// Get statistics
    pub fn stats(&self) -> &ReplicationStats {
        &self.stats
    }

    fn hash_code(code: &[u8]) -> u64 {
        let mut hash = 0u64;
        for b in code {
            hash = hash.wrapping_mul(31).wrapping_add(*b as u64);
        }
        hash
    }
}

impl Default for ReplicationManager {
    fn default() -> Self {
        Self::new(NodeId(0), ReplicationConfig::default())
    }
}

/// Replica health
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplicaHealth {
    /// All replicas healthy
    Healthy,
    /// Some replicas missing or stale
    Degraded,
    /// Critical - below minimum
    Critical,
    /// Segment missing
    Missing,
}

/// Replication error
#[derive(Debug)]
pub enum ReplicationError {
    /// Segment not found
    SegmentNotFound(SegmentId),
    /// Replica not found
    ReplicaNotFound(ReplicaId),
    /// Insufficient replicas
    InsufficientReplicas,
    /// Sync failed
    SyncFailed,
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_segment() {
        let mut manager = ReplicationManager::new(NodeId(1), ReplicationConfig::default());

        let id = manager.create_segment("test", vec![1, 2, 3], Vec::new());
        assert!(manager.get_segment(id).is_some());

        let segment = manager.get_segment(id).unwrap();
        assert_eq!(segment.code, vec![1, 2, 3]);
    }

    #[test]
    fn test_add_replica() {
        let mut manager = ReplicationManager::new(NodeId(1), ReplicationConfig::default());

        let seg_id = manager.create_segment("test", vec![1, 2, 3], Vec::new());
        let rep_id = manager.add_replica(seg_id, NodeId(2)).unwrap();

        let replicas = manager.get_replicas(seg_id);
        assert_eq!(replicas.len(), 1);
        assert_eq!(replicas[0].id, rep_id);
    }

    #[test]
    fn test_replica_health() {
        let mut manager = ReplicationManager::new(NodeId(1), ReplicationConfig {
            replication_factor: 2,
            ..Default::default()
        });

        let seg_id = manager.create_segment("test", vec![1, 2, 3], Vec::new());

        // Only local copy
        let health = manager.check_health(seg_id);
        assert_eq!(health, ReplicaHealth::Degraded);

        // Add replica
        let rep_id = manager.add_replica(seg_id, NodeId(2)).unwrap();
        manager.sync_replica(rep_id).unwrap();

        let health = manager.check_health(seg_id);
        assert_eq!(health, ReplicaHealth::Healthy);
    }

    #[test]
    fn test_random_placement() {
        let placement = RandomPlacement::new();
        let nodes = vec![NodeId(1), NodeId(2), NodeId(3), NodeId(4)];

        let segment = CodeSegment {
            id: SegmentId(1),
            name: String::from("test"),
            code: Vec::new(),
            version: 1,
            hash: 0,
            dependencies: Vec::new(),
            metadata: SegmentMetadata {
                size: 0,
                created_at: 0,
                last_modified: 0,
                origin: None,
                checksum: 0,
            },
        };

        let selected = placement.select_nodes(&segment, &nodes, 2);
        assert_eq!(selected.len(), 2);
    }
}
