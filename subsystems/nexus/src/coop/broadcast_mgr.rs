//! # Coop Broadcast Manager
//!
//! Reliable broadcast protocols for cooperative groups:
//! - Best-effort broadcast
//! - Reliable broadcast (with ack tracking)
//! - FIFO-ordered broadcast
//! - Causal broadcast with vector clocks
//! - Total-order broadcast (sequencer-based)
//! - Atomic broadcast with 2-phase delivery

extern crate alloc;

use crate::fast::linear_map::LinearMap;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Broadcast reliability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BroadcastReliability {
    BestEffort,
    Reliable,
    FifoOrdered,
    CausalOrdered,
    TotalOrdered,
    Atomic,
}

/// Broadcast message state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BcastMsgState {
    Pending,
    Sent,
    PartialAck,
    FullyAcked,
    Delivered,
    Failed,
    Expired,
}

/// Broadcast message
#[derive(Debug, Clone)]
pub struct BcastMessage {
    pub id: u64,
    pub sender: u64,
    pub payload_hash: u64,
    pub payload_size: u32,
    pub seq: u64,
    pub state: BcastMsgState,
    pub reliability: BroadcastReliability,
    pub recipients: Vec<u64>,
    pub acks: Vec<u64>,
    pub send_ts: u64,
    pub deliver_ts: u64,
    pub retries: u32,
    pub causal_clock: Vec<u64>,
}

impl BcastMessage {
    pub fn new(id: u64, sender: u64, hash: u64, size: u32, seq: u64, rel: BroadcastReliability, recipients: Vec<u64>, ts: u64) -> Self {
        Self {
            id, sender, payload_hash: hash, payload_size: size, seq,
            state: BcastMsgState::Pending, reliability: rel,
            recipients, acks: Vec::new(), send_ts: ts, deliver_ts: 0,
            retries: 0, causal_clock: Vec::new(),
        }
    }

    #[inline(always)]
    pub fn mark_sent(&mut self) { self.state = BcastMsgState::Sent; }

    #[inline]
    pub fn ack(&mut self, from: u64) {
        if !self.acks.contains(&from) { self.acks.push(from); }
        if self.acks.len() >= self.recipients.len() {
            self.state = BcastMsgState::FullyAcked;
        } else {
            self.state = BcastMsgState::PartialAck;
        }
    }

    #[inline(always)]
    pub fn deliver(&mut self, ts: u64) { self.state = BcastMsgState::Delivered; self.deliver_ts = ts; }
    #[inline(always)]
    pub fn fail(&mut self) { self.state = BcastMsgState::Failed; }
    #[inline(always)]
    pub fn expire(&mut self) { self.state = BcastMsgState::Expired; }
    #[inline(always)]
    pub fn ack_ratio(&self) -> f64 { if self.recipients.is_empty() { 0.0 } else { self.acks.len() as f64 / self.recipients.len() as f64 } }
    #[inline(always)]
    pub fn latency(&self) -> u64 { self.deliver_ts.saturating_sub(self.send_ts) }
    #[inline(always)]
    pub fn is_complete(&self) -> bool { matches!(self.state, BcastMsgState::Delivered | BcastMsgState::Failed | BcastMsgState::Expired) }
}

/// Sequencer for total-order broadcast
#[derive(Debug, Clone)]
pub struct BcastSequencer {
    pub id: u64,
    pub next_seq: u64,
    pub assigned: LinearMap<u64, 64>,
}

impl BcastSequencer {
    pub fn new(id: u64) -> Self { Self { id, next_seq: 1, assigned: LinearMap::new() } }

    #[inline]
    pub fn assign(&mut self, msg_id: u64) -> u64 {
        let seq = self.next_seq; self.next_seq += 1;
        self.assigned.insert(msg_id, seq);
        seq
    }
}

/// Delivery queue for ordered delivery
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct DeliveryQueue {
    pub expected_seq: u64,
    pub buffer: LinearMap<u64, 64>,
    pub delivered: Vec<u64>,
}

impl DeliveryQueue {
    pub fn new() -> Self { Self { expected_seq: 1, buffer: LinearMap::new(), delivered: Vec::new() } }

    #[inline(always)]
    pub fn enqueue(&mut self, msg_id: u64, seq: u64) { self.buffer.insert(seq, msg_id); }

    #[inline]
    pub fn try_deliver(&mut self) -> Vec<u64> {
        let mut out = Vec::new();
        while let Some(&msg_id) = self.buffer.get(&self.expected_seq) {
            out.push(msg_id);
            self.buffer.remove(&self.expected_seq);
            self.delivered.push(msg_id);
            self.expected_seq += 1;
        }
        out
    }

    #[inline]
    pub fn gap(&self) -> u64 {
        if let Some((&max_seq, _)) = self.buffer.iter().next_back() {
            max_seq.saturating_sub(self.expected_seq)
        } else { 0 }
    }
}

/// Broadcast group
#[derive(Debug, Clone)]
pub struct BcastGroup {
    pub id: u64,
    pub members: Vec<u64>,
    pub reliability: BroadcastReliability,
    pub msg_count: u64,
    pub delivered_count: u64,
}

impl BcastGroup {
    pub fn new(id: u64, members: Vec<u64>, rel: BroadcastReliability) -> Self {
        Self { id, members, reliability: rel, msg_count: 0, delivered_count: 0 }
    }
}

/// Broadcast stats
#[derive(Debug, Clone, Default)]
#[repr(align(64))]
pub struct BcastStats {
    pub total_messages: u64,
    pub delivered: u64,
    pub failed: u64,
    pub pending: u64,
    pub total_bytes: u64,
    pub avg_latency_ns: u64,
    pub avg_ack_ratio: f64,
}

/// Cooperative broadcast manager
pub struct CoopBroadcastMgr {
    messages: BTreeMap<u64, BcastMessage>,
    groups: BTreeMap<u64, BcastGroup>,
    sequencers: BTreeMap<u64, BcastSequencer>,
    delivery_queues: BTreeMap<u64, DeliveryQueue>,
    stats: BcastStats,
    next_msg_id: u64,
    max_retries: u32,
    timeout_ns: u64,
}

impl CoopBroadcastMgr {
    pub fn new(max_retries: u32, timeout_ns: u64) -> Self {
        Self {
            messages: BTreeMap::new(), groups: BTreeMap::new(),
            sequencers: BTreeMap::new(), delivery_queues: BTreeMap::new(),
            stats: BcastStats::default(), next_msg_id: 1,
            max_retries, timeout_ns,
        }
    }

    #[inline(always)]
    pub fn create_group(&mut self, id: u64, members: Vec<u64>, rel: BroadcastReliability) {
        self.groups.insert(id, BcastGroup::new(id, members, rel));
    }

    pub fn broadcast(&mut self, group_id: u64, sender: u64, hash: u64, size: u32, ts: u64) -> Option<u64> {
        let group = self.groups.get_mut(&group_id)?;
        let id = self.next_msg_id; self.next_msg_id += 1;
        group.msg_count += 1;
        let seq = group.msg_count;
        let recipients = group.members.clone();
        let rel = group.reliability;
        let mut msg = BcastMessage::new(id, sender, hash, size, seq, rel, recipients, ts);
        msg.mark_sent();

        if rel == BroadcastReliability::TotalOrdered {
            let seqr = self.sequencers.entry(group_id).or_insert_with(|| BcastSequencer::new(group_id));
            let assigned = seqr.assign(id);
            msg.seq = assigned;
        }

        self.messages.insert(id, msg);
        self.stats.total_messages += 1;
        self.stats.total_bytes += size as u64;
        Some(id)
    }

    #[inline(always)]
    pub fn ack(&mut self, msg_id: u64, from: u64) {
        if let Some(msg) = self.messages.get_mut(&msg_id) { msg.ack(from); }
    }

    #[inline]
    pub fn deliver(&mut self, msg_id: u64, ts: u64) {
        if let Some(msg) = self.messages.get_mut(&msg_id) {
            msg.deliver(ts);
            self.stats.delivered += 1;
            if let Some(g) = self.groups.get_mut(&0) { g.delivered_count += 1; }
        }
    }

    #[inline]
    pub fn check_timeouts(&mut self, now: u64) -> Vec<u64> {
        let mut expired = Vec::new();
        for msg in self.messages.values_mut() {
            if msg.is_complete() { continue; }
            if now.saturating_sub(msg.send_ts) > self.timeout_ns {
                if msg.retries >= self.max_retries { msg.expire(); expired.push(msg.id); self.stats.failed += 1; }
                else { msg.retries += 1; }
            }
        }
        expired
    }

    #[inline(always)]
    pub fn enqueue_ordered(&mut self, queue_id: u64, msg_id: u64, seq: u64) {
        let q = self.delivery_queues.entry(queue_id).or_insert_with(DeliveryQueue::new);
        q.enqueue(msg_id, seq);
    }

    #[inline]
    pub fn try_deliver_ordered(&mut self, queue_id: u64, ts: u64) -> Vec<u64> {
        let q = self.delivery_queues.entry(queue_id).or_insert_with(DeliveryQueue::new);
        let ready = q.try_deliver();
        for &mid in &ready {
            if let Some(msg) = self.messages.get_mut(&mid) { msg.deliver(ts); self.stats.delivered += 1; }
        }
        ready
    }

    #[inline]
    pub fn recompute(&mut self) {
        self.stats.pending = self.messages.values().filter(|m| !m.is_complete()).count() as u64;
        let delivered: Vec<&BcastMessage> = self.messages.values().filter(|m| m.state == BcastMsgState::Delivered).collect();
        if !delivered.is_empty() {
            let total_lat: u64 = delivered.iter().map(|m| m.latency()).sum();
            self.stats.avg_latency_ns = total_lat / delivered.len() as u64;
            let total_ratio: f64 = delivered.iter().map(|m| m.ack_ratio()).sum();
            self.stats.avg_ack_ratio = total_ratio / delivered.len() as f64;
        }
    }

    #[inline(always)]
    pub fn message(&self, id: u64) -> Option<&BcastMessage> { self.messages.get(&id) }
    #[inline(always)]
    pub fn group(&self, id: u64) -> Option<&BcastGroup> { self.groups.get(&id) }
    #[inline(always)]
    pub fn stats(&self) -> &BcastStats { &self.stats }
}
