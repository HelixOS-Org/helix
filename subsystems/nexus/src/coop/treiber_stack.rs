// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Treiber Stack (lock-free LIFO stack)

extern crate alloc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreiberOpResult {
    Success,
    Empty,
    Contended,
    AbaDetected,
}

#[derive(Debug, Clone)]
pub struct TreiberNode {
    pub value: u64,
    pub next: Option<u64>,
    pub tag: u64,
}

impl TreiberNode {
    pub fn new(value: u64) -> Self {
        Self { value, next: None, tag: 0 }
    }
}

#[derive(Debug, Clone)]
pub struct TreiberStackState {
    pub head_tag: u64,
    pub size: u64,
    pub push_count: u64,
    pub pop_count: u64,
    pub cas_failures: u64,
    pub aba_detections: u64,
    pub max_size: u64,
}

impl TreiberStackState {
    pub fn new() -> Self {
        Self {
            head_tag: 0, size: 0,
            push_count: 0, pop_count: 0,
            cas_failures: 0, aba_detections: 0,
            max_size: 0,
        }
    }

    pub fn push(&mut self) {
        self.size += 1;
        self.push_count += 1;
        self.head_tag += 1;
        if self.size > self.max_size { self.max_size = self.size; }
    }

    pub fn pop(&mut self) -> bool {
        if self.size == 0 { return false; }
        self.size -= 1;
        self.pop_count += 1;
        self.head_tag += 1;
        true
    }

    pub fn record_cas_fail(&mut self) { self.cas_failures += 1; }

    pub fn contention_rate(&self) -> u64 {
        let total = self.push_count + self.pop_count;
        if total == 0 { 0 } else { (self.cas_failures * 100) / total }
    }
}

#[derive(Debug, Clone)]
pub struct TreiberStackStats {
    pub total_stacks: u64,
    pub total_pushes: u64,
    pub total_pops: u64,
    pub total_cas_failures: u64,
    pub total_empty_pops: u64,
}

pub struct CoopTreiberStack {
    stacks: Vec<TreiberStackState>,
    stats: TreiberStackStats,
}

impl CoopTreiberStack {
    pub fn new() -> Self {
        Self {
            stacks: Vec::new(),
            stats: TreiberStackStats {
                total_stacks: 0, total_pushes: 0,
                total_pops: 0, total_cas_failures: 0,
                total_empty_pops: 0,
            },
        }
    }

    pub fn create_stack(&mut self) -> usize {
        let idx = self.stacks.len();
        self.stacks.push(TreiberStackState::new());
        self.stats.total_stacks += 1;
        idx
    }

    pub fn push(&mut self, stack_idx: usize) {
        if let Some(s) = self.stacks.get_mut(stack_idx) {
            s.push();
            self.stats.total_pushes += 1;
        }
    }

    pub fn pop(&mut self, stack_idx: usize) -> TreiberOpResult {
        if let Some(s) = self.stacks.get_mut(stack_idx) {
            if s.pop() {
                self.stats.total_pops += 1;
                TreiberOpResult::Success
            } else {
                self.stats.total_empty_pops += 1;
                TreiberOpResult::Empty
            }
        } else { TreiberOpResult::Empty }
    }

    pub fn stats(&self) -> &TreiberStackStats { &self.stats }
}
