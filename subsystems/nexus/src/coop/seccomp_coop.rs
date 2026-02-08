// SPDX-License-Identifier: GPL-2.0
//! Coop seccomp — cooperative seccomp filter coordination

extern crate alloc;
use alloc::vec::Vec;

/// Seccomp coop strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompCoopStrategy {
    Shared,
    Inherited,
    Independent,
    Synchronized,
}

/// Seccomp coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompCoopEvent {
    FilterSync,
    NotifForward,
    PolicyUpdate,
    GroupRestrict,
}

/// Seccomp coop record
#[derive(Debug, Clone)]
pub struct SeccompCoopRecord {
    pub event: SeccompCoopEvent,
    pub strategy: SeccompCoopStrategy,
    pub source_pid: u32,
    pub target_pid: u32,
    pub filter_count: u32,
}

impl SeccompCoopRecord {
    pub fn new(event: SeccompCoopEvent) -> Self {
        Self {
            event,
            strategy: SeccompCoopStrategy::Shared,
            source_pid: 0,
            target_pid: 0,
            filter_count: 0,
        }
    }
}

/// Seccomp coop stats
#[derive(Debug, Clone)]
pub struct SeccompCoopStats {
    pub total_events: u64,
    pub syncs: u64,
    pub forwards: u64,
    pub policy_updates: u64,
}

/// Main coop seccomp
#[derive(Debug)]
pub struct CoopSeccomp {
    pub stats: SeccompCoopStats,
}

impl CoopSeccomp {
    pub fn new() -> Self {
        Self {
            stats: SeccompCoopStats {
                total_events: 0,
                syncs: 0,
                forwards: 0,
                policy_updates: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &SeccompCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            SeccompCoopEvent::FilterSync => self.stats.syncs += 1,
            SeccompCoopEvent::NotifForward => self.stats.forwards += 1,
            SeccompCoopEvent::PolicyUpdate => self.stats.policy_updates += 1,
            _ => {},
        }
    }
}
