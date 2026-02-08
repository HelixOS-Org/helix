//! # Cooperative Migration Coordination
//!
//! Process migration coordination between cooperating entities:
//! - Pre-migration negotiation
//! - State transfer coordination
//! - Post-migration verification
//! - Live migration with minimal downtime
//! - Migration rollback support

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// MIGRATION TYPES
// ============================================================================

/// Migration state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopMigrationState {
    /// Planning phase
    Planning,
    /// Negotiating with target
    Negotiating,
    /// Pre-copy (transferring state)
    PreCopy,
    /// Stop-and-copy (final transfer)
    StopAndCopy,
    /// Resuming on target
    Resuming,
    /// Complete
    Complete,
    /// Failed
    Failed,
    /// Rolled back
    RolledBack,
}

/// Migration reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoopMigrationReason {
    /// Load balancing
    LoadBalance,
    /// Thermal management
    Thermal,
    /// Energy optimization
    Energy,
    /// Affinity improvement
    Affinity,
    /// Maintenance (source going down)
    Maintenance,
    /// User request
    UserRequest,
    /// Resource shortage
    ResourceShortage,
}

/// Target type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationTarget {
    /// CPU core
    CpuCore(u32),
    /// NUMA node
    NumaNode(u32),
    /// CPU socket
    Socket(u32),
}

// ============================================================================
// MIGRATION PLAN
// ============================================================================

/// State transfer item
#[derive(Debug, Clone)]
pub struct StateTransferItem {
    /// Item type
    pub item_type: StateItemType,
    /// Size in bytes
    pub size: u64,
    /// Is transferred
    pub transferred: bool,
    /// Transfer duration (ns)
    pub transfer_time_ns: u64,
}

/// State item type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateItemType {
    /// Thread context
    ThreadContext,
    /// Memory pages (working set)
    WorkingSet,
    /// TLB state
    TlbState,
    /// Cache state
    CacheState,
    /// File descriptor table
    FdTable,
    /// IPC bindings
    IpcBindings,
    /// Timer state
    Timers,
}

/// Migration plan
#[derive(Debug)]
pub struct MigrationPlan {
    /// Migration id
    pub id: u64,
    /// Process to migrate
    pub pid: u64,
    /// Source
    pub source: MigrationTarget,
    /// Destination
    pub destination: MigrationTarget,
    /// Reason
    pub reason: CoopMigrationReason,
    /// State
    pub state: CoopMigrationState,
    /// State items to transfer
    items: Vec<StateTransferItem>,
    /// Planning start
    pub plan_start: u64,
    /// Execution start
    pub exec_start: u64,
    /// Completion time
    pub completed_at: Option<u64>,
    /// Downtime (ns, during stop-and-copy)
    pub downtime_ns: u64,
    /// Total transferred bytes
    pub bytes_transferred: u64,
    /// Dirty page iterations (pre-copy rounds)
    pub precopy_rounds: u32,
    /// Max pre-copy rounds
    pub max_precopy_rounds: u32,
}

impl MigrationPlan {
    pub fn new(
        id: u64,
        pid: u64,
        source: MigrationTarget,
        destination: MigrationTarget,
        reason: CoopMigrationReason,
        now: u64,
    ) -> Self {
        Self {
            id,
            pid,
            source,
            destination,
            reason,
            state: CoopMigrationState::Planning,
            items: Vec::new(),
            plan_start: now,
            exec_start: 0,
            completed_at: None,
            downtime_ns: 0,
            bytes_transferred: 0,
            precopy_rounds: 0,
            max_precopy_rounds: 5,
        }
    }

    /// Add state item
    pub fn add_item(&mut self, item: StateTransferItem) {
        self.items.push(item);
    }

    /// Begin negotiation
    pub fn begin_negotiation(&mut self) {
        if self.state == CoopMigrationState::Planning {
            self.state = CoopMigrationState::Negotiating;
        }
    }

    /// Begin pre-copy
    pub fn begin_precopy(&mut self, now: u64) {
        if self.state == CoopMigrationState::Negotiating {
            self.state = CoopMigrationState::PreCopy;
            self.exec_start = now;
        }
    }

    /// Complete a pre-copy round
    pub fn complete_precopy_round(&mut self, dirty_bytes: u64) {
        self.precopy_rounds += 1;
        self.bytes_transferred += dirty_bytes;
    }

    /// Ready for stop-and-copy?
    pub fn ready_for_stop_copy(&self) -> bool {
        self.precopy_rounds >= self.max_precopy_rounds
    }

    /// Begin stop-and-copy
    pub fn begin_stop_copy(&mut self) {
        if self.state == CoopMigrationState::PreCopy {
            self.state = CoopMigrationState::StopAndCopy;
        }
    }

    /// Begin resume
    pub fn begin_resume(&mut self, downtime_ns: u64) {
        if self.state == CoopMigrationState::StopAndCopy {
            self.downtime_ns = downtime_ns;
            self.state = CoopMigrationState::Resuming;
        }
    }

    /// Complete migration
    pub fn complete(&mut self, now: u64) {
        self.state = CoopMigrationState::Complete;
        self.completed_at = Some(now);
        for item in &mut self.items {
            item.transferred = true;
        }
    }

    /// Fail migration
    pub fn fail(&mut self) {
        self.state = CoopMigrationState::Failed;
    }

    /// Rollback
    pub fn rollback(&mut self) {
        self.state = CoopMigrationState::RolledBack;
    }

    /// Total state size
    pub fn total_state_size(&self) -> u64 {
        self.items.iter().map(|i| i.size).sum()
    }

    /// Transfer progress
    pub fn transfer_progress(&self) -> f64 {
        let total = self.total_state_size();
        if total == 0 {
            return 1.0;
        }
        let transferred: u64 = self
            .items
            .iter()
            .filter(|i| i.transferred)
            .map(|i| i.size)
            .sum();
        transferred as f64 / total as f64
    }

    /// Total migration time (ns)
    pub fn total_time_ns(&self, now: u64) -> u64 {
        let end = self.completed_at.unwrap_or(now);
        end.saturating_sub(self.plan_start)
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Migration coordination stats
#[derive(Debug, Clone, Default)]
pub struct CoopMigrationStats {
    /// Active migrations
    pub active_count: usize,
    /// Total completed
    pub total_completed: u64,
    /// Total failed
    pub total_failed: u64,
    /// Total rolled back
    pub total_rollback: u64,
    /// Average downtime (ns)
    pub avg_downtime_ns: f64,
}

/// Cooperative migration coordinator
pub struct CoopMigrationCoordinator {
    /// Active plans
    plans: BTreeMap<u64, MigrationPlan>,
    /// Next plan id
    next_id: u64,
    /// Completed downtimes for average
    completed_downtimes: Vec<u64>,
    /// Max completed history
    max_history: usize,
    /// Stats
    stats: CoopMigrationStats,
}

impl CoopMigrationCoordinator {
    pub fn new() -> Self {
        Self {
            plans: BTreeMap::new(),
            next_id: 1,
            completed_downtimes: Vec::new(),
            max_history: 128,
            stats: CoopMigrationStats::default(),
        }
    }

    /// Create migration plan
    pub fn create_plan(
        &mut self,
        pid: u64,
        source: MigrationTarget,
        destination: MigrationTarget,
        reason: CoopMigrationReason,
        now: u64,
    ) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let plan = MigrationPlan::new(id, pid, source, destination, reason, now);
        self.plans.insert(id, plan);
        self.update_stats();
        id
    }

    /// Get plan
    pub fn plan(&self, id: u64) -> Option<&MigrationPlan> {
        self.plans.get(&id)
    }

    /// Get mutable plan
    pub fn plan_mut(&mut self, id: u64) -> Option<&mut MigrationPlan> {
        self.plans.get_mut(&id)
    }

    /// Complete migration
    pub fn complete(&mut self, id: u64, now: u64) -> bool {
        if let Some(plan) = self.plans.get_mut(&id) {
            plan.complete(now);
            if self.completed_downtimes.len() >= self.max_history {
                self.completed_downtimes.remove(0);
            }
            self.completed_downtimes.push(plan.downtime_ns);
            self.stats.total_completed += 1;
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Fail migration
    pub fn fail(&mut self, id: u64) -> bool {
        if let Some(plan) = self.plans.get_mut(&id) {
            plan.fail();
            self.stats.total_failed += 1;
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Rollback migration
    pub fn rollback(&mut self, id: u64) -> bool {
        if let Some(plan) = self.plans.get_mut(&id) {
            plan.rollback();
            self.stats.total_rollback += 1;
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Cleanup completed/failed
    pub fn cleanup(&mut self) {
        self.plans.retain(|_, p| {
            p.state != CoopMigrationState::Complete
                && p.state != CoopMigrationState::Failed
                && p.state != CoopMigrationState::RolledBack
        });
        self.update_stats();
    }

    fn update_stats(&mut self) {
        self.stats.active_count = self
            .plans
            .values()
            .filter(|p| {
                p.state != CoopMigrationState::Complete
                    && p.state != CoopMigrationState::Failed
                    && p.state != CoopMigrationState::RolledBack
            })
            .count();
        if !self.completed_downtimes.is_empty() {
            let sum: u64 = self.completed_downtimes.iter().sum();
            self.stats.avg_downtime_ns = sum as f64 / self.completed_downtimes.len() as f64;
        }
    }

    /// Stats
    pub fn stats(&self) -> &CoopMigrationStats {
        &self.stats
    }
}
