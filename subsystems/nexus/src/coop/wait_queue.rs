// SPDX-License-Identifier: GPL-2.0
//! Coop wait_queue_v2 — generation-2 wait queue implementation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Wait queue v2 entry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WqV2EntryState {
    Waiting,
    Woken,
    TimedOut,
    Interrupted,
}

/// Wait queue v2 entry
#[derive(Debug)]
pub struct WqV2Entry {
    pub tid: u64,
    pub state: WqV2EntryState,
    pub exclusive: bool,
    pub enqueue_time: u64,
    pub wake_time: u64,
    pub key: u64,
}

impl WqV2Entry {
    pub fn new(tid: u64, exclusive: bool, key: u64, now: u64) -> Self {
        Self { tid, state: WqV2EntryState::Waiting, exclusive, enqueue_time: now, wake_time: 0, key }
    }
}

/// Wait queue v2
#[derive(Debug)]
#[repr(align(64))]
pub struct WaitQueueV2 {
    pub id: u64,
    pub entries: Vec<WqV2Entry>,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_timeouts: u64,
}

impl WaitQueueV2 {
    pub fn new(id: u64) -> Self {
        Self { id, entries: Vec::new(), total_waits: 0, total_wakes: 0, total_timeouts: 0 }
    }

    #[inline(always)]
    pub fn enqueue(&mut self, tid: u64, exclusive: bool, key: u64, now: u64) {
        self.entries.push(WqV2Entry::new(tid, exclusive, key, now));
        self.total_waits += 1;
    }

    #[inline]
    pub fn wake_one(&mut self, now: u64) -> Option<u64> {
        for entry in &mut self.entries {
            if entry.state == WqV2EntryState::Waiting {
                entry.state = WqV2EntryState::Woken;
                entry.wake_time = now;
                self.total_wakes += 1;
                return Some(entry.tid);
            }
        }
        None
    }

    pub fn wake_all(&mut self, now: u64) -> u32 {
        let mut woken = 0u32;
        for entry in &mut self.entries {
            if entry.state == WqV2EntryState::Waiting {
                entry.state = WqV2EntryState::Woken;
                entry.wake_time = now;
                woken += 1;
            }
        }
        self.total_wakes += woken as u64;
        woken
    }

    #[inline]
    pub fn drain_woken(&mut self) -> Vec<WqV2Entry> {
        let mut woken = Vec::new();
        let mut remaining = Vec::new();
        for entry in self.entries.drain(..) {
            if entry.state == WqV2EntryState::Woken { woken.push(entry); }
            else { remaining.push(entry); }
        }
        self.entries = remaining;
        woken
    }

    #[inline(always)]
    pub fn waiting_count(&self) -> u32 {
        self.entries.iter().filter(|e| e.state == WqV2EntryState::Waiting).count() as u32
    }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WaitQueueV2Stats {
    pub total_queues: u32,
    pub total_waits: u64,
    pub total_wakes: u64,
    pub total_waiting: u32,
}

/// Main coop wait queue v2 manager
#[repr(align(64))]
pub struct CoopWaitQueueV2 {
    queues: BTreeMap<u64, WaitQueueV2>,
    next_id: u64,
}

impl CoopWaitQueueV2 {
    pub fn new() -> Self { Self { queues: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.queues.insert(id, WaitQueueV2::new(id));
        id
    }

    #[inline(always)]
    pub fn enqueue(&mut self, qid: u64, tid: u64, exclusive: bool, key: u64, now: u64) {
        if let Some(q) = self.queues.get_mut(&qid) { q.enqueue(tid, exclusive, key, now); }
    }

    #[inline(always)]
    pub fn wake_one(&mut self, qid: u64, now: u64) -> Option<u64> {
        if let Some(q) = self.queues.get_mut(&qid) { q.wake_one(now) } else { None }
    }

    #[inline(always)]
    pub fn wake_all(&mut self, qid: u64, now: u64) -> u32 {
        if let Some(q) = self.queues.get_mut(&qid) { q.wake_all(now) } else { 0 }
    }

    #[inline(always)]
    pub fn destroy(&mut self, qid: u64) { self.queues.remove(&qid); }

    #[inline]
    pub fn stats(&self) -> WaitQueueV2Stats {
        let waits: u64 = self.queues.values().map(|q| q.total_waits).sum();
        let wakes: u64 = self.queues.values().map(|q| q.total_wakes).sum();
        let waiting: u32 = self.queues.values().map(|q| q.waiting_count()).sum();
        WaitQueueV2Stats { total_queues: self.queues.len() as u32, total_waits: waits, total_wakes: wakes, total_waiting: waiting }
    }
}
