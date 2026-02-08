// SPDX-License-Identifier: GPL-2.0
//! Coop async_oneshot — async one-shot channel for single value transfer.

extern crate alloc;

/// Oneshot state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneshotState {
    Empty,
    Filled,
    Taken,
    Cancelled,
}

/// Oneshot channel
#[derive(Debug)]
pub struct AsyncOneshot {
    pub state: OneshotState,
    pub value_hash: u64,
    pub sender_tid: u64,
    pub receiver_tid: u64,
    pub created_at: u64,
    pub filled_at: u64,
    pub taken_at: u64,
}

impl AsyncOneshot {
    pub fn new(sender: u64, receiver: u64, now: u64) -> Self {
        Self { state: OneshotState::Empty, value_hash: 0, sender_tid: sender, receiver_tid: receiver, created_at: now, filled_at: 0, taken_at: 0 }
    }

    pub fn send(&mut self, value_hash: u64, now: u64) -> bool {
        if self.state != OneshotState::Empty { return false; }
        self.value_hash = value_hash;
        self.state = OneshotState::Filled;
        self.filled_at = now;
        true
    }

    pub fn recv(&mut self, now: u64) -> Option<u64> {
        if self.state != OneshotState::Filled { return None; }
        self.state = OneshotState::Taken;
        self.taken_at = now;
        Some(self.value_hash)
    }

    pub fn cancel(&mut self) { self.state = OneshotState::Cancelled; }
    pub fn latency_ns(&self) -> u64 { if self.taken_at > self.filled_at { self.taken_at - self.filled_at } else { 0 } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct AsyncOneshotStats {
    pub total_channels: u32,
    pub filled: u32,
    pub taken: u32,
    pub cancelled: u32,
}

/// Main coop async oneshot manager
pub struct CoopAsyncOneshot {
    channels: alloc::vec::Vec<AsyncOneshot>,
}

impl CoopAsyncOneshot {
    pub fn new() -> Self { Self { channels: alloc::vec::Vec::new() } }

    pub fn create(&mut self, sender: u64, receiver: u64, now: u64) -> usize {
        let idx = self.channels.len();
        self.channels.push(AsyncOneshot::new(sender, receiver, now));
        idx
    }

    pub fn send(&mut self, idx: usize, val: u64, now: u64) -> bool {
        if idx < self.channels.len() { self.channels[idx].send(val, now) } else { false }
    }

    pub fn recv(&mut self, idx: usize, now: u64) -> Option<u64> {
        if idx < self.channels.len() { self.channels[idx].recv(now) } else { None }
    }

    pub fn stats(&self) -> AsyncOneshotStats {
        let filled = self.channels.iter().filter(|c| c.state == OneshotState::Filled).count() as u32;
        let taken = self.channels.iter().filter(|c| c.state == OneshotState::Taken).count() as u32;
        let cancelled = self.channels.iter().filter(|c| c.state == OneshotState::Cancelled).count() as u32;
        AsyncOneshotStats { total_channels: self.channels.len() as u32, filled, taken, cancelled }
    }
}
