// SPDX-License-Identifier: GPL-2.0
//! Bridge quota_bridge — filesystem disk quota management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Quota type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaType {
    User,
    Group,
    Project,
}

/// Quota state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaState {
    Off,
    Running,
    Suspended,
    Error,
}

/// Quota limit enforcement
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaEnforcement {
    Soft,
    Hard,
    None,
}

/// Disk quota entry
#[derive(Debug, Clone)]
pub struct DiskQuota {
    pub id: u64,
    pub qtype: QuotaType,
    pub uid_gid: u32,
    pub blocks_used: u64,
    pub blocks_soft: u64,
    pub blocks_hard: u64,
    pub inodes_used: u64,
    pub inodes_soft: u64,
    pub inodes_hard: u64,
    pub grace_period_blocks: u64,
    pub grace_period_inodes: u64,
    pub block_grace_expires: u64,
    pub inode_grace_expires: u64,
    pub warnings_issued: u32,
}

impl DiskQuota {
    pub fn new(id: u64, qtype: QuotaType, uid_gid: u32) -> Self {
        Self {
            id, qtype, uid_gid,
            blocks_used: 0, blocks_soft: u64::MAX, blocks_hard: u64::MAX,
            inodes_used: 0, inodes_soft: u64::MAX, inodes_hard: u64::MAX,
            grace_period_blocks: 604_800_000_000_000, // 7 days ns
            grace_period_inodes: 604_800_000_000_000,
            block_grace_expires: 0, inode_grace_expires: 0,
            warnings_issued: 0,
        }
    }

    pub fn set_block_limits(&mut self, soft: u64, hard: u64) {
        self.blocks_soft = soft;
        self.blocks_hard = hard;
    }

    pub fn set_inode_limits(&mut self, soft: u64, hard: u64) {
        self.inodes_soft = soft;
        self.inodes_hard = hard;
    }

    pub fn check_block_alloc(&self, additional: u64) -> QuotaEnforcement {
        let new_total = self.blocks_used + additional;
        if new_total > self.blocks_hard { return QuotaEnforcement::Hard; }
        if new_total > self.blocks_soft { return QuotaEnforcement::Soft; }
        QuotaEnforcement::None
    }

    pub fn check_inode_alloc(&self) -> QuotaEnforcement {
        let new_total = self.inodes_used + 1;
        if new_total > self.inodes_hard { return QuotaEnforcement::Hard; }
        if new_total > self.inodes_soft { return QuotaEnforcement::Soft; }
        QuotaEnforcement::None
    }

    pub fn block_utilization(&self) -> f64 {
        if self.blocks_hard == u64::MAX || self.blocks_hard == 0 { return 0.0; }
        self.blocks_used as f64 / self.blocks_hard as f64
    }

    pub fn inode_utilization(&self) -> f64 {
        if self.inodes_hard == u64::MAX || self.inodes_hard == 0 { return 0.0; }
        self.inodes_used as f64 / self.inodes_hard as f64
    }
}

/// Quota violation event
#[derive(Debug, Clone)]
pub struct QuotaViolation {
    pub quota_id: u64,
    pub qtype: QuotaType,
    pub enforcement: QuotaEnforcement,
    pub resource: QuotaResource,
    pub amount: u64,
    pub timestamp: u64,
}

/// Resource type for violation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaResource {
    Blocks,
    Inodes,
}

/// Per-filesystem quota state
#[derive(Debug, Clone)]
pub struct FsQuotaState {
    pub fs_id: u64,
    pub state: QuotaState,
    pub user_quotas: bool,
    pub group_quotas: bool,
    pub project_quotas: bool,
    pub total_violations: u64,
}

impl FsQuotaState {
    pub fn new(fs_id: u64) -> Self {
        Self {
            fs_id, state: QuotaState::Off,
            user_quotas: false, group_quotas: false,
            project_quotas: false, total_violations: 0,
        }
    }
}

/// Bridge stats
#[derive(Debug, Clone)]
pub struct QuotaBridgeStats {
    pub total_quotas: u32,
    pub total_violations: u64,
    pub hard_violations: u64,
    pub over_soft_limit: u32,
    pub filesystems_tracked: u32,
}

/// Main quota bridge
pub struct BridgeQuota {
    quotas: BTreeMap<u64, DiskQuota>,
    fs_states: BTreeMap<u64, FsQuotaState>,
    violations: Vec<QuotaViolation>,
    next_id: u64,
    max_violations: usize,
}

impl BridgeQuota {
    pub fn new() -> Self {
        Self {
            quotas: BTreeMap::new(), fs_states: BTreeMap::new(),
            violations: Vec::new(), next_id: 1, max_violations: 4096,
        }
    }

    pub fn create_quota(&mut self, qtype: QuotaType, uid_gid: u32) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.quotas.insert(id, DiskQuota::new(id, qtype, uid_gid));
        id
    }

    pub fn set_block_limits(&mut self, id: u64, soft: u64, hard: u64) {
        if let Some(q) = self.quotas.get_mut(&id) { q.set_block_limits(soft, hard); }
    }

    pub fn check_alloc(&mut self, id: u64, blocks: u64, now: u64) -> QuotaEnforcement {
        let result = self.quotas.get(&id).map(|q| q.check_block_alloc(blocks)).unwrap_or(QuotaEnforcement::None);
        if result != QuotaEnforcement::None {
            if let Some(q) = self.quotas.get(&id) {
                if self.violations.len() >= self.max_violations { self.violations.drain(..self.max_violations / 4); }
                self.violations.push(QuotaViolation {
                    quota_id: id, qtype: q.qtype, enforcement: result,
                    resource: QuotaResource::Blocks, amount: blocks, timestamp: now,
                });
            }
        }
        result
    }

    pub fn stats(&self) -> QuotaBridgeStats {
        let hard = self.violations.iter().filter(|v| v.enforcement == QuotaEnforcement::Hard).count() as u64;
        let over_soft = self.quotas.values().filter(|q| q.blocks_used > q.blocks_soft || q.inodes_used > q.inodes_soft).count() as u32;
        QuotaBridgeStats {
            total_quotas: self.quotas.len() as u32,
            total_violations: self.violations.len() as u64,
            hard_violations: hard, over_soft_limit: over_soft,
            filesystems_tracked: self.fs_states.len() as u32,
        }
    }
}

// ============================================================================
// Merged from quota_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaV2Type {
    User,
    Group,
    Project,
}

/// Quota limits
#[derive(Debug)]
pub struct QuotaV2Limits {
    pub block_hard: u64,
    pub block_soft: u64,
    pub inode_hard: u64,
    pub inode_soft: u64,
    pub grace_period_s: u64,
}

impl QuotaV2Limits {
    pub fn new(bhard: u64, bsoft: u64, ihard: u64, isoft: u64) -> Self {
        Self { block_hard: bhard, block_soft: bsoft, inode_hard: ihard, inode_soft: isoft, grace_period_s: 604800 }
    }
}

/// Quota usage
#[derive(Debug)]
pub struct QuotaV2Usage {
    pub id: u64,
    pub quota_type: QuotaV2Type,
    pub owner_id: u32,
    pub limits: QuotaV2Limits,
    pub blocks_used: u64,
    pub inodes_used: u64,
    pub grace_deadline: u64,
    pub warnings_issued: u32,
}

impl QuotaV2Usage {
    pub fn new(id: u64, qt: QuotaV2Type, owner: u32, limits: QuotaV2Limits) -> Self {
        Self { id, quota_type: qt, owner_id: owner, limits, blocks_used: 0, inodes_used: 0, grace_deadline: 0, warnings_issued: 0 }
    }

    pub fn block_usage_ratio(&self) -> f64 { if self.limits.block_hard == 0 { 0.0 } else { self.blocks_used as f64 / self.limits.block_hard as f64 } }
    pub fn inode_usage_ratio(&self) -> f64 { if self.limits.inode_hard == 0 { 0.0 } else { self.inodes_used as f64 / self.limits.inode_hard as f64 } }
    pub fn over_soft_block(&self) -> bool { self.blocks_used > self.limits.block_soft }
    pub fn over_hard_block(&self) -> bool { self.blocks_used > self.limits.block_hard }
}

/// Stats
#[derive(Debug, Clone)]
pub struct QuotaV2BridgeStats {
    pub total_quotas: u32,
    pub user_quotas: u32,
    pub group_quotas: u32,
    pub over_soft: u32,
    pub over_hard: u32,
}

/// Main quota v2 bridge
pub struct BridgeQuotaV2 {
    quotas: BTreeMap<u64, QuotaV2Usage>,
    next_id: u64,
}

impl BridgeQuotaV2 {
    pub fn new() -> Self { Self { quotas: BTreeMap::new(), next_id: 1 } }

    pub fn create(&mut self, qt: QuotaV2Type, owner: u32, limits: QuotaV2Limits) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.quotas.insert(id, QuotaV2Usage::new(id, qt, owner, limits));
        id
    }

    pub fn update_usage(&mut self, id: u64, blocks: u64, inodes: u64) {
        if let Some(q) = self.quotas.get_mut(&id) { q.blocks_used = blocks; q.inodes_used = inodes; }
    }

    pub fn stats(&self) -> QuotaV2BridgeStats {
        let user = self.quotas.values().filter(|q| q.quota_type == QuotaV2Type::User).count() as u32;
        let group = self.quotas.values().filter(|q| q.quota_type == QuotaV2Type::Group).count() as u32;
        let over_soft = self.quotas.values().filter(|q| q.over_soft_block()).count() as u32;
        let over_hard = self.quotas.values().filter(|q| q.over_hard_block()).count() as u32;
        QuotaV2BridgeStats { total_quotas: self.quotas.len() as u32, user_quotas: user, group_quotas: group, over_soft, over_hard }
    }
}
