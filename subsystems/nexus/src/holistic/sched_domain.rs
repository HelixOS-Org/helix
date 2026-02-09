//! # Holistic Scheduling Domain
//!
//! Scheduling domain hierarchy management:
//! - Multi-level scheduling domains (SMT, Core, LLC, NUMA, System)
//! - Load balancing across domains
//! - Domain-aware task placement
//! - Imbalance detection and migration
//! - Power-aware domain management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use libm::sqrt;

// ============================================================================
// DOMAIN TYPES
// ============================================================================

/// Scheduling domain level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchedDomainLevel {
    /// Simultaneous multithreading (hyperthreads)
    Smt,
    /// Physical core
    Core,
    /// Last-level cache
    Llc,
    /// NUMA node
    Numa,
    /// Full system
    System,
}

/// Domain balance state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainBalanceState {
    /// Balanced within threshold
    Balanced,
    /// Slightly imbalanced
    Imbalanced,
    /// Severely imbalanced, needs migration
    Critical,
}

/// Migration urgency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationUrgency {
    /// Can wait for next balance tick
    Low,
    /// Should migrate soon
    Medium,
    /// Migrate immediately (affinity violation or starvation)
    High,
    /// Emergency (CPU overloaded, others idle)
    Emergency,
}

// ============================================================================
// CPU GROUP
// ============================================================================

/// CPU group within a domain
#[derive(Debug, Clone)]
pub struct SchedCpuGroup {
    /// Group ID
    pub group_id: u32,
    /// CPU IDs in this group
    pub cpus: Vec<u32>,
    /// Total load (abstract units)
    pub total_load: u64,
    /// Runqueue length sum
    pub total_runqueue: u32,
    /// Capacity (abstract units)
    pub capacity: u64,
    /// Idle CPUs count
    pub idle_cpus: u32,
}

impl SchedCpuGroup {
    pub fn new(group_id: u32) -> Self {
        Self {
            group_id,
            cpus: Vec::new(),
            total_load: 0,
            total_runqueue: 0,
            capacity: 0,
            idle_cpus: 0,
        }
    }

    /// Average load per CPU
    #[inline]
    pub fn avg_load(&self) -> f64 {
        if self.cpus.is_empty() {
            return 0.0;
        }
        self.total_load as f64 / self.cpus.len() as f64
    }

    /// Utilization
    #[inline]
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.total_load as f64 / self.capacity as f64
    }

    /// Has spare capacity?
    #[inline(always)]
    pub fn has_spare(&self) -> bool {
        self.idle_cpus > 0 || self.utilization() < 0.8
    }
}

// ============================================================================
// SCHEDULING DOMAIN
// ============================================================================

/// Scheduling domain
#[derive(Debug)]
pub struct SchedDomain {
    /// Domain ID
    pub domain_id: u32,
    /// Level
    pub level: SchedDomainLevel,
    /// Groups in this domain
    groups: Vec<SchedCpuGroup>,
    /// Balance interval (ticks)
    pub balance_interval: u32,
    /// Ticks since last balance
    pub ticks_since_balance: u32,
    /// Imbalance threshold (percentage)
    pub imbalance_threshold: f64,
    /// Balance state
    pub state: DomainBalanceState,
    /// Migration cost (ns, for this domain level)
    pub migration_cost_ns: u64,
    /// Total balances performed
    pub total_balances: u64,
    /// Total migrations
    pub total_migrations: u64,
}

impl SchedDomain {
    pub fn new(domain_id: u32, level: SchedDomainLevel) -> Self {
        let (interval, threshold, cost) = match level {
            SchedDomainLevel::Smt => (1, 0.1, 1000),
            SchedDomainLevel::Core => (2, 0.15, 5000),
            SchedDomainLevel::Llc => (4, 0.2, 50000),
            SchedDomainLevel::Numa => (8, 0.3, 500000),
            SchedDomainLevel::System => (16, 0.4, 5000000),
        };
        Self {
            domain_id,
            level,
            groups: Vec::new(),
            balance_interval: interval,
            ticks_since_balance: 0,
            imbalance_threshold: threshold,
            state: DomainBalanceState::Balanced,
            migration_cost_ns: cost,
            total_balances: 0,
            total_migrations: 0,
        }
    }

    /// Add group
    #[inline(always)]
    pub fn add_group(&mut self, group: SchedCpuGroup) {
        self.groups.push(group);
    }

    /// Check if balance is needed
    #[inline(always)]
    pub fn needs_balance(&self) -> bool {
        self.ticks_since_balance >= self.balance_interval
    }

    /// Calculate imbalance
    pub fn calculate_imbalance(&self) -> f64 {
        if self.groups.len() < 2 {
            return 0.0;
        }
        let loads: Vec<f64> = self.groups.iter().map(|g| g.avg_load()).collect();
        let mean = loads.iter().sum::<f64>() / loads.len() as f64;
        if mean < 1.0 {
            return 0.0;
        }
        let variance = loads.iter()
            .map(|l| (l - mean) * (l - mean))
            .sum::<f64>() / loads.len() as f64;
        sqrt(variance) / mean
    }

    /// Find busiest and idlest groups
    pub fn find_imbalance_pair(&self) -> Option<(usize, usize)> {
        if self.groups.len() < 2 {
            return None;
        }
        let mut busiest_idx = 0;
        let mut busiest_load = 0.0f64;
        let mut idlest_idx = 0;
        let mut idlest_load = f64::MAX;

        for (i, g) in self.groups.iter().enumerate() {
            let load = g.avg_load();
            if load > busiest_load {
                busiest_load = load;
                busiest_idx = i;
            }
            if load < idlest_load {
                idlest_load = load;
                idlest_idx = i;
            }
        }

        if busiest_idx != idlest_idx && busiest_load > 0.0 {
            let diff = (busiest_load - idlest_load) / busiest_load;
            if diff > self.imbalance_threshold {
                return Some((busiest_idx, idlest_idx));
            }
        }
        None
    }

    /// Tick
    #[inline]
    pub fn tick(&mut self) {
        self.ticks_since_balance += 1;
        let imbalance = self.calculate_imbalance();
        self.state = if imbalance > self.imbalance_threshold * 2.0 {
            DomainBalanceState::Critical
        } else if imbalance > self.imbalance_threshold {
            DomainBalanceState::Imbalanced
        } else {
            DomainBalanceState::Balanced
        };
    }

    /// Balance (returns number of suggested migrations)
    pub fn balance(&mut self) -> u32 {
        self.ticks_since_balance = 0;
        self.total_balances += 1;

        if let Some((busiest, idlest)) = self.find_imbalance_pair() {
            let busy_load = self.groups[busiest].avg_load();
            let idle_load = self.groups[idlest].avg_load();
            let transfer = ((busy_load - idle_load) / 2.0) as u32;
            let migrations = transfer.min(self.groups[busiest].total_runqueue / 2).max(1);
            self.total_migrations += migrations as u64;
            migrations
        } else {
            0
        }
    }
}

// ============================================================================
// MIGRATION SUGGESTION
// ============================================================================

/// Task migration suggestion
#[derive(Debug, Clone)]
pub struct SchedMigrationSuggestion {
    /// Domain level
    pub level: SchedDomainLevel,
    /// Source group ID
    pub src_group: u32,
    /// Destination group ID
    pub dst_group: u32,
    /// Number of tasks to migrate
    pub task_count: u32,
    /// Urgency
    pub urgency: MigrationUrgency,
    /// Estimated cost (ns)
    pub cost_ns: u64,
}

// ============================================================================
// ENGINE
// ============================================================================

/// Scheduling domain stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct HolisticSchedDomainStats {
    /// Active domains
    pub active_domains: usize,
    /// Total groups
    pub total_groups: usize,
    /// Imbalanced domains
    pub imbalanced_domains: usize,
    /// Total balances
    pub total_balances: u64,
    /// Total migrations
    pub total_migrations: u64,
    /// Worst imbalance
    pub worst_imbalance: f64,
}

/// System-wide scheduling domain manager
pub struct HolisticSchedDomain {
    /// Domains by level then ID
    domains: BTreeMap<u32, SchedDomain>,
    /// Stats
    stats: HolisticSchedDomainStats,
}

impl HolisticSchedDomain {
    pub fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            stats: HolisticSchedDomainStats::default(),
        }
    }

    /// Create domain
    #[inline(always)]
    pub fn create_domain(&mut self, domain_id: u32, level: SchedDomainLevel) {
        self.domains.insert(domain_id, SchedDomain::new(domain_id, level));
        self.update_stats();
    }

    /// Add group to domain
    #[inline]
    pub fn add_group(&mut self, domain_id: u32, group: SchedCpuGroup) {
        if let Some(domain) = self.domains.get_mut(&domain_id) {
            domain.add_group(group);
        }
        self.update_stats();
    }

    /// Tick all domains
    pub fn tick(&mut self) -> Vec<SchedMigrationSuggestion> {
        let mut suggestions = Vec::new();
        let domain_ids: Vec<u32> = self.domains.keys().copied().collect();

        for &did in &domain_ids {
            if let Some(domain) = self.domains.get_mut(&did) {
                domain.tick();
                if domain.needs_balance() {
                    let migrations = domain.balance();
                    if migrations > 0 {
                        if let Some((src, dst)) = domain.find_imbalance_pair() {
                            let urgency = match domain.state {
                                DomainBalanceState::Critical => MigrationUrgency::Emergency,
                                DomainBalanceState::Imbalanced => MigrationUrgency::Medium,
                                DomainBalanceState::Balanced => MigrationUrgency::Low,
                            };
                            suggestions.push(SchedMigrationSuggestion {
                                level: domain.level,
                                src_group: domain.groups.get(src).map(|g| g.group_id).unwrap_or(0),
                                dst_group: domain.groups.get(dst).map(|g| g.group_id).unwrap_or(0),
                                task_count: migrations,
                                urgency,
                                cost_ns: domain.migration_cost_ns * migrations as u64,
                            });
                        }
                    }
                }
            }
        }
        self.update_stats();
        suggestions
    }

    fn update_stats(&mut self) {
        self.stats.active_domains = self.domains.len();
        self.stats.total_groups = self.domains.values()
            .map(|d| d.groups.len())
            .sum();
        self.stats.imbalanced_domains = self.domains.values()
            .filter(|d| !matches!(d.state, DomainBalanceState::Balanced))
            .count();
        self.stats.total_balances = self.domains.values().map(|d| d.total_balances).sum();
        self.stats.total_migrations = self.domains.values().map(|d| d.total_migrations).sum();
        self.stats.worst_imbalance = self.domains.values()
            .map(|d| d.calculate_imbalance())
            .fold(0.0_f64, f64::max);
    }

    /// Stats
    #[inline(always)]
    pub fn stats(&self) -> &HolisticSchedDomainStats {
        &self.stats
    }
}
