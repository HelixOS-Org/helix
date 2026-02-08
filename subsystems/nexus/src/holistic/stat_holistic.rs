// SPDX-License-Identifier: GPL-2.0
//! Holistic stat — comprehensive stat analysis across all filesystem operations

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Holistic stat call type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticStatCall {
    Stat,
    Fstat,
    Lstat,
    Fstatat,
    Statx,
    Newfstatat,
}

/// Holistic stat file type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticStatFileType {
    Regular,
    Directory,
    Symlink,
    CharDev,
    BlockDev,
    Fifo,
    Socket,
}

/// Stat call pattern
#[derive(Debug, Clone)]
pub struct StatCallPattern {
    pub path_hash: u64,
    pub call_count: u64,
    pub avg_latency_ns: u64,
    pub total_latency_ns: u64,
    pub last_file_type: HolisticStatFileType,
}

impl StatCallPattern {
    pub fn new(path: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        Self {
            path_hash: h,
            call_count: 0,
            avg_latency_ns: 0,
            total_latency_ns: 0,
            last_file_type: HolisticStatFileType::Regular,
        }
    }

    pub fn record(&mut self, latency_ns: u64, ft: HolisticStatFileType) {
        self.call_count += 1;
        self.total_latency_ns += latency_ns;
        self.avg_latency_ns = self.total_latency_ns / self.call_count;
        self.last_file_type = ft;
    }
}

/// Holistic stat stats
#[derive(Debug, Clone)]
pub struct HolisticStatStats {
    pub total_calls: u64,
    pub by_call: BTreeMap<u8, u64>,
    pub total_latency_ns: u64,
    pub unique_paths: u64,
}

/// Main holistic stat
#[derive(Debug)]
pub struct HolisticStat {
    pub patterns: BTreeMap<u64, StatCallPattern>,
    pub stats: HolisticStatStats,
}

impl HolisticStat {
    pub fn new() -> Self {
        Self {
            patterns: BTreeMap::new(),
            stats: HolisticStatStats {
                total_calls: 0,
                by_call: BTreeMap::new(),
                total_latency_ns: 0,
                unique_paths: 0,
            },
        }
    }

    pub fn record(
        &mut self,
        path: &[u8],
        call: HolisticStatCall,
        latency_ns: u64,
        ft: HolisticStatFileType,
    ) {
        self.stats.total_calls += 1;
        self.stats.total_latency_ns += latency_ns;
        *self.stats.by_call.entry(call as u8).or_insert(0) += 1;
        let mut h: u64 = 0xcbf29ce484222325;
        for b in path {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        if !self.patterns.contains_key(&h) {
            self.stats.unique_paths += 1;
            self.patterns.insert(h, StatCallPattern::new(path));
        }
        if let Some(p) = self.patterns.get_mut(&h) {
            p.record(latency_ns, ft);
        }
    }

    pub fn avg_latency_ns(&self) -> u64 {
        if self.stats.total_calls == 0 {
            0
        } else {
            self.stats.total_latency_ns / self.stats.total_calls
        }
    }
}
