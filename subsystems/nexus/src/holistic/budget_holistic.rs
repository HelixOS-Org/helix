//! # Holistic Budget Manager
//!
//! System-wide resource budget tracking and enforcement:
//! - Per-tenant resource budgets
//! - Budget carryover between periods
//! - Burst budget allowance
//! - Budget forecasting
//! - Cross-resource budget correlation

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

// ============================================================================
// BUDGET TYPES
// ============================================================================

/// Budgeted resource
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetedResource {
    /// CPU time (ns)
    CpuTime,
    /// Memory pages
    MemoryPages,
    /// I/O operations
    IoOps,
    /// I/O bytes
    IoBytes,
    /// Network bytes
    NetworkBytes,
    /// Syscalls
    Syscalls,
}

/// Budget period
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetPeriod {
    /// Per-second
    PerSecond,
    /// Per-minute
    PerMinute,
    /// Per-hour
    PerHour,
    /// Per-day
    PerDay,
}

impl BudgetPeriod {
    /// Duration in nanoseconds
    #[inline]
    pub fn duration_ns(&self) -> u64 {
        match self {
            BudgetPeriod::PerSecond => 1_000_000_000,
            BudgetPeriod::PerMinute => 60_000_000_000,
            BudgetPeriod::PerHour => 3_600_000_000_000,
            BudgetPeriod::PerDay => 86_400_000_000_000,
        }
    }
}

/// Budget state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetState {
    /// Within budget
    WithinBudget,
    /// Warning (>80%)
    Warning,
    /// Over budget
    OverBudget,
    /// Exhausted
    Exhausted,
}

// ============================================================================
// RESOURCE BUDGET
// ============================================================================

/// Single resource budget entry
#[derive(Debug, Clone)]
pub struct ResourceBudget {
    /// Resource type
    pub resource: BudgetedResource,
    /// Budget limit per period
    pub limit: u64,
    /// Current usage
    pub used: u64,
    /// Period
    pub period: BudgetPeriod,
    /// Period start
    pub period_start: u64,
    /// Carryover from previous period
    pub carryover: u64,
    /// Max carryover
    pub max_carryover: u64,
    /// Burst allowance (extra budget)
    pub burst_allowance: u64,
    /// Burst used
    pub burst_used: u64,
    /// Warning threshold (fraction)
    pub warn_threshold: f64,
}

impl ResourceBudget {
    pub fn new(resource: BudgetedResource, limit: u64, period: BudgetPeriod) -> Self {
        Self {
            resource,
            limit,
            used: 0,
            period,
            period_start: 0,
            carryover: 0,
            max_carryover: limit / 4,
            burst_allowance: limit / 10,
            burst_used: 0,
            warn_threshold: 0.8,
        }
    }

    /// Available budget
    #[inline(always)]
    pub fn available(&self) -> u64 {
        let total = self.limit + self.carryover + self.burst_allowance;
        total.saturating_sub(self.used + self.burst_used)
    }

    /// Try consume
    pub fn try_consume(&mut self, amount: u64) -> bool {
        let remaining = self.limit.saturating_sub(self.used) + self.carryover;
        if amount <= remaining {
            self.used += amount;
            true
        } else if amount <= remaining + self.burst_allowance.saturating_sub(self.burst_used) {
            // Use burst
            let from_budget = remaining;
            let from_burst = amount - from_budget;
            self.used += from_budget;
            self.burst_used += from_burst;
            true
        } else {
            false
        }
    }

    /// State
    pub fn state(&self) -> BudgetState {
        let usage_ratio = self.usage_ratio();
        if self.available() == 0 {
            BudgetState::Exhausted
        } else if usage_ratio > 1.0 {
            BudgetState::OverBudget
        } else if usage_ratio > self.warn_threshold {
            BudgetState::Warning
        } else {
            BudgetState::WithinBudget
        }
    }

    /// Usage ratio
    #[inline]
    pub fn usage_ratio(&self) -> f64 {
        if self.limit == 0 {
            return 0.0;
        }
        (self.used + self.burst_used) as f64 / self.limit as f64
    }

    /// Reset for new period
    #[inline]
    pub fn reset_period(&mut self, now: u64) {
        // Carryover unused
        let unused = self.limit.saturating_sub(self.used);
        self.carryover = unused.min(self.max_carryover);
        self.used = 0;
        self.burst_used = 0;
        self.period_start = now;
    }

    /// Is period expired?
    #[inline(always)]
    pub fn period_expired(&self, now: u64) -> bool {
        now.saturating_sub(self.period_start) >= self.period.duration_ns()
    }
}

// ============================================================================
// TENANT BUDGET
// ============================================================================

/// Per-tenant budget
#[derive(Debug)]
pub struct TenantBudget {
    /// Tenant id
    pub id: u64,
    /// Resource budgets
    budgets: BTreeMap<u8, ResourceBudget>,
    /// Usage history (period totals)
    usage_history: VecDeque<BTreeMap<u8, u64>>,
    /// Max history periods
    max_history: usize,
}

impl TenantBudget {
    pub fn new(id: u64) -> Self {
        Self {
            id,
            budgets: BTreeMap::new(),
            usage_history: VecDeque::new(),
            max_history: 24,
        }
    }

    /// Set budget for resource
    #[inline(always)]
    pub fn set_budget(&mut self, budget: ResourceBudget) {
        self.budgets.insert(budget.resource as u8, budget);
    }

    /// Try consume resource
    #[inline]
    pub fn consume(&mut self, resource: BudgetedResource, amount: u64) -> bool {
        let key = resource as u8;
        if let Some(budget) = self.budgets.get_mut(&key) {
            budget.try_consume(amount)
        } else {
            true // no budget = unlimited
        }
    }

    /// Check periods and reset
    pub fn check_periods(&mut self, now: u64) {
        let mut period_usage = BTreeMap::new();
        for (&key, budget) in self.budgets.iter_mut() {
            if budget.period_expired(now) {
                period_usage.insert(key, budget.used);
                budget.reset_period(now);
            }
        }
        if !period_usage.is_empty() {
            if self.usage_history.len() >= self.max_history {
                self.usage_history.pop_front();
            }
            self.usage_history.push_back(period_usage);
        }
    }

    /// Worst state across all resources
    #[inline]
    pub fn worst_state(&self) -> BudgetState {
        self.budgets.values()
            .map(|b| b.state())
            .max_by_key(|s| *s as u8)
            .unwrap_or(BudgetState::WithinBudget)
    }

    /// Forecast: average usage over history
    #[inline]
    pub fn forecast(&self, resource: BudgetedResource) -> Option<f64> {
        let key = resource as u8;
        let values: Vec<u64> = self.usage_history.iter()
            .filter_map(|h| h.get(&key).copied())
            .collect();
        if values.is_empty() {
            return None;
        }
        let sum: u64 = values.iter().sum();
        Some(sum as f64 / values.len() as f64)
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Budget manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticBudgetStats {
    /// Total tenants
    pub total_tenants: usize,
    /// Over-budget tenants
    pub over_budget: usize,
    /// Warning tenants
    pub warning: usize,
    /// Exhausted tenants
    pub exhausted: usize,
}

/// Holistic budget manager
pub struct HolisticBudgetEngine {
    /// Tenants
    tenants: BTreeMap<u64, TenantBudget>,
    /// Stats
    stats: HolisticBudgetStats,
}

impl HolisticBudgetEngine {
    pub fn new() -> Self {
        Self {
            tenants: BTreeMap::new(),
            stats: HolisticBudgetStats::default(),
        }
    }

    /// Register tenant
    #[inline(always)]
    pub fn register(&mut self, id: u64) {
        self.tenants.insert(id, TenantBudget::new(id));
        self.update_stats();
    }

    /// Set budget
    #[inline]
    pub fn set_budget(&mut self, tenant_id: u64, budget: ResourceBudget) -> bool {
        if let Some(tenant) = self.tenants.get_mut(&tenant_id) {
            tenant.set_budget(budget);
            true
        } else {
            false
        }
    }

    /// Consume resource
    #[inline]
    pub fn consume(&mut self, tenant_id: u64, resource: BudgetedResource, amount: u64) -> bool {
        if let Some(tenant) = self.tenants.get_mut(&tenant_id) {
            let ok = tenant.consume(resource, amount);
            self.update_stats();
            ok
        } else {
            false
        }
    }

    /// Periodic check
    #[inline]
    pub fn check_periods(&mut self, now: u64) {
        for tenant in self.tenants.values_mut() {
            tenant.check_periods(now);
        }
        self.update_stats();
    }

    /// Remove tenant
    #[inline(always)]
    pub fn remove(&mut self, id: u64) {
        self.tenants.remove(&id);
        self.update_stats();
    }

    /// Get tenant
    #[inline(always)]
    pub fn tenant(&self, id: u64) -> Option<&TenantBudget> {
        self.tenants.get(&id)
    }

    fn update_stats(&mut self) {
        self.stats.total_tenants = self.tenants.len();
        self.stats.over_budget = self.tenants.values()
            .filter(|t| t.worst_state() == BudgetState::OverBudget).count();
        self.stats.warning = self.tenants.values()
            .filter(|t| t.worst_state() == BudgetState::Warning).count();
        self.stats.exhausted = self.tenants.values()
            .filter(|t| t.worst_state() == BudgetState::Exhausted).count();
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticBudgetStats {
        &self.stats
    }
}
