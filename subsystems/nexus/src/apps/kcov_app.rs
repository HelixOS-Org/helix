// SPDX-License-Identifier: GPL-2.0
//! Apps kcov_app — kernel code coverage tracking.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Coverage mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KcovMode {
    TracePC,
    TraceCmp,
    Disabled,
}

/// Coverage entry
#[derive(Debug, Clone)]
pub struct CoverageHit {
    pub pc: u64,
    pub hit_count: u64,
    pub first_seen: u64,
    pub last_seen: u64,
}

impl CoverageHit {
    pub fn new(pc: u64, now: u64) -> Self { Self { pc, hit_count: 1, first_seen: now, last_seen: now } }
    #[inline(always)]
    pub fn hit(&mut self, now: u64) { self.hit_count += 1; self.last_seen = now; }
}

/// Comparison entry
#[derive(Debug, Clone)]
pub struct CmpEntry {
    pub pc: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub size: u8,
    pub is_const: bool,
}

/// Kcov instance
#[derive(Debug)]
pub struct KcovInstance {
    pub id: u64,
    pub tid: u64,
    pub mode: KcovMode,
    pub coverage: BTreeMap<u64, CoverageHit>,
    pub comparisons: Vec<CmpEntry>,
    pub buffer_size: u32,
    pub enabled: bool,
    pub total_hits: u64,
}

impl KcovInstance {
    pub fn new(id: u64, tid: u64, buffer_size: u32) -> Self {
        Self { id, tid, mode: KcovMode::Disabled, coverage: BTreeMap::new(), comparisons: Vec::new(), buffer_size, enabled: false, total_hits: 0 }
    }

    #[inline(always)]
    pub fn enable(&mut self, mode: KcovMode) { self.mode = mode; self.enabled = true; }
    #[inline(always)]
    pub fn disable(&mut self) { self.mode = KcovMode::Disabled; self.enabled = false; }

    #[inline]
    pub fn trace_pc(&mut self, pc: u64, now: u64) {
        if !self.enabled || self.mode != KcovMode::TracePC { return; }
        self.total_hits += 1;
        if let Some(hit) = self.coverage.get_mut(&pc) { hit.hit(now); }
        else { self.coverage.insert(pc, CoverageHit::new(pc, now)); }
    }

    #[inline]
    pub fn trace_cmp(&mut self, pc: u64, arg1: u64, arg2: u64, size: u8) {
        if !self.enabled || self.mode != KcovMode::TraceCmp { return; }
        if self.comparisons.len() < self.buffer_size as usize {
            self.comparisons.push(CmpEntry { pc, arg1, arg2, size, is_const: false });
        }
    }

    #[inline(always)]
    pub fn unique_edges(&self) -> u32 { self.coverage.len() as u32 }
    #[inline(always)]
    pub fn coverage_density(&self) -> f64 { if self.total_hits == 0 { 0.0 } else { self.coverage.len() as f64 / self.total_hits as f64 } }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct KcovAppStats {
    pub total_instances: u32,
    pub enabled_instances: u32,
    pub total_edges: u64,
    pub total_hits: u64,
    pub total_comparisons: u64,
    pub avg_density: f64,
}

/// Main kcov app
pub struct AppKcov {
    instances: BTreeMap<u64, KcovInstance>,
    next_id: u64,
}

impl AppKcov {
    pub fn new() -> Self { Self { instances: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, tid: u64, buffer: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.instances.insert(id, KcovInstance::new(id, tid, buffer));
        id
    }

    #[inline(always)]
    pub fn enable(&mut self, id: u64, mode: KcovMode) { if let Some(i) = self.instances.get_mut(&id) { i.enable(mode); } }

    #[inline]
    pub fn stats(&self) -> KcovAppStats {
        let enabled = self.instances.values().filter(|i| i.enabled).count() as u32;
        let edges: u64 = self.instances.values().map(|i| i.unique_edges() as u64).sum();
        let hits: u64 = self.instances.values().map(|i| i.total_hits).sum();
        let cmps: u64 = self.instances.values().map(|i| i.comparisons.len() as u64).sum();
        let densities: Vec<f64> = self.instances.values().filter(|i| i.total_hits > 0).map(|i| i.coverage_density()).collect();
        let avg = if densities.is_empty() { 0.0 } else { densities.iter().sum::<f64>() / densities.len() as f64 };
        KcovAppStats { total_instances: self.instances.len() as u32, enabled_instances: enabled, total_edges: edges, total_hits: hits, total_comparisons: cmps, avg_density: avg }
    }
}
