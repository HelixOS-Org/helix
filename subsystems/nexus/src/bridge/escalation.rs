//! # Bridge Priority Escalation
//!
//! Dynamic priority escalation for syscalls:
//! - Age-based escalation
//! - Deadline-based escalation
//! - Starvation prevention
//! - Priority inheritance
//! - Escalation policy management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// ESCALATION TYPES
// ============================================================================

/// Base priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BasePriority {
    /// Idle
    Idle = 0,
    /// Low
    Low = 1,
    /// Normal
    Normal = 2,
    /// High
    High = 3,
    /// RealTime
    RealTime = 4,
    /// Critical
    Critical = 5,
}

/// Escalation reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationReason {
    /// Age-based (waited too long)
    Aging,
    /// Deadline approaching
    DeadlineApproaching,
    /// Priority inheritance
    Inheritance,
    /// Starvation detected
    Starvation,
    /// Manual override
    Manual,
    /// SLA requirement
    SlaRequirement,
}

/// Escalation state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationState {
    /// Normal (no escalation)
    Normal,
    /// Escalated once
    Escalated,
    /// Fully escalated (max priority)
    FullyEscalated,
    /// De-escalated (returning to normal)
    DeEscalated,
}

// ============================================================================
// TRACKED SYSCALL
// ============================================================================

/// Tracked syscall for escalation
#[derive(Debug, Clone)]
pub struct TrackedSyscall {
    /// Syscall id
    pub id: u64,
    /// Process id
    pub pid: u64,
    /// Syscall number
    pub syscall_nr: u32,
    /// Base priority
    pub base_priority: BasePriority,
    /// Current (effective) priority
    pub effective_priority: u8,
    /// Escalation level
    pub escalation_level: u8,
    /// Max escalation level
    pub max_escalation: u8,
    /// Created time
    pub created_at: u64,
    /// Deadline (if any)
    pub deadline_ns: Option<u64>,
    /// Escalation reasons
    pub reasons: Vec<EscalationReason>,
    /// State
    pub state: EscalationState,
    /// Last escalation time
    pub last_escalated_at: u64,
}

impl TrackedSyscall {
    pub fn new(id: u64, pid: u64, syscall_nr: u32, base: BasePriority, now: u64) -> Self {
        Self {
            id,
            pid,
            syscall_nr,
            base_priority: base,
            effective_priority: base as u8 * 20,
            escalation_level: 0,
            max_escalation: 5,
            created_at: now,
            deadline_ns: None,
            reasons: Vec::new(),
            state: EscalationState::Normal,
            last_escalated_at: now,
        }
    }

    /// Age (ns)
    pub fn age_ns(&self, now: u64) -> u64 {
        now.saturating_sub(self.created_at)
    }

    /// Time to deadline
    pub fn time_to_deadline(&self, now: u64) -> Option<u64> {
        self.deadline_ns.map(|d| d.saturating_sub(now))
    }

    /// Is deadline imminent?
    pub fn deadline_imminent(&self, now: u64, threshold_ns: u64) -> bool {
        self.time_to_deadline(now)
            .map(|t| t < threshold_ns)
            .unwrap_or(false)
    }

    /// Escalate
    pub fn escalate(&mut self, reason: EscalationReason, now: u64) -> bool {
        if self.escalation_level >= self.max_escalation {
            return false;
        }

        self.escalation_level += 1;
        self.effective_priority = self
            .effective_priority
            .saturating_add(10)
            .min(255);

        if !self.reasons.contains(&reason) {
            self.reasons.push(reason);
        }

        self.state = if self.escalation_level >= self.max_escalation {
            EscalationState::FullyEscalated
        } else {
            EscalationState::Escalated
        };

        self.last_escalated_at = now;
        true
    }

    /// De-escalate
    pub fn de_escalate(&mut self) -> bool {
        if self.escalation_level == 0 {
            return false;
        }
        self.escalation_level -= 1;
        self.effective_priority = self.effective_priority.saturating_sub(10);
        if self.escalation_level == 0 {
            self.state = EscalationState::Normal;
            self.effective_priority = self.base_priority as u8 * 20;
        } else {
            self.state = EscalationState::DeEscalated;
        }
        true
    }
}

// ============================================================================
// ESCALATION POLICY
// ============================================================================

/// Escalation policy
#[derive(Debug, Clone)]
pub struct EscalationPolicy {
    /// Age threshold for escalation (ns)
    pub age_threshold_ns: u64,
    /// Deadline proximity threshold (ns)
    pub deadline_threshold_ns: u64,
    /// Starvation detection window (ns)
    pub starvation_window_ns: u64,
    /// Max escalation per tick
    pub max_escalations_per_tick: usize,
    /// Cooldown between escalations (ns)
    pub cooldown_ns: u64,
    /// Enable inheritance
    pub enable_inheritance: bool,
}

impl EscalationPolicy {
    pub fn default_policy() -> Self {
        Self {
            age_threshold_ns: 100_000_000, // 100ms
            deadline_threshold_ns: 50_000_000, // 50ms
            starvation_window_ns: 500_000_000, // 500ms
            max_escalations_per_tick: 8,
            cooldown_ns: 10_000_000, // 10ms
            enable_inheritance: true,
        }
    }
}

// ============================================================================
// ESCALATION MANAGER
// ============================================================================

/// Escalation stats
#[derive(Debug, Clone, Default)]
pub struct EscalationStats {
    /// Tracked syscalls
    pub tracked: usize,
    /// Total escalations
    pub total_escalations: u64,
    /// Age-based escalations
    pub age_escalations: u64,
    /// Deadline escalations
    pub deadline_escalations: u64,
    /// Inheritance escalations
    pub inheritance_escalations: u64,
    /// Starvation escalations
    pub starvation_escalations: u64,
    /// Fully escalated
    pub fully_escalated: usize,
}

/// Bridge priority escalation manager
pub struct BridgeEscalationManager {
    /// Tracked syscalls
    tracked: BTreeMap<u64, TrackedSyscall>,
    /// Process to syscall mapping
    process_map: BTreeMap<u64, Vec<u64>>,
    /// Policy
    policy: EscalationPolicy,
    /// Stats
    stats: EscalationStats,
}

impl BridgeEscalationManager {
    pub fn new() -> Self {
        Self {
            tracked: BTreeMap::new(),
            process_map: BTreeMap::new(),
            policy: EscalationPolicy::default_policy(),
            stats: EscalationStats::default(),
        }
    }

    /// Set policy
    pub fn set_policy(&mut self, policy: EscalationPolicy) {
        self.policy = policy;
    }

    /// Track syscall
    pub fn track(
        &mut self,
        id: u64,
        pid: u64,
        syscall_nr: u32,
        base: BasePriority,
        deadline_ns: Option<u64>,
        now: u64,
    ) {
        let mut sc = TrackedSyscall::new(id, pid, syscall_nr, base, now);
        sc.deadline_ns = deadline_ns;
        self.tracked.insert(id, sc);
        self.process_map
            .entry(pid)
            .or_insert_with(Vec::new)
            .push(id);
        self.stats.tracked = self.tracked.len();
    }

    /// Remove tracking (completed)
    pub fn complete(&mut self, id: u64) {
        if let Some(sc) = self.tracked.remove(&id) {
            if let Some(pids) = self.process_map.get_mut(&sc.pid) {
                pids.retain(|&x| x != id);
            }
        }
        self.stats.tracked = self.tracked.len();
    }

    /// Run escalation tick
    pub fn tick(&mut self, now: u64) -> Vec<(u64, EscalationReason)> {
        let mut escalated = Vec::new();
        let mut count = 0;

        // Collect candidates
        let candidates: Vec<(u64, u64, bool, bool)> = self
            .tracked
            .iter()
            .map(|(&id, sc)| {
                let aged = sc.age_ns(now) > self.policy.age_threshold_ns;
                let deadline = sc.deadline_imminent(now, self.policy.deadline_threshold_ns);
                let since_last = now.saturating_sub(sc.last_escalated_at);
                let cooled = since_last >= self.policy.cooldown_ns;
                (id, since_last, aged && cooled, deadline && cooled)
            })
            .collect();

        for (id, _since_last, aged, deadline) in candidates {
            if count >= self.policy.max_escalations_per_tick {
                break;
            }

            if deadline {
                if let Some(sc) = self.tracked.get_mut(&id) {
                    if sc.escalate(EscalationReason::DeadlineApproaching, now) {
                        escalated.push((id, EscalationReason::DeadlineApproaching));
                        self.stats.deadline_escalations += 1;
                        self.stats.total_escalations += 1;
                        count += 1;
                    }
                }
            } else if aged {
                if let Some(sc) = self.tracked.get_mut(&id) {
                    if sc.escalate(EscalationReason::Aging, now) {
                        escalated.push((id, EscalationReason::Aging));
                        self.stats.age_escalations += 1;
                        self.stats.total_escalations += 1;
                        count += 1;
                    }
                }
            }
        }

        self.stats.fully_escalated = self
            .tracked
            .values()
            .filter(|s| matches!(s.state, EscalationState::FullyEscalated))
            .count();

        escalated
    }

    /// Priority inheritance
    pub fn inherit(&mut self, from_pid: u64, to_id: u64, now: u64) -> bool {
        if !self.policy.enable_inheritance {
            return false;
        }

        let max_prio = self
            .process_map
            .get(&from_pid)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.tracked.get(id))
                    .map(|sc| sc.effective_priority)
                    .max()
                    .unwrap_or(0)
            })
            .unwrap_or(0);

        if let Some(sc) = self.tracked.get_mut(&to_id) {
            if max_prio > sc.effective_priority {
                sc.effective_priority = max_prio;
                sc.escalate(EscalationReason::Inheritance, now);
                self.stats.inheritance_escalations += 1;
                self.stats.total_escalations += 1;
                return true;
            }
        }
        false
    }

    /// Get effective priority
    pub fn effective_priority(&self, id: u64) -> Option<u8> {
        self.tracked.get(&id).map(|sc| sc.effective_priority)
    }

    /// Stats
    pub fn stats(&self) -> &EscalationStats {
        &self.stats
    }
}
