// SPDX-License-Identifier: GPL-2.0
//! Coop batch_sync — batch synchronization for group operations.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Batch operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchOpType {
    AllComplete,
    AnyComplete,
    Quorum,
    Ordered,
}

/// Batch participant state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantState {
    Waiting,
    Ready,
    Committed,
    Aborted,
    TimedOut,
}

/// Batch participant
#[derive(Debug, Clone)]
pub struct BatchParticipant {
    pub id: u32,
    pub state: ParticipantState,
    pub joined_at: u64,
    pub ready_at: u64,
    pub result: Option<i64>,
}

impl BatchParticipant {
    pub fn new(id: u32, now: u64) -> Self {
        Self { id, state: ParticipantState::Waiting, joined_at: now, ready_at: 0, result: None }
    }

    pub fn mark_ready(&mut self, result: i64, now: u64) {
        self.state = ParticipantState::Ready;
        self.ready_at = now;
        self.result = Some(result);
    }

    pub fn wait_time(&self) -> u64 {
        if self.ready_at > 0 { self.ready_at - self.joined_at } else { 0 }
    }
}

/// Batch synchronization group
#[derive(Debug)]
pub struct BatchGroup {
    pub id: u64,
    pub op_type: BatchOpType,
    pub participants: Vec<BatchParticipant>,
    pub required_count: u32,
    pub quorum_size: u32,
    pub timeout_ns: u64,
    pub created_at: u64,
    pub completed_at: u64,
    pub is_resolved: bool,
    pub success: bool,
}

impl BatchGroup {
    pub fn new(id: u64, op_type: BatchOpType, required: u32, timeout: u64, now: u64) -> Self {
        let quorum = match op_type {
            BatchOpType::Quorum => (required / 2) + 1,
            _ => required,
        };
        Self {
            id, op_type, participants: Vec::new(), required_count: required,
            quorum_size: quorum, timeout_ns: timeout, created_at: now,
            completed_at: 0, is_resolved: false, success: false,
        }
    }

    pub fn add_participant(&mut self, pid: u32, now: u64) -> bool {
        if self.is_resolved { return false; }
        if self.participants.iter().any(|p| p.id == pid) { return false; }
        self.participants.push(BatchParticipant::new(pid, now));
        true
    }

    pub fn mark_ready(&mut self, pid: u32, result: i64, now: u64) -> bool {
        for p in &mut self.participants {
            if p.id == pid && p.state == ParticipantState::Waiting {
                p.mark_ready(result, now);
                return true;
            }
        }
        false
    }

    pub fn ready_count(&self) -> u32 {
        self.participants.iter().filter(|p| p.state == ParticipantState::Ready).count() as u32
    }

    pub fn is_complete(&self) -> bool {
        match self.op_type {
            BatchOpType::AllComplete => self.ready_count() >= self.required_count,
            BatchOpType::AnyComplete => self.ready_count() >= 1,
            BatchOpType::Quorum => self.ready_count() >= self.quorum_size,
            BatchOpType::Ordered => {
                self.ready_count() >= self.required_count
            }
        }
    }

    pub fn check_timeout(&mut self, now: u64) -> bool {
        if self.is_resolved { return false; }
        if self.timeout_ns > 0 && now.saturating_sub(self.created_at) >= self.timeout_ns {
            for p in &mut self.participants {
                if p.state == ParticipantState::Waiting {
                    p.state = ParticipantState::TimedOut;
                }
            }
            self.is_resolved = true;
            self.success = false;
            self.completed_at = now;
            return true;
        }
        false
    }

    pub fn try_resolve(&mut self, now: u64) -> bool {
        if self.is_resolved { return false; }
        if self.is_complete() {
            for p in &mut self.participants {
                if p.state == ParticipantState::Ready {
                    p.state = ParticipantState::Committed;
                }
            }
            self.is_resolved = true;
            self.success = true;
            self.completed_at = now;
            return true;
        }
        false
    }

    pub fn duration(&self) -> u64 {
        if self.completed_at > 0 { self.completed_at - self.created_at } else { 0 }
    }

    pub fn completion_ratio(&self) -> f64 {
        if self.required_count == 0 { return 0.0; }
        self.ready_count() as f64 / self.required_count as f64
    }
}

/// Batch sync stats
#[derive(Debug, Clone)]
pub struct BatchSyncStats {
    pub active_groups: u32,
    pub total_created: u64,
    pub total_completed: u64,
    pub total_timed_out: u64,
    pub total_participants: u64,
    pub avg_completion_ns: u64,
}

/// Main batch sync manager
pub struct CoopBatchSync {
    groups: BTreeMap<u64, BatchGroup>,
    completed: Vec<BatchGroup>,
    max_completed: usize,
    next_id: u64,
    total_created: u64,
    total_completed: u64,
    total_timed_out: u64,
    total_completion_ns: u64,
}

impl CoopBatchSync {
    pub fn new(max_history: usize) -> Self {
        Self {
            groups: BTreeMap::new(), completed: Vec::new(),
            max_completed: max_history, next_id: 1,
            total_created: 0, total_completed: 0,
            total_timed_out: 0, total_completion_ns: 0,
        }
    }

    pub fn create_group(&mut self, op_type: BatchOpType, required: u32, timeout: u64, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.total_created += 1;
        self.groups.insert(id, BatchGroup::new(id, op_type, required, timeout, now));
        id
    }

    pub fn join(&mut self, group_id: u64, pid: u32, now: u64) -> bool {
        self.groups.get_mut(&group_id).map(|g| g.add_participant(pid, now)).unwrap_or(false)
    }

    pub fn signal_ready(&mut self, group_id: u64, pid: u32, result: i64, now: u64) -> bool {
        if let Some(group) = self.groups.get_mut(&group_id) {
            if group.mark_ready(pid, result, now) {
                if group.try_resolve(now) {
                    self.total_completed += 1;
                    self.total_completion_ns += group.duration();
                }
                return true;
            }
        }
        false
    }

    pub fn tick(&mut self, now: u64) -> Vec<u64> {
        let mut timed_out = Vec::new();
        for (&id, group) in self.groups.iter_mut() {
            if group.check_timeout(now) {
                timed_out.push(id);
                self.total_timed_out += 1;
            }
        }
        timed_out
    }

    pub fn collect_resolved(&mut self) -> Vec<u64> {
        let resolved: Vec<u64> = self.groups.iter()
            .filter(|(_, g)| g.is_resolved)
            .map(|(&id, _)| id)
            .collect();
        for id in &resolved {
            if let Some(g) = self.groups.remove(id) {
                if self.completed.len() >= self.max_completed { self.completed.remove(0); }
                self.completed.push(g);
            }
        }
        resolved
    }

    pub fn stats(&self) -> BatchSyncStats {
        let total_parts: u64 = self.groups.values()
            .map(|g| g.participants.len() as u64).sum();
        BatchSyncStats {
            active_groups: self.groups.len() as u32,
            total_created: self.total_created,
            total_completed: self.total_completed,
            total_timed_out: self.total_timed_out,
            total_participants: total_parts,
            avg_completion_ns: if self.total_completed > 0 {
                self.total_completion_ns / self.total_completed
            } else { 0 },
        }
    }
}
