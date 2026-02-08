// SPDX-License-Identifier: GPL-2.0
//! NEXUS Holistic — Session (holistic session analysis)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Session health classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticSessionHealth {
    Healthy,
    NoTty,
    Zombie,
    Oversized,
    LeaderDead,
}

/// Session analysis entry
#[derive(Debug, Clone)]
pub struct HolisticSessionEntry {
    pub sid: u64,
    pub health: HolisticSessionHealth,
    pub group_count: u32,
    pub total_processes: u32,
    pub has_tty: bool,
    pub leader_alive: bool,
}

/// Session holistic stats
#[derive(Debug, Clone)]
pub struct HolisticSessionStats {
    pub total_analyzed: u64,
    pub healthy: u64,
    pub no_tty: u64,
    pub leader_dead: u64,
    pub avg_groups: f64,
    pub avg_processes: f64,
}

/// Manager for holistic session analysis
pub struct HolisticSessionManager {
    entries: BTreeMap<u64, HolisticSessionEntry>,
    stats: HolisticSessionStats,
}

impl HolisticSessionManager {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            stats: HolisticSessionStats {
                total_analyzed: 0,
                healthy: 0,
                no_tty: 0,
                leader_dead: 0,
                avg_groups: 0.0,
                avg_processes: 0.0,
            },
        }
    }

    pub fn analyze_session(&mut self, sid: u64, groups: u32, procs: u32, tty: bool, leader_alive: bool) -> HolisticSessionHealth {
        let health = if !leader_alive {
            self.stats.leader_dead += 1;
            HolisticSessionHealth::LeaderDead
        } else if !tty {
            self.stats.no_tty += 1;
            HolisticSessionHealth::NoTty
        } else {
            self.stats.healthy += 1;
            HolisticSessionHealth::Healthy
        };
        let entry = HolisticSessionEntry {
            sid,
            health,
            group_count: groups,
            total_processes: procs,
            has_tty: tty,
            leader_alive,
        };
        self.entries.insert(sid, entry);
        self.stats.total_analyzed += 1;
        let n = self.stats.total_analyzed as f64;
        self.stats.avg_groups = (self.stats.avg_groups * (n - 1.0) + groups as f64) / n;
        self.stats.avg_processes = (self.stats.avg_processes * (n - 1.0) + procs as f64) / n;
        health
    }

    pub fn stats(&self) -> &HolisticSessionStats {
        &self.stats
    }
}
