//! # Resource Negotiation Engine
//!
//! Contracts between kernel and applications for guaranteed resource access.
//! Applications can request resource guarantees; the kernel evaluates feasibility
//! and either accepts, modifies, or rejects the contract.

use alloc::vec::Vec;

// ============================================================================
// CONTRACT IDENTIFIERS
// ============================================================================

/// Unique contract identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ContractId(pub u64);

// ============================================================================
// RESOURCE TYPES
// ============================================================================

/// A resource demand from an application
#[derive(Debug, Clone, Copy)]
pub struct ResourceDemand {
    /// CPU share requested (0.0 - 1.0 of a single core)
    pub cpu_share: f64,
    /// Memory requested (bytes)
    pub memory_bytes: u64,
    /// I/O bandwidth requested (bytes/sec)
    pub io_bandwidth: u64,
    /// Network bandwidth requested (bytes/sec)
    pub network_bandwidth: u64,
    /// Latency SLA (microseconds), 0 = no requirement
    pub latency_us: u64,
    /// Priority level (0-15)
    pub priority: u8,
    /// Duration of the contract (ms), 0 = indefinite
    pub duration_ms: u64,
}

impl ResourceDemand {
    pub fn new() -> Self {
        Self {
            cpu_share: 0.0,
            memory_bytes: 0,
            io_bandwidth: 0,
            network_bandwidth: 0,
            latency_us: 0,
            priority: 5,
            duration_ms: 0,
        }
    }

    pub fn with_cpu(mut self, share: f64) -> Self {
        self.cpu_share = share.clamp(0.0, 1.0);
        self
    }

    pub fn with_memory(mut self, bytes: u64) -> Self {
        self.memory_bytes = bytes;
        self
    }

    pub fn with_io(mut self, bps: u64) -> Self {
        self.io_bandwidth = bps;
        self
    }

    pub fn with_latency(mut self, us: u64) -> Self {
        self.latency_us = us;
        self
    }
}

/// A resource offer from the kernel
#[derive(Debug, Clone, Copy)]
pub struct ResourceOffer {
    /// CPU share offered
    pub cpu_share: f64,
    /// Memory offered (bytes)
    pub memory_bytes: u64,
    /// I/O bandwidth offered (bytes/sec)
    pub io_bandwidth: u64,
    /// Network bandwidth offered (bytes/sec)
    pub network_bandwidth: u64,
    /// Latency guarantee (microseconds)
    pub latency_us: u64,
    /// Satisfaction ratio (0.0 - 1.0, how much of the demand is met)
    pub satisfaction: f64,
}

impl ResourceOffer {
    /// Create an offer that fully satisfies the demand
    pub fn full(demand: &ResourceDemand) -> Self {
        Self {
            cpu_share: demand.cpu_share,
            memory_bytes: demand.memory_bytes,
            io_bandwidth: demand.io_bandwidth,
            network_bandwidth: demand.network_bandwidth,
            latency_us: demand.latency_us,
            satisfaction: 1.0,
        }
    }

    /// Create a scaled offer (partial satisfaction)
    pub fn scaled(demand: &ResourceDemand, factor: f64) -> Self {
        let factor = factor.clamp(0.0, 1.0);
        Self {
            cpu_share: demand.cpu_share * factor,
            memory_bytes: (demand.memory_bytes as f64 * factor) as u64,
            io_bandwidth: (demand.io_bandwidth as f64 * factor) as u64,
            network_bandwidth: (demand.network_bandwidth as f64 * factor) as u64,
            latency_us: if factor >= 0.5 { demand.latency_us } else { 0 },
            satisfaction: factor,
        }
    }
}

// ============================================================================
// CONTRACTS
// ============================================================================

/// State of a resource contract
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractState {
    /// Contract has been proposed by the app
    Proposed,
    /// Kernel has made a counter-offer
    CounterOffered,
    /// Contract has been accepted by both parties
    Active,
    /// Contract is being renegotiated
    Renegotiating,
    /// Contract violated by one party
    Violated,
    /// Contract has been revoked by the kernel
    Revoked,
    /// Contract expired
    Expired,
    /// Contract was rejected
    Rejected,
}

/// A resource contract
#[derive(Debug, Clone)]
pub struct Contract {
    /// Contract ID
    pub id: ContractId,
    /// Process ID
    pub pid: u64,
    /// Session ID
    pub session_id: u64,
    /// Current state
    pub state: ContractState,
    /// Original demand
    pub demand: ResourceDemand,
    /// Current offer (if any)
    pub offer: Option<ResourceOffer>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
    /// Number of violations
    pub violations: u32,
    /// Maximum allowed violations before revocation
    pub max_violations: u32,
}

impl Contract {
    pub fn new(id: ContractId, pid: u64, session_id: u64, demand: ResourceDemand) -> Self {
        Self {
            id,
            pid,
            session_id,
            state: ContractState::Proposed,
            demand,
            offer: None,
            created_at: 0,
            updated_at: 0,
            violations: 0,
            max_violations: 3,
        }
    }

    /// Accept the contract with a specific offer
    pub fn accept(&mut self, offer: ResourceOffer, timestamp: u64) {
        self.state = ContractState::Active;
        self.offer = Some(offer);
        self.updated_at = timestamp;
    }

    /// Counter-offer
    pub fn counter_offer(&mut self, offer: ResourceOffer, timestamp: u64) {
        self.state = ContractState::CounterOffered;
        self.offer = Some(offer);
        self.updated_at = timestamp;
    }

    /// Reject the contract
    pub fn reject(&mut self, timestamp: u64) {
        self.state = ContractState::Rejected;
        self.updated_at = timestamp;
    }

    /// Record a violation
    pub fn record_violation(&mut self, timestamp: u64) -> bool {
        self.violations += 1;
        self.updated_at = timestamp;
        if self.violations >= self.max_violations {
            self.state = ContractState::Revoked;
            true
        } else {
            self.state = ContractState::Violated;
            false
        }
    }

    /// Check if contract has expired
    pub fn check_expiry(&mut self, current_time: u64) -> bool {
        if self.demand.duration_ms > 0
            && current_time.saturating_sub(self.created_at) > self.demand.duration_ms
        {
            self.state = ContractState::Expired;
            true
        } else {
            false
        }
    }

    /// Whether the contract is active
    pub fn is_active(&self) -> bool {
        self.state == ContractState::Active
    }
}

// ============================================================================
// NEGOTIATION ENGINE
// ============================================================================

/// System resource capacity (what the kernel has available)
#[derive(Debug, Clone, Copy)]
pub struct SystemCapacity {
    /// Total CPU cores
    pub total_cpu_cores: u32,
    /// Available CPU share (0.0 - total_cores)
    pub available_cpu: f64,
    /// Total memory (bytes)
    pub total_memory: u64,
    /// Available memory (bytes)
    pub available_memory: u64,
    /// Total I/O bandwidth (bytes/sec)
    pub total_io_bandwidth: u64,
    /// Available I/O bandwidth (bytes/sec)
    pub available_io_bandwidth: u64,
}

/// Negotiation engine that evaluates demands against system capacity
pub struct NegotiationEngine {
    /// Active contracts
    contracts: Vec<Contract>,
    /// Next contract ID
    next_id: u64,
    /// System capacity snapshot
    capacity: SystemCapacity,
    /// Minimum satisfaction ratio to auto-accept
    min_satisfaction: f64,
}

impl NegotiationEngine {
    pub fn new(capacity: SystemCapacity) -> Self {
        Self {
            contracts: Vec::new(),
            next_id: 1,
            capacity,
            min_satisfaction: 0.5,
        }
    }

    /// Update system capacity snapshot
    pub fn update_capacity(&mut self, capacity: SystemCapacity) {
        self.capacity = capacity;
    }

    /// Evaluate a resource demand and return a contract with the best offer
    pub fn negotiate(&mut self, pid: u64, session_id: u64, demand: ResourceDemand) -> Contract {
        let id = ContractId(self.next_id);
        self.next_id += 1;

        let mut contract = Contract::new(id, pid, session_id, demand);

        // Calculate what we can actually provide
        let cpu_ratio = if demand.cpu_share > 0.0 {
            (self.capacity.available_cpu / demand.cpu_share).min(1.0)
        } else {
            1.0
        };
        let mem_ratio = if demand.memory_bytes > 0 {
            (self.capacity.available_memory as f64 / demand.memory_bytes as f64).min(1.0)
        } else {
            1.0
        };
        let io_ratio = if demand.io_bandwidth > 0 {
            (self.capacity.available_io_bandwidth as f64 / demand.io_bandwidth as f64).min(1.0)
        } else {
            1.0
        };

        let overall_ratio = cpu_ratio.min(mem_ratio).min(io_ratio);

        if overall_ratio >= 1.0 {
            // Full satisfaction
            let offer = ResourceOffer::full(&demand);
            contract.accept(offer, 0);
        } else if overall_ratio >= self.min_satisfaction {
            // Partial satisfaction — counter-offer
            let offer = ResourceOffer::scaled(&demand, overall_ratio);
            contract.counter_offer(offer, 0);
        } else {
            // Cannot satisfy minimum
            contract.reject(0);
        }

        self.contracts.push(contract.clone());
        contract
    }

    /// Get all active contracts for a PID
    pub fn contracts_for(&self, pid: u64) -> Vec<&Contract> {
        self.contracts
            .iter()
            .filter(|c| c.pid == pid && c.is_active())
            .collect()
    }

    /// Expire old contracts
    pub fn expire_contracts(&mut self, current_time: u64) -> usize {
        let mut expired = 0;
        for contract in &mut self.contracts {
            if contract.is_active() && contract.check_expiry(current_time) {
                expired += 1;
            }
        }
        expired
    }

    /// Total active contracts
    pub fn active_count(&self) -> usize {
        self.contracts.iter().filter(|c| c.is_active()).count()
    }

    /// Total committed resources across all active contracts
    pub fn committed_resources(&self) -> (f64, u64, u64) {
        let mut cpu = 0.0;
        let mut mem = 0u64;
        let mut io = 0u64;
        for c in &self.contracts {
            if let Some(ref offer) = c.offer {
                if c.is_active() {
                    cpu += offer.cpu_share;
                    mem = mem.saturating_add(offer.memory_bytes);
                    io = io.saturating_add(offer.io_bandwidth);
                }
            }
        }
        (cpu, mem, io)
    }
}
