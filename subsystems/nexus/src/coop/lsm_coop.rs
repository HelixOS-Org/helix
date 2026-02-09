// SPDX-License-Identifier: GPL-2.0
//! Coop LSM — cooperative LSM hook coordination

extern crate alloc;
use alloc::vec::Vec;

/// LSM coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmCoopEvent {
    HookChain,
    PolicyMerge,
    DecisionAggregate,
    ModuleRegister,
    ModuleUnregister,
}

/// LSM coop policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmCoopPolicy {
    MostRestrictive,
    LeastRestrictive,
    FirstDeny,
    Unanimous,
}

/// LSM coop record
#[derive(Debug, Clone)]
pub struct LsmCoopRecord {
    pub event: LsmCoopEvent,
    pub policy: LsmCoopPolicy,
    pub module_count: u32,
    pub decisions_merged: u32,
    pub latency_ns: u64,
}

impl LsmCoopRecord {
    pub fn new(event: LsmCoopEvent) -> Self {
        Self {
            event,
            policy: LsmCoopPolicy::MostRestrictive,
            module_count: 0,
            decisions_merged: 0,
            latency_ns: 0,
        }
    }
}

/// LSM coop stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct LsmCoopStats {
    pub total_events: u64,
    pub hook_chains: u64,
    pub policy_merges: u64,
    pub modules_active: u32,
}

/// Main coop LSM
#[derive(Debug)]
pub struct CoopLsm {
    pub stats: LsmCoopStats,
}

impl CoopLsm {
    pub fn new() -> Self {
        Self {
            stats: LsmCoopStats {
                total_events: 0,
                hook_chains: 0,
                policy_merges: 0,
                modules_active: 0,
            },
        }
    }

    pub fn record(&mut self, rec: &LsmCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            LsmCoopEvent::HookChain => self.stats.hook_chains += 1,
            LsmCoopEvent::PolicyMerge | LsmCoopEvent::DecisionAggregate => {
                self.stats.policy_merges += 1
            },
            LsmCoopEvent::ModuleRegister => self.stats.modules_active += 1,
            LsmCoopEvent::ModuleUnregister => {
                if self.stats.modules_active > 0 {
                    self.stats.modules_active -= 1;
                }
            },
        }
    }
}
