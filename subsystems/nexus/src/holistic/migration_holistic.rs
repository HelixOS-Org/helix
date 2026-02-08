//! # Holistic Migration Coordinator
//!
//! System-wide process and resource migration:
//! - Live migration planning
//! - Cost-benefit analysis
//! - Migration execution tracking
//! - NUMA-aware migration
//! - Migration impact assessment

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// MIGRATION TYPES
// ============================================================================

/// Migration reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticMigrationReason {
    /// Load balancing
    LoadBalance,
    /// Thermal avoidance
    ThermalAvoidance,
    /// Power consolidation
    PowerConsolidation,
    /// NUMA locality
    NumaLocality,
    /// Affinity optimization
    AffinityOptimization,
    /// Failure recovery
    FailureRecovery,
    /// SLA compliance
    SlaCompliance,
    /// Manual request
    ManualRequest,
}

/// Migration target
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationTarget {
    /// CPU core
    Core(u32),
    /// NUMA node
    NumaNode(u32),
    /// CPU cluster
    Cluster(u32),
}

/// Migration state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolisticMigrationState {
    /// Planned
    Planned,
    /// Approved
    Approved,
    /// InProgress
    InProgress,
    /// Completed
    Completed,
    /// Failed
    Failed,
    /// Cancelled
    Cancelled,
}

/// Migration priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MigrationPriority {
    /// Low
    Low,
    /// Normal
    Normal,
    /// High
    High,
    /// Urgent
    Urgent,
}

// ============================================================================
// MIGRATION PLAN
// ============================================================================

/// Cost/benefit analysis
#[derive(Debug, Clone)]
pub struct MigrationCostBenefit {
    /// Estimated migration cost (ns)
    pub cost_ns: u64,
    /// Expected latency improvement
    pub latency_benefit: f64,
    /// Expected throughput improvement
    pub throughput_benefit: f64,
    /// Power saving
    pub power_benefit: f64,
    /// Thermal improvement
    pub thermal_benefit: f64,
    /// Cache disruption cost
    pub cache_disruption: f64,
    /// TLB flush cost
    pub tlb_flush_cost: f64,
}

impl MigrationCostBenefit {
    /// Net benefit score
    pub fn net_benefit(&self) -> f64 {
        let benefit = self.latency_benefit * 0.3
            + self.throughput_benefit * 0.3
            + self.power_benefit * 0.2
            + self.thermal_benefit * 0.2;
        let cost = self.cache_disruption * 0.5 + self.tlb_flush_cost * 0.5;
        benefit - cost
    }

    /// Is migration worthwhile?
    pub fn is_worthwhile(&self) -> bool {
        self.net_benefit() > 0.1
    }
}

/// Migration request
#[derive(Debug, Clone)]
pub struct MigrationRequest {
    /// Request id
    pub id: u64,
    /// Process to migrate
    pub pid: u64,
    /// Source
    pub source: MigrationTarget,
    /// Destination
    pub destination: MigrationTarget,
    /// Reason
    pub reason: HolisticMigrationReason,
    /// Priority
    pub priority: MigrationPriority,
    /// State
    pub state: HolisticMigrationState,
    /// Cost/benefit
    pub analysis: MigrationCostBenefit,
    /// Created at
    pub created_at: u64,
    /// Started at
    pub started_at: Option<u64>,
    /// Completed at
    pub completed_at: Option<u64>,
    /// Memory to migrate (bytes)
    pub memory_bytes: u64,
}

impl MigrationRequest {
    pub fn new(
        id: u64,
        pid: u64,
        source: MigrationTarget,
        destination: MigrationTarget,
        reason: HolisticMigrationReason,
        now: u64,
    ) -> Self {
        Self {
            id,
            pid,
            source,
            destination,
            reason,
            priority: MigrationPriority::Normal,
            state: HolisticMigrationState::Planned,
            analysis: MigrationCostBenefit {
                cost_ns: 0,
                latency_benefit: 0.0,
                throughput_benefit: 0.0,
                power_benefit: 0.0,
                thermal_benefit: 0.0,
                cache_disruption: 0.0,
                tlb_flush_cost: 0.0,
            },
            created_at: now,
            started_at: None,
            completed_at: None,
            memory_bytes: 0,
        }
    }

    /// Approve
    pub fn approve(&mut self) {
        if matches!(self.state, HolisticMigrationState::Planned) {
            self.state = HolisticMigrationState::Approved;
        }
    }

    /// Start
    pub fn start(&mut self, now: u64) {
        if matches!(self.state, HolisticMigrationState::Approved) {
            self.state = HolisticMigrationState::InProgress;
            self.started_at = Some(now);
        }
    }

    /// Complete
    pub fn complete(&mut self, now: u64) {
        if matches!(self.state, HolisticMigrationState::InProgress) {
            self.state = HolisticMigrationState::Completed;
            self.completed_at = Some(now);
        }
    }

    /// Fail
    pub fn fail(&mut self) {
        if matches!(self.state, HolisticMigrationState::InProgress) {
            self.state = HolisticMigrationState::Failed;
        }
    }

    /// Cancel
    pub fn cancel(&mut self) {
        if !matches!(
            self.state,
            HolisticMigrationState::Completed | HolisticMigrationState::Failed
        ) {
            self.state = HolisticMigrationState::Cancelled;
        }
    }

    /// Duration (ns)
    pub fn duration_ns(&self) -> Option<u64> {
        match (self.started_at, self.completed_at) {
            (Some(s), Some(c)) => Some(c.saturating_sub(s)),
            _ => None,
        }
    }

    /// Migration bandwidth (bytes/ns)
    pub fn bandwidth(&self) -> Option<f64> {
        self.duration_ns()
            .filter(|&d| d > 0)
            .map(|d| self.memory_bytes as f64 / d as f64)
    }
}

// ============================================================================
// MIGRATION HISTORY
// ============================================================================

/// Migration outcome for a process
#[derive(Debug, Clone)]
pub struct ProcessMigrationHistory {
    /// Process id
    pub pid: u64,
    /// Total migrations
    pub total: u64,
    /// Successful migrations
    pub successful: u64,
    /// Failed migrations
    pub failed: u64,
    /// Total downtime (ns)
    pub total_downtime_ns: u64,
    /// Last migration time
    pub last_migration: u64,
    /// Average migration duration
    avg_duration_ns: f64,
}

impl ProcessMigrationHistory {
    pub fn new(pid: u64) -> Self {
        Self {
            pid,
            total: 0,
            successful: 0,
            failed: 0,
            total_downtime_ns: 0,
            last_migration: 0,
            avg_duration_ns: 0.0,
        }
    }

    /// Record outcome
    pub fn record(&mut self, success: bool, duration_ns: u64, now: u64) {
        self.total += 1;
        if success {
            self.successful += 1;
        } else {
            self.failed += 1;
        }
        self.total_downtime_ns += duration_ns;
        self.last_migration = now;

        // Running average
        let alpha = 0.2;
        self.avg_duration_ns = alpha * duration_ns as f64 + (1.0 - alpha) * self.avg_duration_ns;
    }

    /// Success rate
    pub fn success_rate(&self) -> f64 {
        if self.total == 0 {
            return 1.0;
        }
        self.successful as f64 / self.total as f64
    }

    /// Average duration
    pub fn avg_duration(&self) -> f64 {
        self.avg_duration_ns
    }
}

// ============================================================================
// MIGRATION ENGINE
// ============================================================================

/// Migration stats
#[derive(Debug, Clone, Default)]
pub struct HolisticMigrationStats {
    /// Total requests
    pub total_requests: u64,
    /// Active migrations
    pub active: usize,
    /// Completed
    pub completed: u64,
    /// Failed
    pub failed: u64,
    /// Cancelled
    pub cancelled: u64,
    /// Approved pending
    pub approved_pending: usize,
    /// Average benefit score
    pub avg_benefit: f64,
}

/// Holistic migration coordinator
pub struct HolisticMigrationEngine {
    /// Pending requests
    requests: BTreeMap<u64, MigrationRequest>,
    /// Process histories
    histories: BTreeMap<u64, ProcessMigrationHistory>,
    /// Next id
    next_id: u64,
    /// Max concurrent migrations
    max_concurrent: usize,
    /// Stats
    stats: HolisticMigrationStats,
}

impl HolisticMigrationEngine {
    pub fn new() -> Self {
        Self {
            requests: BTreeMap::new(),
            histories: BTreeMap::new(),
            next_id: 1,
            max_concurrent: 4,
            stats: HolisticMigrationStats::default(),
        }
    }

    /// Plan migration
    pub fn plan(
        &mut self,
        pid: u64,
        source: MigrationTarget,
        dest: MigrationTarget,
        reason: HolisticMigrationReason,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let req = MigrationRequest::new(id, pid, source, dest, reason, now);
        self.requests.insert(id, req);
        self.stats.total_requests += 1;
        id
    }

    /// Set cost/benefit
    pub fn set_analysis(&mut self, id: u64, analysis: MigrationCostBenefit) {
        if let Some(req) = self.requests.get_mut(&id) {
            req.analysis = analysis;
        }
    }

    /// Approve migration
    pub fn approve(&mut self, id: u64) -> bool {
        if let Some(req) = self.requests.get_mut(&id) {
            req.approve();
            self.update_counts();
            true
        } else {
            false
        }
    }

    /// Auto-approve if beneficial
    pub fn auto_approve(&mut self) -> Vec<u64> {
        let planned: Vec<u64> = self
            .requests
            .iter()
            .filter(|(_, r)| matches!(r.state, HolisticMigrationState::Planned))
            .filter(|(_, r)| r.analysis.is_worthwhile())
            .map(|(&id, _)| id)
            .collect();

        for &id in &planned {
            if let Some(req) = self.requests.get_mut(&id) {
                req.approve();
            }
        }
        self.update_counts();
        planned
    }

    /// Start next approved migration
    pub fn start_next(&mut self, now: u64) -> Option<u64> {
        let active = self
            .requests
            .values()
            .filter(|r| matches!(r.state, HolisticMigrationState::InProgress))
            .count();

        if active >= self.max_concurrent {
            return None;
        }

        // Find highest priority approved request
        let best = self
            .requests
            .iter()
            .filter(|(_, r)| matches!(r.state, HolisticMigrationState::Approved))
            .max_by_key(|(_, r)| r.priority)
            .map(|(&id, _)| id);

        if let Some(id) = best {
            if let Some(req) = self.requests.get_mut(&id) {
                req.start(now);
                self.update_counts();
            }
        }
        best
    }

    /// Complete migration
    pub fn complete(&mut self, id: u64, now: u64) -> bool {
        if let Some(req) = self.requests.get_mut(&id) {
            let pid = req.pid;
            req.complete(now);
            let duration = req.duration_ns().unwrap_or(0);

            let history = self
                .histories
                .entry(pid)
                .or_insert_with(|| ProcessMigrationHistory::new(pid));
            history.record(true, duration, now);

            self.stats.completed += 1;
            self.update_counts();
            true
        } else {
            false
        }
    }

    /// Fail migration
    pub fn fail_migration(&mut self, id: u64, now: u64) -> bool {
        if let Some(req) = self.requests.get_mut(&id) {
            let pid = req.pid;
            req.fail();

            let history = self
                .histories
                .entry(pid)
                .or_insert_with(|| ProcessMigrationHistory::new(pid));
            history.record(false, 0, now);

            self.stats.failed += 1;
            self.update_counts();
            true
        } else {
            false
        }
    }

    /// Cancel migration
    pub fn cancel(&mut self, id: u64) -> bool {
        if let Some(req) = self.requests.get_mut(&id) {
            req.cancel();
            self.stats.cancelled += 1;
            self.update_counts();
            true
        } else {
            false
        }
    }

    /// Process history
    pub fn history(&self, pid: u64) -> Option<&ProcessMigrationHistory> {
        self.histories.get(&pid)
    }

    fn update_counts(&mut self) {
        self.stats.active = self
            .requests
            .values()
            .filter(|r| matches!(r.state, HolisticMigrationState::InProgress))
            .count();
        self.stats.approved_pending = self
            .requests
            .values()
            .filter(|r| matches!(r.state, HolisticMigrationState::Approved))
            .count();

        let benefits: Vec<f64> = self
            .requests
            .values()
            .filter(|r| matches!(r.state, HolisticMigrationState::Completed))
            .map(|r| r.analysis.net_benefit())
            .collect();
        if !benefits.is_empty() {
            self.stats.avg_benefit = benefits.iter().sum::<f64>() / benefits.len() as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &HolisticMigrationStats {
        &self.stats
    }
}
