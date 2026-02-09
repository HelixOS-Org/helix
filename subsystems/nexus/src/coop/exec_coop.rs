// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Exec (cooperative process execution)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Cooperative exec phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopExecPhase {
    PathResolution,
    BinaryLoad,
    AddressSetup,
    AuxVecBuild,
    EntryTransfer,
    Complete,
    Failed,
}

/// Exec cooperation record
#[derive(Debug, Clone)]
pub struct CoopExecRecord {
    pub pid: u64,
    pub path: String,
    pub phase: CoopExecPhase,
    pub pages_mapped: u64,
    pub relocs_applied: u32,
    pub latency_us: u64,
}

/// Exec cooperation stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopExecStats {
    pub total_execs: u64,
    pub successful: u64,
    pub failed: u64,
    pub cached_loads: u64,
    pub avg_pages_mapped: u64,
    pub avg_relocs: u64,
}

/// Manager for cooperative exec operations
pub struct CoopExecManager {
    records: Vec<CoopExecRecord>,
    binary_cache: BTreeMap<u64, String>,
    stats: CoopExecStats,
}

impl CoopExecManager {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            binary_cache: BTreeMap::new(),
            stats: CoopExecStats {
                total_execs: 0,
                successful: 0,
                failed: 0,
                cached_loads: 0,
                avg_pages_mapped: 0,
                avg_relocs: 0,
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

    pub fn exec(&mut self, pid: u64, path: &str, pages: u64, relocs: u32) -> CoopExecPhase {
        self.stats.total_execs += 1;
        let hash = Self::hash_path(path);
        let cached = self.binary_cache.contains_key(&hash);
        if cached {
            self.stats.cached_loads += 1;
        }
        self.binary_cache.insert(hash, String::from(path));
        let record = CoopExecRecord {
            pid,
            path: String::from(path),
            phase: CoopExecPhase::Complete,
            pages_mapped: pages,
            relocs_applied: relocs,
            latency_us: if cached { 80 } else { 350 },
        };
        self.records.push(record);
        self.stats.successful += 1;
        CoopExecPhase::Complete
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopExecStats {
        &self.stats
    }
}
