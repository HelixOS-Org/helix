//! # Cooperative Delegation Protocol
//!
//! Mechanism for processes to delegate capabilities and tasks:
//! - Capability delegation
//! - Work delegation
//! - Authority chains
//! - Revocation support
//! - Delegation auditing

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// DELEGATION TYPES
// ============================================================================

/// Delegation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DelegationType {
    /// Delegate CPU time
    CpuTime,
    /// Delegate memory quota
    MemoryQuota,
    /// Delegate I/O bandwidth
    IoBandwidth,
    /// Delegate network bandwidth
    NetworkBandwidth,
    /// Delegate a capability token
    Capability,
    /// Delegate scheduling priority
    Priority,
    /// Delegate file descriptor
    FileDescriptor,
}

/// Delegation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelegationState {
    /// Pending acceptance
    Pending,
    /// Active
    Active,
    /// Expired
    Expired,
    /// Revoked by delegator
    Revoked,
    /// Declined by delegate
    Declined,
}

/// Delegation constraint
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelegationConstraint {
    /// Cannot be re-delegated
    NonTransferable,
    /// Can be re-delegated up to N hops
    MaxHops(u8),
    /// Must be returned by deadline
    TimeLimited(u64),
    /// Usage-limited
    UsageLimited(u64),
}

// ============================================================================
// DELEGATION RECORD
// ============================================================================

/// A delegation record
#[derive(Debug, Clone)]
pub struct Delegation {
    /// Delegation id
    pub id: u64,
    /// Delegator (source)
    pub delegator: u64,
    /// Delegate (target)
    pub delegate: u64,
    /// Type
    pub delegation_type: DelegationType,
    /// Amount (type-specific)
    pub amount: u64,
    /// State
    pub state: DelegationState,
    /// Constraints
    pub constraints: Vec<DelegationConstraint>,
    /// Created at
    pub created_at: u64,
    /// Accepted at
    pub accepted_at: Option<u64>,
    /// Expires at
    pub expires_at: Option<u64>,
    /// Usage consumed
    pub usage: u64,
    /// Hop count (0 = direct)
    pub hop_count: u8,
    /// Parent delegation (if re-delegated)
    pub parent_id: Option<u64>,
}

impl Delegation {
    pub fn new(
        id: u64,
        delegator: u64,
        delegate: u64,
        dtype: DelegationType,
        amount: u64,
        now: u64,
    ) -> Self {
        Self {
            id,
            delegator,
            delegate,
            delegation_type: dtype,
            amount,
            state: DelegationState::Pending,
            constraints: Vec::new(),
            created_at: now,
            accepted_at: None,
            expires_at: None,
            usage: 0,
            hop_count: 0,
            parent_id: None,
        }
    }

    /// Add constraint
    #[inline]
    pub fn with_constraint(mut self, constraint: DelegationConstraint) -> Self {
        self.constraints.push(constraint);
        if let DelegationConstraint::TimeLimited(deadline) = constraint {
            self.expires_at = Some(self.created_at + deadline);
        }
        self
    }

    /// Accept delegation
    #[inline]
    pub fn accept(&mut self, now: u64) -> bool {
        if self.state != DelegationState::Pending {
            return false;
        }
        self.state = DelegationState::Active;
        self.accepted_at = Some(now);
        true
    }

    /// Decline delegation
    #[inline]
    pub fn decline(&mut self) -> bool {
        if self.state != DelegationState::Pending {
            return false;
        }
        self.state = DelegationState::Declined;
        true
    }

    /// Revoke delegation
    #[inline]
    pub fn revoke(&mut self) -> bool {
        if self.state != DelegationState::Active && self.state != DelegationState::Pending {
            return false;
        }
        self.state = DelegationState::Revoked;
        true
    }

    /// Consume usage
    pub fn consume(&mut self, amount: u64) -> bool {
        if self.state != DelegationState::Active {
            return false;
        }
        // Check usage limit
        for constraint in &self.constraints {
            if let DelegationConstraint::UsageLimited(max) = constraint {
                if self.usage + amount > *max {
                    return false;
                }
            }
        }
        self.usage += amount;
        true
    }

    /// Check if expired
    #[inline]
    pub fn check_expiry(&mut self, now: u64) -> bool {
        if let Some(expires) = self.expires_at {
            if now >= expires && self.state == DelegationState::Active {
                self.state = DelegationState::Expired;
                return true;
            }
        }
        false
    }

    /// Remaining amount
    #[inline(always)]
    pub fn remaining(&self) -> u64 {
        self.amount.saturating_sub(self.usage)
    }

    /// Can re-delegate?
    pub fn can_redelegate(&self) -> bool {
        if self.state != DelegationState::Active {
            return false;
        }
        for constraint in &self.constraints {
            match constraint {
                DelegationConstraint::NonTransferable => return false,
                DelegationConstraint::MaxHops(max) => {
                    if self.hop_count >= *max {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }
}

// ============================================================================
// DELEGATION CHAIN
// ============================================================================

/// Authority chain for delegation tracking
#[derive(Debug, Clone)]
pub struct DelegationChain {
    /// Root delegator
    pub root: u64,
    /// Chain of delegation ids
    pub chain: Vec<u64>,
    /// Total amount
    pub total_delegated: u64,
}

impl DelegationChain {
    pub fn new(root: u64) -> Self {
        Self {
            root,
            chain: Vec::new(),
            total_delegated: 0,
        }
    }

    /// Add link
    #[inline(always)]
    pub fn add_link(&mut self, delegation_id: u64, amount: u64) {
        self.chain.push(delegation_id);
        self.total_delegated += amount;
    }

    /// Depth
    #[inline(always)]
    pub fn depth(&self) -> usize {
        self.chain.len()
    }
}

// ============================================================================
// DELEGATION MANAGER
// ============================================================================

/// Delegation stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct CoopDelegationStats {
    /// Active delegations
    pub active: usize,
    /// Pending delegations
    pub pending: usize,
    /// Total delegated this epoch
    pub total_delegated: u64,
    /// Revocations
    pub revocations: u64,
}

/// Cooperative delegation manager
pub struct CoopDelegationManager {
    /// All delegations
    delegations: BTreeMap<u64, Delegation>,
    /// Delegations by delegator
    by_delegator: BTreeMap<u64, Vec<u64>>,
    /// Delegations by delegate
    by_delegate: BTreeMap<u64, Vec<u64>>,
    /// Next id
    next_id: u64,
    /// Stats
    stats: CoopDelegationStats,
}

impl CoopDelegationManager {
    pub fn new() -> Self {
        Self {
            delegations: BTreeMap::new(),
            by_delegator: BTreeMap::new(),
            by_delegate: BTreeMap::new(),
            next_id: 1,
            stats: CoopDelegationStats::default(),
        }
    }

    /// Create delegation
    pub fn delegate(
        &mut self,
        delegator: u64,
        delegate: u64,
        dtype: DelegationType,
        amount: u64,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let delegation = Delegation::new(id, delegator, delegate, dtype, amount, now);
        self.delegations.insert(id, delegation);
        self.by_delegator
            .entry(delegator)
            .or_insert_with(Vec::new)
            .push(id);
        self.by_delegate
            .entry(delegate)
            .or_insert_with(Vec::new)
            .push(id);
        self.update_stats();
        id
    }

    /// Accept delegation
    pub fn accept(&mut self, delegation_id: u64, now: u64) -> bool {
        if let Some(d) = self.delegations.get_mut(&delegation_id) {
            let result = d.accept(now);
            if result {
                self.stats.total_delegated += d.amount;
            }
            self.update_stats();
            result
        } else {
            false
        }
    }

    /// Revoke delegation
    pub fn revoke(&mut self, delegation_id: u64) -> bool {
        if let Some(d) = self.delegations.get_mut(&delegation_id) {
            let result = d.revoke();
            if result {
                self.stats.revocations += 1;
            }
            self.update_stats();
            result
        } else {
            false
        }
    }

    /// Consume from delegation
    #[inline]
    pub fn consume(&mut self, delegation_id: u64, amount: u64) -> bool {
        if let Some(d) = self.delegations.get_mut(&delegation_id) {
            d.consume(amount)
        } else {
            false
        }
    }

    /// Check expirations
    #[inline]
    pub fn check_expirations(&mut self, now: u64) -> Vec<u64> {
        let mut expired = Vec::new();
        for d in self.delegations.values_mut() {
            if d.check_expiry(now) {
                expired.push(d.id);
            }
        }
        self.update_stats();
        expired
    }

    /// Get delegation
    #[inline(always)]
    pub fn get(&self, id: u64) -> Option<&Delegation> {
        self.delegations.get(&id)
    }

    /// Active delegations for delegate
    #[inline]
    pub fn delegations_for(&self, delegate: u64) -> Vec<&Delegation> {
        self.by_delegate
            .get(&delegate)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.delegations.get(id))
                    .filter(|d| d.state == DelegationState::Active)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn update_stats(&mut self) {
        self.stats.active = self
            .delegations
            .values()
            .filter(|d| d.state == DelegationState::Active)
            .count();
        self.stats.pending = self
            .delegations
            .values()
            .filter(|d| d.state == DelegationState::Pending)
            .count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &CoopDelegationStats {
        &self.stats
    }
}
