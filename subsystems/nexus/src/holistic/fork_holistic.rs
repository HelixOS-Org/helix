// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Fork (holistic fork analysis)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Fork pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticForkPattern {
    Sequential,
    FanOut,
    TreeSpawn,
    ForkBomb,
    ServerModel,
    WorkerPool,
}

/// Fork analysis entry
#[derive(Debug, Clone)]
pub struct HolisticForkEntry {
    pub parent: u64,
    pub depth: u32,
    pub children_count: u32,
    pub pattern: HolisticForkPattern,
    pub cow_efficiency: f64,
    pub timestamp: u64,
}

/// Fork holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticForkStats {
    pub total_analyzed: u64,
    pub sequential_forks: u64,
    pub fan_out_forks: u64,
    pub bomb_detected: u64,
    pub avg_depth: f64,
    pub avg_cow_efficiency: f64,
}

/// Manager for holistic fork analysis
pub struct HolisticForkManager {
    entries: Vec<HolisticForkEntry>,
    tree_depth: LinearMap<u32, 64>,
    children_count: LinearMap<u32, 64>,
    stats: HolisticForkStats,
}

impl HolisticForkManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            tree_depth: LinearMap::new(),
            children_count: LinearMap::new(),
            stats: HolisticForkStats {
                total_analyzed: 0,
                sequential_forks: 0,
                fan_out_forks: 0,
                bomb_detected: 0,
                avg_depth: 0.0,
                avg_cow_efficiency: 0.0,
            },
        }
    }

    pub fn analyze_fork(&mut self, parent: u64, child: u64, cow_ratio: f64) -> HolisticForkPattern {
        let depth = self.tree_depth.get(parent).cloned().unwrap_or(0) + 1;
        self.tree_depth.insert(child, depth);
        let count = self.children_count.entry(parent).or_insert(0);
        *count += 1;
        let current_count = *count;
        let pattern = if current_count > 100 && depth > 10 {
            self.stats.bomb_detected += 1;
            HolisticForkPattern::ForkBomb
        } else if current_count > 8 {
            self.stats.fan_out_forks += 1;
            HolisticForkPattern::FanOut
        } else {
            self.stats.sequential_forks += 1;
            HolisticForkPattern::Sequential
        };
        let entry = HolisticForkEntry {
            parent,
            depth,
            children_count: current_count,
            pattern,
            cow_efficiency: cow_ratio,
            timestamp: self.stats.total_analyzed,
        };
        self.entries.push(entry);
        self.stats.total_analyzed += 1;
        let n = self.stats.total_analyzed as f64;
        self.stats.avg_cow_efficiency = (self.stats.avg_cow_efficiency * (n - 1.0) + cow_ratio) / n;
        pattern
    }

    #[inline(always)]
    pub fn max_depth(&self) -> u32 {
        self.tree_depth.values().cloned().max().unwrap_or(0)
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticForkStats {
        &self.stats
    }
}
