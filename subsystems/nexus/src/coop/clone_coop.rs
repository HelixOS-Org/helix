// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Clone (cooperative process/thread cloning)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Clone sharing policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopCloneSharingPolicy {
    ShareNothing,
    ShareVm,
    ShareFd,
    ShareSignals,
    ShareAll,
    Custom,
}

/// Clone result
#[derive(Debug, Clone)]
pub struct CoopCloneResult {
    pub child_id: u64,
    pub shared_resources: u32,
    pub private_resources: u32,
    pub setup_us: u64,
}

/// Clone cooperation stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct CoopCloneStats {
    pub total_clones: u64,
    pub share_all: u64,
    pub share_nothing: u64,
    pub custom_sharing: u64,
    pub avg_shared_resources: u64,
}

/// Manager for cooperative clone operations
pub struct CoopCloneManager {
    results: Vec<CoopCloneResult>,
    policy_map: BTreeMap<u64, CoopCloneSharingPolicy>,
    next_id: u64,
    stats: CoopCloneStats,
}

impl CoopCloneManager {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            policy_map: BTreeMap::new(),
            next_id: 50000,
            stats: CoopCloneStats {
                total_clones: 0,
                share_all: 0,
                share_nothing: 0,
                custom_sharing: 0,
                avg_shared_resources: 0,
            },
        }
    }

    #[inline(always)]
    pub fn set_policy(&mut self, pid: u64, policy: CoopCloneSharingPolicy) {
        self.policy_map.insert(pid, policy);
    }

    pub fn clone_cooperative(&mut self, parent: u64) -> CoopCloneResult {
        let child = self.next_id;
        self.next_id += 1;
        let policy = self.policy_map.get(&parent).cloned().unwrap_or(CoopCloneSharingPolicy::ShareVm);
        let (shared, private) = match policy {
            CoopCloneSharingPolicy::ShareAll => (8, 0),
            CoopCloneSharingPolicy::ShareNothing => (0, 8),
            CoopCloneSharingPolicy::ShareVm => (3, 5),
            CoopCloneSharingPolicy::Custom => (4, 4),
            _ => (2, 6),
        };
        let result = CoopCloneResult {
            child_id: child,
            shared_resources: shared,
            private_resources: private,
            setup_us: (shared as u64) * 20 + (private as u64) * 50,
        };
        self.results.push(result.clone());
        self.stats.total_clones += 1;
        match policy {
            CoopCloneSharingPolicy::ShareAll => self.stats.share_all += 1,
            CoopCloneSharingPolicy::ShareNothing => self.stats.share_nothing += 1,
            CoopCloneSharingPolicy::Custom => self.stats.custom_sharing += 1,
            _ => {}
        }
        result
    }

    #[inline(always)]
    pub fn stats(&self) -> &CoopCloneStats {
        &self.stats
    }
}
