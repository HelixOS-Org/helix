// SPDX-License-Identifier: GPL-2.0
//! Apps madvise_mgr — madvise hint processor for memory management optimization.

extern crate alloc;

use crate::fast::array_map::ArrayMap;
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Madvise advice types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadviseAdvice {
    Normal,
    Random,
    Sequential,
    WillNeed,
    DontNeed,
    Free,
    Remove,
    DontFork,
    DoFork,
    Mergeable,
    Unmergeable,
    HugePage,
    NoHugePage,
    DontDump,
    DoDump,
    Cold,
    PageOut,
    PopulateRead,
    PopulateWrite,
    Collapse,
}

impl MadviseAdvice {
    #[inline(always)]
    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::DontNeed | Self::Free | Self::Remove | Self::PageOut)
    }

    #[inline(always)]
    pub fn affects_thp(&self) -> bool {
        matches!(self, Self::HugePage | Self::NoHugePage | Self::Collapse)
    }

    #[inline(always)]
    pub fn affects_ksm(&self) -> bool {
        matches!(self, Self::Mergeable | Self::Unmergeable)
    }
}

/// Madvise region
#[derive(Debug, Clone, Copy)]
pub struct MadviseRegion {
    pub start: u64,
    pub len: u64,
    pub advice: MadviseAdvice,
    pub applied_at: u64,
}

impl MadviseRegion {
    #[inline(always)]
    pub fn end(&self) -> u64 { self.start + self.len }
    #[inline(always)]
    pub fn pages(&self, page_size: u64) -> u64 {
        (self.len + page_size - 1) / page_size
    }
}

/// Process madvise state
#[derive(Debug)]
#[repr(align(64))]
pub struct ProcessMadviseState {
    pub pid: u32,
    pub active_hints: Vec<MadviseRegion>,
    pub total_calls: u64,
    pub total_bytes_affected: u64,
    pub advice_counts: ArrayMap<u64, 32>,
}

impl ProcessMadviseState {
    pub fn new(pid: u32) -> Self {
        Self {
            pid, active_hints: Vec::new(), total_calls: 0,
            total_bytes_affected: 0, advice_counts: ArrayMap::new(0),
        }
    }

    #[inline]
    pub fn apply_hint(&mut self, start: u64, len: u64, advice: MadviseAdvice, now: u64) {
        self.total_calls += 1;
        self.total_bytes_affected += len;
        self.advice_counts.add(advice as usize, 1);

        // Remove conflicting hints for same range
        self.active_hints.retain(|h| !(h.start == start && h.len == len));
        self.active_hints.push(MadviseRegion { start, len, advice, applied_at: now });
    }

    #[inline]
    pub fn hints_in_range(&self, start: u64, end: u64) -> Vec<&MadviseRegion> {
        self.active_hints.iter()
            .filter(|h| h.start < end && h.end() > start)
            .collect()
    }

    pub fn dominant_advice(&self) -> Option<MadviseAdvice> {
        self.advice_counts.iter()
            .max_by_key(|(_, &count)| count)
            .and_then(|(&k, _)| {
                Some(match k {
                    0 => MadviseAdvice::Normal,
                    1 => MadviseAdvice::Random,
                    2 => MadviseAdvice::Sequential,
                    3 => MadviseAdvice::WillNeed,
                    4 => MadviseAdvice::DontNeed,
                    _ => MadviseAdvice::Normal,
                })
            })
    }
}

/// Madvise operation result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MadviseResult {
    Success,
    InvalidRange,
    InvalidAdvice,
    PermissionDenied,
    NoMemory,
    Busy,
}

/// Madvise event for tracking
#[derive(Debug, Clone, Copy)]
pub struct MadviseEvent {
    pub pid: u32,
    pub start: u64,
    pub len: u64,
    pub advice: MadviseAdvice,
    pub result: MadviseResult,
    pub timestamp: u64,
}

/// Process madvise (pidfd-based) request
#[derive(Debug, Clone, Copy)]
pub struct ProcessMadviseRequest {
    pub target_pid: u32,
    pub requester_pid: u32,
    pub advice: MadviseAdvice,
    pub start: u64,
    pub len: u64,
}

/// Madvise manager stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MadviseMgrStats {
    pub tracked_processes: u32,
    pub total_calls: u64,
    pub total_bytes: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub destructive_calls: u64,
    pub thp_calls: u64,
}

/// Main madvise manager
pub struct AppMadviseMgr {
    processes: BTreeMap<u32, ProcessMadviseState>,
    events: VecDeque<MadviseEvent>,
    max_events: usize,
    success_count: u64,
    failure_count: u64,
    destructive_calls: u64,
    thp_calls: u64,
    page_size: u64,
}

impl AppMadviseMgr {
    pub fn new(page_size: u64) -> Self {
        Self {
            processes: BTreeMap::new(), events: VecDeque::new(),
            max_events: 4096, success_count: 0, failure_count: 0,
            destructive_calls: 0, thp_calls: 0, page_size,
        }
    }

    #[inline]
    pub fn ensure_process(&mut self, pid: u32) {
        if !self.processes.contains_key(&pid) {
            self.processes.insert(pid, ProcessMadviseState::new(pid));
        }
    }

    pub fn apply(&mut self, pid: u32, start: u64, len: u64, advice: MadviseAdvice, now: u64) -> MadviseResult {
        if start % self.page_size != 0 { return MadviseResult::InvalidRange; }
        if len == 0 { return MadviseResult::InvalidRange; }

        self.ensure_process(pid);
        if let Some(state) = self.processes.get_mut(&pid) {
            state.apply_hint(start, len, advice, now);
        }

        if advice.is_destructive() { self.destructive_calls += 1; }
        if advice.affects_thp() { self.thp_calls += 1; }
        self.success_count += 1;

        self.record_event(MadviseEvent { pid, start, len, advice, result: MadviseResult::Success, timestamp: now });
        MadviseResult::Success
    }

    #[inline]
    pub fn process_madvise(&mut self, req: ProcessMadviseRequest, now: u64) -> MadviseResult {
        if req.requester_pid == req.target_pid {
            return self.apply(req.target_pid, req.start, req.len, req.advice, now);
        }
        // Cross-process: only allow non-destructive hints
        if req.advice.is_destructive() {
            self.failure_count += 1;
            return MadviseResult::PermissionDenied;
        }
        self.apply(req.target_pid, req.start, req.len, req.advice, now)
    }

    fn record_event(&mut self, event: MadviseEvent) {
        if self.events.len() >= self.max_events { self.events.pop_front(); }
        self.events.push_back(event);
    }

    #[inline(always)]
    pub fn remove_process(&mut self, pid: u32) -> bool {
        self.processes.remove(&pid).is_some()
    }

    #[inline]
    pub fn hints_for_range(&self, pid: u32, start: u64, end: u64) -> Vec<MadviseRegion> {
        self.processes.get(&pid)
            .map(|s| s.hints_in_range(start, end).into_iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn stats(&self) -> MadviseMgrStats {
        let total_calls: u64 = self.processes.values().map(|p| p.total_calls).sum();
        let total_bytes: u64 = self.processes.values().map(|p| p.total_bytes_affected).sum();
        MadviseMgrStats {
            tracked_processes: self.processes.len() as u32,
            total_calls, total_bytes,
            success_count: self.success_count,
            failure_count: self.failure_count,
            destructive_calls: self.destructive_calls,
            thp_calls: self.thp_calls,
        }
    }
}
