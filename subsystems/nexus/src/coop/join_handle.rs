// SPDX-License-Identifier: GPL-2.0
//! Coop join_handle — task join handle management.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Join handle state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinState {
    Running,
    Completed,
    Panicked,
    Cancelled,
    Detached,
}

/// Join result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinResult {
    Ok,
    Panic,
    Cancelled,
    Timeout,
    AlreadyJoined,
}

/// Join handle
#[derive(Debug)]
pub struct JoinHandle {
    pub id: u64,
    pub task_id: u64,
    pub state: JoinState,
    pub result_code: i64,
    pub created_at: u64,
    pub completed_at: u64,
    pub joiners: Vec<u64>,
    pub detached: bool,
}

impl JoinHandle {
    pub fn new(id: u64, task_id: u64, now: u64) -> Self {
        Self {
            id, task_id, state: JoinState::Running, result_code: 0,
            created_at: now, completed_at: 0, joiners: Vec::new(), detached: false,
        }
    }

    #[inline]
    pub fn complete(&mut self, code: i64, now: u64) {
        self.state = JoinState::Completed;
        self.result_code = code;
        self.completed_at = now;
    }

    #[inline(always)]
    pub fn panic(&mut self, now: u64) {
        self.state = JoinState::Panicked;
        self.completed_at = now;
    }

    #[inline(always)]
    pub fn cancel(&mut self, now: u64) {
        self.state = JoinState::Cancelled;
        self.completed_at = now;
    }

    #[inline(always)]
    pub fn detach(&mut self) {
        self.detached = true;
        self.state = JoinState::Detached;
    }

    #[inline(always)]
    pub fn add_joiner(&mut self, tid: u64) { self.joiners.push(tid); }

    #[inline(always)]
    pub fn is_done(&self) -> bool {
        matches!(self.state, JoinState::Completed | JoinState::Panicked | JoinState::Cancelled)
    }

    #[inline(always)]
    pub fn lifetime_ns(&self) -> u64 {
        if self.completed_at > 0 { self.completed_at - self.created_at }
        else { 0 }
    }
}

/// Join group (multiple handles joined together)
#[derive(Debug)]
pub struct JoinGroup {
    pub id: u64,
    pub handles: Vec<u64>,
    pub join_all: bool,
    pub completed_count: u32,
}

impl JoinGroup {
    pub fn new(id: u64, join_all: bool) -> Self {
        Self { id, handles: Vec::new(), join_all, completed_count: 0 }
    }

    #[inline(always)]
    pub fn add(&mut self, handle_id: u64) { self.handles.push(handle_id); }

    #[inline(always)]
    pub fn is_satisfied(&self, done_count: u32) -> bool {
        if self.join_all { done_count == self.handles.len() as u32 }
        else { done_count > 0 }
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct JoinHandleStats {
    pub total_handles: u32,
    pub running: u32,
    pub completed: u32,
    pub panicked: u32,
    pub cancelled: u32,
    pub detached: u32,
    pub total_groups: u32,
    pub avg_lifetime_ns: u64,
}

/// Main join handle manager
pub struct CoopJoinHandle {
    handles: BTreeMap<u64, JoinHandle>,
    groups: BTreeMap<u64, JoinGroup>,
    next_id: u64,
    next_group_id: u64,
}

impl CoopJoinHandle {
    pub fn new() -> Self {
        Self { handles: BTreeMap::new(), groups: BTreeMap::new(), next_id: 1, next_group_id: 1 }
    }

    #[inline]
    pub fn spawn(&mut self, task_id: u64, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.handles.insert(id, JoinHandle::new(id, task_id, now));
        id
    }

    #[inline(always)]
    pub fn complete(&mut self, handle_id: u64, code: i64, now: u64) {
        if let Some(h) = self.handles.get_mut(&handle_id) { h.complete(code, now); }
    }

    pub fn join(&mut self, handle_id: u64, joiner_tid: u64) -> JoinResult {
        if let Some(h) = self.handles.get_mut(&handle_id) {
            if h.is_done() {
                match h.state {
                    JoinState::Completed => JoinResult::Ok,
                    JoinState::Panicked => JoinResult::Panic,
                    JoinState::Cancelled => JoinResult::Cancelled,
                    _ => JoinResult::Ok,
                }
            } else {
                h.add_joiner(joiner_tid);
                JoinResult::Ok // would block
            }
        } else { JoinResult::AlreadyJoined }
    }

    #[inline]
    pub fn create_group(&mut self, join_all: bool) -> u64 {
        let id = self.next_group_id;
        self.next_group_id += 1;
        self.groups.insert(id, JoinGroup::new(id, join_all));
        id
    }

    pub fn stats(&self) -> JoinHandleStats {
        let running = self.handles.values().filter(|h| h.state == JoinState::Running).count() as u32;
        let completed = self.handles.values().filter(|h| h.state == JoinState::Completed).count() as u32;
        let panicked = self.handles.values().filter(|h| h.state == JoinState::Panicked).count() as u32;
        let cancelled = self.handles.values().filter(|h| h.state == JoinState::Cancelled).count() as u32;
        let detached = self.handles.values().filter(|h| h.state == JoinState::Detached).count() as u32;
        let lifetimes: Vec<u64> = self.handles.values().filter(|h| h.is_done()).map(|h| h.lifetime_ns()).collect();
        let avg = if lifetimes.is_empty() { 0 } else { lifetimes.iter().sum::<u64>() / lifetimes.len() as u64 };
        JoinHandleStats {
            total_handles: self.handles.len() as u32, running, completed,
            panicked, cancelled, detached, total_groups: self.groups.len() as u32,
            avg_lifetime_ns: avg,
        }
    }
}

// ============================================================================
// Merged from join_handle_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinStateV2 {
    Running,
    Completed,
    Panicked,
    Cancelled,
    Detached,
}

/// Join result
#[derive(Debug, Clone)]
pub struct JoinResultV2 {
    pub result_hash: u64,
    pub completed_at: u64,
    pub duration_ns: u64,
}

/// Join handle v2
#[derive(Debug)]
pub struct JoinHandleV2 {
    pub id: u64,
    pub task_id: u64,
    pub state: JoinStateV2,
    pub result: Option<JoinResultV2>,
    pub created_at: u64,
    pub joiners: u32,
    pub cancel_requested: bool,
}

impl JoinHandleV2 {
    pub fn new(id: u64, task_id: u64, now: u64) -> Self {
        Self { id, task_id, state: JoinStateV2::Running, result: None, created_at: now, joiners: 0, cancel_requested: false }
    }

    #[inline(always)]
    pub fn complete(&mut self, result_hash: u64, now: u64) {
        self.state = JoinStateV2::Completed;
        self.result = Some(JoinResultV2 { result_hash, completed_at: now, duration_ns: now - self.created_at });
    }

    #[inline(always)]
    pub fn cancel(&mut self) { self.cancel_requested = true; self.state = JoinStateV2::Cancelled; }
    #[inline(always)]
    pub fn detach(&mut self) { self.state = JoinStateV2::Detached; }
    #[inline(always)]
    pub fn is_finished(&self) -> bool { !matches!(self.state, JoinStateV2::Running) }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct JoinHandleV2Stats {
    pub total_handles: u32,
    pub running: u32,
    pub completed: u32,
    pub cancelled: u32,
    pub detached: u32,
    pub avg_duration_ns: u64,
}

/// Main join handle v2 manager
pub struct CoopJoinHandleV2 {
    handles: BTreeMap<u64, JoinHandleV2>,
    next_id: u64,
}

impl CoopJoinHandleV2 {
    pub fn new() -> Self { Self { handles: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn spawn(&mut self, task_id: u64, now: u64) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.handles.insert(id, JoinHandleV2::new(id, task_id, now));
        id
    }

    #[inline(always)]
    pub fn complete(&mut self, id: u64, result_hash: u64, now: u64) {
        if let Some(h) = self.handles.get_mut(&id) { h.complete(result_hash, now); }
    }

    #[inline(always)]
    pub fn cancel(&mut self, id: u64) {
        if let Some(h) = self.handles.get_mut(&id) { h.cancel(); }
    }

    #[inline]
    pub fn stats(&self) -> JoinHandleV2Stats {
        let running = self.handles.values().filter(|h| h.state == JoinStateV2::Running).count() as u32;
        let completed = self.handles.values().filter(|h| h.state == JoinStateV2::Completed).count() as u32;
        let cancelled = self.handles.values().filter(|h| h.state == JoinStateV2::Cancelled).count() as u32;
        let detached = self.handles.values().filter(|h| h.state == JoinStateV2::Detached).count() as u32;
        let durs: Vec<u64> = self.handles.values().filter_map(|h| h.result.as_ref().map(|r| r.duration_ns)).collect();
        let avg = if durs.is_empty() { 0 } else { durs.iter().sum::<u64>() / durs.len() as u64 };
        JoinHandleV2Stats { total_handles: self.handles.len() as u32, running, completed, cancelled, detached, avg_duration_ns: avg }
    }
}
