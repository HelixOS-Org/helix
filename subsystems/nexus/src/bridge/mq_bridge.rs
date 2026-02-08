// SPDX-License-Identifier: GPL-2.0
//! Bridge mq_bridge — POSIX message queue bridge.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// MQ message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MqPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// MQ message
#[derive(Debug)]
pub struct MqMessage {
    pub id: u64,
    pub priority: MqPriority,
    pub data_hash: u64,
    pub size: u32,
    pub sent_at: u64,
}

/// MQ descriptor
#[derive(Debug)]
pub struct MqDescriptor {
    pub name_hash: u64,
    pub max_msg: u32,
    pub max_msgsize: u32,
    pub cur_msgs: u32,
    pub messages: Vec<MqMessage>,
    pub total_sent: u64,
    pub total_received: u64,
    pub total_overflows: u64,
    pub owner_uid: u32,
    pub mode: u32,
}

impl MqDescriptor {
    pub fn new(name: u64, max_msg: u32, max_size: u32, uid: u32) -> Self {
        Self { name_hash: name, max_msg, max_msgsize: max_size, cur_msgs: 0, messages: Vec::new(), total_sent: 0, total_received: 0, total_overflows: 0, owner_uid: uid, mode: 0o660 }
    }

    pub fn send(&mut self, msg: MqMessage) -> bool {
        if self.cur_msgs >= self.max_msg { self.total_overflows += 1; return false; }
        self.cur_msgs += 1;
        self.total_sent += 1;
        self.messages.push(msg);
        self.messages.sort_by(|a, b| b.priority.cmp(&a.priority));
        true
    }

    pub fn receive(&mut self) -> Option<MqMessage> {
        if self.messages.is_empty() { return None; }
        self.cur_msgs -= 1;
        self.total_received += 1;
        Some(self.messages.remove(0))
    }
}

/// Stats
#[derive(Debug, Clone)]
pub struct MqBridgeStats {
    pub total_queues: u32,
    pub total_messages: u32,
    pub total_sent: u64,
    pub total_received: u64,
    pub total_overflows: u64,
}

/// Main bridge MQ
pub struct BridgeMq {
    queues: BTreeMap<u64, MqDescriptor>,
    next_msg_id: u64,
}

impl BridgeMq {
    pub fn new() -> Self { Self { queues: BTreeMap::new(), next_msg_id: 1 } }

    pub fn open(&mut self, name: u64, max_msg: u32, max_size: u32, uid: u32) {
        self.queues.insert(name, MqDescriptor::new(name, max_msg, max_size, uid));
    }

    pub fn send(&mut self, name: u64, prio: MqPriority, data: u64, size: u32, now: u64) -> bool {
        let mid = self.next_msg_id; self.next_msg_id += 1;
        if let Some(q) = self.queues.get_mut(&name) {
            q.send(MqMessage { id: mid, priority: prio, data_hash: data, size, sent_at: now })
        } else { false }
    }

    pub fn receive(&mut self, name: u64) -> Option<MqMessage> {
        if let Some(q) = self.queues.get_mut(&name) { q.receive() }
        else { None }
    }

    pub fn unlink(&mut self, name: u64) { self.queues.remove(&name); }

    pub fn stats(&self) -> MqBridgeStats {
        let msgs: u32 = self.queues.values().map(|q| q.cur_msgs).sum();
        let sent: u64 = self.queues.values().map(|q| q.total_sent).sum();
        let recv: u64 = self.queues.values().map(|q| q.total_received).sum();
        let overflow: u64 = self.queues.values().map(|q| q.total_overflows).sum();
        MqBridgeStats { total_queues: self.queues.len() as u32, total_messages: msgs, total_sent: sent, total_received: recv, total_overflows: overflow }
    }
}
