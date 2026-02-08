// SPDX-License-Identifier: GPL-2.0
//! Coop AppArmor — cooperative AppArmor profile stacking

extern crate alloc;
use alloc::vec::Vec;

/// AppArmor coop event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppArmorCoopEvent {
    ProfileStack,
    ProfileInherit,
    HatChange,
    NamespaceCreate,
    PolicySync,
}

/// AppArmor coop record
#[derive(Debug, Clone)]
pub struct AppArmorCoopRecord {
    pub event: AppArmorCoopEvent,
    pub profile_hash: u64,
    pub ns_hash: u64,
    pub stack_depth: u32,
    pub pid: u32,
}

impl AppArmorCoopRecord {
    pub fn new(event: AppArmorCoopEvent, profile: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in profile { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { event, profile_hash: h, ns_hash: 0, stack_depth: 1, pid: 0 }
    }
}

/// AppArmor coop stats
#[derive(Debug, Clone)]
pub struct AppArmorCoopStats {
    pub total_events: u64,
    pub profile_stacks: u64,
    pub inherits: u64,
    pub policy_syncs: u64,
}

/// Main coop AppArmor
#[derive(Debug)]
pub struct CoopAppArmor {
    pub stats: AppArmorCoopStats,
}

impl CoopAppArmor {
    pub fn new() -> Self {
        Self { stats: AppArmorCoopStats { total_events: 0, profile_stacks: 0, inherits: 0, policy_syncs: 0 } }
    }

    pub fn record(&mut self, rec: &AppArmorCoopRecord) {
        self.stats.total_events += 1;
        match rec.event {
            AppArmorCoopEvent::ProfileStack => self.stats.profile_stacks += 1,
            AppArmorCoopEvent::ProfileInherit => self.stats.inherits += 1,
            AppArmorCoopEvent::PolicySync => self.stats.policy_syncs += 1,
            _ => {}
        }
    }
}
