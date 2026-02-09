// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Elimination Stack (contention-reducing lock-free stack)

extern crate alloc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElimSlotState {
    Empty,
    Waiting,
    Busy,
    Matched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElimOpType {
    Push(u64),
    Pop,
}

#[derive(Debug, Clone)]
pub struct ElimSlot {
    pub state: ElimSlotState,
    pub op: Option<ElimOpType>,
    pub value: u64,
    pub thread_id: u32,
    pub exchanges: u64,
    pub timeouts: u64,
}

impl ElimSlot {
    pub fn new() -> Self {
        Self { state: ElimSlotState::Empty, op: None, value: 0, thread_id: 0, exchanges: 0, timeouts: 0 }
    }

    #[inline]
    pub fn offer(&mut self, thread_id: u32, op: ElimOpType) {
        self.state = ElimSlotState::Waiting;
        self.op = Some(op);
        self.thread_id = thread_id;
        if let ElimOpType::Push(v) = op { self.value = v; }
    }

    pub fn try_match(&mut self, other_op: ElimOpType) -> Option<u64> {
        if self.state != ElimSlotState::Waiting { return None; }
        match (self.op, other_op) {
            (Some(ElimOpType::Push(_)), ElimOpType::Pop) => {
                self.state = ElimSlotState::Matched;
                self.exchanges += 1;
                Some(self.value)
            }
            (Some(ElimOpType::Pop), ElimOpType::Push(v)) => {
                self.state = ElimSlotState::Matched;
                self.value = v;
                self.exchanges += 1;
                Some(v)
            }
            _ => None,
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.state = ElimSlotState::Empty;
        self.op = None;
    }
}

#[derive(Debug, Clone)]
pub struct ElimArrayConfig {
    pub num_slots: u32,
    pub timeout_ns: u64,
    pub backoff_base: u64,
}

impl ElimArrayConfig {
    #[inline(always)]
    pub fn default_config() -> Self {
        Self { num_slots: 16, timeout_ns: 1_000_000, backoff_base: 100 }
    }
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ElimStackStats {
    pub total_pushes: u64,
    pub total_pops: u64,
    pub total_eliminations: u64,
    pub total_timeouts: u64,
    pub stack_ops: u64,
    pub elimination_rate: u64,
}

pub struct CoopElimStack {
    slots: Vec<ElimSlot>,
    config: ElimArrayConfig,
    stack_size: u64,
    rng_state: u64,
    stats: ElimStackStats,
}

impl CoopElimStack {
    pub fn new(config: ElimArrayConfig) -> Self {
        let slots = (0..config.num_slots).map(|_| ElimSlot::new()).collect();
        Self {
            slots, config, stack_size: 0,
            rng_state: 0xdeadbeefcafe1234,
            stats: ElimStackStats {
                total_pushes: 0, total_pops: 0,
                total_eliminations: 0, total_timeouts: 0,
                stack_ops: 0, elimination_rate: 0,
            },
        }
    }

    fn random_slot(&mut self) -> usize {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as usize) % self.slots.len()
    }

    pub fn push(&mut self, thread_id: u32, value: u64) {
        self.stats.total_pushes += 1;
        let slot_idx = self.random_slot();
        if let Some(slot) = self.slots.get_mut(slot_idx) {
            if slot.try_match(ElimOpType::Push(value)).is_some() {
                self.stats.total_eliminations += 1;
                slot.reset();
                return;
            }
            slot.offer(thread_id, ElimOpType::Push(value));
        }
        self.stack_size += 1;
        self.stats.stack_ops += 1;
    }

    pub fn pop(&mut self, thread_id: u32) -> Option<u64> {
        self.stats.total_pops += 1;
        let slot_idx = self.random_slot();
        if let Some(slot) = self.slots.get_mut(slot_idx) {
            if let Some(val) = slot.try_match(ElimOpType::Pop) {
                self.stats.total_eliminations += 1;
                slot.reset();
                return Some(val);
            }
        }
        if self.stack_size > 0 {
            self.stack_size -= 1;
            self.stats.stack_ops += 1;
            Some(0)
        } else { None }
    }

    #[inline(always)]
    pub fn stats(&self) -> &ElimStackStats { &self.stats }
}
