//! # Bridge MSG Bridge
//!
//! System V message queue (msgget/msgsnd/msgrcv/msgctl) bridging:
//! - Message queue creation and management
//! - Message type-based routing
//! - Send/receive with priority and blocking semantics
//! - Per-queue byte and message limits
//! - IPC namespace isolation
//! - Queue utilization statistics

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// Message queue permission
#[derive(Debug, Clone, Copy)]
pub struct MsgPerm {
    pub uid: u32,
    pub gid: u32,
    pub cuid: u32,
    pub cgid: u32,
    pub mode: u16,
}

impl MsgPerm {
    pub fn new(uid: u32, gid: u32, mode: u16) -> Self {
        Self { uid, gid, cuid: uid, cgid: gid, mode }
    }
}

/// Message entry in queue
#[derive(Debug, Clone)]
pub struct MsgEntry {
    pub msg_type: i64,
    pub size: usize,
    pub sender_pid: u64,
    pub sent_ts: u64,
}

impl MsgEntry {
    pub fn new(msg_type: i64, size: usize, pid: u64, ts: u64) -> Self {
        Self { msg_type, size, sender_pid: pid, sent_ts: ts }
    }
}

/// Message queue state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgQueueState {
    Active,
    Full,
    Draining,
    Destroyed,
}

/// Message queue descriptor
#[derive(Debug, Clone)]
pub struct MsgQueue {
    pub msq_id: u32,
    pub key: i32,
    pub perm: MsgPerm,
    pub state: MsgQueueState,
    pub messages: Vec<MsgEntry>,
    pub max_bytes: usize,
    pub current_bytes: usize,
    pub max_msgs: u32,
    pub creator_pid: u64,
    pub last_send_pid: u64,
    pub last_recv_pid: u64,
    pub send_time: u64,
    pub recv_time: u64,
    pub change_time: u64,
    pub total_sent: u64,
    pub total_received: u64,
    pub send_waiter_count: u32,
    pub recv_waiter_count: u32,
    pub ns_id: u64,
}

impl MsgQueue {
    pub fn new(id: u32, key: i32, perm: MsgPerm, ts: u64) -> Self {
        Self {
            msq_id: id, key, perm, state: MsgQueueState::Active,
            messages: Vec::new(), max_bytes: 16384, current_bytes: 0,
            max_msgs: 1024, creator_pid: 0, last_send_pid: 0,
            last_recv_pid: 0, send_time: 0, recv_time: 0, change_time: ts,
            total_sent: 0, total_received: 0, send_waiter_count: 0,
            recv_waiter_count: 0, ns_id: 0,
        }
    }

    pub fn can_send(&self, size: usize) -> bool {
        self.state == MsgQueueState::Active
            && self.current_bytes + size <= self.max_bytes
            && (self.messages.len() as u32) < self.max_msgs
    }

    pub fn send(&mut self, msg_type: i64, size: usize, pid: u64, ts: u64) -> bool {
        if !self.can_send(size) { return false; }
        self.messages.push(MsgEntry::new(msg_type, size, pid, ts));
        self.current_bytes += size;
        self.last_send_pid = pid;
        self.send_time = ts;
        self.total_sent += 1;
        if self.current_bytes >= self.max_bytes {
            self.state = MsgQueueState::Full;
        }
        true
    }

    pub fn receive(&mut self, msg_type: i64, pid: u64, ts: u64) -> Option<MsgEntry> {
        let idx = if msg_type == 0 {
            if self.messages.is_empty() { None } else { Some(0) }
        } else if msg_type > 0 {
            self.messages.iter().position(|m| m.msg_type == msg_type)
        } else {
            // negative: lowest type <= |msg_type|
            let abs_type = -msg_type;
            let mut best: Option<(usize, i64)> = None;
            for (i, m) in self.messages.iter().enumerate() {
                if m.msg_type <= abs_type {
                    match best {
                        None => best = Some((i, m.msg_type)),
                        Some((_, bt)) if m.msg_type < bt => best = Some((i, m.msg_type)),
                        _ => {}
                    }
                }
            }
            best.map(|(i, _)| i)
        };

        if let Some(i) = idx {
            let msg = self.messages.remove(i);
            self.current_bytes = self.current_bytes.saturating_sub(msg.size);
            self.last_recv_pid = pid;
            self.recv_time = ts;
            self.total_received += 1;
            if self.state == MsgQueueState::Full && self.current_bytes < self.max_bytes {
                self.state = MsgQueueState::Active;
            }
            Some(msg)
        } else { None }
    }

    pub fn utilization(&self) -> f64 {
        if self.max_bytes == 0 { return 0.0; }
        self.current_bytes as f64 / self.max_bytes as f64
    }

    pub fn message_count(&self) -> usize { self.messages.len() }

    pub fn avg_msg_size(&self) -> f64 {
        if self.messages.is_empty() { return 0.0; }
        self.current_bytes as f64 / self.messages.len() as f64
    }

    pub fn type_histogram(&self) -> BTreeMap<i64, u32> {
        let mut hist = BTreeMap::new();
        for m in &self.messages {
            *hist.entry(m.msg_type).or_insert(0) += 1;
        }
        hist
    }
}

/// MSG bridge stats
#[derive(Debug, Clone, Default)]
pub struct MsgBridgeStats {
    pub total_queues: usize,
    pub active_queues: usize,
    pub full_queues: usize,
    pub total_messages: usize,
    pub total_bytes: usize,
    pub total_sent: u64,
    pub total_received: u64,
    pub peak_queues: usize,
}

/// Bridge message queue manager
pub struct BridgeMsgBridge {
    queues: BTreeMap<u32, MsgQueue>,
    key_to_id: BTreeMap<i32, u32>,
    next_id: u32,
    stats: MsgBridgeStats,
}

impl BridgeMsgBridge {
    pub fn new() -> Self {
        Self {
            queues: BTreeMap::new(), key_to_id: BTreeMap::new(),
            next_id: 1, stats: MsgBridgeStats::default(),
        }
    }

    pub fn msgget(&mut self, key: i32, uid: u32, gid: u32, mode: u16, ts: u64) -> u32 {
        if key != 0 {
            if let Some(&existing) = self.key_to_id.get(&key) {
                return existing;
            }
        }
        let id = self.next_id;
        self.next_id += 1;
        let q = MsgQueue::new(id, key, MsgPerm::new(uid, gid, mode), ts);
        self.queues.insert(id, q);
        if key != 0 { self.key_to_id.insert(key, id); }
        id
    }

    pub fn msgsnd(&mut self, msq_id: u32, msg_type: i64, size: usize, pid: u64, ts: u64) -> bool {
        if let Some(q) = self.queues.get_mut(&msq_id) {
            q.send(msg_type, size, pid, ts)
        } else { false }
    }

    pub fn msgrcv(&mut self, msq_id: u32, msg_type: i64, pid: u64, ts: u64) -> Option<MsgEntry> {
        if let Some(q) = self.queues.get_mut(&msq_id) {
            q.receive(msg_type, pid, ts)
        } else { None }
    }

    pub fn msgctl_rmid(&mut self, msq_id: u32) -> bool {
        if let Some(q) = self.queues.get(&msq_id) {
            let key = q.key;
            self.queues.remove(&msq_id);
            if key != 0 { self.key_to_id.remove(&key); }
            true
        } else { false }
    }

    pub fn set_max_bytes(&mut self, msq_id: u32, max: usize) {
        if let Some(q) = self.queues.get_mut(&msq_id) { q.max_bytes = max; }
    }

    pub fn recompute(&mut self) {
        self.stats.total_queues = self.queues.len();
        self.stats.active_queues = self.queues.values().filter(|q| q.state == MsgQueueState::Active).count();
        self.stats.full_queues = self.queues.values().filter(|q| q.state == MsgQueueState::Full).count();
        self.stats.total_messages = self.queues.values().map(|q| q.messages.len()).sum();
        self.stats.total_bytes = self.queues.values().map(|q| q.current_bytes).sum();
        self.stats.total_sent = self.queues.values().map(|q| q.total_sent).sum();
        self.stats.total_received = self.queues.values().map(|q| q.total_received).sum();
        if self.stats.total_queues > self.stats.peak_queues { self.stats.peak_queues = self.stats.total_queues; }
    }

    pub fn queue(&self, id: u32) -> Option<&MsgQueue> { self.queues.get(&id) }
    pub fn stats(&self) -> &MsgBridgeStats { &self.stats }
}
