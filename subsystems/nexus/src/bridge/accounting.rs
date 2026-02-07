//! # Bridge Accounting Engine
//!
//! Syscall resource accounting and cost tracking:
//! - Per-syscall cost accounting
//! - Process resource budgets
//! - Cost attribution
//! - Billing summaries
//! - Chargeback reporting

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ACCOUNTING TYPES
// ============================================================================

/// Accountable resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AccountingResource {
    /// CPU time (ns)
    CpuTime,
    /// Memory allocated (bytes)
    Memory,
    /// I/O bytes read
    IoRead,
    /// I/O bytes written
    IoWrite,
    /// Network bytes sent
    NetSend,
    /// Network bytes received
    NetRecv,
    /// Page faults
    PageFaults,
    /// Context switches
    ContextSwitches,
}

/// Cost unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostUnit {
    /// Nanoseconds
    Nanoseconds,
    /// Bytes
    Bytes,
    /// Count
    Count,
    /// Abstract cost units
    CostUnits,
}

// ============================================================================
// SYSCALL COST
// ============================================================================

/// Cost of a single syscall
#[derive(Debug, Clone)]
pub struct SyscallCost {
    /// Syscall number
    pub syscall_nr: u32,
    /// CPU time (ns)
    pub cpu_ns: u64,
    /// Memory touched (bytes)
    pub memory_bytes: u64,
    /// I/O bytes
    pub io_bytes: u64,
    /// Abstract cost
    pub cost_units: u64,
    /// Timestamp
    pub timestamp: u64,
}

impl SyscallCost {
    pub fn new(syscall_nr: u32, cpu_ns: u64, now: u64) -> Self {
        Self {
            syscall_nr,
            cpu_ns,
            memory_bytes: 0,
            io_bytes: 0,
            cost_units: cpu_ns, // Default: cost = cpu time
            timestamp: now,
        }
    }

    /// Total weighted cost
    pub fn total_cost(&self) -> u64 {
        self.cpu_ns + self.memory_bytes / 1024 + self.io_bytes / 512
    }
}

// ============================================================================
// COST MODEL
// ============================================================================

/// Cost model for syscall pricing
#[derive(Debug, Clone)]
pub struct CostModel {
    /// Cost per ns of CPU
    pub cpu_cost_per_ns: f64,
    /// Cost per byte of memory
    pub memory_cost_per_byte: f64,
    /// Cost per byte of I/O
    pub io_cost_per_byte: f64,
    /// Cost per page fault
    pub page_fault_cost: f64,
    /// Cost per context switch
    pub context_switch_cost: f64,
    /// Per-syscall overrides
    overrides: BTreeMap<u32, f64>,
}

impl CostModel {
    pub fn default_model() -> Self {
        Self {
            cpu_cost_per_ns: 1.0,
            memory_cost_per_byte: 0.001,
            io_cost_per_byte: 0.01,
            page_fault_cost: 1000.0,
            context_switch_cost: 5000.0,
            overrides: BTreeMap::new(),
        }
    }

    /// Set override for specific syscall
    pub fn set_override(&mut self, syscall_nr: u32, multiplier: f64) {
        self.overrides.insert(syscall_nr, multiplier);
    }

    /// Calculate cost
    pub fn calculate(&self, cost: &SyscallCost) -> f64 {
        let base = cost.cpu_ns as f64 * self.cpu_cost_per_ns
            + cost.memory_bytes as f64 * self.memory_cost_per_byte
            + cost.io_bytes as f64 * self.io_cost_per_byte;

        let multiplier = self.overrides.get(&cost.syscall_nr).copied().unwrap_or(1.0);
        base * multiplier
    }
}

// ============================================================================
// PROCESS ACCOUNT
// ============================================================================

/// Per-resource counter
#[derive(Debug, Clone, Default)]
pub struct ResourceCounter {
    /// Total usage
    pub total: u64,
    /// Current window usage
    pub window_usage: u64,
    /// Window start
    pub window_start: u64,
}

impl ResourceCounter {
    /// Record usage
    pub fn record(&mut self, amount: u64, now: u64) {
        self.total += amount;
        self.window_usage += amount;
    }

    /// Reset window
    pub fn reset_window(&mut self, now: u64) {
        self.window_usage = 0;
        self.window_start = now;
    }
}

/// Process accounting record
#[derive(Debug, Clone)]
pub struct ProcessAccount {
    /// Process id
    pub pid: u64,
    /// Per-resource counters
    pub resources: BTreeMap<u8, ResourceCounter>,
    /// Total cost
    pub total_cost: f64,
    /// Window cost
    pub window_cost: f64,
    /// Budget limit (None = unlimited)
    pub budget: Option<f64>,
    /// Syscall counts
    pub syscall_counts: BTreeMap<u32, u64>,
    /// Total syscalls
    pub total_syscalls: u64,
}

impl ProcessAccount {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            resources: BTreeMap::new(),
            total_cost: 0.0,
            window_cost: 0.0,
            budget: None,
            syscall_counts: BTreeMap::new(),
            total_syscalls: 0,
        }
    }

    /// Record resource usage
    pub fn record_resource(&mut self, resource: AccountingResource, amount: u64, now: u64) {
        let counter = self
            .resources
            .entry(resource as u8)
            .or_insert_with(ResourceCounter::default);
        counter.record(amount, now);
    }

    /// Record cost
    pub fn record_cost(&mut self, cost: f64, syscall_nr: u32) {
        self.total_cost += cost;
        self.window_cost += cost;
        *self.syscall_counts.entry(syscall_nr).or_insert(0) += 1;
        self.total_syscalls += 1;
    }

    /// Is over budget?
    pub fn is_over_budget(&self) -> bool {
        self.budget.map(|b| self.window_cost > b).unwrap_or(false)
    }

    /// Budget utilization
    pub fn budget_utilization(&self) -> Option<f64> {
        self.budget.map(|b| {
            if b > 0.0 {
                self.window_cost / b
            } else {
                0.0
            }
        })
    }

    /// Top syscalls by cost
    pub fn top_syscalls(&self, count: usize) -> Vec<(u32, u64)> {
        let mut entries: Vec<(u32, u64)> = self.syscall_counts.iter().map(|(&k, &v)| (k, v)).collect();
        entries.sort_by(|a, b| b.1.cmp(&a.1));
        entries.truncate(count);
        entries
    }

    /// Reset window
    pub fn reset_window(&mut self, now: u64) {
        self.window_cost = 0.0;
        for counter in self.resources.values_mut() {
            counter.reset_window(now);
        }
    }
}

// ============================================================================
// ACCOUNTING ENGINE
// ============================================================================

/// Accounting stats
#[derive(Debug, Clone, Default)]
pub struct BridgeAccountingStats {
    /// Processes tracked
    pub processes: usize,
    /// Total cost recorded
    pub total_cost: f64,
    /// Syscalls accounted
    pub syscalls_accounted: u64,
    /// Over-budget processes
    pub over_budget: usize,
}

/// Bridge accounting engine
pub struct BridgeAccountingEngine {
    /// Process accounts
    accounts: BTreeMap<u64, ProcessAccount>,
    /// Cost model
    model: CostModel,
    /// Stats
    stats: BridgeAccountingStats,
}

impl BridgeAccountingEngine {
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
            model: CostModel::default_model(),
            stats: BridgeAccountingStats::default(),
        }
    }

    /// Set cost model
    pub fn set_model(&mut self, model: CostModel) {
        self.model = model;
    }

    /// Set budget
    pub fn set_budget(&mut self, pid: u64, budget: f64) {
        let account = self
            .accounts
            .entry(pid)
            .or_insert_with(|| ProcessAccount::new(pid));
        account.budget = Some(budget);
    }

    /// Record syscall cost
    pub fn record(&mut self, pid: u64, cost: SyscallCost) {
        let calculated = self.model.calculate(&cost);
        let syscall_nr = cost.syscall_nr;

        let account = self
            .accounts
            .entry(pid)
            .or_insert_with(|| ProcessAccount::new(pid));
        account.record_cost(calculated, syscall_nr);
        account.record_resource(AccountingResource::CpuTime, cost.cpu_ns, cost.timestamp);
        if cost.memory_bytes > 0 {
            account.record_resource(AccountingResource::Memory, cost.memory_bytes, cost.timestamp);
        }
        if cost.io_bytes > 0 {
            account.record_resource(AccountingResource::IoRead, cost.io_bytes, cost.timestamp);
        }

        self.stats.total_cost += calculated;
        self.stats.syscalls_accounted += 1;
        self.stats.processes = self.accounts.len();
        self.stats.over_budget = self
            .accounts
            .values()
            .filter(|a| a.is_over_budget())
            .count();
    }

    /// Get account
    pub fn account(&self, pid: u64) -> Option<&ProcessAccount> {
        self.accounts.get(&pid)
    }

    /// Top spenders
    pub fn top_spenders(&self, count: usize) -> Vec<(u64, f64)> {
        let mut spenders: Vec<(u64, f64)> = self
            .accounts
            .values()
            .map(|a| (a.pid, a.total_cost))
            .collect();
        spenders.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        spenders.truncate(count);
        spenders
    }

    /// Over-budget processes
    pub fn over_budget_processes(&self) -> Vec<u64> {
        self.accounts
            .values()
            .filter(|a| a.is_over_budget())
            .map(|a| a.pid)
            .collect()
    }

    /// Reset all windows
    pub fn reset_windows(&mut self, now: u64) {
        for account in self.accounts.values_mut() {
            account.reset_window(now);
        }
    }

    /// Stats
    pub fn stats(&self) -> &BridgeAccountingStats {
        &self.stats
    }
}
