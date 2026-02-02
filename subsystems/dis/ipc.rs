//! # DIS IPC - Inter-Process Communication
//!
//! This module provides the IPC infrastructure for the Dynamic Intent Scheduling
//! system. It enables communication between tasks, the scheduler, and external
//! components.
//!
//! ## IPC Types
//!
//! - **Messages**: Asynchronous message passing
//! - **Channels**: Bidirectional communication channels
//! - **Notifications**: One-way event notifications
//! - **Shared Memory**: Zero-copy data sharing
//! - **RPC**: Request-response pattern
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │                            DIS IPC SYSTEM                                    │
//! │                                                                              │
//! │  ┌─────────────────────────────────────────────────────────────────────────┐ │
//! │  │                         MESSAGE ROUTER                                  │ │
//! │  │                                                                         │ │
//! │  │  ┌─────────────┐    ┌─────────────┐    ┌─────────────────────────────┐  │ │
//! │  │  │   SENDER    │ -> │   ROUTER    │ -> │   RECEIVER                  │  │ │
//! │  │  │   Task A    │    │             │    │   Task B                    │  │ │
//! │  │  └─────────────┘    └─────────────┘    └─────────────────────────────┘  │ │
//! │  └─────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                              │
//! │  ┌─────────────────────────────────────────────────────────────────────────┐ │
//! │  │                         CHANNEL MANAGER                                 │ │
//! │  │                                                                         │ │
//! │  │   Channel 1: Task A <=======> Task B                                    │ │
//! │  │   Channel 2: Task C <=======> Task D                                    │ │
//! │  │   Channel 3: Scheduler <====> Monitor                                   │ │
//! │  └─────────────────────────────────────────────────────────────────────────┘ │
//! │                                                                              │
//! │  ┌─────────────────────────────────────────────────────────────────────────┐ │
//! │  │                       NOTIFICATION HUB                                  │ │
//! │  │                                                                         │ │
//! │  │   ┌──────────────┐        ┌───────────────────────────────────────────┐ │ │
//! │  │   │  Publisher   │ -----> │  Subscriber 1, Subscriber 2, ...          │ │ │
//! │  │   └──────────────┘        └───────────────────────────────────────────┘ │ │
//! │  └─────────────────────────────────────────────────────────────────────────┘ │
//! └──────────────────────────────────────────────────────────────────────────────┘
//! ```

use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use spin::{Mutex, RwLock};

use super::intent::IntentClass;
use super::{DISError, DISResult, Nanoseconds, TaskId};

// =============================================================================
// Message Types
// =============================================================================

/// Unique message identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(u64);

impl MessageId {
    /// Create new message ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw ID
    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// Channel identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChannelId(u64);

impl ChannelId {
    /// Create new channel ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    /// Get raw ID
    pub fn raw(&self) -> u64 {
        self.0
    }
}

/// IPC message
#[derive(Debug, Clone)]
pub struct Message {
    /// Message ID
    pub id: MessageId,
    /// Sender task
    pub sender: TaskId,
    /// Recipient task (None for broadcast)
    pub recipient: Option<TaskId>,
    /// Message type
    pub msg_type: MessageType,
    /// Message payload
    pub payload: MessagePayload,
    /// Priority
    pub priority: MessagePriority,
    /// Flags
    pub flags: MessageFlags,
    /// Timestamp
    pub timestamp: Nanoseconds,
    /// Expiry (optional)
    pub expiry: Option<Nanoseconds>,
}

/// Message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Data message
    Data,
    /// Request (expects response)
    Request,
    /// Response to request
    Response,
    /// Notification (one-way)
    Notification,
    /// Signal
    Signal,
    /// Control message
    Control,
    /// Error message
    Error,
}

/// Message payload
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// Empty message
    Empty,
    /// Raw bytes
    Bytes(Vec<u8>),
    /// Integer value
    Integer(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Task-related data
    TaskInfo(TaskInfoPayload),
    /// Scheduler-related data
    SchedulerInfo(SchedulerInfoPayload),
    /// Policy-related data
    PolicyInfo(PolicyInfoPayload),
    /// Intent-related data
    IntentInfo(IntentInfoPayload),
    /// Statistics data
    StatsInfo(StatsInfoPayload),
    /// Custom data
    Custom(CustomPayload),
}

/// Task information payload
#[derive(Debug, Clone)]
pub struct TaskInfoPayload {
    pub task_id: TaskId,
    pub name: String,
    pub state: u8,
    pub priority: i8,
    pub cpu_time: u64,
}

/// Scheduler information payload
#[derive(Debug, Clone)]
pub struct SchedulerInfoPayload {
    pub schedules: u64,
    pub context_switches: u64,
    pub preemptions: u64,
    pub cpu_load: u8,
    pub runnable_tasks: u32,
}

/// Policy information payload
#[derive(Debug, Clone)]
pub struct PolicyInfoPayload {
    pub policy_id: u64,
    pub name: String,
    pub active: bool,
    pub applications: u64,
}

/// Intent information payload
#[derive(Debug, Clone)]
pub struct IntentInfoPayload {
    pub intent_id: u64,
    pub class: IntentClass,
    pub priority: i8,
}

/// Statistics information payload
#[derive(Debug, Clone)]
pub struct StatsInfoPayload {
    pub cpu_time: u64,
    pub wait_time: u64,
    pub context_switches: u64,
    pub preemptions: u64,
}

/// Custom payload
#[derive(Debug, Clone)]
pub struct CustomPayload {
    pub type_id: u32,
    pub data: Vec<u8>,
}

/// Message priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MessagePriority {
    /// Low priority
    Low    = 0,
    /// Normal priority
    Normal = 1,
    /// High priority
    High   = 2,
    /// Urgent (bypasses queue)
    Urgent = 3,
}

bitflags::bitflags! {
    /// Message flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MessageFlags: u32 {
        /// Requires acknowledgment
        const ACK_REQUIRED = 1 << 0;
        /// Is acknowledgment
        const IS_ACK = 1 << 1;
        /// Is response
        const IS_RESPONSE = 1 << 2;
        /// Is broadcast
        const BROADCAST = 1 << 3;
        /// Is multicast
        const MULTICAST = 1 << 4;
        /// Encrypted
        const ENCRYPTED = 1 << 5;
        /// Signed
        const SIGNED = 1 << 6;
        /// No reply expected
        const NO_REPLY = 1 << 7;
        /// Urgent delivery
        const URGENT = 1 << 8;
        /// Reliable delivery
        const RELIABLE = 1 << 9;
    }
}

impl Message {
    /// Create new message
    pub fn new(
        sender: TaskId,
        recipient: Option<TaskId>,
        msg_type: MessageType,
        payload: MessagePayload,
    ) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: MessageId::new(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
            sender,
            recipient,
            msg_type,
            payload,
            priority: MessagePriority::Normal,
            flags: MessageFlags::empty(),
            timestamp: Nanoseconds::zero(),
            expiry: None,
        }
    }

    /// Create data message
    pub fn data(sender: TaskId, recipient: TaskId, data: Vec<u8>) -> Self {
        Self::new(
            sender,
            Some(recipient),
            MessageType::Data,
            MessagePayload::Bytes(data),
        )
    }

    /// Create request message
    pub fn request(sender: TaskId, recipient: TaskId, payload: MessagePayload) -> Self {
        Self::new(sender, Some(recipient), MessageType::Request, payload)
    }

    /// Create notification
    pub fn notification(sender: TaskId, payload: MessagePayload) -> Self {
        Self::new(sender, None, MessageType::Notification, payload)
    }

    /// Create signal
    pub fn signal(sender: TaskId, recipient: TaskId, signal: i64) -> Self {
        Self::new(
            sender,
            Some(recipient),
            MessageType::Signal,
            MessagePayload::Integer(signal),
        )
    }

    /// Set priority
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set flags
    pub fn with_flags(mut self, flags: MessageFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set expiry
    pub fn with_expiry(mut self, expiry: Nanoseconds) -> Self {
        self.expiry = Some(expiry);
        self
    }

    /// Check if expired
    pub fn is_expired(&self, now: Nanoseconds) -> bool {
        self.expiry.map_or(false, |exp| now > exp)
    }
}

// =============================================================================
// Message Queue
// =============================================================================

/// Message queue for a task
pub struct MessageQueue {
    /// Task ID
    task_id: TaskId,
    /// Queue
    queue: VecDeque<Message>,
    /// Maximum size
    max_size: usize,
    /// Pending count
    pending: AtomicU32,
    /// Total received
    received: AtomicU64,
    /// Total dropped
    dropped: AtomicU64,
}

impl MessageQueue {
    /// Create new message queue
    pub fn new(task_id: TaskId, max_size: usize) -> Self {
        Self {
            task_id,
            queue: VecDeque::with_capacity(max_size),
            max_size,
            pending: AtomicU32::new(0),
            received: AtomicU64::new(0),
            dropped: AtomicU64::new(0),
        }
    }

    /// Enqueue message
    pub fn enqueue(&mut self, msg: Message) -> DISResult<()> {
        if self.queue.len() >= self.max_size {
            // Drop oldest if full
            self.queue.pop_front();
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }

        self.queue.push_back(msg);
        self.pending.fetch_add(1, Ordering::Relaxed);
        self.received.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Dequeue message
    pub fn dequeue(&mut self) -> Option<Message> {
        let msg = self.queue.pop_front();
        if msg.is_some() {
            self.pending.fetch_sub(1, Ordering::Relaxed);
        }
        msg
    }

    /// Peek at next message
    pub fn peek(&self) -> Option<&Message> {
        self.queue.front()
    }

    /// Get pending count
    pub fn pending(&self) -> u32 {
        self.pending.load(Ordering::Relaxed)
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.pending() == 0
    }

    /// Clear queue
    pub fn clear(&mut self) {
        self.queue.clear();
        self.pending.store(0, Ordering::Relaxed);
    }
}

// =============================================================================
// Channel
// =============================================================================

/// Bidirectional communication channel
pub struct Channel {
    /// Channel ID
    pub id: ChannelId,
    /// Endpoint A
    pub endpoint_a: TaskId,
    /// Endpoint B
    pub endpoint_b: TaskId,
    /// Messages from A to B
    a_to_b: Mutex<VecDeque<Message>>,
    /// Messages from B to A
    b_to_a: Mutex<VecDeque<Message>>,
    /// Channel state
    state: AtomicU32,
    /// Created timestamp
    created: Nanoseconds,
    /// Statistics
    stats: ChannelStats,
}

/// Channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ChannelState {
    /// Channel is open
    Open    = 0,
    /// Channel is closing
    Closing = 1,
    /// Channel is closed
    Closed  = 2,
}

/// Channel statistics
#[derive(Debug, Default)]
struct ChannelStats {
    messages_a_to_b: AtomicU64,
    messages_b_to_a: AtomicU64,
    bytes_a_to_b: AtomicU64,
    bytes_b_to_a: AtomicU64,
}

impl Channel {
    /// Create new channel
    pub fn new(id: ChannelId, endpoint_a: TaskId, endpoint_b: TaskId) -> Self {
        Self {
            id,
            endpoint_a,
            endpoint_b,
            a_to_b: Mutex::new(VecDeque::new()),
            b_to_a: Mutex::new(VecDeque::new()),
            state: AtomicU32::new(ChannelState::Open as u32),
            created: Nanoseconds::zero(),
            stats: ChannelStats::default(),
        }
    }

    /// Send message
    pub fn send(&self, from: TaskId, msg: Message) -> DISResult<()> {
        if self.state.load(Ordering::Relaxed) != ChannelState::Open as u32 {
            return Err(DISError::ChannelClosed);
        }

        if from == self.endpoint_a {
            self.a_to_b.lock().push_back(msg);
            self.stats.messages_a_to_b.fetch_add(1, Ordering::Relaxed);
        } else if from == self.endpoint_b {
            self.b_to_a.lock().push_back(msg);
            self.stats.messages_b_to_a.fetch_add(1, Ordering::Relaxed);
        } else {
            return Err(DISError::NotChannelEndpoint);
        }

        Ok(())
    }

    /// Receive message
    pub fn receive(&self, to: TaskId) -> Option<Message> {
        if to == self.endpoint_a {
            self.b_to_a.lock().pop_front()
        } else if to == self.endpoint_b {
            self.a_to_b.lock().pop_front()
        } else {
            None
        }
    }

    /// Check for pending messages
    pub fn has_pending(&self, to: TaskId) -> bool {
        if to == self.endpoint_a {
            !self.b_to_a.lock().is_empty()
        } else if to == self.endpoint_b {
            !self.a_to_b.lock().is_empty()
        } else {
            false
        }
    }

    /// Close channel
    pub fn close(&self) {
        self.state
            .store(ChannelState::Closed as u32, Ordering::SeqCst);
    }

    /// Check if closed
    pub fn is_closed(&self) -> bool {
        self.state.load(Ordering::Relaxed) == ChannelState::Closed as u32
    }
}

// =============================================================================
// Notification
// =============================================================================

/// Notification topic
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TopicId(u64);

impl TopicId {
    /// Create new topic ID
    pub const fn new(id: u64) -> Self {
        Self(id)
    }

    // Predefined topics
    pub const SCHEDULER: Self = Self(1);
    pub const TASK_EVENTS: Self = Self(2);
    pub const POLICY_EVENTS: Self = Self(3);
    pub const STATS: Self = Self(4);
    pub const SECURITY: Self = Self(5);
}

/// Notification
#[derive(Debug, Clone)]
pub struct Notification {
    /// Topic
    pub topic: TopicId,
    /// Event type
    pub event: NotificationEvent,
    /// Payload
    pub payload: MessagePayload,
    /// Timestamp
    pub timestamp: Nanoseconds,
}

/// Notification events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationEvent {
    // Task events
    TaskCreated,
    TaskDestroyed,
    TaskStateChanged,
    TaskMigrated,

    // Scheduler events
    SchedulerTick,
    ContextSwitch,
    LoadBalance,

    // Policy events
    PolicyAdded,
    PolicyRemoved,
    PolicyTriggered,

    // Stats events
    StatsUpdate,
    ThresholdExceeded,

    // Security events
    CapabilityGranted,
    CapabilityRevoked,
    SecurityViolation,

    // Custom
    Custom(u32),
}

/// Notification subscriber
pub struct Subscriber {
    /// Task ID
    pub task_id: TaskId,
    /// Subscribed topics
    pub topics: Vec<TopicId>,
    /// Notification queue
    pub queue: Mutex<VecDeque<Notification>>,
    /// Filter (optional)
    pub filter: Option<fn(&Notification) -> bool>,
}

impl Subscriber {
    /// Create new subscriber
    pub fn new(task_id: TaskId) -> Self {
        Self {
            task_id,
            topics: Vec::new(),
            queue: Mutex::new(VecDeque::new()),
            filter: None,
        }
    }

    /// Subscribe to topic
    pub fn subscribe(&mut self, topic: TopicId) {
        if !self.topics.contains(&topic) {
            self.topics.push(topic);
        }
    }

    /// Unsubscribe from topic
    pub fn unsubscribe(&mut self, topic: TopicId) {
        self.topics.retain(|&t| t != topic);
    }

    /// Check if subscribed
    pub fn is_subscribed(&self, topic: TopicId) -> bool {
        self.topics.contains(&topic)
    }

    /// Receive notification
    pub fn receive(&self, notification: Notification) {
        if let Some(filter) = self.filter {
            if !filter(&notification) {
                return;
            }
        }
        self.queue.lock().push_back(notification);
    }

    /// Get next notification
    pub fn next(&self) -> Option<Notification> {
        self.queue.lock().pop_front()
    }
}

// =============================================================================
// IPC Manager
// =============================================================================

/// The main IPC manager
pub struct IPCManager {
    /// Message queues per task
    queues: RwLock<BTreeMap<TaskId, Mutex<MessageQueue>>>,
    /// Channels
    channels: RwLock<BTreeMap<ChannelId, Channel>>,
    /// Subscribers
    subscribers: RwLock<BTreeMap<TaskId, Subscriber>>,
    /// Next channel ID
    next_channel_id: AtomicU64,
    /// Statistics
    stats: IPCStats,
    /// Current time
    current_time: AtomicU64,
}

/// IPC statistics
#[derive(Debug, Default)]
struct IPCStats {
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
    messages_dropped: AtomicU64,
    channels_created: AtomicU64,
    channels_closed: AtomicU64,
    notifications_sent: AtomicU64,
}

impl IPCManager {
    /// Create new IPC manager
    pub fn new() -> Self {
        Self {
            queues: RwLock::new(BTreeMap::new()),
            channels: RwLock::new(BTreeMap::new()),
            subscribers: RwLock::new(BTreeMap::new()),
            next_channel_id: AtomicU64::new(1),
            stats: IPCStats::default(),
            current_time: AtomicU64::new(0),
        }
    }

    /// Register task
    pub fn register_task(&self, task_id: TaskId) {
        self.queues
            .write()
            .insert(task_id, Mutex::new(MessageQueue::new(task_id, 256)));
        self.subscribers
            .write()
            .insert(task_id, Subscriber::new(task_id));
    }

    /// Unregister task
    pub fn unregister_task(&self, task_id: TaskId) {
        self.queues.write().remove(&task_id);
        self.subscribers.write().remove(&task_id);

        // Close channels with this task
        let channels_to_close: Vec<_> = self
            .channels
            .read()
            .iter()
            .filter(|(_, ch)| ch.endpoint_a == task_id || ch.endpoint_b == task_id)
            .map(|(id, _)| *id)
            .collect();

        for id in channels_to_close {
            self.close_channel(id);
        }
    }

    // =========================================================================
    // Message Operations
    // =========================================================================

    /// Send message
    pub fn send(&self, msg: Message) -> DISResult<()> {
        let recipient = msg.recipient.ok_or(DISError::NoRecipient)?;

        let queues = self.queues.read();
        if let Some(queue) = queues.get(&recipient) {
            queue.lock().enqueue(msg)?;
            self.stats.messages_sent.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(DISError::TaskNotFound(recipient))
        }
    }

    /// Receive message
    pub fn receive(&self, task_id: TaskId) -> Option<Message> {
        let queues = self.queues.read();
        if let Some(queue) = queues.get(&task_id) {
            let msg = queue.lock().dequeue();
            if msg.is_some() {
                self.stats.messages_received.fetch_add(1, Ordering::Relaxed);
            }
            msg
        } else {
            None
        }
    }

    /// Check for pending messages
    pub fn has_messages(&self, task_id: TaskId) -> bool {
        self.queues
            .read()
            .get(&task_id)
            .map(|q| !q.lock().is_empty())
            .unwrap_or(false)
    }

    /// Broadcast message
    pub fn broadcast(&self, msg: Message) -> usize {
        let mut count = 0;

        for (_, queue) in self.queues.read().iter() {
            if queue.lock().enqueue(msg.clone()).is_ok() {
                count += 1;
            }
        }

        self.stats
            .messages_sent
            .fetch_add(count as u64, Ordering::Relaxed);
        count
    }

    // =========================================================================
    // Channel Operations
    // =========================================================================

    /// Create channel
    pub fn create_channel(&self, endpoint_a: TaskId, endpoint_b: TaskId) -> ChannelId {
        let id = ChannelId::new(self.next_channel_id.fetch_add(1, Ordering::Relaxed));
        let channel = Channel::new(id, endpoint_a, endpoint_b);

        self.channels.write().insert(id, channel);
        self.stats.channels_created.fetch_add(1, Ordering::Relaxed);

        id
    }

    /// Close channel
    pub fn close_channel(&self, id: ChannelId) -> bool {
        if let Some(channel) = self.channels.write().remove(&id) {
            channel.close();
            self.stats.channels_closed.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Send on channel
    pub fn channel_send(&self, channel_id: ChannelId, from: TaskId, msg: Message) -> DISResult<()> {
        let channels = self.channels.read();
        if let Some(channel) = channels.get(&channel_id) {
            channel.send(from, msg)
        } else {
            Err(DISError::ChannelNotFound)
        }
    }

    /// Receive from channel
    pub fn channel_receive(&self, channel_id: ChannelId, to: TaskId) -> Option<Message> {
        self.channels
            .read()
            .get(&channel_id)
            .and_then(|ch| ch.receive(to))
    }

    // =========================================================================
    // Notification Operations
    // =========================================================================

    /// Subscribe to topic
    pub fn subscribe(&self, task_id: TaskId, topic: TopicId) {
        if let Some(sub) = self.subscribers.write().get_mut(&task_id) {
            sub.subscribe(topic);
        }
    }

    /// Unsubscribe from topic
    pub fn unsubscribe(&self, task_id: TaskId, topic: TopicId) {
        if let Some(sub) = self.subscribers.write().get_mut(&task_id) {
            sub.unsubscribe(topic);
        }
    }

    /// Publish notification
    pub fn publish(&self, notification: Notification) -> usize {
        let mut count = 0;

        for sub in self.subscribers.read().values() {
            if sub.is_subscribed(notification.topic) {
                sub.receive(notification.clone());
                count += 1;
            }
        }

        self.stats
            .notifications_sent
            .fetch_add(count as u64, Ordering::Relaxed);
        count
    }

    /// Get notification
    pub fn get_notification(&self, task_id: TaskId) -> Option<Notification> {
        self.subscribers
            .read()
            .get(&task_id)
            .and_then(|sub| sub.next())
    }

    // =========================================================================
    // Statistics
    // =========================================================================

    /// Get IPC statistics
    pub fn statistics(&self) -> IPCStatistics {
        IPCStatistics {
            messages_sent: self.stats.messages_sent.load(Ordering::Relaxed),
            messages_received: self.stats.messages_received.load(Ordering::Relaxed),
            messages_dropped: self.stats.messages_dropped.load(Ordering::Relaxed),
            channels_open: self.channels.read().len() as u64,
            channels_created: self.stats.channels_created.load(Ordering::Relaxed),
            channels_closed: self.stats.channels_closed.load(Ordering::Relaxed),
            notifications_sent: self.stats.notifications_sent.load(Ordering::Relaxed),
            subscribers: self.subscribers.read().len() as u64,
        }
    }
}

impl Default for IPCManager {
    fn default() -> Self {
        Self::new()
    }
}

/// IPC statistics
#[derive(Debug, Clone)]
pub struct IPCStatistics {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub messages_dropped: u64,
    pub channels_open: u64,
    pub channels_created: u64,
    pub channels_closed: u64,
    pub notifications_sent: u64,
    pub subscribers: u64,
}

// =============================================================================
// Request-Response Pattern
// =============================================================================

/// RPC request
#[derive(Debug, Clone)]
pub struct Request {
    /// Request ID
    pub id: MessageId,
    /// Method name
    pub method: String,
    /// Parameters
    pub params: MessagePayload,
    /// Timeout
    pub timeout: Option<Nanoseconds>,
}

/// RPC response
#[derive(Debug, Clone)]
pub struct Response {
    /// Request ID this responds to
    pub request_id: MessageId,
    /// Success flag
    pub success: bool,
    /// Result payload
    pub result: MessagePayload,
    /// Error (if any)
    pub error: Option<String>,
}

impl Request {
    /// Create new request
    pub fn new(method: &str, params: MessagePayload) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: MessageId::new(NEXT_ID.fetch_add(1, Ordering::Relaxed)),
            method: method.to_string(),
            params,
            timeout: None,
        }
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Nanoseconds) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl Response {
    /// Create success response
    pub fn success(request_id: MessageId, result: MessagePayload) -> Self {
        Self {
            request_id,
            success: true,
            result,
            error: None,
        }
    }

    /// Create error response
    pub fn error(request_id: MessageId, error: &str) -> Self {
        Self {
            request_id,
            success: false,
            result: MessagePayload::Empty,
            error: Some(error.to_string()),
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::data(TaskId::new(1), TaskId::new(2), vec![1, 2, 3]);
        assert_eq!(msg.sender, TaskId::new(1));
        assert_eq!(msg.recipient, Some(TaskId::new(2)));
    }

    #[test]
    fn test_message_queue() {
        let mut queue = MessageQueue::new(TaskId::new(1), 10);

        let msg = Message::data(TaskId::new(2), TaskId::new(1), vec![1, 2, 3]);
        queue.enqueue(msg).unwrap();

        assert_eq!(queue.pending(), 1);

        let received = queue.dequeue().unwrap();
        assert_eq!(received.sender, TaskId::new(2));

        assert!(queue.is_empty());
    }

    #[test]
    fn test_channel() {
        let channel = Channel::new(ChannelId::new(1), TaskId::new(1), TaskId::new(2));

        let msg = Message::data(TaskId::new(1), TaskId::new(2), vec![1, 2, 3]);
        channel.send(TaskId::new(1), msg).unwrap();

        assert!(channel.has_pending(TaskId::new(2)));

        let received = channel.receive(TaskId::new(2)).unwrap();
        assert_eq!(received.sender, TaskId::new(1));
    }

    #[test]
    fn test_ipc_manager() {
        let manager = IPCManager::new();

        manager.register_task(TaskId::new(1));
        manager.register_task(TaskId::new(2));

        let msg = Message::data(TaskId::new(1), TaskId::new(2), vec![1, 2, 3]);
        manager.send(msg).unwrap();

        assert!(manager.has_messages(TaskId::new(2)));

        let received = manager.receive(TaskId::new(2)).unwrap();
        assert_eq!(received.sender, TaskId::new(1));
    }

    #[test]
    fn test_notifications() {
        let manager = IPCManager::new();

        manager.register_task(TaskId::new(1));
        manager.subscribe(TaskId::new(1), TopicId::SCHEDULER);

        let notification = Notification {
            topic: TopicId::SCHEDULER,
            event: NotificationEvent::SchedulerTick,
            payload: MessagePayload::Empty,
            timestamp: Nanoseconds::zero(),
        };

        let count = manager.publish(notification);
        assert_eq!(count, 1);

        let received = manager.get_notification(TaskId::new(1)).unwrap();
        assert_eq!(received.event, NotificationEvent::SchedulerTick);
    }
}
