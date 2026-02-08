//! # Cooperative Quota System
//!
//! Cooperative resource quota negotiation and sharing:
//! - Quota pools
//! - Quota lending/borrowing
//! - Burst quota
//! - Quota cascading
//! - Usage tracking and enforcement

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// QUOTA TYPES
// ============================================================================

/// Quota resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CoopQuotaResource {
    /// CPU time (ns per period)
    CpuTime,
    /// Memory (bytes)
    Memory,
    /// I/O operations per second
    IoOps,
    /// I/O bandwidth (bytes/s)
    IoBandwidth,
    /// Network bandwidth (bytes/s)
    NetBandwidth,
    /// File descriptors
    FileDescriptors,
    /// Threads
    Threads,
}

/// Quota state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopQuotaState {
    /// Within limits
    Normal,
    /// Approaching limit
    Warning,
    /// At limit
    AtLimit,
    /// Over limit (borrowing)
    Borrowing,
    /// Suspended (exceeded all limits)
    Suspended,
}

// ============================================================================
// QUOTA ENTRY
// ============================================================================

/// Individual quota entry
#[derive(Debug, Clone)]
pub struct QuotaEntry {
    /// Resource
    pub resource: CoopQuotaResource,
    /// Base allocation
    pub base: u64,
    /// Current usage
    pub usage: u64,
    /// Burst limit (above base, temporary)
    pub burst_limit: u64,
    /// Burst usage
    pub burst_usage: u64,
    /// Borrowed from others
    pub borrowed: u64,
    /// Lent to others
    pub lent: u64,
    /// Peak usage
    pub peak_usage: u64,
}

impl QuotaEntry {
    pub fn new(resource: CoopQuotaResource, base: u64) -> Self {
        Self {
            resource,
            base,
            usage: 0,
            burst_limit: base / 4, // 25% burst by default
            burst_usage: 0,
            borrowed: 0,
            lent: 0,
            peak_usage: 0,
        }
    }

    /// Effective limit (base + burst + borrowed - lent)
    pub fn effective_limit(&self) -> u64 {
        self.base
            .saturating_add(self.burst_limit)
            .saturating_add(self.borrowed)
            .saturating_sub(self.lent)
    }

    /// Available capacity
    pub fn available(&self) -> u64 {
        self.effective_limit().saturating_sub(self.usage)
    }

    /// Utilization (against effective limit)
    pub fn utilization(&self) -> f64 {
        let limit = self.effective_limit();
        if limit == 0 {
            return 0.0;
        }
        self.usage as f64 / limit as f64
    }

    /// State
    pub fn state(&self) -> CoopQuotaState {
        let util = self.utilization();
        if util > 1.0 {
            CoopQuotaState::Suspended
        } else if self.borrowed > 0 {
            CoopQuotaState::Borrowing
        } else if util > 0.95 {
            CoopQuotaState::AtLimit
        } else if util > 0.8 {
            CoopQuotaState::Warning
        } else {
            CoopQuotaState::Normal
        }
    }

    /// Consume
    pub fn consume(&mut self, amount: u64) -> bool {
        if self.usage + amount > self.effective_limit() {
            return false;
        }
        self.usage += amount;
        if self.usage > self.peak_usage {
            self.peak_usage = self.usage;
        }
        true
    }

    /// Release
    pub fn release(&mut self, amount: u64) {
        self.usage = self.usage.saturating_sub(amount);
    }

    /// Lendable (spare capacity beyond usage + safety margin)
    pub fn lendable(&self) -> u64 {
        let safety = self.base / 10; // 10% safety margin
        if self.usage + safety < self.base {
            self.base - self.usage - safety
        } else {
            0
        }
    }
}

// ============================================================================
// QUOTA POOL
// ============================================================================

/// A pool of shared quotas
#[derive(Debug, Clone)]
pub struct CoopQuotaPool {
    /// Pool ID
    pub id: u64,
    /// Members
    pub members: Vec<u64>,
    /// Shared resource
    pub resource: CoopQuotaResource,
    /// Total pool capacity
    pub total_capacity: u64,
    /// Per-member allocation
    pub allocations: BTreeMap<u64, u64>,
    /// Per-member usage
    pub usage: BTreeMap<u64, u64>,
}

impl CoopQuotaPool {
    pub fn new(id: u64, resource: CoopQuotaResource, capacity: u64) -> Self {
        Self {
            id,
            members: Vec::new(),
            resource,
            total_capacity: capacity,
            allocations: BTreeMap::new(),
            usage: BTreeMap::new(),
        }
    }

    /// Add member with allocation
    pub fn add_member(&mut self, pid: u64, allocation: u64) {
        if !self.members.contains(&pid) {
            self.members.push(pid);
        }
        self.allocations.insert(pid, allocation);
        self.usage.entry(pid).or_insert(0);
    }

    /// Remove member
    pub fn remove_member(&mut self, pid: u64) {
        self.members.retain(|&m| m != pid);
        self.allocations.remove(&pid);
        self.usage.remove(&pid);
    }

    /// Total allocated
    pub fn total_allocated(&self) -> u64 {
        self.allocations.values().sum()
    }

    /// Total used
    pub fn total_used(&self) -> u64 {
        self.usage.values().sum()
    }

    /// Pool utilization
    pub fn utilization(&self) -> f64 {
        if self.total_capacity == 0 {
            return 0.0;
        }
        self.total_used() as f64 / self.total_capacity as f64
    }

    /// Free capacity in pool
    pub fn free_capacity(&self) -> u64 {
        self.total_capacity.saturating_sub(self.total_used())
    }

    /// Record usage
    pub fn record_usage(&mut self, pid: u64, amount: u64) -> bool {
        let total_after = self.total_used() + amount;
        if total_after > self.total_capacity {
            return false;
        }
        *self.usage.entry(pid).or_insert(0) += amount;
        true
    }
}

// ============================================================================
// LENDING
// ============================================================================

/// Quota loan
#[derive(Debug, Clone)]
pub struct QuotaLoan {
    /// Loan ID
    pub id: u64,
    /// Lender PID
    pub lender: u64,
    /// Borrower PID
    pub borrower: u64,
    /// Resource
    pub resource: CoopQuotaResource,
    /// Amount
    pub amount: u64,
    /// Duration (ns)
    pub duration_ns: u64,
    /// Created at
    pub created_at: u64,
    /// Expires at
    pub expires_at: u64,
    /// Returned
    pub returned: bool,
}

impl QuotaLoan {
    pub fn new(
        id: u64,
        lender: u64,
        borrower: u64,
        resource: CoopQuotaResource,
        amount: u64,
        duration_ns: u64,
        now: u64,
    ) -> Self {
        Self {
            id,
            lender,
            borrower,
            resource,
            amount,
            duration_ns,
            created_at: now,
            expires_at: now + duration_ns,
            returned: false,
        }
    }

    /// Is expired
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }
}

// ============================================================================
// QUOTA MANAGER
// ============================================================================

/// Cooperative quota stats
#[derive(Debug, Clone, Default)]
pub struct CoopQuotaStats {
    /// Tracked processes
    pub process_count: usize,
    /// Active pools
    pub pool_count: usize,
    /// Active loans
    pub active_loans: usize,
    /// Total loans
    pub total_loans: u64,
    /// Total borrowed
    pub total_borrowed: u64,
    /// Quota violations
    pub violations: u64,
}

/// Cooperative quota manager
pub struct CoopQuotaManager {
    /// Per-process quotas
    quotas: BTreeMap<(u64, u8), QuotaEntry>,
    /// Pools
    pools: BTreeMap<u64, CoopQuotaPool>,
    /// Active loans
    loans: BTreeMap<u64, QuotaLoan>,
    /// Next IDs
    next_pool_id: u64,
    next_loan_id: u64,
    /// Stats
    stats: CoopQuotaStats,
}

impl CoopQuotaManager {
    pub fn new() -> Self {
        Self {
            quotas: BTreeMap::new(),
            pools: BTreeMap::new(),
            loans: BTreeMap::new(),
            next_pool_id: 1,
            next_loan_id: 1,
            stats: CoopQuotaStats::default(),
        }
    }

    /// Set quota for process
    pub fn set_quota(&mut self, pid: u64, resource: CoopQuotaResource, base: u64) {
        let key = (pid, resource as u8);
        self.quotas.insert(key, QuotaEntry::new(resource, base));
    }

    /// Consume quota
    pub fn consume(&mut self, pid: u64, resource: CoopQuotaResource, amount: u64) -> bool {
        let key = (pid, resource as u8);
        if let Some(entry) = self.quotas.get_mut(&key) {
            if entry.consume(amount) {
                return true;
            }
            self.stats.violations += 1;
        }
        false
    }

    /// Release quota
    pub fn release(&mut self, pid: u64, resource: CoopQuotaResource, amount: u64) {
        let key = (pid, resource as u8);
        if let Some(entry) = self.quotas.get_mut(&key) {
            entry.release(amount);
        }
    }

    /// Create pool
    pub fn create_pool(&mut self, resource: CoopQuotaResource, capacity: u64) -> u64 {
        let id = self.next_pool_id;
        self.next_pool_id += 1;
        self.pools
            .insert(id, CoopQuotaPool::new(id, resource, capacity));
        self.stats.pool_count = self.pools.len();
        id
    }

    /// Lend quota
    pub fn lend(
        &mut self,
        lender: u64,
        borrower: u64,
        resource: CoopQuotaResource,
        amount: u64,
        duration_ns: u64,
        now: u64,
    ) -> Option<u64> {
        let lender_key = (lender, resource as u8);
        let borrower_key = (borrower, resource as u8);

        // Check lendable
        let lendable = self.quotas.get(&lender_key)?.lendable();
        if amount > lendable {
            return None;
        }

        // Create loan
        let id = self.next_loan_id;
        self.next_loan_id += 1;

        self.quotas.get_mut(&lender_key)?.lent += amount;
        if let Some(borrower_quota) = self.quotas.get_mut(&borrower_key) {
            borrower_quota.borrowed += amount;
        }

        self.loans.insert(
            id,
            QuotaLoan::new(id, lender, borrower, resource, amount, duration_ns, now),
        );
        self.stats.total_loans += 1;
        self.stats.total_borrowed += amount;
        self.stats.active_loans = self.loans.values().filter(|l| !l.returned).count();
        Some(id)
    }

    /// Return loan
    pub fn return_loan(&mut self, loan_id: u64) {
        if let Some(loan) = self.loans.get_mut(&loan_id) {
            if !loan.returned {
                loan.returned = true;
                let lender_key = (loan.lender, loan.resource as u8);
                let borrower_key = (loan.borrower, loan.resource as u8);
                if let Some(q) = self.quotas.get_mut(&lender_key) {
                    q.lent = q.lent.saturating_sub(loan.amount);
                }
                if let Some(q) = self.quotas.get_mut(&borrower_key) {
                    q.borrowed = q.borrowed.saturating_sub(loan.amount);
                }
            }
        }
        self.stats.active_loans = self.loans.values().filter(|l| !l.returned).count();
    }

    /// Check expired loans
    pub fn check_expired_loans(&mut self, now: u64) {
        let expired: Vec<u64> = self
            .loans
            .iter()
            .filter(|(_, l)| !l.returned && l.is_expired(now))
            .map(|(&id, _)| id)
            .collect();
        for id in expired {
            self.return_loan(id);
        }
    }

    /// Get quota entry
    pub fn quota(&self, pid: u64, resource: CoopQuotaResource) -> Option<&QuotaEntry> {
        self.quotas.get(&(pid, resource as u8))
    }

    /// Stats
    pub fn stats(&self) -> &CoopQuotaStats {
        &self.stats
    }
}

// ============================================================================
// Merged from quota_v2_coop
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopQuotaV2Type {
    User,
    Group,
    Project,
}

/// Quota enforcement level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopQuotaV2Enforcement {
    None,
    SoftLimit,
    HardLimit,
    GracePeriod,
}

/// Cooperative quota entry
#[derive(Debug, Clone)]
pub struct CoopQuotaV2Entry {
    pub quota_id: u64,
    pub quota_type: CoopQuotaV2Type,
    pub owner_id: u32,
    pub block_soft_limit: u64,
    pub block_hard_limit: u64,
    pub inode_soft_limit: u64,
    pub inode_hard_limit: u64,
    pub blocks_used: u64,
    pub inodes_used: u64,
    pub grace_period: u64,
    pub enforcement: CoopQuotaV2Enforcement,
}

/// Stats for quota cooperation
#[derive(Debug, Clone)]
pub struct CoopQuotaV2Stats {
    pub total_quotas: u64,
    pub over_soft_limit: u64,
    pub over_hard_limit: u64,
    pub warnings_issued: u64,
    pub allocations_denied: u64,
}

/// Manager for quota cooperative operations
pub struct CoopQuotaV2Manager {
    quotas: BTreeMap<u64, CoopQuotaV2Entry>,
    owner_index: BTreeMap<u64, u64>,
    next_id: u64,
    stats: CoopQuotaV2Stats,
}

impl CoopQuotaV2Manager {
    pub fn new() -> Self {
        Self {
            quotas: BTreeMap::new(),
            owner_index: BTreeMap::new(),
            next_id: 1,
            stats: CoopQuotaV2Stats {
                total_quotas: 0,
                over_soft_limit: 0,
                over_hard_limit: 0,
                warnings_issued: 0,
                allocations_denied: 0,
            },
        }
    }

    fn owner_key(quota_type: CoopQuotaV2Type, owner_id: u32) -> u64 {
        let type_bits = match quota_type {
            CoopQuotaV2Type::User => 0u64,
            CoopQuotaV2Type::Group => 1u64,
            CoopQuotaV2Type::Project => 2u64,
        };
        (type_bits << 32) | owner_id as u64
    }

    pub fn set_quota(&mut self, quota_type: CoopQuotaV2Type, owner_id: u32, block_soft: u64, block_hard: u64, inode_soft: u64, inode_hard: u64) -> u64 {
        let key = Self::owner_key(quota_type, owner_id);
        let id = self.next_id;
        self.next_id += 1;
        let entry = CoopQuotaV2Entry {
            quota_id: id,
            quota_type,
            owner_id,
            block_soft_limit: block_soft,
            block_hard_limit: block_hard,
            inode_soft_limit: inode_soft,
            inode_hard_limit: inode_hard,
            blocks_used: 0,
            inodes_used: 0,
            grace_period: 604800,
            enforcement: CoopQuotaV2Enforcement::HardLimit,
        };
        self.quotas.insert(id, entry);
        self.owner_index.insert(key, id);
        self.stats.total_quotas += 1;
        id
    }

    pub fn charge_blocks(&mut self, quota_type: CoopQuotaV2Type, owner_id: u32, blocks: u64) -> bool {
        let key = Self::owner_key(quota_type, owner_id);
        if let Some(&qid) = self.owner_index.get(&key) {
            if let Some(q) = self.quotas.get_mut(&qid) {
                if q.blocks_used + blocks > q.block_hard_limit && q.block_hard_limit > 0 {
                    self.stats.allocations_denied += 1;
                    self.stats.over_hard_limit += 1;
                    return false;
                }
                q.blocks_used += blocks;
                if q.blocks_used > q.block_soft_limit && q.block_soft_limit > 0 {
                    self.stats.over_soft_limit += 1;
                    self.stats.warnings_issued += 1;
                }
                return true;
            }
        }
        true
    }

    pub fn release_blocks(&mut self, quota_type: CoopQuotaV2Type, owner_id: u32, blocks: u64) {
        let key = Self::owner_key(quota_type, owner_id);
        if let Some(&qid) = self.owner_index.get(&key) {
            if let Some(q) = self.quotas.get_mut(&qid) {
                q.blocks_used = q.blocks_used.saturating_sub(blocks);
            }
        }
    }

    pub fn stats(&self) -> &CoopQuotaV2Stats {
        &self.stats
    }
}
