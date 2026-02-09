// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Exec (holistic execution analysis)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Exec pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticExecPattern {
    ForkExec,
    ChainExec,
    ScriptInterpreter,
    SelfExec,
    BatchExec,
}

/// Exec analysis record
#[derive(Debug, Clone)]
pub struct HolisticExecRecord {
    pub pid: u64,
    pub path_hash: u64,
    pub pattern: HolisticExecPattern,
    pub load_pages: u64,
    pub relocation_count: u32,
    pub latency_us: u64,
}

/// Exec holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticExecStats {
    pub total_analyzed: u64,
    pub fork_exec_count: u64,
    pub chain_exec_count: u64,
    pub unique_binaries: u64,
    pub avg_load_pages: f64,
    pub cache_hit_ratio: f64,
}

/// Manager for holistic exec analysis
pub struct HolisticExecManager {
    records: Vec<HolisticExecRecord>,
    binary_frequency: LinearMap<u64, 64>,
    last_exec_pid: LinearMap<u64, 64>,
    stats: HolisticExecStats,
}

impl HolisticExecManager {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            binary_frequency: LinearMap::new(),
            last_exec_pid: LinearMap::new(),
            stats: HolisticExecStats {
                total_analyzed: 0,
                fork_exec_count: 0,
                chain_exec_count: 0,
                unique_binaries: 0,
                avg_load_pages: 0.0,
                cache_hit_ratio: 0.0,
            },
        }
    }

    fn hash_path(path: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn analyze_exec(&mut self, pid: u64, path: &str, pages: u64, relocs: u32, after_fork: bool) -> HolisticExecPattern {
        let hash = Self::hash_path(path);
        let freq = self.binary_frequency.entry(hash).or_insert(0);
        let was_new = *freq == 0;
        *freq += 1;
        if was_new {
            self.stats.unique_binaries += 1;
        }
        let pattern = if after_fork {
            self.stats.fork_exec_count += 1;
            HolisticExecPattern::ForkExec
        } else if self.last_exec_pid.get(pid).is_some() {
            self.stats.chain_exec_count += 1;
            HolisticExecPattern::ChainExec
        } else {
            HolisticExecPattern::ForkExec
        };
        self.last_exec_pid.insert(pid, hash);
        let record = HolisticExecRecord {
            pid,
            path_hash: hash,
            pattern,
            load_pages: pages,
            relocation_count: relocs,
            latency_us: pages * 2 + relocs as u64,
        };
        self.records.push(record);
        self.stats.total_analyzed += 1;
        pattern
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticExecStats {
        &self.stats
    }
}
