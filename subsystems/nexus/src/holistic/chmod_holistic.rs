// SPDX-License-Identifier: GPL-2.0
//! Holistic chmod — permission change analysis with security impact assessment

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Holistic chmod risk level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodRiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

/// Permission change record
#[derive(Debug, Clone)]
pub struct ChmodChangeRecord {
    pub inode: u64,
    pub old_mode: u32,
    pub new_mode: u32,
    pub uid: u32,
    pub risk: ChmodRiskLevel,
    pub timestamp_ns: u64,
}

impl ChmodChangeRecord {
    pub fn new(inode: u64, old_mode: u32, new_mode: u32) -> Self {
        let risk = if new_mode & 0o4000 != 0 && old_mode & 0o4000 == 0 {
            ChmodRiskLevel::Critical
        } else if new_mode & 0o002 != 0 && old_mode & 0o002 == 0 {
            ChmodRiskLevel::High
        } else if new_mode & 0o020 != 0 && old_mode & 0o020 == 0 {
            ChmodRiskLevel::Medium
        } else if new_mode != old_mode {
            ChmodRiskLevel::Low
        } else {
            ChmodRiskLevel::Safe
        };
        Self {
            inode,
            old_mode,
            new_mode,
            uid: 0,
            risk,
            timestamp_ns: 0,
        }
    }

    pub fn setuid_added(&self) -> bool {
        self.new_mode & 0o4000 != 0 && self.old_mode & 0o4000 == 0
    }
    pub fn setgid_added(&self) -> bool {
        self.new_mode & 0o2000 != 0 && self.old_mode & 0o2000 == 0
    }
    pub fn world_writable_added(&self) -> bool {
        self.new_mode & 0o002 != 0 && self.old_mode & 0o002 == 0
    }
}

/// Holistic chmod stats
#[derive(Debug, Clone)]
pub struct HolisticChmodStats {
    pub total_changes: u64,
    pub setuid_additions: u64,
    pub world_writable: u64,
    pub critical_risks: u64,
    pub high_risks: u64,
}

/// Main holistic chmod
#[derive(Debug)]
pub struct HolisticChmod {
    pub stats: HolisticChmodStats,
}

impl HolisticChmod {
    pub fn new() -> Self {
        Self {
            stats: HolisticChmodStats {
                total_changes: 0,
                setuid_additions: 0,
                world_writable: 0,
                critical_risks: 0,
                high_risks: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &ChmodChangeRecord) {
        self.stats.total_changes += 1;
        if rec.setuid_added() {
            self.stats.setuid_additions += 1;
        }
        if rec.world_writable_added() {
            self.stats.world_writable += 1;
        }
        match rec.risk {
            ChmodRiskLevel::Critical => self.stats.critical_risks += 1,
            ChmodRiskLevel::High => self.stats.high_risks += 1,
            _ => {},
        }
    }

    pub fn risk_rate(&self) -> f64 {
        if self.stats.total_changes == 0 {
            0.0
        } else {
            (self.stats.critical_risks + self.stats.high_risks) as f64
                / self.stats.total_changes as f64
        }
    }
}
