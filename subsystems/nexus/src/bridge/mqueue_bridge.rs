// SPDX-License-Identifier: GPL-2.0
//! Bridge mqueue_bridge — POSIX message queue bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Message priority (0-31)
pub type MqPriority = u32;

/// Queue attributes
#[derive(Debug, Clone)]
pub struct MqAttr {
    pub flags: u32,
    pub max_msg: u32,
    pub msg_size: u32,
    pub cur_msgs: u32,
}

impl MqAttr {
    pub fn new(max_msg: u32, msg_size: u32) -> Self { Self { flags: 0, max_msg, msg_size, cur_msgs: 0 } }
}

/// Message
#[derive(Debug, Clone)]
pub struct MqMessage {
    pub priority: MqPriority,
    pub size: u32,
    pub data_hash: u64,
    pub sender_pid: u64,
    pub timestamp: u64,
}

/// Message queue
#[derive(Debug)]
#[repr(align(64))]
pub struct MessageQueue {
    pub id: u64,
    pub name_hash: u64,
    pub attr: MqAttr,
    pub messages: VecDeque<MqMessage>,
    pub readers_waiting: u32,
    pub writers_waiting: u32,
    pub send_count: u64,
    pub recv_count: u64,
    pub owner_uid: u32,
    pub permissions: u32,
}

impl MessageQueue {
    pub fn new(id: u64, name_hash: u64, max_msg: u32, msg_size: u32) -> Self {
        Self { id, name_hash, attr: MqAttr::new(max_msg, msg_size), messages: VecDeque::new(), readers_waiting: 0, writers_waiting: 0, send_count: 0, recv_count: 0, owner_uid: 0, permissions: 0o644 }
    }

    #[inline]
    pub fn send(&mut self, msg: MqMessage) -> bool {
        if self.messages.len() as u32 >= self.attr.max_msg { return false; }
        self.messages.push_back(msg);
        self.messages.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.attr.cur_msgs += 1;
        self.send_count += 1;
        true
    }

    #[inline]
    pub fn receive(&mut self) -> Option<MqMessage> {
        if self.messages.is_empty() { return None; }
        self.attr.cur_msgs -= 1;
        self.recv_count += 1;
        self.messages.pop_front()
    }

    #[inline(always)]
    pub fn is_full(&self) -> bool { self.attr.cur_msgs >= self.attr.max_msg }
    #[inline(always)]
    pub fn is_empty(&self) -> bool { self.messages.is_empty() }
}

/// Stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MqueueBridgeStats {
    pub total_queues: u32,
    pub total_messages: u32,
    pub total_sent: u64,
    pub total_received: u64,
    pub full_queues: u32,
}

/// Main mqueue bridge
#[repr(align(64))]
pub struct BridgeMqueue {
    queues: BTreeMap<u64, MessageQueue>,
    next_id: u64,
}

impl BridgeMqueue {
    pub fn new() -> Self { Self { queues: BTreeMap::new(), next_id: 1 } }

    #[inline]
    pub fn create(&mut self, name_hash: u64, max_msg: u32, msg_size: u32) -> u64 {
        let id = self.next_id; self.next_id += 1;
        self.queues.insert(id, MessageQueue::new(id, name_hash, max_msg, msg_size));
        id
    }

    #[inline(always)]
    pub fn destroy(&mut self, id: u64) { self.queues.remove(&id); }

    #[inline(always)]
    pub fn send(&mut self, queue: u64, msg: MqMessage) -> bool {
        if let Some(q) = self.queues.get_mut(&queue) { q.send(msg) } else { false }
    }

    #[inline(always)]
    pub fn receive(&mut self, queue: u64) -> Option<MqMessage> {
        self.queues.get_mut(&queue)?.receive()
    }

    #[inline]
    pub fn stats(&self) -> MqueueBridgeStats {
        let msgs: u32 = self.queues.values().map(|q| q.attr.cur_msgs).sum();
        let sent: u64 = self.queues.values().map(|q| q.send_count).sum();
        let recv: u64 = self.queues.values().map(|q| q.recv_count).sum();
        let full = self.queues.values().filter(|q| q.is_full()).count() as u32;
        MqueueBridgeStats { total_queues: self.queues.len() as u32, total_messages: msgs, total_sent: sent, total_received: recv, full_queues: full }
    }
}

// ============================================================================
// Merged from mqueue_v2_bridge
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqueueV2Op {
    Open,
    Close,
    Unlink,
    Send,
    Receive,
    TimedSend,
    TimedReceive,
    Getattr,
    Setattr,
    Notify,
}

/// Mqueue v2 result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MqueueV2Result {
    Success,
    WouldBlock,
    Timeout,
    QueueFull,
    PermissionDenied,
    Error,
}

/// Mqueue v2 record
#[derive(Debug, Clone)]
pub struct MqueueV2Record {
    pub op: MqueueV2Op,
    pub result: MqueueV2Result,
    pub mqd: i32,
    pub priority: u32,
    pub msg_size: u32,
    pub name_hash: u64,
}

impl MqueueV2Record {
    pub fn new(op: MqueueV2Op, name: &[u8]) -> Self {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
        Self { op, result: MqueueV2Result::Success, mqd: -1, priority: 0, msg_size: 0, name_hash: h }
    }
}

/// Mqueue v2 bridge stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct MqueueV2BridgeStats {
    pub total_ops: u64,
    pub sends: u64,
    pub receives: u64,
    pub queues_opened: u64,
    pub timeouts: u64,
}

/// Main bridge mqueue v2
#[derive(Debug)]
pub struct BridgeMqueueV2 {
    pub stats: MqueueV2BridgeStats,
}

impl BridgeMqueueV2 {
    pub fn new() -> Self {
        Self { stats: MqueueV2BridgeStats { total_ops: 0, sends: 0, receives: 0, queues_opened: 0, timeouts: 0 } }
    }

    #[inline]
    pub fn record(&mut self, rec: &MqueueV2Record) {
        self.stats.total_ops += 1;
        match rec.op {
            MqueueV2Op::Send | MqueueV2Op::TimedSend => self.stats.sends += 1,
            MqueueV2Op::Receive | MqueueV2Op::TimedReceive => self.stats.receives += 1,
            MqueueV2Op::Open => self.stats.queues_opened += 1,
            _ => {}
        }
        if rec.result == MqueueV2Result::Timeout { self.stats.timeouts += 1; }
    }
}
