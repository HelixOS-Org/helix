// SPDX-License-Identifier: MIT
//! # Cooperative VMA Management
//!
//! Virtual Memory Area coordination across processes:
//! - Shared VMA tree for process groups
//! - Address space layout negotiation
//! - VMA inheritance protocol (fork/exec)
//! - Cross-process gap analysis for allocation hints
//! - Memory layout templates for process families

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmaInheritance {
    None,
    Copy,
    Share,
    ZeroCopy,
}

#[derive(Debug, Clone)]
pub struct CoopVma {
    pub start: u64,
    pub end: u64,
    pub owner: u64,
    pub inheritance: VmaInheritance,
    pub shared_with: Vec<u64>,
    pub fault_count: u64,
    pub resident: u64,
}

impl CoopVma {
    #[inline(always)]
    pub fn size(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }
    #[inline(always)]
    pub fn pages(&self) -> u64 {
        self.size() / 4096
    }
    #[inline(always)]
    pub fn is_shared(&self) -> bool {
        !self.shared_with.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct LayoutTemplate {
    pub name_hash: u64,
    pub regions: Vec<(u64, u64, VmaInheritance)>, // (offset, size, inheritance)
    pub total_size: u64,
    pub usage_count: u64,
}

#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct VmaCoopStats {
    pub shared_vmas: u64,
    pub inheritance_copies: u64,
    pub gap_hints_given: u64,
    pub layout_templates: u64,
    pub fork_optimizations: u64,
}

pub struct VmaCoopManager {
    /// group_id → list of cooperative VMAs
    group_vmas: BTreeMap<u64, Vec<CoopVma>>,
    /// template_hash → LayoutTemplate
    templates: BTreeMap<u64, LayoutTemplate>,
    /// pid → group_id mapping
    pid_groups: LinearMap<u64, 64>,
    stats: VmaCoopStats,
}

impl VmaCoopManager {
    pub fn new() -> Self {
        Self {
            group_vmas: BTreeMap::new(),
            templates: BTreeMap::new(),
            pid_groups: LinearMap::new(),
            stats: VmaCoopStats::default(),
        }
    }

    #[inline]
    pub fn create_group(&mut self, group_id: u64, pids: &[u64]) {
        self.group_vmas.insert(group_id, Vec::new());
        for &pid in pids {
            self.pid_groups.insert(pid, group_id);
        }
    }

    #[inline]
    pub fn add_vma(&mut self, group_id: u64, vma: CoopVma) {
        if vma.is_shared() {
            self.stats.shared_vmas += 1;
        }
        let vmas = self.group_vmas.entry(group_id).or_insert_with(Vec::new);
        let pos = vmas.partition_point(|v| v.start < vma.start);
        vmas.insert(pos, vma);
    }

    /// Find gaps in a group's address space for new allocations
    pub fn find_gaps(&self, group_id: u64, min_size: u64) -> Vec<(u64, u64)> {
        let vmas = match self.group_vmas.get(&group_id) {
            Some(v) if v.len() >= 2 => v,
            _ => return Vec::new(),
        };

        let mut gaps = Vec::new();
        for i in 1..vmas.len() {
            let gap_start = vmas[i - 1].end;
            let gap_end = vmas[i].start;
            let gap_size = gap_end.saturating_sub(gap_start);
            if gap_size >= min_size {
                gaps.push((gap_start, gap_size));
            }
        }
        self.stats.gap_hints_given.wrapping_add(gaps.len() as u64);
        gaps
    }

    /// Prepare VMA inheritance for fork
    pub fn prepare_fork(
        &mut self,
        parent_pid: u64,
        child_pid: u64,
    ) -> Vec<(u64, u64, VmaInheritance)> {
        let group_id = match self.pid_groups.get(parent_pid) {
            Some(g) => *g,
            None => return Vec::new(),
        };
        self.pid_groups.insert(child_pid, group_id);

        let mut inherited = Vec::new();
        if let Some(vmas) = self.group_vmas.get_mut(&group_id) {
            for vma in vmas.iter_mut() {
                if vma.owner == parent_pid {
                    match vma.inheritance {
                        VmaInheritance::Share => {
                            vma.shared_with.push(child_pid);
                            inherited.push((vma.start, vma.size(), VmaInheritance::Share));
                        },
                        VmaInheritance::Copy => {
                            inherited.push((vma.start, vma.size(), VmaInheritance::Copy));
                            self.stats.inheritance_copies += 1;
                        },
                        VmaInheritance::ZeroCopy => {
                            // CoW semantics
                            vma.shared_with.push(child_pid);
                            inherited.push((vma.start, vma.size(), VmaInheritance::ZeroCopy));
                            self.stats.fork_optimizations += 1;
                        },
                        VmaInheritance::None => {},
                    }
                }
            }
        }
        inherited
    }

    /// Register a layout template learned from a process family
    #[inline(always)]
    pub fn register_template(&mut self, name_hash: u64, template: LayoutTemplate) {
        self.templates.insert(name_hash, template);
        self.stats.layout_templates += 1;
    }

    /// Apply a known template to a new process
    #[inline]
    pub fn apply_template(&mut self, name_hash: u64) -> Option<&LayoutTemplate> {
        if let Some(t) = self.templates.get_mut(&name_hash) {
            t.usage_count += 1;
            Some(t)
        } else {
            None
        }
    }

    #[inline]
    pub fn group_vmas(&self, group_id: u64) -> &[CoopVma] {
        self.group_vmas
            .get(&group_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    #[inline(always)]
    pub fn stats(&self) -> &VmaCoopStats {
        &self.stats
    }
}
