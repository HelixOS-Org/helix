//! Power-Aware Work Scheduler
//!
//! This module provides energy-efficient work scheduling with CPU power state awareness.

use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};
use super::{CpuId, WorkInfo, WorkPriority};

/// CPU power state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CpuPowerState {
    /// Deep sleep (highest latency)
    DeepSleep  = 0,
    /// Light sleep
    LightSleep = 1,
    /// Idle
    Idle       = 2,
    /// Active
    Active     = 3,
    /// Turbo boost
    Turbo      = 4,
}

impl CpuPowerState {
    /// Get wakeup latency estimate (nanoseconds)
    #[inline]
    pub fn wakeup_latency_ns(&self) -> u64 {
        match self {
            Self::DeepSleep => 1_000_000, // 1ms
            Self::LightSleep => 100_000,  // 100µs
            Self::Idle => 10_000,         // 10µs
            Self::Active => 1_000,        // 1µs
            Self::Turbo => 100,           // 100ns
        }
    }

    /// Get power consumption estimate (relative units)
    #[inline]
    pub fn power_units(&self) -> u32 {
        match self {
            Self::DeepSleep => 1,
            Self::LightSleep => 5,
            Self::Idle => 20,
            Self::Active => 80,
            Self::Turbo => 120,
        }
    }
}

/// Power-aware scheduling decision
#[derive(Debug, Clone)]
pub struct PowerSchedulingDecision {
    /// Target CPU
    pub cpu_id: CpuId,
    /// Should wake CPU
    pub should_wake: bool,
    /// Delay before executing (nanoseconds)
    pub delay_ns: u64,
    /// Batch with other work items
    pub batch_id: Option<u64>,
    /// Estimated energy cost
    pub energy_cost: u32,
    /// Reason for decision
    pub reason: PowerDecisionReason,
}

/// Reason for power-aware decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerDecisionReason {
    /// CPU already active
    CpuActive,
    /// Work is urgent
    UrgentWork,
    /// Batching opportunity
    Batching,
    /// Energy saving
    EnergySaving,
    /// Load balancing
    LoadBalancing,
    /// NUMA locality
    NumaLocality,
}

/// Power-aware work scheduler
#[repr(align(64))]
pub struct PowerAwareWorkScheduler {
    /// Per-CPU power states
    cpu_states: BTreeMap<CpuId, CpuPowerState>,
    /// Per-CPU pending work count
    pending_work: BTreeMap<CpuId, u64>,
    /// Batch timeout (nanoseconds)
    batch_timeout_ns: u64,
    /// Current batch ID
    current_batch_id: AtomicU64,
    /// Energy budget (units per second)
    energy_budget: u32,
    /// Current energy consumption
    current_energy: AtomicU64,
    /// Last energy reset timestamp
    last_energy_reset: u64,
    /// Power saving mode enabled
    power_saving_mode: bool,
}

impl PowerAwareWorkScheduler {
    /// Create new power-aware scheduler
    pub fn new() -> Self {
        Self {
            cpu_states: BTreeMap::new(),
            pending_work: BTreeMap::new(),
            batch_timeout_ns: 1_000_000, // 1ms
            current_batch_id: AtomicU64::new(1),
            energy_budget: 10000,
            current_energy: AtomicU64::new(0),
            last_energy_reset: 0,
            power_saving_mode: false,
        }
    }

    /// Register CPU
    #[inline(always)]
    pub fn register_cpu(&mut self, cpu_id: CpuId, initial_state: CpuPowerState) {
        self.cpu_states.insert(cpu_id, initial_state);
        self.pending_work.insert(cpu_id, 0);
    }

    /// Update CPU power state
    #[inline(always)]
    pub fn update_cpu_state(&mut self, cpu_id: CpuId, state: CpuPowerState) {
        self.cpu_states.insert(cpu_id, state);
    }

    /// Set power saving mode
    #[inline(always)]
    pub fn set_power_saving(&mut self, enabled: bool) {
        self.power_saving_mode = enabled;
    }

    /// Schedule work item with power awareness
    pub fn schedule_work(
        &mut self,
        work: &WorkInfo,
        current_time: u64,
        _preferred_cpu: Option<CpuId>,
    ) -> PowerSchedulingDecision {
        // Find best CPU
        let mut best_cpu: Option<CpuId> = None;
        let mut best_score = i64::MIN;

        for (&cpu_id, &state) in &self.cpu_states {
            // Check CPU affinity
            if (work.cpu_affinity & (1 << cpu_id.0)) == 0 {
                continue;
            }

            let pending = self.pending_work.get(&cpu_id).copied().unwrap_or(0);
            let wakeup_cost = state.wakeup_latency_ns();

            // Calculate score (higher is better)
            let mut score = 1000i64;

            // Prefer active CPUs to avoid wakeup cost
            if state >= CpuPowerState::Active {
                score += 500;
            }

            // Prefer CPUs with less pending work
            score -= (pending * 10) as i64;

            // Factor in wakeup latency
            score -= (wakeup_cost / 10000) as i64;

            if score > best_score {
                best_score = score;
                best_cpu = Some(cpu_id);
            }
        }

        let target_cpu = best_cpu.unwrap_or(CpuId::new(0));
        let state = self
            .cpu_states
            .get(&target_cpu)
            .copied()
            .unwrap_or(CpuPowerState::Idle);

        // Determine if we should batch
        let should_batch = self.power_saving_mode
            && work.priority <= WorkPriority::Normal
            && state < CpuPowerState::Active;

        let (delay_ns, batch_id, reason) = if should_batch {
            let batch = self.current_batch_id.fetch_add(1, Ordering::Relaxed);
            (
                self.batch_timeout_ns,
                Some(batch),
                PowerDecisionReason::Batching,
            )
        } else if work.priority >= WorkPriority::High {
            (0, None, PowerDecisionReason::UrgentWork)
        } else if state >= CpuPowerState::Active {
            (0, None, PowerDecisionReason::CpuActive)
        } else {
            (0, None, PowerDecisionReason::LoadBalancing)
        };

        // Calculate energy cost
        let wakeup_cost = if state < CpuPowerState::Active {
            state.power_units() * 10
        } else {
            0
        };
        let execution_cost =
            (work.expected_duration_ns / 1000000) as u32 * CpuPowerState::Active.power_units();
        let energy_cost = wakeup_cost + execution_cost;

        // Update pending work count
        if let Some(pending) = self.pending_work.get_mut(&target_cpu) {
            *pending += 1;
        }

        // Update energy consumption
        self.current_energy
            .fetch_add(energy_cost as u64, Ordering::Relaxed);

        PowerSchedulingDecision {
            cpu_id: target_cpu,
            should_wake: state < CpuPowerState::Active && delay_ns == 0,
            delay_ns,
            batch_id,
            energy_cost,
            reason,
        }
    }

    /// Record work completion
    #[inline]
    pub fn record_completion(&mut self, cpu_id: CpuId) {
        if let Some(pending) = self.pending_work.get_mut(&cpu_id) {
            *pending = pending.saturating_sub(1);
        }
    }

    /// Reset energy counter
    #[inline(always)]
    pub fn reset_energy(&mut self, current_time: u64) {
        self.current_energy.store(0, Ordering::Relaxed);
        self.last_energy_reset = current_time;
    }

    /// Get current energy consumption rate
    #[inline]
    pub fn energy_consumption_rate(&self, current_time: u64) -> f32 {
        let elapsed = current_time - self.last_energy_reset;
        if elapsed == 0 {
            return 0.0;
        }
        let consumed = self.current_energy.load(Ordering::Relaxed);
        consumed as f32 / (elapsed as f32 / 1_000_000_000.0) // units/sec
    }

    /// Check if under energy budget
    #[inline(always)]
    pub fn under_energy_budget(&self, current_time: u64) -> bool {
        self.energy_consumption_rate(current_time) <= self.energy_budget as f32
    }

    /// Get CPU count
    #[inline(always)]
    pub fn cpu_count(&self) -> usize {
        self.cpu_states.len()
    }

    /// Set energy budget
    #[inline(always)]
    pub fn set_energy_budget(&mut self, budget: u32) {
        self.energy_budget = budget;
    }
}

impl Default for PowerAwareWorkScheduler {
    fn default() -> Self {
        Self::new()
    }
}
