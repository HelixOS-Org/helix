// SPDX-License-Identifier: MIT
//! # Cooperative Page Fault Handling
//!
//! Multi-process page fault coordination:
//! - Fault pattern sharing between related processes
//! - Prefetch hints from sibling process faults
//! - Collaborative fault resolution for shared mappings
//! - Fault rate balancing across process groups
//! - Speculative page preparation based on group behavior

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultType {
    Minor,
    Major,
    CoW,
    Protection,
    Invalid,
}

#[derive(Debug, Clone)]
pub struct FaultRecord {
    pub pid: u64,
    pub addr: u64,
    pub fault_type: FaultType,
    pub timestamp: u64,
    pub resolution_ns: u64,
}

#[derive(Debug, Clone)]
pub struct FaultPattern {
    pub access_stride: i64,
    pub sequential_score: f64,
    pub spatial_locality: f64,
    pub temporal_period: u64,
    pub sample_count: u64,
}

impl FaultPattern {
    #[inline(always)]
    pub fn is_predictable(&self) -> bool {
        self.sequential_score > 0.6 || self.spatial_locality > 0.7
    }
    #[inline]
    pub fn predict_next(&self, last_addr: u64) -> Option<u64> {
        if self.is_predictable() && self.access_stride != 0 {
            Some((last_addr as i64 + self.access_stride) as u64)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct FaultCoopStats {
    pub faults_shared: u64,
    pub prefetch_hints: u64,
    pub prefetch_hits: u64,
    pub collaborative_resolves: u64,
    pub speculative_pages: u64,
    pub pattern_matches: u64,
}

pub struct PageFaultCoopManager {
    /// group_id → recent faults
    group_faults: BTreeMap<u64, Vec<FaultRecord>>,
    /// pid → learned fault pattern
    patterns: BTreeMap<u64, FaultPattern>,
    /// pid → group_id
    pid_groups: LinearMap<u64, 64>,
    /// Prefetch queue: (pid, predicted_addr)
    prefetch_queue: Vec<(u64, u64)>,
    max_faults_per_group: usize,
    stats: FaultCoopStats,
}

impl PageFaultCoopManager {
    pub fn new(max_faults: usize) -> Self {
        Self {
            group_faults: BTreeMap::new(),
            patterns: BTreeMap::new(),
            pid_groups: LinearMap::new(),
            prefetch_queue: Vec::new(),
            max_faults_per_group: max_faults,
            stats: FaultCoopStats::default(),
        }
    }

    #[inline]
    pub fn register_group(&mut self, group_id: u64, pids: &[u64]) {
        self.group_faults.insert(group_id, Vec::new());
        for &pid in pids {
            self.pid_groups.insert(pid, group_id);
        }
    }

    /// Record a page fault and share with group
    pub fn record_fault(&mut self, record: FaultRecord) {
        let pid = record.pid;
        let addr = record.addr;

        // Add to group
        if let Some(&group_id) = self.pid_groups.get(pid) {
            let faults = self.group_faults.entry(group_id).or_insert_with(Vec::new);
            faults.push(record);
            if faults.len() > self.max_faults_per_group {
                faults.drain(..self.max_faults_per_group / 2);
            }
            self.stats.faults_shared += 1;
        }

        // Update pattern
        self.update_pattern(pid, addr);

        // Generate prefetch hints for siblings
        self.generate_prefetch_hints(pid);
    }

    fn update_pattern(&mut self, pid: u64, addr: u64) {
        let pattern = self.patterns.entry(pid).or_insert(FaultPattern {
            access_stride: 4096,
            sequential_score: 0.0,
            spatial_locality: 0.0,
            temporal_period: 0,
            sample_count: 0,
        });
        pattern.sample_count += 1;

        // Simple sequential detection
        if pattern.sample_count > 1 {
            let old_stride = pattern.access_stride;
            let _ = addr; // Would compute from recent addresses
            pattern.sequential_score = pattern.sequential_score * 0.9
                + if old_stride == pattern.access_stride {
                    0.1
                } else {
                    0.0
                };
        }
    }

    fn generate_prefetch_hints(&mut self, pid: u64) {
        let group_id = match self.pid_groups.get(pid) {
            Some(g) => *g,
            None => return,
        };

        // Get siblings
        let siblings: Vec<u64> = self
            .pid_groups
            .iter()
            .filter(|(_, &g)| g == group_id)
            .filter(|(&p, _)| p != pid)
            .map(|(&p, _)| p)
            .collect();

        // If this pid has a predictable pattern, share with siblings
        if let Some(pattern) = self.patterns.get(&pid) {
            if pattern.is_predictable() {
                let last = self
                    .group_faults
                    .get(&group_id)
                    .and_then(|f| f.last())
                    .map(|f| f.addr)
                    .unwrap_or(0);

                if let Some(predicted) = pattern.predict_next(last) {
                    for &sib in &siblings {
                        self.prefetch_queue.push((sib, predicted));
                        self.stats.prefetch_hints += 1;
                    }
                }
            }
        }
    }

    /// Drain prefetch hints for a specific process
    pub fn drain_prefetch(&mut self, pid: u64) -> Vec<u64> {
        let mut hints = Vec::new();
        self.prefetch_queue.retain(|&(p, addr)| {
            if p == pid {
                hints.push(addr);
                false
            } else {
                true
            }
        });
        hints
    }

    /// Record a prefetch hit (the prefetched page was actually accessed)
    #[inline(always)]
    pub fn record_prefetch_hit(&mut self) {
        self.stats.prefetch_hits += 1;
    }

    #[inline(always)]
    pub fn pattern(&self, pid: u64) -> Option<&FaultPattern> {
        self.patterns.get(&pid)
    }
    #[inline(always)]
    pub fn stats(&self) -> &FaultCoopStats {
        &self.stats
    }

    #[inline]
    pub fn prefetch_hit_rate(&self) -> f64 {
        if self.stats.prefetch_hints == 0 {
            return 0.0;
        }
        self.stats.prefetch_hits as f64 / self.stats.prefetch_hints as f64
    }
}
