// SPDX-License-Identifier: MIT
//! # Cooperative Memory Mapping
//!
//! Cross-process mmap coordination:
//! - Shared mapping deduplication across cooperating processes
//! - CoW page sharing negotiation
//! - Address space layout hints for neighboring processes
//! - Mapping conflict resolution protocol
//! - Lazy binding coordination for large mappings

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmapCoopAction {
    Share,
    Duplicate,
    CopyOnWrite,
    Reject,
}

#[derive(Debug, Clone)]
pub struct SharedMapping {
    pub mapping_id: u64,
    pub owner_pid: u64,
    pub participants: Vec<u64>,
    pub base_addr: u64,
    pub size: u64,
    pub cow_pages: u64,
    pub shared_pages: u64,
    pub creation_epoch: u64,
}

impl SharedMapping {
    pub fn participant_count(&self) -> usize {
        self.participants.len() + 1
    }
    pub fn savings_bytes(&self) -> u64 {
        self.shared_pages * 4096 * (self.participant_count() as u64 - 1).max(1)
    }
    pub fn cow_ratio(&self) -> f64 {
        let total = self.cow_pages + self.shared_pages;
        if total == 0 {
            return 0.0;
        }
        self.cow_pages as f64 / total as f64
    }
}

#[derive(Debug, Clone)]
pub struct MappingConflict {
    pub pid_a: u64,
    pub pid_b: u64,
    pub addr_start: u64,
    pub addr_end: u64,
    pub resolved: bool,
    pub resolution: MmapCoopAction,
}

#[derive(Debug, Clone, Default)]
pub struct MmapCoopStats {
    pub total_shared_mappings: u64,
    pub total_savings_bytes: u64,
    pub conflicts_resolved: u64,
    pub cow_faults_deferred: u64,
    pub dedup_merges: u64,
}

pub struct MmapCoopManager {
    mappings: BTreeMap<u64, SharedMapping>,
    /// Pending conflicts
    conflicts: Vec<MappingConflict>,
    /// file_hash → mapping_id for dedup
    file_dedup: BTreeMap<u64, u64>,
    next_id: u64,
    stats: MmapCoopStats,
}

impl MmapCoopManager {
    pub fn new() -> Self {
        Self {
            mappings: BTreeMap::new(),
            conflicts: Vec::new(),
            file_dedup: BTreeMap::new(),
            next_id: 1,
            stats: MmapCoopStats::default(),
        }
    }

    /// Try to find an existing shared mapping to join
    pub fn find_shared(&self, file_hash: u64) -> Option<&SharedMapping> {
        let id = self.file_dedup.get(&file_hash)?;
        self.mappings.get(id)
    }

    /// Register a new shared mapping
    pub fn register_mapping(
        &mut self,
        owner: u64,
        base: u64,
        size: u64,
        file_hash: u64,
        epoch: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.mappings.insert(id, SharedMapping {
            mapping_id: id,
            owner_pid: owner,
            participants: Vec::new(),
            base_addr: base,
            size,
            cow_pages: 0,
            shared_pages: size / 4096,
            creation_epoch: epoch,
        });
        self.file_dedup.insert(file_hash, id);
        self.stats.total_shared_mappings += 1;
        id
    }

    /// A process joins an existing shared mapping
    pub fn join_mapping(&mut self, mapping_id: u64, pid: u64) -> bool {
        if let Some(m) = self.mappings.get_mut(&mapping_id) {
            if !m.participants.contains(&pid) {
                m.participants.push(pid);
                self.stats.dedup_merges += 1;
                self.stats.total_savings_bytes += m.size;
                return true;
            }
        }
        false
    }

    /// Record a CoW fault on a shared mapping
    pub fn record_cow_fault(&mut self, mapping_id: u64) {
        if let Some(m) = self.mappings.get_mut(&mapping_id) {
            m.cow_pages += 1;
            m.shared_pages = m.shared_pages.saturating_sub(1);
            self.stats.cow_faults_deferred += 1;
        }
    }

    /// Detect address overlap conflicts between two processes
    pub fn detect_conflict(&mut self, pid_a: u64, pid_b: u64, start: u64, end: u64) {
        self.conflicts.push(MappingConflict {
            pid_a,
            pid_b,
            addr_start: start,
            addr_end: end,
            resolved: false,
            resolution: MmapCoopAction::Reject,
        });
    }

    /// Resolve pending conflicts
    pub fn resolve_conflicts(&mut self) -> usize {
        let mut resolved = 0;
        for conflict in &mut self.conflicts {
            if !conflict.resolved {
                // Default strategy: CoW for overlapping regions
                conflict.resolution = MmapCoopAction::CopyOnWrite;
                conflict.resolved = true;
                resolved += 1;
            }
        }
        self.stats.conflicts_resolved += resolved as u64;
        self.conflicts.retain(|c| !c.resolved);
        resolved
    }

    pub fn mapping(&self, id: u64) -> Option<&SharedMapping> {
        self.mappings.get(&id)
    }
    pub fn stats(&self) -> &MmapCoopStats {
        &self.stats
    }
}
