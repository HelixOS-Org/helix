// SPDX-License-Identifier: MIT
//! # Cooperative Shared Memory Protocol
//!
//! Multi-process shared memory negotiation:
//! - Shared segment lifecycle management (create/attach/detach/destroy)
//! - Access permission negotiation between consumers
//! - Memory fence synchronization points
//! - Reader/writer lock-free coordination
//! - Segment migration between NUMA nodes

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShmRole {
    Owner,
    Reader,
    Writer,
    ReadWriter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FenceType {
    Acquire,
    Release,
    AcqRel,
    SeqCst,
}

#[derive(Debug, Clone)]
pub struct ShmParticipant {
    pub pid: u64,
    pub role: ShmRole,
    pub attached_at: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub fence_count: u64,
    pub numa_node: u32,
}

#[derive(Debug, Clone)]
pub struct ShmSegment {
    pub segment_id: u64,
    pub owner: u64,
    pub size: u64,
    pub participants: Vec<ShmParticipant>,
    pub fence_epoch: u64,
    pub dirty: bool,
    pub numa_preferred: Option<u32>,
    pub created_at: u64,
}

impl ShmSegment {
    pub fn active_writers(&self) -> usize {
        self.participants
            .iter()
            .filter(|p| matches!(p.role, ShmRole::Writer | ShmRole::ReadWriter))
            .count()
    }

    pub fn should_migrate_numa(&self) -> Option<u32> {
        // If majority of participants are on a different NUMA node than preferred
        let preferred = self.numa_preferred?;
        let on_preferred = self
            .participants
            .iter()
            .filter(|p| p.numa_node == preferred)
            .count();
        if on_preferred * 2 < self.participants.len() {
            // Find majority node
            let mut node_counts: BTreeMap<u32, usize> = BTreeMap::new();
            for p in &self.participants {
                *node_counts.entry(p.numa_node).or_insert(0) += 1;
            }
            node_counts
                .into_iter()
                .max_by_key(|(_, c)| *c)
                .map(|(n, _)| n)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ShmCoopStats {
    pub active_segments: u64,
    pub total_attached: u64,
    pub fence_syncs: u64,
    pub numa_migrations: u64,
    pub permission_negotiations: u64,
}

pub struct ShmCoopManager {
    segments: BTreeMap<u64, ShmSegment>,
    /// pid → list of segment_ids they participate in
    pid_segments: BTreeMap<u64, Vec<u64>>,
    next_id: u64,
    stats: ShmCoopStats,
}

impl ShmCoopManager {
    pub fn new() -> Self {
        Self {
            segments: BTreeMap::new(),
            pid_segments: BTreeMap::new(),
            next_id: 1,
            stats: ShmCoopStats::default(),
        }
    }

    pub fn create_segment(&mut self, owner: u64, size: u64, numa_node: u32, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let participant = ShmParticipant {
            pid: owner,
            role: ShmRole::Owner,
            attached_at: now,
            bytes_read: 0,
            bytes_written: 0,
            fence_count: 0,
            numa_node,
        };
        self.segments.insert(id, ShmSegment {
            segment_id: id,
            owner,
            size,
            participants: alloc::vec![participant],
            fence_epoch: 0,
            dirty: false,
            numa_preferred: Some(numa_node),
            created_at: now,
        });
        self.pid_segments
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(id);
        self.stats.active_segments += 1;
        id
    }

    pub fn attach(
        &mut self,
        segment_id: u64,
        pid: u64,
        role: ShmRole,
        numa_node: u32,
        now: u64,
    ) -> bool {
        let seg = match self.segments.get_mut(&segment_id) {
            Some(s) => s,
            None => return false,
        };
        // Check permission: only one writer allowed unless role is ReadWriter
        if matches!(role, ShmRole::Writer) && seg.active_writers() > 0 {
            self.stats.permission_negotiations += 1;
            return false;
        }
        seg.participants.push(ShmParticipant {
            pid,
            role,
            attached_at: now,
            bytes_read: 0,
            bytes_written: 0,
            fence_count: 0,
            numa_node,
        });
        self.pid_segments
            .entry(pid)
            .or_insert_with(Vec::new)
            .push(segment_id);
        self.stats.total_attached += 1;
        true
    }

    pub fn detach(&mut self, segment_id: u64, pid: u64) -> bool {
        let seg = match self.segments.get_mut(&segment_id) {
            Some(s) => s,
            None => return false,
        };
        let before = seg.participants.len();
        seg.participants.retain(|p| p.pid != pid);
        if let Some(segs) = self.pid_segments.get_mut(&pid) {
            segs.retain(|&s| s != segment_id);
        }
        seg.participants.len() < before
    }

    /// Perform a fence synchronization
    pub fn fence(&mut self, segment_id: u64, _fence_type: FenceType) {
        if let Some(seg) = self.segments.get_mut(&segment_id) {
            seg.fence_epoch += 1;
            seg.dirty = false;
            for p in &mut seg.participants {
                p.fence_count += 1;
            }
            self.stats.fence_syncs += 1;
        }
    }

    /// Check and suggest NUMA migrations
    pub fn check_numa_migrations(&self) -> Vec<(u64, u32)> {
        let mut suggestions = Vec::new();
        for (id, seg) in &self.segments {
            if let Some(target_node) = seg.should_migrate_numa() {
                suggestions.push((*id, target_node));
            }
        }
        suggestions
    }

    pub fn segment(&self, id: u64) -> Option<&ShmSegment> {
        self.segments.get(&id)
    }
    pub fn stats(&self) -> &ShmCoopStats {
        &self.stats
    }
}
