//! # Coop SLA Engine
//!
//! Service Level Agreement management for cooperative processes:
//! - SLA definition and negotiation
//! - SLO (Service Level Objective) monitoring
//! - Breach detection and penalty computation
//! - SLA tiering (gold/silver/bronze)
//! - Error budget tracking

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// SLA TYPES
// ============================================================================

/// SLA tier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SlaTier {
    /// Best effort (no guarantees)
    BestEffort,
    /// Bronze tier
    Bronze,
    /// Silver tier
    Silver,
    /// Gold tier
    Gold,
    /// Platinum tier
    Platinum,
}

/// SLO metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SloMetric {
    /// Availability (uptime %)
    Availability,
    /// Latency (percentile ns)
    Latency,
    /// Throughput (ops/sec)
    Throughput,
    /// Error rate
    ErrorRate,
    /// Response time
    ResponseTime,
    /// Queue wait time
    QueueWait,
}

/// SLO status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SloStatus {
    /// Within target
    Met,
    /// Close to breach (within 10%)
    Warning,
    /// Breached
    Breached,
    /// Unknown (insufficient data)
    Unknown,
}

// ============================================================================
// SLO DEFINITION
// ============================================================================

/// Service Level Objective
#[derive(Debug, Clone)]
pub struct SloDefinition {
    /// SLO ID
    pub slo_id: u64,
    /// Metric type
    pub metric: SloMetric,
    /// Target value
    pub target: f64,
    /// Warning threshold (fraction of target at which to warn)
    pub warning_threshold: f64,
    /// Measurement window (ns)
    pub window_ns: u64,
    /// Percentile (for latency metrics, e.g. 0.99)
    pub percentile: f64,
}

impl SloDefinition {
    pub fn new(slo_id: u64, metric: SloMetric, target: f64) -> Self {
        Self {
            slo_id,
            metric,
            target,
            warning_threshold: 0.9,
            window_ns: 60_000_000_000, // 60s window
            percentile: 0.99,
        }
    }

    /// Check status given current value
    pub fn check(&self, current: f64) -> SloStatus {
        match self.metric {
            SloMetric::Availability | SloMetric::Throughput => {
                // Higher is better
                if current >= self.target {
                    SloStatus::Met
                } else if current >= self.target * self.warning_threshold {
                    SloStatus::Warning
                } else {
                    SloStatus::Breached
                }
            }
            SloMetric::Latency | SloMetric::ErrorRate | SloMetric::ResponseTime | SloMetric::QueueWait => {
                // Lower is better
                if current <= self.target {
                    SloStatus::Met
                } else if current <= self.target / self.warning_threshold {
                    SloStatus::Warning
                } else {
                    SloStatus::Breached
                }
            }
        }
    }
}

// ============================================================================
// ERROR BUDGET
// ============================================================================

/// Error budget tracker
#[derive(Debug, Clone)]
pub struct ErrorBudget {
    /// Total budget (violations allowed in window)
    pub total_budget: f64,
    /// Consumed budget
    pub consumed: f64,
    /// Window start (ns)
    pub window_start_ns: u64,
    /// Window length (ns)
    pub window_ns: u64,
}

impl ErrorBudget {
    pub fn new(total: f64, window_ns: u64, now: u64) -> Self {
        Self {
            total_budget: total,
            consumed: 0.0,
            window_start_ns: now,
            window_ns,
        }
    }

    /// Consume budget
    pub fn consume(&mut self, amount: f64) {
        self.consumed += amount;
    }

    /// Remaining budget
    pub fn remaining(&self) -> f64 {
        (self.total_budget - self.consumed).max(0.0)
    }

    /// Remaining fraction
    pub fn remaining_fraction(&self) -> f64 {
        if self.total_budget <= 0.0 {
            return 0.0;
        }
        self.remaining() / self.total_budget
    }

    /// Budget exhausted?
    pub fn exhausted(&self) -> bool {
        self.consumed >= self.total_budget
    }

    /// Reset if window expired
    pub fn check_window(&mut self, now: u64) {
        if now.saturating_sub(self.window_start_ns) >= self.window_ns {
            self.consumed = 0.0;
            self.window_start_ns = now;
        }
    }

    /// Burn rate (budget consumed / time elapsed)
    pub fn burn_rate(&self, now: u64) -> f64 {
        let elapsed = now.saturating_sub(self.window_start_ns) as f64;
        if elapsed <= 0.0 {
            return 0.0;
        }
        self.consumed / (elapsed / 1_000_000_000.0)
    }
}

// ============================================================================
// SLA CONTRACT
// ============================================================================

/// SLA contract for a process/service
#[derive(Debug)]
pub struct SlaContract {
    /// Contract ID
    pub contract_id: u64,
    /// Provider PID
    pub provider_pid: u64,
    /// Consumer PID
    pub consumer_pid: u64,
    /// Tier
    pub tier: SlaTier,
    /// SLOs
    pub slos: Vec<SloDefinition>,
    /// Error budgets per SLO
    pub error_budgets: BTreeMap<u64, ErrorBudget>,
    /// Current metric values
    pub current_values: BTreeMap<u64, f64>,
    /// Breach count
    pub breach_count: u64,
    /// Created (ns)
    pub created_ns: u64,
    /// Active
    pub active: bool,
}

impl SlaContract {
    pub fn new(contract_id: u64, provider: u64, consumer: u64, tier: SlaTier, now: u64) -> Self {
        Self {
            contract_id,
            provider_pid: provider,
            consumer_pid: consumer,
            tier,
            slos: Vec::new(),
            error_budgets: BTreeMap::new(),
            current_values: BTreeMap::new(),
            breach_count: 0,
            created_ns: now,
            active: true,
        }
    }

    /// Add SLO with error budget
    pub fn add_slo(&mut self, slo: SloDefinition, error_budget: f64, now: u64) {
        let slo_id = slo.slo_id;
        let window_ns = slo.window_ns;
        self.slos.push(slo);
        self.error_budgets.insert(slo_id, ErrorBudget::new(error_budget, window_ns, now));
    }

    /// Update metric value
    pub fn update_metric(&mut self, slo_id: u64, value: f64, now: u64) {
        self.current_values.insert(slo_id, value);

        // Check SLO and consume error budget
        for slo in &self.slos {
            if slo.slo_id == slo_id {
                let status = slo.check(value);
                if matches!(status, SloStatus::Breached) {
                    self.breach_count += 1;
                    if let Some(budget) = self.error_budgets.get_mut(&slo_id) {
                        budget.check_window(now);
                        budget.consume(1.0);
                    }
                }
                break;
            }
        }
    }

    /// Overall health
    pub fn health(&self) -> f64 {
        if self.slos.is_empty() {
            return 1.0;
        }
        let met_count = self.slos.iter()
            .filter(|slo| {
                let value = self.current_values.get(&slo.slo_id).copied().unwrap_or(0.0);
                matches!(slo.check(value), SloStatus::Met)
            })
            .count();
        met_count as f64 / self.slos.len() as f64
    }

    /// All budgets OK?
    pub fn budgets_ok(&self) -> bool {
        self.error_budgets.values().all(|b| !b.exhausted())
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// SLA engine stats
#[derive(Debug, Clone, Default)]
pub struct CoopSlaStats {
    /// Active contracts
    pub active_contracts: usize,
    /// Total SLOs
    pub total_slos: usize,
    /// Breached SLOs
    pub breached_slos: usize,
    /// Average health
    pub avg_health: f64,
    /// Exhausted budgets
    pub exhausted_budgets: usize,
}

/// Coop SLA engine
pub struct CoopSlaEngine {
    /// Contracts
    contracts: BTreeMap<u64, SlaContract>,
    /// Process -> contract IDs
    process_contracts: BTreeMap<u64, Vec<u64>>,
    /// Stats
    stats: CoopSlaStats,
    /// Next contract ID
    next_id: u64,
}

impl CoopSlaEngine {
    pub fn new() -> Self {
        Self {
            contracts: BTreeMap::new(),
            process_contracts: BTreeMap::new(),
            stats: CoopSlaStats::default(),
            next_id: 1,
        }
    }

    /// Create SLA contract
    pub fn create_contract(&mut self, provider: u64, consumer: u64, tier: SlaTier, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let contract = SlaContract::new(id, provider, consumer, tier, now);
        self.contracts.insert(id, contract);
        self.process_contracts.entry(provider).or_insert_with(Vec::new).push(id);
        self.process_contracts.entry(consumer).or_insert_with(Vec::new).push(id);
        self.update_stats();
        id
    }

    /// Add SLO to contract
    pub fn add_slo(&mut self, contract_id: u64, slo: SloDefinition, error_budget: f64, now: u64) {
        if let Some(contract) = self.contracts.get_mut(&contract_id) {
            contract.add_slo(slo, error_budget, now);
        }
        self.update_stats();
    }

    /// Update metric
    pub fn update_metric(&mut self, contract_id: u64, slo_id: u64, value: f64, now: u64) {
        if let Some(contract) = self.contracts.get_mut(&contract_id) {
            contract.update_metric(slo_id, value, now);
        }
        self.update_stats();
    }

    /// Get contract health
    pub fn contract_health(&self, contract_id: u64) -> f64 {
        self.contracts.get(&contract_id).map(|c| c.health()).unwrap_or(0.0)
    }

    /// Remove process
    pub fn remove_process(&mut self, pid: u64) {
        if let Some(ids) = self.process_contracts.remove(&pid) {
            for id in ids {
                if let Some(contract) = self.contracts.get_mut(&id) {
                    contract.active = false;
                }
            }
        }
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.active_contracts = self.contracts.values().filter(|c| c.active).count();
        self.stats.total_slos = self.contracts.values().map(|c| c.slos.len()).sum();
        self.stats.breached_slos = self.contracts.values()
            .flat_map(|c| c.slos.iter().map(move |slo| {
                let val = c.current_values.get(&slo.slo_id).copied().unwrap_or(0.0);
                slo.check(val)
            }))
            .filter(|s| matches!(s, SloStatus::Breached))
            .count();
        let healths: Vec<f64> = self.contracts.values()
            .filter(|c| c.active)
            .map(|c| c.health())
            .collect();
        if !healths.is_empty() {
            self.stats.avg_health = healths.iter().sum::<f64>() / healths.len() as f64;
        }
        self.stats.exhausted_budgets = self.contracts.values()
            .flat_map(|c| c.error_budgets.values())
            .filter(|b| b.exhausted())
            .count();
    }

    /// Stats
    pub fn stats(&self) -> &CoopSlaStats {
        &self.stats
    }
}
