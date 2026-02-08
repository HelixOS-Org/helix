// SPDX-License-Identifier: GPL-2.0
//! Coop broadcast_chan — broadcast channel for multi-consumer.

extern crate alloc;

use alloc::vec::Vec;

/// Broadcast channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BroadcastState {
    Active,
    Closed,
    Full,
}

/// Broadcast message
#[derive(Debug, Clone)]
pub struct BroadcastMsg {
    pub seq: u64,
    pub data_hash: u64,
    pub sender_id: u64,
    pub timestamp: u64,
}

/// Subscriber
#[derive(Debug)]
pub struct BroadcastSubscriber {
    pub id: u64,
    pub last_seen_seq: u64,
    pub recv_count: u64,
    pub lag: u64,
}

impl BroadcastSubscriber {
    pub fn new(id: u64) -> Self { Self { id, last_seen_seq: 0, recv_count: 0, lag: 0 } }
}

/// Broadcast channel
#[derive(Debug)]
pub struct BroadcastChannel {
    pub id: u64,
    pub state: BroadcastState,
    pub buffer: Vec<BroadcastMsg>,
    pub capacity: usize,
    pub subscribers: Vec<BroadcastSubscriber>,
    pub send_seq: u64,
    pub send_count: u64,
    pub overflow_count: u64,
}

impl BroadcastChannel {
    pub fn new(id: u64, capacity: usize) -> Self {
        Self { id, state: BroadcastState::Active, buffer: Vec::new(), subscribers: Vec::new(), capacity, send_seq: 0, send_count: 0, overflow_count: 0 }
    }

    pub fn subscribe(&mut self) -> u64 {
        let sub_id = self.subscribers.len() as u64 + 1;
        let mut sub = BroadcastSubscriber::new(sub_id);
        sub.last_seen_seq = self.send_seq;
        self.subscribers.push(sub);
        sub_id
    }

    pub fn send(&mut self, data_hash: u64, sender_id: u64, now: u64) -> u64 {
        self.send_seq += 1;
        let msg = BroadcastMsg { seq: self.send_seq, data_hash, sender_id, timestamp: now };
        if self.buffer.len() >= self.capacity { self.buffer.remove(0); self.overflow_count += 1; }
        self.buffer.push(msg);
        self.send_count += 1;
        self.send_seq
    }

    pub fn recv(&mut self, subscriber_id: u64) -> Option<BroadcastMsg> {
        let sub = self.subscribers.iter_mut().find(|s| s.id == subscriber_id)?;
        let msg = self.buffer.iter().find(|m| m.seq > sub.last_seen_seq)?.clone();
        sub.last_seen_seq = msg.seq;
        sub.recv_count += 1;
        Some(msg)
    }

    pub fn close(&mut self) { self.state = BroadcastState::Closed; }

    pub fn max_lag(&self) -> u64 {
        self.subscribers.iter().map(|s| self.send_seq.saturating_sub(s.last_seen_seq)).max().unwrap_or(0)
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct BroadcastChanStats {
    pub total_channels: u32,
    pub total_subscribers: u32,
    pub total_sent: u64,
    pub total_overflow: u64,
    pub max_lag: u64,
}

/// Main broadcast channel manager
pub struct CoopBroadcastChan {
    channels: Vec<BroadcastChannel>,
    next_id: u64,
}

impl CoopBroadcastChan {
    pub fn new() -> Self { Self { channels: Vec::new(), next_id: 1 } }

    pub fn create(&mut self, capacity: usize) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.channels.push(BroadcastChannel::new(id, capacity));
        id
    }

    pub fn stats(&self) -> BroadcastChanStats {
        let subs: u32 = self.channels.iter().map(|c| c.subscribers.len() as u32).sum();
        let sent: u64 = self.channels.iter().map(|c| c.send_count).sum();
        let overflow: u64 = self.channels.iter().map(|c| c.overflow_count).sum();
        let lag = self.channels.iter().map(|c| c.max_lag()).max().unwrap_or(0);
        BroadcastChanStats { total_channels: self.channels.len() as u32, total_subscribers: subs, total_sent: sent, total_overflow: overflow, max_lag: lag }
    }
}
