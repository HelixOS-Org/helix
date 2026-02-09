// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Flat Combining (batched lock-free operations)

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatCombineOpType {
    Insert,
    Remove,
    Lookup,
    Update,
    Custom(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlatCombineState {
    Pending,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct FlatCombineRequest {
    pub thread_id: u32,
    pub op_type: FlatCombineOpType,
    pub key: u64,
    pub value: u64,
    pub state: FlatCombineState,
    pub result: Option<u64>,
    pub timestamp: u64,
}

impl FlatCombineRequest {
    pub fn new(thread_id: u32, op: FlatCombineOpType, key: u64, value: u64, ts: u64) -> Self {
        Self { thread_id, op_type: op, key, value, state: FlatCombineState::Pending, result: None, timestamp: ts }
    }

    #[inline(always)]
    pub fn complete(&mut self, result: u64) {
        self.state = FlatCombineState::Completed;
        self.result = Some(result);
    }
}

#[derive(Debug, Clone)]
pub struct FlatCombineSlot {
    pub thread_id: u32,
    pub request: Option<FlatCombineRequest>,
    pub age: u64,
    pub combine_count: u64,
}

impl FlatCombineSlot {
    pub fn new(thread_id: u32) -> Self {
        Self { thread_id, request: None, age: 0, combine_count: 0 }
    }

    #[inline(always)]
    pub fn publish(&mut self, req: FlatCombineRequest) {
        self.request = Some(req);
        self.age += 1;
    }

    #[inline(always)]
    pub fn take_result(&mut self) -> Option<u64> {
        self.request.take().and_then(|r| r.result)
    }
}

#[derive(Debug, Clone)]
pub struct FlatCombineRound {
    pub combiner_id: u32,
    pub ops_combined: u32,
    pub round_latency_ns: u64,
}

#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct FlatCombineStats {
    pub total_threads: u32,
    pub total_operations: u64,
    pub total_rounds: u64,
    pub total_ops_combined: u64,
    pub avg_batch_size: u64,
    pub combiner_changes: u64,
}

pub struct CoopFlatCombine {
    slots: BTreeMap<u32, FlatCombineSlot>,
    current_combiner: Option<u32>,
    lock: AtomicU64,
    stats: FlatCombineStats,
}

impl CoopFlatCombine {
    pub fn new() -> Self {
        Self {
            slots: BTreeMap::new(),
            current_combiner: None,
            lock: AtomicU64::new(0),
            stats: FlatCombineStats {
                total_threads: 0, total_operations: 0,
                total_rounds: 0, total_ops_combined: 0,
                avg_batch_size: 0, combiner_changes: 0,
            },
        }
    }

    #[inline(always)]
    pub fn register_thread(&mut self, id: u32) {
        self.slots.insert(id, FlatCombineSlot::new(id));
        self.stats.total_threads += 1;
    }

    #[inline]
    pub fn publish_request(&mut self, thread_id: u32, req: FlatCombineRequest) {
        if let Some(slot) = self.slots.get_mut(&thread_id) {
            slot.publish(req);
            self.stats.total_operations += 1;
        }
    }

    pub fn combine_round(&mut self, combiner_id: u32) -> u32 {
        let mut combined = 0u32;
        for slot in self.slots.values_mut() {
            if let Some(ref mut req) = slot.request {
                if req.state == FlatCombineState::Pending {
                    req.complete(req.key.wrapping_add(req.value));
                    slot.combine_count += 1;
                    combined += 1;
                }
            }
        }
        if self.current_combiner != Some(combiner_id) {
            self.current_combiner = Some(combiner_id);
            self.stats.combiner_changes += 1;
        }
        self.stats.total_rounds += 1;
        self.stats.total_ops_combined += combined as u64;
        if self.stats.total_rounds > 0 {
            self.stats.avg_batch_size = self.stats.total_ops_combined / self.stats.total_rounds;
        }
        combined
    }

    #[inline(always)]
    pub fn stats(&self) -> &FlatCombineStats { &self.stats }
}
