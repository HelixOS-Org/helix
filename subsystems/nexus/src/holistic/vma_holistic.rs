// SPDX-License-Identifier: MIT
//! # Holistic VMA Optimization
//!
//! System-wide Virtual Memory Area analysis:
//! - Global VMA density monitoring
//! - System-wide address space layout scoring
//! - VMA count per-process ranking (too many = overhead)
//! - Global merge opportunity detection
//! - Address space template learning from workload classes

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmaHealth { Optimal, Acceptable, Fragmented, Overloaded }

impl VmaHealth {
    pub fn from_count(vma_count: u64) -> Self {
        if vma_count < 100 { Self::Optimal }
        else if vma_count < 500 { Self::Acceptable }
        else if vma_count < 2000 { Self::Fragmented }
        else { Self::Overloaded }
    }
}

#[derive(Debug, Clone)]
pub struct ProcessVmaProfile {
    pub pid: u64,
    pub vma_count: u64,
    pub total_mapped: u64,
    pub total_resident: u64,
    pub merge_opportunities: u64,
    pub avg_vma_size: u64,
    pub health: VmaHealth,
}

impl ProcessVmaProfile {
    pub fn overhead_ratio(&self) -> f64 {
        // VMA metadata overhead: ~200 bytes per VMA
        let overhead = self.vma_count * 200;
        overhead as f64 / self.total_mapped.max(1) as f64
    }
    pub fn residency(&self) -> f64 {
        if self.total_mapped == 0 { return 0.0; }
        self.total_resident as f64 / self.total_mapped as f64
    }
}

#[derive(Debug, Clone)]
pub struct WorkloadTemplate {
    pub class_hash: u64,
    pub typical_vma_count: u64,
    pub typical_layout: Vec<(u64, u64)>, // (relative_offset, size)
    pub sample_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct VmaHolisticStats {
    pub total_system_vmas: u64,
    pub avg_vma_per_process: f64,
    pub total_merge_opportunities: u64,
    pub overloaded_processes: u64,
    pub templates_learned: u64,
    pub system_vma_overhead_bytes: u64,
}

pub struct VmaHolisticManager {
    profiles: BTreeMap<u64, ProcessVmaProfile>,
    templates: BTreeMap<u64, WorkloadTemplate>,
    stats: VmaHolisticStats,
}

impl VmaHolisticManager {
    pub fn new() -> Self {
        Self {
            profiles: BTreeMap::new(),
            templates: BTreeMap::new(),
            stats: VmaHolisticStats::default(),
        }
    }

    pub fn update_profile(&mut self, profile: ProcessVmaProfile) {
        if profile.health == VmaHealth::Overloaded {
            self.stats.overloaded_processes += 1;
        }
        self.stats.total_merge_opportunities += profile.merge_opportunities;
        self.profiles.insert(profile.pid, profile);
        self.recompute_stats();
    }

    fn recompute_stats(&mut self) {
        let count = self.profiles.len() as u64;
        if count == 0 { return; }
        self.stats.total_system_vmas = self.profiles.values().map(|p| p.vma_count).sum();
        self.stats.avg_vma_per_process = self.stats.total_system_vmas as f64 / count as f64;
        self.stats.system_vma_overhead_bytes = self.stats.total_system_vmas * 200;
    }

    /// Find processes with most VMAs (potential overhead problems)
    pub fn top_vma_consumers(&self, n: usize) -> Vec<(u64, u64)> {
        let mut sorted: Vec<_> = self.profiles.iter()
            .map(|(&pid, p)| (pid, p.vma_count))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().take(n).collect()
    }

    /// Find processes with most merge opportunities
    pub fn top_merge_candidates(&self, n: usize) -> Vec<(u64, u64)> {
        let mut sorted: Vec<_> = self.profiles.iter()
            .map(|(&pid, p)| (pid, p.merge_opportunities))
            .filter(|&(_, m)| m > 0)
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.into_iter().take(n).collect()
    }

    /// Learn a layout template from observed workloads
    pub fn learn_template(&mut self, class_hash: u64, layout: Vec<(u64, u64)>) {
        let vma_count = layout.len() as u64;
        let template = self.templates.entry(class_hash).or_insert(WorkloadTemplate {
            class_hash, typical_vma_count: vma_count,
            typical_layout: layout.clone(), sample_count: 0,
        });
        template.sample_count += 1;
        // EMA update of typical count
        template.typical_vma_count = (template.typical_vma_count * 7 + vma_count) / 8;
        self.stats.templates_learned = self.templates.len() as u64;
    }

    /// Check if a process deviates from its expected template
    pub fn deviation_from_template(&self, pid: u64, class_hash: u64) -> Option<f64> {
        let profile = self.profiles.get(&pid)?;
        let template = self.templates.get(&class_hash)?;
        let expected = template.typical_vma_count;
        let actual = profile.vma_count;
        Some((actual as f64 - expected as f64).abs() / expected.max(1) as f64)
    }

    /// System-wide VMA density (VMAs per GB of mapped memory)
    pub fn system_vma_density(&self) -> f64 {
        let total_mapped: u64 = self.profiles.values().map(|p| p.total_mapped).sum();
        let gb = total_mapped as f64 / (1024.0 * 1024.0 * 1024.0);
        if gb < 0.001 { return 0.0; }
        self.stats.total_system_vmas as f64 / gb
    }

    pub fn profile(&self, pid: u64) -> Option<&ProcessVmaProfile> { self.profiles.get(&pid) }
    pub fn stats(&self) -> &VmaHolisticStats { &self.stats }
}
