//! # Cooperative Event Bus
//!
//! Publish-subscribe event bus for kernel subsystem cooperation:
//! - Topic-based event routing
//! - Priority-ordered delivery
//! - Subscriber filtering
//! - Event persistence for replay
//! - Backpressure management
//! - Dead-letter queue for failed deliveries

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Event priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventBusPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Event delivery status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryStatus {
    Pending,
    Delivered,
    Failed,
    DeadLettered,
    Expired,
}

/// Event payload (generic kernel event)
#[derive(Debug, Clone)]
pub struct BusEvent {
    pub event_id: u64,
    pub topic_hash: u64,
    pub source_id: u64,
    pub priority: EventBusPriority,
    pub payload_type: u32,
    pub payload_size: u32,
    pub timestamp: u64,
    pub ttl_ns: u64,
    pub sequence: u64,
}

impl BusEvent {
    pub fn new(event_id: u64, topic_hash: u64, source_id: u64) -> Self {
        Self {
            event_id,
            topic_hash,
            source_id,
            priority: EventBusPriority::Normal,
            payload_type: 0,
            payload_size: 0,
            timestamp: 0,
            ttl_ns: 10_000_000_000, // 10s default
            sequence: 0,
        }
    }

    pub fn is_expired(&self, now: u64) -> bool {
        self.ttl_ns > 0 && now > self.timestamp + self.ttl_ns
    }
}

/// Subscriber filter
#[derive(Debug, Clone)]
pub struct SubscriberFilter {
    pub min_priority: EventBusPriority,
    pub source_whitelist: Vec<u64>,
    pub payload_types: Vec<u32>,
}

impl SubscriberFilter {
    pub fn accept_all() -> Self {
        Self {
            min_priority: EventBusPriority::Low,
            source_whitelist: Vec::new(),
            payload_types: Vec::new(),
        }
    }

    pub fn matches(&self, event: &BusEvent) -> bool {
        if event.priority < self.min_priority { return false; }
        if !self.source_whitelist.is_empty() && !self.source_whitelist.contains(&event.source_id) {
            return false;
        }
        if !self.payload_types.is_empty() && !self.payload_types.contains(&event.payload_type) {
            return false;
        }
        true
    }
}

/// Subscriber registration
#[derive(Debug, Clone)]
pub struct Subscriber {
    pub subscriber_id: u64,
    pub filter: SubscriberFilter,
    pub pending_count: u64,
    pub delivered_count: u64,
    pub failed_count: u64,
    pub max_pending: u32,
    pub active: bool,
}

impl Subscriber {
    pub fn new(subscriber_id: u64) -> Self {
        Self {
            subscriber_id,
            filter: SubscriberFilter::accept_all(),
            pending_count: 0,
            delivered_count: 0,
            failed_count: 0,
            max_pending: 1024,
            active: true,
        }
    }

    pub fn is_backpressured(&self) -> bool {
        self.pending_count as u32 >= self.max_pending
    }
}

/// Topic with subscribers
#[derive(Debug, Clone)]
pub struct EventTopic {
    pub topic_hash: u64,
    pub subscribers: Vec<u64>,
    pub total_published: u64,
    pub total_delivered: u64,
    pub retained_event: Option<BusEvent>,
}

impl EventTopic {
    pub fn new(topic_hash: u64) -> Self {
        Self {
            topic_hash,
            subscribers: Vec::new(),
            total_published: 0,
            total_delivered: 0,
            retained_event: None,
        }
    }
}

/// Dead letter entry
#[derive(Debug, Clone)]
pub struct DeadLetter {
    pub event: BusEvent,
    pub subscriber_id: u64,
    pub reason: DeliveryStatus,
    pub timestamp: u64,
}

/// Coop event bus stats
#[derive(Debug, Clone, Default)]
pub struct CoopEventBusStats {
    pub total_topics: usize,
    pub total_subscribers: usize,
    pub total_published: u64,
    pub total_delivered: u64,
    pub total_dead_letters: usize,
    pub backpressured_subs: usize,
}

/// Cooperative Event Bus
pub struct CoopEventBus {
    topics: BTreeMap<u64, EventTopic>,
    subscribers: BTreeMap<u64, Subscriber>,
    dead_letters: Vec<DeadLetter>,
    next_event_id: u64,
    sequence: u64,
    max_dead_letters: usize,
    stats: CoopEventBusStats,
}

impl CoopEventBus {
    pub fn new() -> Self {
        Self {
            topics: BTreeMap::new(),
            subscribers: BTreeMap::new(),
            dead_letters: Vec::new(),
            next_event_id: 1,
            sequence: 0,
            max_dead_letters: 1024,
            stats: CoopEventBusStats::default(),
        }
    }

    pub fn create_topic(&mut self, topic_hash: u64) {
        self.topics.entry(topic_hash).or_insert_with(|| EventTopic::new(topic_hash));
    }

    pub fn subscribe(&mut self, subscriber_id: u64, topic_hash: u64) {
        self.subscribers.entry(subscriber_id)
            .or_insert_with(|| Subscriber::new(subscriber_id));
        if let Some(topic) = self.topics.get_mut(&topic_hash) {
            if !topic.subscribers.contains(&subscriber_id) {
                topic.subscribers.push(subscriber_id);
            }
        }
        self.recompute();
    }

    pub fn unsubscribe(&mut self, subscriber_id: u64, topic_hash: u64) {
        if let Some(topic) = self.topics.get_mut(&topic_hash) {
            topic.subscribers.retain(|&s| s != subscriber_id);
        }
        self.recompute();
    }

    pub fn publish(&mut self, topic_hash: u64, source_id: u64, now: u64) -> u64 {
        let event_id = self.next_event_id;
        self.next_event_id += 1;
        self.sequence += 1;

        let mut event = BusEvent::new(event_id, topic_hash, source_id);
        event.timestamp = now;
        event.sequence = self.sequence;

        if let Some(topic) = self.topics.get_mut(&topic_hash) {
            topic.total_published += 1;

            let subs: Vec<u64> = topic.subscribers.clone();
            for &sub_id in &subs {
                if let Some(sub) = self.subscribers.get_mut(&sub_id) {
                    if !sub.active { continue; }
                    if !sub.filter.matches(&event) { continue; }
                    if sub.is_backpressured() {
                        self.dead_letters.push(DeadLetter {
                            event: event.clone(),
                            subscriber_id: sub_id,
                            reason: DeliveryStatus::Failed,
                            timestamp: now,
                        });
                        sub.failed_count += 1;
                    } else {
                        sub.pending_count += 1;
                        sub.delivered_count += 1;
                        topic.total_delivered += 1;
                    }
                }
            }

            topic.retained_event = Some(event);
        }

        if self.dead_letters.len() > self.max_dead_letters {
            self.dead_letters.drain(..self.dead_letters.len() / 2);
        }

        self.recompute();
        event_id
    }

    pub fn ack(&mut self, subscriber_id: u64) {
        if let Some(sub) = self.subscribers.get_mut(&subscriber_id) {
            if sub.pending_count > 0 { sub.pending_count -= 1; }
        }
    }

    pub fn expire_events(&mut self, now: u64) {
        // Clean up expired retained events
        for topic in self.topics.values_mut() {
            if let Some(ref event) = topic.retained_event {
                if event.is_expired(now) {
                    topic.retained_event = None;
                }
            }
        }
    }

    fn recompute(&mut self) {
        self.stats.total_topics = self.topics.len();
        self.stats.total_subscribers = self.subscribers.len();
        self.stats.total_published = self.topics.values().map(|t| t.total_published).sum();
        self.stats.total_delivered = self.topics.values().map(|t| t.total_delivered).sum();
        self.stats.total_dead_letters = self.dead_letters.len();
        self.stats.backpressured_subs = self.subscribers.values()
            .filter(|s| s.is_backpressured()).count();
    }

    pub fn stats(&self) -> &CoopEventBusStats {
        &self.stats
    }

    pub fn topic(&self, hash: u64) -> Option<&EventTopic> {
        self.topics.get(&hash)
    }
}

// ============================================================================
// Merged from event_bus_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventBusPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Event v2
#[derive(Debug)]
pub struct BusEventV2 {
    pub id: u64,
    pub topic_hash: u64,
    pub priority: EventBusPriority,
    pub data_hash: u64,
    pub timestamp: u64,
    pub delivered: u32,
    pub dropped: u32,
}

/// Subscriber v2
#[derive(Debug)]
pub struct BusSubscriberV2 {
    pub id: u64,
    pub topics: Vec<u64>,
    pub filter_mask: u64,
    pub received: u64,
    pub dropped: u64,
    pub queue_capacity: u32,
    pub queue_used: u32,
}

impl BusSubscriberV2 {
    pub fn new(id: u64, cap: u32) -> Self {
        Self { id, topics: Vec::new(), filter_mask: u64::MAX, received: 0, dropped: 0, queue_capacity: cap, queue_used: 0 }
    }

    pub fn subscribe(&mut self, topic: u64) {
        if !self.topics.contains(&topic) { self.topics.push(topic); }
    }

    pub fn unsubscribe(&mut self, topic: u64) { self.topics.retain(|&t| t != topic); }

    pub fn deliver(&mut self) -> bool {
        if self.queue_used >= self.queue_capacity { self.dropped += 1; return false; }
        self.queue_used += 1;
        self.received += 1;
        true
    }

    pub fn consume(&mut self) { if self.queue_used > 0 { self.queue_used -= 1; } }
}

/// Stats
#[derive(Debug, Clone)]
pub struct EventBusV2Stats {
    pub total_subscribers: u32,
    pub total_events_published: u64,
    pub total_delivered: u64,
    pub total_dropped: u64,
    pub active_topics: u32,
}

/// Main coop event bus v2
pub struct CoopEventBusV2 {
    subscribers: BTreeMap<u64, BusSubscriberV2>,
    events_published: u64,
    total_delivered: u64,
    total_dropped: u64,
    topics: BTreeMap<u64, u32>,
    next_sub_id: u64,
    next_event_id: u64,
}

impl CoopEventBusV2 {
    pub fn new() -> Self {
        Self { subscribers: BTreeMap::new(), events_published: 0, total_delivered: 0, total_dropped: 0, topics: BTreeMap::new(), next_sub_id: 1, next_event_id: 1 }
    }

    pub fn add_subscriber(&mut self, cap: u32) -> u64 {
        let id = self.next_sub_id; self.next_sub_id += 1;
        self.subscribers.insert(id, BusSubscriberV2::new(id, cap));
        id
    }

    pub fn subscribe(&mut self, sub_id: u64, topic: u64) {
        if let Some(s) = self.subscribers.get_mut(&sub_id) { s.subscribe(topic); }
        *self.topics.entry(topic).or_insert(0) += 1;
    }

    pub fn publish(&mut self, topic: u64, priority: EventBusPriority, data_hash: u64, now: u64) -> u64 {
        let eid = self.next_event_id; self.next_event_id += 1;
        self.events_published += 1;
        let sub_ids: Vec<u64> = self.subscribers.iter()
            .filter(|(_, s)| s.topics.contains(&topic))
            .map(|(&id, _)| id).collect();
        for sid in sub_ids {
            if let Some(s) = self.subscribers.get_mut(&sid) {
                if s.deliver() { self.total_delivered += 1; }
                else { self.total_dropped += 1; }
            }
        }
        eid
    }

    pub fn remove_subscriber(&mut self, id: u64) { self.subscribers.remove(&id); }

    pub fn stats(&self) -> EventBusV2Stats {
        EventBusV2Stats { total_subscribers: self.subscribers.len() as u32, total_events_published: self.events_published, total_delivered: self.total_delivered, total_dropped: self.total_dropped, active_topics: self.topics.len() as u32 }
    }
}

// ============================================================================
// Merged from event_bus_v3
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventV3Priority {
    Low,
    Normal,
    High,
    Urgent,
    Critical,
}

/// Event delivery mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventV3Delivery {
    Unicast,
    Multicast,
    Broadcast,
    Sticky,
}

/// An event on the bus
#[derive(Debug, Clone)]
pub struct EventV3Message {
    pub id: u64,
    pub topic_hash: u64,
    pub priority: EventV3Priority,
    pub delivery: EventV3Delivery,
    pub payload_size: usize,
    pub source_id: u64,
    pub timestamp_tick: u64,
    pub ttl: u32,
}

/// A topic subscription
#[derive(Debug, Clone)]
pub struct EventV3Subscriber {
    pub subscriber_id: u64,
    pub topic_hash: u64,
    pub filter_mask: u64,
    pub events_received: u64,
    pub active: bool,
}

/// An event topic
#[derive(Debug, Clone)]
pub struct EventV3Topic {
    pub topic_hash: u64,
    pub subscribers: Vec<EventV3Subscriber>,
    pub events_published: u64,
    pub sticky_event: Option<EventV3Message>,
}

impl EventV3Topic {
    pub fn new(hash: u64) -> Self {
        Self {
            topic_hash: hash,
            subscribers: Vec::new(),
            events_published: 0,
            sticky_event: None,
        }
    }

    pub fn subscribe(&mut self, sub_id: u64, filter: u64) {
        self.subscribers.push(EventV3Subscriber {
            subscriber_id: sub_id,
            topic_hash: self.topic_hash,
            filter_mask: filter,
            events_received: 0,
            active: true,
        });
    }

    pub fn unsubscribe(&mut self, sub_id: u64) {
        self.subscribers.retain(|s| s.subscriber_id != sub_id);
    }

    pub fn publish(&mut self, event: EventV3Message) -> u64 {
        self.events_published += 1;
        let mut delivered = 0u64;
        for sub in self.subscribers.iter_mut() {
            if sub.active {
                sub.events_received += 1;
                delivered += 1;
            }
        }
        if event.delivery == EventV3Delivery::Sticky {
            self.sticky_event = Some(event);
        }
        delivered
    }
}

/// Statistics for event bus V3
#[derive(Debug, Clone)]
pub struct EventBusV3Stats {
    pub topics_created: u64,
    pub subscriptions: u64,
    pub events_published: u64,
    pub events_delivered: u64,
    pub events_dropped: u64,
    pub sticky_events: u64,
}

/// Main event bus V3 coop manager
#[derive(Debug)]
pub struct CoopEventBusV3 {
    topics: BTreeMap<u64, EventV3Topic>,
    next_event_id: u64,
    stats: EventBusV3Stats,
}

impl CoopEventBusV3 {
    pub fn new() -> Self {
        Self {
            topics: BTreeMap::new(),
            next_event_id: 1,
            stats: EventBusV3Stats {
                topics_created: 0,
                subscriptions: 0,
                events_published: 0,
                events_delivered: 0,
                events_dropped: 0,
                sticky_events: 0,
            },
        }
    }

    fn hash_topic(name: &str) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for b in name.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    pub fn create_topic(&mut self, name: &str) -> u64 {
        let hash = Self::hash_topic(name);
        if !self.topics.contains_key(&hash) {
            self.topics.insert(hash, EventV3Topic::new(hash));
            self.stats.topics_created += 1;
        }
        hash
    }

    pub fn subscribe(&mut self, topic: &str, sub_id: u64, filter: u64) -> bool {
        let hash = Self::hash_topic(topic);
        if let Some(t) = self.topics.get_mut(&hash) {
            t.subscribe(sub_id, filter);
            self.stats.subscriptions += 1;
            true
        } else {
            false
        }
    }

    pub fn publish(
        &mut self,
        topic: &str,
        source: u64,
        priority: EventV3Priority,
        delivery: EventV3Delivery,
        payload_size: usize,
        tick: u64,
    ) -> u64 {
        let hash = Self::hash_topic(topic);
        let event_id = self.next_event_id;
        self.next_event_id += 1;
        let msg = EventV3Message {
            id: event_id,
            topic_hash: hash,
            priority,
            delivery,
            payload_size,
            source_id: source,
            timestamp_tick: tick,
            ttl: 10,
        };
        if delivery == EventV3Delivery::Sticky {
            self.stats.sticky_events += 1;
        }
        if let Some(t) = self.topics.get_mut(&hash) {
            let delivered = t.publish(msg);
            self.stats.events_published += 1;
            self.stats.events_delivered += delivered;
            delivered
        } else {
            0
        }
    }

    pub fn stats(&self) -> &EventBusV3Stats {
        &self.stats
    }
}
