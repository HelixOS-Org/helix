// SPDX-License-Identifier: GPL-2.0
//! Coop broadcast — reliable broadcast channel for cooperative multi-process messaging.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Broadcast delivery guarantee
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryGuarantee {
    /// Best effort — fire and forget
    BestEffort,
    /// Reliable — ensure at least once delivery
    Reliable,
    /// Causal — preserve causal order
    Causal,
    /// Total order — all subscribers see same order
    TotalOrder,
    /// FIFO — per-publisher order preserved
    Fifo,
}

/// Broadcast message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BroadcastPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
    Critical = 4,
}

/// A broadcast message
#[derive(Debug, Clone)]
pub struct BroadcastMsg {
    pub seq: u64,
    pub topic: String,
    pub publisher_id: u64,
    pub priority: BroadcastPriority,
    pub payload_size: usize,
    pub payload_hash: u64,
    pub timestamp_ns: u64,
    pub ttl_ns: u64,
    pub delivery: DeliveryGuarantee,
    pub ack_count: u32,
}

impl BroadcastMsg {
    pub fn new(seq: u64, topic: String, publisher: u64, payload_size: usize) -> Self {
        Self {
            seq,
            topic,
            publisher_id: publisher,
            priority: BroadcastPriority::Normal,
            payload_size,
            payload_hash: 0,
            timestamp_ns: 0,
            ttl_ns: 30_000_000_000, // 30s default
            delivery: DeliveryGuarantee::BestEffort,
            ack_count: 0,
        }
    }

    pub fn is_expired(&self, now_ns: u64) -> bool {
        if self.ttl_ns == 0 { return false; }
        now_ns.saturating_sub(self.timestamp_ns) > self.ttl_ns
    }
}

/// Subscriber descriptor
#[derive(Debug, Clone)]
pub struct Subscriber {
    pub id: u64,
    pub pid: u64,
    pub topic_filter: String,
    pub delivered_seq: u64,
    pub pending_count: u32,
    pub max_pending: u32,
    pub dropped_count: u64,
    pub total_received: u64,
}

impl Subscriber {
    pub fn new(id: u64, pid: u64, topic: String) -> Self {
        Self {
            id,
            pid,
            topic_filter: topic,
            delivered_seq: 0,
            pending_count: 0,
            max_pending: 1024,
            dropped_count: 0,
            total_received: 0,
        }
    }

    pub fn matches_topic(&self, topic: &str) -> bool {
        if self.topic_filter == "*" { return true; }
        if self.topic_filter.ends_with(".*") {
            let prefix = &self.topic_filter[..self.topic_filter.len() - 2];
            topic.starts_with(prefix)
        } else {
            self.topic_filter == topic
        }
    }

    pub fn can_accept(&self) -> bool {
        self.pending_count < self.max_pending
    }

    pub fn deliver(&mut self, seq: u64) -> bool {
        if !self.can_accept() {
            self.dropped_count += 1;
            return false;
        }
        self.pending_count += 1;
        self.total_received += 1;
        if seq > self.delivered_seq {
            self.delivered_seq = seq;
        }
        true
    }

    pub fn ack(&mut self) {
        self.pending_count = self.pending_count.saturating_sub(1);
    }

    pub fn drop_rate(&self) -> f64 {
        let total = self.total_received + self.dropped_count;
        if total == 0 { return 0.0; }
        self.dropped_count as f64 / total as f64
    }
}

/// A broadcast topic/channel
#[derive(Debug)]
pub struct BroadcastTopic {
    pub name: String,
    pub msg_count: u64,
    pub subscriber_count: u32,
    pub publisher_count: u32,
    pub total_bytes: u64,
    subscribers: Vec<u64>,
    pub created_ns: u64,
}

impl BroadcastTopic {
    pub fn new(name: String, created_ns: u64) -> Self {
        Self {
            name,
            msg_count: 0,
            subscriber_count: 0,
            publisher_count: 0,
            total_bytes: 0,
            subscribers: Vec::new(),
            created_ns,
        }
    }

    pub fn add_subscriber(&mut self, sub_id: u64) {
        if !self.subscribers.contains(&sub_id) {
            self.subscribers.push(sub_id);
            self.subscriber_count += 1;
        }
    }

    pub fn remove_subscriber(&mut self, sub_id: u64) {
        if let Some(pos) = self.subscribers.iter().position(|&s| s == sub_id) {
            self.subscribers.swap_remove(pos);
            self.subscriber_count = self.subscriber_count.saturating_sub(1);
        }
    }

    pub fn record_publish(&mut self, size: usize) {
        self.msg_count += 1;
        self.total_bytes += size as u64;
    }

    pub fn avg_msg_size(&self) -> f64 {
        if self.msg_count == 0 { return 0.0; }
        self.total_bytes as f64 / self.msg_count as f64
    }
}

/// Broadcast stats
#[derive(Debug, Clone)]
pub struct BroadcastStats {
    pub total_topics: u64,
    pub total_subscribers: u64,
    pub total_messages: u64,
    pub total_deliveries: u64,
    pub total_drops: u64,
    pub total_bytes: u64,
    pub total_acks: u64,
}

/// Main broadcast manager
pub struct CoopBroadcast {
    topics: BTreeMap<String, BroadcastTopic>,
    subscribers: BTreeMap<u64, Subscriber>,
    messages: Vec<BroadcastMsg>,
    next_sub_id: u64,
    next_seq: u64,
    max_history: usize,
    stats: BroadcastStats,
}

impl CoopBroadcast {
    pub fn new(max_history: usize) -> Self {
        Self {
            topics: BTreeMap::new(),
            subscribers: BTreeMap::new(),
            messages: Vec::new(),
            next_sub_id: 1,
            next_seq: 1,
            max_history,
            stats: BroadcastStats {
                total_topics: 0,
                total_subscribers: 0,
                total_messages: 0,
                total_deliveries: 0,
                total_drops: 0,
                total_bytes: 0,
                total_acks: 0,
            },
        }
    }

    pub fn create_topic(&mut self, name: String, now_ns: u64) {
        if !self.topics.contains_key(&name) {
            self.topics.insert(name.clone(), BroadcastTopic::new(name, now_ns));
            self.stats.total_topics += 1;
        }
    }

    pub fn subscribe(&mut self, pid: u64, topic: String) -> u64 {
        let id = self.next_sub_id;
        self.next_sub_id += 1;
        let sub = Subscriber::new(id, pid, topic.clone());
        // Add to matching topics
        for t in self.topics.values_mut() {
            if sub.matches_topic(&t.name) {
                t.add_subscriber(id);
            }
        }
        self.subscribers.insert(id, sub);
        self.stats.total_subscribers += 1;
        id
    }

    pub fn unsubscribe(&mut self, sub_id: u64) -> bool {
        if let Some(sub) = self.subscribers.remove(&sub_id) {
            for t in self.topics.values_mut() {
                if sub.matches_topic(&t.name) {
                    t.remove_subscriber(sub_id);
                }
            }
            self.stats.total_subscribers = self.stats.total_subscribers.saturating_sub(1);
            true
        } else {
            false
        }
    }

    pub fn publish(&mut self, topic: &str, publisher: u64, payload_size: usize, payload_hash: u64, now_ns: u64) -> u64 {
        let seq = self.next_seq;
        self.next_seq += 1;

        let mut msg = BroadcastMsg::new(seq, String::from(topic), publisher, payload_size);
        msg.payload_hash = payload_hash;
        msg.timestamp_ns = now_ns;

        if let Some(t) = self.topics.get_mut(topic) {
            t.record_publish(payload_size);
        }

        // Deliver to matching subscribers
        let sub_ids: Vec<u64> = self.subscribers.iter()
            .filter(|(_, s)| s.matches_topic(topic))
            .map(|(&id, _)| id)
            .collect();

        let mut delivered = 0u32;
        for sub_id in sub_ids {
            if let Some(sub) = self.subscribers.get_mut(&sub_id) {
                if sub.deliver(seq) {
                    delivered += 1;
                } else {
                    self.stats.total_drops += 1;
                }
            }
        }
        msg.ack_count = delivered;
        self.stats.total_messages += 1;
        self.stats.total_deliveries += delivered as u64;
        self.stats.total_bytes += payload_size as u64;

        self.messages.push(msg);
        if self.messages.len() > self.max_history {
            self.messages.remove(0);
        }
        seq
    }

    pub fn ack(&mut self, sub_id: u64) {
        if let Some(sub) = self.subscribers.get_mut(&sub_id) {
            sub.ack();
            self.stats.total_acks += 1;
        }
    }

    pub fn expire_messages(&mut self, now_ns: u64) -> usize {
        let before = self.messages.len();
        self.messages.retain(|m| !m.is_expired(now_ns));
        before - self.messages.len()
    }

    pub fn busiest_topics(&self, top: usize) -> Vec<(&str, u64)> {
        let mut v: Vec<(&str, u64)> = self.topics.iter()
            .map(|(name, t)| (name.as_str(), t.msg_count))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        v.truncate(top);
        v
    }

    pub fn slowest_subscribers(&self, top: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<(u64, f64)> = self.subscribers.iter()
            .map(|(&id, s)| (id, s.drop_rate()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(top);
        v
    }

    pub fn get_subscriber(&self, id: u64) -> Option<&Subscriber> {
        self.subscribers.get(&id)
    }

    pub fn stats(&self) -> &BroadcastStats {
        &self.stats
    }
}
