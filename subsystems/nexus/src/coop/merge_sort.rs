// SPDX-License-Identifier: GPL-2.0
//! Coop merge_sort — cooperative parallel merge sort.

extern crate alloc;

use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

/// Sort state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortState {
    Idle,
    Splitting,
    Sorting,
    Merging,
    Complete,
}

/// Merge task (unit of work)
#[derive(Debug, Clone)]
pub struct MergeTask {
    pub id: u64,
    pub level: u32,
    pub left_start: usize,
    pub left_end: usize,
    pub right_start: usize,
    pub right_end: usize,
    pub state: SortState,
    pub comparisons: u64,
    pub swaps: u64,
}

impl MergeTask {
    pub fn new(id: u64, level: u32, ls: usize, le: usize, rs: usize, re: usize) -> Self {
        Self { id, level, left_start: ls, left_end: le, right_start: rs, right_end: re, state: SortState::Idle, comparisons: 0, swaps: 0 }
    }

    #[inline(always)]
    pub fn elements(&self) -> usize {
        (self.left_end - self.left_start) + (self.right_end - self.right_start)
    }
}

/// Sort session (one sort operation)
#[derive(Debug)]
pub struct SortSession {
    pub id: u64,
    pub data: Vec<u64>,
    pub order: SortOrder,
    pub state: SortState,
    pub tasks: Vec<MergeTask>,
    pub total_comparisons: u64,
    pub total_swaps: u64,
    pub started_at: u64,
    pub completed_at: u64,
}

impl SortSession {
    pub fn new(id: u64, data: Vec<u64>, order: SortOrder, now: u64) -> Self {
        Self {
            id, data, order, state: SortState::Idle,
            tasks: Vec::new(), total_comparisons: 0, total_swaps: 0,
            started_at: now, completed_at: 0,
        }
    }

    pub fn generate_tasks(&mut self) {
        self.state = SortState::Splitting;
        let n = self.data.len();
        if n <= 1 { self.state = SortState::Complete; return; }

        let mut task_id = 0u64;
        let mut size = 1usize;
        let mut level = 0u32;
        while size < n {
            let mut i = 0;
            while i < n {
                let left_start = i;
                let left_end = (i + size).min(n);
                let right_start = left_end;
                let right_end = (i + 2 * size).min(n);
                if right_start < right_end {
                    self.tasks.push(MergeTask::new(task_id, level, left_start, left_end, right_start, right_end));
                    task_id += 1;
                }
                i += 2 * size;
            }
            size *= 2;
            level += 1;
        }
        self.state = SortState::Sorting;
    }

    pub fn execute_task(&mut self, task_idx: usize) -> bool {
        if task_idx >= self.tasks.len() { return false; }
        let task = &mut self.tasks[task_idx];
        task.state = SortState::Merging;

        // Merge in-place (simplified via temp buffer)
        let left = self.data[task.left_start..task.left_end].to_vec();
        let right = self.data[task.right_start..task.right_end].to_vec();
        let mut i = 0;
        let mut j = 0;
        let mut k = task.left_start;
        let ascending = self.order == SortOrder::Ascending;

        while i < left.len() && j < right.len() {
            task.comparisons += 1;
            let take_left = if ascending { left[i] <= right[j] } else { left[i] >= right[j] };
            if take_left { self.data[k] = left[i]; i += 1; }
            else { self.data[k] = right[j]; j += 1; task.swaps += 1; }
            k += 1;
        }
        while i < left.len() { self.data[k] = left[i]; i += 1; k += 1; }
        while j < right.len() { self.data[k] = right[j]; j += 1; k += 1; }

        task.state = SortState::Complete;
        self.total_comparisons += task.comparisons;
        self.total_swaps += task.swaps;
        true
    }

    #[inline(always)]
    pub fn is_complete(&self) -> bool { self.tasks.iter().all(|t| t.state == SortState::Complete) }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MergeSortStats {
    pub total_sessions: u32,
    pub active_sessions: u32,
    pub total_comparisons: u64,
    pub total_swaps: u64,
    pub total_elements_sorted: u64,
}

/// Main merge sort manager
pub struct CoopMergeSort {
    sessions: BTreeMap<u64, SortSession>,
    next_id: u64,
}

impl CoopMergeSort {
    pub fn new() -> Self { Self { sessions: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn start_sort(&mut self, data: Vec<u64>, order: SortOrder, now: u64) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut session = SortSession::new(id, data, order, now);
        session.generate_tasks();
        self.sessions.insert(id, session);
        id
    }

    #[inline]
    pub fn step(&mut self, session_id: u64) -> bool {
        if let Some(session) = self.sessions.get_mut(&session_id) {
            let next = session.tasks.iter().position(|t| t.state == SortState::Idle);
            if let Some(idx) = next { session.execute_task(idx) }
            else { false }
        } else { false }
    }

    #[inline]
    pub fn stats(&self) -> MergeSortStats {
        let active = self.sessions.values().filter(|s| !s.is_complete()).count() as u32;
        let comps: u64 = self.sessions.values().map(|s| s.total_comparisons).sum();
        let swaps: u64 = self.sessions.values().map(|s| s.total_swaps).sum();
        let elems: u64 = self.sessions.values().map(|s| s.data.len() as u64).sum();
        MergeSortStats {
            total_sessions: self.sessions.len() as u32, active_sessions: active,
            total_comparisons: comps, total_swaps: swaps, total_elements_sorted: elems,
        }
    }
}
