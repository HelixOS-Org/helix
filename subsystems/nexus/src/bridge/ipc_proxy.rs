//! # Bridge IPC Proxy
//!
//! Inter-Process Communication proxy layer:
//! - Unix socket, pipe, shared memory abstraction
//! - Message routing between processes
//! - IPC channel creation and lifecycle
//! - Flow control and backpressure
//! - Message batching for throughput
//! - Cross-namespace IPC bridging

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

/// IPC channel type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcChannelType {
    Pipe,
    UnixStream,
    UnixDgram,
    SharedMemory,
    MessageQueue,
    Futex,
}

/// Channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcChannelState {
    Created,
    Connected,
    HalfClosed,
    Closed,
    Error,
}

/// IPC message
#[derive(Debug, Clone)]
pub struct IpcMessage {
    pub msg_id: u64,
    pub channel_id: u64,
    pub sender_pid: u64,
    pub receiver_pid: u64,
    pub payload_size: u32,
    pub payload_hash: u64,
    pub priority: u8,
    pub timestamp: u64,
}

impl IpcMessage {
    pub fn new(msg_id: u64, channel_id: u64, sender: u64, receiver: u64, size: u32) -> Self {
        Self {
            msg_id,
            channel_id,
            sender_pid: sender,
            receiver_pid: receiver,
            payload_size: size,
            payload_hash: 0,
            priority: 0,
            timestamp: 0,
        }
    }
}

/// IPC channel
#[derive(Debug, Clone)]
pub struct IpcChannel {
    pub channel_id: u64,
    pub channel_type: IpcChannelType,
    pub state: IpcChannelState,
    pub endpoint_a: u64, // pid
    pub endpoint_b: u64, // pid
    pub buffer_size: u32,
    pub buffered_bytes: u32,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_transferred: u64,
    pub created_ts: u64,
    pub backpressure: bool,
}

impl IpcChannel {
    pub fn new(id: u64, chan_type: IpcChannelType, pid_a: u64, pid_b: u64, buffer_size: u32) -> Self {
        Self {
            channel_id: id,
            channel_type: chan_type,
            state: IpcChannelState::Created,
            endpoint_a: pid_a,
            endpoint_b: pid_b,
            buffer_size,
            buffered_bytes: 0,
            messages_sent: 0,
            messages_received: 0,
            bytes_transferred: 0,
            created_ts: 0,
            backpressure: false,
        }
    }

    pub fn utilization(&self) -> f64 {
        if self.buffer_size == 0 { return 0.0; }
        self.buffered_bytes as f64 / self.buffer_size as f64
    }

    pub fn send(&mut self, size: u32) -> bool {
        if self.buffered_bytes + size > self.buffer_size {
            self.backpressure = true;
            return false;
        }
        self.buffered_bytes += size;
        self.messages_sent += 1;
        self.bytes_transferred += size as u64;
        true
    }

    pub fn receive(&mut self, size: u32) -> bool {
        if self.buffered_bytes < size { return false; }
        self.buffered_bytes -= size;
        self.messages_received += 1;
        if self.utilization() < 0.5 {
            self.backpressure = false;
        }
        true
    }
}

/// Routing entry for cross-namespace IPC
#[derive(Debug, Clone)]
pub struct IpcRoute {
    pub src_namespace: u32,
    pub dst_namespace: u32,
    pub src_pid: u64,
    pub dst_pid: u64,
    pub channel_id: u64,
    pub allowed: bool,
}

/// Message batch for throughput optimization
#[derive(Debug, Clone)]
pub struct IpcBatch {
    pub channel_id: u64,
    pub messages: Vec<IpcMessage>,
    pub total_bytes: u64,
    pub deadline_ns: u64,
}

impl IpcBatch {
    pub fn new(channel_id: u64) -> Self {
        Self {
            channel_id,
            messages: Vec::new(),
            total_bytes: 0,
            deadline_ns: 0,
        }
    }

    pub fn add(&mut self, msg: IpcMessage) {
        self.total_bytes += msg.payload_size as u64;
        self.messages.push(msg);
    }

    pub fn is_ready(&self, max_batch: usize, now: u64) -> bool {
        self.messages.len() >= max_batch || (self.deadline_ns > 0 && now >= self.deadline_ns)
    }
}

/// IPC proxy stats
#[derive(Debug, Clone, Default)]
pub struct BridgeIpcProxyStats {
    pub active_channels: usize,
    pub total_messages: u64,
    pub total_bytes: u64,
    pub backpressured_channels: usize,
    pub routes: usize,
    pub pending_batches: usize,
}

/// Bridge IPC Proxy
pub struct BridgeIpcProxy {
    channels: BTreeMap<u64, IpcChannel>,
    routes: Vec<IpcRoute>,
    batches: BTreeMap<u64, IpcBatch>,
    next_channel_id: u64,
    next_msg_id: u64,
    max_batch_size: usize,
    stats: BridgeIpcProxyStats,
}

impl BridgeIpcProxy {
    pub fn new() -> Self {
        Self {
            channels: BTreeMap::new(),
            routes: Vec::new(),
            batches: BTreeMap::new(),
            next_channel_id: 1,
            next_msg_id: 1,
            max_batch_size: 32,
            stats: BridgeIpcProxyStats::default(),
        }
    }

    pub fn create_channel(&mut self, chan_type: IpcChannelType, pid_a: u64, pid_b: u64, buffer_size: u32) -> u64 {
        let id = self.next_channel_id;
        self.next_channel_id += 1;
        let mut chan = IpcChannel::new(id, chan_type, pid_a, pid_b, buffer_size);
        chan.state = IpcChannelState::Connected;
        self.channels.insert(id, chan);
        self.recompute();
        id
    }

    pub fn close_channel(&mut self, channel_id: u64) {
        if let Some(chan) = self.channels.get_mut(&channel_id) {
            chan.state = IpcChannelState::Closed;
        }
        self.recompute();
    }

    /// Send a message through a channel
    pub fn send_message(&mut self, channel_id: u64, sender: u64, size: u32, now: u64) -> Option<u64> {
        let chan = self.channels.get_mut(&channel_id)?;
        if chan.state != IpcChannelState::Connected { return None; }
        if !chan.send(size) { return None; }

        let receiver = if chan.endpoint_a == sender { chan.endpoint_b } else { chan.endpoint_a };
        let msg_id = self.next_msg_id;
        self.next_msg_id += 1;

        let msg = IpcMessage {
            msg_id,
            channel_id,
            sender_pid: sender,
            receiver_pid: receiver,
            payload_size: size,
            payload_hash: 0,
            priority: 0,
            timestamp: now,
        };

        // Add to batch
        let batch = self.batches.entry(channel_id).or_insert_with(|| IpcBatch::new(channel_id));
        batch.add(msg);

        self.recompute();
        Some(msg_id)
    }

    /// Receive pending messages from a channel
    pub fn receive_batch(&mut self, channel_id: u64) -> Vec<IpcMessage> {
        if let Some(batch) = self.batches.remove(&channel_id) {
            if let Some(chan) = self.channels.get_mut(&channel_id) {
                for msg in &batch.messages {
                    chan.receive(msg.payload_size);
                }
            }
            self.recompute();
            batch.messages
        } else {
            Vec::new()
        }
    }

    /// Add cross-namespace route
    pub fn add_route(&mut self, route: IpcRoute) {
        self.routes.push(route);
        self.recompute();
    }

    /// Check if IPC between two pids is routable
    pub fn is_routable(&self, src_ns: u32, dst_ns: u32, src_pid: u64, dst_pid: u64) -> bool {
        if src_ns == dst_ns { return true; }
        self.routes.iter().any(|r| {
            r.allowed && r.src_namespace == src_ns && r.dst_namespace == dst_ns
                && r.src_pid == src_pid && r.dst_pid == dst_pid
        })
    }

    fn recompute(&mut self) {
        let active = self.channels.values()
            .filter(|c| c.state == IpcChannelState::Connected)
            .count();
        let total_msgs: u64 = self.channels.values().map(|c| c.messages_sent).sum();
        let total_bytes: u64 = self.channels.values().map(|c| c.bytes_transferred).sum();
        let bp = self.channels.values().filter(|c| c.backpressure).count();

        self.stats = BridgeIpcProxyStats {
            active_channels: active,
            total_messages: total_msgs,
            total_bytes: total_bytes,
            backpressured_channels: bp,
            routes: self.routes.len(),
            pending_batches: self.batches.len(),
        };
    }

    pub fn stats(&self) -> &BridgeIpcProxyStats {
        &self.stats
    }

    pub fn channel(&self, id: u64) -> Option<&IpcChannel> {
        self.channels.get(&id)
    }
}
