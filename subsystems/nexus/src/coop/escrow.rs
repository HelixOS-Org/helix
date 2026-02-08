//! # Cooperative Escrow Service
//!
//! Resource escrow for cooperative transactions:
//! - Hold resources in escrow during negotiations
//! - Conditional release based on agreement
//! - Timeout-based auto-release
//! - Multi-party escrow support
//! - Dispute resolution through arbiter

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ESCROW TYPES
// ============================================================================

/// Escrow state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscrowState {
    /// Created, awaiting deposit
    Pending,
    /// Resources deposited
    Funded,
    /// Conditions met, awaiting release
    Ready,
    /// Released to recipient
    Released,
    /// Returned to depositor
    Refunded,
    /// Dispute raised
    Disputed,
    /// Expired (timed out)
    Expired,
}

/// Escrow resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscrowResourceType {
    /// CPU time credits
    CpuCredits,
    /// Memory pages
    MemoryPages,
    /// I/O bandwidth tokens
    IoBandwidth,
    /// Network bandwidth
    NetworkBandwidth,
    /// GPU time slices
    GpuTime,
    /// Storage quota
    StorageQuota,
    /// Generic token
    Token,
}

/// Escrow condition
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscrowCondition {
    /// Time-based release
    TimeRelease,
    /// Mutual agreement
    MutualAgreement,
    /// Task completion
    TaskCompletion,
    /// Threshold met
    ThresholdMet,
    /// External signal
    ExternalSignal,
}

/// Resolution type for disputes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisputeResolution {
    /// Release to recipient
    ReleaseToRecipient,
    /// Refund to depositor
    RefundToDepositor,
    /// Split between parties
    Split,
    /// Escalate to higher authority
    Escalate,
}

// ============================================================================
// ESCROW CONTRACT
// ============================================================================

/// Resource held in escrow
#[derive(Debug, Clone)]
pub struct EscrowResource {
    /// Resource type
    pub resource_type: EscrowResourceType,
    /// Amount
    pub amount: u64,
    /// Depositor process
    pub depositor: u64,
    /// Deposit timestamp
    pub deposited_at: u64,
}

/// Escrow contract
#[derive(Debug)]
pub struct EscrowContract {
    /// Unique contract id
    pub id: u64,
    /// State
    pub state: EscrowState,
    /// Depositor (payer)
    pub depositor: u64,
    /// Recipient (payee)
    pub recipient: u64,
    /// Arbiter (dispute resolver)
    pub arbiter: Option<u64>,
    /// Resources held
    pub resources: Vec<EscrowResource>,
    /// Release conditions
    pub conditions: Vec<EscrowCondition>,
    /// Conditions satisfied
    conditions_met: Vec<bool>,
    /// Creation time
    pub created_at: u64,
    /// Expiry time (absolute)
    pub expires_at: u64,
    /// Release time (when released)
    pub released_at: Option<u64>,
}

impl EscrowContract {
    pub fn new(
        id: u64,
        depositor: u64,
        recipient: u64,
        conditions: Vec<EscrowCondition>,
        created_at: u64,
        ttl_ns: u64,
    ) -> Self {
        let len = conditions.len();
        Self {
            id,
            state: EscrowState::Pending,
            depositor,
            recipient,
            arbiter: None,
            resources: Vec::new(),
            conditions,
            conditions_met: alloc::vec![false; len],
            created_at,
            expires_at: created_at + ttl_ns,
            released_at: None,
        }
    }

    /// Deposit resource
    pub fn deposit(&mut self, resource: EscrowResource) {
        if self.state == EscrowState::Pending {
            self.resources.push(resource);
            if !self.resources.is_empty() {
                self.state = EscrowState::Funded;
            }
        }
    }

    /// Mark condition as met
    pub fn satisfy_condition(&mut self, index: usize) {
        if index < self.conditions_met.len() {
            self.conditions_met[index] = true;
        }
        if self.all_conditions_met() && self.state == EscrowState::Funded {
            self.state = EscrowState::Ready;
        }
    }

    /// All conditions satisfied?
    pub fn all_conditions_met(&self) -> bool {
        self.conditions_met.iter().all(|&m| m)
    }

    /// Release to recipient
    pub fn release(&mut self, now: u64) -> bool {
        if self.state == EscrowState::Ready || self.state == EscrowState::Funded {
            self.state = EscrowState::Released;
            self.released_at = Some(now);
            true
        } else {
            false
        }
    }

    /// Refund to depositor
    pub fn refund(&mut self, now: u64) -> bool {
        if self.state == EscrowState::Funded || self.state == EscrowState::Pending {
            self.state = EscrowState::Refunded;
            self.released_at = Some(now);
            true
        } else {
            false
        }
    }

    /// Raise dispute
    pub fn dispute(&mut self) -> bool {
        if self.state == EscrowState::Funded || self.state == EscrowState::Ready {
            self.state = EscrowState::Disputed;
            true
        } else {
            false
        }
    }

    /// Check expiry
    pub fn check_expiry(&mut self, now: u64) -> bool {
        if now >= self.expires_at
            && self.state != EscrowState::Released
            && self.state != EscrowState::Refunded
            && self.state != EscrowState::Expired
        {
            self.state = EscrowState::Expired;
            true
        } else {
            false
        }
    }

    /// Total escrowed amount
    pub fn total_amount(&self) -> u64 {
        self.resources.iter().map(|r| r.amount).sum()
    }

    /// Duration held (ns)
    pub fn held_duration(&self, now: u64) -> u64 {
        let end = self.released_at.unwrap_or(now);
        end.saturating_sub(self.created_at)
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Escrow stats
#[derive(Debug, Clone, Default)]
pub struct CoopEscrowStats {
    /// Active escrows
    pub active_count: usize,
    /// Total created
    pub total_created: u64,
    /// Total released
    pub total_released: u64,
    /// Total refunded
    pub total_refunded: u64,
    /// Total expired
    pub total_expired: u64,
    /// Total disputed
    pub total_disputed: u64,
    /// Total resources escrowed
    pub total_resources_escrowed: u64,
}

/// Cooperative escrow manager
pub struct CoopEscrowManager {
    /// Active contracts
    contracts: BTreeMap<u64, EscrowContract>,
    /// Next contract id
    next_id: u64,
    /// Stats
    stats: CoopEscrowStats,
}

impl CoopEscrowManager {
    pub fn new() -> Self {
        Self {
            contracts: BTreeMap::new(),
            next_id: 1,
            stats: CoopEscrowStats::default(),
        }
    }

    /// Create new escrow
    pub fn create(
        &mut self,
        depositor: u64,
        recipient: u64,
        conditions: Vec<EscrowCondition>,
        now: u64,
        ttl_ns: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let contract = EscrowContract::new(id, depositor, recipient, conditions, now, ttl_ns);
        self.contracts.insert(id, contract);
        self.stats.total_created += 1;
        self.update_stats();
        id
    }

    /// Deposit to escrow
    pub fn deposit(&mut self, id: u64, resource: EscrowResource) -> bool {
        if let Some(contract) = self.contracts.get_mut(&id) {
            contract.deposit(resource);
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Satisfy condition
    pub fn satisfy(&mut self, id: u64, condition_index: usize) -> bool {
        if let Some(contract) = self.contracts.get_mut(&id) {
            contract.satisfy_condition(condition_index);
            true
        } else {
            false
        }
    }

    /// Release escrow
    pub fn release(&mut self, id: u64, now: u64) -> bool {
        if let Some(contract) = self.contracts.get_mut(&id) {
            if contract.release(now) {
                self.stats.total_released += 1;
                self.update_stats();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Refund escrow
    pub fn refund(&mut self, id: u64, now: u64) -> bool {
        if let Some(contract) = self.contracts.get_mut(&id) {
            if contract.refund(now) {
                self.stats.total_refunded += 1;
                self.update_stats();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Check all for expiry
    pub fn check_expiry(&mut self, now: u64) {
        let ids: Vec<u64> = self.contracts.keys().copied().collect();
        for id in ids {
            if let Some(contract) = self.contracts.get_mut(&id) {
                if contract.check_expiry(now) {
                    self.stats.total_expired += 1;
                }
            }
        }
        self.update_stats();
    }

    /// Get contract
    pub fn contract(&self, id: u64) -> Option<&EscrowContract> {
        self.contracts.get(&id)
    }

    fn update_stats(&mut self) {
        self.stats.active_count = self
            .contracts
            .values()
            .filter(|c| c.state == EscrowState::Funded || c.state == EscrowState::Ready)
            .count();
        self.stats.total_resources_escrowed = self
            .contracts
            .values()
            .filter(|c| c.state == EscrowState::Funded || c.state == EscrowState::Ready)
            .map(|c| c.total_amount())
            .sum();
    }

    /// Stats
    pub fn stats(&self) -> &CoopEscrowStats {
        &self.stats
    }
}
