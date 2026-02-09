//! # Cooperative Resource Budgeting
//!
//! Time and resource budget management for cooperative scheduling:
//! - CPU time budget allocation
//! - Budget lending between processes
//! - Deficit tracking and recovery
//! - Budget groups with proportional sharing
//! - Over-budget penalties
//! - Budget forecasting

extern crate alloc;

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// BUDGET TYPES
// ============================================================================

/// Budget resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BudgetResource {
    /// CPU time (microseconds)
    CpuTime,
    /// Memory bandwidth (bytes/period)
    MemoryBandwidth,
    /// I/O bandwidth (bytes/period)
    IoBandwidth,
    /// Network bandwidth (bytes/period)
    NetworkBandwidth,
    /// Wakeups per period
    Wakeups,
}

/// Budget state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetState {
    /// Under budget
    Healthy,
    /// Near budget limit (>80%)
    Warning,
    /// At budget limit
    AtLimit,
    /// Over budget (in deficit)
    Deficit,
    /// Exhausted (hard limit)
    Exhausted,
}

// ============================================================================
// BUDGET ALLOCATION
// ============================================================================

/// Single resource budget
#[derive(Debug, Clone)]
pub struct ResourceBudgetEntry {
    /// Resource
    pub resource: BudgetResource,
    /// Allocated budget per period
    pub allocation: u64,
    /// Current usage this period
    pub usage: u64,
    /// Accumulated deficit
    pub deficit: u64,
    /// Borrowed amount
    pub borrowed: u64,
    /// Lent amount
    pub lent: u64,
    /// Peak usage ever
    pub peak_usage: u64,
    /// Periods in deficit
    pub deficit_periods: u64,
}

impl ResourceBudgetEntry {
    pub fn new(resource: BudgetResource, allocation: u64) -> Self {
        Self {
            resource,
            allocation,
            usage: 0,
            deficit: 0,
            borrowed: 0,
            lent: 0,
            peak_usage: 0,
            deficit_periods: 0,
        }
    }

    /// Available budget
    #[inline(always)]
    pub fn available(&self) -> u64 {
        let total = self.allocation + self.borrowed;
        total.saturating_sub(self.usage + self.lent)
    }

    /// Usage percentage
    #[inline]
    pub fn usage_pct(&self) -> u32 {
        if self.allocation == 0 {
            return 100;
        }
        ((self.usage * 100) / self.allocation).min(200) as u32
    }

    /// State
    pub fn state(&self) -> BudgetState {
        let pct = self.usage_pct();
        if pct > 100 {
            if self.deficit > self.allocation / 2 {
                BudgetState::Exhausted
            } else {
                BudgetState::Deficit
            }
        } else if pct >= 100 {
            BudgetState::AtLimit
        } else if pct >= 80 {
            BudgetState::Warning
        } else {
            BudgetState::Healthy
        }
    }

    /// Consume budget
    pub fn consume(&mut self, amount: u64) -> BudgetState {
        self.usage += amount;
        if self.usage > self.peak_usage {
            self.peak_usage = self.usage;
        }

        if self.usage > self.allocation + self.borrowed {
            self.deficit = self.usage - (self.allocation + self.borrowed);
        }

        self.state()
    }

    /// Reset for new period
    #[inline]
    pub fn reset_period(&mut self) {
        if self.usage > self.allocation {
            self.deficit_periods += 1;
        }
        self.usage = 0;
        self.borrowed = 0;
        self.lent = 0;
        // Deficit carries over but decays
        self.deficit = self.deficit * 3 / 4;
    }
}

// ============================================================================
// PROCESS BUDGET
// ============================================================================

/// Complete budget for a process
#[derive(Debug, Clone)]
pub struct ProcessBudget {
    /// Process ID
    pub pid: u64,
    /// Resource budgets
    pub resources: BTreeMap<u8, ResourceBudgetEntry>,
    /// Priority weight
    pub weight: u32,
    /// Group ID (if any)
    pub group_id: Option<u64>,
    /// Active
    pub active: bool,
}

impl ProcessBudget {
    pub fn new(pid: u64, weight: u32) -> Self {
        Self {
            pid,
            resources: BTreeMap::new(),
            weight,
            group_id: None,
            active: true,
        }
    }

    /// Add resource budget
    #[inline(always)]
    pub fn add_resource(&mut self, resource: BudgetResource, allocation: u64) {
        self.resources.insert(
            resource as u8,
            ResourceBudgetEntry::new(resource, allocation),
        );
    }

    /// Consume resource
    #[inline]
    pub fn consume(&mut self, resource: BudgetResource, amount: u64) -> BudgetState {
        if let Some(entry) = self.resources.get_mut(&(resource as u8)) {
            entry.consume(amount)
        } else {
            BudgetState::Healthy
        }
    }

    /// Available for resource
    #[inline]
    pub fn available(&self, resource: BudgetResource) -> u64 {
        self.resources
            .get(&(resource as u8))
            .map(|e| e.available())
            .unwrap_or(u64::MAX)
    }

    /// Worst state across all resources
    #[inline]
    pub fn worst_state(&self) -> BudgetState {
        self.resources
            .values()
            .map(|e| e.state())
            .max_by_key(|s| *s as u8)
            .unwrap_or(BudgetState::Healthy)
    }

    /// Total deficit
    #[inline(always)]
    pub fn total_deficit(&self) -> u64 {
        self.resources.values().map(|e| e.deficit).sum()
    }

    /// Reset all for new period
    #[inline]
    pub fn reset_period(&mut self) {
        for entry in self.resources.values_mut() {
            entry.reset_period();
        }
    }
}

// ============================================================================
// BUDGET LOAN
// ============================================================================

/// Budget loan between processes
#[derive(Debug, Clone)]
pub struct BudgetLoan {
    /// Loan ID
    pub id: u64,
    /// Lender PID
    pub lender: u64,
    /// Borrower PID
    pub borrower: u64,
    /// Resource
    pub resource: BudgetResource,
    /// Amount
    pub amount: u64,
    /// Created timestamp
    pub created_at: u64,
    /// Expires timestamp
    pub expires_at: u64,
    /// Returned
    pub returned: bool,
}

// ============================================================================
// BUDGET GROUP
// ============================================================================

/// Group of processes sharing budget
#[derive(Debug, Clone)]
pub struct BudgetGroup {
    /// Group ID
    pub id: u64,
    /// Name
    pub name: String,
    /// Members
    pub members: Vec<u64>,
    /// Shared budget pool
    pub pool: BTreeMap<u8, u64>,
    /// Max members
    pub max_members: usize,
}

impl BudgetGroup {
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            members: Vec::new(),
            pool: BTreeMap::new(),
            max_members: 32,
        }
    }

    /// Add member
    #[inline]
    pub fn add_member(&mut self, pid: u64) -> bool {
        if self.members.len() >= self.max_members {
            return false;
        }
        if !self.members.contains(&pid) {
            self.members.push(pid);
        }
        true
    }

    /// Remove member
    #[inline(always)]
    pub fn remove_member(&mut self, pid: u64) {
        self.members.retain(|&p| p != pid);
    }

    /// Set pool for resource
    #[inline(always)]
    pub fn set_pool(&mut self, resource: BudgetResource, amount: u64) {
        self.pool.insert(resource as u8, amount);
    }

    /// Fair share per member
    #[inline]
    pub fn fair_share(&self, resource: BudgetResource) -> u64 {
        if self.members.is_empty() {
            return 0;
        }
        self.pool.get(&(resource as u8)).copied().unwrap_or(0) / self.members.len() as u64
    }
}

// ============================================================================
// BUDGET FORECAST
// ============================================================================

/// Budget usage forecast
#[derive(Debug, Clone)]
pub struct BudgetForecast {
    /// Process ID
    pub pid: u64,
    /// Resource
    pub resource: BudgetResource,
    /// Predicted usage for next period
    pub predicted_usage: u64,
    /// Confidence (0.0-1.0)
    pub confidence: f64,
    /// Recommended allocation
    pub recommended_allocation: u64,
    /// Will exhaust at current rate
    pub will_exhaust: bool,
    /// Periods until exhaustion
    pub periods_until_exhaust: u64,
}

// ============================================================================
// BUDGET MANAGER
// ============================================================================

/// Budget manager stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BudgetManagerStats {
    /// Total processes
    pub total_processes: usize,
    /// Active loans
    pub active_loans: usize,
    /// Budget groups
    pub budget_groups: usize,
    /// Processes in deficit
    pub in_deficit: usize,
    /// Total loans made
    pub total_loans: u64,
}

/// Cooperative budget manager
pub struct CoopBudgetManager {
    /// Per-process budgets
    budgets: BTreeMap<u64, ProcessBudget>,
    /// Active loans
    loans: Vec<BudgetLoan>,
    /// Groups
    groups: BTreeMap<u64, BudgetGroup>,
    /// Next loan ID
    next_loan_id: u64,
    /// Usage history for forecasting (pid → resource → recent usages)
    history: BTreeMap<u64, BTreeMap<u8, VecDeque<u64>>>,
    /// Max history per resource
    max_history: usize,
    /// Stats
    stats: BudgetManagerStats,
}

impl CoopBudgetManager {
    pub fn new() -> Self {
        Self {
            budgets: BTreeMap::new(),
            loans: Vec::new(),
            groups: BTreeMap::new(),
            next_loan_id: 1,
            history: BTreeMap::new(),
            max_history: 32,
            stats: BudgetManagerStats::default(),
        }
    }

    /// Register process with budget
    #[inline(always)]
    pub fn register(&mut self, budget: ProcessBudget) {
        self.budgets.insert(budget.pid, budget);
        self.stats.total_processes = self.budgets.len();
    }

    /// Consume resource
    #[inline]
    pub fn consume(&mut self, pid: u64, resource: BudgetResource, amount: u64) -> BudgetState {
        if let Some(budget) = self.budgets.get_mut(&pid) {
            budget.consume(resource, amount)
        } else {
            BudgetState::Healthy
        }
    }

    /// Lend budget between processes
    pub fn lend(
        &mut self,
        lender_pid: u64,
        borrower_pid: u64,
        resource: BudgetResource,
        amount: u64,
        now: u64,
        duration_ms: u64,
    ) -> Option<u64> {
        // Check lender has enough
        let available = self.budgets.get(&lender_pid)?.available(resource);

        if available < amount {
            return None;
        }

        // Update lender
        if let Some(lender) = self.budgets.get_mut(&lender_pid) {
            if let Some(entry) = lender.resources.get_mut(&(resource as u8)) {
                entry.lent += amount;
            }
        }

        // Update borrower
        if let Some(borrower) = self.budgets.get_mut(&borrower_pid) {
            if let Some(entry) = borrower.resources.get_mut(&(resource as u8)) {
                entry.borrowed += amount;
            }
        }

        let loan_id = self.next_loan_id;
        self.next_loan_id += 1;

        self.loans.push(BudgetLoan {
            id: loan_id,
            lender: lender_pid,
            borrower: borrower_pid,
            resource,
            amount,
            created_at: now,
            expires_at: now + duration_ms,
            returned: false,
        });

        self.stats.active_loans = self.loans.iter().filter(|l| !l.returned).count();
        self.stats.total_loans += 1;

        Some(loan_id)
    }

    /// Return loan
    pub fn return_loan(&mut self, loan_id: u64) -> bool {
        let loan = match self
            .loans
            .iter_mut()
            .find(|l| l.id == loan_id && !l.returned)
        {
            Some(l) => {
                l.returned = true;
                l.clone()
            },
            None => return false,
        };

        // Reverse the lend
        if let Some(lender) = self.budgets.get_mut(&loan.lender) {
            if let Some(entry) = lender.resources.get_mut(&(loan.resource as u8)) {
                entry.lent = entry.lent.saturating_sub(loan.amount);
            }
        }

        if let Some(borrower) = self.budgets.get_mut(&loan.borrower) {
            if let Some(entry) = borrower.resources.get_mut(&(loan.resource as u8)) {
                entry.borrowed = entry.borrowed.saturating_sub(loan.amount);
            }
        }

        self.stats.active_loans = self.loans.iter().filter(|l| !l.returned).count();
        true
    }

    /// Expire old loans
    pub fn expire_loans(&mut self, now: u64) {
        let expired: Vec<u64> = self
            .loans
            .iter()
            .filter(|l| !l.returned && now >= l.expires_at)
            .map(|l| l.id)
            .collect();

        for id in expired {
            self.return_loan(id);
        }
    }

    /// Period reset
    pub fn reset_period(&mut self) {
        // Record history
        for (pid, budget) in &self.budgets {
            let pid_history = self.history.entry(*pid).or_insert_with(BTreeMap::new);
            for (res_key, entry) in &budget.resources {
                let res_hist = pid_history.entry(*res_key).or_insert_with(VecDeque::new);
                res_hist.push_back(entry.usage);
                if res_hist.len() > self.max_history {
                    res_hist.pop_front();
                }
            }
        }

        for budget in self.budgets.values_mut() {
            budget.reset_period();
        }

        self.update_stats();
    }

    /// Forecast budget usage
    pub fn forecast(&self, pid: u64, resource: BudgetResource) -> Option<BudgetForecast> {
        let budget = self.budgets.get(&pid)?;
        let entry = budget.resources.get(&(resource as u8))?;

        let hist = self
            .history
            .get(&pid)
            .and_then(|h| h.get(&(resource as u8)));

        let (predicted, confidence) = match hist {
            Some(h) if h.len() >= 3 => {
                let avg: u64 = h.iter().sum::<u64>() / h.len() as u64;
                // Simple trend: compare last to average
                let last = *h.back().unwrap_or(&0);
                let trend = if last > avg {
                    last + (last - avg) / 4
                } else {
                    avg
                };
                let conf = 0.5 + (h.len() as f64 / (self.max_history as f64 * 2.0)).min(0.4);
                (trend, conf)
            },
            _ => (entry.usage, 0.3),
        };

        let recommended = (predicted as f64 * 1.2) as u64;
        let will_exhaust = predicted > entry.allocation;
        let periods = if predicted > entry.allocation {
            0
        } else if predicted > 0 {
            (entry.allocation - predicted) / predicted.max(1)
        } else {
            u64::MAX
        };

        Some(BudgetForecast {
            pid,
            resource,
            predicted_usage: predicted,
            confidence,
            recommended_allocation: recommended,
            will_exhaust,
            periods_until_exhaust: periods,
        })
    }

    fn update_stats(&mut self) {
        self.stats.total_processes = self.budgets.len();
        self.stats.budget_groups = self.groups.len();
        self.stats.in_deficit = self
            .budgets
            .values()
            .filter(|b| {
                matches!(
                    b.worst_state(),
                    BudgetState::Deficit | BudgetState::Exhausted
                )
            })
            .count();
    }

    /// Get budget
    #[inline(always)]
    pub fn budget(&self, pid: u64) -> Option<&ProcessBudget> {
        self.budgets.get(&pid)
    }

    /// Get stats
    #[inline(always)]
    pub fn stats(&self) -> &BudgetManagerStats {
        &self.stats
    }

    /// Unregister
    #[inline]
    pub fn unregister(&mut self, pid: u64) {
        self.budgets.remove(&pid);
        self.history.remove(&pid);
        self.loans.retain(|l| l.lender != pid && l.borrower != pid);
        self.stats.total_processes = self.budgets.len();
    }
}
