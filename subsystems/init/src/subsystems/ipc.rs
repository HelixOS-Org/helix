//! # IPC Subsystem
//!
//! Inter-Process Communication subsystem for kernel and userspace.
//! Supports message passing, shared memory, signals, and synchronization primitives.

use crate::context::InitContext;
use crate::error::{ErrorKind, InitError, InitResult};
use crate::phase::{InitPhase, PhaseCapabilities};
use crate::subsystem::{Dependency, Subsystem, SubsystemId, SubsystemInfo};

extern crate alloc;
use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

// =============================================================================
// IPC TYPES
// =============================================================================

/// IPC endpoint identifier
pub type EndpointId = u64;

/// Message identifier
pub type MessageId = u64;

/// Channel identifier
pub type ChannelId = u64;

/// Maximum message size
pub const MAX_MESSAGE_SIZE: usize = 64 * 1024; // 64 KB

/// Maximum messages per queue
pub const MAX_QUEUE_SIZE: usize = 256;

// =============================================================================
// MESSAGE TYPES
// =============================================================================

/// IPC message
#[derive(Debug, Clone)]
pub struct Message {
    pub id: MessageId,
    pub sender: EndpointId,
    pub recipient: EndpointId,
    pub msg_type: MessageType,
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub priority: u8,
    pub flags: MessageFlags,
}

impl Message {
    /// Create new message
    pub fn new(sender: EndpointId, recipient: EndpointId, data: Vec<u8>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            sender,
            recipient,
            msg_type: MessageType::Data,
            data,
            timestamp: 0, // Set by IPC subsystem
            priority: 128,
            flags: MessageFlags::default(),
        }
    }

    /// Create system message
    pub fn system(sender: EndpointId, recipient: EndpointId, msg_type: MessageType) -> Self {
        let mut msg = Self::new(sender, recipient, Vec::new());
        msg.msg_type = msg_type;
        msg
    }

    /// Get message size
    pub fn size(&self) -> usize {
        self.data.len()
    }
}

/// Message type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Data,
    Request,
    Response,
    Signal,
    Notify,
    Broadcast,
    Multicast,
    Error,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::Data
    }
}

/// Message flags
#[derive(Debug, Clone, Copy, Default)]
pub struct MessageFlags {
    pub urgent: bool,
    pub no_reply: bool,
    pub sync: bool,
    pub broadcast: bool,
}

// =============================================================================
// CHANNELS
// =============================================================================

/// IPC channel
pub struct Channel {
    pub id: ChannelId,
    pub name: String,
    pub endpoint_a: EndpointId,
    pub endpoint_b: EndpointId,
    pub buffer_a_to_b: VecDeque<Message>,
    pub buffer_b_to_a: VecDeque<Message>,
    pub closed: AtomicBool,
    pub messages_sent: AtomicU64,
    pub messages_received: AtomicU64,
}

impl Channel {
    /// Create new channel
    pub fn new(id: ChannelId, name: String, a: EndpointId, b: EndpointId) -> Self {
        Self {
            id,
            name,
            endpoint_a: a,
            endpoint_b: b,
            buffer_a_to_b: VecDeque::with_capacity(32),
            buffer_b_to_a: VecDeque::with_capacity(32),
            closed: AtomicBool::new(false),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
        }
    }

    /// Is channel closed?
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Close channel
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    /// Send message from endpoint
    pub fn send(&mut self, msg: Message) -> InitResult<()> {
        if self.is_closed() {
            return Err(InitError::new(ErrorKind::Unavailable, "Channel closed"));
        }

        let buffer = if msg.sender == self.endpoint_a {
            &mut self.buffer_a_to_b
        } else {
            &mut self.buffer_b_to_a
        };

        if buffer.len() >= MAX_QUEUE_SIZE {
            return Err(InitError::new(ErrorKind::BufferFull, "Queue full"));
        }

        buffer.push_back(msg);
        self.messages_sent.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Receive message for endpoint
    pub fn receive(&mut self, endpoint: EndpointId) -> Option<Message> {
        let buffer = if endpoint == self.endpoint_a {
            &mut self.buffer_b_to_a
        } else {
            &mut self.buffer_a_to_b
        };

        let msg = buffer.pop_front();
        if msg.is_some() {
            self.messages_received.fetch_add(1, Ordering::Relaxed);
        }
        msg
    }
}

// =============================================================================
// ENDPOINTS
// =============================================================================

/// IPC endpoint
pub struct Endpoint {
    pub id: EndpointId,
    pub name: String,
    pub owner: u64, // Task ID
    pub channels: Vec<ChannelId>,
    pub inbox: VecDeque<Message>,
    pub max_inbox_size: usize,
}

impl Endpoint {
    /// Create new endpoint
    pub fn new(id: EndpointId, name: String, owner: u64) -> Self {
        Self {
            id,
            name,
            owner,
            channels: Vec::new(),
            inbox: VecDeque::with_capacity(32),
            max_inbox_size: MAX_QUEUE_SIZE,
        }
    }

    /// Queue message in inbox
    pub fn queue(&mut self, msg: Message) -> InitResult<()> {
        if self.inbox.len() >= self.max_inbox_size {
            return Err(InitError::new(ErrorKind::BufferFull, "Inbox full"));
        }
        self.inbox.push_back(msg);
        Ok(())
    }

    /// Get next message from inbox
    pub fn dequeue(&mut self) -> Option<Message> {
        self.inbox.pop_front()
    }

    /// Check if inbox has messages
    pub fn has_messages(&self) -> bool {
        !self.inbox.is_empty()
    }
}

// =============================================================================
// SHARED MEMORY
// =============================================================================

/// Shared memory region
pub struct SharedMemory {
    pub id: u64,
    pub name: String,
    pub base: u64,
    pub size: usize,
    pub owners: Vec<EndpointId>,
    pub permissions: SharedMemPermissions,
}

impl SharedMemory {
    /// Create new shared memory region
    pub fn new(id: u64, name: String, base: u64, size: usize) -> Self {
        Self {
            id,
            name,
            base,
            size,
            owners: Vec::new(),
            permissions: SharedMemPermissions::default(),
        }
    }
}

/// Shared memory permissions
#[derive(Debug, Clone, Copy, Default)]
pub struct SharedMemPermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

// =============================================================================
// SYNCHRONIZATION PRIMITIVES
// =============================================================================

/// Mutex state
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutexState {
    Unlocked  = 0,
    Locked    = 1,
    Contended = 2,
}

/// IPC Mutex
pub struct IpcMutex {
    pub id: u64,
    pub name: String,
    pub state: AtomicU32,
    pub owner: AtomicU64,
    pub waiters: VecDeque<u64>, // Task IDs
}

impl IpcMutex {
    /// Create new mutex
    pub fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            state: AtomicU32::new(MutexState::Unlocked as u32),
            owner: AtomicU64::new(0),
            waiters: VecDeque::new(),
        }
    }

    /// Try to lock
    pub fn try_lock(&self, task_id: u64) -> bool {
        let result = self.state.compare_exchange(
            MutexState::Unlocked as u32,
            MutexState::Locked as u32,
            Ordering::Acquire,
            Ordering::Relaxed,
        );

        if result.is_ok() {
            self.owner.store(task_id, Ordering::Release);
            true
        } else {
            false
        }
    }

    /// Unlock
    pub fn unlock(&self, task_id: u64) -> bool {
        if self.owner.load(Ordering::Acquire) != task_id {
            return false;
        }

        self.owner.store(0, Ordering::Release);
        self.state
            .store(MutexState::Unlocked as u32, Ordering::Release);
        true
    }
}

/// IPC Semaphore
pub struct IpcSemaphore {
    pub id: u64,
    pub name: String,
    pub count: AtomicU32,
    pub max_count: u32,
    pub waiters: VecDeque<u64>,
}

impl IpcSemaphore {
    /// Create new semaphore
    pub fn new(id: u64, name: String, initial: u32, max: u32) -> Self {
        Self {
            id,
            name,
            count: AtomicU32::new(initial),
            max_count: max,
            waiters: VecDeque::new(),
        }
    }

    /// Wait (decrement)
    pub fn wait(&self) -> bool {
        loop {
            let current = self.count.load(Ordering::Acquire);
            if current == 0 {
                return false;
            }

            if self
                .count
                .compare_exchange(current, current - 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }

    /// Signal (increment)
    pub fn signal(&self) -> bool {
        loop {
            let current = self.count.load(Ordering::Acquire);
            if current >= self.max_count {
                return false;
            }

            if self
                .count
                .compare_exchange(current, current + 1, Ordering::AcqRel, Ordering::Relaxed)
                .is_ok()
            {
                return true;
            }
        }
    }
}

// =============================================================================
// IPC SUBSYSTEM
// =============================================================================

/// IPC Subsystem
///
/// Manages inter-process communication.
pub struct IpcSubsystem {
    info: SubsystemInfo,

    // Endpoints
    endpoints: Vec<Endpoint>,
    next_endpoint_id: AtomicU64,

    // Channels
    channels: Vec<Channel>,
    next_channel_id: AtomicU64,

    // Shared memory
    shared_regions: Vec<SharedMemory>,
    next_shm_id: AtomicU64,

    // Synchronization primitives
    mutexes: Vec<IpcMutex>,
    semaphores: Vec<IpcSemaphore>,
    next_sync_id: AtomicU64,

    // Statistics
    messages_sent: AtomicU64,
    messages_received: AtomicU64,
}

static IPC_DEPS: [Dependency; 2] = [
    Dependency::required("scheduler"),
    Dependency::required("heap"),
];

impl IpcSubsystem {
    /// Create new IPC subsystem
    pub fn new() -> Self {
        Self {
            info: SubsystemInfo::new("ipc", InitPhase::Core)
                .with_priority(700)
                .with_description("Inter-process communication")
                .with_dependencies(&IPC_DEPS)
                .provides(PhaseCapabilities::IPC),
            endpoints: Vec::new(),
            next_endpoint_id: AtomicU64::new(1),
            channels: Vec::new(),
            next_channel_id: AtomicU64::new(1),
            shared_regions: Vec::new(),
            next_shm_id: AtomicU64::new(1),
            mutexes: Vec::new(),
            semaphores: Vec::new(),
            next_sync_id: AtomicU64::new(1),
            messages_sent: AtomicU64::new(0),
            messages_received: AtomicU64::new(0),
        }
    }

    /// Create endpoint
    pub fn create_endpoint(&mut self, name: &str, owner: u64) -> EndpointId {
        let id = self.next_endpoint_id.fetch_add(1, Ordering::SeqCst);
        self.endpoints
            .push(Endpoint::new(id, String::from(name), owner));
        id
    }

    /// Get endpoint by ID
    pub fn get_endpoint(&self, id: EndpointId) -> Option<&Endpoint> {
        self.endpoints.iter().find(|e| e.id == id)
    }

    /// Get endpoint by ID (mutable)
    pub fn get_endpoint_mut(&mut self, id: EndpointId) -> Option<&mut Endpoint> {
        self.endpoints.iter_mut().find(|e| e.id == id)
    }

    /// Find endpoint by name
    pub fn find_endpoint(&self, name: &str) -> Option<EndpointId> {
        self.endpoints.iter().find(|e| e.name == name).map(|e| e.id)
    }

    /// Create channel between endpoints
    pub fn create_channel(
        &mut self,
        name: &str,
        a: EndpointId,
        b: EndpointId,
    ) -> InitResult<ChannelId> {
        // Verify endpoints exist
        if self.get_endpoint(a).is_none() || self.get_endpoint(b).is_none() {
            return Err(InitError::new(ErrorKind::NotFound, "Endpoint not found"));
        }

        let id = self.next_channel_id.fetch_add(1, Ordering::SeqCst);
        let channel = Channel::new(id, String::from(name), a, b);
        self.channels.push(channel);

        // Add channel to endpoints
        if let Some(ep_a) = self.get_endpoint_mut(a) {
            ep_a.channels.push(id);
        }
        if let Some(ep_b) = self.get_endpoint_mut(b) {
            ep_b.channels.push(id);
        }

        Ok(id)
    }

    /// Send message
    pub fn send(&mut self, msg: Message) -> InitResult<MessageId> {
        let msg_id = msg.id;

        // Try channel first
        for channel in &mut self.channels {
            if (channel.endpoint_a == msg.sender && channel.endpoint_b == msg.recipient)
                || (channel.endpoint_b == msg.sender && channel.endpoint_a == msg.recipient)
            {
                channel.send(msg)?;
                self.messages_sent.fetch_add(1, Ordering::Relaxed);
                return Ok(msg_id);
            }
        }

        // Fall back to endpoint inbox
        if let Some(endpoint) = self.get_endpoint_mut(msg.recipient) {
            endpoint.queue(msg)?;
            self.messages_sent.fetch_add(1, Ordering::Relaxed);
            return Ok(msg_id);
        }

        Err(InitError::new(ErrorKind::NotFound, "Recipient not found"))
    }

    /// Receive message
    pub fn receive(&mut self, endpoint_id: EndpointId) -> Option<Message> {
        // Check channels first
        for channel in &mut self.channels {
            if channel.endpoint_a == endpoint_id || channel.endpoint_b == endpoint_id {
                if let Some(msg) = channel.receive(endpoint_id) {
                    self.messages_received.fetch_add(1, Ordering::Relaxed);
                    return Some(msg);
                }
            }
        }

        // Check endpoint inbox
        if let Some(endpoint) = self.get_endpoint_mut(endpoint_id) {
            if let Some(msg) = endpoint.dequeue() {
                self.messages_received.fetch_add(1, Ordering::Relaxed);
                return Some(msg);
            }
        }

        None
    }

    /// Create shared memory region
    pub fn create_shared_memory(&mut self, name: &str, size: usize) -> InitResult<u64> {
        let id = self.next_shm_id.fetch_add(1, Ordering::SeqCst);

        // In real kernel: allocate physical pages and map them
        let base = 0x1000_0000 + (id * 0x10000); // Placeholder

        self.shared_regions
            .push(SharedMemory::new(id, String::from(name), base, size));

        Ok(id)
    }

    /// Create mutex
    pub fn create_mutex(&mut self, name: &str) -> u64 {
        let id = self.next_sync_id.fetch_add(1, Ordering::SeqCst);
        self.mutexes.push(IpcMutex::new(id, String::from(name)));
        id
    }

    /// Create semaphore
    pub fn create_semaphore(&mut self, name: &str, initial: u32, max: u32) -> u64 {
        let id = self.next_sync_id.fetch_add(1, Ordering::SeqCst);
        self.semaphores
            .push(IpcSemaphore::new(id, String::from(name), initial, max));
        id
    }

    /// Get statistics
    pub fn stats(&self) -> IpcStats {
        IpcStats {
            endpoints: self.endpoints.len(),
            channels: self.channels.len(),
            shared_regions: self.shared_regions.len(),
            mutexes: self.mutexes.len(),
            semaphores: self.semaphores.len(),
            messages_sent: self.messages_sent.load(Ordering::Relaxed),
            messages_received: self.messages_received.load(Ordering::Relaxed),
        }
    }
}

/// IPC statistics
#[derive(Debug, Clone)]
pub struct IpcStats {
    pub endpoints: usize,
    pub channels: usize,
    pub shared_regions: usize,
    pub mutexes: usize,
    pub semaphores: usize,
    pub messages_sent: u64,
    pub messages_received: u64,
}

impl Default for IpcSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl Subsystem for IpcSubsystem {
    fn info(&self) -> &SubsystemInfo {
        &self.info
    }

    fn init(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        ctx.info("Initializing IPC subsystem");

        // Create kernel endpoint
        let kernel_ep = self.create_endpoint("kernel", 0);
        ctx.debug(alloc::format!("Kernel endpoint: {}", kernel_ep));

        // Create system services endpoint
        let services_ep = self.create_endpoint("services", 0);
        ctx.debug(alloc::format!("Services endpoint: {}", services_ep));

        // Create channel between kernel and services
        let _channel = self.create_channel("kernel-services", kernel_ep, services_ep)?;

        ctx.info("IPC subsystem initialized");

        Ok(())
    }

    fn shutdown(&mut self, ctx: &mut InitContext) -> InitResult<()> {
        let stats = self.stats();
        ctx.info(alloc::format!(
            "IPC shutdown: {} endpoints, {} channels, {} messages",
            stats.endpoints,
            stats.channels,
            stats.messages_sent
        ));

        // Close all channels
        for channel in &self.channels {
            channel.close();
        }

        Ok(())
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_subsystem() {
        let sub = IpcSubsystem::new();
        assert_eq!(sub.info().phase, InitPhase::Core);
        assert!(sub.info().provides.contains(PhaseCapabilities::IPC));
    }

    #[test]
    fn test_endpoint_creation() {
        let mut ipc = IpcSubsystem::new();

        let ep = ipc.create_endpoint("test", 1);
        assert!(ep > 0);

        let endpoint = ipc.get_endpoint(ep).unwrap();
        assert_eq!(endpoint.name, "test");
        assert_eq!(endpoint.owner, 1);
    }

    #[test]
    fn test_message() {
        let msg = Message::new(1, 2, vec![0, 1, 2, 3]);

        assert!(msg.id > 0);
        assert_eq!(msg.sender, 1);
        assert_eq!(msg.recipient, 2);
        assert_eq!(msg.size(), 4);
    }

    #[test]
    fn test_mutex() {
        let mutex = IpcMutex::new(1, String::from("test"));

        assert!(mutex.try_lock(1));
        assert!(!mutex.try_lock(2)); // Already locked
        assert!(mutex.unlock(1));
        assert!(mutex.try_lock(2)); // Now available
    }

    #[test]
    fn test_semaphore() {
        let sem = IpcSemaphore::new(1, String::from("test"), 2, 2);

        assert!(sem.wait()); // 2 -> 1
        assert!(sem.wait()); // 1 -> 0
        assert!(!sem.wait()); // Already 0

        assert!(sem.signal()); // 0 -> 1
        assert!(sem.wait()); // 1 -> 0
    }
}
