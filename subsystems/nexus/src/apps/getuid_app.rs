// SPDX-License-Identifier: GPL-2.0
//! App getuid — getuid/geteuid/getresuid syscall interface

extern crate alloc;

/// Getuid variant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetuidVariant {
    Getuid,
    Geteuid,
    Getresuid,
}

/// Getuid result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GetuidResult {
    Success,
    Fault,
    Error,
}

/// Getuid record
#[derive(Debug, Clone)]
pub struct GetuidRecord {
    pub variant: GetuidVariant,
    pub result: GetuidResult,
    pub uid: u32,
    pub euid: u32,
    pub suid: u32,
}

impl GetuidRecord {
    pub fn new(variant: GetuidVariant) -> Self {
        Self {
            variant,
            result: GetuidResult::Success,
            uid: 0,
            euid: 0,
            suid: 0,
        }
    }
}

/// Getuid app stats
#[derive(Debug, Clone)]
pub struct GetuidAppStats {
    pub total_ops: u64,
    pub getuid_calls: u64,
    pub geteuid_calls: u64,
    pub getresuid_calls: u64,
}

/// Main app getuid
#[derive(Debug)]
pub struct AppGetuid {
    pub stats: GetuidAppStats,
}

impl AppGetuid {
    pub fn new() -> Self {
        Self {
            stats: GetuidAppStats {
                total_ops: 0,
                getuid_calls: 0,
                geteuid_calls: 0,
                getresuid_calls: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &GetuidRecord) {
        self.stats.total_ops += 1;
        match rec.variant {
            GetuidVariant::Getuid => self.stats.getuid_calls += 1,
            GetuidVariant::Geteuid => self.stats.geteuid_calls += 1,
            GetuidVariant::Getresuid => self.stats.getresuid_calls += 1,
        }
    }
}
