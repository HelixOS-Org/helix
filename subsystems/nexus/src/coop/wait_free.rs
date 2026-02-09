// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Wait-Free (universal wait-free data structures)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitFreeOpType {
    Read,
    Write,
    CAS,
    FAA,
    Swap,
    Announce,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitFreeProgress {
    Completed,
    Helping,
    Pending,
}

#[derive(Debug, Clone)]
pub struct WaitFreeAnnouncement {
    pub thread_id: u32,
    pub op_type: WaitFreeOpType,
    pub sequence: u64,
    pub progress: WaitFreeProgress,
    pub helped_by: Option<u32>,
}

impl WaitFreeAnnouncement {
    pub fn new(thread_id: u32, op_type: WaitFreeOpType, seq: u64) -> Self {
        Self { thread_id, op_type, sequence: seq, progress: WaitFreeProgress::Pending, helped_by: None }
    }

    #[inline(always)]
    pub fn complete(&mut self) { self.progress = WaitFreeProgress::Completed; }
    #[inline(always)]
    pub fn help(&mut self, helper: u32) {
        self.progress = WaitFreeProgress::Helping;
        self.helped_by = Some(helper);
    }
}

#[derive(Debug, Clone)]
pub struct WaitFreeRegister {
    pub id: u32,
    pub value: AtomicU64,
    pub version: AtomicU64,
    pub reads: u64,
    pub writes: u64,
}

impl WaitFreeRegister {
    pub fn new(id: u32) -> Self {
        Self { id, value: AtomicU64::new(0), version: AtomicU64::new(0), reads: 0, writes: 0 }
    }

    #[inline]
    pub fn read(&self) -> (u64, u64) {
        let ver = self.version.load(Ordering::Acquire);
        let val = self.value.load(Ordering::Acquire);
        (val, ver)
    }

    #[inline(always)]
    pub fn write(&self, val: u64) -> u64 {
        self.value.store(val, Ordering::Release);
        self.version.fetch_add(1, Ordering::Release)
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WaitFreeThreadState {
    pub thread_id: u32,
    pub total_ops: u64,
    pub total_helps_given: u64,
    pub total_helps_received: u64,
    pub max_phase: u64,
}

impl WaitFreeThreadState {
    pub fn new(thread_id: u32) -> Self {
        Self { thread_id, total_ops: 0, total_helps_given: 0, total_helps_received: 0, max_phase: 0 }
    }

    #[inline(always)]
    pub fn record_op(&mut self) { self.total_ops += 1; }
    #[inline(always)]
    pub fn record_help_given(&mut self) { self.total_helps_given += 1; }
    #[inline(always)]
    pub fn record_help_received(&mut self) { self.total_helps_received += 1; }

    #[inline(always)]
    pub fn help_ratio(&self) -> u64 {
        if self.total_ops == 0 { 0 } else { (self.total_helps_given * 100) / self.total_ops }
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct WaitFreeStats {
    pub total_threads: u32,
    pub total_ops: u64,
    pub total_helps: u64,
    pub total_announcements: u64,
    pub max_helping_chain: u32,
}

pub struct CoopWaitFree {
    threads: BTreeMap<u32, WaitFreeThreadState>,
    announcements: Vec<WaitFreeAnnouncement>,
    next_seq: AtomicU64,
    stats: WaitFreeStats,
}

impl CoopWaitFree {
    pub fn new() -> Self {
        Self {
            threads: BTreeMap::new(),
            announcements: Vec::new(),
            next_seq: AtomicU64::new(1),
            stats: WaitFreeStats {
                total_threads: 0, total_ops: 0,
                total_helps: 0, total_announcements: 0,
                max_helping_chain: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register_thread(&mut self, id: u32) {
        self.threads.insert(id, WaitFreeThreadState::new(id));
        self.stats.total_threads += 1;
    }

    #[inline]
    pub fn announce(&mut self, thread_id: u32, op_type: WaitFreeOpType) -> u64 {
        let seq = self.next_seq.fetch_add(1, Ordering::Relaxed);
        self.announcements.push(WaitFreeAnnouncement::new(thread_id, op_type, seq));
        self.stats.total_announcements += 1;
        if let Some(t) = self.threads.get_mut(&thread_id) {
            t.record_op();
            self.stats.total_ops += 1;
        }
        seq
    }

    pub fn help_pending(&mut self, helper_id: u32) -> u32 {
        let mut helped = 0u32;
        for ann in &mut self.announcements {
            if ann.progress == WaitFreeProgress::Pending {
                ann.help(helper_id);
                helped += 1;
                self.stats.total_helps += 1;
            }
        }
        if let Some(t) = self.threads.get_mut(&helper_id) {
            t.total_helps_given += helped as u64;
        }
        helped
    }

    #[inline(always)]
    pub fn stats(&self) -> &WaitFreeStats { &self.stats }
}
