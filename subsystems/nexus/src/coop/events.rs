//! # Event-Driven Cooperation Notifications
//!
//! Pub/sub event system for cooperation:
//! - Event types for all cooperation activities
//! - Subscription management
//! - Event filtering and routing
//! - Event aggregation and batching
//! - Event history for analysis
//! - Cross-module event propagation

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// EVENT TYPES
// ============================================================================

/// Cooperation event category
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventCategory {
    /// Session lifecycle events
    Session,
    /// Hint events
    Hint,
    /// Advisory events
    Advisory,
    /// Contract events
    Contract,
    /// Trust events
    Trust,
    /// Reward events
    Reward,
    /// Channel events
    Channel,
    /// Registry events
    Registry,
    /// Compliance events
    Compliance,
    /// System events
    System,
}

/// Specific event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    // Session
    SessionCreated,
    SessionActivated,
    SessionSuspended,
    SessionResumed,
    SessionTerminated,

    // Hint
    HintReceived,
    HintProcessed,
    HintApplied,
    HintRejected,

    // Advisory
    AdvisoryEmitted,
    AdvisoryDelivered,
    AdvisoryAcknowledged,
    AdvisoryExpired,

    // Contract
    ContractNegotiated,
    ContractActivated,
    ContractRenewed,
    ContractViolated,
    ContractTerminated,

    // Trust
    TrustIncreased,
    TrustDecreased,
    TrustLevelChanged,

    // Reward
    RewardIssued,
    PenaltyIssued,
    LevelUp,
    LevelDown,

    // Channel
    ChannelOpened,
    ChannelClosed,
    ChannelBackpressure,

    // Registry
    CapabilityRegistered,
    CapabilityUnregistered,
    CapabilityAcquired,
    CapabilityReleased,

    // Compliance
    ComplianceWarning,
    ComplianceViolation,
    ComplianceCritical,
    ComplianceRestored,

    // System
    SystemPressure,
    SystemRecovery,
    SystemShutdown,
}

impl EventType {
    pub fn category(&self) -> EventCategory {
        match self {
            Self::SessionCreated
            | Self::SessionActivated
            | Self::SessionSuspended
            | Self::SessionResumed
            | Self::SessionTerminated => EventCategory::Session,

            Self::HintReceived
            | Self::HintProcessed
            | Self::HintApplied
            | Self::HintRejected => EventCategory::Hint,

            Self::AdvisoryEmitted
            | Self::AdvisoryDelivered
            | Self::AdvisoryAcknowledged
            | Self::AdvisoryExpired => EventCategory::Advisory,

            Self::ContractNegotiated
            | Self::ContractActivated
            | Self::ContractRenewed
            | Self::ContractViolated
            | Self::ContractTerminated => EventCategory::Contract,

            Self::TrustIncreased
            | Self::TrustDecreased
            | Self::TrustLevelChanged => EventCategory::Trust,

            Self::RewardIssued
            | Self::PenaltyIssued
            | Self::LevelUp
            | Self::LevelDown => EventCategory::Reward,

            Self::ChannelOpened
            | Self::ChannelClosed
            | Self::ChannelBackpressure => EventCategory::Channel,

            Self::CapabilityRegistered
            | Self::CapabilityUnregistered
            | Self::CapabilityAcquired
            | Self::CapabilityReleased => EventCategory::Registry,

            Self::ComplianceWarning
            | Self::ComplianceViolation
            | Self::ComplianceCritical
            | Self::ComplianceRestored => EventCategory::Compliance,

            Self::SystemPressure
            | Self::SystemRecovery
            | Self::SystemShutdown => EventCategory::System,
        }
    }
}

// ============================================================================
// EVENT DATA
// ============================================================================

/// An cooperation event
#[derive(Debug, Clone)]
pub struct CoopEvent {
    /// Event ID
    pub id: u64,
    /// Event type
    pub event_type: EventType,
    /// Source PID (0 = kernel)
    pub source_pid: u64,
    /// Target PID (0 = broadcast)
    pub target_pid: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Numeric parameter 1
    pub param1: u64,
    /// Numeric parameter 2
    pub param2: u64,
    /// Sequence number
    pub sequence: u64,
}

impl CoopEvent {
    pub fn new(
        id: u64,
        event_type: EventType,
        source_pid: u64,
        target_pid: u64,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            event_type,
            source_pid,
            target_pid,
            timestamp,
            param1: 0,
            param2: 0,
            sequence: 0,
        }
    }

    pub fn with_params(mut self, p1: u64, p2: u64) -> Self {
        self.param1 = p1;
        self.param2 = p2;
        self
    }
}

// ============================================================================
// SUBSCRIPTION
// ============================================================================

/// Subscription ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SubscriptionId(pub u64);

/// Event filter for subscription
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Categories to include (empty = all)
    pub categories: Vec<EventCategory>,
    /// Specific event types (empty = all in categories)
    pub event_types: Vec<EventType>,
    /// Source PID filter (None = any)
    pub source_pid: Option<u64>,
    /// Target PID filter (None = any)
    pub target_pid: Option<u64>,
}

impl EventFilter {
    /// Match everything
    pub fn all() -> Self {
        Self {
            categories: Vec::new(),
            event_types: Vec::new(),
            source_pid: None,
            target_pid: None,
        }
    }

    /// Match specific category
    pub fn category(cat: EventCategory) -> Self {
        let mut f = Self::all();
        f.categories.push(cat);
        f
    }

    /// Check if event matches filter
    pub fn matches(&self, event: &CoopEvent) -> bool {
        // Category filter
        if !self.categories.is_empty() {
            let cat = event.event_type.category();
            if !self.categories.contains(&cat) {
                return false;
            }
        }

        // Event type filter
        if !self.event_types.is_empty() && !self.event_types.contains(&event.event_type) {
            return false;
        }

        // Source PID filter
        if let Some(pid) = self.source_pid {
            if event.source_pid != pid {
                return false;
            }
        }

        // Target PID filter
        if let Some(pid) = self.target_pid {
            if event.target_pid != pid && event.target_pid != 0 {
                return false;
            }
        }

        true
    }
}

/// A subscription
struct Subscription {
    /// Subscription ID
    id: SubscriptionId,
    /// Subscriber PID
    subscriber_pid: u64,
    /// Filter
    filter: EventFilter,
    /// Events matched
    matched: u64,
    /// Active
    active: bool,
}

// ============================================================================
// EVENT HISTORY
// ============================================================================

/// Event history ring buffer
struct EventHistory {
    /// Events
    events: Vec<CoopEvent>,
    /// Capacity
    capacity: usize,
    /// Write position
    write_pos: usize,
    /// Total events recorded
    total: u64,
}

impl EventHistory {
    fn new(capacity: usize) -> Self {
        Self {
            events: Vec::with_capacity(capacity),
            capacity,
            write_pos: 0,
            total: 0,
        }
    }

    fn record(&mut self, event: CoopEvent) {
        if self.events.len() < self.capacity {
            self.events.push(event);
        } else {
            self.events[self.write_pos] = event;
        }
        self.write_pos = (self.write_pos + 1) % self.capacity;
        self.total += 1;
    }

    fn recent(&self, count: usize) -> Vec<&CoopEvent> {
        let len = self.events.len();
        if len == 0 {
            return Vec::new();
        }
        let count = count.min(len);
        let start = if len < self.capacity {
            len.saturating_sub(count)
        } else {
            (self.write_pos + self.capacity - count) % self.capacity
        };

        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let idx = (start + i) % len;
            result.push(&self.events[idx]);
        }
        result
    }

    fn count_by_type(&self, event_type: EventType) -> u64 {
        self.events
            .iter()
            .filter(|e| e.event_type == event_type)
            .count() as u64
    }
}

// ============================================================================
// AGGREGATED STATS
// ============================================================================

/// Event statistics per category
#[derive(Debug, Clone, Default)]
pub struct CategoryStats {
    /// Total events
    pub total: u64,
    /// Events in last minute
    pub last_minute: u64,
    /// Events in last hour
    pub last_hour: u64,
}

// ============================================================================
// EVENT BUS
// ============================================================================

/// Central event bus for cooperation events
pub struct CoopEventBus {
    /// Subscriptions
    subscriptions: BTreeMap<u64, Subscription>,
    /// PID → subscription IDs
    pid_subscriptions: BTreeMap<u64, Vec<SubscriptionId>>,
    /// Per-subscriber pending events
    pending: BTreeMap<u64, Vec<CoopEvent>>,
    /// Event history
    history: EventHistory,
    /// Per-category counters
    category_counts: BTreeMap<u8, u64>,
    /// Next subscription ID
    next_sub_id: u64,
    /// Next event ID
    next_event_id: u64,
    /// Global sequence number
    sequence: u64,
    /// Max pending per subscriber
    max_pending: usize,
    /// Total events published
    pub total_published: u64,
    /// Total events delivered
    pub total_delivered: u64,
    /// Total events dropped
    pub total_dropped: u64,
}

impl CoopEventBus {
    pub fn new(history_capacity: usize, max_pending: usize) -> Self {
        Self {
            subscriptions: BTreeMap::new(),
            pid_subscriptions: BTreeMap::new(),
            pending: BTreeMap::new(),
            history: EventHistory::new(history_capacity),
            category_counts: BTreeMap::new(),
            next_sub_id: 1,
            next_event_id: 1,
            sequence: 0,
            max_pending,
            total_published: 0,
            total_delivered: 0,
            total_dropped: 0,
        }
    }

    /// Subscribe to events
    pub fn subscribe(
        &mut self,
        pid: u64,
        filter: EventFilter,
    ) -> SubscriptionId {
        let id = SubscriptionId(self.next_sub_id);
        self.next_sub_id += 1;

        let sub = Subscription {
            id,
            subscriber_pid: pid,
            filter,
            matched: 0,
            active: true,
        };

        self.subscriptions.insert(id.0, sub);
        self.pid_subscriptions
            .entry(pid)
            .or_insert_with(Vec::new)
            .push(id);
        self.pending.entry(pid).or_insert_with(Vec::new);

        id
    }

    /// Unsubscribe
    pub fn unsubscribe(&mut self, id: SubscriptionId) {
        if let Some(sub) = self.subscriptions.remove(&id.0) {
            if let Some(pids) = self.pid_subscriptions.get_mut(&sub.subscriber_pid) {
                pids.retain(|&sid| sid != id);
            }
        }
    }

    /// Unsubscribe all for PID
    pub fn unsubscribe_all(&mut self, pid: u64) {
        if let Some(sub_ids) = self.pid_subscriptions.remove(&pid) {
            for id in sub_ids {
                self.subscriptions.remove(&id.0);
            }
        }
        self.pending.remove(&pid);
    }

    /// Publish an event
    pub fn publish(
        &mut self,
        event_type: EventType,
        source_pid: u64,
        target_pid: u64,
        timestamp: u64,
    ) -> u64 {
        let event_id = self.next_event_id;
        self.next_event_id += 1;
        self.sequence += 1;

        let mut event = CoopEvent::new(event_id, event_type, source_pid, target_pid, timestamp);
        event.sequence = self.sequence;

        // Update category counter
        let cat_key = event_type.category() as u8;
        *self.category_counts.entry(cat_key).or_insert(0) += 1;

        // Record in history
        self.history.record(event.clone());

        // Route to matching subscribers
        let sub_ids: Vec<(u64, u64)> = self
            .subscriptions
            .iter()
            .filter(|(_, sub)| sub.active && sub.filter.matches(&event))
            .map(|(&id, sub)| (id, sub.subscriber_pid))
            .collect();

        for (sub_id, pid) in sub_ids {
            if let Some(queue) = self.pending.get_mut(&pid) {
                if queue.len() < self.max_pending {
                    queue.push(event.clone());
                    if let Some(sub) = self.subscriptions.get_mut(&sub_id) {
                        sub.matched += 1;
                    }
                } else {
                    self.total_dropped += 1;
                }
            }
        }

        self.total_published += 1;
        event_id
    }

    /// Publish with parameters
    pub fn publish_with_params(
        &mut self,
        event_type: EventType,
        source_pid: u64,
        target_pid: u64,
        timestamp: u64,
        param1: u64,
        param2: u64,
    ) -> u64 {
        let id = self.publish(event_type, source_pid, target_pid, timestamp);

        // Update params in history (last recorded)
        if let Some(recent) = self.history.events.last_mut() {
            if recent.id == id {
                recent.param1 = param1;
                recent.param2 = param2;
            }
        }

        id
    }

    /// Poll events for a subscriber
    pub fn poll(&mut self, pid: u64, max_events: usize) -> Vec<CoopEvent> {
        let queue = match self.pending.get_mut(&pid) {
            Some(q) => q,
            None => return Vec::new(),
        };

        let count = max_events.min(queue.len());
        let events: Vec<CoopEvent> = queue.drain(..count).collect();
        self.total_delivered += events.len() as u64;
        events
    }

    /// Pending events for a subscriber
    pub fn pending_count(&self, pid: u64) -> usize {
        self.pending.get(&pid).map_or(0, |q| q.len())
    }

    /// Get recent events from history
    pub fn recent_events(&self, count: usize) -> Vec<&CoopEvent> {
        self.history.recent(count)
    }

    /// Count events of type in history
    pub fn count_event_type(&self, event_type: EventType) -> u64 {
        self.history.count_by_type(event_type)
    }

    /// Category event count
    pub fn category_count(&self, category: EventCategory) -> u64 {
        self.category_counts
            .get(&(category as u8))
            .copied()
            .unwrap_or(0)
    }

    /// Subscription count
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Total events in history
    pub fn history_total(&self) -> u64 {
        self.history.total
    }
}
