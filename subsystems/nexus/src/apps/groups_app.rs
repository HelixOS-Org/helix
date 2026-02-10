// SPDX-License-Identifier: GPL-2.0
//! App groups — getgroups/setgroups supplementary groups interface

extern crate alloc;

/// Groups operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupsOp {
    Getgroups,
    Setgroups,
}

/// Groups result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupsResult {
    Success,
    PermissionDenied,
    InvalidArg,
    TooMany,
    Error,
}

/// Groups record
#[derive(Debug, Clone)]
pub struct GroupsRecord {
    pub op: GroupsOp,
    pub result: GroupsResult,
    pub count: u32,
    pub pid: u32,
}

impl GroupsRecord {
    pub fn new(op: GroupsOp, count: u32) -> Self {
        Self {
            op,
            result: GroupsResult::Success,
            count,
            pid: 0,
        }
    }
}

/// Groups app stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct GroupsAppStats {
    pub total_ops: u64,
    pub getgroups: u64,
    pub setgroups: u64,
    pub errors: u64,
}

/// Main app groups
#[derive(Debug)]
pub struct AppGroups {
    pub stats: GroupsAppStats,
}

impl AppGroups {
    pub fn new() -> Self {
        Self {
            stats: GroupsAppStats {
                total_ops: 0,
                getgroups: 0,
                setgroups: 0,
                errors: 0,
            },
        }
    }

    #[inline]
    pub fn record(&mut self, rec: &GroupsRecord) {
        self.stats.total_ops += 1;
        match rec.op {
            GroupsOp::Getgroups => self.stats.getgroups += 1,
            GroupsOp::Setgroups => self.stats.setgroups += 1,
        }
        if rec.result != GroupsResult::Success {
            self.stats.errors += 1;
        }
    }
}
