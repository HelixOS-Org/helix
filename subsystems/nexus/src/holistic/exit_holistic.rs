// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Exit (holistic exit/cleanup analysis)

extern crate alloc;
use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Exit pattern classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticExitPattern {
    CleanShutdown,
    CrashExit,
    OomKill,
    SignalDeath,
    GroupExit,
    Cascade,
}

/// Exit analysis record
#[derive(Debug, Clone)]
pub struct HolisticExitRecord {
    pub pid: u64,
    pub exit_code: i32,
    pub pattern: HolisticExitPattern,
    pub cleanup_us: u64,
    pub resources_leaked: u32,
    pub orphans_created: u32,
}

/// Exit holistic stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct HolisticExitStats {
    pub total_analyzed: u64,
    pub clean_exits: u64,
    pub crash_exits: u64,
    pub oom_kills: u64,
    pub cascades: u64,
    pub total_orphans: u64,
    pub total_leaked: u64,
}

/// Manager for holistic exit analysis
pub struct HolisticExitManager {
    records: Vec<HolisticExitRecord>,
    exit_times: LinearMap<u64, 64>,
    stats: HolisticExitStats,
}

impl HolisticExitManager {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            exit_times: LinearMap::new(),
            stats: HolisticExitStats {
                total_analyzed: 0,
                clean_exits: 0,
                crash_exits: 0,
                oom_kills: 0,
                cascades: 0,
                total_orphans: 0,
                total_leaked: 0,
            },
        }
    }

    pub fn analyze_exit(&mut self, pid: u64, code: i32, cleanup_us: u64, leaked: u32, orphans: u32) -> HolisticExitPattern {
        let pattern = if code == 0 && leaked == 0 {
            self.stats.clean_exits += 1;
            HolisticExitPattern::CleanShutdown
        } else if code == 137 {
            self.stats.oom_kills += 1;
            HolisticExitPattern::OomKill
        } else if code < 0 {
            self.stats.crash_exits += 1;
            HolisticExitPattern::CrashExit
        } else if orphans > 10 {
            self.stats.cascades += 1;
            HolisticExitPattern::Cascade
        } else {
            self.stats.crash_exits += 1;
            HolisticExitPattern::SignalDeath
        };
        let record = HolisticExitRecord {
            pid,
            exit_code: code,
            pattern,
            cleanup_us,
            resources_leaked: leaked,
            orphans_created: orphans,
        };
        self.records.push(record);
        self.stats.total_analyzed += 1;
        self.stats.total_orphans += orphans as u64;
        self.stats.total_leaked += leaked as u64;
        self.exit_times.insert(pid, cleanup_us);
        pattern
    }

    #[inline(always)]
    pub fn stats(&self) -> &HolisticExitStats {
        &self.stats
    }
}
