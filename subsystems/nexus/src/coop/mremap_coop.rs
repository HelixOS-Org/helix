// SPDX-License-Identifier: MIT
//! # Cooperative Memory Remap
//!
//! Multi-process mremap coordination:
//! - Coordinated region growth/shrink across shared mappings
//! - Address space negotiation when remapping overlaps
//! - Cooperative ASLR re-randomization
//! - Batch remap operations for process groups
//! - Migration-aware remap for live process migration

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemapOp {
    Grow,
    Shrink,
    Move,
    Split,
}

#[derive(Debug, Clone)]
pub struct RemapRequest {
    pub request_id: u64,
    pub pid: u64,
    pub old_addr: u64,
    pub old_size: u64,
    pub new_size: u64,
    pub op: RemapOp,
    pub timestamp: u64,
    pub approved: bool,
}

impl RemapRequest {
    #[inline(always)]
    pub fn size_delta(&self) -> i64 {
        self.new_size as i64 - self.old_size as i64
    }
    #[inline(always)]
    pub fn is_expansion(&self) -> bool {
        self.new_size > self.old_size
    }
}

#[derive(Debug, Clone)]
pub struct AddressNegotiation {
    pub initiator: u64,
    pub affected_pids: Vec<u64>,
    pub proposed_range: (u64, u64),
    pub conflicts: Vec<(u64, u64, u64)>, // (pid, overlap_start, overlap_end)
    pub resolved: bool,
}

#[derive(Debug, Clone)]
pub struct BatchRemap {
    pub group_id: u64,
    pub requests: Vec<RemapRequest>,
    pub total_growth: i64,
    pub committed: bool,
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct MremapCoopStats {
    pub remaps_coordinated: u64,
    pub conflicts_resolved: u64,
    pub batch_remaps: u64,
    pub aslr_rerandomizations: u64,
    pub migration_remaps: u64,
    pub growth_denied: u64,
}

pub struct MremapCoopManager {
    pending: BTreeMap<u64, RemapRequest>,
    negotiations: Vec<AddressNegotiation>,
    batches: BTreeMap<u64, BatchRemap>,
    /// Address space reservations: pid → (start, end)
    reservations: BTreeMap<u64, Vec<(u64, u64)>>,
    next_id: u64,
    stats: MremapCoopStats,
}

impl MremapCoopManager {
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            negotiations: Vec::new(),
            batches: BTreeMap::new(),
            reservations: BTreeMap::new(),
            next_id: 1,
            stats: MremapCoopStats::default(),
        }
    }

    /// Submit a remap request
    pub fn submit_request(
        &mut self,
        pid: u64,
        old_addr: u64,
        old_size: u64,
        new_size: u64,
        op: RemapOp,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.pending.insert(id, RemapRequest {
            request_id: id,
            pid,
            old_addr,
            old_size,
            new_size,
            op,
            timestamp: now,
            approved: false,
        });
        id
    }

    /// Check if a remap conflicts with other processes
    pub fn check_conflicts(&self, request_id: u64) -> Vec<(u64, u64, u64)> {
        let req = match self.pending.get(&request_id) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let new_end = req.old_addr + req.new_size;
        let mut conflicts = Vec::new();

        for (&pid, ranges) in &self.reservations {
            if pid == req.pid {
                continue;
            }
            for &(start, end) in ranges {
                if req.old_addr < end && new_end > start {
                    conflicts.push((pid, start.max(req.old_addr), end.min(new_end)));
                }
            }
        }
        conflicts
    }

    /// Approve a remap request after conflict resolution
    pub fn approve(&mut self, request_id: u64) -> bool {
        if let Some(req) = self.pending.get_mut(&request_id) {
            req.approved = true;
            // Update reservation
            let pid = req.pid;
            let new_range = (req.old_addr, req.old_addr + req.new_size);
            let ranges = self.reservations.entry(pid).or_insert_with(Vec::new);
            ranges.retain(|&(s, _)| s != req.old_addr);
            ranges.push(new_range);
            self.stats.remaps_coordinated += 1;
            true
        } else {
            false
        }
    }

    /// Create a batch remap for a process group
    #[inline]
    pub fn create_batch(&mut self, group_id: u64, requests: Vec<RemapRequest>) -> u64 {
        let total: i64 = requests.iter().map(|r| r.size_delta()).sum();
        self.batches.insert(group_id, BatchRemap {
            group_id,
            requests,
            total_growth: total,
            committed: false,
        });
        self.stats.batch_remaps += 1;
        group_id
    }

    /// Commit a batch remap atomically
    pub fn commit_batch(&mut self, group_id: u64) -> bool {
        if let Some(batch) = self.batches.get_mut(&group_id) {
            if batch.committed {
                return false;
            }
            for req in &mut batch.requests {
                req.approved = true;
                let ranges = self.reservations.entry(req.pid).or_insert_with(Vec::new);
                ranges.retain(|&(s, _)| s != req.old_addr);
                ranges.push((req.old_addr, req.old_addr + req.new_size));
            }
            batch.committed = true;
            true
        } else {
            false
        }
    }

    /// Trigger ASLR re-randomization for a process group
    pub fn rerandomize_aslr(&mut self, pids: &[u64]) -> Vec<(u64, u64)> {
        let mut new_bases = Vec::new();
        let mut seed = 0xDEAD_BEEF_u64;
        for &pid in pids {
            // xorshift64
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let new_base = (seed % (1u64 << 47)) & !0xFFF;
            new_bases.push((pid, new_base));
        }
        self.stats.aslr_rerandomizations += pids.len() as u64;
        new_bases
    }

    #[inline(always)]
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
    #[inline(always)]
    pub fn stats(&self) -> &MremapCoopStats {
        &self.stats
    }
}
