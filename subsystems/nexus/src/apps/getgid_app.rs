// SPDX-License-Identifier: GPL-2.0
//! App getgid — getgid/getegid/getresgid syscall interface

extern crate alloc;

/// Getgid variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetgidVariant {
    Getgid,
    Getegid,
    Getresgid,
}

/// Getgid result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetgidResult {
    Success,
    Fault,
    Error,
}

/// Getgid record
#[derive(Debug, Clone)]
pub struct GetgidRecord {
    pub variant: GetgidVariant,
    pub result: GetgidResult,
    pub gid: u32,
    pub egid: u32,
    pub sgid: u32,
}

impl GetgidRecord {
    pub fn new(variant: GetgidVariant) -> Self {
        Self {
            variant,
            result: GetgidResult::Success,
            gid: 0,
            egid: 0,
            sgid: 0,
        }
    }
}

/// Getgid app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct GetgidAppStats {
    pub total_ops: u64,
    pub getgid_calls: u64,
    pub getegid_calls: u64,
    pub getresgid_calls: u64,
}

/// Main app getgid
#[derive(Debug)]
pub struct AppGetgid {
    pub stats: GetgidAppStats,
}

impl AppGetgid {
    pub fn new() -> Self {
        Self {
            stats: GetgidAppStats {
                total_ops: 0,
                getgid_calls: 0,
                getegid_calls: 0,
                getresgid_calls: 0,
            },
        }
    }

    #[inline]
    pub fn record(&mut self, rec: &GetgidRecord) {
        self.stats.total_ops += 1;
        match rec.variant {
            GetgidVariant::Getgid => self.stats.getgid_calls += 1,
            GetgidVariant::Getegid => self.stats.getegid_calls += 1,
            GetgidVariant::Getresgid => self.stats.getresgid_calls += 1,
        }
    }
}
