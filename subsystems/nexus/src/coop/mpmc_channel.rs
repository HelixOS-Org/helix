// SPDX-License-Identifier: GPL-2.0
//! Coop mpmc_channel — multi-producer multi-consumer bounded channel.

extern crate alloc;

use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpmcState {
    Open,
    SenderClosed,
    ReceiverClosed,
    Closed,
}

/// MPMC channel message
#[derive(Debug)]
pub struct MpmcMsg {
    pub id: u64,
    pub sender_tid: u64,
    pub data_hash: u64,
    pub size: u32,
    pub timestamp: u64,
}

/// MPMC channel
#[derive(Debug)]
pub struct MpmcChannel {
    pub capacity: u32,
    pub state: MpmcState,
    pub buffer: VecDeque<MpmcMsg>,
    pub senders: u32,
    pub receivers: u32,
    pub total_sent: u64,
    pub total_received: u64,
    pub total_full_blocks: u64,
    pub total_empty_blocks: u64,
}

impl MpmcChannel {
    pub fn new(cap: u32) -> Self {
        Self { capacity: cap, state: MpmcState::Open, buffer: VecDeque::new(), senders: 0, receivers: 0, total_sent: 0, total_received: 0, total_full_blocks: 0, total_empty_blocks: 0 }
    }

    #[inline]
    pub fn send(&mut self, msg: MpmcMsg) -> bool {
        if self.buffer.len() as u32 >= self.capacity { self.total_full_blocks += 1; return false; }
        self.buffer.push_back(msg);
        self.total_sent += 1;
        true
    }

    #[inline]
    pub fn recv(&mut self) -> Option<MpmcMsg> {
        if self.buffer.is_empty() { self.total_empty_blocks += 1; return None; }
        self.total_received += 1;
        self.buffer.pop_front()
    }

    #[inline(always)]
    pub fn len(&self) -> u32 { self.buffer.len() as u32 }
    #[inline(always)]
    pub fn is_full(&self) -> bool { self.buffer.len() as u32 >= self.capacity }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MpmcChannelStats {
    pub total_channels: u32,
    pub total_sent: u64,
    pub total_received: u64,
    pub total_blocked: u64,
}

/// Main coop MPMC channel manager
pub struct CoopMpmcChannel {
    channels: Vec<MpmcChannel>,
    next_msg_id: u64,
}

impl CoopMpmcChannel {
    pub fn new() -> Self { Self { channels: Vec::new(), next_msg_id: 1 } }

    #[inline]
    pub fn create(&mut self, cap: u32) -> usize {
        let idx = self.channels.len();
        self.channels.push(MpmcChannel::new(cap));
        idx
    }

    #[inline]
    pub fn send(&mut self, ch: usize, sender: u64, data: u64, size: u32, now: u64) -> bool {
        if ch >= self.channels.len() { return false; }
        let mid = self.next_msg_id; self.next_msg_id += 1;
        self.channels[ch].send(MpmcMsg { id: mid, sender_tid: sender, data_hash: data, size, timestamp: now })
    }

    #[inline(always)]
    pub fn recv(&mut self, ch: usize) -> Option<MpmcMsg> {
        if ch < self.channels.len() { self.channels[ch].recv() } else { None }
    }

    #[inline]
    pub fn stats(&self) -> MpmcChannelStats {
        let sent: u64 = self.channels.iter().map(|c| c.total_sent).sum();
        let recv: u64 = self.channels.iter().map(|c| c.total_received).sum();
        let blocked: u64 = self.channels.iter().map(|c| c.total_full_blocks + c.total_empty_blocks).sum();
        MpmcChannelStats { total_channels: self.channels.len() as u32, total_sent: sent, total_received: recv, total_blocked: blocked }
    }
}
