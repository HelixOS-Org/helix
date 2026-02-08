// SPDX-License-Identifier: GPL-2.0
//! NEXUS Coop — Exchanger (thread rendezvous value exchange)

extern crate alloc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeResult {
    Success(u64),
    TimedOut,
    Interrupted,
    Busy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExchangeSlotState {
    Empty,
    Offering(u64),
    Matched,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct ExchangeSlot {
    pub state: ExchangeSlotState,
    pub offered_value: u64,
    pub received_value: u64,
    pub offerer_id: u32,
    pub matcher_id: u32,
    pub exchanges: u64,
}

impl ExchangeSlot {
    pub fn new() -> Self {
        Self {
            state: ExchangeSlotState::Empty,
            offered_value: 0, received_value: 0,
            offerer_id: 0, matcher_id: 0, exchanges: 0,
        }
    }

    pub fn offer(&mut self, thread_id: u32, value: u64) {
        self.state = ExchangeSlotState::Offering(value);
        self.offered_value = value;
        self.offerer_id = thread_id;
    }

    pub fn try_exchange(&mut self, thread_id: u32, value: u64) -> Option<u64> {
        if let ExchangeSlotState::Offering(offered) = self.state {
            self.matcher_id = thread_id;
            self.received_value = value;
            self.state = ExchangeSlotState::Matched;
            self.exchanges += 1;
            Some(offered)
        } else { None }
    }

    pub fn collect(&mut self) -> Option<u64> {
        if self.state == ExchangeSlotState::Matched {
            self.state = ExchangeSlotState::Empty;
            Some(self.received_value)
        } else { None }
    }

    pub fn cancel(&mut self) { self.state = ExchangeSlotState::Cancelled; }
    pub fn reset(&mut self) { self.state = ExchangeSlotState::Empty; }
}

#[derive(Debug, Clone)]
pub struct ExchangerArena {
    pub slots: Vec<ExchangeSlot>,
    pub size: u32,
}

impl ExchangerArena {
    pub fn new(size: u32) -> Self {
        let slots = (0..size).map(|_| ExchangeSlot::new()).collect();
        Self { slots, size }
    }

    pub fn total_exchanges(&self) -> u64 {
        self.slots.iter().map(|s| s.exchanges).sum()
    }
}

#[derive(Debug, Clone)]
pub struct ExchangerStats {
    pub total_offers: u64,
    pub total_exchanges: u64,
    pub total_timeouts: u64,
    pub total_cancels: u64,
    pub avg_wait_ns: u64,
}

pub struct CoopExchanger {
    arena: ExchangerArena,
    rng_state: u64,
    stats: ExchangerStats,
}

impl CoopExchanger {
    pub fn new(arena_size: u32) -> Self {
        Self {
            arena: ExchangerArena::new(arena_size),
            rng_state: 0xfeedface12345678,
            stats: ExchangerStats {
                total_offers: 0, total_exchanges: 0,
                total_timeouts: 0, total_cancels: 0,
                avg_wait_ns: 0,
            },
        }
    }

    fn random_slot(&mut self) -> usize {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
        (self.rng_state as usize) % self.arena.size as usize
    }

    pub fn exchange(&mut self, thread_id: u32, value: u64) -> ExchangeResult {
        self.stats.total_offers += 1;
        let idx = self.random_slot();
        if let Some(slot) = self.arena.slots.get_mut(idx) {
            if let Some(received) = slot.try_exchange(thread_id, value) {
                self.stats.total_exchanges += 1;
                return ExchangeResult::Success(received);
            }
            slot.offer(thread_id, value);
        }
        ExchangeResult::Busy
    }

    pub fn success_rate(&self) -> u64 {
        if self.stats.total_offers == 0 { 0 }
        else { (self.stats.total_exchanges * 100) / self.stats.total_offers }
    }

    pub fn stats(&self) -> &ExchangerStats { &self.stats }
}
