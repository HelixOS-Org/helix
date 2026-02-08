// SPDX-License-Identifier: GPL-2.0
//! Holistic sched_domains — scheduler domain hierarchy and load balancing topology.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Domain level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DomainLevel {
    /// SMT siblings
    Smt,
    /// Cores in same cluster
    Cluster,
    /// Cores in same LLC / die
    Mc,
    /// Cores in same package
    Die,
    /// Same NUMA node
    Numa,
    /// Cross-NUMA
    NumaCross,
    /// System-wide
    System,
}

impl DomainLevel {
    pub fn balance_interval_ms(&self) -> u64 {
        match self {
            Self::Smt => 1,
            Self::Cluster => 2,
            Self::Mc => 4,
            Self::Die => 8,
            Self::Numa => 16,
            Self::NumaCross => 64,
            Self::System => 128,
        }
    }

    pub fn migration_cost_ns(&self) -> u64 {
        match self {
            Self::Smt => 500,
            Self::Cluster => 2_000,
            Self::Mc => 10_000,
            Self::Die => 50_000,
            Self::Numa => 500_000,
            Self::NumaCross => 2_000_000,
            Self::System => 5_000_000,
        }
    }
}

/// Domain flags
#[derive(Debug, Clone, Copy)]
pub struct DomainFlags(pub u32);

impl DomainFlags {
    pub const LOAD_BALANCE: Self = Self(0x01);
    pub const BALANCE_NEWIDLE: Self = Self(0x02);
    pub const BALANCE_EXEC: Self = Self(0x04);
    pub const BALANCE_FORK: Self = Self(0x08);
    pub const BALANCE_WAKE: Self = Self(0x10);
    pub const WAKE_AFFINE: Self = Self(0x20);
    pub const SHARE_CPUPOWER: Self = Self(0x40);
    pub const SHARE_POWERDOMAIN: Self = Self(0x80);
    pub const SERIALIZE: Self = Self(0x100);
    pub const PREFER_SIBLING: Self = Self(0x200);
    pub const OVERLAP: Self = Self(0x400);

    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn count_set(&self) -> u32 {
        self.0.count_ones()
    }
}

/// Scheduler group within a domain
#[derive(Debug, Clone)]
pub struct SchedGroup {
    pub id: u32,
    pub cpus: Vec<u32>,
    pub capacity: u64,
    pub load: u64,
    pub nr_running: u32,
    pub group_weight: u32,
    pub group_type: GroupType,
}

impl SchedGroup {
    pub fn new(id: u32, cpus: Vec<u32>) -> Self {
        let w = cpus.len() as u32;
        Self {
            id, cpus, capacity: 0, load: 0,
            nr_running: 0, group_weight: w,
            group_type: GroupType::Other,
        }
    }

    pub fn avg_load(&self) -> u64 {
        if self.group_weight == 0 { return 0; }
        self.load / self.group_weight as u64
    }

    pub fn is_idle(&self) -> bool {
        self.nr_running == 0
    }

    pub fn is_overloaded(&self) -> bool {
        self.nr_running as u64 > self.capacity
    }

    pub fn imbalance_vs(&self, other: &SchedGroup) -> i64 {
        self.avg_load() as i64 - other.avg_load() as i64
    }
}

/// Group classification for load balancing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GroupType {
    HasSpare,
    FullyBusy,
    Misfit,
    Asym,
    Imbalanced,
    Overloaded,
    Other,
}

/// A scheduler domain
#[derive(Debug)]
pub struct SchedDomain {
    pub id: u32,
    pub name: String,
    pub level: DomainLevel,
    pub flags: DomainFlags,
    pub span: Vec<u32>,
    pub groups: Vec<SchedGroup>,
    pub parent_id: Option<u32>,
    pub child_id: Option<u32>,
    pub balance_count: u64,
    pub balance_failed: u64,
    pub idle_count: u64,
    pub max_newidle_lb_cost_ns: u64,
    pub last_balance_timestamp: u64,
    pub busy_factor: u32,
    pub imbalance_pct: u32,
}

impl SchedDomain {
    pub fn new(id: u32, name: String, level: DomainLevel) -> Self {
        Self {
            id, name, level,
            flags: DomainFlags(DomainFlags::LOAD_BALANCE.0
                | DomainFlags::BALANCE_NEWIDLE.0
                | DomainFlags::BALANCE_WAKE.0),
            span: Vec::new(),
            groups: Vec::new(),
            parent_id: None,
            child_id: None,
            balance_count: 0,
            balance_failed: 0,
            idle_count: 0,
            max_newidle_lb_cost_ns: 0,
            last_balance_timestamp: 0,
            busy_factor: 32,
            imbalance_pct: 125,
        }
    }

    pub fn cpu_count(&self) -> usize {
        self.span.len()
    }

    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn balance_success_rate(&self) -> f64 {
        let total = self.balance_count;
        if total == 0 { return 1.0; }
        1.0 - (self.balance_failed as f64 / total as f64)
    }

    pub fn busiest_group(&self) -> Option<&SchedGroup> {
        self.groups.iter().max_by_key(|g| g.load)
    }

    pub fn idlest_group(&self) -> Option<&SchedGroup> {
        self.groups.iter().min_by_key(|g| g.load)
    }

    pub fn load_imbalance(&self) -> u64 {
        if let (Some(busiest), Some(idlest)) = (self.busiest_group(), self.idlest_group()) {
            busiest.avg_load().saturating_sub(idlest.avg_load())
        } else {
            0
        }
    }

    pub fn total_load(&self) -> u64 {
        self.groups.iter().map(|g| g.load).sum()
    }

    pub fn total_capacity(&self) -> u64 {
        self.groups.iter().map(|g| g.capacity).sum()
    }
}

/// Balance decision
#[derive(Debug, Clone)]
pub struct BalanceDecision {
    pub domain_id: u32,
    pub src_group: u32,
    pub dst_group: u32,
    pub nr_tasks_moved: u32,
    pub load_moved: u64,
    pub reason: BalanceReason,
    pub timestamp: u64,
}

/// Balance reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceReason {
    Regular,
    NewIdle,
    Fork,
    Exec,
    Wake,
    Forced,
}

/// Sched domains stats
#[derive(Debug, Clone)]
pub struct SchedDomainsStats {
    pub total_domains: u32,
    pub total_groups: u32,
    pub balance_attempts: u64,
    pub tasks_moved: u64,
    pub load_moved: u64,
    pub newidle_balances: u64,
}

/// Main scheduler domains manager
pub struct HolisticSchedDomains {
    domains: BTreeMap<u32, SchedDomain>,
    cpu_to_leaf: BTreeMap<u32, u32>,
    history: Vec<BalanceDecision>,
    max_history: usize,
    next_id: u32,
    stats: SchedDomainsStats,
}

impl HolisticSchedDomains {
    pub fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            cpu_to_leaf: BTreeMap::new(),
            history: Vec::new(),
            max_history: 2048,
            next_id: 1,
            stats: SchedDomainsStats {
                total_domains: 0, total_groups: 0,
                balance_attempts: 0, tasks_moved: 0,
                load_moved: 0, newidle_balances: 0,
            },
        }
    }

    pub fn add_domain(&mut self, mut domain: SchedDomain) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        domain.id = id;
        self.stats.total_domains += 1;
        self.stats.total_groups += domain.group_count() as u32;
        // map cpus to leaf domain
        if domain.child_id.is_none() {
            for &cpu in &domain.span {
                self.cpu_to_leaf.insert(cpu, id);
            }
        }
        self.domains.insert(id, domain);
        id
    }

    pub fn record_balance(&mut self, decision: BalanceDecision) {
        self.stats.balance_attempts += 1;
        self.stats.tasks_moved += decision.nr_tasks_moved as u64;
        self.stats.load_moved += decision.load_moved;
        if decision.reason == BalanceReason::NewIdle {
            self.stats.newidle_balances += 1;
        }
        if let Some(d) = self.domains.get_mut(&decision.domain_id) {
            d.balance_count += 1;
        }
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(decision);
    }

    pub fn domain_chain(&self, cpu: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        if let Some(&leaf) = self.cpu_to_leaf.get(&cpu) {
            let mut cur = Some(leaf);
            while let Some(id) = cur {
                chain.push(id);
                cur = self.domains.get(&id).and_then(|d| d.parent_id);
            }
        }
        chain
    }

    pub fn most_imbalanced(&self, n: usize) -> Vec<(u32, u64)> {
        let mut v: Vec<_> = self.domains.iter()
            .map(|(&id, d)| (id, d.load_imbalance()))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(n);
        v
    }

    pub fn get_domain(&self, id: u32) -> Option<&SchedDomain> {
        self.domains.get(&id)
    }

    pub fn stats(&self) -> &SchedDomainsStats {
        &self.stats
    }
}
