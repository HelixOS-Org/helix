//! Power-Aware Timer Scheduler
//!
//! Energy-efficient timer scheduling with CPU awareness.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::core::NexusTimestamp;

use super::{CpuId, TimerId, TimerInfo, TimerType};

/// Timer migration
#[derive(Debug, Clone)]
pub struct TimerMigration {
    /// Timer ID
    pub timer_id: TimerId,
    /// From CPU
    pub from_cpu: CpuId,
    /// To CPU
    pub to_cpu: CpuId,
    /// Reason
    pub reason: MigrationReason,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Migration reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationReason {
    /// CPU going idle
    CpuIdle,
    /// Load balancing
    LoadBalance,
    /// Power saving
    PowerSaving,
    /// Affinity change
    AffinityChange,
}

/// Scheduling decision
#[derive(Debug, Clone)]
pub struct SchedulingDecision {
    /// Timer ID
    pub timer_id: TimerId,
    /// Assigned CPU
    pub cpu: CpuId,
    /// Decision type
    pub decision: DecisionType,
    /// Timestamp
    pub timestamp: NexusTimestamp,
}

/// Decision type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionType {
    /// Place on idle CPU
    PlaceOnIdle,
    /// Place on busy CPU (coalesce)
    Coalesce,
    /// Defer
    Defer,
    /// Honor affinity
    HonorAffinity,
}

/// Power-aware timer scheduler
pub struct PowerAwareScheduler {
    /// Timers per CPU
    per_cpu: BTreeMap<CpuId, Vec<TimerId>>,
    /// CPU idle states
    cpu_idle: BTreeMap<CpuId, bool>,
    /// Migration opportunities
    migrations: Vec<TimerMigration>,
    /// Scheduling decisions
    decisions: Vec<SchedulingDecision>,
}

impl PowerAwareScheduler {
    /// Create new scheduler
    pub fn new() -> Self {
        Self {
            per_cpu: BTreeMap::new(),
            cpu_idle: BTreeMap::new(),
            migrations: Vec::new(),
            decisions: Vec::new(),
        }
    }

    /// Register CPU
    pub fn register_cpu(&mut self, cpu: CpuId) {
        self.per_cpu.insert(cpu, Vec::new());
        self.cpu_idle.insert(cpu, true);
    }

    /// Set CPU idle state
    pub fn set_cpu_idle(&mut self, cpu: CpuId, idle: bool) {
        self.cpu_idle.insert(cpu, idle);
    }

    /// Schedule timer
    pub fn schedule(&mut self, timer: &TimerInfo) -> CpuId {
        // Honor pinned affinity
        if let Some(cpu) = timer.cpu {
            if timer.timer_type == TimerType::Pinned {
                self.record_decision(timer.id, cpu, DecisionType::HonorAffinity);
                return cpu;
            }
        }

        // For deferrable timers, try to coalesce with busy CPUs
        if timer.timer_type.is_deferrable() {
            if let Some(busy_cpu) = self.find_busy_cpu() {
                self.record_decision(timer.id, busy_cpu, DecisionType::Coalesce);
                return busy_cpu;
            }
        }

        // Find CPU with least timers
        let best_cpu = self
            .per_cpu
            .iter()
            .min_by_key(|(_, timers)| timers.len())
            .map(|(&cpu, _)| cpu)
            .unwrap_or(0);

        let decision = if self.cpu_idle.get(&best_cpu).copied().unwrap_or(true) {
            DecisionType::PlaceOnIdle
        } else {
            DecisionType::Coalesce
        };

        self.record_decision(timer.id, best_cpu, decision);
        best_cpu
    }

    /// Find busy CPU
    fn find_busy_cpu(&self) -> Option<CpuId> {
        self.cpu_idle
            .iter()
            .find(|(_, &idle)| !idle)
            .map(|(&cpu, _)| cpu)
    }

    /// Record decision
    fn record_decision(&mut self, timer_id: TimerId, cpu: CpuId, decision: DecisionType) {
        self.decisions.push(SchedulingDecision {
            timer_id,
            cpu,
            decision,
            timestamp: NexusTimestamp::now(),
        });

        if let Some(timers) = self.per_cpu.get_mut(&cpu) {
            if !timers.contains(&timer_id) {
                timers.push(timer_id);
            }
        }
    }

    /// Migrate timer
    pub fn migrate(&mut self, timer_id: TimerId, from: CpuId, to: CpuId, reason: MigrationReason) {
        // Remove from source
        if let Some(timers) = self.per_cpu.get_mut(&from) {
            timers.retain(|&id| id != timer_id);
        }

        // Add to destination
        if let Some(timers) = self.per_cpu.get_mut(&to) {
            if !timers.contains(&timer_id) {
                timers.push(timer_id);
            }
        }

        self.migrations.push(TimerMigration {
            timer_id,
            from_cpu: from,
            to_cpu: to,
            reason,
            timestamp: NexusTimestamp::now(),
        });
    }

    /// Get timers on CPU
    pub fn timers_on_cpu(&self, cpu: CpuId) -> &[TimerId] {
        self.per_cpu.get(&cpu).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get migrations
    pub fn migrations(&self) -> &[TimerMigration] {
        &self.migrations
    }

    /// Get decisions
    pub fn decisions(&self) -> &[SchedulingDecision] {
        &self.decisions
    }
}

impl Default for PowerAwareScheduler {
    fn default() -> Self {
        Self::new()
    }
}
