//! # Fast IPC Channels for Cooperation Protocol
//!
//! Lock-free, shared-memory inspired channels:
//! - Ring-buffer based message passing
//! - Priority message lanes
//! - Flow control with backpressure
//! - Channel multiplexing
//! - Zero-copy message path (when possible)
//! - Statistics and monitoring

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

// ============================================================================
// CHANNEL TYPES
// ============================================================================

/// Channel direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDirection {
    /// App → Kernel
    AppToKernel,
    /// Kernel → App
    KernelToApp,
    /// Bidirectional
    Bidirectional,
}

/// Channel priority lane
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChannelPriority {
    /// Background/bulk
    Background,
    /// Normal priority
    Normal,
    /// High priority (advisories)
    High,
    /// Critical (must deliver)
    Critical,
}

/// Channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    /// Being set up
    Creating,
    /// Open and active
    Open,
    /// Paused (flow control)
    Paused,
    /// Draining (closing, delivering remaining)
    Draining,
    /// Closed
    Closed,
    /// Error state
    Error,
}

/// A channel message
#[derive(Debug, Clone)]
pub struct ChannelMessage {
    /// Message sequence number
    pub sequence: u64,
    /// Priority
    pub priority: ChannelPriority,
    /// Message type tag
    pub msg_type: u32,
    /// Payload (up to 256 bytes inline)
    pub payload: MessagePayload,
    /// Timestamp
    pub timestamp: u64,
    /// Flags
    pub flags: u32,
}

/// Message payload
#[derive(Debug, Clone)]
pub enum MessagePayload {
    /// Empty (signal only)
    Empty,
    /// Small inline payload
    Inline(InlinePayload),
    /// Large payload reference
    Reference(PayloadRef),
}

/// Inline payload (fits in channel without extra allocation)
#[derive(Debug, Clone)]
pub struct InlinePayload {
    /// Data bytes
    pub data: [u8; 128],
    /// Actual length
    pub len: u8,
}

impl InlinePayload {
    pub fn new() -> Self {
        Self {
            data: [0u8; 128],
            len: 0,
        }
    }

    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut p = Self::new();
        let copy_len = bytes.len().min(128);
        p.data[..copy_len].copy_from_slice(&bytes[..copy_len]);
        p.len = copy_len as u8;
        p
    }

    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

/// Reference to a larger payload in shared memory
#[derive(Debug, Clone, Copy)]
pub struct PayloadRef {
    /// Shared memory region ID
    pub region_id: u64,
    /// Offset within region
    pub offset: u64,
    /// Length
    pub length: u32,
}

// ============================================================================
// CHANNEL RING BUFFER
// ============================================================================

/// Ring buffer for a single priority lane
struct PriorityLane {
    /// Messages
    messages: Vec<Option<ChannelMessage>>,
    /// Capacity
    capacity: usize,
    /// Write index
    write_idx: usize,
    /// Read index
    read_idx: usize,
    /// Current count
    count: usize,
    /// Total enqueued
    total_enqueued: u64,
    /// Total dequeued
    total_dequeued: u64,
    /// Total dropped (overflow)
    total_dropped: u64,
}

impl PriorityLane {
    fn new(capacity: usize) -> Self {
        let mut messages = Vec::with_capacity(capacity);
        for _ in 0..capacity {
            messages.push(None);
        }
        Self {
            messages,
            capacity,
            write_idx: 0,
            read_idx: 0,
            count: 0,
            total_enqueued: 0,
            total_dequeued: 0,
            total_dropped: 0,
        }
    }

    fn enqueue(&mut self, msg: ChannelMessage) -> bool {
        if self.count >= self.capacity {
            self.total_dropped += 1;
            return false;
        }
        self.messages[self.write_idx] = Some(msg);
        self.write_idx = (self.write_idx + 1) % self.capacity;
        self.count += 1;
        self.total_enqueued += 1;
        true
    }

    fn dequeue(&mut self) -> Option<ChannelMessage> {
        if self.count == 0 {
            return None;
        }
        let msg = self.messages[self.read_idx].take();
        self.read_idx = (self.read_idx + 1) % self.capacity;
        self.count -= 1;
        self.total_dequeued += 1;
        msg
    }

    fn peek(&self) -> Option<&ChannelMessage> {
        if self.count == 0 {
            return None;
        }
        self.messages[self.read_idx].as_ref()
    }

    fn is_empty(&self) -> bool {
        self.count == 0
    }

    fn is_full(&self) -> bool {
        self.count >= self.capacity
    }

    fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.count as f64 / self.capacity as f64
    }
}

// ============================================================================
// CHANNEL
// ============================================================================

/// Channel ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChannelId(pub u64);

/// Flow control state
#[derive(Debug, Clone, Copy)]
pub struct FlowControl {
    /// High watermark (pause)
    pub high_watermark: f64,
    /// Low watermark (resume)
    pub low_watermark: f64,
    /// Currently in backpressure
    pub in_backpressure: bool,
    /// Times backpressure triggered
    pub backpressure_count: u64,
}

impl Default for FlowControl {
    fn default() -> Self {
        Self {
            high_watermark: 0.8,
            low_watermark: 0.3,
            in_backpressure: false,
            backpressure_count: 0,
        }
    }
}

/// Channel statistics
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ChannelStats {
    /// Total sent
    pub total_sent: u64,
    /// Total received
    pub total_received: u64,
    /// Total dropped
    pub total_dropped: u64,
    /// Backpressure events
    pub backpressure_events: u64,
    /// Current queue depth
    pub queue_depth: usize,
    /// Max queue depth seen
    pub max_queue_depth: usize,
    /// Average utilization
    pub avg_utilization: f64,
}

/// A cooperation channel
pub struct Channel {
    /// Channel ID
    pub id: ChannelId,
    /// Endpoint A PID (typically app)
    pub endpoint_a: u64,
    /// Endpoint B PID (0 = kernel)
    pub endpoint_b: u64,
    /// Direction
    pub direction: ChannelDirection,
    /// State
    pub state: ChannelState,
    /// Priority lanes (Background, Normal, High, Critical)
    lanes: [PriorityLane; 4],
    /// Flow control
    flow: FlowControl,
    /// Creation time
    pub created_at: u64,
    /// Last activity
    pub last_activity: u64,
    /// Next sequence number
    next_sequence: u64,
}

impl Channel {
    pub fn new(
        id: ChannelId,
        endpoint_a: u64,
        endpoint_b: u64,
        direction: ChannelDirection,
        capacity_per_lane: usize,
        timestamp: u64,
    ) -> Self {
        Self {
            id,
            endpoint_a,
            endpoint_b,
            direction,
            state: ChannelState::Creating,
            lanes: [
                PriorityLane::new(capacity_per_lane),
                PriorityLane::new(capacity_per_lane),
                PriorityLane::new(capacity_per_lane / 2),
                PriorityLane::new(capacity_per_lane / 4),
            ],
            flow: FlowControl::default(),
            created_at: timestamp,
            last_activity: timestamp,
            next_sequence: 1,
        }
    }

    /// Open the channel
    #[inline(always)]
    pub fn open(&mut self) {
        self.state = ChannelState::Open;
    }

    /// Send a message
    pub fn send(
        &mut self,
        msg_type: u32,
        payload: MessagePayload,
        priority: ChannelPriority,
        timestamp: u64,
    ) -> Result<u64, ChannelError> {
        if self.state != ChannelState::Open {
            return Err(ChannelError::NotOpen);
        }

        let lane_idx = priority as usize;
        if lane_idx >= 4 {
            return Err(ChannelError::InvalidPriority);
        }

        // Check flow control
        if self.flow.in_backpressure && priority < ChannelPriority::Critical {
            return Err(ChannelError::Backpressure);
        }

        let seq = self.next_sequence;
        self.next_sequence += 1;

        let msg = ChannelMessage {
            sequence: seq,
            priority,
            msg_type,
            payload,
            timestamp,
            flags: 0,
        };

        if self.lanes[lane_idx].enqueue(msg) {
            self.last_activity = timestamp;
            self.update_flow_control();
            Ok(seq)
        } else {
            Err(ChannelError::Full)
        }
    }

    /// Receive next message (highest priority first)
    #[inline]
    pub fn receive(&mut self) -> Option<ChannelMessage> {
        // Check from highest priority to lowest
        for lane_idx in (0..4).rev() {
            if let Some(msg) = self.lanes[lane_idx].dequeue() {
                self.update_flow_control();
                return Some(msg);
            }
        }
        None
    }

    /// Peek next message
    #[inline]
    pub fn peek(&self) -> Option<&ChannelMessage> {
        for lane_idx in (0..4).rev() {
            if let Some(msg) = self.lanes[lane_idx].peek() {
                return Some(msg);
            }
        }
        None
    }

    /// Update flow control state
    fn update_flow_control(&mut self) {
        let max_util = self
            .lanes
            .iter()
            .map(|l| l.utilization())
            .fold(0.0f64, |a, b| if a > b { a } else { b });

        if !self.flow.in_backpressure && max_util >= self.flow.high_watermark {
            self.flow.in_backpressure = true;
            self.flow.backpressure_count += 1;
        } else if self.flow.in_backpressure && max_util <= self.flow.low_watermark {
            self.flow.in_backpressure = false;
        }
    }

    /// Get channel statistics
    pub fn stats(&self) -> ChannelStats {
        let total_sent: u64 = self.lanes.iter().map(|l| l.total_enqueued).sum();
        let total_received: u64 = self.lanes.iter().map(|l| l.total_dequeued).sum();
        let total_dropped: u64 = self.lanes.iter().map(|l| l.total_dropped).sum();
        let queue_depth: usize = self.lanes.iter().map(|l| l.count).sum();

        ChannelStats {
            total_sent,
            total_received,
            total_dropped,
            backpressure_events: self.flow.backpressure_count,
            queue_depth,
            max_queue_depth: queue_depth, // Simplified
            avg_utilization: self
                .lanes
                .iter()
                .map(|l| l.utilization())
                .sum::<f64>()
                / 4.0,
        }
    }

    /// Is channel empty?
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.lanes.iter().all(|l| l.is_empty())
    }

    /// Close channel (drain first)
    #[inline]
    pub fn close(&mut self) {
        if self.is_empty() {
            self.state = ChannelState::Closed;
        } else {
            self.state = ChannelState::Draining;
        }
    }
}

/// Channel error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelError {
    /// Channel not open
    NotOpen,
    /// Lane is full
    Full,
    /// Backpressure active
    Backpressure,
    /// Invalid priority
    InvalidPriority,
    /// Channel not found
    NotFound,
    /// Already exists
    AlreadyExists,
    /// Limit reached
    LimitReached,
}

// ============================================================================
// CHANNEL MANAGER
// ============================================================================

/// Manages all cooperation channels
pub struct ChannelManager {
    /// Channels by ID
    channels: BTreeMap<u64, Channel>,
    /// PID → channel IDs
    pid_channels: BTreeMap<u64, Vec<ChannelId>>,
    /// Next channel ID
    next_id: u64,
    /// Max channels per process
    max_per_process: usize,
    /// Total channels created
    pub total_created: u64,
    /// Total channels closed
    pub total_closed: u64,
    /// Total messages sent
    pub total_messages: u64,
}

impl ChannelManager {
    pub fn new(max_per_process: usize) -> Self {
        Self {
            channels: BTreeMap::new(),
            pid_channels: BTreeMap::new(),
            next_id: 1,
            max_per_process,
            total_created: 0,
            total_closed: 0,
            total_messages: 0,
        }
    }

    /// Create a channel
    pub fn create(
        &mut self,
        endpoint_a: u64,
        endpoint_b: u64,
        direction: ChannelDirection,
        capacity: usize,
        timestamp: u64,
    ) -> Result<ChannelId, ChannelError> {
        // Check limits
        let a_count = self.pid_channels.get(&endpoint_a).map_or(0, |v| v.len());
        if a_count >= self.max_per_process {
            return Err(ChannelError::LimitReached);
        }

        let id = ChannelId(self.next_id);
        self.next_id += 1;

        let mut channel = Channel::new(id, endpoint_a, endpoint_b, direction, capacity, timestamp);
        channel.open();

        self.channels.insert(id.0, channel);
        self.pid_channels
            .entry(endpoint_a)
            .or_insert_with(Vec::new)
            .push(id);
        if endpoint_b != 0 {
            self.pid_channels
                .entry(endpoint_b)
                .or_insert_with(Vec::new)
                .push(id);
        }

        self.total_created += 1;
        Ok(id)
    }

    /// Get channel
    #[inline(always)]
    pub fn get(&self, id: ChannelId) -> Option<&Channel> {
        self.channels.get(&id.0)
    }

    /// Get mutable channel
    #[inline(always)]
    pub fn get_mut(&mut self, id: ChannelId) -> Option<&mut Channel> {
        self.channels.get_mut(&id.0)
    }

    /// Send on channel
    #[inline]
    pub fn send(
        &mut self,
        id: ChannelId,
        msg_type: u32,
        payload: MessagePayload,
        priority: ChannelPriority,
        timestamp: u64,
    ) -> Result<u64, ChannelError> {
        let channel = self
            .channels
            .get_mut(&id.0)
            .ok_or(ChannelError::NotFound)?;
        let seq = channel.send(msg_type, payload, priority, timestamp)?;
        self.total_messages += 1;
        Ok(seq)
    }

    /// Receive from channel
    #[inline]
    pub fn receive(&mut self, id: ChannelId) -> Result<Option<ChannelMessage>, ChannelError> {
        let channel = self
            .channels
            .get_mut(&id.0)
            .ok_or(ChannelError::NotFound)?;
        Ok(channel.receive())
    }

    /// Close a channel
    #[inline]
    pub fn close(&mut self, id: ChannelId) {
        if let Some(channel) = self.channels.get_mut(&id.0) {
            channel.close();
            self.total_closed += 1;
        }
    }

    /// Close all channels for a PID
    #[inline]
    pub fn close_all_for_pid(&mut self, pid: u64) {
        if let Some(ids) = self.pid_channels.remove(&pid) {
            for id in ids {
                if let Some(ch) = self.channels.get_mut(&id.0) {
                    ch.close();
                    self.total_closed += 1;
                }
            }
        }
    }

    /// Active channel count
    #[inline]
    pub fn active_count(&self) -> usize {
        self.channels
            .values()
            .filter(|c| c.state == ChannelState::Open)
            .count()
    }

    /// Total channel count
    #[inline(always)]
    pub fn total_count(&self) -> usize {
        self.channels.len()
    }
}

// ============================================================================
// Merged from channel_v2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelType {
    /// Unbounded — infinite capacity
    Unbounded,
    /// Bounded — fixed capacity
    Bounded,
    /// Rendezvous — zero capacity, synchronous handoff
    Rendezvous,
    /// Oneshot — single message only
    Oneshot,
    /// Priority — messages ordered by priority
    Priority,
}

/// Channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelState {
    Open,
    SenderClosed,
    ReceiverClosed,
    FullyClosed,
}

/// A message in a channel
#[derive(Debug, Clone)]
pub struct ChannelMsg {
    pub seq: u64,
    pub sender_pid: u64,
    pub payload_size: usize,
    pub payload_hash: u64,
    pub priority: u32,
    pub enqueue_ns: u64,
    pub deadline_ns: u64,
}

impl ChannelMsg {
    pub fn new(seq: u64, sender: u64, size: usize) -> Self {
        Self {
            seq,
            sender_pid: sender,
            payload_size: size,
            payload_hash: 0,
            priority: 0,
            enqueue_ns: 0,
            deadline_ns: 0,
        }
    }

    #[inline(always)]
    pub fn with_priority(mut self, prio: u32) -> Self {
        self.priority = prio;
        self
    }

    #[inline(always)]
    pub fn is_expired(&self, now_ns: u64) -> bool {
        self.deadline_ns > 0 && now_ns > self.deadline_ns
    }
}

/// Sender endpoint
#[derive(Debug)]
pub struct ChannelSender {
    pub id: u64,
    pub pid: u64,
    pub sends: u64,
    pub blocked_count: u64,
    pub bytes_sent: u64,
}

impl ChannelSender {
    pub fn new(id: u64, pid: u64) -> Self {
        Self { id, pid, sends: 0, blocked_count: 0, bytes_sent: 0 }
    }

    #[inline(always)]
    pub fn record_send(&mut self, size: usize) {
        self.sends += 1;
        self.bytes_sent += size as u64;
    }

    #[inline(always)]
    pub fn record_blocked(&mut self) {
        self.blocked_count += 1;
    }

    #[inline(always)]
    pub fn block_rate(&self) -> f64 {
        if self.sends == 0 { return 0.0; }
        self.blocked_count as f64 / (self.sends + self.blocked_count) as f64
    }
}

/// Receiver endpoint
#[derive(Debug)]
pub struct ChannelReceiver {
    pub id: u64,
    pub pid: u64,
    pub receives: u64,
    pub empty_polls: u64,
    pub bytes_received: u64,
    pub total_latency_ns: u64,
}

impl ChannelReceiver {
    pub fn new(id: u64, pid: u64) -> Self {
        Self { id, pid, receives: 0, empty_polls: 0, bytes_received: 0, total_latency_ns: 0 }
    }

    #[inline]
    pub fn record_receive(&mut self, size: usize, latency_ns: u64) {
        self.receives += 1;
        self.bytes_received += size as u64;
        self.total_latency_ns += latency_ns;
    }

    #[inline(always)]
    pub fn avg_latency_ns(&self) -> f64 {
        if self.receives == 0 { return 0.0; }
        self.total_latency_ns as f64 / self.receives as f64
    }

    #[inline]
    pub fn empty_rate(&self) -> f64 {
        let total = self.receives + self.empty_polls;
        if total == 0 { return 0.0; }
        self.empty_polls as f64 / total as f64
    }
}

/// A channel instance
#[derive(Debug)]
pub struct ChannelInstance {
    pub id: u64,
    pub name: String,
    pub ch_type: ChannelType,
    pub state: ChannelState,
    pub capacity: usize,
    queue: Vec<ChannelMsg>,
    senders: Vec<ChannelSender>,
    receivers: Vec<ChannelReceiver>,
    next_seq: u64,
    pub total_sends: u64,
    pub total_receives: u64,
    pub total_bytes: u64,
    pub overflow_count: u64,
    pub created_ns: u64,
}

impl ChannelInstance {
    pub fn new(id: u64, name: String, ch_type: ChannelType, capacity: usize) -> Self {
        Self {
            id,
            name,
            ch_type,
            state: ChannelState::Open,
            capacity,
            queue: Vec::new(),
            senders: Vec::new(),
            receivers: Vec::new(),
            next_seq: 1,
            total_sends: 0,
            total_receives: 0,
            total_bytes: 0,
            overflow_count: 0,
            created_ns: 0,
        }
    }

    #[inline(always)]
    pub fn add_sender(&mut self, id: u64, pid: u64) {
        self.senders.push(ChannelSender::new(id, pid));
    }

    #[inline(always)]
    pub fn add_receiver(&mut self, id: u64, pid: u64) {
        self.receivers.push(ChannelReceiver::new(id, pid));
    }

    pub fn send(&mut self, sender_pid: u64, payload_size: usize, now_ns: u64) -> Option<u64> {
        if self.state == ChannelState::ReceiverClosed || self.state == ChannelState::FullyClosed {
            return None;
        }
        if self.ch_type == ChannelType::Bounded && self.queue.len() >= self.capacity {
            if let Some(s) = self.senders.iter_mut().find(|s| s.pid == sender_pid) {
                s.record_blocked();
            }
            self.overflow_count += 1;
            return None;
        }
        if self.ch_type == ChannelType::Oneshot && self.total_sends > 0 {
            return None;
        }

        let seq = self.next_seq;
        self.next_seq += 1;
        let mut msg = ChannelMsg::new(seq, sender_pid, payload_size);
        msg.enqueue_ns = now_ns;
        self.queue.push(msg);

        if let Some(s) = self.senders.iter_mut().find(|s| s.pid == sender_pid) {
            s.record_send(payload_size);
        }
        self.total_sends += 1;
        self.total_bytes += payload_size as u64;
        Some(seq)
    }

    pub fn receive(&mut self, receiver_pid: u64, now_ns: u64) -> Option<ChannelMsg> {
        if self.queue.is_empty() {
            if let Some(r) = self.receivers.iter_mut().find(|r| r.pid == receiver_pid) {
                r.empty_polls += 1;
            }
            return None;
        }

        // Priority channel: pick highest priority
        let idx = if self.ch_type == ChannelType::Priority {
            self.queue.iter().enumerate()
                .max_by_key(|(_, m)| m.priority)
                .map(|(i, _)| i)
                .unwrap_or(0)
        } else {
            0
        };

        let msg = self.queue.remove(idx);
        let latency = now_ns.saturating_sub(msg.enqueue_ns);

        if let Some(r) = self.receivers.iter_mut().find(|r| r.pid == receiver_pid) {
            r.record_receive(msg.payload_size, latency);
        }
        self.total_receives += 1;
        Some(msg)
    }

    #[inline(always)]
    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    #[inline(always)]
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 { return 0.0; }
        self.queue.len() as f64 / self.capacity as f64
    }

    #[inline]
    pub fn close_sender(&mut self) {
        self.state = match self.state {
            ChannelState::ReceiverClosed => ChannelState::FullyClosed,
            _ => ChannelState::SenderClosed,
        };
    }

    #[inline]
    pub fn close_receiver(&mut self) {
        self.state = match self.state {
            ChannelState::SenderClosed => ChannelState::FullyClosed,
            _ => ChannelState::ReceiverClosed,
        };
    }

    #[inline(always)]
    pub fn throughput_ratio(&self) -> f64 {
        if self.total_sends == 0 { return 0.0; }
        self.total_receives as f64 / self.total_sends as f64
    }
}

/// Channel v2 stats
#[derive(Debug, Clone)]
#[repr(align(64))]
pub struct ChannelV2Stats {
    pub total_channels: u64,
    pub total_sends: u64,
    pub total_receives: u64,
    pub total_bytes: u64,
    pub total_overflows: u64,
    pub active_channels: u64,
}

/// Main channel v2 manager
pub struct CoopChannelV2 {
    channels: BTreeMap<u64, ChannelInstance>,
    next_id: u64,
    next_endpoint_id: u64,
    stats: ChannelV2Stats,
}

impl CoopChannelV2 {
    pub fn new() -> Self {
        Self {
            channels: BTreeMap::new(),
            next_id: 1,
            next_endpoint_id: 1,
            stats: ChannelV2Stats {
                total_channels: 0,
                total_sends: 0,
                total_receives: 0,
                total_bytes: 0,
                total_overflows: 0,
                active_channels: 0,
            },
        }
    }

    #[inline]
    pub fn create_channel(&mut self, name: String, ch_type: ChannelType, capacity: usize) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.channels.insert(id, ChannelInstance::new(id, name, ch_type, capacity));
        self.stats.total_channels += 1;
        self.stats.active_channels += 1;
        id
    }

    #[inline]
    pub fn register_sender(&mut self, chan_id: u64, pid: u64) -> u64 {
        let eid = self.next_endpoint_id;
        self.next_endpoint_id += 1;
        if let Some(ch) = self.channels.get_mut(&chan_id) {
            ch.add_sender(eid, pid);
        }
        eid
    }

    #[inline]
    pub fn register_receiver(&mut self, chan_id: u64, pid: u64) -> u64 {
        let eid = self.next_endpoint_id;
        self.next_endpoint_id += 1;
        if let Some(ch) = self.channels.get_mut(&chan_id) {
            ch.add_receiver(eid, pid);
        }
        eid
    }

    pub fn send(&mut self, chan_id: u64, sender_pid: u64, payload_size: usize, now_ns: u64) -> Option<u64> {
        if let Some(ch) = self.channels.get_mut(&chan_id) {
            let result = ch.send(sender_pid, payload_size, now_ns);
            if result.is_some() {
                self.stats.total_sends += 1;
                self.stats.total_bytes += payload_size as u64;
            } else {
                self.stats.total_overflows += 1;
            }
            result
        } else {
            None
        }
    }

    #[inline]
    pub fn receive(&mut self, chan_id: u64, receiver_pid: u64, now_ns: u64) -> Option<ChannelMsg> {
        if let Some(ch) = self.channels.get_mut(&chan_id) {
            let msg = ch.receive(receiver_pid, now_ns);
            if msg.is_some() {
                self.stats.total_receives += 1;
            }
            msg
        } else {
            None
        }
    }

    #[inline]
    pub fn close_channel(&mut self, chan_id: u64) {
        if let Some(ch) = self.channels.get_mut(&chan_id) {
            ch.close_sender();
            ch.close_receiver();
            self.stats.active_channels = self.stats.active_channels.saturating_sub(1);
        }
    }

    #[inline]
    pub fn fullest_channels(&self, top: usize) -> Vec<(u64, f64)> {
        let mut v: Vec<(u64, f64)> = self.channels.iter()
            .filter(|(_, ch)| ch.capacity > 0)
            .map(|(&id, ch)| (id, ch.utilization()))
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(core::cmp::Ordering::Equal));
        v.truncate(top);
        v
    }

    #[inline(always)]
    pub fn get_channel(&self, id: u64) -> Option<&ChannelInstance> {
        self.channels.get(&id)
    }

    #[inline(always)]
    pub fn stats(&self) -> &ChannelV2Stats {
        &self.stats
    }
}
