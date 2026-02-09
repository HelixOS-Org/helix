// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Thread (holistic thread analysis)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Thread usage pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticThreadPattern {
    WorkerPool,
    ProducerConsumer,
    PipelineStages,
    ForkJoin,
    SingleMain,
    ThreadBomb,
}

/// Thread analysis entry
#[derive(Debug, Clone)]
pub struct HolisticThreadEntry {
    pub tgid: u64,
    pub thread_count: u32,
    pub pattern: HolisticThreadPattern,
    pub cpu_utilization: f64,
    pub contention_ratio: f64,
}

/// Thread holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticThreadStats {
    pub total_analyzed: u64,
    pub worker_pools: u64,
    pub fork_joins: u64,
    pub thread_bombs: u64,
    pub avg_thread_count: f64,
    pub avg_utilization: f64,
}

/// Manager for holistic thread analysis
pub struct HolisticThreadManager {
    entries: Vec<HolisticThreadEntry>,
    group_counts: LinearMap<u32, 64>,
    stats: HolisticThreadStats,
}

impl HolisticThreadManager {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            group_counts: LinearMap::new(),
            stats: HolisticThreadStats {
                total_analyzed: 0,
                worker_pools: 0,
                fork_joins: 0,
                thread_bombs: 0,
                avg_thread_count: 0.0,
                avg_utilization: 0.0,
            },
        }
    }

    pub fn analyze_group(&mut self, tgid: u64, count: u32, util: f64, contention: f64) -> HolisticThreadPattern {
        self.group_counts.insert(tgid, count);
        let pattern = if count > 1000 {
            self.stats.thread_bombs += 1;
            HolisticThreadPattern::ThreadBomb
        } else if contention < 0.1 && count > 4 {
            self.stats.worker_pools += 1;
            HolisticThreadPattern::WorkerPool
        } else if contention > 0.5 {
            self.stats.fork_joins += 1;
            HolisticThreadPattern::ForkJoin
        } else if count == 1 {
            HolisticThreadPattern::SingleMain
        } else {
            HolisticThreadPattern::ProducerConsumer
        };
        let entry = HolisticThreadEntry {
            tgid,
            thread_count: count,
            pattern,
            cpu_utilization: util,
            contention_ratio: contention,
        };
        self.entries.push(entry);
        self.stats.total_analyzed += 1;
        let n = self.stats.total_analyzed as f64;
        self.stats.avg_thread_count = (self.stats.avg_thread_count * (n - 1.0) + count as f64) / n;
        self.stats.avg_utilization = (self.stats.avg_utilization * (n - 1.0) + util) / n;
        pattern
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticThreadStats {
        &self.stats
    }
}
