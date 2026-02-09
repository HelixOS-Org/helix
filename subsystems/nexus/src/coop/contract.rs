//! # Cooperative Contracts
//!
//! Formal agreements between processes and the kernel:
//! - QoS contracts with guarantees
//! - Penalty/reward system
//! - Contract negotiation protocol
//! - SLA-like guarantees
//! - Contract monitoring and enforcement
//! - Breach detection and remediation

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// CONTRACT TYPES
// ============================================================================

/// Contract type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ContractType {
    /// CPU time guarantee
    CpuGuarantee,
    /// Latency bound
    LatencyBound,
    /// Throughput minimum
    ThroughputMin,
    /// Memory reservation
    MemoryReservation,
    /// I/O bandwidth
    IoBandwidth,
    /// Deadline guarantee
    DeadlineGuarantee,
    /// Composite (multiple terms)
    Composite,
}

/// Contract state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractState {
    /// Being negotiated
    Negotiating,
    /// Active and being enforced
    Active,
    /// Suspended (by kernel or process)
    Suspended,
    /// Breached (violation detected)
    Breached,
    /// Expired
    Expired,
    /// Terminated
    Terminated,
}

// ============================================================================
// CONTRACT TERMS
// ============================================================================

/// A single contract term
#[derive(Debug, Clone)]
pub struct ContractTerm {
    /// Term type
    pub term_type: TermType,
    /// Guaranteed value
    pub guaranteed: u64,
    /// Best-effort target (above guarantee)
    pub target: u64,
    /// Unit
    pub unit: TermUnit,
    /// Measurement period (ms)
    pub period_ms: u64,
    /// Penalty for kernel breach
    pub kernel_penalty: u32,
    /// Penalty for process breach
    pub process_penalty: u32,
}

/// Term type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermType {
    /// CPU time per period
    CpuTimePeriod,
    /// Maximum latency
    MaxLatency,
    /// Minimum throughput
    MinThroughput,
    /// Memory cap
    MemoryCap,
    /// I/O bandwidth minimum
    IoMin,
    /// Wakeup latency
    WakeupLatency,
    /// Deadline miss rate
    DeadlineMissRate,
    /// Yield frequency (process commits to yield)
    YieldFrequency,
}

/// Unit for terms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermUnit {
    /// Microseconds
    Microseconds,
    /// Milliseconds
    Milliseconds,
    /// Bytes
    Bytes,
    /// Bytes per second
    BytesPerSecond,
    /// Operations per second
    OpsPerSecond,
    /// Percentage
    Percent,
    /// Count
    Count,
}

// ============================================================================
// CONTRACT DEFINITION
// ============================================================================

/// Contract definition
#[derive(Debug, Clone)]
pub struct Contract {
    /// Contract ID
    pub id: u64,
    /// Process ID
    pub pid: u64,
    /// Contract name
    pub name: String,
    /// Type
    pub contract_type: ContractType,
    /// State
    pub state: ContractState,
    /// Terms
    pub terms: Vec<ContractTerm>,
    /// Created timestamp
    pub created_at: u64,
    /// Expires timestamp (0 = no expiry)
    pub expires_at: u64,
    /// Breach count
    pub breaches: u64,
    /// Compliance score (0-100)
    pub compliance_score: u32,
    /// Priority (affects enforcement priority)
    pub priority: u32,
}

impl Contract {
    pub fn new(id: u64, pid: u64, name: String, contract_type: ContractType) -> Self {
        Self {
            id,
            pid,
            name,
            contract_type,
            state: ContractState::Negotiating,
            terms: Vec::new(),
            created_at: 0,
            expires_at: 0,
            breaches: 0,
            compliance_score: 100,
            priority: 5,
        }
    }

    /// Add term
    #[inline(always)]
    pub fn add_term(&mut self, term: ContractTerm) {
        self.terms.push(term);
    }

    /// Activate
    #[inline(always)]
    pub fn activate(&mut self, now: u64) {
        self.state = ContractState::Active;
        self.created_at = now;
    }

    /// Record breach
    #[inline]
    pub fn record_breach(&mut self) {
        self.breaches += 1;
        self.compliance_score = self.compliance_score.saturating_sub(5);
        if self.compliance_score < 50 {
            self.state = ContractState::Breached;
        }
    }

    /// Is expired
    #[inline(always)]
    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at > 0 && now >= self.expires_at
    }
}

// ============================================================================
// NEGOTIATION
// ============================================================================

/// Negotiation offer
#[derive(Debug, Clone)]
pub struct NegotiationOffer {
    /// Process proposed terms
    pub proposed_terms: Vec<ContractTerm>,
    /// Process priority preference
    pub priority: u32,
    /// Process identity/reputation score
    pub reputation: u32,
}

/// Negotiation response
#[derive(Debug, Clone)]
pub struct NegotiationResponse {
    /// Accepted
    pub accepted: bool,
    /// Counter-offer terms (if not accepted)
    pub counter_terms: Vec<ContractTerm>,
    /// Reason for rejection
    pub rejection_reason: Option<String>,
    /// Available capacity
    pub available_capacity_pct: u32,
}

// ============================================================================
// BREACH
// ============================================================================

/// Contract breach event
#[derive(Debug, Clone)]
pub struct ContractBreach {
    /// Contract ID
    pub contract_id: u64,
    /// Process ID
    pub pid: u64,
    /// Breaching party
    pub party: BreachParty,
    /// Which term was breached
    pub term_index: usize,
    /// Actual value
    pub actual_value: u64,
    /// Guaranteed value
    pub guaranteed_value: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Severity
    pub severity: BreachSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreachParty {
    /// Kernel failed to provide guarantee
    Kernel,
    /// Process violated its obligations
    Process,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BreachSeverity {
    /// Minor deviation
    Minor       = 0,
    /// Notable breach
    Notable     = 1,
    /// Significant breach
    Significant = 2,
    /// Critical breach
    Critical    = 3,
}

// ============================================================================
// CONTRACT MONITOR
// ============================================================================

/// Monitoring measurement for a contract term
#[derive(Debug, Clone)]
pub struct TermMeasurement {
    /// Term index
    pub term_index: usize,
    /// Measured value
    pub value: u64,
    /// Timestamp
    pub timestamp: u64,
    /// In compliance
    pub compliant: bool,
}

// ============================================================================
// CONTRACT MANAGER
// ============================================================================

/// Contract manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct ContractManagerStats {
    /// Total contracts
    pub total: usize,
    /// Active contracts
    pub active: usize,
    /// Breached contracts
    pub breached: usize,
    /// Total breaches
    pub total_breaches: u64,
    /// Average compliance
    pub avg_compliance: u32,
    /// Negotiations pending
    pub pending_negotiations: usize,
}

/// Cooperative contract manager
pub struct CoopContractManager {
    /// Contracts by ID
    contracts: BTreeMap<u64, Contract>,
    /// Process to contracts
    pid_contracts: BTreeMap<u64, Vec<u64>>,
    /// Breach history
    breach_history: VecDeque<ContractBreach>,
    /// Pending negotiations
    negotiations: BTreeMap<u64, NegotiationOffer>,
    /// Next contract ID
    next_id: u64,
    /// System capacity tracking
    committed_capacity: BTreeMap<u8, u64>,
    /// Stats
    stats: ContractManagerStats,
    /// Max breach history
    max_breaches: usize,
}

impl CoopContractManager {
    pub fn new() -> Self {
        Self {
            contracts: BTreeMap::new(),
            pid_contracts: BTreeMap::new(),
            breach_history: VecDeque::new(),
            negotiations: BTreeMap::new(),
            next_id: 1,
            committed_capacity: BTreeMap::new(),
            stats: ContractManagerStats::default(),
            max_breaches: 1024,
        }
    }

    /// Submit negotiation offer
    #[inline]
    pub fn negotiate(&mut self, pid: u64, offer: NegotiationOffer) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.negotiations.insert(id, offer);
        self.stats.pending_negotiations = self.negotiations.len();
        id
    }

    /// Evaluate negotiation
    pub fn evaluate_negotiation(&mut self, negotiation_id: u64) -> Option<NegotiationResponse> {
        let offer = self.negotiations.get(&negotiation_id)?;

        // Simple capacity check — can we honor these terms?
        let mut can_honor = true;
        let mut counter = Vec::new();

        for term in &offer.proposed_terms {
            let committed = self
                .committed_capacity
                .get(&(term.term_type as u8))
                .copied()
                .unwrap_or(0);

            // Simplified: check if we have capacity
            let capacity_available = committed < 80; // <80% committed

            if !capacity_available {
                can_honor = false;
                // Counter with reduced guarantee
                let mut reduced = term.clone();
                reduced.guaranteed = term.guaranteed * 3 / 4;
                counter.push(reduced);
            }
        }

        Some(NegotiationResponse {
            accepted: can_honor,
            counter_terms: counter,
            rejection_reason: if can_honor {
                None
            } else {
                Some(String::from("Insufficient capacity"))
            },
            available_capacity_pct:
                100u32.saturating_sub(
                    self.committed_capacity.values().max().copied().unwrap_or(0) as u32
                ),
        })
    }

    /// Accept and create contract
    pub fn accept_contract(
        &mut self,
        pid: u64,
        name: String,
        contract_type: ContractType,
        terms: Vec<ContractTerm>,
        now: u64,
        duration_ms: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let mut contract = Contract::new(id, pid, name, contract_type);
        for term in terms {
            contract.add_term(term);
        }
        contract.activate(now);
        if duration_ms > 0 {
            contract.expires_at = now + duration_ms;
        }

        self.contracts.insert(id, contract);
        self.pid_contracts
            .entry(pid)
            .or_insert_with(Vec::new)
            .push(id);

        self.update_stats();
        id
    }

    /// Record measurement
    pub fn record_measurement(&mut self, contract_id: u64, measurement: TermMeasurement) {
        if !measurement.compliant {
            if let Some(contract) = self.contracts.get_mut(&contract_id) {
                contract.record_breach();

                let breach = ContractBreach {
                    contract_id,
                    pid: contract.pid,
                    party: BreachParty::Kernel,
                    term_index: measurement.term_index,
                    actual_value: measurement.value,
                    guaranteed_value: contract
                        .terms
                        .get(measurement.term_index)
                        .map(|t| t.guaranteed)
                        .unwrap_or(0),
                    timestamp: measurement.timestamp,
                    severity: BreachSeverity::Minor,
                };

                self.breach_history.push_back(breach);
                if self.breach_history.len() > self.max_breaches {
                    self.breach_history.pop_front();
                }

                self.stats.total_breaches += 1;
            }
        }
    }

    /// Check expirations
    pub fn check_expirations(&mut self, now: u64) -> Vec<u64> {
        let mut expired = Vec::new();
        for (id, contract) in &mut self.contracts {
            if contract.is_expired(now) && contract.state == ContractState::Active {
                contract.state = ContractState::Expired;
                expired.push(*id);
            }
        }
        if !expired.is_empty() {
            self.update_stats();
        }
        expired
    }

    /// Terminate contract
    #[inline]
    pub fn terminate(&mut self, contract_id: u64) {
        if let Some(contract) = self.contracts.get_mut(&contract_id) {
            contract.state = ContractState::Terminated;
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.total = self.contracts.len();
        self.stats.active = self
            .contracts
            .values()
            .filter(|c| c.state == ContractState::Active)
            .count();
        self.stats.breached = self
            .contracts
            .values()
            .filter(|c| c.state == ContractState::Breached)
            .count();

        let active_contracts: Vec<&Contract> = self
            .contracts
            .values()
            .filter(|c| c.state == ContractState::Active)
            .collect();
        if !active_contracts.is_empty() {
            self.stats.avg_compliance = (active_contracts
                .iter()
                .map(|c| c.compliance_score as u64)
                .sum::<u64>()
                / active_contracts.len() as u64) as u32;
        }
    }

    /// Get contract
    #[inline(always)]
    pub fn contract(&self, id: u64) -> Option<&Contract> {
        self.contracts.get(&id)
    }

    /// Get contracts for process
    #[inline]
    pub fn contracts_for_pid(&self, pid: u64) -> Vec<&Contract> {
        self.pid_contracts
            .get(&pid)
            .map(|ids| ids.iter().filter_map(|id| self.contracts.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &ContractManagerStats {
        &self.stats
    }

    /// Unregister process
    #[inline]
    pub fn unregister(&mut self, pid: u64) {
        if let Some(ids) = self.pid_contracts.remove(&pid) {
            for id in ids {
                if let Some(c) = self.contracts.get_mut(&id) {
                    c.state = ContractState::Terminated;
                }
            }
        }
        self.update_stats();
    }
}
