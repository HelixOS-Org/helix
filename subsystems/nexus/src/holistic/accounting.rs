//! # Holistic Accounting Engine
//!
//! System-wide resource accounting:
//! - Per-subsystem resource tracking
//! - Cost attribution
//! - Budget enforcement
//! - Chargeback calculation
//! - Accounting reports

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ACCOUNTING TYPES
// ============================================================================

/// Accountable resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountableResource {
    /// CPU cycles
    CpuCycles,
    /// Memory pages
    MemoryPages,
    /// I/O bytes
    IoBytes,
    /// Network bytes
    NetworkBytes,
    /// Storage bytes
    StorageBytes,
    /// GPU cycles
    GpuCycles,
    /// Power (mW-hours)
    PowerMwh,
    /// Interrupts serviced
    Interrupts,
}

impl AccountableResource {
    /// Unit cost (abstract cost units)
    pub fn unit_cost(&self) -> f64 {
        match self {
            Self::CpuCycles => 0.001,
            Self::MemoryPages => 0.1,
            Self::IoBytes => 0.0001,
            Self::NetworkBytes => 0.0002,
            Self::StorageBytes => 0.00005,
            Self::GpuCycles => 0.005,
            Self::PowerMwh => 1.0,
            Self::Interrupts => 0.01,
        }
    }
}

/// Accounting entity type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityType {
    /// Single process
    Process,
    /// Process group
    ProcessGroup,
    /// Container/cgroup
    Container,
    /// User
    User,
    /// Subsystem
    Subsystem,
}

// ============================================================================
// RESOURCE LEDGER
// ============================================================================

/// Resource usage entry
#[derive(Debug, Clone)]
pub struct UsageEntry {
    /// Resource
    pub resource: AccountableResource,
    /// Quantity used
    pub quantity: u64,
    /// Cost
    pub cost: f64,
    /// Timestamp
    pub timestamp: u64,
}

/// Resource ledger per entity
#[derive(Debug)]
pub struct ResourceLedger {
    /// Entity id
    pub entity_id: u64,
    /// Entity type
    pub entity_type: EntityType,
    /// Cumulative usage per resource
    pub usage: BTreeMap<u8, u64>,
    /// Cumulative cost per resource
    pub costs: BTreeMap<u8, f64>,
    /// Total cost
    pub total_cost: f64,
    /// Budget (if any)
    pub budget: Option<f64>,
    /// Entry count
    pub entries: u64,
    /// Created at
    pub created_at: u64,
}

impl ResourceLedger {
    pub fn new(entity_id: u64, entity_type: EntityType, now: u64) -> Self {
        Self {
            entity_id,
            entity_type,
            usage: BTreeMap::new(),
            costs: BTreeMap::new(),
            total_cost: 0.0,
            budget: None,
            entries: 0,
            created_at: now,
        }
    }

    /// Record usage
    #[inline]
    pub fn record(&mut self, resource: AccountableResource, quantity: u64) {
        let key = resource as u8;
        *self.usage.entry(key).or_insert(0) += quantity;
        let cost = quantity as f64 * resource.unit_cost();
        *self.costs.entry(key).or_insert(0.0) += cost;
        self.total_cost += cost;
        self.entries += 1;
    }

    /// Usage of resource
    #[inline(always)]
    pub fn usage_of(&self, resource: AccountableResource) -> u64 {
        self.usage.get(&(resource as u8)).copied().unwrap_or(0)
    }

    /// Cost of resource
    #[inline(always)]
    pub fn cost_of(&self, resource: AccountableResource) -> f64 {
        self.costs.get(&(resource as u8)).copied().unwrap_or(0.0)
    }

    /// Budget utilization
    #[inline]
    pub fn budget_utilization(&self) -> f64 {
        if let Some(budget) = self.budget {
            if budget > 0.0 {
                return self.total_cost / budget;
            }
        }
        0.0
    }

    /// Is over budget?
    #[inline]
    pub fn is_over_budget(&self) -> bool {
        if let Some(budget) = self.budget {
            return self.total_cost > budget;
        }
        false
    }

    /// Top cost resources
    pub fn top_costs(&self) -> Vec<(AccountableResource, f64)> {
        let resources = [
            AccountableResource::CpuCycles,
            AccountableResource::MemoryPages,
            AccountableResource::IoBytes,
            AccountableResource::NetworkBytes,
            AccountableResource::StorageBytes,
            AccountableResource::GpuCycles,
            AccountableResource::PowerMwh,
            AccountableResource::Interrupts,
        ];
        let mut result: Vec<_> = resources
            .iter()
            .map(|&r| (r, self.cost_of(r)))
            .filter(|(_, c)| *c > 0.0)
            .collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        result
    }
}

// ============================================================================
// ACCOUNTING REPORT
// ============================================================================

/// Accounting period
#[derive(Debug, Clone)]
pub struct AccountingPeriod {
    /// Start time
    pub start: u64,
    /// End time
    pub end: u64,
    /// Total cost
    pub total_cost: f64,
    /// Entity count
    pub entity_count: usize,
    /// Per-entity costs
    pub entity_costs: Vec<(u64, f64)>,
}

impl AccountingPeriod {
    pub fn new(start: u64, end: u64) -> Self {
        Self {
            start,
            end,
            total_cost: 0.0,
            entity_count: 0,
            entity_costs: Vec::new(),
        }
    }

    /// Duration (ns)
    #[inline(always)]
    pub fn duration_ns(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    /// Cost rate (per second)
    #[inline]
    pub fn cost_rate(&self) -> f64 {
        let dur = self.duration_ns();
        if dur == 0 {
            return 0.0;
        }
        self.total_cost / (dur as f64 / 1_000_000_000.0)
    }
}

// ============================================================================
// ACCOUNTING ENGINE
// ============================================================================

/// Accounting stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticAccountingStats {
    /// Entities tracked
    pub entities: usize,
    /// Total cost
    pub total_cost: f64,
    /// Over-budget entities
    pub over_budget: usize,
    /// Total records
    pub total_records: u64,
}

/// Holistic accounting engine
pub struct HolisticAccountingEngine {
    /// Ledgers
    ledgers: BTreeMap<u64, ResourceLedger>,
    /// Stats
    stats: HolisticAccountingStats,
}

impl HolisticAccountingEngine {
    pub fn new() -> Self {
        Self {
            ledgers: BTreeMap::new(),
            stats: HolisticAccountingStats::default(),
        }
    }

    /// Register entity
    #[inline]
    pub fn register(&mut self, entity_id: u64, entity_type: EntityType, now: u64) {
        self.ledgers
            .entry(entity_id)
            .or_insert_with(|| ResourceLedger::new(entity_id, entity_type, now));
        self.stats.entities = self.ledgers.len();
    }

    /// Set budget
    #[inline]
    pub fn set_budget(&mut self, entity_id: u64, budget: f64) {
        if let Some(ledger) = self.ledgers.get_mut(&entity_id) {
            ledger.budget = Some(budget);
        }
    }

    /// Record usage
    #[inline]
    pub fn record(&mut self, entity_id: u64, resource: AccountableResource, quantity: u64) {
        if let Some(ledger) = self.ledgers.get_mut(&entity_id) {
            ledger.record(resource, quantity);
            self.stats.total_cost = self.ledgers.values().map(|l| l.total_cost).sum();
            self.stats.total_records += 1;
            self.stats.over_budget = self.ledgers.values().filter(|l| l.is_over_budget()).count();
        }
    }

    /// Get ledger
    #[inline(always)]
    pub fn ledger(&self, entity_id: u64) -> Option<&ResourceLedger> {
        self.ledgers.get(&entity_id)
    }

    /// Top spenders
    #[inline]
    pub fn top_spenders(&self, limit: usize) -> Vec<(u64, f64)> {
        let mut spenders: Vec<_> = self
            .ledgers
            .values()
            .map(|l| (l.entity_id, l.total_cost))
            .collect();
        spenders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        spenders.truncate(limit);
        spenders
    }

    /// Over-budget entities
    #[inline]
    pub fn over_budget(&self) -> Vec<(u64, f64)> {
        self.ledgers
            .values()
            .filter(|l| l.is_over_budget())
            .map(|l| (l.entity_id, l.budget_utilization()))
            .collect()
    }

    /// Generate period report
    pub fn generate_report(&self, start: u64, end: u64) -> AccountingPeriod {
        let mut report = AccountingPeriod::new(start, end);
        for ledger in self.ledgers.values() {
            report.entity_costs.push((ledger.entity_id, ledger.total_cost));
            report.total_cost += ledger.total_cost;
        }
        report.entity_count = self.ledgers.len();
        report.entity_costs.sort_by(|a, b| {
            b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal)
        });
        report
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticAccountingStats {
        &self.stats
    }
}
