//! # Cooperative Group Scheduler
//!
//! Cooperative scheduling for groups of related processes:
//! - Gang scheduling for parallel workloads
//! - Co-scheduling of communicating processes
//! - Group timeslice allocation
//! - Group priority inheritance
//! - Cooperative yield protocol
//! - Group CPU budget management

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Group scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupSchedPolicyV2 {
    /// Gang: all threads run simultaneously
    Gang,
    /// Coscheduled: communicating processes on nearby CPUs
    Coschedule,
    /// Proportional: weighted fair share within group
    Proportional,
    /// Priority: strict priority ordering
    StrictPriority,
    /// Cooperative: voluntary yield protocol
    Cooperative,
}

/// Member state in the group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMemberState {
    Ready,
    Running,
    Blocked,
    Yielded,
    Exited,
}

/// Group member
#[derive(Debug, Clone)]
pub struct GroupMember {
    pub pid: u64,
    pub thread_id: u64,
    pub state: GroupMemberState,
    pub weight: u32,
    pub cpu_affinity: u64,
    pub runtime_ns: u64,
    pub wait_ns: u64,
    pub yield_count: u64,
    pub last_scheduled: u64,
}

impl GroupMember {
    pub fn new(pid: u64, thread_id: u64) -> Self {
        Self {
            pid,
            thread_id,
            state: GroupMemberState::Ready,
            weight: 1024,
            cpu_affinity: u64::MAX,
            runtime_ns: 0,
            wait_ns: 0,
            yield_count: 0,
            last_scheduled: 0,
        }
    }

    pub fn is_schedulable(&self) -> bool {
        matches!(self.state, GroupMemberState::Ready | GroupMemberState::Yielded)
    }
}

/// Scheduling group
#[derive(Debug, Clone)]
pub struct SchedGroupV2 {
    pub group_id: u64,
    pub policy: GroupSchedPolicyV2,
    pub members: Vec<GroupMember>,
    pub cpu_budget_ns: u64,
    pub budget_consumed_ns: u64,
    pub budget_period_ns: u64,
    pub group_priority: i32,
    pub total_runtime_ns: u64,
    pub gang_aligned: bool,
    pub max_members: u32,
}

impl SchedGroupV2 {
    pub fn new(group_id: u64, policy: GroupSchedPolicyV2) -> Self {
        Self {
            group_id,
            policy,
            members: Vec::new(),
            cpu_budget_ns: 0,
            budget_consumed_ns: 0,
            budget_period_ns: 100_000_000,
            group_priority: 0,
            total_runtime_ns: 0,
            gang_aligned: false,
            max_members: 256,
        }
    }

    pub fn add_member(&mut self, member: GroupMember) -> bool {
        if self.members.len() as u32 >= self.max_members { return false; }
        self.members.push(member);
        true
    }

    pub fn remove_member(&mut self, thread_id: u64) {
        self.members.retain(|m| m.thread_id != thread_id);
    }

    pub fn ready_count(&self) -> usize {
        self.members.iter().filter(|m| m.is_schedulable()).count()
    }

    pub fn all_ready(&self) -> bool {
        self.members.iter().all(|m| m.is_schedulable() || m.state == GroupMemberState::Running)
    }

    /// For gang scheduling: check if all members can be co-scheduled
    pub fn can_gang_schedule(&self, available_cpus: u32) -> bool {
        if self.policy != GroupSchedPolicyV2::Gang { return true; }
        let ready = self.ready_count();
        ready as u32 <= available_cpus
    }

    pub fn budget_remaining(&self) -> u64 {
        if self.cpu_budget_ns == 0 { return u64::MAX; }
        self.cpu_budget_ns.saturating_sub(self.budget_consumed_ns)
    }

    pub fn consume_budget(&mut self, ns: u64) {
        self.budget_consumed_ns += ns;
        self.total_runtime_ns += ns;
    }

    pub fn refill_budget(&mut self) {
        self.budget_consumed_ns = 0;
    }

    pub fn is_throttled(&self) -> bool {
        self.cpu_budget_ns > 0 && self.budget_consumed_ns >= self.cpu_budget_ns
    }

    /// Select next member to schedule (for non-gang policies)
    pub fn select_next(&self) -> Option<u64> {
        match self.policy {
            GroupSchedPolicyV2::StrictPriority => {
                // Highest weight first
                self.members.iter()
                    .filter(|m| m.is_schedulable())
                    .max_by_key(|m| m.weight)
                    .map(|m| m.thread_id)
            }
            GroupSchedPolicyV2::Proportional => {
                // Least runtime relative to weight
                self.members.iter()
                    .filter(|m| m.is_schedulable())
                    .min_by_key(|m| {
                        if m.weight == 0 { u64::MAX }
                        else { m.runtime_ns / m.weight as u64 }
                    })
                    .map(|m| m.thread_id)
            }
            _ => {
                // FIFO for others
                self.members.iter()
                    .filter(|m| m.is_schedulable())
                    .next()
                    .map(|m| m.thread_id)
            }
        }
    }
}

/// Coop group sched stats
#[derive(Debug, Clone, Default)]
pub struct CoopGroupSchedStats {
    pub total_groups: usize,
    pub total_members: usize,
    pub total_runtime_ns: u64,
    pub throttled_groups: usize,
    pub gang_groups: usize,
}

/// Cooperative Group Scheduler
pub struct CoopGroupSched {
    groups: BTreeMap<u64, SchedGroupV2>,
    next_group_id: u64,
    stats: CoopGroupSchedStats,
}

impl CoopGroupSched {
    pub fn new() -> Self {
        Self {
            groups: BTreeMap::new(),
            next_group_id: 1,
            stats: CoopGroupSchedStats::default(),
        }
    }

    pub fn create_group(&mut self, policy: GroupSchedPolicyV2) -> u64 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.insert(id, SchedGroupV2::new(id, policy));
        self.recompute();
        id
    }

    pub fn destroy_group(&mut self, group_id: u64) {
        self.groups.remove(&group_id);
        self.recompute();
    }

    pub fn add_member(&mut self, group_id: u64, member: GroupMember) -> bool {
        let ok = if let Some(group) = self.groups.get_mut(&group_id) {
            group.add_member(member)
        } else { false };
        self.recompute();
        ok
    }

    pub fn remove_member(&mut self, group_id: u64, thread_id: u64) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.remove_member(thread_id);
        }
        self.recompute();
    }

    pub fn set_budget(&mut self, group_id: u64, budget_ns: u64, period_ns: u64) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.cpu_budget_ns = budget_ns;
            group.budget_period_ns = period_ns;
        }
    }

    pub fn consume_runtime(&mut self, group_id: u64, ns: u64) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            group.consume_budget(ns);
        }
    }

    pub fn period_tick(&mut self) {
        for group in self.groups.values_mut() {
            group.refill_budget();
        }
        self.recompute();
    }

    pub fn throttled_groups(&self) -> Vec<u64> {
        self.groups.values()
            .filter(|g| g.is_throttled())
            .map(|g| g.group_id)
            .collect()
    }

    fn recompute(&mut self) {
        self.stats.total_groups = self.groups.len();
        self.stats.total_members = self.groups.values().map(|g| g.members.len()).sum();
        self.stats.total_runtime_ns = self.groups.values().map(|g| g.total_runtime_ns).sum();
        self.stats.throttled_groups = self.groups.values().filter(|g| g.is_throttled()).count();
        self.stats.gang_groups = self.groups.values()
            .filter(|g| g.policy == GroupSchedPolicyV2::Gang).count();
    }

    pub fn group(&self, id: u64) -> Option<&SchedGroupV2> {
        self.groups.get(&id)
    }

    pub fn stats(&self) -> &CoopGroupSchedStats {
        &self.stats
    }
}
