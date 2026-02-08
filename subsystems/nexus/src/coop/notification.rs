//! # Cooperative Notification System
//!
//! Pub/sub notification system for cooperative scheduling:
//! - Topic-based publish/subscribe
//! - Priority-aware delivery
//! - Reliable delivery with acknowledgment
//! - Broadcast and multicast support
//! - Dead letter handling

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// NOTIFICATION TYPES
// ============================================================================

/// Notification priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationPriority {
    /// Low priority (batch)
    Low,
    /// Normal
    Normal,
    /// High
    High,
    /// Urgent
    Urgent,
    /// Critical
    Critical,
}

/// Delivery guarantee
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryGuarantee {
    /// At most once (fire and forget)
    AtMostOnce,
    /// At least once (with retry)
    AtLeastOnce,
    /// Exactly once (with dedup)
    ExactlyOnce,
}

/// Notification state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationState {
    /// Pending delivery
    Pending,
    /// Delivered
    Delivered,
    /// Acknowledged
    Acknowledged,
    /// Failed
    Failed,
    /// Dead letter (undeliverable)
    DeadLetter,
    /// Expired
    Expired,
}

// ============================================================================
// NOTIFICATION
// ============================================================================

/// A notification message
#[derive(Debug, Clone)]
pub struct Notification {
    /// Message id
    pub id: u64,
    /// Topic
    pub topic: String,
    /// Sender pid
    pub sender: u64,
    /// Priority
    pub priority: NotificationPriority,
    /// Delivery guarantee
    pub guarantee: DeliveryGuarantee,
    /// State
    pub state: NotificationState,
    /// Payload type code
    pub payload_type: u32,
    /// Payload size (bytes)
    pub payload_size: u32,
    /// Create time
    pub created_at: u64,
    /// Expiry time
    pub expires_at: u64,
    /// Delivery attempts
    pub attempts: u32,
    /// Max attempts
    pub max_attempts: u32,
    /// Content hash (for dedup)
    pub content_hash: u64,
}

impl Notification {
    pub fn new(
        id: u64,
        topic: String,
        sender: u64,
        priority: NotificationPriority,
        guarantee: DeliveryGuarantee,
        now: u64,
        ttl_ns: u64,
    ) -> Self {
        Self {
            id,
            topic,
            sender,
            priority,
            guarantee,
            state: NotificationState::Pending,
            payload_type: 0,
            payload_size: 0,
            created_at: now,
            expires_at: now + ttl_ns,
            attempts: 0,
            max_attempts: match guarantee {
                DeliveryGuarantee::AtMostOnce => 1,
                DeliveryGuarantee::AtLeastOnce => 3,
                DeliveryGuarantee::ExactlyOnce => 5,
            },
            content_hash: 0,
        }
    }

    /// Is expired?
    pub fn is_expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }

    /// Can retry?
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_attempts
            && self.guarantee != DeliveryGuarantee::AtMostOnce
    }

    /// Record delivery attempt
    pub fn attempt(&mut self) -> bool {
        self.attempts += 1;
        self.attempts <= self.max_attempts
    }
}

// ============================================================================
// SUBSCRIPTION
// ============================================================================

/// Subscription filter
#[derive(Debug, Clone)]
pub struct SubscriptionFilter {
    /// Minimum priority
    pub min_priority: NotificationPriority,
    /// Payload type filter (None = all)
    pub payload_type: Option<u32>,
}

/// Subscription
#[derive(Debug)]
pub struct Subscription {
    /// Subscription id
    pub id: u64,
    /// Subscriber pid
    pub subscriber: u64,
    /// Topic
    pub topic: String,
    /// Filter
    pub filter: SubscriptionFilter,
    /// Delivery guarantee
    pub guarantee: DeliveryGuarantee,
    /// Created at
    pub created_at: u64,
    /// Messages received
    pub received: u64,
    /// Messages acknowledged
    pub acknowledged: u64,
}

impl Subscription {
    pub fn new(
        id: u64,
        subscriber: u64,
        topic: String,
        guarantee: DeliveryGuarantee,
        now: u64,
    ) -> Self {
        Self {
            id,
            subscriber,
            topic,
            filter: SubscriptionFilter {
                min_priority: NotificationPriority::Low,
                payload_type: None,
            },
            guarantee,
            created_at: now,
            received: 0,
            acknowledged: 0,
        }
    }

    /// Matches notification?
    pub fn matches(&self, notif: &Notification) -> bool {
        if notif.topic != self.topic {
            return false;
        }
        if notif.priority < self.filter.min_priority {
            return false;
        }
        if let Some(pt) = self.filter.payload_type {
            if notif.payload_type != pt {
                return false;
            }
        }
        true
    }
}

// ============================================================================
// TOPIC
// ============================================================================

/// Topic info
#[derive(Debug)]
pub struct Topic {
    /// Name
    pub name: String,
    /// Subscriber ids
    pub subscribers: Vec<u64>,
    /// Total published
    pub total_published: u64,
    /// Total delivered
    pub total_delivered: u64,
    /// Dead letters
    pub dead_letters: u64,
}

impl Topic {
    pub fn new(name: String) -> Self {
        Self {
            name,
            subscribers: Vec::new(),
            total_published: 0,
            total_delivered: 0,
            dead_letters: 0,
        }
    }
}

// ============================================================================
// ENGINE
// ============================================================================

/// Notification stats
#[derive(Debug, Clone, Default)]
pub struct CoopNotificationStats {
    /// Total topics
    pub total_topics: usize,
    /// Total subscriptions
    pub total_subscriptions: usize,
    /// Total published
    pub total_published: u64,
    /// Total delivered
    pub total_delivered: u64,
    /// Total dead letters
    pub total_dead_letters: u64,
    /// Pending notifications
    pub pending_count: usize,
}

/// Cooperative notification manager
pub struct CoopNotificationManager {
    /// Topics: name hash -> Topic
    topics: BTreeMap<u64, Topic>,
    /// Subscriptions
    subscriptions: BTreeMap<u64, Subscription>,
    /// Pending notifications
    pending: Vec<Notification>,
    /// Dead letters
    dead_letters: Vec<Notification>,
    /// Dedup set (content hashes for exactly-once)
    dedup: BTreeMap<u64, u64>, // hash -> timestamp
    /// Next ids
    next_notif_id: u64,
    next_sub_id: u64,
    /// Max pending
    max_pending: usize,
    /// Max dead letters
    max_dead: usize,
    /// Stats
    stats: CoopNotificationStats,
}

impl CoopNotificationManager {
    pub fn new() -> Self {
        Self {
            topics: BTreeMap::new(),
            subscriptions: BTreeMap::new(),
            pending: Vec::new(),
            dead_letters: Vec::new(),
            dedup: BTreeMap::new(),
            next_notif_id: 1,
            next_sub_id: 1,
            max_pending: 10000,
            max_dead: 1000,
            stats: CoopNotificationStats::default(),
        }
    }

    fn topic_key(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in name.bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    /// Create topic
    pub fn create_topic(&mut self, name: String) -> u64 {
        let key = Self::topic_key(&name);
        self.topics.entry(key).or_insert_with(|| Topic::new(name));
        self.update_stats();
        key
    }

    /// Subscribe
    pub fn subscribe(
        &mut self,
        pid: u64,
        topic_name: String,
        guarantee: DeliveryGuarantee,
        now: u64,
    ) -> u64 {
        let topic_key = Self::topic_key(&topic_name);
        // Ensure topic exists
        self.topics.entry(topic_key).or_insert_with(|| Topic::new(topic_name.clone()));

        let sub_id = self.next_sub_id;
        self.next_sub_id += 1;

        let sub = Subscription::new(sub_id, pid, topic_name, guarantee, now);
        self.subscriptions.insert(sub_id, sub);

        if let Some(topic) = self.topics.get_mut(&topic_key) {
            topic.subscribers.push(sub_id);
        }

        self.update_stats();
        sub_id
    }

    /// Unsubscribe
    pub fn unsubscribe(&mut self, sub_id: u64) -> bool {
        if let Some(sub) = self.subscriptions.remove(&sub_id) {
            let topic_key = Self::topic_key(&sub.topic);
            if let Some(topic) = self.topics.get_mut(&topic_key) {
                topic.subscribers.retain(|&s| s != sub_id);
            }
            self.update_stats();
            true
        } else {
            false
        }
    }

    /// Publish notification
    pub fn publish(
        &mut self,
        topic_name: &str,
        sender: u64,
        priority: NotificationPriority,
        guarantee: DeliveryGuarantee,
        content_hash: u64,
        now: u64,
        ttl_ns: u64,
    ) -> u64 {
        // Dedup check for exactly-once
        if guarantee == DeliveryGuarantee::ExactlyOnce {
            if self.dedup.contains_key(&content_hash) {
                return 0; // duplicate
            }
            self.dedup.insert(content_hash, now);
        }

        let id = self.next_notif_id;
        self.next_notif_id += 1;

        let mut notif = Notification::new(
            id,
            String::from(topic_name),
            sender,
            priority,
            guarantee,
            now,
            ttl_ns,
        );
        notif.content_hash = content_hash;

        let topic_key = Self::topic_key(topic_name);
        if let Some(topic) = self.topics.get_mut(&topic_key) {
            topic.total_published += 1;
        }

        if self.pending.len() < self.max_pending {
            self.pending.push(notif);
        }

        self.stats.total_published += 1;
        self.update_stats();
        id
    }

    /// Process pending deliveries
    pub fn process_pending(&mut self, now: u64) -> Vec<(u64, u64)> {
        let mut deliveries = Vec::new(); // (subscriber_pid, notif_id)
        let mut to_dead = Vec::new();
        let mut delivered_indices = Vec::new();

        for (idx, notif) in self.pending.iter_mut().enumerate() {
            if notif.is_expired(now) {
                notif.state = NotificationState::Expired;
                to_dead.push(idx);
                continue;
            }

            let topic_key = Self::topic_key(&notif.topic);
            if let Some(topic) = self.topics.get(&topic_key) {
                let mut any_delivered = false;
                for &sub_id in &topic.subscribers {
                    if let Some(sub) = self.subscriptions.get(&sub_id) {
                        if sub.matches(notif) {
                            deliveries.push((sub.subscriber, notif.id));
                            any_delivered = true;
                        }
                    }
                }
                if any_delivered {
                    notif.state = NotificationState::Delivered;
                    delivered_indices.push(idx);
                } else if !notif.can_retry() {
                    to_dead.push(idx);
                }
            }
        }

        // Remove delivered (reverse order)
        delivered_indices.sort_unstable();
        for &idx in delivered_indices.iter().rev() {
            if idx < self.pending.len() {
                let notif = self.pending.remove(idx);
                self.stats.total_delivered += 1;
                let topic_key = Self::topic_key(&notif.topic);
                if let Some(topic) = self.topics.get_mut(&topic_key) {
                    topic.total_delivered += 1;
                }
            }
        }

        // Move to dead letter
        to_dead.sort_unstable();
        for &idx in to_dead.iter().rev() {
            if idx < self.pending.len() {
                let mut notif = self.pending.remove(idx);
                notif.state = NotificationState::DeadLetter;
                if self.dead_letters.len() < self.max_dead {
                    self.dead_letters.push(notif);
                }
                self.stats.total_dead_letters += 1;
            }
        }

        self.update_stats();
        deliveries
    }

    /// Cleanup old dedup entries
    pub fn cleanup_dedup(&mut self, now: u64, max_age_ns: u64) {
        let cutoff = now.saturating_sub(max_age_ns);
        self.dedup.retain(|_, &mut ts| ts >= cutoff);
    }

    fn update_stats(&mut self) {
        self.stats.total_topics = self.topics.len();
        self.stats.total_subscriptions = self.subscriptions.len();
        self.stats.pending_count = self.pending.len();
    }

    /// Stats
    pub fn stats(&self) -> &CoopNotificationStats {
        &self.stats
    }
}
